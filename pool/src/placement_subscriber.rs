// FEATURE: T2

//! Logical-replication subscriber for `pg_dist_shard` + `pg_dist_placement`.
//!
//! Tracks placement generation deltas streamed from the Citus coordinator and
//! exposes a deterministic API for the plan-cache + prepared-statement
//! invalidators. The wire protocol bytes are handled by the proxy's
//! tokio-postgres replication path; this module owns the apply semantics so
//! the surface is testable without a live PostgreSQL.

use crate::{Placement, ShardMap, ShardMapError};
use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

/// Logical-replication delta for a single placement row.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum PlacementDelta {
    Upsert(Placement),
    Delete { shard_id: u64, node_id: u32 },
}

/// Subscriber state. Folds incoming deltas into a `ShardMap` and reports the
/// set of shards whose placement generation moved so plan caches +
/// prepared-statement caches can be invalidated only on the affected
/// fragments.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PlacementSubscriber {
    by_shard: BTreeMap<u64, Vec<Placement>>,
    applied_lsn: u64,
}

impl Default for PlacementSubscriber {
    fn default() -> Self {
        Self::new()
    }
}

impl PlacementSubscriber {
    pub fn new() -> Self {
        Self {
            by_shard: BTreeMap::new(),
            applied_lsn: 0,
        }
    }

    pub fn applied_lsn(&self) -> u64 {
        self.applied_lsn
    }

    pub fn shard_count(&self) -> usize {
        self.by_shard.len()
    }

    /// Apply a batch of deltas at `commit_lsn`. Returns the set of shards
    /// whose effective generation changed. Idempotent on
    /// `commit_lsn <= applied_lsn`.
    pub fn apply_batch(
        &mut self,
        commit_lsn: u64,
        deltas: Vec<PlacementDelta>,
    ) -> Result<Vec<u64>, PlacementSubscriberError> {
        if commit_lsn == 0 {
            return Err(PlacementSubscriberError::InvalidLsn);
        }
        if commit_lsn <= self.applied_lsn {
            return Ok(Vec::new());
        }

        let mut changed: BTreeMap<u64, u64> = BTreeMap::new();
        for delta in deltas {
            match delta {
                PlacementDelta::Upsert(placement) => {
                    let previous_generation = self
                        .by_shard
                        .get(&placement.shard_id)
                        .and_then(|placements| {
                            placements
                                .iter()
                                .map(|placement| placement.generation)
                                .max()
                        })
                        .unwrap_or(0);

                    let entries = self.by_shard.entry(placement.shard_id).or_default();
                    if let Some(existing) = entries
                        .iter_mut()
                        .find(|existing| existing.node_id == placement.node_id)
                    {
                        *existing = placement.clone();
                    } else {
                        entries.push(placement.clone());
                    }
                    entries.sort_by_key(|placement| placement.node_id);

                    let new_max = entries
                        .iter()
                        .map(|placement| placement.generation)
                        .max()
                        .unwrap_or(0);
                    if new_max != previous_generation {
                        changed.insert(placement.shard_id, new_max);
                    }
                }
                PlacementDelta::Delete { shard_id, node_id } => {
                    let entries = self
                        .by_shard
                        .get_mut(&shard_id)
                        .ok_or(PlacementSubscriberError::UnknownShard(shard_id))?;
                    let before_max = entries.iter().map(|placement| placement.generation).max();
                    entries.retain(|placement| placement.node_id != node_id);
                    if entries.is_empty() {
                        self.by_shard.remove(&shard_id);
                        changed.insert(shard_id, 0);
                    } else {
                        let after_max = entries.iter().map(|placement| placement.generation).max();
                        if before_max != after_max {
                            changed.insert(shard_id, after_max.unwrap_or(0));
                        }
                    }
                }
            }
        }

        self.applied_lsn = commit_lsn;
        Ok(changed.into_keys().collect())
    }

    /// Snapshot the current placement map. Returns an empty error if the
    /// subscriber holds no placements.
    pub fn snapshot(&self) -> Result<ShardMap, ShardMapError> {
        let placements = self
            .by_shard
            .values()
            .flat_map(|placements| placements.iter().cloned())
            .collect::<Vec<_>>();
        ShardMap::from_placements(placements)
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum PlacementSubscriberError {
    InvalidLsn,
    UnknownShard(u64),
}

impl fmt::Display for PlacementSubscriberError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidLsn => write!(formatter, "commit_lsn must be greater than zero"),
            Self::UnknownShard(shard_id) => write!(formatter, "unknown shard {shard_id}"),
        }
    }
}

