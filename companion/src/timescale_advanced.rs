// FEATURE: TS10
// FEATURE: TS11

use std::error::Error;
use std::fmt;

const FEATURE_IDS: &[&str] = &["TS10", "TS11"];
const DEFAULT_BASE_TABLE: &str = "public.ts10_ts11_metrics";
const DEFAULT_SOURCE_CAGG: &str = "public.ts10_hourly";
const DEFAULT_TARGET_CAGG: &str = "public.ts10_daily";
const DEFAULT_BLOOM_TABLE: &str = "public.ts11_segmentby_bloom_filters";
const DEFAULT_BLOOM_BITS: u32 = 2048;
const DEFAULT_BLOOM_HASHES: u32 = 3;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TimescaleAdvancedPlan {
    pub base_table: String,
    pub time_column: String,
    pub tenant_column: String,
    pub value_column: String,
    pub segmentby_columns: Vec<String>,
    pub source_cagg: String,
    pub target_cagg: String,
    pub bloom_table: String,
    pub bloom_bit_count: u32,
    pub bloom_hash_count: u32,
}

impl TimescaleAdvancedPlan {
    pub fn validate(&self) -> Result<(), TimescaleAdvancedError> {
        validate_qualified_identifier("base_table", &self.base_table)?;
        validate_identifier("time_column", &self.time_column)?;
        validate_identifier("tenant_column", &self.tenant_column)?;
        validate_identifier("value_column", &self.value_column)?;
        validate_identifier_list("segmentby_columns", &self.segmentby_columns)?;
        validate_qualified_identifier("source_cagg", &self.source_cagg)?;
        validate_qualified_identifier("target_cagg", &self.target_cagg)?;
        validate_qualified_identifier("bloom_table", &self.bloom_table)?;
        if self.bloom_bit_count == 0 {
            return Err(TimescaleAdvancedError::InvalidPositive("bloom_bit_count"));
        }
        if self.bloom_hash_count == 0 {
            return Err(TimescaleAdvancedError::InvalidPositive("bloom_hash_count"));
        }
        if self.bloom_hash_count > 8 {
            return Err(TimescaleAdvancedError::InvalidRange("bloom_hash_count"));
        }
        Ok(())
    }

