// FEATURE: T1

//! Settings-bucket pool fingerprinting and per-bucket backend connection
//! accounting.
//!
//! The pool reuses one backend connection across many client sessions when the
//! tracked `citus.*` GUC set matches. Each unique fingerprint gets its own
//! pool of backend connections capped by the policy's `max_connections`.

use crate::{PoolRuntimeError, SessionSetting, SettingsBucketPolicy};
use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

/// Per-fingerprint state tracked inside a `SettingsBucketPoolMap`.
///
/// `assigned` holds the count of client sessions currently bound to a backend
/// in this bucket; backends are reusable once a client unbinds.
#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct SettingsBucketEntry {
    pub fingerprint: String,
    pub assigned: u32,
    pub borrowed_total: u64,
}

/// Map of fingerprint to bucket state with a shared per-bucket connection cap.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SettingsBucketPoolMap {
    policy: SettingsBucketPolicy,
    entries: BTreeMap<String, SettingsBucketEntry>,
}

impl SettingsBucketPoolMap {
    pub fn new(policy: SettingsBucketPolicy) -> Result<Self, PoolRuntimeError> {
        policy.validate()?;
        Ok(Self {
            policy,
            entries: BTreeMap::new(),
        })
    }

    pub fn policy(&self) -> &SettingsBucketPolicy {
        &self.policy
    }

    /// Compute the fingerprint for the supplied session settings using the
    /// stored policy. Equivalent to `SettingsBucketPolicy::fingerprint`.
    pub fn fingerprint(&self, settings: &[SessionSetting]) -> Result<String, PoolRuntimeError> {
        self.policy.fingerprint(settings)
    }

    /// Acquire a backend slot for the supplied session settings, returning the
    /// bucket fingerprint plus a snapshot of the resulting entry.
    pub fn acquire(
        &mut self,
        settings: &[SessionSetting],
    ) -> Result<SettingsBucketEntry, SettingsBucketError> {
        let fingerprint = self
            .policy
            .fingerprint(settings)
            .map_err(SettingsBucketError::Runtime)?;

        let max_connections = self.policy.max_connections;
        let entry =
            self.entries
                .entry(fingerprint.clone())
                .or_insert_with(|| SettingsBucketEntry {
                    fingerprint: fingerprint.clone(),
                    assigned: 0,
                    borrowed_total: 0,
                });

        if entry.assigned >= max_connections {
            return Err(SettingsBucketError::PoolExhausted {
                fingerprint,
                max_connections,
            });
        }

        entry.assigned += 1;
        entry.borrowed_total += 1;
        Ok(entry.clone())
    }

    /// Release a previously-acquired backend slot.
    pub fn release(
        &mut self,
        fingerprint: &str,
    ) -> Result<SettingsBucketEntry, SettingsBucketError> {
        let entry = self
            .entries
            .get_mut(fingerprint)
            .ok_or_else(|| SettingsBucketError::UnknownFingerprint(fingerprint.to_string()))?;
        if entry.assigned == 0 {
            return Err(SettingsBucketError::ReleaseUnderflow(
                fingerprint.to_string(),
            ));
        }
        entry.assigned -= 1;
        Ok(entry.clone())
    }

    pub fn entries(&self) -> impl Iterator<Item = &SettingsBucketEntry> {
        self.entries.values()
    }

    pub fn bucket_count(&self) -> usize {
        self.entries.len()
    }

    pub fn total_assigned(&self) -> u32 {
        self.entries.values().map(|entry| entry.assigned).sum()
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum SettingsBucketError {
    PoolExhausted {
        fingerprint: String,
        max_connections: u32,
    },
    ReleaseUnderflow(String),
    Runtime(PoolRuntimeError),
    UnknownFingerprint(String),
}

impl fmt::Display for SettingsBucketError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PoolExhausted {
                fingerprint,
                max_connections,
            } => write!(
                formatter,
                "settings bucket {fingerprint} exhausted at {max_connections} backends"
            ),
            Self::ReleaseUnderflow(fingerprint) => write!(
                formatter,
                "release underflow for settings bucket {fingerprint}"
            ),
            Self::Runtime(error) => write!(formatter, "{error}"),
            Self::UnknownFingerprint(fingerprint) => {
                write!(
                    formatter,
                    "unknown settings bucket fingerprint {fingerprint}"
                )
            }
        }
    }
}

