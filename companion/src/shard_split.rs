// FEATURE: S1

use std::error::Error;
use std::fmt;

const FEATURE_ID: &str = "S1";
const DEFAULT_TABLE: &str = "public.s1_orders";
const DEFAULT_INITIAL_SHARD_COUNT: u32 = 4;
const DEFAULT_TENANT_ID: i64 = 4;
const DEFAULT_TRANSFER_MODE: &str = "block_writes";
const DEFAULT_CASCADE_OPTION: &str = "CASCADE";

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ShardSplitPlan {
    pub table_name: String,
    pub tenant_column: String,
    pub order_column: String,
    pub amount_column: String,
    pub tenant_id: i64,
    pub initial_shard_count: u32,
    pub cascade_option: String,
    pub shard_transfer_mode: String,
}

impl ShardSplitPlan {
    pub fn validate(&self) -> Result<(), ShardSplitError> {
        validate_qualified_identifier("table_name", &self.table_name)?;
        validate_identifier("tenant_column", &self.tenant_column)?;
        validate_identifier("order_column", &self.order_column)?;
        validate_identifier("amount_column", &self.amount_column)?;
        if self.tenant_id <= 0 {
            return Err(ShardSplitError::InvalidPositive("tenant_id"));
        }
        if self.initial_shard_count == 0 {
            return Err(ShardSplitError::InvalidPositive("initial_shard_count"));
        }
        validate_cascade_option(&self.cascade_option)?;
        validate_transfer_mode(&self.shard_transfer_mode)?;
        Ok(())
    }

    pub fn to_sql_plan(&self) -> Result<ShardSplitSqlPlan, ShardSplitError> {
        self.validate()?;
        let table = quote_qualified_identifier("table_name", &self.table_name)?;
        let tenant_column = quote_identifier("tenant_column", &self.tenant_column)?;
        let table_literal = sql_literal(&self.table_name);
        let cascade_literal = sql_literal(&self.cascade_option);
        let transfer_mode_literal = sql_literal(&self.shard_transfer_mode);
        let tenant_id = self.tenant_id;
        let statements = vec![
            "DROP TABLE IF EXISTS pg_temp.ai_blaise_s1_split_observations".to_string(),
            "CREATE TEMP TABLE ai_blaise_s1_split_observations (ordinal integer PRIMARY KEY, marker text NOT NULL UNIQUE, value text NOT NULL, detail text NOT NULL DEFAULT '')".to_string(),
            "INSERT INTO ai_blaise_s1_split_observations (ordinal, marker, value, detail) SELECT 10, 'split_wal_level', current_setting('wal_level'), ''".to_string(),
            format!(
                "INSERT INTO ai_blaise_s1_split_observations (ordinal, marker, value, detail)\nSELECT 20, 'split_shard_count_before', count(*)::text, ''\nFROM pg_dist_shard\nWHERE logicalrelid = {table_literal}::regclass"
            ),
            format!(
                "INSERT INTO ai_blaise_s1_split_observations (ordinal, marker, value, detail)\nSELECT 30, 'split_tenant_rows_before', count(*)::text, ''\nFROM {table}\nWHERE {tenant_column} = {tenant_id}"
            ),
            format!(
                "INSERT INTO ai_blaise_s1_split_observations (ordinal, marker, value, detail)\nSELECT 40, 'split_tenant_shard_before', get_shard_id_for_distribution_column({table_literal}, {tenant_id})::text, ''"
            ),
            "DROP TABLE IF EXISTS pg_temp.ai_blaise_s1_split_result".to_string(),
            "CREATE TEMP TABLE ai_blaise_s1_split_result (new_shard_id bigint NOT NULL)".to_string(),
            format!(
                "INSERT INTO ai_blaise_s1_split_result (new_shard_id)\nSELECT isolate_tenant_to_new_shard({table_literal}::regclass, {tenant_id}, {cascade_literal}, {transfer_mode_literal})::bigint"
            ),
            "INSERT INTO ai_blaise_s1_split_observations (ordinal, marker, value, detail) SELECT 50, 'split_new_shard_id', new_shard_id::text, '' FROM ai_blaise_s1_split_result".to_string(),
            format!(
                "INSERT INTO ai_blaise_s1_split_observations (ordinal, marker, value, detail)\nSELECT 60, 'split_shard_count_after', count(*)::text, ''\nFROM pg_dist_shard\nWHERE logicalrelid = {table_literal}::regclass"
            ),
            format!(
                "INSERT INTO ai_blaise_s1_split_observations (ordinal, marker, value, detail)\nSELECT 70, 'split_tenant_rows_after', count(*)::text, ''\nFROM {table}\nWHERE {tenant_column} = {tenant_id}"
            ),
            format!(
                "INSERT INTO ai_blaise_s1_split_observations (ordinal, marker, value, detail)\nSELECT 80, 'split_tenant_shard_after', get_shard_id_for_distribution_column({table_literal}, {tenant_id})::text, ''"
            ),
            "INSERT INTO ai_blaise_s1_split_observations (ordinal, marker, value, detail)\nSELECT 90, 'split_tenant_shard_changed',\n  ((SELECT value FROM ai_blaise_s1_split_observations WHERE marker = 'split_tenant_shard_before') <>\n   (SELECT value FROM ai_blaise_s1_split_observations WHERE marker = 'split_tenant_shard_after'))::text, ''".to_string(),
            format!(
                "INSERT INTO ai_blaise_s1_split_observations (ordinal, marker, value, detail)\nSELECT 100, 'split_isolated_range_exact', (shardminvalue = shardmaxvalue)::text, shardminvalue || ':' || shardmaxvalue\nFROM pg_dist_shard\nWHERE logicalrelid = {table_literal}::regclass\n  AND shardid = (SELECT new_shard_id FROM ai_blaise_s1_split_result)"
            ),
            "INSERT INTO ai_blaise_s1_split_observations (ordinal, marker, value, detail) VALUES (110, 'policy_scheduler_exercised', 'false', '')".to_string(),
            "INSERT INTO ai_blaise_s1_split_observations (ordinal, marker, value, detail) VALUES (120, 'threshold_telemetry_exercised', 'false', '')".to_string(),
            "INSERT INTO ai_blaise_s1_split_observations (ordinal, marker, value, detail) VALUES (130, 'rollback_automation_exercised', 'false', '')".to_string(),
            "INSERT INTO ai_blaise_s1_split_observations (ordinal, marker, value, detail) VALUES (140, 'multi_node_movement_exercised', 'false', '')".to_string(),
            "INSERT INTO ai_blaise_s1_split_observations (ordinal, marker, value, detail) VALUES (150, 'kubernetes_traffic_exercised', 'false', '')".to_string(),
            "SELECT marker, value, detail FROM ai_blaise_s1_split_observations ORDER BY ordinal".to_string(),
        ];

        Ok(ShardSplitSqlPlan {
            feature_id: FEATURE_ID,
            statements,
            table_name: self.table_name.clone(),
            tenant_id: self.tenant_id,
            initial_shard_count: self.initial_shard_count,
            cascade_option: self.cascade_option.clone(),
            shard_transfer_mode: self.shard_transfer_mode.clone(),
        })
    }

