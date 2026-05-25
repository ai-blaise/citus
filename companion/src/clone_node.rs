// FEATURE: S3

use std::error::Error;
use std::fmt;

const FEATURE_ID: &str = "S3";
const DEFAULT_TABLE: &str = "public.s3_orders";
const DEFAULT_SHARD_COUNT: u32 = 4;
const DEFAULT_EXPECTED_ROWS: u64 = 20;
const DEFAULT_EXPECTED_SUM: u64 = 5_060;
const DEFAULT_CATCHUP_TIMEOUT_SECONDS: u32 = 30;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct CloneNodePlan {
    pub table_name: String,
    pub tenant_column: String,
    pub order_column: String,
    pub amount_column: String,
    pub primary_worker_var: String,
    pub clone_worker_var: String,
    pub shard_count: u32,
    pub expected_rows: u64,
    pub expected_sum: u64,
    pub catchup_timeout_seconds: u32,
}

impl CloneNodePlan {
    pub fn validate(&self) -> Result<(), CloneNodeError> {
        validate_qualified_identifier("table_name", &self.table_name)?;
        validate_identifier("tenant_column", &self.tenant_column)?;
        validate_identifier("order_column", &self.order_column)?;
        validate_identifier("amount_column", &self.amount_column)?;
        validate_psql_variable("primary_worker_var", &self.primary_worker_var)?;
        validate_psql_variable("clone_worker_var", &self.clone_worker_var)?;
        if self.shard_count == 0 {
            return Err(CloneNodeError::InvalidPositive("shard_count"));
        }
        if self.expected_rows == 0 {
            return Err(CloneNodeError::InvalidPositive("expected_rows"));
        }
        if self.expected_sum == 0 {
            return Err(CloneNodeError::InvalidPositive("expected_sum"));
        }
        if self.catchup_timeout_seconds == 0 {
            return Err(CloneNodeError::InvalidPositive("catchup_timeout_seconds"));
        }
        Ok(())
    }

