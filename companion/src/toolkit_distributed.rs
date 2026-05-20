// FEATURE: T8
// FEATURE: TS13
// FEATURE: TS14
// FEATURE: TS15
// FEATURE: TS16
// FEATURE: TS17
// FEATURE: L9

use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ToolkitDistributedPlan {
    pub source_table: String,
    pub worker_view: String,
    pub coordinator_view: String,
    pub distribution_column: String,
    pub time_column: Option<String>,
    pub value_column: String,
    pub bucket_width: Option<String>,
    pub aggregate: ToolkitAggregateKind,
}

impl ToolkitDistributedPlan {
    pub fn new(
        source_table: impl Into<String>,
        worker_view: impl Into<String>,
        coordinator_view: impl Into<String>,
        distribution_column: impl Into<String>,
        value_column: impl Into<String>,
        aggregate: ToolkitAggregateKind,
    ) -> Result<Self, ToolkitDistributedError> {
        let plan = Self {
            source_table: source_table.into(),
            worker_view: worker_view.into(),
            coordinator_view: coordinator_view.into(),
            distribution_column: distribution_column.into(),
            time_column: None,
            value_column: value_column.into(),
            bucket_width: None,
            aggregate,
        };
        plan.validate_required_fields()?;
        Ok(plan)
    }

    pub fn with_time_column(mut self, time_column: impl Into<String>) -> Self {
        self.time_column = Some(time_column.into());
        self
    }

    pub fn with_bucket_width(mut self, bucket_width: impl Into<String>) -> Self {
        self.bucket_width = Some(bucket_width.into());
        self
    }

    pub fn validate(&self) -> Result<(), ToolkitDistributedError> {
        self.validate_required_fields()?;

        if self.aggregate.requires_time_column()
            && self.time_column.as_deref().unwrap_or("").trim().is_empty()
        {
            return Err(ToolkitDistributedError::MissingRequiredField("time_column"));
        }

        if self.aggregate.requires_bucket_width()
            && self.bucket_width.as_deref().unwrap_or("").trim().is_empty()
        {
            return Err(ToolkitDistributedError::MissingRequiredField(
                "bucket_width",
            ));
        }

        Ok(())
    }

    fn validate_required_fields(&self) -> Result<(), ToolkitDistributedError> {
        validate_required("source_table", &self.source_table)?;
        validate_required("worker_view", &self.worker_view)?;
        validate_required("coordinator_view", &self.coordinator_view)?;
        validate_required("distribution_column", &self.distribution_column)?;
        validate_required("value_column", &self.value_column)
    }

