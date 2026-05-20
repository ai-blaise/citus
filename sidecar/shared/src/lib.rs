//! Shared sidecar primitives.

// FEATURE: O4

pub mod contracts;
pub mod runtime;

pub use contracts::{
    AnalyticalMirrorContract, AuthIssuerContract, BackupRestoreContract, CdcSink,
    CdcStreamContract, DeliveryRetryPolicy, RealtimeContract, RepackContract,
    RepackExecutionStrategy, SidecarContractError, SidecarRuntimeContracts, StorageContract,
};
pub use runtime::{
    HttpMethod, HttpProbeRequest, HttpProbeResponse, SidecarRuntime, SidecarRuntimeError,
};

use std::time::{Duration, SystemTime};

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct HealthReport {
    pub component: String,
    pub state: ComponentState,
    pub started_at: SystemTime,
    pub checked_at: SystemTime,
    pub detail: Option<String>,
}

impl HealthReport {
    pub fn ready(component: impl Into<String>, started_at: SystemTime) -> Self {
        let checked_at = SystemTime::now();
        Self {
            component: component.into(),
            state: ComponentState::Ready,
            started_at,
            checked_at,
            detail: None,
        }
    }

    pub fn not_ready(
        component: impl Into<String>,
        started_at: SystemTime,
        detail: impl Into<String>,
    ) -> Self {
        let checked_at = SystemTime::now();
        Self {
            component: component.into(),
            state: ComponentState::NotReady,
            started_at,
            checked_at,
            detail: Some(detail.into()),
        }
    }

    pub fn draining(
        component: impl Into<String>,
        started_at: SystemTime,
        detail: impl Into<String>,
    ) -> Self {
        let checked_at = SystemTime::now();
        Self {
            component: component.into(),
            state: ComponentState::Draining,
            started_at,
            checked_at,
            detail: Some(detail.into()),
        }
    }

    pub fn uptime(&self) -> Option<Duration> {
        self.checked_at.duration_since(self.started_at).ok()
    }

    pub fn is_ready(&self) -> bool {
        self.state == ComponentState::Ready
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum ComponentState {
    Ready,
    NotReady,
    Draining,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct DrainState {
    pub accepting_new_work: bool,
    pub in_flight_work: u64,
}

impl DrainState {
    pub fn active(in_flight_work: u64) -> Self {
        Self {
            accepting_new_work: true,
            in_flight_work,
        }
    }

    pub fn draining(in_flight_work: u64) -> Self {
        Self {
            accepting_new_work: false,
            in_flight_work,
        }
    }

    pub fn is_drained(&self) -> bool {
        !self.accepting_new_work && self.in_flight_work == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ready_report_is_ready() {
        let started_at = SystemTime::now() - Duration::from_secs(5);
        let report = HealthReport::ready("vectorizer", started_at);

        assert!(report.is_ready());
        assert!(report.uptime().expect("uptime") >= Duration::from_secs(5));
    }

    #[test]
    fn not_ready_report_carries_detail() {
        let report = HealthReport::not_ready(
            "cdc",
            SystemTime::now(),
            "logical replication slot unavailable",
        );

        assert!(!report.is_ready());
        assert_eq!(report.state, ComponentState::NotReady);
        assert_eq!(
            report.detail.as_deref(),
            Some("logical replication slot unavailable")
        );
    }

    #[test]
    fn drain_state_requires_no_new_work_and_no_in_flight_work() {
        assert!(!DrainState::active(0).is_drained());
        assert!(!DrainState::draining(2).is_drained());
        assert!(DrainState::draining(0).is_drained());
    }
}
