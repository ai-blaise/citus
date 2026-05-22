// FEATURE: S6
// FEATURE: S13
// FEATURE: T2

use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PlacementGenerationQuery {
    pub shard_id: u64,
}

impl PlacementGenerationQuery {
    pub fn validate(&self) -> Result<(), RouterAssistError> {
        validate_shard_id(self.shard_id)
    }

    /// Render the SQL command that reads the placement-generation counter
    /// from the coordinator. The SELECT returns a single bigint row.
    ///
    /// FEATURE: T2 -- companion-side subscriber for partial plan-cache
    /// invalidation, paired with patches/0005-placement-generation-counter.patch.
    pub fn to_sql_plan(&self) -> Result<RouterAssistSqlPlan, RouterAssistError> {
        self.validate()?;
        RouterAssistSqlPlan::new(
            "T2",
            vec!["SELECT pg_catalog.citus_placement_generation();".to_string()],
        )
    }
}

/// SQL command envelope mirroring PlanFreezeSqlPlan: a single feature id and an
/// ordered list of SQL statements the executor must run on the coordinator.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct RouterAssistSqlPlan {
    pub feature_id: &'static str,
    pub commands: Vec<String>,
}

impl RouterAssistSqlPlan {
    fn new(feature_id: &'static str, commands: Vec<String>) -> Result<Self, RouterAssistError> {
        if commands.is_empty() || commands.iter().any(|command| command.trim().is_empty()) {
            return Err(RouterAssistError::MissingRequiredField("commands"));
        }
        Ok(Self {
            feature_id,
            commands,
        })
    }

    pub fn script(&self) -> String {
        self.commands.join(
            "
",
        )
    }
}

/// One observation of the coordinator's placement-generation counter, parsed
/// out of the row returned by [`PlacementGenerationQuery::to_sql_plan`].
///
/// FEATURE: T2
#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub struct PlacementGenerationSample {
    pub generation: u64,
}

impl PlacementGenerationSample {
    /// Parse a bigint row value into a sample. The catalog UDF returns the
    /// counter as `int8`, which arrives over the wire either as a numeric
    /// string or an `i64`. Both are accepted; negative or overflowing values
    /// are rejected because the source counter is `uint64`.
    pub fn from_catalog_value(value: i64) -> Result<Self, RouterAssistError> {
        if value < 0 {
            return Err(RouterAssistError::InvalidGenerationSample);
        }
        // Negative values were rejected above, so the bit pattern is a valid u64.
        #[allow(clippy::cast_sign_loss)]
        let generation = value as u64;
        Ok(Self { generation })
    }
}

/// Companion-side state that watches the placement-generation counter and
/// emits partial-invalidation hints to the pool's shard-map subscriber path.
///
/// The subscriber is intentionally additive: relcache invalidations from
/// Citus still drive correctness; the counter lets the pool avoid dropping
/// every cached plan when only a handful of placements moved.
///
/// FEATURE: T2
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PlacementGenerationSubscriber {
    last_observed: Option<PlacementGenerationSample>,
}

impl PlacementGenerationSubscriber {
    pub fn new() -> Self {
        Self {
            last_observed: None,
        }
    }

    /// Returns the most recently recorded sample, if any.
    pub fn last_observed(&self) -> Option<PlacementGenerationSample> {
        self.last_observed
    }

    /// Record a fresh sample. Returns an [`InvalidationHint`] describing the
    /// transition; the pool consumes the hint to decide between
    /// `Invalidate::Partial`, `Invalidate::Full`, or `Invalidate::None`.
    pub fn observe(
        &mut self,
        sample: PlacementGenerationSample,
    ) -> Result<InvalidationHint, RouterAssistError> {
        let hint = match self.last_observed {
            None => InvalidationHint::Initial(sample),
            Some(previous) if previous == sample => InvalidationHint::Unchanged(sample),
            Some(previous) if sample.generation > previous.generation => {
                InvalidationHint::Advanced {
                    previous,
                    current: sample,
                }
            }
            Some(previous) => {
                // The counter is process-local on the coordinator; a strictly
                // lower value means the coordinator restarted and the cache
                // must be dropped wholesale.
                InvalidationHint::Reset {
                    previous,
                    current: sample,
                }
            }
        };
        self.last_observed = Some(sample);
        Ok(hint)
    }
}

impl Default for PlacementGenerationSubscriber {
    fn default() -> Self {
        Self::new()
    }
}

