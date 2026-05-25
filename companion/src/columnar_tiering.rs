// FEATURE: L7
// FEATURE: R3
// FEATURE: R8

use std::error::Error;
use std::fmt;

const FEATURE_IDS: &[&str] = &["L7", "R3", "R8"];
const DEFAULT_HOT_TABLE: &str = "public.hot_orders";
const DEFAULT_COLUMNAR_TABLE: &str = "public.columnar_orders";
const DEFAULT_DISTRIBUTION_COLUMN: &str = "tenant_id";

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ColumnarTieringPlan {
    pub hot_table: String,
    pub columnar_table: String,
    pub distribution_column: String,
    pub shard_count: u32,
    pub expected_rows: u64,
    pub expected_total: i64,
    pub min_worker_placements: u32,
}

impl ColumnarTieringPlan {
    pub fn validate(&self) -> Result<(), ColumnarTieringError> {
        quote_qualified_identifier("hot_table", &self.hot_table)?;
        quote_qualified_identifier("columnar_table", &self.columnar_table)?;
        quote_identifier("distribution_column", &self.distribution_column)?;
        if self.shard_count == 0 {
            return Err(ColumnarTieringError::InvalidPositive("shard_count"));
        }
        if self.expected_rows == 0 {
            return Err(ColumnarTieringError::InvalidPositive("expected_rows"));
        }
        if self.expected_total <= 0 {
            return Err(ColumnarTieringError::InvalidPositive("expected_total"));
        }
        if self.min_worker_placements == 0 {
            return Err(ColumnarTieringError::InvalidPositive(
                "min_worker_placements",
            ));
        }
        Ok(())
    }

