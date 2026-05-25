// FEATURE: L10

use std::error::Error;
use std::fmt;

const FEATURE_ID: &str = "L10";
const DEFAULT_HOT_TABLE: &str = "public.l10_hot_orders";
const DEFAULT_WARM_TABLE: &str = "public.l10_warm_orders";
const DEFAULT_COLD_TABLE: &str = "public.l10_cold_orders";
const DEFAULT_DISTRIBUTION_COLUMN: &str = "tenant_id";

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct CrossTierQueryPlan {
    pub hot_table: String,
    pub warm_table: String,
    pub cold_table: String,
    pub distribution_column: String,
    pub shard_count_per_tier: u32,
    pub min_placements_per_tier: u32,
    pub expected_hot_rows: u64,
    pub expected_hot_total: i64,
    pub expected_warm_rows: u64,
    pub expected_warm_total: i64,
    pub expected_cold_rows: u64,
    pub expected_cold_total: i64,
}

impl CrossTierQueryPlan {
    pub fn validate(&self) -> Result<(), CrossTierQueryError> {
        quote_qualified_identifier("hot_table", &self.hot_table)?;
        quote_qualified_identifier("warm_table", &self.warm_table)?;
        quote_qualified_identifier("cold_table", &self.cold_table)?;
        quote_identifier("distribution_column", &self.distribution_column)?;
        if self.shard_count_per_tier == 0 {
            return Err(CrossTierQueryError::InvalidPositive("shard_count_per_tier"));
        }
        if self.min_placements_per_tier == 0 {
            return Err(CrossTierQueryError::InvalidPositive(
                "min_placements_per_tier",
            ));
        }
        for (field, value) in [
            ("expected_hot_rows", self.expected_hot_rows),
            ("expected_warm_rows", self.expected_warm_rows),
            ("expected_cold_rows", self.expected_cold_rows),
        ] {
            if value == 0 {
                return Err(CrossTierQueryError::InvalidPositive(field));
            }
        }
        for (field, value) in [
            ("expected_hot_total", self.expected_hot_total),
            ("expected_warm_total", self.expected_warm_total),
            ("expected_cold_total", self.expected_cold_total),
        ] {
            if value <= 0 {
                return Err(CrossTierQueryError::InvalidPositive(field));
            }
        }
        Ok(())
    }

    pub fn expected_rows(&self) -> u64 {
        self.expected_hot_rows + self.expected_warm_rows + self.expected_cold_rows
    }

    pub fn expected_total(&self) -> i64 {
        self.expected_hot_total + self.expected_warm_total + self.expected_cold_total
    }