impl Error for SettingsBucketError {}

impl From<PoolRuntimeError> for SettingsBucketError {
    fn from(error: PoolRuntimeError) -> Self {
        Self::Runtime(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn policy(max: u32) -> SettingsBucketPolicy {
        SettingsBucketPolicy {
            bucket_name: "test".to_string(),
            tracked_gucs: vec!["citus.enable_repartition_joins".to_string()],
            max_connections: max,
        }
    }

    fn setting(name: &str, value: &str) -> SessionSetting {
        SessionSetting {
            name: name.to_string(),
            value: value.to_string(),
        }
    }

    #[test]
    fn same_fingerprint_reuses_bucket() {
        let mut map = SettingsBucketPoolMap::new(policy(4)).expect("policy");
        let settings = vec![setting("citus.enable_repartition_joins", "off")];
        let first = map.acquire(&settings).expect("acquire-1");
        let second = map.acquire(&settings).expect("acquire-2");

        assert_eq!(first.fingerprint, second.fingerprint);
        assert_eq!(map.bucket_count(), 1);
        assert_eq!(map.total_assigned(), 2);
    }

    #[test]
    fn different_fingerprints_get_distinct_buckets() {
        let mut map = SettingsBucketPoolMap::new(policy(4)).expect("policy");
        let off = vec![setting("citus.enable_repartition_joins", "off")];
        let on = vec![setting("citus.enable_repartition_joins", "on")];

        let off_entry = map.acquire(&off).expect("acquire-off");
        let on_entry = map.acquire(&on).expect("acquire-on");

        assert_ne!(off_entry.fingerprint, on_entry.fingerprint);
        assert_eq!(map.bucket_count(), 2);
    }

    #[test]
    fn pool_exhaustion_returns_error() {
        let mut map = SettingsBucketPoolMap::new(policy(1)).expect("policy");
        let settings = vec![setting("citus.enable_repartition_joins", "off")];

        map.acquire(&settings).expect("acquire-1");
        let error = map.acquire(&settings).expect_err("acquire-2");

        assert!(matches!(
            error,
            SettingsBucketError::PoolExhausted {
                max_connections: 1,
                ..
            }
        ));
    }

    #[test]
    fn release_lets_pool_admit_new_session() {
        let mut map = SettingsBucketPoolMap::new(policy(1)).expect("policy");
        let settings = vec![setting("citus.enable_repartition_joins", "off")];

        let first = map.acquire(&settings).expect("acquire-1");
        map.release(&first.fingerprint).expect("release-1");
        let second = map.acquire(&settings).expect("acquire-2");
        assert_eq!(second.assigned, 1);
        assert_eq!(second.borrowed_total, 2);
    }

    #[test]
    fn release_unknown_fingerprint_is_rejected() {
        let mut map = SettingsBucketPoolMap::new(policy(1)).expect("policy");
        let error = map.release("missing").expect_err("release");
        assert!(matches!(error, SettingsBucketError::UnknownFingerprint(_)));
    }

    #[test]
    fn release_underflow_is_rejected() {
        let mut map = SettingsBucketPoolMap::new(policy(2)).expect("policy");
        let settings = vec![setting("citus.enable_repartition_joins", "off")];
        let entry = map.acquire(&settings).expect("acquire");
        map.release(&entry.fingerprint).expect("release-1");
        let error = map.release(&entry.fingerprint).expect_err("release-2");
        assert!(matches!(error, SettingsBucketError::ReleaseUnderflow(_)));
    }
}