    pub fn report(&self) -> Result<ShardSplitReport, ShardSplitError> {
        let sql_plan = self.to_sql_plan()?;
        let script = sql_plan.render_psql_script();
        Ok(ShardSplitReport {
            feature_id: FEATURE_ID,
            table_name: self.table_name.clone(),
            tenant_id: self.tenant_id,
            initial_shard_count: self.initial_shard_count,
            statement_count: sql_plan.statements.len(),
            uses_isolate_tenant_to_new_shard: script.contains("isolate_tenant_to_new_shard"),
            requires_logical_wal: script.contains("current_setting('wal_level')"),
            records_shard_count_before_after: script.contains("split_shard_count_before")
                && script.contains("split_shard_count_after"),
            records_row_preservation: script.contains("split_tenant_rows_before")
                && script.contains("split_tenant_rows_after"),
            records_isolated_range: script.contains("split_isolated_range_exact"),
            fail_closed_checks: canonical_shard_split_fail_closed_checks(),
            policy_scheduler_exercised: false,
            threshold_telemetry_exercised: false,
            rollback_automation_exercised: false,
            multi_node_movement_exercised: false,
            kubernetes_traffic_exercised: false,
        })
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ShardSplitSqlPlan {
    pub feature_id: &'static str,
    pub statements: Vec<String>,
    pub table_name: String,
    pub tenant_id: i64,
    pub initial_shard_count: u32,
    pub cascade_option: String,
    pub shard_transfer_mode: String,
}

impl ShardSplitSqlPlan {
    pub fn render_psql_script(&self) -> String {
        self.statements
            .iter()
            .map(|statement| {
                if statement.ends_with(';') {
                    statement.clone()
                } else {
                    format!("{statement};")
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ShardSplitReport {
    pub feature_id: &'static str,
    pub table_name: String,
    pub tenant_id: i64,
    pub initial_shard_count: u32,
    pub statement_count: usize,
    pub uses_isolate_tenant_to_new_shard: bool,
    pub requires_logical_wal: bool,
    pub records_shard_count_before_after: bool,
    pub records_row_preservation: bool,
    pub records_isolated_range: bool,
    pub fail_closed_checks: usize,
    pub policy_scheduler_exercised: bool,
    pub threshold_telemetry_exercised: bool,
    pub rollback_automation_exercised: bool,
    pub multi_node_movement_exercised: bool,
    pub kubernetes_traffic_exercised: bool,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum ShardSplitError {
    MissingRequiredField(&'static str),
    InvalidIdentifier { field: &'static str, value: String },
    InvalidPositive(&'static str),
    UnsupportedCascadeOption(String),
    UnsupportedTransferMode(String),
}

impl fmt::Display for ShardSplitError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingRequiredField(field) => write!(formatter, "{field} is required"),
            Self::InvalidIdentifier { field, value } => {
                write!(
                    formatter,
                    "{field} is not a safe PostgreSQL identifier: {value}"
                )
            }
            Self::InvalidPositive(field) => write!(formatter, "{field} must be greater than zero"),
            Self::UnsupportedCascadeOption(value) => {
                write!(formatter, "unsupported cascade option: {value}")
            }
            Self::UnsupportedTransferMode(value) => {
                write!(formatter, "unsupported shard transfer mode: {value}")
            }
        }
    }
}

impl Error for ShardSplitError {}

pub fn canonical_shard_split_plan() -> ShardSplitPlan {
    ShardSplitPlan {
        table_name: DEFAULT_TABLE.to_string(),
        tenant_column: "tenant_id".to_string(),
        order_column: "order_id".to_string(),
        amount_column: "total".to_string(),
        tenant_id: DEFAULT_TENANT_ID,
        initial_shard_count: DEFAULT_INITIAL_SHARD_COUNT,
        cascade_option: DEFAULT_CASCADE_OPTION.to_string(),
        shard_transfer_mode: DEFAULT_TRANSFER_MODE.to_string(),
    }
}

pub fn canonical_shard_split_sql_plan() -> Result<ShardSplitSqlPlan, ShardSplitError> {
    canonical_shard_split_plan().to_sql_plan()
}

pub fn canonical_shard_split_report() -> Result<ShardSplitReport, ShardSplitError> {
    canonical_shard_split_plan().report()
}

pub fn canonical_shard_split_fail_closed_checks() -> usize {
    let mut checks = 0;

    let mut missing_table = canonical_shard_split_plan();
    missing_table.table_name.clear();
    if matches!(
        missing_table.validate(),
        Err(ShardSplitError::MissingRequiredField("table_name"))
    ) {
        checks += 1;
    }

    let mut unsafe_table = canonical_shard_split_plan();
    unsafe_table.table_name = "public.s1_orders;drop".to_string();
    if matches!(
        unsafe_table.validate(),
        Err(ShardSplitError::InvalidIdentifier { .. })
    ) {
        checks += 1;
    }

    let mut zero_tenant = canonical_shard_split_plan();
    zero_tenant.tenant_id = 0;
    if matches!(
        zero_tenant.validate(),
        Err(ShardSplitError::InvalidPositive("tenant_id"))
    ) {
        checks += 1;
    }

    let mut zero_shards = canonical_shard_split_plan();
    zero_shards.initial_shard_count = 0;
    if matches!(
        zero_shards.validate(),
        Err(ShardSplitError::InvalidPositive("initial_shard_count"))
    ) {
        checks += 1;
    }

    let mut unsafe_transfer = canonical_shard_split_plan();
    unsafe_transfer.shard_transfer_mode = "force;drop".to_string();
    if matches!(
        unsafe_transfer.validate(),
        Err(ShardSplitError::UnsupportedTransferMode(_))
    ) {
        checks += 1;
    }

    let mut unsafe_cascade = canonical_shard_split_plan();
    unsafe_cascade.cascade_option = "CASCADE;drop".to_string();
    if matches!(
        unsafe_cascade.validate(),
        Err(ShardSplitError::UnsupportedCascadeOption(_))
    ) {
        checks += 1;
    }

    checks
}

fn quote_qualified_identifier(field: &'static str, value: &str) -> Result<String, ShardSplitError> {
    validate_qualified_identifier(field, value)?;
    value
        .split('.')
        .map(|part| quote_identifier(field, part))
        .collect::<Result<Vec<_>, _>>()
        .map(|parts| parts.join("."))
}

fn validate_qualified_identifier(field: &'static str, value: &str) -> Result<(), ShardSplitError> {
    if value.trim().is_empty() {
        return Err(ShardSplitError::MissingRequiredField(field));
    }
    let parts = value.split('.').collect::<Vec<_>>();
    if parts.is_empty() || parts.len() > 2 {
        return Err(ShardSplitError::InvalidIdentifier {
            field,
            value: value.to_string(),
        });
    }
    for part in parts {
        validate_identifier(field, part)?;
    }
    Ok(())
}

fn quote_identifier(field: &'static str, value: &str) -> Result<String, ShardSplitError> {
    validate_identifier(field, value)?;
    Ok(format!("\"{value}\""))
}

fn validate_identifier(field: &'static str, value: &str) -> Result<(), ShardSplitError> {
    if value.trim().is_empty() {
        return Err(ShardSplitError::MissingRequiredField(field));
    }
    if value.len() > 63
        || value
            .chars()
            .next()
            .is_some_and(|character| !(character == '_' || character.is_ascii_alphabetic()))
        || !value
            .chars()
            .all(|character| character == '_' || character.is_ascii_alphanumeric())
    {
        return Err(ShardSplitError::InvalidIdentifier {
            field,
            value: value.to_string(),
        });
    }
    Ok(())
}

fn validate_cascade_option(value: &str) -> Result<(), ShardSplitError> {
    match value {
        "CASCADE" | "RESTRICT" => Ok(()),
        _ => Err(ShardSplitError::UnsupportedCascadeOption(value.to_string())),
    }
}

fn validate_transfer_mode(value: &str) -> Result<(), ShardSplitError> {
    match value {
        "block_writes" | "force_logical" | "auto" => Ok(()),
        _ => Err(ShardSplitError::UnsupportedTransferMode(value.to_string())),
    }
}

fn sql_literal(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_report_covers_live_shard_split_boundary() {
        let report = canonical_shard_split_report().expect("report");

        assert_eq!(report.feature_id, "S1");
        assert_eq!(report.table_name, DEFAULT_TABLE);
        assert_eq!(report.tenant_id, 4);
        assert_eq!(report.initial_shard_count, 4);
        assert_eq!(report.statement_count, 21);
        assert!(report.uses_isolate_tenant_to_new_shard);
        assert!(report.requires_logical_wal);
        assert!(report.records_shard_count_before_after);
        assert!(report.records_row_preservation);
        assert!(report.records_isolated_range);
        assert_eq!(report.fail_closed_checks, 6);
        assert!(!report.policy_scheduler_exercised);
        assert!(!report.threshold_telemetry_exercised);
        assert!(!report.rollback_automation_exercised);
        assert!(!report.multi_node_movement_exercised);
        assert!(!report.kubernetes_traffic_exercised);
    }

    #[test]
    fn sql_plan_renders_isolate_tenant_call_and_markers() {
        let sql_plan = canonical_shard_split_sql_plan().expect("sql plan");
        let script = sql_plan.render_psql_script();

        assert!(script.contains("isolate_tenant_to_new_shard('public.s1_orders'::regclass, 4, 'CASCADE', 'block_writes')"));
        assert!(script.contains("split_shard_count_before"));
        assert!(script.contains("split_shard_count_after"));
        assert!(script.contains("split_tenant_rows_before"));
        assert!(script.contains("split_tenant_rows_after"));
        assert!(script.contains("split_isolated_range_exact"));
        assert!(script.contains("policy_scheduler_exercised"));
    }

    #[test]
    fn rejects_unsafe_table_identifier() {
        let mut plan = canonical_shard_split_plan();
        plan.table_name = "public.s1_orders;drop".to_string();

        assert!(matches!(
            plan.validate(),
            Err(ShardSplitError::InvalidIdentifier { .. })
        ));
    }

    #[test]
    fn rejects_unsupported_transfer_mode() {
        let mut plan = canonical_shard_split_plan();
        plan.shard_transfer_mode = "force;drop".to_string();

        assert_eq!(
            plan.validate(),
            Err(ShardSplitError::UnsupportedTransferMode(
                "force;drop".to_string()
            ))
        );
    }

    #[test]
    fn rejects_unsupported_cascade_option() {
        let mut plan = canonical_shard_split_plan();
        plan.cascade_option = "CASCADE;drop".to_string();

        assert_eq!(
            plan.validate(),
            Err(ShardSplitError::UnsupportedCascadeOption(
                "CASCADE;drop".to_string()
            ))
        );
    }
}
