// FEATURE: R7

use std::error::Error;
use std::fmt;

use crate::crds::scheduled_repack::{
    RepackStrategy, ScheduledRepackSpec, ScheduledRepackSpecError,
};

pub const REPACK_POLICY_TABLE: &str = "companion_internal.scheduled_repack_policies";
pub const REPACK_QUEUE_TABLE: &str = "companion_internal.repack_queue";
pub const PG_CRON_SCHEDULE_FUNCTION: &str = "cron.schedule";
pub const PG_CRON_UNSCHEDULE_FUNCTION: &str = "cron.unschedule";

const REPACK_TABLES_SQL: &str = r#"CREATE TABLE IF NOT EXISTS companion_internal.scheduled_repack_policies (
    job_name text PRIMARY KEY,
    target_table text NOT NULL,
    cron_schedule text NOT NULL,
    strategy text NOT NULL,
    payload jsonb NOT NULL,
    updated_at timestamptz NOT NULL DEFAULT now()
);
CREATE TABLE IF NOT EXISTS companion_internal.repack_queue (
    request_id bigserial PRIMARY KEY,
    job_name text NOT NULL REFERENCES companion_internal.scheduled_repack_policies(job_name) ON DELETE CASCADE,
    target_table text NOT NULL,
    strategy text NOT NULL,
    payload jsonb NOT NULL,
    enqueued_at timestamptz NOT NULL DEFAULT now(),
    started_at timestamptz,
    completed_at timestamptz,
    error text
);"#;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ScheduledRepackReconcilePlan {
    pub job_name: String,
    pub target: String,
    pub schedule: String,
    pub strategy: RepackStrategy,
}

impl ScheduledRepackReconcilePlan {
    pub fn from_spec(
        repack_name: &str,
        spec: &ScheduledRepackSpec,
    ) -> Result<Self, ScheduledRepackReconcileError> {
        let trimmed_name = repack_name.trim();
        if trimmed_name.is_empty() {
            return Err(ScheduledRepackReconcileError::MissingRepackName);
        }
        spec.validate()?;

        Ok(Self {
            job_name: format!("ai-blaise-citus-repack-{}", sanitize_name(trimmed_name)),
            target: spec.target.clone(),
            schedule: spec.schedule.clone(),
            strategy: spec.strategy,
        })
    }