    pub fn to_sql_plan(&self) -> Result<ToolkitSqlPlan, ToolkitDistributedError> {
        self.validate()?;
        let partial_expression = self.aggregate.partial_expression(self);
        let finalize_expression = self.aggregate.finalize_expression("partial_state");

        ToolkitSqlPlan::new(
            self.aggregate.feature_id(),
            vec![
                format!(
                    "CREATE OR REPLACE VIEW {} AS\n\
                     SELECT {} AS distribution_key,\n\
                            {} AS partial_state\n\
                     FROM {}\n\
                     GROUP BY 1;",
                    self.worker_view,
                    self.distribution_column,
                    partial_expression,
                    self.source_table
                ),
                format!(
                    "CREATE OR REPLACE VIEW {} AS\n\
                     SELECT distribution_key,\n\
                            {} AS aggregate_value\n\
                     FROM {}\n\
                     GROUP BY 1;",
                    self.coordinator_view, finalize_expression, self.worker_view
                ),
            ],
        )
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum ToolkitAggregateKind {
    TimeBucketGapfill,
    CounterAgg,
    GaugeAgg,
    HeartbeatAgg,
    PercentileAgg,
    FrequencyAgg,
    HyperLogLog,
    TDigest,
    AsapSmooth,
    Lttb,
    CandlestickAgg,
    StateVec,
    RangeAgg,
    TimeWeightedAverage,
}

impl ToolkitAggregateKind {
    pub fn feature_id(self) -> &'static str {
        match self {
            Self::TimeBucketGapfill => "TS13",
            Self::CounterAgg | Self::GaugeAgg | Self::HeartbeatAgg => "TS14",
            Self::PercentileAgg | Self::FrequencyAgg => "TS15",
            Self::AsapSmooth | Self::Lttb => "TS16",
            Self::CandlestickAgg | Self::StateVec | Self::RangeAgg => "TS17",
            Self::HyperLogLog | Self::TDigest | Self::TimeWeightedAverage => "T8",
        }
    }

    fn requires_time_column(self) -> bool {
        matches!(
            self,
            Self::TimeBucketGapfill
                | Self::AsapSmooth
                | Self::Lttb
                | Self::CandlestickAgg
                | Self::TimeWeightedAverage
        )
    }

    fn requires_bucket_width(self) -> bool {
        matches!(self, Self::TimeBucketGapfill)
    }

    fn partial_expression(self, plan: &ToolkitDistributedPlan) -> String {
        let value = &plan.value_column;
        let time = plan.time_column.as_deref().unwrap_or("");
        let bucket = plan.bucket_width.as_deref().unwrap_or("");
        match self {
            Self::TimeBucketGapfill => format!(
                "time_bucket_gapfill({}, {}) WITHIN GROUP (ORDER BY {})",
                sql_literal(bucket),
                time,
                time
            ),
            Self::CounterAgg => format!("counter_agg({value})"),
            Self::GaugeAgg => format!("gauge_agg({value})"),
            Self::HeartbeatAgg => format!("heartbeat_agg({value})"),
            Self::PercentileAgg => format!("percentile_agg({value})"),
            Self::FrequencyAgg => format!("freq_agg({value})"),
            Self::HyperLogLog => format!("hyperloglog({value})"),
            Self::TDigest => format!("tdigest({value})"),
            Self::AsapSmooth => format!("asap_smooth({time}, {value})"),
            Self::Lttb => format!("lttb({time}, {value})"),
            Self::CandlestickAgg => format!("candlestick_agg({time}, {value})"),
            Self::StateVec => format!("state_agg({value})"),
            Self::RangeAgg => format!("range_agg({value})"),
            Self::TimeWeightedAverage => format!("time_weight({time}, {value})"),
        }
    }

    fn finalize_expression(self, partial_column: &str) -> String {
        match self {
            Self::TimeBucketGapfill => format!("locf(interpolate({partial_column}))"),
            Self::CounterAgg => format!("rollup({partial_column})"),
            Self::GaugeAgg => format!("rollup({partial_column})"),
            Self::HeartbeatAgg => format!("heartbeat_agg_rollup({partial_column})"),
            Self::PercentileAgg => format!("approx_percentile(0.99, rollup({partial_column}))"),
            Self::FrequencyAgg => format!("topn(10, rollup({partial_column}))"),
            Self::HyperLogLog => format!("distinct_count(rollup({partial_column}))"),
            Self::TDigest => format!("approx_percentile(0.99, rollup({partial_column}))"),
            Self::AsapSmooth => format!("asap_smooth_final(rollup({partial_column}))"),
            Self::Lttb => format!("lttb_final(rollup({partial_column}))"),
            Self::CandlestickAgg => format!("rollup({partial_column})"),
            Self::StateVec => format!("rollup({partial_column})"),
            Self::RangeAgg => format!("rollup({partial_column})"),
            Self::TimeWeightedAverage => format!("average(rollup({partial_column}))"),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ToolkitSqlPlan {
    pub feature_id: &'static str,
    pub commands: Vec<String>,
}

impl ToolkitSqlPlan {
    fn new(
        feature_id: &'static str,
        commands: Vec<String>,
    ) -> Result<Self, ToolkitDistributedError> {
        if commands.is_empty() || commands.iter().any(|command| command.trim().is_empty()) {
            return Err(ToolkitDistributedError::MissingRequiredField("commands"));
        }
        Ok(Self {
            feature_id,
            commands,
        })
    }

    pub fn script(&self) -> String {
        self.commands.join("\n")
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum ToolkitDistributedError {
    MissingRequiredField(&'static str),
}

impl fmt::Display for ToolkitDistributedError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingRequiredField(field) => write!(formatter, "{field} must not be empty"),
        }
    }
}

impl Error for ToolkitDistributedError {}

fn validate_required(field: &'static str, value: &str) -> Result<(), ToolkitDistributedError> {
    if value.trim().is_empty() {
        return Err(ToolkitDistributedError::MissingRequiredField(field));
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
    fn percentile_aggregate_renders_worker_partial_and_coordinator_rollup() {
        let plan = ToolkitDistributedPlan::new(
            "metrics.cpu",
            "companion.worker_cpu_p99",
            "companion.cpu_p99",
            "tenant_id",
            "usage_percent",
            ToolkitAggregateKind::PercentileAgg,
        )
        .unwrap()
        .to_sql_plan()
        .unwrap();

        assert_eq!(plan.feature_id, "TS15");
        assert!(plan.script().contains("percentile_agg(usage_percent)"));
        assert!(plan
            .script()
            .contains("approx_percentile(0.99, rollup(partial_state))"));
    }

    #[test]
    fn gapfill_requires_time_column_and_bucket_width() {
        let plan = ToolkitDistributedPlan::new(
            "metrics.cpu",
            "companion.worker_gapfill",
            "companion.gapfill",
            "tenant_id",
            "usage_percent",
            ToolkitAggregateKind::TimeBucketGapfill,
        )
        .unwrap();

        assert_eq!(
            plan.to_sql_plan(),
            Err(ToolkitDistributedError::MissingRequiredField("time_column"))
        );
    }

    #[test]
    fn gapfill_renders_time_bucket_gapfill_plan() {
        let plan = ToolkitDistributedPlan::new(
            "metrics.cpu",
            "companion.worker_gapfill",
            "companion.gapfill",
            "tenant_id",
            "usage_percent",
            ToolkitAggregateKind::TimeBucketGapfill,
        )
        .unwrap()
        .with_time_column("created_at")
        .with_bucket_width("1 minute")
        .to_sql_plan()
        .unwrap();

        assert_eq!(plan.feature_id, "TS13");
        assert!(plan
            .script()
            .contains("time_bucket_gapfill('1 minute', created_at)"));
    }

    #[test]
    fn metric_aggregate_maps_to_ts14() {
        let plan = ToolkitDistributedPlan::new(
            "metrics.requests",
            "companion.worker_counter",
            "companion.counter",
            "tenant_id",
            "request_count",
            ToolkitAggregateKind::CounterAgg,
        )
        .unwrap()
        .to_sql_plan()
        .unwrap();

        assert_eq!(plan.feature_id, "TS14");
        assert!(plan.script().contains("counter_agg(request_count)"));
    }

    #[test]
    fn plan_requires_distribution_column() {
        let plan = ToolkitDistributedPlan::new(
            "metrics.cpu",
            "companion.worker_cpu_p99",
            "companion.cpu_p99",
            "",
            "usage_percent",
            ToolkitAggregateKind::PercentileAgg,
        );

        assert_eq!(
            plan,
            Err(ToolkitDistributedError::MissingRequiredField(
                "distribution_column"
            ))
        );
    }
}
