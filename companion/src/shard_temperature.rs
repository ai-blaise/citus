// FEATURE: R12

use std::error::Error;
use std::fmt;

const FEATURE_ID: &str = "R12";
const DEFAULT_METRICS_TABLE: &str = "public.ai_blaise_shard_temperature_samples";
const HOT_TIER: &str = "hot";
const WARM_TIER: &str = "warm";
const COLD_TIER: &str = "cold";

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ShardTemperatureRankingPlan {
    pub metrics_table: String,
    pub hot_threshold: u32,
    pub warm_threshold: u32,
    pub max_ranked_shards: u32,
}

impl ShardTemperatureRankingPlan {
    pub fn validate(&self) -> Result<(), ShardTemperatureError> {
        quote_qualified_identifier("metrics_table", &self.metrics_table)?;
        if self.hot_threshold == 0 {
            return Err(ShardTemperatureError::InvalidPositive("hot_threshold"));
        }
        if self.warm_threshold == 0 {
            return Err(ShardTemperatureError::InvalidPositive("warm_threshold"));
        }
        if self.hot_threshold <= self.warm_threshold {
            return Err(ShardTemperatureError::InvalidThresholdOrder);
        }
        if self.max_ranked_shards == 0 {
            return Err(ShardTemperatureError::InvalidPositive("max_ranked_shards"));
        }
        Ok(())
    }

    pub fn to_sql_plan(&self) -> Result<ShardTemperatureSqlPlan, ShardTemperatureError> {
        self.validate()?;
        let metrics_table = quote_qualified_identifier("metrics_table", &self.metrics_table)?;
        let query = format!(
            "WITH shard_metrics AS (\n  SELECT\n    ds.shardid AS shard_id,\n    format('%I.%I', n.nspname, c.relname) AS table_name,\n    m.read_ops_per_min,\n    m.write_ops_per_min,\n    m.bytes_read_per_min,\n    m.bytes_written_per_min,\n    m.cold_age_seconds,\n    ROUND(\n      (m.read_ops_per_min * 0.70)\n      + (m.write_ops_per_min * 1.10)\n      + (((m.bytes_read_per_min::numeric + m.bytes_written_per_min::numeric) / 1048576.0) * 0.05)\n      + ((GREATEST(0, 3600 - LEAST(m.cold_age_seconds, 3600))::numeric / 3600.0) * 5.0),\n      4\n    ) AS temperature_score\n  FROM pg_dist_shard ds\n  JOIN pg_class c ON c.oid = ds.logicalrelid\n  JOIN pg_namespace n ON n.oid = c.relnamespace\n  JOIN {metrics_table} m ON m.shard_id = ds.shardid\n  WHERE m.sample_valid\n), ranked AS (\n  SELECT\n    shard_id,\n    table_name,\n    temperature_score,\n    CASE\n      WHEN temperature_score >= {hot_threshold} THEN '{HOT_TIER}'\n      WHEN temperature_score >= {warm_threshold} THEN '{WARM_TIER}'\n      ELSE '{COLD_TIER}'\n    END AS target_tier,\n    DENSE_RANK() OVER (ORDER BY temperature_score DESC, shard_id ASC) AS temperature_rank,\n    read_ops_per_min,\n    write_ops_per_min,\n    (bytes_read_per_min::numeric + bytes_written_per_min::numeric) AS bytes_per_min,\n    cold_age_seconds\n  FROM shard_metrics\n)\nSELECT shard_id, table_name, temperature_score, target_tier, temperature_rank,\n       read_ops_per_min, write_ops_per_min, bytes_per_min, cold_age_seconds\nFROM ranked\nORDER BY temperature_rank, shard_id\nLIMIT {max_ranked_shards}",
            metrics_table = metrics_table,
            hot_threshold = self.hot_threshold,
            warm_threshold = self.warm_threshold,
            max_ranked_shards = self.max_ranked_shards,
        );

        Ok(ShardTemperatureSqlPlan {
            feature_id: FEATURE_ID,
            metrics_table: self.metrics_table.clone(),
            statements: vec![query],
            hot_threshold: self.hot_threshold,
            warm_threshold: self.warm_threshold,
            max_ranked_shards: self.max_ranked_shards,
        })
    }

