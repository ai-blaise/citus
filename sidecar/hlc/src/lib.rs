//! Hybrid logical clock sidecar contracts.

// FEATURE: S9
// FEATURE: Edge1

pub mod runtime;

pub use runtime::{
    canonical_hlc_runtime_report, render_closed_ts_json, HlcRuntime, HlcRuntimeError,
    HlcRuntimeReport, PeerClockExchange,
};

use std::cmp::Ordering;
use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct HlcTimestamp {
    pub physical_ms: u64,
    pub logical: u32,
}

impl HlcTimestamp {
    pub fn new(physical_ms: u64, logical: u32) -> Result<Self, HlcError> {
        if physical_ms == 0 {
            return Err(HlcError::InvalidPhysicalTime);
        }
        Ok(Self {
            physical_ms,
            logical,
        })
    }
}

impl Ord for HlcTimestamp {
    fn cmp(&self, other: &Self) -> Ordering {
        self.physical_ms
            .cmp(&other.physical_ms)
            .then_with(|| self.logical.cmp(&other.logical))
    }
}

impl PartialOrd for HlcTimestamp {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct HlcClock {
    pub node_id: String,
    pub timestamp: HlcTimestamp,
    pub max_offset_ms: u64,
}

impl HlcClock {
    pub fn tick(&mut self, physical_ms: u64) -> Result<HlcTimestamp, HlcError> {
        self.validate_physical_time(physical_ms)?;
        if physical_ms > self.timestamp.physical_ms {
            self.timestamp = HlcTimestamp {
                physical_ms,
                logical: 0,
            };
        } else {
            self.timestamp.logical = self
                .timestamp
                .logical
                .checked_add(1)
                .ok_or(HlcError::LogicalCounterOverflow)?;
        }
        Ok(self.timestamp)
    }

    pub fn observe(
        &mut self,
        remote: HlcTimestamp,
        physical_ms: u64,
    ) -> Result<HlcTimestamp, HlcError> {
        self.validate_physical_time(physical_ms)?;
        let max_physical = self
            .timestamp
            .physical_ms
            .max(remote.physical_ms)
            .max(physical_ms);

        let logical =
            if max_physical == self.timestamp.physical_ms && max_physical == remote.physical_ms {
                self.timestamp
                    .logical
                    .max(remote.logical)
                    .checked_add(1)
                    .ok_or(HlcError::LogicalCounterOverflow)?
            } else if max_physical == self.timestamp.physical_ms {
                self.timestamp
                    .logical
                    .checked_add(1)
                    .ok_or(HlcError::LogicalCounterOverflow)?
            } else if max_physical == remote.physical_ms {
                remote
                    .logical
                    .checked_add(1)
                    .ok_or(HlcError::LogicalCounterOverflow)?
            } else {
                0
            };

        self.timestamp = HlcTimestamp {
            physical_ms: max_physical,
            logical,
        };
        Ok(self.timestamp)
    }