    pub fn strategy_str(&self) -> &'static str {
        match self.strategy {
            RepackStrategy::PgRepack => "pg_repack",
            RepackStrategy::RepackConcurrentlyPg19 => "repack_concurrently_pg19",
        }
    }

    pub fn payload_json(&self) -> String {
        format!(
            "{{\"target\":\"{target}\",\"strategy\":\"{strategy}\",\"job\":\"{job}\"}}",
            target = escape_json(&self.target),
            strategy = self.strategy_str(),
            job = escape_json(&self.job_name),
        )
    }

    pub fn apply_plan(&self) -> ScheduledRepackApplyPlan {
        ScheduledRepackApplyPlan {
            steps: vec![
                ScheduledRepackApplyStep::new(
                    "ensure_ai_blaise_citus_extension",
                    "CREATE EXTENSION IF NOT EXISTS ai_blaise_citus;",
                    true,
                ),
                ScheduledRepackApplyStep::new(
                    "ensure_pg_cron_extension",
                    "CREATE EXTENSION IF NOT EXISTS pg_cron;",
                    true,
                ),
                ScheduledRepackApplyStep::new(
                    "ensure_repack_policy_tables",
                    REPACK_TABLES_SQL,
                    true,
                ),
                ScheduledRepackApplyStep::new(
                    "upsert_scheduled_repack_policy",
                    upsert_policy_sql(self),
                    true,
                ),
                ScheduledRepackApplyStep::new(
                    "schedule_repack_queue_job",
                    schedule_queue_job_sql(self),
                    true,
                ),
            ],
        }
    }

    pub fn apply_sql_script(&self) -> String {
        self.apply_plan().sql_script()
    }

    pub fn teardown_sql(&self) -> String {
        format!(
            r#"DO $ai_blaise_repack_teardown$
BEGIN
    IF EXISTS (SELECT 1 FROM cron.job WHERE jobname = {job}) THEN
        PERFORM {unschedule}({job});
    END IF;
    DELETE FROM {policy_table} WHERE job_name = {job};
END
$ai_blaise_repack_teardown$;"#,
            job = sql_literal(&self.job_name),
            unschedule = PG_CRON_UNSCHEDULE_FUNCTION,
            policy_table = REPACK_POLICY_TABLE,
        )
    }

    pub fn bloat_estimate_sql(&self) -> String {
        format!(
            r#"SELECT coalesce(sum(n_dead_tup), 0)::bigint AS dead_tuple_estimate
FROM pg_catalog.pg_stat_user_tables
WHERE schemaname || '.' || relname = {target};"#,
            target = sql_literal(&self.target),
        )
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ScheduledRepackApplyStep {
    pub name: String,
    pub sql: String,
    pub idempotent: bool,
}

impl ScheduledRepackApplyStep {
    fn new(name: impl Into<String>, sql: impl Into<String>, idempotent: bool) -> Self {
        Self {
            name: name.into(),
            sql: sql.into(),
            idempotent,
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ScheduledRepackApplyPlan {
    pub steps: Vec<ScheduledRepackApplyStep>,
}

impl ScheduledRepackApplyPlan {
    pub fn sql_script(&self) -> String {
        self.steps
            .iter()
            .map(|step| step.sql.trim_end())
            .collect::<Vec<_>>()
            .join("\n")
    }
}

fn upsert_policy_sql(plan: &ScheduledRepackReconcilePlan) -> String {
    format!(
        r#"INSERT INTO {table}(job_name, target_table, cron_schedule, strategy, payload)
VALUES ({job}, {target}, {schedule}, {strategy}, {payload}::jsonb)
ON CONFLICT (job_name) DO UPDATE
SET target_table = EXCLUDED.target_table,
    cron_schedule = EXCLUDED.cron_schedule,
    strategy = EXCLUDED.strategy,
    payload = EXCLUDED.payload,
    updated_at = now();"#,
        table = REPACK_POLICY_TABLE,
        job = sql_literal(&plan.job_name),
        target = sql_literal(&plan.target),
        schedule = sql_literal(&plan.schedule),
        strategy = sql_literal(plan.strategy_str()),
        payload = sql_literal(&plan.payload_json()),
    )
}

fn schedule_queue_job_sql(plan: &ScheduledRepackReconcilePlan) -> String {
    let command = format!(
        "INSERT INTO {queue}(job_name, target_table, strategy, payload) VALUES ({job}, {target}, {strategy}, {payload}::jsonb);",
        queue = REPACK_QUEUE_TABLE,
        job = sql_literal(&plan.job_name),
        target = sql_literal(&plan.target),
        strategy = sql_literal(plan.strategy_str()),
        payload = sql_literal(&plan.payload_json()),
    );

    format!(
        r#"DO $ai_blaise_repack_schedule$
BEGIN
    IF EXISTS (SELECT 1 FROM cron.job WHERE jobname = {job}) THEN
        PERFORM {unschedule}({job});
    END IF;
    PERFORM {schedule_function}({job}, {schedule}, {command});
END
$ai_blaise_repack_schedule$;"#,
        job = sql_literal(&plan.job_name),
        unschedule = PG_CRON_UNSCHEDULE_FUNCTION,
        schedule_function = PG_CRON_SCHEDULE_FUNCTION,
        schedule = sql_literal(&plan.schedule),
        command = sql_literal(&command),
    )
}

fn sanitize_name(raw: &str) -> String {
    let sanitized = raw
        .trim()
        .chars()
        .map(|character| match character {
            'A'..='Z' => character.to_ascii_lowercase(),
            'a'..='z' | '0'..='9' | '-' => character,
            _ => '-',
        })
        .collect::<String>()
        .trim_matches('-')
        .to_string();
    if sanitized.is_empty() {
        "repack".to_string()
    } else {
        sanitized
    }
}

fn sql_literal(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

fn escape_json(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            character if (character as u32) < 0x20 => {
                escaped.push_str(&format!("\\u{:04x}", character as u32));
            }
            character => escaped.push(character),
        }
    }
    escaped
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum ScheduledRepackReconcileError {
    InvalidSpec(ScheduledRepackSpecError),
    MissingRepackName,
}

impl fmt::Display for ScheduledRepackReconcileError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSpec(error) => write!(formatter, "{error}"),
            Self::MissingRepackName => write!(formatter, "repack_name must not be empty"),
        }
    }
}

impl Error for ScheduledRepackReconcileError {}

impl From<ScheduledRepackSpecError> for ScheduledRepackReconcileError {
    fn from(error: ScheduledRepackSpecError) -> Self {
        Self::InvalidSpec(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pg_repack_plan_renders_queue_policy_and_cron_sql() {
        let spec = ScheduledRepackSpec {
            target: "public.orders".to_string(),
            schedule: "0 3 * * 0".to_string(),
            strategy: RepackStrategy::PgRepack,
        };

        let plan =
            ScheduledRepackReconcilePlan::from_spec("weekly-orders", &spec).expect("valid plan");

        assert_eq!(plan.job_name, "ai-blaise-citus-repack-weekly-orders");
        assert_eq!(plan.strategy_str(), "pg_repack");
        assert!(plan.payload_json().contains("\"target\":\"public.orders\""));

        let apply_plan = plan.apply_plan();
        assert_eq!(apply_plan.steps.len(), 5);
        assert!(apply_plan.sql_script().contains(REPACK_POLICY_TABLE));
        assert!(apply_plan.sql_script().contains(REPACK_QUEUE_TABLE));
        assert!(apply_plan.sql_script().contains(PG_CRON_SCHEDULE_FUNCTION));
        assert!(apply_plan.sql_script().contains("'0 3 * * 0'"));

        let teardown = plan.teardown_sql();
        assert!(teardown.contains(PG_CRON_UNSCHEDULE_FUNCTION));
        assert!(teardown.contains(REPACK_POLICY_TABLE));

        let bloat = plan.bloat_estimate_sql();
        assert!(bloat.contains("pg_stat_user_tables"));
    }

    #[test]
    fn pg19_strategy_renders_repack_concurrently() {
        let spec = ScheduledRepackSpec {
            target: "public.events".to_string(),
            schedule: "0 4 * * *".to_string(),
            strategy: RepackStrategy::RepackConcurrentlyPg19,
        };

        let plan =
            ScheduledRepackReconcilePlan::from_spec("events-nightly", &spec).expect("valid plan");

        assert_eq!(plan.strategy_str(), "repack_concurrently_pg19");
        assert!(plan
            .apply_sql_script()
            .contains("'repack_concurrently_pg19'"));
    }

    #[test]
    fn empty_repack_name_is_rejected() {
        let spec = ScheduledRepackSpec {
            target: "public.orders".to_string(),
            schedule: "0 3 * * 0".to_string(),
            strategy: RepackStrategy::PgRepack,
        };

        assert_eq!(
            ScheduledRepackReconcilePlan::from_spec("  ", &spec),
            Err(ScheduledRepackReconcileError::MissingRepackName)
        );
    }

    #[test]
    fn invalid_spec_propagates_validation_error() {
        let spec = ScheduledRepackSpec {
            target: String::new(),
            schedule: "0 3 * * 0".to_string(),
            strategy: RepackStrategy::PgRepack,
        };

        assert_eq!(
            ScheduledRepackReconcilePlan::from_spec("weekly", &spec),
            Err(ScheduledRepackReconcileError::InvalidSpec(
                ScheduledRepackSpecError::MissingRequiredField("target")
            ))
        );
    }

    #[test]
    fn validated_target_is_rendered_into_sql_and_json() {
        let spec = ScheduledRepackSpec {
            target: "public.orders".to_string(),
            schedule: "0 3 * * 0".to_string(),
            strategy: RepackStrategy::PgRepack,
        };

        let plan = ScheduledRepackReconcilePlan::from_spec("edge", &spec).expect("valid plan");
        assert!(plan.apply_sql_script().contains("'public.orders'"));
        assert!(plan.payload_json().contains("\"target\":\"public.orders\""));
    }
}