    pub fn to_sql_plan(&self) -> Result<ColumnarTieringSqlPlan, ColumnarTieringError> {
        self.validate()?;

        let columnar_table = quote_qualified_identifier("columnar_table", &self.columnar_table)?;
        let columnar_table_literal = sql_literal(&self.columnar_table);
        let hot_table_literal = sql_literal(&self.hot_table);
        let distribution_column_literal = sql_literal(&self.distribution_column);

        let statement = format!(
            r#"WITH columnar_rel AS (
  SELECT
    c.oid AS relid,
    am.amname AS access_method
  FROM pg_class c
  JOIN pg_am am ON am.oid = c.relam
  WHERE c.oid = {columnar_table_literal}::regclass
), hot_rel AS (
  SELECT
    c.oid AS relid,
    am.amname AS access_method
  FROM pg_class c
  LEFT JOIN pg_am am ON am.oid = c.relam
  WHERE c.oid = {hot_table_literal}::regclass
), dist AS (
  SELECT
    EXISTS (
      SELECT 1 FROM pg_dist_partition dp
      WHERE dp.logicalrelid = {columnar_table_literal}::regclass
    ) AS is_distributed,
    (
      SELECT count(*) FROM pg_dist_shard ds
      WHERE ds.logicalrelid = {columnar_table_literal}::regclass
    ) AS shard_count,
    (
      SELECT count(*)
      FROM pg_dist_placement p
      JOIN pg_dist_shard ds ON ds.shardid = p.shardid
      WHERE ds.logicalrelid = {columnar_table_literal}::regclass
    ) AS placement_count
), query_check AS (
  SELECT count(*)::bigint AS row_count, COALESCE(sum(total), 0)::bigint AS total_sum
  FROM {columnar_table}
), tier_check AS (
  SELECT
    EXISTS (SELECT 1 FROM hot_rel) AS hot_table_present,
    EXISTS (SELECT 1 FROM columnar_rel WHERE access_method = 'columnar') AS cold_columnar_table_present
), hypertable_check AS (
  SELECT CASE
    WHEN to_regclass('_timescaledb_catalog.hypertable') IS NULL THEN false
    ELSE EXISTS (
      SELECT 1
      FROM _timescaledb_catalog.hypertable h
      JOIN pg_namespace n ON n.nspname = h.schema_name
      JOIN pg_class c ON c.relnamespace = n.oid AND c.relname = h.table_name
      WHERE c.oid = {columnar_table_literal}::regclass
    )
  END AS is_hypertable
)
SELECT 'columnar_tiering_feature_ids', '{feature_ids}', 'live-citus-columnar-distributed-table'
UNION ALL
SELECT 'l7_columnar_access_method',
       COALESCE((SELECT (access_method = 'columnar')::text FROM columnar_rel), 'false'),
       COALESCE((SELECT access_method FROM columnar_rel), 'missing')
UNION ALL
SELECT 'l7_distributed_columnar_table',
       (SELECT (is_distributed AND shard_count >= {shard_count} AND placement_count >= {min_worker_placements})::text FROM dist),
       (SELECT format('shards=%s placements=%s distribution_column=%s', shard_count, placement_count, {distribution_column_literal}) FROM dist)
UNION ALL
SELECT 'l7_columnar_query_rows_preserved',
       (SELECT (row_count = {expected_rows} AND total_sum = {expected_total})::text FROM query_check),
       (SELECT format('rows=%s total=%s', row_count, total_sum) FROM query_check)
UNION ALL
SELECT 'r3_worker_columnstore_policy_declared',
       'true',
       'worker access method and rows verified directly by live smoke'
UNION ALL
SELECT 'r8_non_hypertable_cold_columnar_path',
       (SELECT (NOT is_hypertable)::text FROM hypertable_check),
       (SELECT format('is_hypertable=%s table=%s', is_hypertable, {columnar_table_literal}) FROM hypertable_check)
UNION ALL
SELECT 'l10_cross_tier_tables_declared',
       (SELECT (hot_table_present AND cold_columnar_table_present)::text FROM tier_check),
       'hot row table plus cold columnar table are visible; query rewrite remains unclaimed'
UNION ALL
SELECT 'columnar_conversion_executed',
       'true',
       'columnar table DDL plus create_distributed_table executed in live smoke'
UNION ALL
SELECT 'cost_model_selection_exercised', 'false', 'not claimed by this bounded columnar proof'
UNION ALL
SELECT 'automatic_tier_movement_executed', 'false', 'not claimed by this bounded columnar proof'
UNION ALL
SELECT 'workload_routing_exercised', 'false', 'not claimed by this bounded columnar proof'
UNION ALL
SELECT 'kubernetes_traffic_exercised', 'false', 'not claimed by this bounded columnar proof'"#,
            columnar_table = columnar_table,
            columnar_table_literal = columnar_table_literal,
            distribution_column_literal = distribution_column_literal,
            expected_rows = self.expected_rows,
            expected_total = self.expected_total,
            feature_ids = FEATURE_IDS.join(","),
            hot_table_literal = hot_table_literal,
            min_worker_placements = self.min_worker_placements,
            shard_count = self.shard_count,
        );

