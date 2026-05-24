// FEATURE: R2
// FEATURE: C6
// FEATURE: C7
// FEATURE: C8

use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct BranchSpec {
    pub source_cluster: String,
    pub target_cluster: String,
    pub branch_type: BranchType,
    pub storage: BranchStorageSpec,
    pub suspend: bool,
    pub retention_days: Option<u32>,
}

impl BranchSpec {
    pub fn validate(&self) -> Result<(), BranchSpecError> {
        validate_required("source_cluster", &self.source_cluster)?;
        validate_required("target_cluster", &self.target_cluster)?;
        validate_dns_label("source_cluster", &self.source_cluster)?;
        validate_dns_label("target_cluster", &self.target_cluster)?;
        if self.source_cluster == self.target_cluster {
            return Err(BranchSpecError::SourceTargetConflict);
        }
        self.storage.validate()?;

        if matches!(self.retention_days, Some(0)) {
            return Err(BranchSpecError::InvalidRetention);
        }
        if matches!(self.retention_days, Some(days) if days > 365) {
            return Err(BranchSpecError::RetentionTooLong);
        }
        match self.branch_type {
            BranchType::Snapshot if self.storage.snapshot_class.is_none() => {
                return Err(BranchSpecError::MissingRequiredField(
                    "storage.snapshot_class",
                ));
            }
            BranchType::CopyOnWrite if self.storage.snapshot_class.is_some() => {
                return Err(BranchSpecError::SnapshotClassNotAllowed);
            }
            _ => {}
        }

        Ok(())
    }

