// FEATURE: T2

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
pub enum ShardMapError {
    EmptyPlanShardSet,
    EmptyShardMap,
    InvalidNodeId,
    InvalidPort,
    InvalidShardId,
    MissingHost,
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