    pub fn to_sql_plan(&self) -> Result<CloneNodeSqlPlan, CloneNodeError> {
        self.validate()?;
        let table = quote_qualified_identifier("table_name", &self.table_name)?;
        let table_literal = sql_literal(&self.table_name);
        let tenant_column = quote_identifier("tenant_column", &self.tenant_column)?;
        let order_column = quote_identifier("order_column", &self.order_column)?;
        let amount_column = quote_identifier("amount_column", &self.amount_column)?;
        let primary_var = quote_psql_variable(&self.primary_worker_var);
        let clone_var = quote_psql_variable(&self.clone_worker_var);
        let shard_count = self.shard_count;
        let expected_rows = self.expected_rows;
        let expected_sum = self.expected_sum;
        let catchup_timeout = self.catchup_timeout_seconds;

        let setup_statements = vec![
            "DROP TABLE IF EXISTS pg_temp.ai_blaise_s3_clone_setup_observations".to_string(),
            "CREATE TEMP TABLE ai_blaise_s3_clone_setup_observations (ordinal integer PRIMARY KEY, marker text NOT NULL UNIQUE, value text NOT NULL, detail text NOT NULL DEFAULT '')".to_string(),
            "DROP TABLE IF EXISTS pg_temp.ai_blaise_s3_primary_node".to_string(),
            "CREATE TEMP TABLE ai_blaise_s3_primary_node (nodeid integer NOT NULL)".to_string(),
            format!("INSERT INTO ai_blaise_s3_primary_node (nodeid) SELECT citus_add_node(:{primary_var}, 5432)"),
            "INSERT INTO ai_blaise_s3_clone_setup_observations (ordinal, marker, value, detail) SELECT 10, 's3_primary_nodeid', nodeid::text, '' FROM ai_blaise_s3_primary_node".to_string(),
            format!("DROP TABLE IF EXISTS {table} CASCADE"),
            format!("CREATE TABLE {table}({tenant_column} integer NOT NULL, {order_column} integer NOT NULL, {amount_column} integer NOT NULL, PRIMARY KEY ({tenant_column}, {order_column}))"),
            format!("SELECT create_distributed_table({table_literal}, {tenant_column_literal}, shard_count => {shard_count})", tenant_column_literal = sql_literal(&self.tenant_column)),
            format!("INSERT INTO {table} SELECT tenant_id, order_id, tenant_id * 100 + order_id FROM generate_series(1, 4) AS tenant_id, generate_series(1, 5) AS order_id"),
            format!("INSERT INTO ai_blaise_s3_clone_setup_observations (ordinal, marker, value, detail) SELECT 20, 's3_rows_before_clone', count(*)::text, '' FROM {table}"),
            format!("INSERT INTO ai_blaise_s3_clone_setup_observations (ordinal, marker, value, detail) SELECT 30, 's3_sum_before_clone', sum({amount_column})::text, '' FROM {table}"),
            format!("INSERT INTO ai_blaise_s3_clone_setup_observations (ordinal, marker, value, detail) SELECT 40, 's3_placements_before_clone', count(*)::text, '' FROM pg_dist_shard_placement WHERE nodename = :{primary_var}"),
            format!("INSERT INTO ai_blaise_s3_clone_setup_observations (ordinal, marker, value, detail) VALUES (50, 's3_expected_rows', '{expected_rows}', '')"),
            format!("INSERT INTO ai_blaise_s3_clone_setup_observations (ordinal, marker, value, detail) VALUES (60, 's3_expected_sum', '{expected_sum}', '')"),
            "SELECT marker, value, detail FROM ai_blaise_s3_clone_setup_observations ORDER BY ordinal".to_string(),
        ];

        let promote_statements = vec![
            "DROP TABLE IF EXISTS pg_temp.ai_blaise_s3_clone_observations".to_string(),
            "CREATE TEMP TABLE ai_blaise_s3_clone_observations (ordinal integer PRIMARY KEY, marker text NOT NULL UNIQUE, value text NOT NULL, detail text NOT NULL DEFAULT '')".to_string(),
            "DROP TABLE IF EXISTS pg_temp.ai_blaise_s3_clone_result".to_string(),
            "CREATE TEMP TABLE ai_blaise_s3_clone_result (clone_nodeid integer NOT NULL)".to_string(),
            format!("INSERT INTO ai_blaise_s3_clone_result (clone_nodeid) SELECT citus_add_clone_node(:{clone_var}, 5432, :{primary_var}, 5432)"),
            "INSERT INTO ai_blaise_s3_clone_observations (ordinal, marker, value, detail) SELECT 10, 's3_clone_nodeid', clone_nodeid::text, '' FROM ai_blaise_s3_clone_result".to_string(),
            format!("INSERT INTO ai_blaise_s3_clone_observations (ordinal, marker, value, detail) SELECT 20, 's3_rows_before_promote', count(*)::text, '' FROM {table}"),
            format!("INSERT INTO ai_blaise_s3_clone_observations (ordinal, marker, value, detail) SELECT 30, 's3_sum_before_promote', sum({amount_column})::text, '' FROM {table}"),
            "INSERT INTO ai_blaise_s3_clone_observations (ordinal, marker, value, detail) SELECT 40, 's3_clone_role_before_promote', noderole::text, '' FROM pg_dist_node WHERE nodeid = (SELECT clone_nodeid FROM ai_blaise_s3_clone_result)".to_string(),
            "INSERT INTO ai_blaise_s3_clone_observations (ordinal, marker, value, detail) SELECT 50, 's3_clone_active_before_promote', isactive::text, '' FROM pg_dist_node WHERE nodeid = (SELECT clone_nodeid FROM ai_blaise_s3_clone_result)".to_string(),
            format!("SELECT citus_promote_clone_and_rebalance((SELECT clone_nodeid FROM ai_blaise_s3_clone_result), NULL, {catchup_timeout})"),
            "INSERT INTO ai_blaise_s3_clone_observations (ordinal, marker, value, detail) VALUES (60, 's3_promote_clone_and_rebalance_executed', 'true', '')".to_string(),
            format!("INSERT INTO ai_blaise_s3_clone_observations (ordinal, marker, value, detail) SELECT 70, 's3_rows_after_promote', count(*)::text, '' FROM {table}"),
            format!("INSERT INTO ai_blaise_s3_clone_observations (ordinal, marker, value, detail) SELECT 80, 's3_sum_after_promote', sum({amount_column})::text, '' FROM {table}"),
            "INSERT INTO ai_blaise_s3_clone_observations (ordinal, marker, value, detail) SELECT 90, 's3_clone_role_after_promote', noderole::text, '' FROM pg_dist_node WHERE nodeid = (SELECT clone_nodeid FROM ai_blaise_s3_clone_result)".to_string(),
            "INSERT INTO ai_blaise_s3_clone_observations (ordinal, marker, value, detail) SELECT 100, 's3_clone_active_after_promote', isactive::text, '' FROM pg_dist_node WHERE nodeid = (SELECT clone_nodeid FROM ai_blaise_s3_clone_result)".to_string(),
            "INSERT INTO ai_blaise_s3_clone_observations (ordinal, marker, value, detail) SELECT 110, 's3_clone_should_have_shards_after_promote', shouldhaveshards::text, '' FROM pg_dist_node WHERE nodeid = (SELECT clone_nodeid FROM ai_blaise_s3_clone_result)".to_string(),
            format!("INSERT INTO ai_blaise_s3_clone_observations (ordinal, marker, value, detail) SELECT 120, 's3_clone_shard_placements_after', count(*)::text, '' FROM pg_dist_shard_placement WHERE nodename = :{clone_var}"),
            format!("INSERT INTO ai_blaise_s3_clone_observations (ordinal, marker, value, detail) SELECT 130, 's3_primary_shard_placements_after', count(*)::text, '' FROM pg_dist_shard_placement WHERE nodename = :{primary_var}"),
            "INSERT INTO ai_blaise_s3_clone_observations (ordinal, marker, value, detail) VALUES (140, 'kubernetes_clone_orchestration_exercised', 'false', '')".to_string(),
            "INSERT INTO ai_blaise_s3_clone_observations (ordinal, marker, value, detail) VALUES (150, 'csi_snapshot_exercised', 'false', '')".to_string(),
            "INSERT INTO ai_blaise_s3_clone_observations (ordinal, marker, value, detail) VALUES (160, 'automatic_capacity_policy_exercised', 'false', '')".to_string(),
            "INSERT INTO ai_blaise_s3_clone_observations (ordinal, marker, value, detail) VALUES (170, 'production_traffic_cutover_exercised', 'false', '')".to_string(),
            "SELECT marker, value, detail FROM ai_blaise_s3_clone_observations ORDER BY ordinal".to_string(),
        ];

        Ok(CloneNodeSqlPlan {
            feature_id: FEATURE_ID,
            table_name: self.table_name.clone(),
            primary_worker_var: self.primary_worker_var.clone(),
            clone_worker_var: self.clone_worker_var.clone(),
            setup_statements,
            promote_statements,
        })
    }