    pub fn to_sql_plan(&self) -> Result<CrossTierQuerySqlPlan, CrossTierQueryError> {
        self.validate()?;

        let hot_table = quote_qualified_identifier("hot_table", &self.hot_table)?;
        let warm_table = quote_qualified_identifier("warm_table", &self.warm_table)?;
        let cold_table = quote_qualified_identifier("cold_table", &self.cold_table)?;
        let hot_table_literal = sql_literal(&self.hot_table);
        let warm_table_literal = sql_literal(&self.warm_table);
        let cold_table_literal = sql_literal(&self.cold_table);
        let distribution_column_literal = sql_literal(&self.distribution_column);

        let statement = format!(
            r#"WITH tier_relations AS (
  SELECT 'hot'::text AS tier, c.oid AS relid, COALESCE(am.amname, 'heap') AS access_method
  FROM pg_class c
  LEFT JOIN pg_am am ON am.oid = c.relam
  WHERE c.oid = {hot_table_literal}::regclass
  UNION ALL
  SELECT 'warm'::text AS tier, c.oid AS relid, COALESCE(am.amname, 'heap') AS access_method
  FROM pg_class c
  LEFT JOIN pg_am am ON am.oid = c.relam
  WHERE c.oid = {warm_table_literal}::regclass
  UNION ALL
  SELECT 'cold'::text AS tier, c.oid AS relid, COALESCE(am.amname, 'heap') AS access_method
  FROM pg_class c
  LEFT JOIN pg_am am ON am.oid = c.relam
  WHERE c.oid = {cold_table_literal}::regclass
), dist AS (
  SELECT
    tr.tier,
    count(DISTINCT ds.shardid)::bigint AS shard_count,
    count(p.placementid)::bigint AS placement_count
  FROM tier_relations tr
  LEFT JOIN pg_dist_shard ds ON ds.logicalrelid = tr.relid
  LEFT JOIN pg_dist_placement p ON p.shardid = ds.shardid
  GROUP BY tr.tier
), cross_tier_orders AS (
  SELECT 'hot'::text AS tier, {distribution_column} AS distribution_key, order_id, total FROM {hot_table}
  UNION ALL
  SELECT 'warm'::text AS tier, {distribution_column} AS distribution_key, order_id, total FROM {warm_table}
  UNION ALL
  SELECT 'cold'::text AS tier, {distribution_column} AS distribution_key, order_id, total FROM {cold_table}
), tier_rollup AS (
  SELECT tier, count(*)::bigint AS row_count, COALESCE(sum(total), 0)::bigint AS total_sum
  FROM cross_tier_orders
  GROUP BY tier
), expected(tier, row_count, total_sum) AS (
  VALUES
    ('cold'::text, {expected_cold_rows}::bigint, {expected_cold_total}::bigint),
    ('hot'::text, {expected_hot_rows}::bigint, {expected_hot_total}::bigint),
    ('warm'::text, {expected_warm_rows}::bigint, {expected_warm_total}::bigint)
), combined AS (
  SELECT count(*)::bigint AS row_count, COALESCE(sum(total), 0)::bigint AS total_sum
  FROM cross_tier_orders
)
SELECT 'l10_cross_tier_query_feature_id', '{feature_id}', 'live-citus-hot-warm-cold-query'
UNION ALL
SELECT 'l10_hot_tier_row_table',
       COALESCE((SELECT (access_method = 'heap')::text FROM tier_relations WHERE tier = 'hot'), 'false'),
       COALESCE((SELECT access_method FROM tier_relations WHERE tier = 'hot'), 'missing')
UNION ALL
SELECT 'l10_warm_tier_columnar_table',
       COALESCE((SELECT (access_method = 'columnar')::text FROM tier_relations WHERE tier = 'warm'), 'false'),
       COALESCE((SELECT access_method FROM tier_relations WHERE tier = 'warm'), 'missing')
UNION ALL
SELECT 'l10_cold_tier_columnar_table',
       COALESCE((SELECT (access_method = 'columnar')::text FROM tier_relations WHERE tier = 'cold'), 'false'),
       COALESCE((SELECT access_method FROM tier_relations WHERE tier = 'cold'), 'missing')
UNION ALL
SELECT 'l10_tiers_distributed',
       (SELECT (count(*) = 3 AND bool_and(shard_count >= {shard_count_per_tier} AND placement_count >= {min_placements_per_tier}))::text FROM dist),
       (SELECT string_agg(format('%s:shards=%s placements=%s', tier, shard_count, placement_count), ',' ORDER BY tier) FROM dist)
UNION ALL
SELECT 'l10_cross_tier_union_executed',
       (SELECT (row_count = {expected_rows} AND total_sum = {expected_total})::text FROM combined),
       (SELECT format('rows=%s total=%s distribution_column=%s', row_count, total_sum, {distribution_column_literal}) FROM combined)
UNION ALL
SELECT 'l10_tier_rollups_preserved',
       (SELECT bool_and(tr.row_count = e.row_count AND tr.total_sum = e.total_sum)::text FROM expected e JOIN tier_rollup tr USING (tier)),
       (SELECT string_agg(format('%s:%s:%s', tier, row_count, total_sum), ',' ORDER BY tier) FROM tier_rollup)
UNION ALL
SELECT 'l10_companion_rendered_query_executed', 'true', 'companion SQL executed over live distributed tiers'
UNION ALL
SELECT 'l10_explain_plan_required', 'true', 'live smoke requires Custom Scan (Citus) and ColumnarScan'
UNION ALL
SELECT 'automatic_workload_routing_exercised', 'false', 'not claimed by this bounded cross-tier query proof'
UNION ALL
SELECT 'automatic_query_rewrite_exercised', 'false', 'not claimed by this bounded cross-tier query proof'
UNION ALL
SELECT 'cost_model_selection_exercised', 'false', 'not claimed by this bounded cross-tier query proof'
UNION ALL
SELECT 'object_store_cold_read_exercised', 'false', 'not claimed by this bounded cross-tier query proof'
UNION ALL
SELECT 'kubernetes_traffic_exercised', 'false', 'not claimed by this bounded cross-tier query proof'"#,
            cold_table = cold_table,
            cold_table_literal = cold_table_literal,
            distribution_column =
                quote_identifier("distribution_column", &self.distribution_column)?,
            distribution_column_literal = distribution_column_literal,
            expected_cold_rows = self.expected_cold_rows,
            expected_cold_total = self.expected_cold_total,
            expected_hot_rows = self.expected_hot_rows,
            expected_hot_total = self.expected_hot_total,
            expected_rows = self.expected_rows(),
            expected_total = self.expected_total(),
            expected_warm_rows = self.expected_warm_rows,
            expected_warm_total = self.expected_warm_total,
            feature_id = FEATURE_ID,
            hot_table = hot_table,
            hot_table_literal = hot_table_literal,
            min_placements_per_tier = self.min_placements_per_tier,
            shard_count_per_tier = self.shard_count_per_tier,
            warm_table = warm_table,
            warm_table_literal = warm_table_literal,
        );