    pub fn to_sql_plan(&self) -> Result<TimescaleAdvancedSqlPlan, TimescaleAdvancedError> {
        self.validate()?;
        let base_table = quote_qualified_identifier("base_table", &self.base_table)?;
        let time_column = quote_identifier("time_column", &self.time_column)?;
        let tenant_column = quote_identifier("tenant_column", &self.tenant_column)?;
        let value_column = quote_identifier("value_column", &self.value_column)?;
        let source_cagg = quote_qualified_identifier("source_cagg", &self.source_cagg)?;
        let target_cagg = quote_qualified_identifier("target_cagg", &self.target_cagg)?;
        let bloom_table = quote_qualified_identifier("bloom_table", &self.bloom_table)?;
        let source_cagg_literal = sql_literal(&self.source_cagg);
        let target_cagg_literal = sql_literal(&self.target_cagg);
        let segmentby_setting = sql_literal(&self.segmentby_columns.join(","));
        let segment_projection = self
            .segmentby_columns
            .iter()
            .map(|column| quote_identifier("segmentby_columns", column))
            .collect::<Result<Vec<_>, _>>()?
            .join(", ");
        let segment_key_expression = self
            .segmentby_columns
            .iter()
            .map(|column| {
                format!(
                    "{}::text",
                    quote_identifier("segmentby_columns", column).unwrap()
                )
            })
            .collect::<Vec<_>>()
            .join(", ");
        let bit_positions = (0..self.bloom_hash_count)
            .map(|seed| {
                format!(
                    "(mod(hashtextextended(segment_key, {seed})::numeric + 9223372036854775808, {bits}))::integer",
                    bits = self.bloom_bit_count
                )
            })
            .collect::<Vec<_>>()
            .join(", ");
        let statements = vec![
            format!(
                "CREATE MATERIALIZED VIEW {source_cagg}\nWITH (timescaledb.continuous) AS\nSELECT time_bucket('1 hour', {time_column}) AS bucket, {tenant_column}, avg({value_column}) AS avg_value, count(*) AS row_count\nFROM {base_table}\nGROUP BY 1, 2\nWITH NO DATA"
            ),
            format!("CALL refresh_continuous_aggregate({source_cagg_literal}, NULL, NULL)"),
            format!(
                "CREATE MATERIALIZED VIEW {target_cagg}\nWITH (timescaledb.continuous) AS\nSELECT time_bucket('1 day', bucket) AS bucket, {tenant_column}, avg(avg_value) AS avg_value, sum(row_count) AS row_count\nFROM {source_cagg}\nGROUP BY 1, 2\nWITH NO DATA"
            ),
            format!("CALL refresh_continuous_aggregate({target_cagg_literal}, NULL, NULL)"),
            format!(
                "ALTER TABLE {base_table} SET (timescaledb.compress, timescaledb.compress_segmentby = {segmentby_setting})"
            ),
            format!("DROP TABLE IF EXISTS {bloom_table}"),
            format!(
                "CREATE TABLE {bloom_table} (\n  segment_key text PRIMARY KEY,\n  bit_count integer NOT NULL CHECK (bit_count > 0),\n  hash_count integer NOT NULL CHECK (hash_count > 0),\n  bit_positions integer[] NOT NULL CHECK (array_length(bit_positions, 1) = hash_count),\n  source_rows bigint NOT NULL CHECK (source_rows > 0)\n)"
            ),
            format!(
                "WITH segment_source AS (\n  SELECT concat_ws(':', {segment_key_expression}) AS segment_key, count(*)::bigint AS source_rows\n  FROM {base_table}\n  GROUP BY {segment_projection}\n), bloom AS (\n  SELECT segment_key, source_rows, ARRAY[{bit_positions}] AS bit_positions\n  FROM segment_source\n)\nINSERT INTO {bloom_table} (segment_key, bit_count, hash_count, bit_positions, source_rows)\nSELECT segment_key, {bit_count}, {hash_count}, bit_positions, source_rows\nFROM bloom\nORDER BY segment_key",
                bit_count = self.bloom_bit_count,
                hash_count = self.bloom_hash_count
            ),
            format!(
                "SELECT 'hierarchical_cagg_count' AS marker, count(*)::text AS value, '' AS detail\nFROM timescaledb_information.continuous_aggregates\nWHERE view_schema || '.' || view_name IN ({source_cagg_literal}, {target_cagg_literal})"
            ),
            format!(
                "SELECT 'hierarchical_cagg_daily_rows' AS marker, count(*)::text AS value, '' AS detail\nFROM {target_cagg}"
            ),
            format!(
                "SELECT 'compression_segmentby_columns' AS marker, count(*)::text AS value, string_agg(attname, ',' ORDER BY segmentby_column_index) AS detail\nFROM timescaledb_information.compression_settings\nWHERE hypertable_schema || '.' || hypertable_name = {}\n  AND segmentby_column_index IS NOT NULL",
                sql_literal(&self.base_table)
            ),
            format!(
                "SELECT 'segmentby_bloom_rows' AS marker, count(*)::text AS value, min(bit_count)::text || ':' || min(hash_count)::text AS detail\nFROM {bloom_table}"
            ),
            "SELECT 'native_timescale_bloom_filter' AS marker, 'false' AS value, '' AS detail".to_string(),
            "SELECT 'planner_integration_exercised' AS marker, 'false' AS value, '' AS detail".to_string(),
        ];

        Ok(TimescaleAdvancedSqlPlan {
            feature_ids: FEATURE_IDS,
            statements,
            base_table: self.base_table.clone(),
            source_cagg: self.source_cagg.clone(),
            target_cagg: self.target_cagg.clone(),
            bloom_table: self.bloom_table.clone(),
            bloom_bit_count: self.bloom_bit_count,
            bloom_hash_count: self.bloom_hash_count,
        })
    }

