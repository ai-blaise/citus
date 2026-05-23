// FEATURE: Auth3

//! JWT verification cache for `sidecar/auth` integration.
//!
//! The pool calls `sidecar/auth` `/auth/verify` on every new client
//! connection. The response (a small claim envelope) is cached by JTI for the
//! configured TTL so the auth round trip only fires once per `(JTI, TTL)`
//! window. Revoked tokens are invalidated immediately on the sidecar push
//! channel; expired tokens are evicted lazily.
//!
//! This module owns the cache logic. The actual HTTP/JSON call is performed
//! by the proxy hot path so the sidecar/auth concurrent worker can override
//! the call site without conflicting on this file.

use crate::{PoolRuntimeError, TokenIntrospectionCachePolicy};
use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;
use std::time::{Duration, SystemTime};

/// Claim envelope returned by `sidecar/auth /auth/verify` (subset relevant to
/// the pool — the full envelope is owned by `sidecar/auth`).
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct VerifiedClaims {
    pub jti: String,
    pub tenant_id: String,
    pub subject: String,
    pub roles: Vec<String>,
    pub expires_at: SystemTime,
}

impl VerifiedClaims {
    pub fn validate(&self) -> Result<(), AuthCacheError> {
        if self.jti.trim().is_empty() {
            return Err(AuthCacheError::MissingField("jti"));
        }
        if self.tenant_id.trim().is_empty() {
            return Err(AuthCacheError::MissingField("tenant_id"));
        }
        if self.subject.trim().is_empty() {
            return Err(AuthCacheError::MissingField("subject"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct CacheEntry {
    claims: VerifiedClaims,
    cached_at: SystemTime,
    revoked: bool,
}

/// JTI-keyed verification cache with TTL + capacity bounds.
///
/// Eviction policy:
/// - `expires_at <= now` → expired (drop on lookup).
/// - `cached_at + ttl <= now` → stale (drop on lookup, re-verify).
/// - `len >= max_entries` → drop oldest (lowest `cached_at`) on insert.
/// - Sidecar revocation pushes call `revoke(jti)` to immediately mark the
///   entry as revoked; the next lookup returns
///   `Err(AuthCacheError::Revoked)`.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct AuthVerificationCache {
    ttl: Duration,
    max_entries: u32,
    entries: BTreeMap<String, CacheEntry>,
}

impl AuthVerificationCache {
    pub fn new(policy: &TokenIntrospectionCachePolicy) -> Result<Self, AuthCacheError> {
        policy.validate().map_err(AuthCacheError::Runtime)?;
        Ok(Self {
            ttl: Duration::from_secs(policy.ttl_seconds as u64),
            max_entries: policy.max_entries,
            entries: BTreeMap::new(),
        })
    }

    pub fn ttl(&self) -> Duration {
        self.ttl
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn insert(
        &mut self,
        claims: VerifiedClaims,
        now: SystemTime,
    ) -> Result<(), AuthCacheError> {
        claims.validate()?;
        if claims.expires_at <= now {
            return Err(AuthCacheError::ExpiredOnInsert);
        }
        if self.entries.len() as u32 >= self.max_entries {
            self.evict_oldest();
        }
        self.entries.insert(
            claims.jti.clone(),
            CacheEntry {
                claims,
                cached_at: now,
                revoked: false,
            },
        );
        Ok(())
    }

    fn evict_oldest(&mut self) {
        if let Some(oldest) = self
            .entries
            .iter()
            .min_by_key(|(_, entry)| entry.cached_at)
            .map(|(jti, _)| jti.clone())
        {
            self.entries.remove(&oldest);
        }
    }

    pub fn lookup(&mut self, jti: &str, now: SystemTime) -> Result<VerifiedClaims, AuthCacheError> {
        let entry = match self.entries.get(jti) {
            Some(entry) => entry.clone(),
            None => return Err(AuthCacheError::Miss),
        };
        if entry.revoked {
            self.entries.remove(jti);
            return Err(AuthCacheError::Revoked);
        }
        if entry.claims.expires_at <= now {
            self.entries.remove(jti);
            return Err(AuthCacheError::Expired);
        }
        if entry
            .cached_at
            .checked_add(self.ttl)
            .map(|cutoff| cutoff <= now)
            .unwrap_or(true)
        {
            self.entries.remove(jti);
            return Err(AuthCacheError::Stale);
        }
        Ok(entry.claims)
    }

    pub fn revoke(&mut self, jti: &str) -> bool {
        if let Some(entry) = self.entries.get_mut(jti) {
            entry.revoked = true;
            true
        } else {
            false
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum AuthCacheError {
    Expired,
    ExpiredOnInsert,
    Miss,
    MissingField(&'static str),
    Revoked,
    Runtime(PoolRuntimeError),
    Stale,
}

impl fmt::Display for AuthCacheError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Expired => write!(formatter, "verified-claim expired"),
            Self::ExpiredOnInsert => write!(formatter, "verified-claim already expired"),
            Self::Miss => write!(formatter, "verified-claim cache miss"),
            Self::MissingField(field) => write!(formatter, "{field} must not be empty"),
            Self::Revoked => write!(formatter, "verified-claim revoked"),
            Self::Runtime(error) => write!(formatter, "{error}"),
            Self::Stale => write!(formatter, "verified-claim stale"),
        }
    }
}

impl Error for AuthCacheError {}

impl From<PoolRuntimeError> for AuthCacheError {
    fn from(error: PoolRuntimeError) -> Self {
        Self::Runtime(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn policy(max: u32, ttl: u32) -> TokenIntrospectionCachePolicy {
        TokenIntrospectionCachePolicy {
            max_entries: max,
            ttl_seconds: ttl,
        }
    }

    fn claims(jti: &str, expires_in: Duration) -> VerifiedClaims {
        VerifiedClaims {
            jti: jti.to_string(),
            tenant_id: "tenant-a".to_string(),
            subject: "user-1".to_string(),
            roles: vec!["app_user".to_string()],
            expires_at: SystemTime::now() + expires_in,
        }
    }

    #[test]
    fn insert_then_lookup_returns_claims() {
        let mut cache = AuthVerificationCache::new(&policy(8, 60)).expect("cache");
        let now = SystemTime::now();
        cache
            .insert(claims("jti-1", Duration::from_secs(300)), now)
            .expect("insert");
        let resolved = cache.lookup("jti-1", now).expect("lookup");
        assert_eq!(resolved.tenant_id, "tenant-a");
    }

    #[test]
    fn miss_when_unknown_jti() {
        let mut cache = AuthVerificationCache::new(&policy(8, 60)).expect("cache");
        assert_eq!(
            cache.lookup("absent", SystemTime::now()),
            Err(AuthCacheError::Miss)
        );
    }

    #[test]
    fn expired_token_evicted_on_lookup() {
        let mut cache = AuthVerificationCache::new(&policy(8, 60)).expect("cache");
        let now = SystemTime::now();
        let claims = VerifiedClaims {
            jti: "jti-exp".to_string(),
            tenant_id: "tenant".to_string(),
            subject: "user".to_string(),
            roles: vec![],
            expires_at: now + Duration::from_secs(30),
        };
        cache.insert(claims, now).expect("insert");
        let later = now + Duration::from_secs(120);
        assert_eq!(cache.lookup("jti-exp", later), Err(AuthCacheError::Expired));
        assert!(cache.is_empty());
    }

    #[test]
    fn stale_after_ttl() {
        let mut cache = AuthVerificationCache::new(&policy(8, 5)).expect("cache");
        let now = SystemTime::now();
        cache
            .insert(claims("jti-stale", Duration::from_secs(300)), now)
            .expect("insert");
        let later = now + Duration::from_secs(60);
        assert_eq!(cache.lookup("jti-stale", later), Err(AuthCacheError::Stale));
    }

    #[test]
    fn revoke_invalidates_on_next_lookup() {
        let mut cache = AuthVerificationCache::new(&policy(8, 60)).expect("cache");
        let now = SystemTime::now();
        cache
            .insert(claims("jti-rev", Duration::from_secs(300)), now)
            .expect("insert");
        assert!(cache.revoke("jti-rev"));
        assert_eq!(cache.lookup("jti-rev", now), Err(AuthCacheError::Revoked));
    }

    #[test]
    fn capacity_evicts_oldest() {
        let mut cache = AuthVerificationCache::new(&policy(2, 60)).expect("cache");
        let now = SystemTime::now();
        cache
            .insert(claims("first", Duration::from_secs(300)), now)
            .expect("insert");
        cache
            .insert(
                claims("second", Duration::from_secs(300)),
                now + Duration::from_secs(1),
            )
            .expect("insert");
        cache
            .insert(
                claims("third", Duration::from_secs(300)),
                now + Duration::from_secs(2),
            )
            .expect("insert");
        assert_eq!(cache.len(), 2);
        assert_eq!(cache.lookup("first", now), Err(AuthCacheError::Miss));
    }

    #[test]
    fn insert_rejects_expired_claims() {
        let mut cache = AuthVerificationCache::new(&policy(2, 60)).expect("cache");
        let now = SystemTime::now();
        let expired = VerifiedClaims {
            jti: "expired".to_string(),
            tenant_id: "tenant".to_string(),
            subject: "user".to_string(),
            roles: vec![],
            expires_at: now - Duration::from_secs(1),
        };
        assert_eq!(
            cache.insert(expired, now),
            Err(AuthCacheError::ExpiredOnInsert)
        );
    }
}