    pub fn report(&self) -> Result<ShardTemperatureRankingReport, ShardTemperatureError> {
        let sql_plan = self.to_sql_plan()?;
        let script = sql_plan.render_psql_script();
        Ok(ShardTemperatureRankingReport {
            feature_id: FEATURE_ID,
            metrics_table: self.metrics_table.clone(),
            statement_count: sql_plan.statements.len(),
            joins_citus_catalog: script.contains("pg_dist_shard")
                && script.contains("pg_class")
                && script.contains("pg_namespace"),
            ranks_shards: script.contains("DENSE_RANK()"),
            target_tiers: vec![HOT_TIER, WARM_TIER, COLD_TIER],
            fail_closed_checks: canonical_shard_temperature_fail_closed_checks(),
            automatic_tier_movement: sql_plan.contains_mutating_statement(),
            coldtier_moves_executed: false,
        })
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ShardTemperatureSqlPlan {
    pub feature_id: &'static str,
    pub metrics_table: String,
    pub statements: Vec<String>,
    pub hot_threshold: u32,
    pub warm_threshold: u32,
    pub max_ranked_shards: u32,
}

impl ShardTemperatureSqlPlan {
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
        ]
        .iter()
        .any(|needle| script.contains(needle))
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ShardTemperatureRankingReport {
    pub feature_id: &'static str,
    pub metrics_table: String,
    pub statement_count: usize,
    pub joins_citus_catalog: bool,
    pub ranks_shards: bool,
    pub target_tiers: Vec<&'static str>,
    pub fail_closed_checks: usize,
    pub automatic_tier_movement: bool,
    pub coldtier_moves_executed: bool,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum ShardTemperatureError {
    MissingRequiredField(&'static str),
    InvalidIdentifier { field: &'static str, value: String },
    InvalidPositive(&'static str),
    InvalidThresholdOrder,
}

impl fmt::Display for ShardTemperatureError {
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
            Self::InvalidThresholdOrder => write!(
                formatter,
                "hot_threshold must be greater than warm_threshold"
            ),
        }
    }
}

impl Error for ShardTemperatureError {}

pub fn canonical_shard_temperature_ranking_plan() -> ShardTemperatureRankingPlan {
    ShardTemperatureRankingPlan {
        metrics_table: DEFAULT_METRICS_TABLE.to_string(),
        hot_threshold: 50,
        warm_threshold: 5,
        max_ranked_shards: 64,
    }
}

pub fn canonical_shard_temperature_sql_plan(
) -> Result<ShardTemperatureSqlPlan, ShardTemperatureError> {
    canonical_shard_temperature_ranking_plan().to_sql_plan()
}

pub fn canonical_shard_temperature_ranking_report(
) -> Result<ShardTemperatureRankingReport, ShardTemperatureError> {
    canonical_shard_temperature_ranking_plan().report()
}

pub fn canonical_shard_temperature_fail_closed_checks() -> usize {
    let mut checks = 0;

    let mut empty_metrics = canonical_shard_temperature_ranking_plan();
    empty_metrics.metrics_table.clear();
    if matches!(
        empty_metrics.validate(),
        Err(ShardTemperatureError::MissingRequiredField("metrics_table"))
    ) {
        checks += 1;
    }

    let mut unsafe_metrics = canonical_shard_temperature_ranking_plan();
    unsafe_metrics.metrics_table = "public.bad;drop".to_string();
    if matches!(
        unsafe_metrics.validate(),
        Err(ShardTemperatureError::InvalidIdentifier { .. })
    ) {
        checks += 1;
    }

    let mut zero_hot_threshold = canonical_shard_temperature_ranking_plan();
    zero_hot_threshold.hot_threshold = 0;
    if matches!(
        zero_hot_threshold.validate(),
        Err(ShardTemperatureError::InvalidPositive("hot_threshold"))
    ) {
        checks += 1;
    }

    let mut inverted_thresholds = canonical_shard_temperature_ranking_plan();
    inverted_thresholds.hot_threshold = inverted_thresholds.warm_threshold;
    if matches!(
        inverted_thresholds.validate(),
        Err(ShardTemperatureError::InvalidThresholdOrder)
    ) {
        checks += 1;
    }

    let mut zero_limit = canonical_shard_temperature_ranking_plan();
    zero_limit.max_ranked_shards = 0;
    if matches!(
        zero_limit.validate(),
        Err(ShardTemperatureError::InvalidPositive("max_ranked_shards"))
    ) {
        checks += 1;
    }

    checks
}

fn quote_qualified_identifier(
    field: &'static str,
    value: &str,
) -> Result<String, ShardTemperatureError> {
    if value.trim().is_empty() {
        return Err(ShardTemperatureError::MissingRequiredField(field));
    }
    let parts = value.split('.').collect::<Vec<_>>();
    if parts.is_empty() || parts.len() > 2 {
        return Err(ShardTemperatureError::InvalidIdentifier {
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

fn quote_identifier(field: &'static str, value: &str) -> Result<String, ShardTemperatureError> {
    validate_identifier(field, value)?;
    Ok(format!("\"{value}\""))
}

fn validate_identifier(field: &'static str, value: &str) -> Result<(), ShardTemperatureError> {
    if value.trim().is_empty() {
        return Err(ShardTemperatureError::MissingRequiredField(field));
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
        return Err(ShardTemperatureError::InvalidIdentifier {
            field,
            value: value.to_string(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_shard_temperature_report_is_catalog_backed_and_read_only() {
        let report = canonical_shard_temperature_ranking_report().expect("report");

        assert_eq!(report.feature_id, "R12");
        assert_eq!(report.metrics_table, DEFAULT_METRICS_TABLE);
        assert_eq!(report.statement_count, 1);
        assert!(report.joins_citus_catalog);
        assert!(report.ranks_shards);
        assert_eq!(report.target_tiers, vec!["hot", "warm", "cold"]);
        assert_eq!(report.fail_closed_checks, 5);
        assert!(!report.automatic_tier_movement);
        assert!(!report.coldtier_moves_executed);
    }

    #[test]
    fn shard_temperature_sql_joins_citus_catalog_and_scores_metrics() {
        let sql_plan = canonical_shard_temperature_sql_plan().expect("sql plan");
        let script = sql_plan.render_psql_script();

        assert!(script.contains("FROM pg_dist_shard ds"));
        assert!(script.contains("JOIN pg_class c ON c.oid = ds.logicalrelid"));
        assert!(script.contains("JOIN pg_namespace n ON n.oid = c.relnamespace"));
        assert!(script.contains("JOIN \"public\".\"ai_blaise_shard_temperature_samples\" m"));
        assert!(script.contains("DENSE_RANK() OVER"));
        assert!(script.contains("WHEN temperature_score >= 50 THEN 'hot'"));
        assert!(script.contains("WHEN temperature_score >= 5 THEN 'warm'"));
        assert!(script.contains("ELSE 'cold'"));
        assert!(script.contains("ORDER BY temperature_rank, shard_id"));
        assert!(script.ends_with(';'));
        assert!(!sql_plan.contains_mutating_statement());
    }

    #[test]
    fn shard_temperature_rejects_unsafe_metrics_table() {
        let mut plan = canonical_shard_temperature_ranking_plan();
        plan.metrics_table = "public.temperature;drop".to_string();

        assert_eq!(
            plan.validate(),
            Err(ShardTemperatureError::InvalidIdentifier {
                field: "metrics_table",
                value: "temperature;drop".to_string(),
            })
        );
    }

    #[test]
    fn shard_temperature_rejects_invalid_thresholds() {
        let mut plan = canonical_shard_temperature_ranking_plan();
        plan.hot_threshold = plan.warm_threshold;

        assert_eq!(
            plan.validate(),
            Err(ShardTemperatureError::InvalidThresholdOrder)
        );
    }

    #[test]
    fn shard_temperature_rejects_zero_limit() {
        let mut plan = canonical_shard_temperature_ranking_plan();
        plan.max_ranked_shards = 0;

        assert_eq!(
            plan.validate(),
            Err(ShardTemperatureError::InvalidPositive("max_ranked_shards"))
        );
    }
}