/// Outcome of a single placement-generation observation.
///
/// FEATURE: T2
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum InvalidationHint {
    /// First sample after subscriber boot; the pool seeds its cache and emits
    /// no invalidation.
    Initial(PlacementGenerationSample),
    /// Counter unchanged; no work to do.
    Unchanged(PlacementGenerationSample),
    /// Counter advanced; the pool can run a partial invalidation, dropping
    /// only the plans whose generation snapshot is older than `current`.
    Advanced {
        previous: PlacementGenerationSample,
        current: PlacementGenerationSample,
    },
    /// Counter went backwards, meaning the coordinator restarted; the pool
    /// must drop the entire plan cache.
    Reset {
        previous: PlacementGenerationSample,
        current: PlacementGenerationSample,
    },
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ShardForValuePlan {
    pub table: String,
    pub distribution_column: String,
    pub value_hash: i64,
    pub shard_count: u32,
    pub strategy: ShardRoutingStrategy,
}

impl ShardForValuePlan {
    pub fn validate(&self) -> Result<(), RouterAssistError> {
        validate_required("table", &self.table)?;
        validate_required("distribution_column", &self.distribution_column)?;
        if self.shard_count == 0 {
            return Err(RouterAssistError::InvalidShardCount);
        }
        self.strategy.validate()
    }

    pub fn target_shard_index(&self) -> Result<u32, RouterAssistError> {
        self.validate()?;
        // Result is taken modulo `shard_count` (u32) before truncation, so the
        // u64 → u32 cast is safe.
        #[allow(clippy::cast_possible_truncation)]
        Ok((self.value_hash.unsigned_abs() % u64::from(self.shard_count)) as u32)
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum ShardRoutingStrategy {
    Hash,
    Range {
        lower_bound: String,
        upper_bound: String,
    },
}

impl ShardRoutingStrategy {
    fn validate(&self) -> Result<(), RouterAssistError> {
        match self {
            Self::Hash => Ok(()),
            Self::Range {
                lower_bound,
                upper_bound,
            } => {
                validate_required("range.lower_bound", lower_bound)?;
                validate_required("range.upper_bound", upper_bound)
            }
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct LocalPlacementCheck {
    pub shard_id: u64,
    pub worker_name: String,
}

impl LocalPlacementCheck {
    pub fn validate(&self) -> Result<(), RouterAssistError> {
        validate_shard_id(self.shard_id)?;
        validate_required("worker_name", &self.worker_name)
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum RouterAssistError {
    InvalidGenerationSample,
    InvalidShardCount,
    InvalidShardId,
    MissingRequiredField(&'static str),
}

impl fmt::Display for RouterAssistError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidGenerationSample => write!(
                formatter,
                "placement_generation sample must be a non-negative bigint"
            ),
            Self::InvalidShardCount => write!(formatter, "shard_count must be greater than zero"),
            Self::InvalidShardId => write!(formatter, "shard_id must be greater than zero"),
            Self::MissingRequiredField(field) => {
                write!(formatter, "{field} must not be empty")
            }
        }
    }
}

impl Error for RouterAssistError {}

fn validate_shard_id(shard_id: u64) -> Result<(), RouterAssistError> {
    if shard_id == 0 {
        return Err(RouterAssistError::InvalidShardId);
    }
    Ok(())
}

fn validate_required(field: &'static str, value: &str) -> Result<(), RouterAssistError> {
    if value.trim().is_empty() {
        return Err(RouterAssistError::MissingRequiredField(field));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_routing_computes_target_index() {
        let plan = ShardForValuePlan {
            table: "public.metrics".to_string(),
            distribution_column: "tenant_id".to_string(),
            value_hash: 42,
            shard_count: 8,
            strategy: ShardRoutingStrategy::Hash,
        };

        assert_eq!(plan.target_shard_index(), Ok(2));
    }

    #[test]
    fn range_routing_requires_bounds() {
        let plan = ShardForValuePlan {
            table: "public.events".to_string(),
            distribution_column: "created_at".to_string(),
            value_hash: 0,
            shard_count: 16,
            strategy: ShardRoutingStrategy::Range {
                lower_bound: String::new(),
                upper_bound: "2026-01-01".to_string(),
            },
        };

        assert_eq!(
            plan.validate(),
            Err(RouterAssistError::MissingRequiredField("range.lower_bound"))
        );
    }

    #[test]
    fn local_placement_requires_worker_name() {
        let check = LocalPlacementCheck {
            shard_id: 1,
            worker_name: " ".to_string(),
        };

        assert_eq!(
            check.validate(),
            Err(RouterAssistError::MissingRequiredField("worker_name"))
        );
    }

    #[test]
    fn placement_generation_query_emits_catalog_select() {
        let plan = PlacementGenerationQuery { shard_id: 1 }
            .to_sql_plan()
            .unwrap();
        assert_eq!(plan.feature_id, "T2");
        assert_eq!(
            plan.script(),
            "SELECT pg_catalog.citus_placement_generation();"
        );
    }

    #[test]
    fn placement_generation_query_rejects_zero_shard_id() {
        let result = PlacementGenerationQuery { shard_id: 0 }.to_sql_plan();
        assert_eq!(result.err(), Some(RouterAssistError::InvalidShardId));
    }

    #[test]
    fn placement_generation_sample_rejects_negative_bigint() {
        assert_eq!(
            PlacementGenerationSample::from_catalog_value(-1).err(),
            Some(RouterAssistError::InvalidGenerationSample)
        );
    }

    #[test]
    fn placement_generation_subscriber_emits_initial_then_advanced() {
        let mut subscriber = PlacementGenerationSubscriber::new();
        let initial = PlacementGenerationSample::from_catalog_value(7).unwrap();
        assert_eq!(
            subscriber.observe(initial).unwrap(),
            InvalidationHint::Initial(initial)
        );

        let next = PlacementGenerationSample::from_catalog_value(9).unwrap();
        assert_eq!(
            subscriber.observe(next).unwrap(),
            InvalidationHint::Advanced {
                previous: initial,
                current: next,
            }
        );

        assert_eq!(
            subscriber.observe(next).unwrap(),
            InvalidationHint::Unchanged(next)
        );
    }

    #[test]
    fn placement_generation_subscriber_detects_coordinator_restart() {
        let mut subscriber = PlacementGenerationSubscriber::new();
        let high = PlacementGenerationSample::from_catalog_value(42).unwrap();
        subscriber.observe(high).unwrap();
        let low = PlacementGenerationSample::from_catalog_value(3).unwrap();
        assert_eq!(
            subscriber.observe(low).unwrap(),
            InvalidationHint::Reset {
                previous: high,
                current: low,
            }
        );
    }
}
