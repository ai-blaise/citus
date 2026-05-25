// FEATURE: MR3

use std::error::Error;
use std::fmt;

const FEATURE_ID: &str = "MR3";
const DEFAULT_TABLE: &str = "public.mr3_orders";
const DEFAULT_LOCALITY_COLUMN: &str = "locality_key";
const DEFAULT_TENANT_COLUMN: &str = "tenant_id";
const DEFAULT_ORDER_COLUMN: &str = "order_id";
const DEFAULT_AMOUNT_COLUMN: &str = "total";
const DEFAULT_CASCADE_OPTION: &str = "CASCADE";
const DEFAULT_TRANSFER_MODE: &str = "block_writes";

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct RegionalRowPlacementPlan {
    pub table_name: String,
    pub locality_column: String,
    pub tenant_column: String,
    pub order_column: String,
    pub amount_column: String,
    pub cascade_option: String,
    pub shard_transfer_mode: String,
    pub regional_keys: Vec<RegionalRowPlacementKey>,
}

impl RegionalRowPlacementPlan {
    pub fn validate(&self) -> Result<(), RegionalRowPlacementError> {
        validate_qualified_identifier("table_name", &self.table_name)?;
        validate_identifier("locality_column", &self.locality_column)?;
        validate_identifier("tenant_column", &self.tenant_column)?;
        validate_identifier("order_column", &self.order_column)?;
        validate_identifier("amount_column", &self.amount_column)?;
        validate_cascade_option(&self.cascade_option)?;
        validate_transfer_mode(&self.shard_transfer_mode)?;
        if self.regional_keys.len() < 2 {
            return Err(RegionalRowPlacementError::TooFewRegions);
        }

        let mut regions = Vec::new();
        let mut locality_keys = Vec::new();
        for key in &self.regional_keys {
            key.validate()?;
            if regions.iter().any(|region| region == &key.region) {
                return Err(RegionalRowPlacementError::DuplicateRegion(
                    key.region.clone(),
                ));
            }
            if locality_keys
                .iter()
                .any(|locality_key| locality_key == &key.locality_key)
            {
                return Err(RegionalRowPlacementError::DuplicateLocalityKey(
                    key.locality_key.clone(),
                ));
            }
            regions.push(key.region.clone());
            locality_keys.push(key.locality_key.clone());
        }
        Ok(())
    }