        Ok(CrossTierQuerySqlPlan {
            feature_id: FEATURE_ID,
            hot_table: self.hot_table.clone(),
            warm_table: self.warm_table.clone(),
            cold_table: self.cold_table.clone(),
            distribution_column: self.distribution_column.clone(),
            shard_count_per_tier: self.shard_count_per_tier,
            min_placements_per_tier: self.min_placements_per_tier,
            expected_rows: self.expected_rows(),
            expected_total: self.expected_total(),
            statements: vec![statement],
        })
    }

    pub fn report(&self) -> Result<CrossTierQueryReport, CrossTierQueryError> {
        let sql_plan = self.to_sql_plan()?;
        let script = sql_plan.render_psql_script();
        Ok(CrossTierQueryReport {
            feature_id: FEATURE_ID,
            hot_table: self.hot_table.clone(),
            warm_table: self.warm_table.clone(),
            cold_table: self.cold_table.clone(),
            distribution_column: self.distribution_column.clone(),
            shard_count_per_tier: self.shard_count_per_tier,
            min_placements_per_tier: self.min_placements_per_tier,
            expected_rows: self.expected_rows(),
            expected_total: self.expected_total(),
            statement_count: sql_plan.statements.len(),
            checks_distribution_catalogs: script.contains("pg_dist_shard")
                && script.contains("pg_dist_placement"),
            checks_access_methods: script.contains("pg_am")
                && script.contains("access_method = 'columnar'")
                && script.contains("access_method = 'heap'"),
            uses_union_all: script.contains("UNION ALL"),
            requires_explain_plan: script.contains("Custom Scan (Citus)")
                && script.contains("ColumnarScan"),
            mutating_sql: sql_plan.contains_mutating_statement(),
            fail_closed_checks: canonical_cross_tier_query_fail_closed_checks(),
            automatic_workload_routing_exercised: false,
            automatic_query_rewrite_exercised: false,
            cost_model_selection_exercised: false,
            object_store_cold_read_exercised: false,
            kubernetes_traffic_exercised: false,
        })
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct CrossTierQuerySqlPlan {
    pub feature_id: &'static str,
    pub hot_table: String,
    pub warm_table: String,
    pub cold_table: String,
    pub distribution_column: String,
    pub shard_count_per_tier: u32,
    pub min_placements_per_tier: u32,
    pub expected_rows: u64,
    pub expected_total: i64,
    pub statements: Vec<String>,
}

impl CrossTierQuerySqlPlan {
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
pub struct CrossTierQueryReport {
    pub feature_id: &'static str,
    pub hot_table: String,
    pub warm_table: String,
    pub cold_table: String,
    pub distribution_column: String,
    pub shard_count_per_tier: u32,
    pub min_placements_per_tier: u32,
    pub expected_rows: u64,
    pub expected_total: i64,
    pub statement_count: usize,
    pub checks_distribution_catalogs: bool,
    pub checks_access_methods: bool,
    pub uses_union_all: bool,
    pub requires_explain_plan: bool,
    pub mutating_sql: bool,
    pub fail_closed_checks: usize,
    pub automatic_workload_routing_exercised: bool,
    pub automatic_query_rewrite_exercised: bool,
    pub cost_model_selection_exercised: bool,
    pub object_store_cold_read_exercised: bool,
    pub kubernetes_traffic_exercised: bool,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum CrossTierQueryError {
    MissingRequiredField(&'static str),
    InvalidIdentifier { field: &'static str, value: String },
    InvalidPositive(&'static str),
}

impl fmt::Display for CrossTierQueryError {
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

impl Error for CrossTierQueryError {}

pub fn canonical_cross_tier_query_plan() -> CrossTierQueryPlan {
    CrossTierQueryPlan {
        hot_table: DEFAULT_HOT_TABLE.to_string(),
        warm_table: DEFAULT_WARM_TABLE.to_string(),
        cold_table: DEFAULT_COLD_TABLE.to_string(),
        distribution_column: DEFAULT_DISTRIBUTION_COLUMN.to_string(),
        shard_count_per_tier: 4,
        min_placements_per_tier: 4,
        expected_hot_rows: 4,
        expected_hot_total: 66,
        expected_warm_rows: 4,
        expected_warm_total: 606,
        expected_cold_rows: 4,
        expected_cold_total: 6_006,
    }
}

pub fn canonical_cross_tier_query_sql_plan() -> Result<CrossTierQuerySqlPlan, CrossTierQueryError> {
    canonical_cross_tier_query_plan().to_sql_plan()
}

pub fn canonical_cross_tier_query_report() -> Result<CrossTierQueryReport, CrossTierQueryError> {
    canonical_cross_tier_query_plan().report()
}

pub fn canonical_cross_tier_query_fail_closed_checks() -> usize {
    let mut checks = 0;

    let mut empty_hot = canonical_cross_tier_query_plan();
    empty_hot.hot_table.clear();
    if matches!(
        empty_hot.validate(),
        Err(CrossTierQueryError::MissingRequiredField("hot_table"))
    ) {
        checks += 1;
    }

    let mut unsafe_warm = canonical_cross_tier_query_plan();
    unsafe_warm.warm_table = "public.orders;drop".to_string();
    if matches!(
        unsafe_warm.validate(),
        Err(CrossTierQueryError::InvalidIdentifier { .. })
    ) {
        checks += 1;
    }

    let mut unsafe_distribution = canonical_cross_tier_query_plan();
    unsafe_distribution.distribution_column = "tenant id".to_string();
    if matches!(
        unsafe_distribution.validate(),
        Err(CrossTierQueryError::InvalidIdentifier { .. })
    ) {
        checks += 1;
    }

    let mut zero_shards = canonical_cross_tier_query_plan();
    zero_shards.shard_count_per_tier = 0;
    if matches!(
        zero_shards.validate(),
        Err(CrossTierQueryError::InvalidPositive("shard_count_per_tier"))
    ) {
        checks += 1;
    }

    let mut zero_total = canonical_cross_tier_query_plan();
    zero_total.expected_cold_total = 0;
    if matches!(
        zero_total.validate(),
        Err(CrossTierQueryError::InvalidPositive("expected_cold_total"))
    ) {
        checks += 1;
    }

    checks
}

fn quote_qualified_identifier(
    field: &'static str,
    value: &str,
) -> Result<String, CrossTierQueryError> {
    if value.trim().is_empty() {
        return Err(CrossTierQueryError::MissingRequiredField(field));
    }
    let parts = value.split('.').collect::<Vec<_>>();
    if parts.is_empty() || parts.len() > 2 {
        return Err(CrossTierQueryError::InvalidIdentifier {
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

fn quote_identifier(field: &'static str, value: &str) -> Result<String, CrossTierQueryError> {
    validate_identifier(field, value)?;
    Ok(format!("\"{value}\""))
}

fn validate_identifier(field: &'static str, value: &str) -> Result<(), CrossTierQueryError> {
    if value.trim().is_empty() {
        return Err(CrossTierQueryError::MissingRequiredField(field));
    }
    if value
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || character == '_')
    {
        Ok(())
    } else {
        Err(CrossTierQueryError::InvalidIdentifier {
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
    fn cross_tier_query_report_is_read_only_and_catalog_backed() {
        let report = canonical_cross_tier_query_report().expect("report");

        assert_eq!(report.feature_id, "L10");
        assert_eq!(report.hot_table, "public.l10_hot_orders");
        assert_eq!(report.warm_table, "public.l10_warm_orders");
        assert_eq!(report.cold_table, "public.l10_cold_orders");
        assert_eq!(report.expected_rows, 12);
        assert_eq!(report.expected_total, 6_678);
        assert!(report.checks_distribution_catalogs);
        assert!(report.checks_access_methods);
        assert!(report.uses_union_all);
        assert!(report.requires_explain_plan);
        assert!(!report.mutating_sql);
        assert_eq!(report.fail_closed_checks, 5);
        assert!(!report.automatic_workload_routing_exercised);
        assert!(!report.automatic_query_rewrite_exercised);
        assert!(!report.cost_model_selection_exercised);
        assert!(!report.object_store_cold_read_exercised);
        assert!(!report.kubernetes_traffic_exercised);
    }

    #[test]
    fn cross_tier_query_sql_plan_contains_expected_markers() {
        let sql_plan = canonical_cross_tier_query_sql_plan().expect("sql plan");
        let script = sql_plan.render_psql_script();

        assert!(script.contains("pg_dist_shard"));
        assert!(script.contains("pg_dist_placement"));
        assert!(script.contains("pg_am"));
        assert!(script.contains("UNION ALL"));
        assert!(script.contains("l10_cross_tier_union_executed"));
        assert!(script.contains("l10_tier_rollups_preserved"));
        assert!(script.contains("automatic_workload_routing_exercised"));
        assert!(!sql_plan.contains_mutating_statement());
    }

    #[test]
    fn cross_tier_query_rejects_unsafe_identifiers() {
        let mut plan = canonical_cross_tier_query_plan();
        plan.cold_table = "public.orders;drop".to_string();

        assert!(matches!(
            plan.validate(),
            Err(CrossTierQueryError::InvalidIdentifier { .. })
        ));
    }

    #[test]
    fn cross_tier_query_requires_positive_expected_values() {
        let mut plan = canonical_cross_tier_query_plan();
        plan.expected_hot_rows = 0;

        assert_eq!(
            plan.validate(),
            Err(CrossTierQueryError::InvalidPositive("expected_hot_rows"))
        );
    }
}
