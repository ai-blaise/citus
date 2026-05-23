//! Hybrid logical clock sidecar contracts.

// FEATURE: S9

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
        validate_required("follower_read.replica", &self.replica)?;
        self.closed_timestamp.validate()?;
        if self.as_of > self.closed_timestamp.closed_at {
            return Err(HlcError::TimestampNotClosed);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum HlcError {
    ClockMovedBackwards,
    InvalidPhysicalTime,
    InvalidReplicaCount,
    InvalidStalenessBudget,
    LogicalCounterOverflow,
    MissingRequiredField(&'static str),
    TimestampNotClosed,
}

impl fmt::Display for HlcError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ClockMovedBackwards => write!(formatter, "clock moved beyond max offset"),
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