    pub fn to_sql_plan(&self) -> Result<RegionalRowPlacementSqlPlan, RegionalRowPlacementError> {
        self.validate()?;
        let table = quote_qualified_identifier("table_name", &self.table_name)?;
        let table_literal = sql_literal(&self.table_name);
        let locality_column = quote_identifier("locality_column", &self.locality_column)?;
        let amount_column = quote_identifier("amount_column", &self.amount_column)?;
        let cascade_literal = sql_literal(&self.cascade_option);
        let transfer_mode_literal = sql_literal(&self.shard_transfer_mode);
        let region_values = self
            .regional_keys
            .iter()
            .map(|key| {
                format!(
                    "({}, {}, {}, :'{}')",
                    sql_literal(&key.region),
                    sql_literal(&key.locality_key),
                    sql_literal(&key.tenant_id),
                    key.expected_worker_psql_var
                )
            })
            .collect::<Vec<_>>()
            .join(",\n  ");
        let expected_regions = self.regional_keys.len();
        let isolation_statements = self
            .regional_keys
            .iter()
            .map(|key| {
                format!(
                    "DO $$\nBEGIN\n  PERFORM isolate_tenant_to_new_shard({table_literal}::regclass, {}::text, {cascade_literal}, {transfer_mode_literal});\nEND\n$$",
                    sql_literal(&key.locality_key)
                )
            })
            .collect::<Vec<_>>();

        let mut statements = isolation_statements;
        statements.extend([
            "DROP TABLE IF EXISTS pg_temp.ai_blaise_mr3_region_keys".to_string(),
            "CREATE TEMP TABLE ai_blaise_mr3_region_keys (region text PRIMARY KEY, locality_key text NOT NULL UNIQUE, tenant_id text NOT NULL, expected_worker text NOT NULL)".to_string(),
            format!(
                "INSERT INTO ai_blaise_mr3_region_keys (region, locality_key, tenant_id, expected_worker) VALUES\n  {region_values}"
            ),
            "DROP TABLE IF EXISTS pg_temp.ai_blaise_mr3_observations".to_string(),
            "CREATE TEMP TABLE ai_blaise_mr3_observations (ordinal integer PRIMARY KEY, marker text NOT NULL UNIQUE, value text NOT NULL, detail text NOT NULL DEFAULT '')".to_string(),
            format!(
                "INSERT INTO ai_blaise_mr3_observations (ordinal, marker, value, detail) VALUES (10, 'mr3_feature_id', {feature_id}, '')",
                feature_id = sql_literal(FEATURE_ID)
            ),
            format!(
                "INSERT INTO ai_blaise_mr3_observations (ordinal, marker, value, detail)\nSELECT 20, 'mr3_region_keys', count(*)::text, string_agg(region || '=' || expected_worker, ',' ORDER BY region)\nFROM ai_blaise_mr3_region_keys\nHAVING count(*) = {expected_regions}"
            ),
            format!(
                "INSERT INTO ai_blaise_mr3_observations (ordinal, marker, value, detail)\nSELECT 30, 'mr3_shard_count_after_isolation', count(*)::text, ''\nFROM pg_dist_shard\nWHERE logicalrelid = {table_literal}::regclass"
            ),
            "DROP TABLE IF EXISTS pg_temp.ai_blaise_mr3_row_counts_before".to_string(),
            "CREATE TEMP TABLE ai_blaise_mr3_row_counts_before (region text PRIMARY KEY, rows_seen bigint NOT NULL, total numeric NOT NULL)".to_string(),
            format!(
                "DO $$\nDECLARE\n  regional_key record;\n  rows_seen bigint;\n  total_seen numeric;\nBEGIN\n  FOR regional_key IN SELECT * FROM ai_blaise_mr3_region_keys ORDER BY region LOOP\n    EXECUTE 'SELECT count(*)::bigint, COALESCE(sum({amount_column}), 0)::numeric FROM {table} WHERE {locality_column} = $1'\n      INTO rows_seen, total_seen\n      USING regional_key.locality_key;\n    INSERT INTO ai_blaise_mr3_row_counts_before (region, rows_seen, total)\n    VALUES (regional_key.region, rows_seen, total_seen);\n  END LOOP;\nEND\n$$"
            ),
            "DROP TABLE IF EXISTS pg_temp.ai_blaise_mr3_isolated_shards".to_string(),
            "CREATE TEMP TABLE ai_blaise_mr3_isolated_shards (region text PRIMARY KEY, new_shard_id bigint NOT NULL)".to_string(),
            format!(
                "INSERT INTO ai_blaise_mr3_isolated_shards (region, new_shard_id)\nSELECT region, get_shard_id_for_distribution_column({table_literal}, locality_key)::bigint\nFROM ai_blaise_mr3_region_keys\nORDER BY region"
            ),
            format!(
                "INSERT INTO ai_blaise_mr3_observations (ordinal, marker, value, detail)\nSELECT 40, 'mr3_shards_isolated', (count(*) = {expected_regions} AND count(DISTINCT new_shard_id) = {expected_regions})::text, string_agg(region || ':' || new_shard_id::text, ',' ORDER BY region)\nFROM ai_blaise_mr3_isolated_shards"
            ),
            "DROP TABLE IF EXISTS pg_temp.ai_blaise_mr3_move_results".to_string(),
            "CREATE TEMP TABLE ai_blaise_mr3_move_results (region text PRIMARY KEY, shardid bigint NOT NULL, source_worker text NOT NULL, target_worker text NOT NULL, moved boolean NOT NULL)".to_string(),
            format!(
                "DO $$\nDECLARE\n  regional_key record;\n  shard_id bigint;\n  current_worker text;\n  moved boolean;\nBEGIN\n  FOR regional_key IN SELECT * FROM ai_blaise_mr3_region_keys ORDER BY region LOOP\n    shard_id := get_shard_id_for_distribution_column({table_literal}, regional_key.locality_key);\n    SELECT n.nodename INTO current_worker\n    FROM pg_dist_placement p\n    JOIN pg_dist_node n USING(groupid)\n    WHERE p.shardid = shard_id\n      AND p.shardstate = 1\n    LIMIT 1;\n    IF current_worker IS NULL THEN\n      RAISE EXCEPTION 'MR3 shard % has no active placement', shard_id;\n    END IF;\n    moved := current_worker <> regional_key.expected_worker;\n    IF moved THEN\n      PERFORM citus_move_shard_placement(shard_id, current_worker, 5432, regional_key.expected_worker, 5432, {transfer_mode_literal});\n    END IF;\n    INSERT INTO ai_blaise_mr3_move_results (region, shardid, source_worker, target_worker, moved)\n    VALUES (regional_key.region, shard_id, current_worker, regional_key.expected_worker, moved);\n  END LOOP;\nEND\n$$"
            ),
            "INSERT INTO ai_blaise_mr3_observations (ordinal, marker, value, detail)\nSELECT 50, 'mr3_citus_move_shard_placement_executed', bool_or(moved)::text, string_agg(region || ':' || source_worker || '->' || target_worker, ',' ORDER BY region)\nFROM ai_blaise_mr3_move_results".to_string(),
            "DROP TABLE IF EXISTS pg_temp.ai_blaise_mr3_row_counts_after".to_string(),
            "CREATE TEMP TABLE ai_blaise_mr3_row_counts_after (region text PRIMARY KEY, rows_seen bigint NOT NULL, total numeric NOT NULL)".to_string(),
            format!(
                "DO $$\nDECLARE\n  regional_key record;\n  rows_seen bigint;\n  total_seen numeric;\nBEGIN\n  FOR regional_key IN SELECT * FROM ai_blaise_mr3_region_keys ORDER BY region LOOP\n    EXECUTE 'SELECT count(*)::bigint, COALESCE(sum({amount_column}), 0)::numeric FROM {table} WHERE {locality_column} = $1'\n      INTO rows_seen, total_seen\n      USING regional_key.locality_key;\n    INSERT INTO ai_blaise_mr3_row_counts_after (region, rows_seen, total)\n    VALUES (regional_key.region, rows_seen, total_seen);\n  END LOOP;\nEND\n$$"
            ),
            format!(
                "INSERT INTO ai_blaise_mr3_observations (ordinal, marker, value, detail)\nSELECT 60, 'mr3_rows_preserved', (count(*) = {expected_regions} AND bool_and(before_counts.rows_seen = after_counts.rows_seen AND before_counts.total = after_counts.total))::text, string_agg(before_counts.region || ':' || before_counts.rows_seen::text || ':' || before_counts.total::text, ',' ORDER BY before_counts.region)\nFROM ai_blaise_mr3_row_counts_before before_counts\nJOIN ai_blaise_mr3_row_counts_after after_counts USING(region)"
            ),
            "DROP TABLE IF EXISTS pg_temp.ai_blaise_mr3_placement_after".to_string(),
            format!(
                "CREATE TEMP TABLE ai_blaise_mr3_placement_after AS\nSELECT\n  regional_key.region,\n  regional_key.locality_key,\n  get_shard_id_for_distribution_column({table_literal}, regional_key.locality_key) AS shardid,\n  regional_key.expected_worker,\n  node.nodename AS observed_worker,\n  node.nodename = regional_key.expected_worker AS placement_matches\nFROM ai_blaise_mr3_region_keys regional_key\nJOIN pg_dist_placement placement\n  ON placement.shardid = get_shard_id_for_distribution_column({table_literal}, regional_key.locality_key)\nJOIN pg_dist_node node USING(groupid)\nWHERE placement.shardstate = 1"
            ),
            format!(
                "INSERT INTO ai_blaise_mr3_observations (ordinal, marker, value, detail)\nSELECT 70, 'mr3_worker_placement_enforced', (count(*) = {expected_regions} AND bool_and(placement_matches))::text, string_agg(region || ':' || observed_worker, ',' ORDER BY region)\nFROM ai_blaise_mr3_placement_after"
            ),
            "INSERT INTO ai_blaise_mr3_observations (ordinal, marker, value, detail)\nSELECT 80, 'mr3_matched_region_count', count(*)::text, string_agg(region || ':' || shardid::text, ',' ORDER BY region)\nFROM ai_blaise_mr3_placement_after\nWHERE placement_matches".to_string(),
        ]);

        statements.extend([
            "INSERT INTO ai_blaise_mr3_observations (ordinal, marker, value, detail) VALUES (90, 'mr3_automatic_repartition_scheduler_exercised', 'false', '')".to_string(),
            "INSERT INTO ai_blaise_mr3_observations (ordinal, marker, value, detail) VALUES (100, 'mr3_kubernetes_operator_reconciliation_exercised', 'false', '')".to_string(),
            "INSERT INTO ai_blaise_mr3_observations (ordinal, marker, value, detail) VALUES (110, 'mr3_regional_traffic_router_exercised', 'false', '')".to_string(),
            "INSERT INTO ai_blaise_mr3_observations (ordinal, marker, value, detail) VALUES (120, 'mr3_multi_region_network_exercised', 'false', '')".to_string(),
            "INSERT INTO ai_blaise_mr3_observations (ordinal, marker, value, detail) VALUES (130, 'mr3_regional_failover_exercised', 'false', '')".to_string(),
            "SELECT marker, value, detail FROM ai_blaise_mr3_observations ORDER BY ordinal".to_string(),
        ]);

        Ok(RegionalRowPlacementSqlPlan {
            feature_id: FEATURE_ID,
            statements,
            table_name: self.table_name.clone(),
            regional_key_count: self.regional_keys.len(),
        })
    }