    pub fn report(&self) -> Result<TimescaleAdvancedReport, TimescaleAdvancedError> {
        let sql_plan = self.to_sql_plan()?;
        let script = sql_plan.render_psql_script();
        Ok(TimescaleAdvancedReport {
            feature_ids: FEATURE_IDS,
            base_table: self.base_table.clone(),
            source_cagg: self.source_cagg.clone(),
            target_cagg: self.target_cagg.clone(),
            bloom_table: self.bloom_table.clone(),
            statement_count: sql_plan.statements.len(),
            hierarchical_cagg_refresh_required: script
                .matches("refresh_continuous_aggregate")
                .count(),
            compression_segmentby_required: script.contains("timescaledb.compress_segmentby"),
            bloom_filter_materialized: script.contains("bit_positions")
                && script.contains("hashtextextended"),
            bloom_bit_count: self.bloom_bit_count,
            bloom_hash_count: self.bloom_hash_count,
            fail_closed_checks: canonical_timescale_advanced_fail_closed_checks(),
            native_timescale_bloom_filter_claimed: false,
            planner_integration_exercised: false,
            multi_worker_fanout_exercised: false,
        })
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TimescaleAdvancedSqlPlan {
    pub feature_ids: &'static [&'static str],
    pub statements: Vec<String>,
    pub base_table: String,
    pub source_cagg: String,
    pub target_cagg: String,
    pub bloom_table: String,
    pub bloom_bit_count: u32,
    pub bloom_hash_count: u32,
}

impl TimescaleAdvancedSqlPlan {
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
pub struct TimescaleAdvancedReport {
    pub feature_ids: &'static [&'static str],
    pub base_table: String,
    pub source_cagg: String,
    pub target_cagg: String,
    pub bloom_table: String,
    pub statement_count: usize,
    pub hierarchical_cagg_refresh_required: usize,
    pub compression_segmentby_required: bool,
    pub bloom_filter_materialized: bool,
    pub bloom_bit_count: u32,
    pub bloom_hash_count: u32,
    pub fail_closed_checks: usize,
    pub native_timescale_bloom_filter_claimed: bool,
    pub planner_integration_exercised: bool,
    pub multi_worker_fanout_exercised: bool,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum TimescaleAdvancedError {
    MissingRequiredField(&'static str),
    InvalidIdentifier { field: &'static str, value: String },
    InvalidPositive(&'static str),
    InvalidRange(&'static str),
}

impl fmt::Display for TimescaleAdvancedError {
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
            Self::InvalidRange(field) => {
                write!(formatter, "{field} is outside the supported range")
            }
        }
    }
}

impl Error for TimescaleAdvancedError {}

pub fn canonical_timescale_advanced_plan() -> TimescaleAdvancedPlan {
    TimescaleAdvancedPlan {
        base_table: DEFAULT_BASE_TABLE.to_string(),
        time_column: "metric_time".to_string(),
        tenant_column: "tenant_id".to_string(),
        value_column: "value".to_string(),
        segmentby_columns: vec!["tenant_id".to_string(), "device_id".to_string()],
        source_cagg: DEFAULT_SOURCE_CAGG.to_string(),
        target_cagg: DEFAULT_TARGET_CAGG.to_string(),
        bloom_table: DEFAULT_BLOOM_TABLE.to_string(),
        bloom_bit_count: DEFAULT_BLOOM_BITS,
        bloom_hash_count: DEFAULT_BLOOM_HASHES,
    }
}

pub fn canonical_timescale_advanced_sql_plan(
) -> Result<TimescaleAdvancedSqlPlan, TimescaleAdvancedError> {
    canonical_timescale_advanced_plan().to_sql_plan()
}

pub fn canonical_timescale_advanced_report(
) -> Result<TimescaleAdvancedReport, TimescaleAdvancedError> {
    canonical_timescale_advanced_plan().report()
}

pub fn canonical_timescale_advanced_fail_closed_checks() -> usize {
    let mut checks = 0;

    let mut missing_base = canonical_timescale_advanced_plan();
    missing_base.base_table.clear();
    if matches!(
        missing_base.validate(),
        Err(TimescaleAdvancedError::MissingRequiredField("base_table"))
    ) {
        checks += 1;
    }

    let mut unsafe_cagg = canonical_timescale_advanced_plan();
    unsafe_cagg.source_cagg = "bad;cagg".to_string();
    if matches!(
        unsafe_cagg.validate(),
        Err(TimescaleAdvancedError::InvalidIdentifier { .. })
    ) {
        checks += 1;
    }

    let mut no_segment = canonical_timescale_advanced_plan();
    no_segment.segmentby_columns.clear();
    if matches!(
        no_segment.validate(),
        Err(TimescaleAdvancedError::MissingRequiredField(
            "segmentby_columns"
        ))
    ) {
        checks += 1;
    }

    let mut zero_bits = canonical_timescale_advanced_plan();
    zero_bits.bloom_bit_count = 0;
    if matches!(
        zero_bits.validate(),
        Err(TimescaleAdvancedError::InvalidPositive("bloom_bit_count"))
    ) {
        checks += 1;
    }

    let mut too_many_hashes = canonical_timescale_advanced_plan();
    too_many_hashes.bloom_hash_count = 9;
    if matches!(
        too_many_hashes.validate(),
        Err(TimescaleAdvancedError::InvalidRange("bloom_hash_count"))
    ) {
        checks += 1;
    }

    checks
}

fn quote_qualified_identifier(
    field: &'static str,
    value: &str,
) -> Result<String, TimescaleAdvancedError> {
    validate_qualified_identifier(field, value)?;
    value
        .split('.')
        .map(|part| quote_identifier(field, part))
        .collect::<Result<Vec<_>, _>>()
        .map(|parts| parts.join("."))
}

fn validate_qualified_identifier(
    field: &'static str,
    value: &str,
) -> Result<(), TimescaleAdvancedError> {
    if value.trim().is_empty() {
        return Err(TimescaleAdvancedError::MissingRequiredField(field));
    }
    let parts = value.split('.').collect::<Vec<_>>();
    if parts.is_empty() || parts.len() > 2 {
        return Err(TimescaleAdvancedError::InvalidIdentifier {
            field,
            value: value.to_string(),
        });
    }
    for part in parts {
        validate_identifier(field, part)?;
    }
    Ok(())
}

fn quote_identifier(field: &'static str, value: &str) -> Result<String, TimescaleAdvancedError> {
    validate_identifier(field, value)?;
    Ok(format!("\"{value}\""))
}

fn validate_identifier(field: &'static str, value: &str) -> Result<(), TimescaleAdvancedError> {
    if value.trim().is_empty() {
        return Err(TimescaleAdvancedError::MissingRequiredField(field));
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
        return Err(TimescaleAdvancedError::InvalidIdentifier {
            field,
            value: value.to_string(),
        });
    }
    Ok(())
}

fn validate_identifier_list(
    field: &'static str,
    values: &[String],
) -> Result<(), TimescaleAdvancedError> {
    if values.is_empty() {
        return Err(TimescaleAdvancedError::MissingRequiredField(field));
    }
    for value in values {
        validate_identifier(field, value)?;
    }
    Ok(())
}

fn sql_literal(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_report_covers_hierarchical_caggs_and_bloom_boundary() {
        let report = canonical_timescale_advanced_report().expect("report");

        assert_eq!(report.feature_ids, ["TS10", "TS11"]);
        assert_eq!(report.statement_count, 14);
        assert_eq!(report.hierarchical_cagg_refresh_required, 2);
        assert!(report.compression_segmentby_required);
        assert!(report.bloom_filter_materialized);
        assert_eq!(report.bloom_bit_count, 2048);
        assert_eq!(report.bloom_hash_count, 3);
        assert_eq!(report.fail_closed_checks, 5);
        assert!(!report.native_timescale_bloom_filter_claimed);
        assert!(!report.planner_integration_exercised);
        assert!(!report.multi_worker_fanout_exercised);
    }

    #[test]
    fn sql_plan_renders_hierarchical_caggs_and_bloom_table() {
        let sql_plan = canonical_timescale_advanced_sql_plan().expect("sql plan");
        let script = sql_plan.render_psql_script();

        assert!(script.contains("CREATE MATERIALIZED VIEW \"public\".\"ts10_hourly\""));
        assert!(script.contains("CREATE MATERIALIZED VIEW \"public\".\"ts10_daily\""));
        assert!(script.contains("CALL refresh_continuous_aggregate('public.ts10_hourly'"));
        assert!(script.contains("timescaledb.compress_segmentby = 'tenant_id,device_id'"));
        assert!(script.contains("CREATE TABLE \"public\".\"ts11_segmentby_bloom_filters\""));
        assert!(script.contains("hashtextextended(segment_key, 0)"));
        assert!(script.contains("native_timescale_bloom_filter"));
    }

    #[test]
    fn rejects_unsafe_cagg_identifier() {
        let mut plan = canonical_timescale_advanced_plan();
        plan.source_cagg = "bad;cagg".to_string();

        assert!(matches!(
            plan.validate(),
            Err(TimescaleAdvancedError::InvalidIdentifier { .. })
        ));
    }

    #[test]
    fn rejects_empty_segmentby_columns() {
        let mut plan = canonical_timescale_advanced_plan();
        plan.segmentby_columns.clear();

        assert_eq!(
            plan.validate(),
            Err(TimescaleAdvancedError::MissingRequiredField(
                "segmentby_columns"
            ))
        );
    }

    #[test]
    fn rejects_unsupported_hash_count() {
        let mut plan = canonical_timescale_advanced_plan();
        plan.bloom_hash_count = 9;

        assert_eq!(
            plan.validate(),
            Err(TimescaleAdvancedError::InvalidRange("bloom_hash_count"))
        );
    }
}
