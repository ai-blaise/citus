// FEATURE: T2

//! Prepared-statement cache per pool with placement-aware invalidation.
//!
//! Tracks the named prepared statements that have been sent to each upstream
//! backend. When a placement bumps generation, the affected statements are
//! flagged so the proxy sends a `DEALLOCATE` on the affected names and re-runs
//! `Parse` on the next `Execute`.

use crate::{CachedPlanGeneration, ShardMap, ShardMapError};
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

/// Single prepared statement scoped to one upstream backend + one settings
/// bucket.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PreparedStatement {
    pub backend_id: String,
    pub statement_name: String,
    pub query_text: String,
    pub shard_ids: Vec<u64>,
    pub generation: CachedPlanGeneration,
}

impl PreparedStatement {
    fn validate(&self) -> Result<(), PreparedCacheError> {
        if self.backend_id.trim().is_empty() {
            return Err(PreparedCacheError::MissingField("backend_id"));
        }
        if self.statement_name.trim().is_empty() {
            return Err(PreparedCacheError::MissingField("statement_name"));
        }
        if self.query_text.trim().is_empty() {
            return Err(PreparedCacheError::MissingField("query_text"));
        }
        if self.shard_ids.is_empty() {
            return Err(PreparedCacheError::MissingField("shard_ids"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct PreparedStatementCache {
    statements: BTreeMap<(String, String), PreparedStatement>,
    invalidated: BTreeSet<(String, String)>,
}

impl PreparedStatementCache {
    pub fn insert(&mut self, statement: PreparedStatement) -> Result<(), PreparedCacheError> {
        statement.validate()?;
        let key = (
            statement.backend_id.clone(),
            statement.statement_name.clone(),
        );
        self.invalidated.remove(&key);
        self.statements.insert(key, statement);
        Ok(())
    }

    pub fn get(&self, backend_id: &str, statement_name: &str) -> Option<&PreparedStatement> {
        self.statements
            .get(&(backend_id.to_string(), statement_name.to_string()))
    }

    /// Returns whether the named statement is currently invalidated and needs
    /// re-prepare on next execute.
    pub fn is_invalidated(&self, backend_id: &str, statement_name: &str) -> bool {
        self.invalidated
            .contains(&(backend_id.to_string(), statement_name.to_string()))
    }

    /// Mark every statement whose shard set intersects `changed_shards` as
    /// invalidated. Returns the deallocate plan: a list of
    /// `(backend_id, statement_name)` pairs the proxy must `DEALLOCATE` on the
    /// backend.
    pub fn invalidate_for_shards(&mut self, changed_shards: &[u64]) -> Vec<(String, String)> {
        let mut plan = Vec::new();
        for (key, statement) in &self.statements {
            if statement
                .shard_ids
                .iter()
                .any(|shard| changed_shards.contains(shard))
            {
                plan.push(key.clone());
            }
        }
        for key in &plan {
            self.invalidated.insert(key.clone());
        }
        plan
    }

    /// Verify the cached generation against the current `ShardMap` and
    /// invalidate stale statements. Equivalent to walking
    /// `invalidate_for_shards` for every shard whose generation moved.
    pub fn invalidate_for_shard_map(
        &mut self,
        shard_map: &ShardMap,
    ) -> Result<Vec<(String, String)>, ShardMapError> {
        let mut stale: Vec<(String, String)> = Vec::new();
        for (key, statement) in &self.statements {
            let current = shard_map.generation_for_shards(&statement.shard_ids)?;
            if current != statement.generation {
                stale.push(key.clone());
            }
        }
        for key in &stale {
            self.invalidated.insert(key.clone());
        }
        Ok(stale)
    }

    /// Confirm the proxy has executed `DEALLOCATE` for a statement; drops it
    /// from the cache so the next Bind triggers a fresh `Parse`.
    pub fn confirm_deallocated(&mut self, backend_id: &str, statement_name: &str) {
        let key = (backend_id.to_string(), statement_name.to_string());
        self.statements.remove(&key);
        self.invalidated.remove(&key);
    }

    pub fn len(&self) -> usize {
        self.statements.len()
    }

    pub fn is_empty(&self) -> bool {
        self.statements.is_empty()
    }

    pub fn invalidated_count(&self) -> usize {
        self.invalidated.len()
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum PreparedCacheError {
    MissingField(&'static str),
}

impl fmt::Display for PreparedCacheError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingField(field) => write!(formatter, "{field} must not be empty"),
        }
    }
}

impl Error for PreparedCacheError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Placement, ShardMap};

    fn shard_map(generation: u64) -> ShardMap {
        ShardMap::from_placements(vec![
            Placement::new(10, 1, "worker-a", 5432, generation).expect("placement"),
            Placement::new(20, 2, "worker-b", 5432, 1).expect("placement"),
        ])
        .expect("shard map")
    }

    fn statement(
        name: &str,
        shard_ids: Vec<u64>,
        generation: CachedPlanGeneration,
    ) -> PreparedStatement {
        PreparedStatement {
            backend_id: "backend-1".to_string(),
            statement_name: name.to_string(),
            query_text: "SELECT 1".to_string(),
            shard_ids,
            generation,
        }
    }

    #[test]
    fn insert_and_get_roundtrip() {
        let map = shard_map(1);
        let generation = map.generation_for_shards(&[10]).expect("generation");
        let mut cache = PreparedStatementCache::default();
        cache
            .insert(statement("orders_by_id", vec![10], generation))
            .expect("insert");
        assert!(cache.get("backend-1", "orders_by_id").is_some());
    }

    #[test]
    fn invalidate_for_shards_marks_affected() {
        let map = shard_map(1);
        let generation = map.generation_for_shards(&[10]).expect("generation");
        let mut cache = PreparedStatementCache::default();
        cache
            .insert(statement("orders_by_id", vec![10], generation.clone()))
            .expect("insert");
        cache
            .insert(statement("events_by_id", vec![20], generation))
            .expect("insert");

        let plan = cache.invalidate_for_shards(&[10]);
        assert_eq!(plan.len(), 1);
        assert_eq!(plan[0].1, "orders_by_id");
        assert!(cache.is_invalidated("backend-1", "orders_by_id"));
        assert!(!cache.is_invalidated("backend-1", "events_by_id"));
    }

    #[test]
    fn invalidate_for_shard_map_detects_generation_bump() {
        let before = shard_map(1);
        let after = shard_map(2);
        let generation = before.generation_for_shards(&[10]).expect("generation");
        let mut cache = PreparedStatementCache::default();
        cache
            .insert(statement("orders_by_id", vec![10], generation))
            .expect("insert");

        let stale = cache.invalidate_for_shard_map(&after).expect("invalidate");
        assert_eq!(stale.len(), 1);
        assert!(cache.is_invalidated("backend-1", "orders_by_id"));
    }

    #[test]
    fn confirm_deallocated_clears_state() {
        let map = shard_map(1);
        let generation = map.generation_for_shards(&[10]).expect("generation");
        let mut cache = PreparedStatementCache::default();
        cache
            .insert(statement("orders_by_id", vec![10], generation))
            .expect("insert");
        cache.invalidate_for_shards(&[10]);
        cache.confirm_deallocated("backend-1", "orders_by_id");

        assert!(cache.get("backend-1", "orders_by_id").is_none());
        assert!(!cache.is_invalidated("backend-1", "orders_by_id"));
        assert_eq!(cache.len(), 0);
    }

    #[test]
    fn empty_statement_rejected() {
        let mut cache = PreparedStatementCache::default();
        let error = cache
            .insert(PreparedStatement {
                backend_id: "".to_string(),
                statement_name: "x".to_string(),
                query_text: "SELECT 1".to_string(),
                shard_ids: vec![1],
                generation: CachedPlanGeneration {
                    generations: vec![],
                },
            })
            .expect_err("insert");
        assert!(matches!(error, PreparedCacheError::MissingField(_)));
    }
}