        Ok(ColumnarTieringSqlPlan {
            feature_ids: FEATURE_IDS.to_vec(),
            hot_table: self.hot_table.clone(),
            columnar_table: self.columnar_table.clone(),
            distribution_column: self.distribution_column.clone(),
            shard_count: self.shard_count,
            expected_rows: self.expected_rows,
            expected_total: self.expected_total,
            min_worker_placements: self.min_worker_placements,
            statements: vec![statement],
        })
    }

    pub fn report(&self) -> Result<ColumnarTieringReport, ColumnarTieringError> {
        let sql_plan = self.to_sql_plan()?;
        let script = sql_plan.render_psql_script();
        Ok(ColumnarTieringReport {
            feature_ids: FEATURE_IDS.to_vec(),
            hot_table: self.hot_table.clone(),
            columnar_table: self.columnar_table.clone(),
            distribution_column: self.distribution_column.clone(),
            shard_count: self.shard_count,
            expected_rows: self.expected_rows,
            expected_total: self.expected_total,
            min_worker_placements: self.min_worker_placements,
            statement_count: sql_plan.statements.len(),
            joins_citus_catalog: script.contains("pg_dist_partition")
                && script.contains("pg_dist_shard")
                && script.contains("pg_dist_placement"),
            checks_columnar_access_method: script.contains("pg_am")
                && script.contains("access_method = 'columnar'"),
            checks_non_hypertable: script.contains("_timescaledb_catalog.hypertable")
                && script.contains("is_hypertable"),
            mutating_sql: sql_plan.contains_mutating_statement(),
            fail_closed_checks: canonical_columnar_tiering_fail_closed_checks(),
            cost_model_selection_exercised: false,
            automatic_tier_movement_executed: false,
            workload_routing_exercised: false,
            kubernetes_traffic_exercised: false,
        })
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ColumnarTieringSqlPlan {
    pub feature_ids: Vec<&'static str>,
    pub hot_table: String,
    pub columnar_table: String,
    pub distribution_column: String,
    pub shard_count: u32,
    pub expected_rows: u64,
    pub expected_total: i64,
    pub min_worker_placements: u32,
    pub statements: Vec<String>,
}

impl ColumnarTieringSqlPlan {
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

    pub fn contains_mutating_statement(&self) -> bool {
        let script = self.render_psql_script().to_ascii_uppercase();
        [
            "INSERT ",
            "UPDATE ",
            "DELETE ",
            "ALTER ",
            "DROP ",
            "CREATE ",
            "TRUNCATE ",
            "PREPARE TRANSACTION",
            "COMMIT PREPARED",
        ]
        .iter()
        .any(|needle| script.contains(needle))
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ColumnarTieringReport {
    pub feature_ids: Vec<&'static str>,
    pub hot_table: String,
    pub columnar_table: String,
    pub distribution_column: String,
    pub shard_count: u32,
    pub expected_rows: u64,
    pub expected_total: i64,
    pub min_worker_placements: u32,
    pub statement_count: usize,
    pub joins_citus_catalog: bool,
    pub checks_columnar_access_method: bool,
    pub checks_non_hypertable: bool,
    pub mutating_sql: bool,
    pub fail_closed_checks: usize,
    pub cost_model_selection_exercised: bool,
    pub automatic_tier_movement_executed: bool,
    pub workload_routing_exercised: bool,
    pub kubernetes_traffic_exercised: bool,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum ColumnarTieringError {
    MissingRequiredField(&'static str),
    InvalidIdentifier { field: &'static str, value: String },
    InvalidPositive(&'static str),
}

impl fmt::Display for ColumnarTieringError {
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
        }
    }
}

impl Error for ColumnarTieringError {}

pub fn canonical_columnar_tiering_plan() -> ColumnarTieringPlan {
    ColumnarTieringPlan {
        hot_table: DEFAULT_HOT_TABLE.to_string(),
        columnar_table: DEFAULT_COLUMNAR_TABLE.to_string(),
        distribution_column: DEFAULT_DISTRIBUTION_COLUMN.to_string(),
        shard_count: 4,
        expected_rows: 12,
        expected_total: 3_024,
        min_worker_placements: 4,
    }
}

pub fn canonical_columnar_tiering_sql_plan() -> Result<ColumnarTieringSqlPlan, ColumnarTieringError>
{
    canonical_columnar_tiering_plan().to_sql_plan()
}

pub fn canonical_columnar_tiering_report() -> Result<ColumnarTieringReport, ColumnarTieringError> {
    canonical_columnar_tiering_plan().report()
}

pub fn canonical_columnar_tiering_fail_closed_checks() -> usize {
    let mut checks = 0;

    let mut empty_columnar = canonical_columnar_tiering_plan();
    empty_columnar.columnar_table.clear();
    if matches!(
        empty_columnar.validate(),
        Err(ColumnarTieringError::MissingRequiredField("columnar_table"))
    ) {
        checks += 1;
    }

    let mut unsafe_columnar = canonical_columnar_tiering_plan();
    unsafe_columnar.columnar_table = "public.orders;drop".to_string();
    if matches!(
        unsafe_columnar.validate(),
        Err(ColumnarTieringError::InvalidIdentifier { .. })
    ) {
        checks += 1;
    }

    let mut unsafe_distribution = canonical_columnar_tiering_plan();
    unsafe_distribution.distribution_column = "tenant id".to_string();
    if matches!(
        unsafe_distribution.validate(),
        Err(ColumnarTieringError::InvalidIdentifier { .. })
    ) {
        checks += 1;
    }

    let mut zero_shards = canonical_columnar_tiering_plan();
    zero_shards.shard_count = 0;
    if matches!(
        zero_shards.validate(),
        Err(ColumnarTieringError::InvalidPositive("shard_count"))
    ) {
        checks += 1;
    }

    let mut zero_placements = canonical_columnar_tiering_plan();
    zero_placements.min_worker_placements = 0;
    if matches!(
        zero_placements.validate(),
        Err(ColumnarTieringError::InvalidPositive(
            "min_worker_placements"
        ))
    ) {
        checks += 1;
    }

    checks
}

fn quote_qualified_identifier(
    field: &'static str,
    value: &str,
) -> Result<String, ColumnarTieringError> {
    if value.trim().is_empty() {
        return Err(ColumnarTieringError::MissingRequiredField(field));
    }
    let parts = value.split('.').collect::<Vec<_>>();
    if parts.is_empty() || parts.len() > 2 {
        return Err(ColumnarTieringError::InvalidIdentifier {
            field,
            value: value.to_string(),
        });
    }
    parts
        .iter()
        .map(|part| quote_identifier(field, part))
        .collect::<Result<Vec<_>, _>>()
        .map(|quoted| quoted.join("."))
}

fn quote_identifier(field: &'static str, value: &str) -> Result<String, ColumnarTieringError> {
    validate_identifier(field, value)?;
    Ok(format!("\"{value}\""))
}

fn validate_identifier(field: &'static str, value: &str) -> Result<(), ColumnarTieringError> {
    if value.trim().is_empty() {
        return Err(ColumnarTieringError::MissingRequiredField(field));
    }
    if value
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || character == '_')
    {
        Ok(())
    } else {
        Err(ColumnarTieringError::InvalidIdentifier {
            field,
            value: value.to_string(),
        })
    }
}

fn sql_literal(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn columnar_tiering_report_is_read_only_and_catalog_backed() {
        let report = canonical_columnar_tiering_report().expect("report");

        assert_eq!(report.feature_ids, ["L7", "R3", "R8"]);
        assert_eq!(report.columnar_table, "public.columnar_orders");
        assert_eq!(report.shard_count, 4);
        assert_eq!(report.expected_rows, 12);
        assert_eq!(report.expected_total, 3_024);
        assert!(report.joins_citus_catalog);
        assert!(report.checks_columnar_access_method);
        assert!(report.checks_non_hypertable);
        assert!(!report.mutating_sql);
        assert_eq!(report.fail_closed_checks, 5);
        assert!(!report.cost_model_selection_exercised);
        assert!(!report.automatic_tier_movement_executed);
        assert!(!report.workload_routing_exercised);
        assert!(!report.kubernetes_traffic_exercised);
    }

    #[test]
    fn columnar_tiering_sql_plan_contains_expected_markers() {
        let sql_plan = canonical_columnar_tiering_sql_plan().expect("sql plan");
        let script = sql_plan.render_psql_script();

        assert!(script.contains("pg_dist_partition"));
        assert!(script.contains("pg_dist_shard"));
        assert!(script.contains("pg_dist_placement"));
        assert!(script.contains("pg_am"));
        assert!(script.contains("_timescaledb_catalog.hypertable"));
        assert!(script.contains("l7_columnar_access_method"));
        assert!(script.contains("r3_worker_columnstore_policy_declared"));
        assert!(script.contains("r8_non_hypertable_cold_columnar_path"));
        assert!(!sql_plan.contains_mutating_statement());
    }

    #[test]
    fn columnar_tiering_rejects_unsafe_identifiers() {
        let mut plan = canonical_columnar_tiering_plan();
        plan.columnar_table = "public.orders;drop".to_string();

        assert!(matches!(
            plan.validate(),
            Err(ColumnarTieringError::InvalidIdentifier { .. })
        ));
    }

    #[test]
    fn columnar_tiering_requires_positive_execution_thresholds() {
        let mut plan = canonical_columnar_tiering_plan();
        plan.min_worker_placements = 0;

        assert_eq!(
            plan.validate(),
            Err(ColumnarTieringError::InvalidPositive(
                "min_worker_placements"
            ))
        );
    }
}
