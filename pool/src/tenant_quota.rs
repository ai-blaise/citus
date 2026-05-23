// FEATURE: Sec12

//! Per-tenant token-bucket admission for pool traffic.
//!
//! The pool keeps a small in-memory quota bucket per configured tenant. Each
//! request consumes a caller-supplied cost, refill is monotonic in
//! milliseconds, and rejection is explicit instead of sleeping inside the hot
//! path. This keeps the proxy fail-fast under overload while preserving a
//! deterministic API for tests and canonical execution.

use crate::{PoolRuntimeError, TenantAdmissionPolicy};
use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum TenantAdmission {
    Admitted { remaining_tokens: u32 },
    Rejected { available_tokens: u32 },
}

impl TenantAdmission {
    pub fn admitted(&self) -> bool {
        matches!(self, Self::Admitted { .. })
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TenantQuotaState {
    pub tokens: u32,
    pub last_refill_ms: u64,
    pub admitted_total: u64,
    pub rejected_total: u64,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TenantQuotaTable {
    policy: TenantAdmissionPolicy,
    tenants: BTreeMap<String, TenantQuotaState>,
}

impl TenantQuotaTable {
    pub fn new(policy: TenantAdmissionPolicy) -> Result<Self, TenantQuotaError> {
        policy.validate().map_err(TenantQuotaError::Runtime)?;
        Ok(Self {
            policy,
            tenants: BTreeMap::new(),
        })
    }

    pub fn policy(&self) -> &TenantAdmissionPolicy {
        &self.policy
    }

    pub fn try_admit(
        &mut self,
        tenant_id: &str,
        now_ms: u64,
        cost: u32,
    ) -> Result<TenantAdmission, TenantQuotaError> {
        if tenant_id.trim().is_empty() {
            return Err(TenantQuotaError::MissingTenant);
        }
        if tenant_id != self.policy.tenant_id {
            return Err(TenantQuotaError::UnknownTenant(tenant_id.to_string()));
        }
        if cost == 0 {
            return Err(TenantQuotaError::InvalidCost);
        }
        if cost > self.policy.burst {
            return Ok(TenantAdmission::Rejected {
                available_tokens: self.state_for(now_ms).tokens,
            });
        }

        let refill_per_second = self.policy.refill_per_second;
        let burst = self.policy.burst;
        let state = self.state_for(now_ms);
        refill(state, now_ms, refill_per_second, burst);

        if state.tokens < cost {
            state.rejected_total += 1;
            return Ok(TenantAdmission::Rejected {
                available_tokens: state.tokens,
            });
        }

        state.tokens -= cost;
        state.admitted_total += 1;
        Ok(TenantAdmission::Admitted {
            remaining_tokens: state.tokens,
        })
    }

    fn state_for(&mut self, now_ms: u64) -> &mut TenantQuotaState {
        self.tenants
            .entry(self.policy.tenant_id.clone())
            .or_insert_with(|| TenantQuotaState {
                tokens: self.policy.burst,
                last_refill_ms: now_ms,
                admitted_total: 0,
                rejected_total: 0,
            })
    }

    pub fn state(&self, tenant_id: &str) -> Option<&TenantQuotaState> {
        self.tenants.get(tenant_id)
    }
}

fn refill(state: &mut TenantQuotaState, now_ms: u64, refill_per_second: u32, burst: u32) {
    if now_ms <= state.last_refill_ms {
        return;
    }
    let elapsed_ms = now_ms - state.last_refill_ms;
    let refill = elapsed_ms.saturating_mul(refill_per_second as u64) / 1_000;
    if refill > 0 {
        state.tokens = burst.min(state.tokens.saturating_add(refill as u32));
        state.last_refill_ms = now_ms;
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum TenantQuotaError {
    InvalidCost,
    MissingTenant,
    Runtime(PoolRuntimeError),
    UnknownTenant(String),
}

impl fmt::Display for TenantQuotaError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidCost => write!(formatter, "quota cost must be greater than zero"),
            Self::MissingTenant => write!(formatter, "tenant_id must not be empty"),
            Self::Runtime(error) => write!(formatter, "{error}"),
            Self::UnknownTenant(tenant_id) => write!(formatter, "unknown tenant {tenant_id}"),
        }
    }
}

impl Error for TenantQuotaError {}

impl From<PoolRuntimeError> for TenantQuotaError {
    fn from(error: PoolRuntimeError) -> Self {
        Self::Runtime(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn policy() -> TenantAdmissionPolicy {
        TenantAdmissionPolicy {
            tenant_id: "tenant-a".to_string(),
            burst: 10,
            refill_per_second: 5,
        }
    }

    #[test]
    fn admits_until_bucket_empty_then_rejects() {
        let mut table = TenantQuotaTable::new(policy()).expect("table");
        assert_eq!(
            table.try_admit("tenant-a", 0, 6),
            Ok(TenantAdmission::Admitted {
                remaining_tokens: 4
            })
        );
        assert_eq!(
            table.try_admit("tenant-a", 0, 5),
            Ok(TenantAdmission::Rejected {
                available_tokens: 4
            })
        );
        assert_eq!(table.state("tenant-a").unwrap().rejected_total, 1);
    }

    #[test]
    fn refills_by_elapsed_milliseconds() {
        let mut table = TenantQuotaTable::new(policy()).expect("table");
        table.try_admit("tenant-a", 0, 10).expect("admit");
        assert_eq!(
            table.try_admit("tenant-a", 1_000, 5),
            Ok(TenantAdmission::Admitted {
                remaining_tokens: 0
            })
        );
    }

    #[test]
    fn rejects_unknown_tenant() {
        let mut table = TenantQuotaTable::new(policy()).expect("table");
        assert_eq!(
            table.try_admit("tenant-b", 0, 1),
            Err(TenantQuotaError::UnknownTenant("tenant-b".to_string()))
        );
    }

    #[test]
    fn rejects_zero_cost() {
        let mut table = TenantQuotaTable::new(policy()).expect("table");
        assert_eq!(
            table.try_admit("tenant-a", 0, 0),
            Err(TenantQuotaError::InvalidCost)
        );
    }
}