    pub fn report(&self) -> Result<RegionalRowPlacementReport, RegionalRowPlacementError> {
        let sql_plan = self.to_sql_plan()?;
        let script = sql_plan.render_psql_script();
        Ok(RegionalRowPlacementReport {
            feature_id: FEATURE_ID,
            table_name: self.table_name.clone(),
            locality_column: self.locality_column.clone(),
            regional_key_count: self.regional_keys.len(),
            statement_count: sql_plan.statements.len(),
            uses_isolate_tenant_to_new_shard: script.contains("isolate_tenant_to_new_shard"),
            uses_citus_move_shard_placement: script.contains("citus_move_shard_placement"),
            records_row_preservation: script.contains("mr3_rows_preserved"),
            records_worker_placement: script.contains("mr3_worker_placement_enforced"),
            requires_worker_psql_variables: self
                .regional_keys
                .iter()
                .all(|key| script.contains(&format!(":'{}'", key.expected_worker_psql_var))),
            fail_closed_checks: canonical_regional_row_placement_fail_closed_checks(),
            automatic_repartition_scheduler_exercised: false,
            kubernetes_operator_reconciliation_exercised: false,
            regional_traffic_router_exercised: false,
            multi_region_network_exercised: false,
            regional_failover_exercised: false,
        })
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct RegionalRowPlacementKey {
    pub region: String,
    pub locality_key: String,
    pub tenant_id: String,
    pub expected_worker_psql_var: String,
}

impl RegionalRowPlacementKey {
    pub fn new(
        region: &str,
        locality_key: &str,
        tenant_id: &str,
        expected_worker_psql_var: &str,
    ) -> Self {
        Self {
            region: region.to_string(),
            locality_key: locality_key.to_string(),
            tenant_id: tenant_id.to_string(),
            expected_worker_psql_var: expected_worker_psql_var.to_string(),
        }
    }

    fn validate(&self) -> Result<(), RegionalRowPlacementError> {
        validate_region(&self.region)?;
        validate_non_empty("locality_key", &self.locality_key)?;
        validate_non_empty("tenant_id", &self.tenant_id)?;
        validate_psql_variable("expected_worker_psql_var", &self.expected_worker_psql_var)
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct RegionalRowPlacementSqlPlan {
    pub feature_id: &'static str,
    pub statements: Vec<String>,
    pub table_name: String,
    pub regional_key_count: usize,
}

impl RegionalRowPlacementSqlPlan {
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
pub struct RegionalRowPlacementReport {
    pub feature_id: &'static str,
    pub table_name: String,
    pub locality_column: String,
    pub regional_key_count: usize,
    pub statement_count: usize,
    pub uses_isolate_tenant_to_new_shard: bool,
    pub uses_citus_move_shard_placement: bool,
    pub records_row_preservation: bool,
    pub records_worker_placement: bool,
    pub requires_worker_psql_variables: bool,
    pub fail_closed_checks: usize,
    pub automatic_repartition_scheduler_exercised: bool,
    pub kubernetes_operator_reconciliation_exercised: bool,
    pub regional_traffic_router_exercised: bool,
    pub multi_region_network_exercised: bool,
    pub regional_failover_exercised: bool,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum RegionalRowPlacementError {
    MissingRequiredField(&'static str),
    InvalidIdentifier { field: &'static str, value: String },
    InvalidRegion(String),
    InvalidPsqlVariable { field: &'static str, value: String },
    UnsupportedCascadeOption(String),
    UnsupportedTransferMode(String),
    TooFewRegions,
    DuplicateRegion(String),
    DuplicateLocalityKey(String),
}

impl fmt::Display for RegionalRowPlacementError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingRequiredField(field) => write!(formatter, "{field} is required"),
            Self::InvalidIdentifier { field, value } => {
                write!(formatter, "{field} is not a safe identifier: {value}")
            }
            Self::InvalidRegion(region) => write!(formatter, "invalid region: {region}"),
            Self::InvalidPsqlVariable { field, value } => {
                write!(formatter, "{field} is not a safe psql variable: {value}")
            }
            Self::UnsupportedCascadeOption(value) => {
                write!(formatter, "unsupported cascade option: {value}")
            }
            Self::UnsupportedTransferMode(value) => {
                write!(formatter, "unsupported shard transfer mode: {value}")
            }
            Self::TooFewRegions => write!(formatter, "at least two regions are required"),
            Self::DuplicateRegion(region) => write!(formatter, "duplicate region: {region}"),
            Self::DuplicateLocalityKey(locality_key) => {
                write!(formatter, "duplicate locality key: {locality_key}")
            }
        }
    }
}

impl Error for RegionalRowPlacementError {}

pub fn canonical_regional_row_placement_plan() -> RegionalRowPlacementPlan {
    RegionalRowPlacementPlan {
        table_name: DEFAULT_TABLE.to_string(),
        locality_column: DEFAULT_LOCALITY_COLUMN.to_string(),
        tenant_column: DEFAULT_TENANT_COLUMN.to_string(),
        order_column: DEFAULT_ORDER_COLUMN.to_string(),
        amount_column: DEFAULT_AMOUNT_COLUMN.to_string(),
        cascade_option: DEFAULT_CASCADE_OPTION.to_string(),
        shard_transfer_mode: DEFAULT_TRANSFER_MODE.to_string(),
        regional_keys: vec![
            RegionalRowPlacementKey::new(
                "us-east-1",
                "us-east-1:tenant-a",
                "tenant-a",
                "mr3_us_worker",
            ),
            RegionalRowPlacementKey::new(
                "eu-west-1",
                "eu-west-1:tenant-b",
                "tenant-b",
                "mr3_eu_worker",
            ),
        ],
    }
}

pub fn canonical_regional_row_placement_sql_plan(
) -> Result<RegionalRowPlacementSqlPlan, RegionalRowPlacementError> {
    canonical_regional_row_placement_plan().to_sql_plan()
}

pub fn canonical_regional_row_placement_report(
) -> Result<RegionalRowPlacementReport, RegionalRowPlacementError> {
    canonical_regional_row_placement_plan().report()
}

pub fn canonical_regional_row_placement_fail_closed_checks() -> usize {
    let mut checks = 0;

    let mut missing_table = canonical_regional_row_placement_plan();
    missing_table.table_name.clear();
    if matches!(
        missing_table.validate(),
        Err(RegionalRowPlacementError::MissingRequiredField(
            "table_name"
        ))
    ) {
        checks += 1;
    }

    let mut unsafe_table = canonical_regional_row_placement_plan();
    unsafe_table.table_name = "public.mr3_orders;drop".to_string();
    if matches!(
        unsafe_table.validate(),
        Err(RegionalRowPlacementError::InvalidIdentifier { .. })
    ) {
        checks += 1;
    }

    let mut single_region = canonical_regional_row_placement_plan();
    single_region.regional_keys.pop();
    if matches!(
        single_region.validate(),
        Err(RegionalRowPlacementError::TooFewRegions)
    ) {
        checks += 1;
    }

    let mut duplicate_region = canonical_regional_row_placement_plan();
    duplicate_region.regional_keys[1].region = duplicate_region.regional_keys[0].region.clone();
    if matches!(
        duplicate_region.validate(),
        Err(RegionalRowPlacementError::DuplicateRegion(_))
    ) {
        checks += 1;
    }

    let mut duplicate_key = canonical_regional_row_placement_plan();
    duplicate_key.regional_keys[1].locality_key =
        duplicate_key.regional_keys[0].locality_key.clone();
    if matches!(
        duplicate_key.validate(),
        Err(RegionalRowPlacementError::DuplicateLocalityKey(_))
    ) {
        checks += 1;
    }

    let mut unsafe_var = canonical_regional_row_placement_plan();
    unsafe_var.regional_keys[0].expected_worker_psql_var = "worker-name".to_string();
    if matches!(
        unsafe_var.validate(),
        Err(RegionalRowPlacementError::InvalidPsqlVariable { .. })
    ) {
        checks += 1;
    }

    let mut unsafe_transfer = canonical_regional_row_placement_plan();
    unsafe_transfer.shard_transfer_mode = "force;drop".to_string();
    if matches!(
        unsafe_transfer.validate(),
        Err(RegionalRowPlacementError::UnsupportedTransferMode(_))
    ) {
        checks += 1;
    }

    checks
}

fn quote_qualified_identifier(
    field: &'static str,
    value: &str,
) -> Result<String, RegionalRowPlacementError> {
    validate_qualified_identifier(field, value)?;
    value
        .split('.')
        .map(|part| quote_identifier(field, part))
        .collect::<Result<Vec<_>, _>>()
        .map(|parts| parts.join("."))
}

fn quote_identifier(field: &'static str, value: &str) -> Result<String, RegionalRowPlacementError> {
    validate_identifier(field, value)?;
    Ok(format!("\"{value}\""))
}

fn validate_qualified_identifier(
    field: &'static str,
    value: &str,
) -> Result<(), RegionalRowPlacementError> {
    validate_non_empty(field, value)?;
    let parts = value.split('.').collect::<Vec<_>>();
    if parts.is_empty() || parts.len() > 2 {
        return Err(RegionalRowPlacementError::InvalidIdentifier {
            field,
            value: value.to_string(),
        });
    }
    for part in parts {
        validate_identifier(field, part)?;
    }
    Ok(())
}

fn validate_identifier(field: &'static str, value: &str) -> Result<(), RegionalRowPlacementError> {
    validate_non_empty(field, value)?;
    if value.len() > 63
        || value
            .chars()
            .next()
            .is_some_and(|character| !(character == '_' || character.is_ascii_alphabetic()))
        || !value
            .chars()
            .all(|character| character == '_' || character.is_ascii_alphanumeric())
    {
        return Err(RegionalRowPlacementError::InvalidIdentifier {
            field,
            value: value.to_string(),
        });
    }
    Ok(())
}

fn validate_region(value: &str) -> Result<(), RegionalRowPlacementError> {
    validate_non_empty("region", value)?;
    if value.len() > 63
        || value
            .chars()
            .any(|character| !(character.is_ascii_alphanumeric() || character == '-'))
    {
        return Err(RegionalRowPlacementError::InvalidRegion(value.to_string()));
    }
    Ok(())
}

fn validate_psql_variable(
    field: &'static str,
    value: &str,
) -> Result<(), RegionalRowPlacementError> {
    validate_non_empty(field, value)?;
    if value.len() > 63
        || value
            .chars()
            .next()
            .is_some_and(|character| !(character == '_' || character.is_ascii_alphabetic()))
        || !value
            .chars()
            .all(|character| character == '_' || character.is_ascii_alphanumeric())
    {
        return Err(RegionalRowPlacementError::InvalidPsqlVariable {
            field,
            value: value.to_string(),
        });
    }
    Ok(())
}

fn validate_non_empty(field: &'static str, value: &str) -> Result<(), RegionalRowPlacementError> {
    if value.trim().is_empty() {
        return Err(RegionalRowPlacementError::MissingRequiredField(field));
    }
    Ok(())
}

fn validate_cascade_option(value: &str) -> Result<(), RegionalRowPlacementError> {
    match value {
        "CASCADE" | "RESTRICT" => Ok(()),
        _ => Err(RegionalRowPlacementError::UnsupportedCascadeOption(
            value.to_string(),
        )),
    }
}

fn validate_transfer_mode(value: &str) -> Result<(), RegionalRowPlacementError> {
    match value {
        "block_writes" | "force_logical" | "auto" => Ok(()),
        _ => Err(RegionalRowPlacementError::UnsupportedTransferMode(
            value.to_string(),
        )),
    }
}

fn sql_literal(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_report_covers_live_multi_worker_boundary() {
        let report = canonical_regional_row_placement_report().expect("report");

        assert_eq!(report.feature_id, "MR3");
        assert_eq!(report.table_name, DEFAULT_TABLE);
        assert_eq!(report.locality_column, DEFAULT_LOCALITY_COLUMN);
        assert_eq!(report.regional_key_count, 2);
        assert_eq!(report.statement_count, 35);
        assert!(report.uses_isolate_tenant_to_new_shard);
        assert!(report.uses_citus_move_shard_placement);
        assert!(report.records_row_preservation);
        assert!(report.records_worker_placement);
        assert!(report.requires_worker_psql_variables);
        assert_eq!(report.fail_closed_checks, 7);
        assert!(!report.automatic_repartition_scheduler_exercised);
        assert!(!report.kubernetes_operator_reconciliation_exercised);
        assert!(!report.regional_traffic_router_exercised);
        assert!(!report.multi_region_network_exercised);
        assert!(!report.regional_failover_exercised);
    }

    #[test]
    fn sql_plan_renders_shard_isolation_movement_and_markers() {
        let sql_plan = canonical_regional_row_placement_sql_plan().expect("sql plan");
        let script = sql_plan.render_psql_script();

        assert!(script.contains("isolate_tenant_to_new_shard"));
        assert!(script.contains("citus_move_shard_placement"));
        assert!(script.contains(":'mr3_us_worker'"));
        assert!(script.contains(":'mr3_eu_worker'"));
        assert!(script.contains("mr3_rows_preserved"));
        assert!(script.contains("mr3_worker_placement_enforced"));
        assert!(script.contains("mr3_multi_region_network_exercised"));
    }

    #[test]
    fn rejects_unsafe_table_identifier() {
        let mut plan = canonical_regional_row_placement_plan();
        plan.table_name = "public.mr3_orders;drop".to_string();

        assert!(matches!(
            plan.validate(),
            Err(RegionalRowPlacementError::InvalidIdentifier { .. })
        ));
    }

    #[test]
    fn rejects_duplicate_regions() {
        let mut plan = canonical_regional_row_placement_plan();
        plan.regional_keys[1].region = plan.regional_keys[0].region.clone();

        assert!(matches!(
            plan.validate(),
            Err(RegionalRowPlacementError::DuplicateRegion(_))
        ));
    }

    #[test]
    fn rejects_unsafe_psql_worker_variable() {
        let mut plan = canonical_regional_row_placement_plan();
        plan.regional_keys[0].expected_worker_psql_var = "worker-name".to_string();

        assert!(matches!(
            plan.validate(),
            Err(RegionalRowPlacementError::InvalidPsqlVariable { .. })
        ));
    }
}