impl Error for PlacementSubscriberError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn placement(shard_id: u64, node_id: u32, generation: u64) -> Placement {
        Placement::new(shard_id, node_id, "worker", 5432, generation).expect("placement")
    }

    #[test]
    fn initial_upsert_reports_changed_shard() {
        let mut subscriber = PlacementSubscriber::new();
        let changed = subscriber
            .apply_batch(1, vec![PlacementDelta::Upsert(placement(10, 1, 1))])
            .expect("apply");
        assert_eq!(changed, vec![10]);
        assert_eq!(subscriber.applied_lsn(), 1);
    }

    #[test]
    fn unchanged_generation_does_not_report_changed() {
        let mut subscriber = PlacementSubscriber::new();
        subscriber
            .apply_batch(1, vec![PlacementDelta::Upsert(placement(10, 1, 5))])
            .expect("apply-1");
        let changed = subscriber
            .apply_batch(2, vec![PlacementDelta::Upsert(placement(10, 1, 5))])
            .expect("apply-2");
        assert!(changed.is_empty());
    }

    #[test]
    fn generation_bump_reports_changed_shard() {
        let mut subscriber = PlacementSubscriber::new();
        subscriber
            .apply_batch(1, vec![PlacementDelta::Upsert(placement(10, 1, 1))])
            .expect("apply-1");
        let changed = subscriber
            .apply_batch(2, vec![PlacementDelta::Upsert(placement(10, 1, 7))])
            .expect("apply-2");
        assert_eq!(changed, vec![10]);
    }

    #[test]
    fn idempotent_on_replay() {
        let mut subscriber = PlacementSubscriber::new();
        subscriber
            .apply_batch(5, vec![PlacementDelta::Upsert(placement(10, 1, 1))])
            .expect("apply");
        let changed = subscriber
            .apply_batch(3, vec![PlacementDelta::Upsert(placement(10, 1, 99))])
            .expect("replay");
        assert!(changed.is_empty());
        assert_eq!(subscriber.applied_lsn(), 5);
    }

    #[test]
    fn delete_drops_shard_when_no_replicas_remain() {
        let mut subscriber = PlacementSubscriber::new();
        subscriber
            .apply_batch(1, vec![PlacementDelta::Upsert(placement(10, 1, 1))])
            .expect("apply-1");
        let changed = subscriber
            .apply_batch(
                2,
                vec![PlacementDelta::Delete {
                    shard_id: 10,
                    node_id: 1,
                }],
            )
            .expect("apply-2");
        assert_eq!(changed, vec![10]);
        assert_eq!(subscriber.shard_count(), 0);
    }

    #[test]
    fn delete_keeps_remaining_replicas() {
        let mut subscriber = PlacementSubscriber::new();
        subscriber
            .apply_batch(
                1,
                vec![
                    PlacementDelta::Upsert(placement(10, 1, 3)),
                    PlacementDelta::Upsert(placement(10, 2, 4)),
                ],
            )
            .expect("apply-1");
        let changed = subscriber
            .apply_batch(
                2,
                vec![PlacementDelta::Delete {
                    shard_id: 10,
                    node_id: 2,
                }],
            )
            .expect("apply-2");
        assert_eq!(changed, vec![10]);
        assert_eq!(subscriber.shard_count(), 1);
    }

    #[test]
    fn snapshot_round_trips_into_shard_map() {
        let mut subscriber = PlacementSubscriber::new();
        subscriber
            .apply_batch(
                1,
                vec![
                    PlacementDelta::Upsert(placement(10, 1, 3)),
                    PlacementDelta::Upsert(placement(20, 2, 1)),
                ],
            )
            .expect("apply");
        let snapshot = subscriber.snapshot().expect("snapshot");
        assert_eq!(snapshot.placements_for_shard(10).unwrap().len(), 1);
        assert_eq!(snapshot.placements_for_shard(20).unwrap().len(), 1);
    }

    #[test]
    fn invalid_lsn_rejected() {
        let mut subscriber = PlacementSubscriber::new();
        assert!(matches!(
            subscriber.apply_batch(0, vec![]),
            Err(PlacementSubscriberError::InvalidLsn)
        ));
    }
}
