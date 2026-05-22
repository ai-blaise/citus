//! Companion structured-log view plan.
//!
//! Vector ships sidecar stdout into PostgreSQL through Loki and a companion
//! foreign table at `companion.sidecar_log_raw(line jsonb, captured_at
//! timestamptz)`. This module renders the deterministic `CREATE VIEW`
//! statements that materialize per-sidecar typed views on top of that raw
//! ingestion table, driven by the canonical log schema in
//! `ai_blaise_citus_sidecar_shared::log_schema`.
//!
//! The view shapes are stable: companion SQL clients (operator dashboards,
//! companion plan-cache scrapers, the citus-watch TUI) can plan against them
//! without inspecting the raw JSON shape directly.

// FEATURE: O15

use std::error::Error;
use std::fmt;

use ai_blaise_citus_sidecar_shared::{
    canonical_sidecar_log_schemas, LogField, LogFieldKind, LogSchemaError, SidecarLogSchema,
};

/// Default raw-ingestion source table that Vector or fluent-bit populate
/// from sidecar stdout. The first column is the parsed JSON document; the
/// second is the wall-clock time the line was captured.
pub const DEFAULT_RAW_TABLE: &str = "companion.sidecar_log_raw";

/// Schema in which the typed per-sidecar views are created.
pub const DEFAULT_VIEW_SCHEMA: &str = "companion";

/// Logical plan for a single sidecar log view.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct LogViewPlan {
    pub sidecar: String,
    pub schema_name: String,
    pub raw_table: String,
    pub view_name: String,
    pub fields: Vec<LogFieldProjection>,
}

/// A single typed projection from the raw JSON document onto a SQL column.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct LogFieldProjection {
    pub column: String,
    pub kind: LogFieldKind,
    pub json_path: JsonPath,
    pub nullable: bool,
}

/// Whether the column reads from a top-level field on the JSON document or
/// the nested `fields` envelope produced by per-sidecar log extensions.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum JsonPath {
    TopLevel(String),
    Extension(String),
}

impl LogFieldProjection {
    fn sql_expression(&self) -> String {
        let raw = match &self.json_path {
            JsonPath::TopLevel(name) => format!("line ->> '{}'", escape_sql_ident(name)),
            JsonPath::Extension(name) => {
                format!("line -> 'fields' ->> '{}'", escape_sql_ident(name))
            }
        };
        match self.kind {
            LogFieldKind::Timestamp => format!("({raw})::timestamptz"),
            LogFieldKind::String => raw,
            LogFieldKind::Integer => format!("({raw})::bigint"),
            LogFieldKind::Float => format!("({raw})::double precision"),
            LogFieldKind::Bool => format!("({raw})::boolean"),
            LogFieldKind::Json => match &self.json_path {
                JsonPath::TopLevel(name) => format!("line -> '{}'", escape_sql_ident(name)),
                JsonPath::Extension(name) => {
                    format!("line -> 'fields' -> '{}'", escape_sql_ident(name))
                }
            },
        }
    }

    fn sql_column(&self) -> String {
        format!(
            "    {} AS {}",
            self.sql_expression(),
            quote_sql_ident(&self.column),
        )
    }
}

impl LogViewPlan {
    /// Construct a `LogViewPlan` from a `SidecarLogSchema`.
    pub fn from_schema(schema: &SidecarLogSchema) -> Result<Self, LogViewError> {
        schema.schema.validate().map_err(LogViewError::Schema)?;

        let mut fields: Vec<LogFieldProjection> = Vec::new();
        for field in schema.schema.common.iter() {
            fields.push(common_projection(field));
        }
        for field in schema.schema.extensions.iter() {
            fields.push(extension_projection(field));
        }

        Ok(Self {
            sidecar: schema.sidecar.to_string(),
            schema_name: DEFAULT_VIEW_SCHEMA.to_string(),
            raw_table: DEFAULT_RAW_TABLE.to_string(),
            view_name: format!("sidecar_{}_log", schema.sidecar),
            fields,
        })
    }

    /// Render the deterministic `CREATE OR REPLACE VIEW` SQL for this plan.
    pub fn render_sql(&self) -> String {
        let projections: Vec<String> = self
            .fields
            .iter()
            .map(LogFieldProjection::sql_column)
            .collect();
        format!(
            "CREATE OR REPLACE VIEW {schema}.{view} AS\nSELECT\n{projections}\nFROM {raw}\nWHERE line ->> 'sidecar' = '{sidecar}';",
            schema = quote_sql_ident(&self.schema_name),
            view = quote_sql_ident(&self.view_name),
            projections = projections.join(",\n"),
            raw = self.raw_table,
            sidecar = escape_sql_ident(&self.sidecar),
        )
    }
}