    fn validate_physical_time(&self, physical_ms: u64) -> Result<(), HlcError> {
        if physical_ms == 0 {
            return Err(HlcError::InvalidPhysicalTime);
        }
        if physical_ms + self.max_offset_ms < self.timestamp.physical_ms {
            return Err(HlcError::ClockMovedBackwards);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ClosedTimestampPlan {
    pub shard_group: String,
    pub closed_at: HlcTimestamp,
    pub max_staleness_ms: u64,
    pub replica_count: u32,
}

impl ClosedTimestampPlan {
    pub fn validate(&self) -> Result<(), HlcError> {
        validate_required("closed_timestamp.shard_group", &self.shard_group)?;
        if self.max_staleness_ms == 0 {
            return Err(HlcError::InvalidStalenessBudget);
        }
        if self.replica_count == 0 {
            return Err(HlcError::InvalidReplicaCount);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct FollowerReadPlan {
    pub replica: String,
    pub as_of: HlcTimestamp,
    pub closed_timestamp: ClosedTimestampPlan,
}

impl FollowerReadPlan {
    pub fn validate(&self) -> Result<(), HlcError> {
        match self.decision()? {
            FollowerReadDecision::ServeFromFollower { .. } => Ok(()),
            FollowerReadDecision::RejectNotClosed { .. } => Err(HlcError::TimestampNotClosed),
        }
    }

    pub fn decision(&self) -> Result<FollowerReadDecision, HlcError> {
        validate_required("follower_read.replica", &self.replica)?;
        self.closed_timestamp.validate()?;
        if self.as_of > self.closed_timestamp.closed_at {
            return Ok(FollowerReadDecision::RejectNotClosed {
                replica: self.replica.clone(),
                as_of: self.as_of,
                closed_at: self.closed_timestamp.closed_at,
            });
        }
        Ok(FollowerReadDecision::ServeFromFollower {
            replica: self.replica.clone(),
            as_of: self.as_of,
            closed_at: self.closed_timestamp.closed_at,
        })
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum FollowerReadDecision {
    ServeFromFollower {
        replica: String,
        as_of: HlcTimestamp,
        closed_at: HlcTimestamp,
    },
    RejectNotClosed {
        replica: String,
        as_of: HlcTimestamp,
        closed_at: HlcTimestamp,
    },
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct EdgeReadPlan {
    pub edge_region: String,
    pub replica: String,
    pub expected_replica: String,
    pub as_of: HlcTimestamp,
    pub closed_timestamp: ClosedTimestampPlan,
}

impl EdgeReadPlan {
    pub fn validate(&self) -> Result<(), HlcError> {
        match self.decision()? {
            EdgeReadDecision::ServeFromEdge { .. } => Ok(()),
            EdgeReadDecision::RejectReplicaMismatch { .. } => Err(HlcError::EdgeReplicaMismatch),
            EdgeReadDecision::RejectNotClosed { .. } => Err(HlcError::TimestampNotClosed),
            EdgeReadDecision::RejectTooStale { .. } => Err(HlcError::TimestampTooStale),
        }
    }

    pub fn decision(&self) -> Result<EdgeReadDecision, HlcError> {
        validate_required("edge_read.edge_region", &self.edge_region)?;
        validate_required("edge_read.replica", &self.replica)?;
        validate_required("edge_read.expected_replica", &self.expected_replica)?;
        self.closed_timestamp.validate()?;
        if self.replica != self.expected_replica {
            return Ok(EdgeReadDecision::RejectReplicaMismatch {
                edge_region: self.edge_region.clone(),
                requested_replica: self.replica.clone(),
                expected_replica: self.expected_replica.clone(),
            });
        }
        if self.as_of > self.closed_timestamp.closed_at {
            return Ok(EdgeReadDecision::RejectNotClosed {
                edge_region: self.edge_region.clone(),
                replica: self.replica.clone(),
                as_of: self.as_of,
                closed_at: self.closed_timestamp.closed_at,
            });
        }
        let observed_staleness_ms = self
            .closed_timestamp
            .closed_at
            .physical_ms
            .saturating_sub(self.as_of.physical_ms);
        if observed_staleness_ms > self.closed_timestamp.max_staleness_ms {
            return Ok(EdgeReadDecision::RejectTooStale {
                edge_region: self.edge_region.clone(),
                replica: self.replica.clone(),
                as_of: self.as_of,
                closed_at: self.closed_timestamp.closed_at,
                max_staleness_ms: self.closed_timestamp.max_staleness_ms,
                observed_staleness_ms,
            });
        }
        Ok(EdgeReadDecision::ServeFromEdge {
            edge_region: self.edge_region.clone(),
            replica: self.replica.clone(),
            as_of: self.as_of,
            closed_at: self.closed_timestamp.closed_at,
            max_staleness_ms: self.closed_timestamp.max_staleness_ms,
            observed_staleness_ms,
        })
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum EdgeReadDecision {
    ServeFromEdge {
        edge_region: String,
        replica: String,
        as_of: HlcTimestamp,
        closed_at: HlcTimestamp,
        max_staleness_ms: u64,
        observed_staleness_ms: u64,
    },
    RejectReplicaMismatch {
        edge_region: String,
        requested_replica: String,
        expected_replica: String,
    },
    RejectNotClosed {
        edge_region: String,
        replica: String,
        as_of: HlcTimestamp,
        closed_at: HlcTimestamp,
    },
    RejectTooStale {
        edge_region: String,
        replica: String,
        as_of: HlcTimestamp,
        closed_at: HlcTimestamp,
        max_staleness_ms: u64,
        observed_staleness_ms: u64,
    },
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum HlcError {
    ClockMovedBackwards,
    EdgeReplicaMismatch,
    InvalidPhysicalTime,
    InvalidReplicaCount,
    InvalidStalenessBudget,
    LogicalCounterOverflow,
    MissingRequiredField(&'static str),
    TimestampNotClosed,
    TimestampTooStale,
}

impl fmt::Display for HlcError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ClockMovedBackwards => write!(formatter, "clock moved beyond max offset"),
            Self::EdgeReplicaMismatch => write!(formatter, "edge replica does not match region"),
            Self::InvalidPhysicalTime => write!(formatter, "physical_ms must be greater than zero"),
            Self::InvalidReplicaCount => {
                write!(formatter, "replica_count must be greater than zero")
            }
            Self::InvalidStalenessBudget => {
                write!(formatter, "max_staleness_ms must be greater than zero")
            }
            Self::LogicalCounterOverflow => write!(formatter, "logical counter overflow"),
            Self::MissingRequiredField(field) => write!(formatter, "{field} must not be empty"),
            Self::TimestampNotClosed => {
                write!(formatter, "AS OF timestamp is newer than closed timestamp")
            }
            Self::TimestampTooStale => {
                write!(formatter, "AS OF timestamp exceeds max staleness budget")
            }
        }
    }
}

impl Error for HlcError {}

fn validate_required(field: &'static str, value: &str) -> Result<(), HlcError> {
    if value.trim().is_empty() {
        return Err(HlcError::MissingRequiredField(field));
    }
    Ok(())
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct HlcCanonicalReport {
    pub ticked_clock: HlcClock,
    pub observed_clock: HlcClock,
    pub follower_read: FollowerReadPlan,
}

pub fn canonical_clock() -> HlcClock {
    HlcClock {
        node_id: "worker-a".to_string(),
        timestamp: HlcTimestamp {
            physical_ms: 1_700_000_000,
            logical: 1,
        },
        max_offset_ms: 500,
    }
}

pub fn canonical_closed_timestamp() -> ClosedTimestampPlan {
    ClosedTimestampPlan {
        shard_group: "orders-sg".to_string(),
        closed_at: HlcTimestamp {
            physical_ms: 1_700_000_000,
            logical: 10,
        },
        max_staleness_ms: 5_000,
        replica_count: 3,
    }
}

pub fn canonical_follower_read_plan() -> FollowerReadPlan {
    FollowerReadPlan {
        replica: "worker-a-replica".to_string(),
        as_of: HlcTimestamp {
            physical_ms: 1_700_000_000,
            logical: 9,
        },
        closed_timestamp: canonical_closed_timestamp(),
    }
}

pub fn canonical_hlc_report() -> Result<HlcCanonicalReport, HlcError> {
    let mut ticked_clock = canonical_clock();
    ticked_clock.tick(1_700_000_001)?;

    let mut observed_clock = canonical_clock();
    observed_clock.observe(
        HlcTimestamp {
            physical_ms: 1_700_000_000,
            logical: 5,
        },
        1_700_000_000,
    )?;

    let follower_read = canonical_follower_read_plan();
    follower_read.validate()?;

    Ok(HlcCanonicalReport {
        ticked_clock,
        observed_clock,
        follower_read,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tick_advances_physical_time_and_resets_logical() {
        let mut clock = valid_clock();

        let timestamp = clock.tick(1_700_000_001).expect("tick");

        assert_eq!(
            timestamp,
            HlcTimestamp {
                physical_ms: 1_700_000_001,
                logical: 0,
            }
        );
    }

    #[test]
    fn observe_merges_remote_timestamp() {
        let mut clock = valid_clock();
        let remote = HlcTimestamp {
            physical_ms: 1_700_000_000,
            logical: 5,
        };

        let timestamp = clock.observe(remote, 1_700_000_000).expect("observe");

        assert_eq!(
            timestamp,
            HlcTimestamp {
                physical_ms: 1_700_000_000,
                logical: 6,
            }
        );
    }

    #[test]
    fn follower_read_requires_closed_timestamp() {
        let plan = FollowerReadPlan {
            replica: "worker-a-replica".to_string(),
            as_of: HlcTimestamp {
                physical_ms: 1_700_000_010,
                logical: 0,
            },
            closed_timestamp: valid_closed_timestamp(),
        };

        assert_eq!(plan.validate(), Err(HlcError::TimestampNotClosed));
        assert!(matches!(
            plan.decision(),
            Ok(FollowerReadDecision::RejectNotClosed { .. })
        ));
    }

    #[test]
    fn follower_read_decision_serves_closed_timestamp() {
        let plan = canonical_follower_read_plan();

        assert_eq!(
            plan.decision(),
            Ok(FollowerReadDecision::ServeFromFollower {
                replica: "worker-a-replica".to_string(),
                as_of: HlcTimestamp {
                    physical_ms: 1_700_000_000,
                    logical: 9,
                },
                closed_at: HlcTimestamp {
                    physical_ms: 1_700_000_000,
                    logical: 10,
                },
            })
        );
    }

    #[test]
    fn edge_read_serves_only_matching_closed_bounded_staleness() {
        let plan = EdgeReadPlan {
            edge_region: "iad-edge".to_string(),
            replica: "worker-a-replica".to_string(),
            expected_replica: "worker-a-replica".to_string(),
            as_of: HlcTimestamp {
                physical_ms: 1_699_999_500,
                logical: 0,
            },
            closed_timestamp: valid_closed_timestamp(),
        };

        assert_eq!(
            plan.decision(),
            Ok(EdgeReadDecision::ServeFromEdge {
                edge_region: "iad-edge".to_string(),
                replica: "worker-a-replica".to_string(),
                as_of: HlcTimestamp {
                    physical_ms: 1_699_999_500,
                    logical: 0,
                },
                closed_at: HlcTimestamp {
                    physical_ms: 1_700_000_000,
                    logical: 10,
                },
                max_staleness_ms: 5_000,
                observed_staleness_ms: 500,
            })
        );

        let mut too_new = plan.clone();
        too_new.as_of = HlcTimestamp {
            physical_ms: 1_700_000_001,
            logical: 0,
        };
        assert!(matches!(
            too_new.decision(),
            Ok(EdgeReadDecision::RejectNotClosed { .. })
        ));

        let mut too_stale = plan.clone();
        too_stale.as_of = HlcTimestamp {
            physical_ms: 1_699_990_000,
            logical: 0,
        };
        assert_eq!(too_stale.validate(), Err(HlcError::TimestampTooStale));
        assert!(matches!(
            too_stale.decision(),
            Ok(EdgeReadDecision::RejectTooStale { .. })
        ));

        let mut mismatch = plan;
        mismatch.replica = "worker-b-replica".to_string();
        assert_eq!(mismatch.validate(), Err(HlcError::EdgeReplicaMismatch));
    }

    #[test]
    fn closed_timestamp_requires_replica_count() {
        let mut plan = valid_closed_timestamp();
        plan.replica_count = 0;

        assert_eq!(plan.validate(), Err(HlcError::InvalidReplicaCount));
    }

    #[test]
    fn canonical_report_is_deterministic() {
        let report = canonical_hlc_report().expect("canonical report");

        assert_eq!(report.ticked_clock.timestamp.physical_ms, 1_700_000_001);
        assert_eq!(report.observed_clock.timestamp.logical, 6);
        assert_eq!(report.follower_read.replica, "worker-a-replica");
    }

    fn valid_clock() -> HlcClock {
        canonical_clock()
    }

    fn valid_closed_timestamp() -> ClosedTimestampPlan {
        canonical_closed_timestamp()
    }
}
