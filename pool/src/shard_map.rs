// FEATURE: T2
// FEATURE: T3

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Placement {
    pub shard_id: u64,
    pub node_id: u32,
    pub host: String,
    pub port: u16,
    pub generation: u64,
}

impl Placement {
    pub fn new(
        shard_id: u64,
        node_id: u32,
        host: impl Into<String>,
        port: u16,
        generation: u64,
    ) -> Result<Self, ShardMapError> {
        let placement = Self {
            shard_id,
            node_id,
            host: host.into(),
            port,
            generation,
        };
        placement.validate()?;
        Ok(placement)
    }

    fn validate(&self) -> Result<(), ShardMapError> {
        if self.shard_id == 0 {
            return Err(ShardMapError::InvalidShardId);
        }
        if self.node_id == 0 {
            return Err(ShardMapError::InvalidNodeId);
        }
        if self.host.trim().is_empty() {
            return Err(ShardMapError::MissingHost);
        }
        if self.port == 0 {
            return Err(ShardMapError::InvalidPort);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ShardMap {
    placements_by_shard: BTreeMap<u64, Vec<Placement>>,
}

impl ShardMap {
    pub fn from_placements(placements: Vec<Placement>) -> Result<Self, ShardMapError> {
        let mut placements_by_shard: BTreeMap<u64, Vec<Placement>> = BTreeMap::new();

        for placement in placements {
            placement.validate()?;
            placements_by_shard
                .entry(placement.shard_id)
                .or_default()
                .push(placement);
        }

        if placements_by_shard.is_empty() {
            return Err(ShardMapError::EmptyShardMap);
        }

        Ok(Self {
            placements_by_shard,
        })
    }

    pub fn placements_for_shard(&self, shard_id: u64) -> Result<&[Placement], ShardMapError> {
        self.placements_by_shard
            .get(&shard_id)
            .map(Vec::as_slice)
            .ok_or(ShardMapError::UnknownShard(shard_id))
    }

    pub fn generation_for_shards(
        &self,
        shard_ids: &[u64],
    ) -> Result<CachedPlanGeneration, ShardMapError> {
        if shard_ids.is_empty() {
            return Err(ShardMapError::EmptyPlanShardSet);
        }

        let mut generations = Vec::with_capacity(shard_ids.len());
        for shard_id in shard_ids {
            let max_generation = self
                .placements_for_shard(*shard_id)?
                .iter()
                .map(|placement| placement.generation)
                .max()
                .ok_or(ShardMapError::UnknownShard(*shard_id))?;

            generations.push(PlacementGeneration {
                shard_id: *shard_id,
                generation: max_generation,
            });
        }

        generations.sort_unstable_by_key(|generation| generation.shard_id);
        generations.dedup_by_key(|generation| generation.shard_id);

        Ok(CachedPlanGeneration { generations })
    }

    pub fn single_shard_route(&self, shard_id: u64) -> Result<ShardRoute, ShardMapError> {
        let placement = self
            .placements_for_shard(shard_id)?
            .iter()
            .max_by_key(|placement| (placement.generation, std::cmp::Reverse(placement.node_id)))
            .ok_or(ShardMapError::UnknownShard(shard_id))?;

        Ok(ShardRoute {
            shard_id,
            node_id: placement.node_id,
            host: placement.host.clone(),
            port: placement.port,
            generation: placement.generation,
        })
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PlacementGeneration {
    pub shard_id: u64,
    pub generation: u64,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct CachedPlanGeneration {
    pub generations: Vec<PlacementGeneration>,
}

impl CachedPlanGeneration {
    pub fn is_valid_for(&self, shard_map: &ShardMap) -> bool {
        self.generations.iter().all(|cached_generation| {
            shard_map
                .generation_for_shards(&[cached_generation.shard_id])
                .map(|current_generation| {
                    current_generation.generations == vec![cached_generation.clone()]
                })
                .unwrap_or(false)
        })
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct CachedPlan {
    pub query_fingerprint: String,
    pub shard_ids: Vec<u64>,
    pub generation: CachedPlanGeneration,
}

#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct PlanCache {
    plans: BTreeMap<String, CachedPlan>,
}

impl PlanCache {
    pub fn upsert(
        &mut self,
        query_fingerprint: impl Into<String>,
        shard_ids: Vec<u64>,
        shard_map: &ShardMap,
    ) -> Result<(), ShardMapError> {
        let query_fingerprint = query_fingerprint.into();
        if query_fingerprint.trim().is_empty() {
            return Err(ShardMapError::MissingQueryFingerprint);
        }
        let generation = shard_map.generation_for_shards(&shard_ids)?;
        self.plans.insert(
            query_fingerprint.clone(),
            CachedPlan {
                query_fingerprint,
                shard_ids,
                generation,
            },
        );
        Ok(())
    }

    pub fn get_valid<'a>(
        &'a self,
        query_fingerprint: &str,
        shard_map: &ShardMap,
    ) -> Result<Option<&'a CachedPlan>, ShardMapError> {
        if query_fingerprint.trim().is_empty() {
            return Err(ShardMapError::MissingQueryFingerprint);
        }
        Ok(self
            .plans
            .get(query_fingerprint)
            .filter(|plan| plan.generation.is_valid_for(shard_map)))
    }

    pub fn invalidate_for_shards(&mut self, shard_ids: &[u64]) -> usize {
        let before = self.plans.len();
        self.plans.retain(|_, plan| {
            !plan
                .shard_ids
                .iter()
                .any(|plan_shard_id| shard_ids.contains(plan_shard_id))
        });
        before - self.plans.len()
    }

    pub fn len(&self) -> usize {
        self.plans.len()
    }

    pub fn is_empty(&self) -> bool {
        self.plans.is_empty()
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ShardRoute {
    pub shard_id: u64,
    pub node_id: u32,
    pub host: String,
    pub port: u16,
    pub generation: u64,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum ShardMapError {
    EmptyPlanShardSet,
    EmptyShardMap,
    InvalidNodeId,
    InvalidPort,
    InvalidShardId,
    MissingHost,
    MissingQueryFingerprint,
    UnknownShard(u64),
}

impl fmt::Display for ShardMapError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyPlanShardSet => write!(formatter, "plan must reference at least one shard"),
            Self::EmptyShardMap => write!(formatter, "shard map must contain at least one shard"),
            Self::InvalidNodeId => write!(formatter, "node_id must be greater than zero"),
            Self::InvalidPort => write!(formatter, "port must be greater than zero"),
            Self::InvalidShardId => write!(formatter, "shard_id must be greater than zero"),
            Self::MissingHost => write!(formatter, "host must not be empty"),
            Self::MissingQueryFingerprint => {
                write!(formatter, "query fingerprint must not be empty")
            }
            Self::UnknownShard(shard_id) => write!(formatter, "unknown shard {shard_id}"),
        }
    }
}

impl Error for ShardMapError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generation_for_shards_uses_highest_placement_generation() {
        let shard_map = ShardMap::from_placements(vec![
            Placement::new(10, 1, "worker-a", 5432, 4).expect("placement"),
            Placement::new(10, 2, "worker-b", 5432, 7).expect("placement"),
            Placement::new(20, 3, "worker-c", 5432, 2).expect("placement"),
        ])
        .expect("shard map");

        let generation = shard_map
            .generation_for_shards(&[20, 10])
            .expect("generation");

        assert_eq!(
            generation,
            CachedPlanGeneration {
                generations: vec![
                    PlacementGeneration {
                        shard_id: 10,
                        generation: 7,
                    },
                    PlacementGeneration {
                        shard_id: 20,
                        generation: 2,
                    },
                ],
            }
        );
    }

    #[test]
    fn single_shard_route_uses_latest_placement_generation() {
        let shard_map = ShardMap::from_placements(vec![
            Placement::new(10, 1, "worker-a", 5432, 4).expect("placement"),
            Placement::new(10, 2, "worker-b", 5433, 7).expect("placement"),
        ])
        .expect("shard map");

        assert_eq!(
            shard_map.single_shard_route(10),
            Ok(ShardRoute {
                shard_id: 10,
                node_id: 2,
                host: "worker-b".to_string(),
                port: 5433,
                generation: 7,
            })
        );
    }

    #[test]
    fn plan_cache_returns_only_generation_valid_plans() {
        let before = ShardMap::from_placements(vec![
            Placement::new(10, 1, "worker-a", 5432, 4).expect("placement"),
            Placement::new(20, 2, "worker-b", 5432, 1).expect("placement"),
        ])
        .expect("shard map");
        let mut cache = PlanCache::default();
        cache
            .upsert("select:orders-by-tenant", vec![10], &before)
            .expect("cache insert");
        cache
            .upsert("select:events-by-tenant", vec![20], &before)
            .expect("cache insert");

        assert!(cache
            .get_valid("select:orders-by-tenant", &before)
            .expect("valid lookup")
            .is_some());

        let after = ShardMap::from_placements(vec![
            Placement::new(10, 1, "worker-a", 5432, 5).expect("placement"),
            Placement::new(20, 2, "worker-b", 5432, 1).expect("placement"),
        ])
        .expect("shard map");

        assert!(cache
            .get_valid("select:orders-by-tenant", &after)
            .expect("stale lookup")
            .is_none());
        assert!(cache
            .get_valid("select:events-by-tenant", &after)
            .expect("fresh lookup")
            .is_some());
    }

    #[test]
    fn plan_cache_invalidates_only_affected_shards() {
        let shard_map = ShardMap::from_placements(vec![
            Placement::new(10, 1, "worker-a", 5432, 4).expect("placement"),
            Placement::new(20, 2, "worker-b", 5432, 1).expect("placement"),
        ])
        .expect("shard map");
        let mut cache = PlanCache::default();
        cache
            .upsert("query-a", vec![10], &shard_map)
            .expect("query-a");
        cache
            .upsert("query-b", vec![20], &shard_map)
            .expect("query-b");

        assert_eq!(cache.invalidate_for_shards(&[10]), 1);
        assert_eq!(cache.len(), 1);
        assert!(cache
            .get_valid("query-b", &shard_map)
            .expect("query-b")
            .is_some());
    }

    #[test]
    fn cached_generation_detects_rebalance() {
        let before = ShardMap::from_placements(vec![
            Placement::new(10, 1, "worker-a", 5432, 4).expect("placement")
        ])
        .expect("shard map");
        let cached = before.generation_for_shards(&[10]).expect("generation");

        let after = ShardMap::from_placements(vec![
            Placement::new(10, 2, "worker-b", 5432, 5).expect("placement")
        ])
        .expect("shard map");

        assert!(cached.is_valid_for(&before));
        assert!(!cached.is_valid_for(&after));
    }

    #[test]
    fn unknown_shard_invalidates_cached_generation() {
        let shard_map = ShardMap::from_placements(vec![
            Placement::new(10, 1, "worker-a", 5432, 4).expect("placement")
        ])
        .expect("shard map");
        let cached = CachedPlanGeneration {
            generations: vec![PlacementGeneration {
                shard_id: 99,
                generation: 1,
            }],
        };

        assert!(!cached.is_valid_for(&shard_map));
    }
}