/// Render the full deterministic SQL bundle: one view per canonical sidecar
/// log schema, in stable order.
pub fn render_all_views() -> Result<String, LogViewError> {
    let mut statements: Vec<String> = Vec::new();
    for sidecar in canonical_sidecar_log_schemas() {
        let plan = LogViewPlan::from_schema(sidecar)?;
        statements.push(plan.render_sql());
    }
    Ok(statements.join("\n\n"))
}

/// Render the canonical plans (without rendering SQL). Used by deterministic
/// runners that emit TSV summaries.
pub fn canonical_log_view_plans() -> Result<Vec<LogViewPlan>, LogViewError> {
    canonical_sidecar_log_schemas()
        .iter()
        .map(LogViewPlan::from_schema)
        .collect()
}

fn common_projection(field: &LogField) -> LogFieldProjection {
    LogFieldProjection {
        column: field.name.to_string(),
        kind: field.kind,
        json_path: JsonPath::TopLevel(field.name.to_string()),
        nullable: !field.required,
    }
}

fn extension_projection(field: &LogField) -> LogFieldProjection {
    LogFieldProjection {
        column: field.name.to_string(),
        kind: field.kind,
        json_path: JsonPath::Extension(field.name.to_string()),
        nullable: !field.required,
    }
}

fn escape_sql_ident(value: &str) -> String {
    value.replace('\'', "''").replace('"', "\"\"")
}

fn quote_sql_ident(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\""))
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum LogViewError {
    Schema(LogSchemaError),
}

impl fmt::Display for LogViewError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Schema(error) => write!(formatter, "{error}"),
        }
    }
}

impl Error for LogViewError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_plans_cover_every_sidecar() {
        let plans = canonical_log_view_plans().unwrap();
        assert_eq!(plans.len(), 17);
        for plan in &plans {
            assert!(plan.view_name.starts_with("sidecar_"));
            assert!(plan.view_name.ends_with("_log"));
            assert!(plan
                .fields
                .iter()
                .any(|field| field.column == "traceparent"));
            assert_eq!(plan.schema_name, DEFAULT_VIEW_SCHEMA);
            assert_eq!(plan.raw_table, DEFAULT_RAW_TABLE);
        }
    }

    #[test]
    fn vectorizer_view_includes_extension_fields_with_typed_columns() {
        let schemas = canonical_sidecar_log_schemas();
        let vectorizer = schemas
            .iter()
            .find(|sidecar| sidecar.sidecar == "vectorizer")
            .unwrap();
        let plan = LogViewPlan::from_schema(vectorizer).unwrap();
        let sql = plan.render_sql();
        assert!(sql.contains("CREATE OR REPLACE VIEW"));
        assert!(sql.contains("\"sidecar_vectorizer_log\""));
        assert!(sql.contains("line -> 'fields' ->> 'provider' AS \"provider\""));
        assert!(sql.contains("(line -> 'fields' ->> 'tokens')::bigint AS \"tokens\""));
        assert!(sql.contains("(line -> 'fields' ->> 'cost_usd')::double precision AS \"cost_usd\""));
        assert!(sql.contains("WHERE line ->> 'sidecar' = 'vectorizer'"));
    }

    #[test]
    fn rendered_bundle_is_deterministic_and_includes_all_sidecars() {
        let bundle_a = render_all_views().unwrap();
        let bundle_b = render_all_views().unwrap();
        assert_eq!(bundle_a, bundle_b);
        assert!(bundle_a.contains("sidecar_realtime_log"));
        assert!(bundle_a.contains("sidecar_raft_log"));
        assert!(bundle_a.contains("sidecar_storage_log"));
        assert!(bundle_a.contains("sidecar_vectorizer_log"));
        // Plain count: 17 sidecars, one view each.
        let count = bundle_a.matches("CREATE OR REPLACE VIEW").count();
        assert_eq!(count, 17);
    }

    #[test]
    fn timestamp_projection_casts_to_timestamptz() {
        let schemas = canonical_sidecar_log_schemas();
        let plan = LogViewPlan::from_schema(&schemas[0]).unwrap();
        let sql = plan.render_sql();
        assert!(sql.contains("(line ->> 'timestamp')::timestamptz AS \"timestamp\""));
    }
}