    pub fn report(&self) -> Result<CloneNodeReport, CloneNodeError> {
        let sql_plan = self.to_sql_plan()?;
        let setup_script = sql_plan.render_setup_psql_script();
        let promote_script = sql_plan.render_promote_psql_script();
        Ok(CloneNodeReport {
            feature_id: FEATURE_ID,
            table_name: self.table_name.clone(),
            shard_count: self.shard_count,
            expected_rows: self.expected_rows,
            expected_sum: self.expected_sum,
            catchup_timeout_seconds: self.catchup_timeout_seconds,
            setup_statement_count: sql_plan.setup_statements.len(),
            promote_statement_count: sql_plan.promote_statements.len(),
            uses_citus_add_clone_node: promote_script.contains("citus_add_clone_node"),
            uses_citus_promote_clone_and_rebalance: promote_script
                .contains("citus_promote_clone_and_rebalance"),
            records_data_preservation: promote_script.contains("s3_rows_after_promote")
                && promote_script.contains("s3_sum_after_promote"),
            records_clone_metadata: promote_script.contains("s3_clone_role_after_promote")
                && promote_script.contains("s3_clone_shard_placements_after"),
            registers_primary_worker: setup_script.contains("citus_add_node"),
            fail_closed_checks: canonical_clone_node_fail_closed_checks(),
            kubernetes_clone_orchestration_exercised: false,
            csi_snapshot_exercised: false,
            automatic_capacity_policy_exercised: false,
            production_traffic_cutover_exercised: false,
        })
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct CloneNodeSqlPlan {
    pub feature_id: &'static str,
    pub table_name: String,
    pub primary_worker_var: String,
    pub clone_worker_var: String,
    pub setup_statements: Vec<String>,
    pub promote_statements: Vec<String>,
}

impl CloneNodeSqlPlan {
    pub fn render_setup_psql_script(&self) -> String {
        render_statements(&self.setup_statements)
    }

    pub fn render_promote_psql_script(&self) -> String {
        render_statements(&self.promote_statements)
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct CloneNodeReport {
    pub feature_id: &'static str,
    pub table_name: String,
    pub shard_count: u32,
    pub expected_rows: u64,
    pub expected_sum: u64,
    pub catchup_timeout_seconds: u32,
    pub setup_statement_count: usize,
    pub promote_statement_count: usize,
    pub uses_citus_add_clone_node: bool,
    pub uses_citus_promote_clone_and_rebalance: bool,
    pub records_data_preservation: bool,
    pub records_clone_metadata: bool,
    pub registers_primary_worker: bool,
    pub fail_closed_checks: usize,
    pub kubernetes_clone_orchestration_exercised: bool,
    pub csi_snapshot_exercised: bool,
    pub automatic_capacity_policy_exercised: bool,
    pub production_traffic_cutover_exercised: bool,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum CloneNodeError {
    MissingRequiredField(&'static str),
    InvalidIdentifier { field: &'static str, value: String },
    InvalidPositive(&'static str),
}

impl fmt::Display for CloneNodeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingRequiredField(field) => write!(formatter, "{field} is required"),
            Self::InvalidIdentifier { field, value } => {
                write!(formatter, "{field} is not a safe identifier: {value}")
            }
            Self::InvalidPositive(field) => write!(formatter, "{field} must be greater than zero"),
        }
    }
}

impl Error for CloneNodeError {}

pub fn canonical_clone_node_plan() -> CloneNodePlan {
    CloneNodePlan {
        table_name: DEFAULT_TABLE.to_string(),
        tenant_column: "tenant_id".to_string(),
        order_column: "order_id".to_string(),
        amount_column: "total".to_string(),
        primary_worker_var: "s3_primary_worker".to_string(),
        clone_worker_var: "s3_clone_worker".to_string(),
        shard_count: DEFAULT_SHARD_COUNT,
        expected_rows: DEFAULT_EXPECTED_ROWS,
        expected_sum: DEFAULT_EXPECTED_SUM,
        catchup_timeout_seconds: DEFAULT_CATCHUP_TIMEOUT_SECONDS,
    }
}

pub fn canonical_clone_node_sql_plan() -> Result<CloneNodeSqlPlan, CloneNodeError> {
    canonical_clone_node_plan().to_sql_plan()
}

pub fn canonical_clone_node_report() -> Result<CloneNodeReport, CloneNodeError> {
    canonical_clone_node_plan().report()
}

pub fn canonical_clone_node_fail_closed_checks() -> usize {
    let mut checks = 0;

    let mut missing_table = canonical_clone_node_plan();
    missing_table.table_name.clear();
    if matches!(
        missing_table.validate(),
        Err(CloneNodeError::MissingRequiredField("table_name"))
    ) {
        checks += 1;
    }

    let mut unsafe_table = canonical_clone_node_plan();
    unsafe_table.table_name = "public.s3_orders;drop".to_string();
    if matches!(
        unsafe_table.validate(),
        Err(CloneNodeError::InvalidIdentifier { .. })
    ) {
        checks += 1;
    }

    let mut unsafe_var = canonical_clone_node_plan();
    unsafe_var.clone_worker_var = "clone-worker;drop".to_string();
    if matches!(
        unsafe_var.validate(),
        Err(CloneNodeError::InvalidIdentifier { .. })
    ) {
        checks += 1;
    }

    let mut zero_shards = canonical_clone_node_plan();
    zero_shards.shard_count = 0;
    if matches!(
        zero_shards.validate(),
        Err(CloneNodeError::InvalidPositive("shard_count"))
    ) {
        checks += 1;
    }

    let mut zero_timeout = canonical_clone_node_plan();
    zero_timeout.catchup_timeout_seconds = 0;
    if matches!(
        zero_timeout.validate(),
        Err(CloneNodeError::InvalidPositive("catchup_timeout_seconds"))
    ) {
        checks += 1;
    }

    checks
}

fn render_statements(statements: &[String]) -> String {
    statements
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

fn validate_qualified_identifier(field: &'static str, value: &str) -> Result<(), CloneNodeError> {
    let (schema, table) = value
        .split_once('.')
        .ok_or_else(|| missing_or_invalid_identifier(field, value))?;
    validate_identifier(field, schema)?;
    validate_identifier(field, table)?;
    Ok(())
}

fn validate_identifier(field: &'static str, value: &str) -> Result<(), CloneNodeError> {
    if value.is_empty() {
        return Err(CloneNodeError::MissingRequiredField(field));
    }
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return Err(CloneNodeError::MissingRequiredField(field));
    };
    if !(first == '_' || first.is_ascii_alphabetic()) {
        return Err(CloneNodeError::InvalidIdentifier {
            field,
            value: value.to_string(),
        });
    }
    if !chars.all(|character| character == '_' || character.is_ascii_alphanumeric()) {
        return Err(CloneNodeError::InvalidIdentifier {
            field,
            value: value.to_string(),
        });
    }
    Ok(())
}

fn validate_psql_variable(field: &'static str, value: &str) -> Result<(), CloneNodeError> {
    validate_identifier(field, value)
}

fn missing_or_invalid_identifier(field: &'static str, value: &str) -> CloneNodeError {
    if value.is_empty() {
        CloneNodeError::MissingRequiredField(field)
    } else {
        CloneNodeError::InvalidIdentifier {
            field,
            value: value.to_string(),
        }
    }
}

fn quote_qualified_identifier(field: &'static str, value: &str) -> Result<String, CloneNodeError> {
    validate_qualified_identifier(field, value)?;
    let (schema, table) = value
        .split_once('.')
        .ok_or_else(|| missing_or_invalid_identifier(field, value))?;
    Ok(format!(
        "{}.{}",
        quote_identifier(field, schema)?,
        quote_identifier(field, table)?
    ))
}

fn quote_identifier(field: &'static str, value: &str) -> Result<String, CloneNodeError> {
    validate_identifier(field, value)?;
    Ok(format!("\"{value}\""))
}

fn quote_psql_variable(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

fn sql_literal(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_report_tracks_clone_primitives() {
        let report = canonical_clone_node_report().expect("report");

        assert_eq!(report.feature_id, "S3");
        assert!(report.uses_citus_add_clone_node);
        assert!(report.uses_citus_promote_clone_and_rebalance);
        assert!(report.records_data_preservation);
        assert!(report.records_clone_metadata);
        assert!(report.registers_primary_worker);
        assert_eq!(report.fail_closed_checks, 5);
        assert!(!report.kubernetes_clone_orchestration_exercised);
    }

    #[test]
    fn setup_sql_registers_primary_and_distributes_table() {
        let sql = canonical_clone_node_sql_plan()
            .expect("sql")
            .render_setup_psql_script();

        assert!(sql.contains("citus_add_node(:'s3_primary_worker', 5432)"));
        assert!(sql.contains("create_distributed_table('public.s3_orders'"));
        assert!(sql.contains("s3_rows_before_clone"));
    }

    #[test]
    fn promote_sql_registers_and_promotes_clone() {
        let sql = canonical_clone_node_sql_plan()
            .expect("sql")
            .render_promote_psql_script();

        assert!(sql.contains(
            "citus_add_clone_node(:'s3_clone_worker', 5432, :'s3_primary_worker', 5432)"
        ));
        assert!(sql.contains("citus_promote_clone_and_rebalance"));
        assert!(sql.contains("s3_clone_shard_placements_after"));
        assert!(sql.contains("production_traffic_cutover_exercised"));
    }

    #[test]
    fn fail_closed_checks_cover_unsafe_inputs() {
        assert_eq!(canonical_clone_node_fail_closed_checks(), 5);
    }
}
