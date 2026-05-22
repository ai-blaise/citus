//! Companion-side OpenTelemetry trace-context plan.
//!
//! The pool proxy peeks the libpq `application_name` startup parameter for
//! an embedded W3C traceparent and replays the bytes to PostgreSQL untouched
//! (see `ai_blaise_citus_pool::trace_tap`). PostgreSQL writes that string
//! into the session-level `application_name` GUC. The companion pgrx
//! extension uses this plan to:
//!
//! * read the inbound traceparent back from `current_setting('application_name')`,
//! * re-parse the traceparent and tracestate via
//!   `ai_blaise_citus_sidecar_shared::parse_application_name`,
//! * project the parsed traceparent into the dedicated `trace.parent` and
//!   `trace.state` session GUCs via `SET LOCAL`, so any subsequent companion
//!   pgrx function can pick it up with `current_setting('trace.parent', true)`,
//!   and
//! * emit an OpenTelemetry span anchored to the trace, with attributes that
//!   match the per-sidecar log schema (see `log_schema` in the shared crate).
//!
//! This module describes the plan as data — concrete pgrx wiring lives behind
//! the `pg18` feature flag in the bin crate and is exercised by the SQL
//! extension smoke test. The plan validation here keeps the contract honest
//! during unit tests run without pgrx.

// FEATURE: O14

use std::error::Error;
use std::fmt;

use ai_blaise_citus_sidecar_shared::{
    parse_application_name, ApplicationNameFields, SetLocalBuilder, TraceContext,
    PG_TRACE_PARENT_GUC,
};

/// Logical plan for the companion-side traceparent projection. The plan
/// records the SQL identifiers companion pgrx code uses so the smoke test
/// can verify them without running the extension.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct CompanionTraceContextPlan {
    /// Name of the companion SQL function that returns the current
    /// traceparent for the session. Defaults to `companion.current_traceparent`.
    pub current_traceparent_function: String,
    /// Name of the companion SQL function that returns the current
    /// tracestate. Defaults to `companion.current_tracestate`.
    pub current_tracestate_function: String,
    /// Name of the companion SQL function that, when called, parses
    /// `application_name` and emits the `SET LOCAL` statements that project
    /// the parsed traceparent and tracestate into the dedicated GUCs.
    pub project_traceparent_function: String,
    /// Name of the GUC the projection writes the traceparent to.
    pub traceparent_guc: String,
    /// Name of the GUC the projection writes the tracestate to.
    pub tracestate_guc: String,
}

impl CompanionTraceContextPlan {
    pub fn canonical() -> Self {
        Self {
            current_traceparent_function: "companion.current_traceparent".to_string(),
            current_tracestate_function: "companion.current_tracestate".to_string(),
            project_traceparent_function: "companion.project_traceparent_from_application_name"
                .to_string(),
            traceparent_guc: PG_TRACE_PARENT_GUC.to_string(),
            tracestate_guc: "trace.state".to_string(),
        }
    }

    pub fn validate(&self) -> Result<(), CompanionTraceContextError> {
        validate_required(
            "current_traceparent_function",
            &self.current_traceparent_function,
        )?;
        validate_required(
            "current_tracestate_function",
            &self.current_tracestate_function,
        )?;
        validate_required(
            "project_traceparent_function",
            &self.project_traceparent_function,
        )?;
        validate_required("traceparent_guc", &self.traceparent_guc)?;
        validate_required("tracestate_guc", &self.tracestate_guc)?;

        if self.traceparent_guc == self.tracestate_guc {
            return Err(CompanionTraceContextError::CollidingGucs);
        }

        Ok(())
    }
}

/// Apply the projection to a parsed `application_name` and render SQL.
///
/// Returns `None` when the application name did not embed a traceparent; the
/// companion pgrx function returns a noop in that case so PostgreSQL session
/// settings stay clean.
pub fn render_projection_sql(application_name: &str) -> Option<String> {
    let fields: ApplicationNameFields = parse_application_name(application_name);
    let mut builder = SetLocalBuilder::new();
    if let (Some(traceparent), state) = (
        fields.traceparent.as_ref(),
        fields.tracestate.clone().unwrap_or_default(),
    ) {
        builder.inject(traceparent, &state);
        builder.render()
    } else {
        None
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum CompanionTraceContextError {
    MissingRequiredField(&'static str),
    CollidingGucs,
}

impl fmt::Display for CompanionTraceContextError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingRequiredField(field) => {
                write!(formatter, "{field} must not be empty")
            }
            Self::CollidingGucs => {
                write!(formatter, "traceparent_guc must differ from tracestate_guc")
            }
        }
    }
}

impl Error for CompanionTraceContextError {}

fn validate_required(field: &'static str, value: &str) -> Result<(), CompanionTraceContextError> {
    if value.trim().is_empty() {
        return Err(CompanionTraceContextError::MissingRequiredField(field));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const TRACEPARENT: &str = "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01";

    #[test]
    fn canonical_plan_validates() {
        let plan = CompanionTraceContextPlan::canonical();
        plan.validate().unwrap();
    }

    #[test]
    fn validate_rejects_empty_function_names() {
        let mut plan = CompanionTraceContextPlan::canonical();
        plan.project_traceparent_function = "  ".to_string();
        assert_eq!(
            plan.validate().unwrap_err(),
            CompanionTraceContextError::MissingRequiredField("project_traceparent_function"),
        );
    }

    #[test]
    fn validate_rejects_colliding_gucs() {
        let mut plan = CompanionTraceContextPlan::canonical();
        plan.tracestate_guc = plan.traceparent_guc.clone();
        assert_eq!(
            plan.validate().unwrap_err(),
            CompanionTraceContextError::CollidingGucs,
        );
    }

    #[test]
    fn render_projection_sql_returns_none_when_no_traceparent_present() {
        assert!(render_projection_sql("psql").is_none());
    }

    #[test]
    fn render_projection_sql_emits_set_local_for_parsed_traceparent() {
        let application_name =
            format!("application=svc;traceparent={TRACEPARENT};tracestate=vendor=opaque");
        let sql = render_projection_sql(&application_name).unwrap();
        assert!(sql.starts_with("SET LOCAL trace.parent ="));
        assert!(sql.contains(TRACEPARENT));
        assert!(sql.contains("SET LOCAL trace.state = 'vendor=opaque'"));
    }
}