    pub fn is_scale_to_zero_enabled(&self) -> bool {
        self.suspend
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum BranchType {
    CopyOnWrite,
    Snapshot,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct BranchStorageSpec {
    pub size: String,
    pub storage_class: Option<String>,
    pub snapshot_class: Option<String>,
}

impl BranchStorageSpec {
    fn validate(&self) -> Result<(), BranchSpecError> {
        validate_required("storage.size", &self.size)?;
        validate_storage_quantity("storage.size", &self.size)?;
        validate_optional("storage.storage_class", &self.storage_class)?;
        validate_optional("storage.snapshot_class", &self.snapshot_class)?;
        if let Some(storage_class) = &self.storage_class {
            validate_dns_label("storage.storage_class", storage_class)?;
        }
        if let Some(snapshot_class) = &self.snapshot_class {
            validate_dns_label("storage.snapshot_class", snapshot_class)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum BranchSpecError {
    InvalidIdentifier(&'static str),
    InvalidRetention,
    InvalidStorageQuantity(&'static str),
    MissingRequiredField(&'static str),
    RetentionTooLong,
    SnapshotClassNotAllowed,
    SourceTargetConflict,
}

impl fmt::Display for BranchSpecError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidIdentifier(field) => {
                write!(
                    formatter,
                    "{field} must be a lowercase Kubernetes DNS label"
                )
            }
            Self::InvalidRetention => {
                write!(formatter, "retention_days must be greater than zero")
            }
            Self::InvalidStorageQuantity(field) => {
                write!(
                    formatter,
                    "{field} must be a positive Kubernetes storage quantity with Ki, Mi, Gi, or Ti suffix"
                )
            }
            Self::MissingRequiredField(field) => {
                write!(formatter, "{field} must not be empty")
            }
            Self::RetentionTooLong => {
                write!(formatter, "retention_days must not exceed 365")
            }
            Self::SnapshotClassNotAllowed => {
                write!(
                    formatter,
                    "storage.snapshot_class is only valid for snapshot branches"
                )
            }
            Self::SourceTargetConflict => {
                write!(formatter, "source_cluster and target_cluster must differ")
            }
        }
    }
}

impl Error for BranchSpecError {}

fn validate_required(field: &'static str, value: &str) -> Result<(), BranchSpecError> {
    if value.trim().is_empty() {
        return Err(BranchSpecError::MissingRequiredField(field));
    }
    Ok(())
}

fn validate_optional(field: &'static str, value: &Option<String>) -> Result<(), BranchSpecError> {
    if matches!(value, Some(value) if value.trim().is_empty()) {
        return Err(BranchSpecError::MissingRequiredField(field));
    }
    Ok(())
}

fn validate_dns_label(field: &'static str, value: &str) -> Result<(), BranchSpecError> {
    let bytes = value.as_bytes();
    if bytes.is_empty()
        || bytes.len() > 63
        || !bytes[0].is_ascii_lowercase()
        || !(bytes[bytes.len() - 1].is_ascii_lowercase() || bytes[bytes.len() - 1].is_ascii_digit())
    {
        return Err(BranchSpecError::InvalidIdentifier(field));
    }
    if !bytes
        .iter()
        .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || *byte == b'-')
    {
        return Err(BranchSpecError::InvalidIdentifier(field));
    }
    Ok(())
}

fn validate_storage_quantity(field: &'static str, value: &str) -> Result<(), BranchSpecError> {
    let Some(unit_start) = value.find(|ch: char| !ch.is_ascii_digit()) else {
        return Err(BranchSpecError::InvalidStorageQuantity(field));
    };
    let (digits, suffix) = value.split_at(unit_start);
    if digits.is_empty() || digits.starts_with('0') || digits.parse::<u64>().unwrap_or(0) == 0 {
        return Err(BranchSpecError::InvalidStorageQuantity(field));
    }
    match suffix {
        "Ki" | "Mi" | "Gi" | "Ti" => Ok(()),
        _ => Err(BranchSpecError::InvalidStorageQuantity(field)),
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum BranchLifecycleAction {
    Apply,
    Suspend,
    Promote,
}

impl BranchLifecycleAction {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Apply => "apply",
            Self::Suspend => "suspend",
            Self::Promote => "promote",
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum BranchLifecyclePhase {
    Pending,
    Ready,
    Suspended,
    Promoted,
    Failed,
}

impl BranchLifecyclePhase {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Ready => "ready",
            Self::Suspended => "suspended",
            Self::Promoted => "promoted",
            Self::Failed => "failed",
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct BranchLifecycleStatus {
    pub phase: BranchLifecyclePhase,
    pub observed_generation: u64,
    pub source_cluster_ready: bool,
    pub snapshot_ready: bool,
    pub target_cluster_ready: bool,
    pub writes_quiesced: bool,
    pub replication_caught_up: bool,
    pub active_sessions: u32,
    pub pending_migrations: u32,
}

impl BranchLifecycleStatus {
    pub fn pending(observed_generation: u64) -> Self {
        Self {
            phase: BranchLifecyclePhase::Pending,
            observed_generation,
            source_cluster_ready: false,
            snapshot_ready: false,
            target_cluster_ready: false,
            writes_quiesced: false,
            replication_caught_up: false,
            active_sessions: 0,
            pending_migrations: 0,
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct BranchLifecyclePlan {
    pub action: BranchLifecycleAction,
    pub from_phase: BranchLifecyclePhase,
    pub to_phase: BranchLifecyclePhase,
    pub steps: Vec<BranchLifecycleStep>,
}

impl BranchLifecyclePlan {
    pub fn step_count(&self) -> usize {
        self.steps.len()
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum BranchLifecycleStep {
    ValidateSpec,
    VerifySourceClusterReady,
    RequestCsiSnapshot,
    WaitForSnapshotReady,
    MaterializeTargetCluster,
    VerifyTargetClusterReady,
    MarkReady,
    RejectNewSessions,
    ScaleTargetComputeToZero,
    MarkSuspended,
    VerifyNoActiveSessions,
    VerifyNoPendingMigrations,
    VerifyWritesQuiesced,
    VerifyReplicationCaughtUp,
    FenceSourceWrites,
    PromoteTargetCluster,
    MarkPromoted,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum BranchLifecycleError {
    Spec(BranchSpecError),
    SourceClusterNotReady,
    SnapshotNotReady,
    TargetClusterNotReady,
    InvalidPhase {
        action: BranchLifecycleAction,
        phase: BranchLifecyclePhase,
    },
    SuspendedPromotionBlocked,
    ActiveSessions(u32),
    PendingMigrations(u32),
    WritesNotQuiesced,
    ReplicationLagging,
}

impl fmt::Display for BranchLifecycleError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Spec(error) => write!(formatter, "{error}"),
            Self::SourceClusterNotReady => write!(formatter, "source cluster is not ready"),
            Self::SnapshotNotReady => write!(formatter, "snapshot is not ready"),
            Self::TargetClusterNotReady => write!(formatter, "target cluster is not ready"),
            Self::InvalidPhase { action, phase } => {
                write!(
                    formatter,
                    "cannot {} branch from {} phase",
                    action.as_str(),
                    phase.as_str()
                )
            }
            Self::SuspendedPromotionBlocked => {
                write!(formatter, "cannot promote while suspend intent is true")
            }
            Self::ActiveSessions(count) => {
                write!(
                    formatter,
                    "cannot continue branch lifecycle with {count} active sessions"
                )
            }
            Self::PendingMigrations(count) => {
                write!(
                    formatter,
                    "cannot continue branch lifecycle with {count} pending migrations"
                )
            }
            Self::WritesNotQuiesced => write!(formatter, "source writes are not quiesced"),
            Self::ReplicationLagging => write!(formatter, "target replication has not caught up"),
        }
    }
}

impl Error for BranchLifecycleError {}

impl From<BranchSpecError> for BranchLifecycleError {
    fn from(error: BranchSpecError) -> Self {
        Self::Spec(error)
    }
}

pub fn plan_branch_lifecycle(
    spec: &BranchSpec,
    status: &BranchLifecycleStatus,
    action: BranchLifecycleAction,
) -> Result<BranchLifecyclePlan, BranchLifecycleError> {
    spec.validate()?;
    match action {
        BranchLifecycleAction::Apply => plan_apply(spec, status),
        BranchLifecycleAction::Suspend => plan_suspend(status),
        BranchLifecycleAction::Promote => plan_promote(spec, status),
    }
}

fn plan_apply(
    spec: &BranchSpec,
    status: &BranchLifecycleStatus,
) -> Result<BranchLifecyclePlan, BranchLifecycleError> {
    if status.phase != BranchLifecyclePhase::Pending {
        return Err(BranchLifecycleError::InvalidPhase {
            action: BranchLifecycleAction::Apply,
            phase: status.phase,
        });
    }
    if !status.source_cluster_ready {
        return Err(BranchLifecycleError::SourceClusterNotReady);
    }

    let mut steps = vec![
        BranchLifecycleStep::ValidateSpec,
        BranchLifecycleStep::VerifySourceClusterReady,
    ];
    if spec.branch_type == BranchType::Snapshot {
        steps.push(BranchLifecycleStep::RequestCsiSnapshot);
        steps.push(BranchLifecycleStep::WaitForSnapshotReady);
    }
    steps.push(BranchLifecycleStep::MaterializeTargetCluster);
    steps.push(BranchLifecycleStep::VerifyTargetClusterReady);
    steps.push(BranchLifecycleStep::MarkReady);

    Ok(BranchLifecyclePlan {
        action: BranchLifecycleAction::Apply,
        from_phase: status.phase,
        to_phase: BranchLifecyclePhase::Ready,
        steps,
    })
}

fn plan_suspend(
    status: &BranchLifecycleStatus,
) -> Result<BranchLifecyclePlan, BranchLifecycleError> {
    if status.phase != BranchLifecyclePhase::Ready {
        return Err(BranchLifecycleError::InvalidPhase {
            action: BranchLifecycleAction::Suspend,
            phase: status.phase,
        });
    }
    ensure_no_active_sessions(status)?;
    ensure_no_pending_migrations(status)?;

    Ok(BranchLifecyclePlan {
        action: BranchLifecycleAction::Suspend,
        from_phase: status.phase,
        to_phase: BranchLifecyclePhase::Suspended,
        steps: vec![
            BranchLifecycleStep::ValidateSpec,
            BranchLifecycleStep::RejectNewSessions,
            BranchLifecycleStep::VerifyNoActiveSessions,
            BranchLifecycleStep::VerifyNoPendingMigrations,
            BranchLifecycleStep::ScaleTargetComputeToZero,
            BranchLifecycleStep::MarkSuspended,
        ],
    })
}

fn plan_promote(
    spec: &BranchSpec,
    status: &BranchLifecycleStatus,
) -> Result<BranchLifecyclePlan, BranchLifecycleError> {
    if spec.suspend {
        return Err(BranchLifecycleError::SuspendedPromotionBlocked);
    }
    if status.phase != BranchLifecyclePhase::Ready {
        return Err(BranchLifecycleError::InvalidPhase {
            action: BranchLifecycleAction::Promote,
            phase: status.phase,
        });
    }
    if spec.branch_type == BranchType::Snapshot && !status.snapshot_ready {
        return Err(BranchLifecycleError::SnapshotNotReady);
    }
    if !status.target_cluster_ready {
        return Err(BranchLifecycleError::TargetClusterNotReady);
    }
    ensure_no_active_sessions(status)?;
    ensure_no_pending_migrations(status)?;
    if !status.writes_quiesced {
        return Err(BranchLifecycleError::WritesNotQuiesced);
    }
    if !status.replication_caught_up {
        return Err(BranchLifecycleError::ReplicationLagging);
    }

    Ok(BranchLifecyclePlan {
        action: BranchLifecycleAction::Promote,
        from_phase: status.phase,
        to_phase: BranchLifecyclePhase::Promoted,
        steps: vec![
            BranchLifecycleStep::ValidateSpec,
            BranchLifecycleStep::VerifyTargetClusterReady,
            BranchLifecycleStep::VerifyNoActiveSessions,
            BranchLifecycleStep::VerifyNoPendingMigrations,
            BranchLifecycleStep::VerifyWritesQuiesced,
            BranchLifecycleStep::VerifyReplicationCaughtUp,
            BranchLifecycleStep::FenceSourceWrites,
            BranchLifecycleStep::PromoteTargetCluster,
            BranchLifecycleStep::MarkPromoted,
        ],
    })
}

fn ensure_no_active_sessions(status: &BranchLifecycleStatus) -> Result<(), BranchLifecycleError> {
    if status.active_sessions > 0 {
        return Err(BranchLifecycleError::ActiveSessions(status.active_sessions));
    }
    Ok(())
}

fn ensure_no_pending_migrations(
    status: &BranchLifecycleStatus,
) -> Result<(), BranchLifecycleError> {
    if status.pending_migrations > 0 {
        return Err(BranchLifecycleError::PendingMigrations(
            status.pending_migrations,
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_copy_on_write_branch_passes() {
        let spec = BranchSpec {
            source_cluster: "prod-us-east".to_string(),
            target_cluster: "branch-review".to_string(),
            branch_type: BranchType::CopyOnWrite,
            storage: BranchStorageSpec {
                size: "256Gi".to_string(),
                storage_class: Some("fast-ssd".to_string()),
                snapshot_class: None,
            },
            suspend: true,
            retention_days: Some(7),
        };

        assert_eq!(spec.validate(), Ok(()));
        assert!(spec.is_scale_to_zero_enabled());
    }

    #[test]
    fn branch_rejects_missing_source_cluster() {
        let mut spec = minimal_spec();
        spec.source_cluster = " ".to_string();

        assert_eq!(
            spec.validate(),
            Err(BranchSpecError::MissingRequiredField("source_cluster"))
        );
    }

    #[test]
    fn branch_rejects_zero_retention() {
        let mut spec = minimal_spec();
        spec.retention_days = Some(0);

        assert_eq!(spec.validate(), Err(BranchSpecError::InvalidRetention));
    }

    #[test]
    fn snapshot_branch_requires_snapshot_class() {
        let mut spec = minimal_spec();
        spec.storage.snapshot_class = None;

        assert_eq!(
            spec.validate(),
            Err(BranchSpecError::MissingRequiredField(
                "storage.snapshot_class"
            ))
        );
    }

    #[test]
    fn branch_rejects_ambiguous_source_target_and_identifiers() {
        let mut spec = minimal_spec();
        spec.target_cluster = spec.source_cluster.clone();
        assert_eq!(spec.validate(), Err(BranchSpecError::SourceTargetConflict));

        spec.target_cluster = "Branch_Review".to_string();
        assert_eq!(
            spec.validate(),
            Err(BranchSpecError::InvalidIdentifier("target_cluster"))
        );
    }

    #[test]
    fn branch_accepts_source_and_target_labels_ending_in_digits() {
        let mut spec = minimal_spec();
        spec.source_cluster = "cluster1".to_string();
        spec.target_cluster = "branch1".to_string();

        assert_eq!(spec.validate(), Ok(()));
    }

    #[test]
    fn branch_rejects_invalid_storage_quantity() {
        let mut spec = minimal_spec();
        spec.storage.size = "0Gi".to_string();

        assert_eq!(
            spec.validate(),
            Err(BranchSpecError::InvalidStorageQuantity("storage.size"))
        );
    }

    #[test]
    fn apply_snapshot_branch_requires_ready_source() {
        let spec = minimal_spec();
        let status = BranchLifecycleStatus::pending(1);

        assert_eq!(
            plan_branch_lifecycle(&spec, &status, BranchLifecycleAction::Apply),
            Err(BranchLifecycleError::SourceClusterNotReady)
        );
    }

    #[test]
    fn apply_snapshot_branch_materializes_deterministic_steps() {
        let spec = minimal_spec();
        let mut status = BranchLifecycleStatus::pending(1);
        status.source_cluster_ready = true;

        let plan = plan_branch_lifecycle(&spec, &status, BranchLifecycleAction::Apply).unwrap();

        assert_eq!(plan.from_phase, BranchLifecyclePhase::Pending);
        assert_eq!(plan.to_phase, BranchLifecyclePhase::Ready);
        assert_eq!(
            plan.steps,
            vec![
                BranchLifecycleStep::ValidateSpec,
                BranchLifecycleStep::VerifySourceClusterReady,
                BranchLifecycleStep::RequestCsiSnapshot,
                BranchLifecycleStep::WaitForSnapshotReady,
                BranchLifecycleStep::MaterializeTargetCluster,
                BranchLifecycleStep::VerifyTargetClusterReady,
                BranchLifecycleStep::MarkReady,
            ]
        );
    }

    #[test]
    fn suspend_fails_closed_with_active_sessions_or_migrations() {
        let spec = minimal_spec();
        let mut status = ready_status();
        status.active_sessions = 1;

        assert_eq!(
            plan_branch_lifecycle(&spec, &status, BranchLifecycleAction::Suspend),
            Err(BranchLifecycleError::ActiveSessions(1))
        );

        status.active_sessions = 0;
        status.pending_migrations = 1;
        assert_eq!(
            plan_branch_lifecycle(&spec, &status, BranchLifecycleAction::Suspend),
            Err(BranchLifecycleError::PendingMigrations(1))
        );
    }

    #[test]
    fn promote_requires_readiness_quiescence_and_catchup() {
        let mut spec = minimal_spec();
        spec.suspend = false;
        let mut status = ready_status();
        status.snapshot_ready = false;

        assert_eq!(
            plan_branch_lifecycle(&spec, &status, BranchLifecycleAction::Promote),
            Err(BranchLifecycleError::SnapshotNotReady)
        );

        status.snapshot_ready = true;
        status.writes_quiesced = false;
        assert_eq!(
            plan_branch_lifecycle(&spec, &status, BranchLifecycleAction::Promote),
            Err(BranchLifecycleError::WritesNotQuiesced)
        );

        status.writes_quiesced = true;
        status.replication_caught_up = false;
        assert_eq!(
            plan_branch_lifecycle(&spec, &status, BranchLifecycleAction::Promote),
            Err(BranchLifecycleError::ReplicationLagging)
        );
    }

    #[test]
    fn promote_plan_is_deterministic_when_ready() {
        let mut spec = minimal_spec();
        spec.suspend = false;
        let status = ready_status();

        let plan = plan_branch_lifecycle(&spec, &status, BranchLifecycleAction::Promote).unwrap();

        assert_eq!(plan.from_phase, BranchLifecyclePhase::Ready);
        assert_eq!(plan.to_phase, BranchLifecyclePhase::Promoted);
        assert_eq!(plan.step_count(), 9);
    }

    fn minimal_spec() -> BranchSpec {
        BranchSpec {
            source_cluster: "prod-us-east".to_string(),
            target_cluster: "branch-review".to_string(),
            branch_type: BranchType::Snapshot,
            storage: BranchStorageSpec {
                size: "128Gi".to_string(),
                storage_class: None,
                snapshot_class: Some("csi-snapshot".to_string()),
            },
            suspend: false,
            retention_days: None,
        }
    }

    fn ready_status() -> BranchLifecycleStatus {
        BranchLifecycleStatus {
            phase: BranchLifecyclePhase::Ready,
            observed_generation: 1,
            source_cluster_ready: true,
            snapshot_ready: true,
            target_cluster_ready: true,
            writes_quiesced: true,
            replication_caught_up: true,
            active_sessions: 0,
            pending_migrations: 0,
        }
    }
}
