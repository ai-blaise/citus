//! ai-blaise Citus companion extension core.

pub mod citus_timescale;

pub use citus_timescale::{
    distribute_hypertable_plan, AddContinuousAggregateDistributedPlan, AddPolicyDistributed,
    AddPolicyDistributedPlan, CompanionError, DistributedHypertablePlan, TimeRangeShardPrunerPlan,
    FEATURE_DISTRIBUTE_HYPERTABLE, FEATURE_TIME_RANGE_SHARD_PRUNER,
};

#[cfg(feature = "pg18")]
mod pg18 {
    use pgrx::prelude::*;

    pgrx::pg_module_magic!();

    #[pg_extern]
    fn companion_feature_status() -> TableIterator<
        'static,
        (
            name!(feature_id, &'static str),
            name!(feature_name, &'static str),
            name!(status, &'static str),
        ),
    > {
        TableIterator::new(vec![
            ("TS1", "distributed hypertable bridge", "planned"),
            ("TS5", "time-range shard pruner", "planned"),
        ])
    }
}
