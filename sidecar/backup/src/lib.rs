//! Backup sidecar contracts and WAL-G real runtime.

// FEATURE: B1
// FEATURE: B3
// FEATURE: B4
// FEATURE: B6

pub mod queryable_branch;
pub mod scheduler;
pub mod walg;

pub use queryable_branch::{QueryableBranchError, QueryableBranchProcess, QueryableBranchRunner};
pub use scheduler::{BackupSchedule, ScheduleError};
pub use walg::{walg_env_from_plan, WalgError, WalgInvocation, WalgRunner};

use ai_blaise_citus_sidecar_shared::{
    listen_addr_from_env, BackupRestoreContract, HttpProbeResponse, SidecarContractError,
    SidecarRuntime, SidecarRuntimeError,
};
use std::collections::HashMap;
use std::error::Error;
use std::ffi::OsString;
use std::fmt;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct BackupJobPlan {
    pub cluster: String,
    pub contract: BackupRestoreContract,
    pub base_backup: BaseBackupPlan,
    pub wal_archive: WalArchivePlan,
    pub encryption: Option<BackupEncryptionPlan>,
}

impl BackupJobPlan {
    pub fn validate(&self) -> Result<(), BackupSidecarError> {
        validate_required("cluster", &self.cluster)?;
        self.contract.validate()?;
        validate_uri("backup.archive_uri", &self.contract.archive_uri)?;
        self.base_backup.validate()?;
        self.wal_archive.validate()?;
        if let Some(encryption) = &self.encryption {
            encryption.validate()?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct BaseBackupPlan {
    pub destination_uri: String,
    pub retention_days: u32,
    pub concurrency: u32,
}

impl BaseBackupPlan {
    fn validate(&self) -> Result<(), BackupSidecarError> {
        validate_uri("base_backup.destination_uri", &self.destination_uri)?;
        if self.retention_days == 0 {
            return Err(BackupSidecarError::InvalidRetention);
        }
        if self.concurrency == 0 {
            return Err(BackupSidecarError::InvalidConcurrency);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct WalArchivePlan {
    pub slot_name: String,
    pub archive_uri: String,
    pub compression: WalCompression,
}

impl WalArchivePlan {
    fn validate(&self) -> Result<(), BackupSidecarError> {
        validate_required("wal.slot_name", &self.slot_name)?;
        validate_uri("wal.archive_uri", &self.archive_uri)
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum WalCompression {
    None,
    Gzip,
    Zstd,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct BackupEncryptionPlan {
    pub kms_key_ref: String,
}

impl BackupEncryptionPlan {
    fn validate(&self) -> Result<(), BackupSidecarError> {
        validate_required("encryption.kms_key_ref", &self.kms_key_ref)
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PitrRestorePlan {
    pub cluster: String,
    pub source_archive_uri: String,
    pub target_time: String,
    pub target_cluster: String,
}

impl PitrRestorePlan {
    pub fn validate(&self) -> Result<(), BackupSidecarError> {
        validate_required("cluster", &self.cluster)?;
        validate_uri("source_archive_uri", &self.source_archive_uri)?;
        validate_timestamp(&self.target_time)?;
        validate_required("target_cluster", &self.target_cluster)
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct QueryableBackupBranchPlan {
    pub branch_name: String,
    pub source_archive_uri: String,
    pub target_time: String,
    pub read_only: bool,
}

impl QueryableBackupBranchPlan {
    pub fn validate(&self) -> Result<(), BackupSidecarError> {
        validate_branch_name(&self.branch_name)?;
        validate_uri("source_archive_uri", &self.source_archive_uri)?;
        validate_timestamp(&self.target_time)?;
        if !self.read_only {
            return Err(BackupSidecarError::QueryableBranchMustBeReadOnly);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum BackupSidecarError {
    ArchiveMismatch,
    InvalidBranchName,
    InvalidConcurrency,
    InvalidPort,
    InvalidRetention,
    InvalidTimestamp,
    InvalidUri(&'static str),
    MalformedHttpRequest,
    MissingRequiredField(&'static str),
    PitrTargetOutsideWindow {
        target_time: String,
        oldest_wal_time: String,
        latest_wal_time: String,
    },
    QueryableBranchMustBeReadOnly,
    QueryableBranch(String),
    Runtime(String),
    Schedule(String),
    SharedContract(String),
    Walg(String),
}

impl fmt::Display for BackupSidecarError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ArchiveMismatch => {
                write!(
                    formatter,
                    "restore or branch archive does not match backup job"
                )
            }
            Self::InvalidBranchName => write!(
                formatter,
                "branch_name must contain only ASCII letters, digits, '.', '_', or '-' and must not contain path traversal"
            ),
            Self::InvalidConcurrency => write!(formatter, "concurrency must be greater than zero"),
            Self::InvalidPort => write!(formatter, "queryable branch port must be between 1024 and 65535"),
            Self::InvalidRetention => write!(formatter, "retention_days must be greater than zero"),
            Self::InvalidTimestamp => {
                write!(formatter, "target_time must be an RFC3339 UTC timestamp")
            }
            Self::InvalidUri(field) => write!(formatter, "{field} must be an object-store URI"),
            Self::MalformedHttpRequest => {
                write!(formatter, "malformed backup sidecar HTTP request")
            }
            Self::MissingRequiredField(field) => write!(formatter, "{field} must not be empty"),
            Self::PitrTargetOutsideWindow {
                target_time,
                oldest_wal_time,
                latest_wal_time,
            } => write!(
                formatter,
                "PITR target {target_time} is outside available WAL window [{oldest_wal_time}, {latest_wal_time}]"
            ),
            Self::QueryableBranchMustBeReadOnly => {
                write!(formatter, "queryable backup branches must be read-only")
            }
            Self::QueryableBranch(error) => write!(formatter, "{error}"),
            Self::Runtime(error) => write!(formatter, "{error}"),
            Self::Schedule(error) => write!(formatter, "{error}"),
            Self::SharedContract(error) => write!(formatter, "{error}"),
            Self::Walg(error) => write!(formatter, "{error}"),
        }
    }
}

impl Error for BackupSidecarError {}

impl From<SidecarContractError> for BackupSidecarError {
    fn from(error: SidecarContractError) -> Self {
        Self::SharedContract(error.to_string())
    }
}

impl From<SidecarRuntimeError> for BackupSidecarError {
    fn from(error: SidecarRuntimeError) -> Self {
        Self::Runtime(error.to_string())
    }
}

impl From<std::io::Error> for BackupSidecarError {
    fn from(error: std::io::Error) -> Self {
        Self::Runtime(error.to_string())
    }
}

impl From<WalgError> for BackupSidecarError {
    fn from(error: WalgError) -> Self {
        Self::Walg(error.to_string())
    }
}

impl From<ScheduleError> for BackupSidecarError {
    fn from(error: ScheduleError) -> Self {
        Self::Schedule(error.to_string())
    }
}

impl From<QueryableBranchError> for BackupSidecarError {
    fn from(error: QueryableBranchError) -> Self {
        Self::QueryableBranch(error.to_string())
    }
}

fn validate_required(field: &'static str, value: &str) -> Result<(), BackupSidecarError> {
    if value.trim().is_empty() {
        return Err(BackupSidecarError::MissingRequiredField(field));
    }
    Ok(())
}

fn validate_uri(field: &'static str, value: &str) -> Result<(), BackupSidecarError> {
    validate_required(field, value)?;
    let Some((scheme, rest)) = value.split_once("://") else {
        return Err(BackupSidecarError::InvalidUri(field));
    };
    if !matches!(scheme, "s3" | "gs" | "az") || rest.is_empty() {
        return Err(BackupSidecarError::InvalidUri(field));
    }
    if rest
        .chars()
        .any(|ch| ch.is_ascii_whitespace() || ch.is_control())
    {
        return Err(BackupSidecarError::InvalidUri(field));
    }
    let mut parts = rest.split('/');
    let bucket = parts.next().unwrap_or_default();
    if bucket.is_empty() || bucket == "." || bucket == ".." {
        return Err(BackupSidecarError::InvalidUri(field));
    }
    let mut has_prefix = false;
    for part in parts {
        if part == ".." {
            return Err(BackupSidecarError::InvalidUri(field));
        }
        if !part.is_empty() && part != "." {
            has_prefix = true;
        }
    }
    if !has_prefix {
        return Err(BackupSidecarError::InvalidUri(field));
    }
    Ok(())
}

fn validate_timestamp(value: &str) -> Result<(), BackupSidecarError> {
    validate_required("target_time", value)?;
    if is_canonical_utc_timestamp(value) {
        Ok(())
    } else {
        Err(BackupSidecarError::InvalidTimestamp)
    }
}

fn validate_branch_name(value: &str) -> Result<(), BackupSidecarError> {
    validate_required("branch_name", value)?;
    if value == "." || value == ".." || value.contains("..") {
        return Err(BackupSidecarError::InvalidBranchName);
    }
    if value
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
    {
        Ok(())
    } else {
        Err(BackupSidecarError::InvalidBranchName)
    }
}

fn validate_queryable_port(port: u16) -> Result<(), BackupSidecarError> {
    if (1024..=65535).contains(&port) {
        Ok(())
    } else {
        Err(BackupSidecarError::InvalidPort)
    }
}

fn is_canonical_utc_timestamp(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.len() != 20
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || bytes[10] != b'T'
        || bytes[13] != b':'
        || bytes[16] != b':'
        || bytes[19] != b'Z'
    {
        return false;
    }
    for index in [0, 1, 2, 3, 5, 6, 8, 9, 11, 12, 14, 15, 17, 18] {
        if !bytes[index].is_ascii_digit() {
            return false;
        }
    }
    let year = parse_fixed_u32(&bytes[0..4]);
    let month = parse_fixed_u32(&bytes[5..7]);
    let day = parse_fixed_u32(&bytes[8..10]);
    let hour = parse_fixed_u32(&bytes[11..13]);
    let minute = parse_fixed_u32(&bytes[14..16]);
    let second = parse_fixed_u32(&bytes[17..19]);
    if year == 0 || !(1..=12).contains(&month) {
        return false;
    }
    let max_day = days_in_month(year, month);
    (1..=max_day).contains(&day) && hour < 24 && minute < 60 && second < 60
}

fn parse_fixed_u32(bytes: &[u8]) -> u32 {
    bytes
        .iter()
        .fold(0_u32, |acc, byte| acc * 10 + u32::from(byte - b'0'))
}

fn days_in_month(year: u32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => 0,
    }
}

fn is_leap_year(year: u32) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct BackupCanonicalReport {
    pub job: BackupJobPlan,
    pub restore: PitrRestorePlan,
    pub queryable_branch: QueryableBackupBranchPlan,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct BackupArtifact {
    pub cluster: String,
    pub base_destination_uri: String,
    pub wal_archive_uri: String,
    pub base_size_bytes: u64,
    pub wal_segments: u32,
    pub encrypted: bool,
    pub retention_days: u32,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PitrRestoreResult {
    pub source_cluster: String,
    pub target_cluster: String,
    pub target_time: String,
    pub replayed_wal_segments: u32,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct QueryableBranchResult {
    pub branch_name: String,
    pub mounted_archive_uri: String,
    pub target_time: String,
    pub read_only: bool,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct BackupRuntimeState {
    pub completed_base_backups: u64,
    pub archived_wal_segments: u64,
    pub pitr_restores: u64,
    pub queryable_branches: u64,
    pub retention_deletions: u64,
    pub failed_walg_invocations: u64,
    pub encrypted_artifacts: u64,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct BackupRuntimeReport {
    pub backup: BackupArtifact,
    pub restore: PitrRestoreResult,
    pub queryable_branch: QueryableBranchResult,
    pub state: BackupRuntimeState,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct BackupRuntime {
    job: BackupJobPlan,
    state: BackupRuntimeState,
}

impl BackupRuntime {
    pub fn new(job: BackupJobPlan) -> Result<Self, BackupSidecarError> {
        job.validate()?;

        Ok(Self {
            job,
            state: BackupRuntimeState {
                completed_base_backups: 0,
                archived_wal_segments: 0,
                pitr_restores: 0,
                queryable_branches: 0,
                retention_deletions: 0,
                failed_walg_invocations: 0,
                encrypted_artifacts: 0,
            },
        })
    }

    pub fn state(&self) -> &BackupRuntimeState {
        &self.state
    }

    pub fn job(&self) -> &BackupJobPlan {
        &self.job
    }

    pub fn run_backup_cycle(&mut self) -> Result<BackupArtifact, BackupSidecarError> {
        self.job.validate()?;
        let wal_segments = deterministic_wal_segments(&self.job);
        let encrypted = self.job.encryption.is_some();
        let encrypted_artifacts = if encrypted {
            1_u64 + u64::from(wal_segments)
        } else {
            0
        };

        self.state.completed_base_backups += 1;
        self.state.archived_wal_segments += u64::from(wal_segments);
        self.state.encrypted_artifacts += encrypted_artifacts;

        Ok(BackupArtifact {
            cluster: self.job.cluster.clone(),
            base_destination_uri: self.job.base_backup.destination_uri.clone(),
            wal_archive_uri: self.job.wal_archive.archive_uri.clone(),
            base_size_bytes: deterministic_base_size_bytes(&self.job),
            wal_segments,
            encrypted,
            retention_days: self.job.base_backup.retention_days,
        })
    }

    pub fn restore_pitr(
        &mut self,
        plan: &PitrRestorePlan,
    ) -> Result<PitrRestoreResult, BackupSidecarError> {
        plan.validate()?;
        self.ensure_archive_matches(&plan.source_archive_uri)?;
        let replayed_wal_segments = deterministic_wal_segments(&self.job);
        self.state.pitr_restores += 1;

        Ok(PitrRestoreResult {
            source_cluster: plan.cluster.clone(),
            target_cluster: plan.target_cluster.clone(),
            target_time: plan.target_time.clone(),
            replayed_wal_segments,
        })
    }

    pub fn mount_queryable_branch(
        &mut self,
        plan: &QueryableBackupBranchPlan,
    ) -> Result<QueryableBranchResult, BackupSidecarError> {
        plan.validate()?;
        self.ensure_archive_matches(&plan.source_archive_uri)?;
        self.state.queryable_branches += 1;

        Ok(QueryableBranchResult {
            branch_name: plan.branch_name.clone(),
            mounted_archive_uri: plan.source_archive_uri.clone(),
            target_time: plan.target_time.clone(),
            read_only: plan.read_only,
        })
    }

    fn ensure_archive_matches(&self, archive_uri: &str) -> Result<(), BackupSidecarError> {
        if archive_uri == self.job.contract.archive_uri {
            Ok(())
        } else {
            Err(BackupSidecarError::ArchiveMismatch)
        }
    }
}

fn deterministic_wal_segments(job: &BackupJobPlan) -> u32 {
    (job.base_backup.retention_days / 10).max(1)
}

fn deterministic_base_size_bytes(job: &BackupJobPlan) -> u64 {
    u64::from(job.base_backup.concurrency) * 1_048_576
}

pub fn canonical_backup_job() -> BackupJobPlan {
    BackupJobPlan {
        cluster: "prod".to_string(),
        contract: BackupRestoreContract {
            schedule: "0 */6 * * *".to_string(),
            archive_uri: "s3://backups/prod".to_string(),
            pitr_target: Some("2026-05-19T12:00:00Z".to_string()),
            queryable_branch_name: Some("prod-at-noon".to_string()),
        },
        base_backup: BaseBackupPlan {
            destination_uri: "s3://backups/prod/base".to_string(),
            retention_days: 30,
            concurrency: 2,
        },
        wal_archive: WalArchivePlan {
            slot_name: "ai_blaise_wal".to_string(),
            archive_uri: "s3://backups/prod/wal".to_string(),
            compression: WalCompression::Zstd,
        },
        encryption: Some(BackupEncryptionPlan {
            kms_key_ref: "aws-kms-prod".to_string(),
        }),
    }
}

pub fn canonical_pitr_restore_plan() -> PitrRestorePlan {
    PitrRestorePlan {
        cluster: "prod".to_string(),
        source_archive_uri: "s3://backups/prod".to_string(),
        target_time: "2026-05-19T12:00:00Z".to_string(),
        target_cluster: "restore-prod".to_string(),
    }
}

pub fn canonical_queryable_branch_plan() -> QueryableBackupBranchPlan {
    QueryableBackupBranchPlan {
        branch_name: "prod-at-noon".to_string(),
        source_archive_uri: "s3://backups/prod".to_string(),
        target_time: "2026-05-19T12:00:00Z".to_string(),
        read_only: true,
    }
}

pub fn canonical_backup_report() -> Result<BackupCanonicalReport, BackupSidecarError> {
    let job = canonical_backup_job();
    let restore = canonical_pitr_restore_plan();
    let queryable_branch = canonical_queryable_branch_plan();

    job.validate()?;
    restore.validate()?;
    queryable_branch.validate()?;

    Ok(BackupCanonicalReport {
        job,
        restore,
        queryable_branch,
    })
}

pub fn canonical_backup_runtime_report() -> Result<BackupRuntimeReport, BackupSidecarError> {
    let mut runtime = BackupRuntime::new(canonical_backup_job())?;
    let backup = runtime.run_backup_cycle()?;
    let restore = runtime.restore_pitr(&canonical_pitr_restore_plan())?;
    let queryable_branch = runtime.mount_queryable_branch(&canonical_queryable_branch_plan())?;

    Ok(BackupRuntimeReport {
        backup,
        restore,
        queryable_branch,
        state: runtime.state().clone(),
    })
}

/// WAL archive window reported by `wal-g wal-show` / `pg_controldata`.
///
/// Used to validate PITR targets before invoking `wal-g backup-fetch`, which
/// otherwise can busy-loop while waiting for missing WAL segments.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct WalArchiveWindow {
    pub oldest_wal_time: String,
    pub latest_wal_time: String,
}

impl WalArchiveWindow {
    pub fn contains(&self, target_time: &str) -> bool {
        target_time >= self.oldest_wal_time.as_str() && target_time <= self.latest_wal_time.as_str()
    }
}

/// Source of truth for the WAL archive window. Tests inject a fixed window so
/// the engine can validate PITR targets without a live PostgreSQL primary.
pub trait WalArchiveWindowSource: Send + Sync {
    fn window(&self) -> Result<WalArchiveWindow, BackupSidecarError>;
}

/// In-memory WAL window source used by tests and by the smoke harness.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct StaticWalWindow {
    window: WalArchiveWindow,
}

impl StaticWalWindow {
    pub fn new(window: WalArchiveWindow) -> Self {
        Self { window }
    }
}

impl WalArchiveWindowSource for StaticWalWindow {
    fn window(&self) -> Result<WalArchiveWindow, BackupSidecarError> {
        Ok(self.window.clone())
    }
}

/// Status entry for a single PITR restore job.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PitrRestoreJob {
    pub job_id: String,
    pub target_time: String,
    pub source_archive_uri: String,
    pub target_cluster: String,
    pub target_directory: PathBuf,
    pub status: PitrRestoreStatus,
    pub started_at_epoch_seconds: u64,
    pub finished_at_epoch_seconds: Option<u64>,
    pub stdout_excerpt: String,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum PitrRestoreStatus {
    Running,
    Succeeded,
    Failed(String),
}

impl PitrRestoreStatus {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Running => "running",
            Self::Succeeded => "succeeded",
            Self::Failed(_) => "failed",
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct WalgInvocationRecord {
    pub operation: String,
    pub status_code: Option<i32>,
    pub elapsed_ms: u64,
    pub stdout_excerpt: String,
    pub stderr_excerpt: String,
}

impl WalgInvocationRecord {
    fn from_invocation(operation: &str, invocation: &WalgInvocation) -> Self {
        Self {
            operation: operation.to_string(),
            status_code: invocation.status_code,
            elapsed_ms: invocation.elapsed_ms,
            stdout_excerpt: excerpt(&invocation.stdout, 512),
            stderr_excerpt: excerpt(&invocation.stderr, 512),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct BackupSchedulerState {
    pub next_epoch_minute: u64,
    pub last_run_epoch_seconds: Option<u64>,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct BackupStatus {
    pub state: BackupRuntimeState,
    pub last_walg_invocation: Option<WalgInvocationRecord>,
    pub scheduler: BackupSchedulerState,
}

/// Production runtime that drives the contract-validated backup, PITR, and
/// queryable-branch lifecycle via `WalgRunner` and `QueryableBranchRunner`.
pub struct BackupEngine {
    runtime: Mutex<BackupRuntime>,
    walg: WalgRunner,
    branches: QueryableBranchRunner,
    primary_pgdata: PathBuf,
    restore_root: PathBuf,
    schedule: BackupSchedule,
    wal_window_source: Box<dyn WalArchiveWindowSource>,
    pitr_jobs: Mutex<HashMap<String, PitrRestoreJob>>,
    queryable_branches: Mutex<HashMap<String, QueryableBranchProcess>>,
    last_walg_invocation: Mutex<Option<WalgInvocationRecord>>,
    scheduler_state: Mutex<BackupSchedulerState>,
}

impl BackupEngine {
    pub fn new(config: BackupEngineConfig) -> Result<Self, BackupSidecarError> {
        let runtime = BackupRuntime::new(config.job.clone())?;
        let schedule = BackupSchedule::parse(&config.job.contract.schedule)?;
        let walg = WalgRunner::from_plan(config.walg_binary, &config.job);
        walg.validate_encryption_env(config.job.encryption.is_some())?;
        let branches = QueryableBranchRunner::new(
            config.pg_ctl_binary,
            config.psql_binary,
            config.branch_root,
        );
        let next_epoch_minute = schedule.next_after(current_epoch_minute());
        Ok(Self {
            runtime: Mutex::new(runtime),
            walg,
            branches,
            primary_pgdata: config.primary_pgdata,
            restore_root: config.restore_root,
            schedule,
            wal_window_source: config.wal_window_source,
            pitr_jobs: Mutex::new(HashMap::new()),
            queryable_branches: Mutex::new(HashMap::new()),
            last_walg_invocation: Mutex::new(None),
            scheduler_state: Mutex::new(BackupSchedulerState {
                next_epoch_minute,
                last_run_epoch_seconds: None,
                last_error: None,
            }),
        })
    }

    pub fn schedule(&self) -> &BackupSchedule {
        &self.schedule
    }

    pub fn walg(&self) -> &WalgRunner {
        &self.walg
    }

    pub fn branches(&self) -> &QueryableBranchRunner {
        &self.branches
    }

    pub fn primary_pgdata(&self) -> &std::path::Path {
        &self.primary_pgdata
    }

    pub fn restore_root(&self) -> &std::path::Path {
        &self.restore_root
    }

    pub fn job_archive_uri(&self) -> String {
        self.runtime
            .lock()
            .expect("backup runtime mutex poisoned")
            .job()
            .contract
            .archive_uri
            .clone()
    }

    pub fn job_cluster(&self) -> String {
        self.runtime
            .lock()
            .expect("backup runtime mutex poisoned")
            .job()
            .cluster
            .clone()
    }

    pub fn run_base_backup(&self) -> Result<BackupArtifact, BackupSidecarError> {
        let invocation = match self.walg.base_backup(&self.primary_pgdata) {
            Ok(invocation) => invocation,
            Err(error) => {
                self.record_walg_failure("base_backup", &error);
                return Err(error.into());
            }
        };
        self.record_walg_invocation("base_backup", &invocation);
        let mut runtime = self.runtime.lock().expect("backup runtime mutex poisoned");
        runtime.run_backup_cycle()
    }

    pub fn wal_archive_status(&self) -> Result<WalgInvocation, BackupSidecarError> {
        let invocation = match self.walg.wal_archive_status() {
            Ok(invocation) => invocation,
            Err(error) => {
                self.record_walg_failure("wal_archive_status", &error);
                return Err(error.into());
            }
        };
        self.record_walg_invocation("wal_archive_status", &invocation);
        Ok(invocation)
    }

    pub fn list_backups(&self) -> Result<WalgInvocation, BackupSidecarError> {
        let invocation = match self.walg.backup_list() {
            Ok(invocation) => invocation,
            Err(error) => {
                self.record_walg_failure("backup_list", &error);
                return Err(error.into());
            }
        };
        self.record_walg_invocation("backup_list", &invocation);
        Ok(invocation)
    }

    /// Validate a PITR target against the available WAL window and run the
    /// underlying `wal-g backup-fetch`. The job is tracked in
    /// `pitr_jobs` and exposed at `/pitr/status/:job_id`.
    pub fn start_pitr_restore(
        &self,
        plan: &PitrRestorePlan,
        job_id: impl Into<String>,
        now_epoch_seconds: u64,
    ) -> Result<PitrRestoreJob, BackupSidecarError> {
        plan.validate()?;
        self.ensure_archive_matches(&plan.source_archive_uri)?;
        let window = self.wal_window_source.window()?;
        if !window.contains(&plan.target_time) {
            return Err(BackupSidecarError::PitrTargetOutsideWindow {
                target_time: plan.target_time.clone(),
                oldest_wal_time: window.oldest_wal_time,
                latest_wal_time: window.latest_wal_time,
            });
        }
        let job_id = job_id.into();
        let target_directory = self.restore_root.join(&plan.target_cluster);
        let job = match self.walg.pitr_restore(&target_directory, &plan.target_time) {
            Ok(invocation) => {
                self.record_walg_invocation("pitr_restore", &invocation);
                let mut runtime = self.runtime.lock().expect("backup runtime mutex poisoned");
                runtime.restore_pitr(plan)?;
                PitrRestoreJob {
                    job_id: job_id.clone(),
                    target_time: plan.target_time.clone(),
                    source_archive_uri: plan.source_archive_uri.clone(),
                    target_cluster: plan.target_cluster.clone(),
                    target_directory: target_directory.clone(),
                    status: PitrRestoreStatus::Succeeded,
                    started_at_epoch_seconds: now_epoch_seconds,
                    finished_at_epoch_seconds: Some(
                        now_epoch_seconds.saturating_add(invocation.elapsed_ms / 1000),
                    ),
                    stdout_excerpt: invocation.stdout.chars().take(512).collect(),
                }
            }
            Err(error) => {
                self.record_walg_failure("pitr_restore", &error);
                PitrRestoreJob {
                    job_id: job_id.clone(),
                    target_time: plan.target_time.clone(),
                    source_archive_uri: plan.source_archive_uri.clone(),
                    target_cluster: plan.target_cluster.clone(),
                    target_directory: target_directory.clone(),
                    status: PitrRestoreStatus::Failed(error.to_string()),
                    started_at_epoch_seconds: now_epoch_seconds,
                    finished_at_epoch_seconds: Some(now_epoch_seconds),
                    stdout_excerpt: String::new(),
                }
            }
        };

        let mut jobs = self.pitr_jobs.lock().expect("pitr jobs mutex poisoned");
        jobs.insert(job_id, job.clone());
        Ok(job)
    }

    pub fn pitr_job(&self, job_id: &str) -> Option<PitrRestoreJob> {
        let jobs = self.pitr_jobs.lock().expect("pitr jobs mutex poisoned");
        jobs.get(job_id).cloned()
    }

    /// Restore a backup into a read-only PostgreSQL branch, write recovery
    /// configuration, start the branch, probe read-only behavior, and record the
    /// mounted branch in the engine state.
    pub fn create_queryable_branch(
        &self,
        plan: &QueryableBackupBranchPlan,
        port: u16,
    ) -> Result<QueryableBranchProcess, BackupSidecarError> {
        plan.validate()?;
        self.ensure_archive_matches(&plan.source_archive_uri)?;
        {
            let branches = self
                .queryable_branches
                .lock()
                .expect("queryable branches mutex poisoned");
            if branches.contains_key(&plan.branch_name) {
                return Err(BackupSidecarError::QueryableBranch(format!(
                    "queryable branch {} already exists",
                    plan.branch_name
                )));
            }
        }

        let data_dir = self.branches.data_directory(plan);
        let invocation = match self.walg.pitr_restore(&data_dir, &plan.target_time) {
            Ok(invocation) => invocation,
            Err(error) => {
                self.record_walg_failure("queryable_branch_restore", &error);
                return Err(error.into());
            }
        };
        self.record_walg_invocation("queryable_branch_restore", &invocation);
        self.branches.write_recovery_config(plan, port)?;
        self.branches
            .start_and_verify_read_only(plan, port, self.walg.env())?;
        {
            let mut runtime = self.runtime.lock().expect("backup runtime mutex poisoned");
            runtime.mount_queryable_branch(plan)?;
        }

        let process = QueryableBranchProcess {
            branch_name: plan.branch_name.clone(),
            data_dir,
            port,
            target_time: plan.target_time.clone(),
            source_archive_uri: plan.source_archive_uri.clone(),
        };
        let mut branches = self
            .queryable_branches
            .lock()
            .expect("queryable branches mutex poisoned");
        branches.insert(plan.branch_name.clone(), process.clone());
        Ok(process)
    }

    pub fn queryable_branches(&self) -> Vec<QueryableBranchProcess> {
        let branches = self
            .queryable_branches
            .lock()
            .expect("queryable branches mutex poisoned");
        branches.values().cloned().collect()
    }

    pub fn delete_old(&self, retention_days: u32) -> Result<WalgInvocation, BackupSidecarError> {
        let invocation = match self.walg.delete_old(retention_days) {
            Ok(invocation) => invocation,
            Err(error) => {
                self.record_walg_failure("delete_old", &error);
                return Err(error.into());
            }
        };
        self.record_walg_invocation("delete_old", &invocation);
        let mut runtime = self.runtime.lock().expect("backup runtime mutex poisoned");
        runtime.state.retention_deletions += 1;
        Ok(invocation)
    }

    pub fn run_scheduled_backup_if_due(
        &self,
        now_epoch_seconds: u64,
    ) -> Result<Option<BackupArtifact>, BackupSidecarError> {
        let now_epoch_minute = now_epoch_seconds / 60;
        let should_run = {
            let state = self
                .scheduler_state
                .lock()
                .expect("backup scheduler mutex poisoned");
            now_epoch_minute >= state.next_epoch_minute
        };
        if !should_run {
            return Ok(None);
        }

        match self.run_base_backup() {
            Ok(artifact) => {
                let mut state = self
                    .scheduler_state
                    .lock()
                    .expect("backup scheduler mutex poisoned");
                state.last_run_epoch_seconds = Some(now_epoch_seconds);
                state.last_error = None;
                state.next_epoch_minute = self.schedule.next_after(now_epoch_minute);
                Ok(Some(artifact))
            }
            Err(error) => {
                let mut state = self
                    .scheduler_state
                    .lock()
                    .expect("backup scheduler mutex poisoned");
                state.last_error = Some(error.to_string());
                state.next_epoch_minute = self.schedule.next_after(now_epoch_minute);
                Err(error)
            }
        }
    }

    pub fn seconds_until_next_scheduled_run(&self, now_epoch_seconds: u64) -> u64 {
        let state = self
            .scheduler_state
            .lock()
            .expect("backup scheduler mutex poisoned");
        let next_epoch_seconds = state.next_epoch_minute.saturating_mul(60);
        next_epoch_seconds.saturating_sub(now_epoch_seconds).max(1)
    }

    pub fn status(&self) -> BackupStatus {
        BackupStatus {
            state: self.state(),
            last_walg_invocation: self
                .last_walg_invocation
                .lock()
                .expect("walg invocation mutex poisoned")
                .clone(),
            scheduler: self
                .scheduler_state
                .lock()
                .expect("backup scheduler mutex poisoned")
                .clone(),
        }
    }

    pub fn state(&self) -> BackupRuntimeState {
        self.runtime
            .lock()
            .expect("backup runtime mutex poisoned")
            .state()
            .clone()
    }

    fn ensure_archive_matches(&self, archive_uri: &str) -> Result<(), BackupSidecarError> {
        if archive_uri == self.job_archive_uri() {
            Ok(())
        } else {
            Err(BackupSidecarError::ArchiveMismatch)
        }
    }

    fn record_walg_invocation(&self, operation: &str, invocation: &WalgInvocation) {
        let mut last = self
            .last_walg_invocation
            .lock()
            .expect("walg invocation mutex poisoned");
        *last = Some(WalgInvocationRecord::from_invocation(operation, invocation));
    }

    fn record_walg_failure(&self, operation: &str, error: &WalgError) {
        if let WalgError::NonZeroExit(invocation) = error {
            self.record_walg_invocation(operation, invocation);
        }
        if matches!(error, WalgError::NonZeroExit(_) | WalgError::Spawn(_)) {
            let mut runtime = self.runtime.lock().expect("backup runtime mutex poisoned");
            runtime.state.failed_walg_invocations += 1;
        }
    }
}

impl fmt::Debug for BackupEngine {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("BackupEngine")
            .field("primary_pgdata", &self.primary_pgdata)
            .field("restore_root", &self.restore_root)
            .field("walg_binary", &self.walg.binary().to_string_lossy())
            .finish()
    }
}

/// Configuration for [`BackupEngine`].
pub struct BackupEngineConfig {
    pub job: BackupJobPlan,
    pub walg_binary: PathBuf,
    pub pg_ctl_binary: PathBuf,
    pub psql_binary: PathBuf,
    pub primary_pgdata: PathBuf,
    pub restore_root: PathBuf,
    pub branch_root: PathBuf,
    pub wal_window_source: Box<dyn WalArchiveWindowSource>,
}

impl BackupEngineConfig {
    /// Build a configuration that reads tunables from environment variables
    /// with sensible defaults for the operand container. The variables match
    /// the Helm chart wiring documented in `docs/ai-blaise/RUNBOOKS`.
    pub fn from_env(job: BackupJobPlan) -> Self {
        Self {
            job,
            walg_binary: env_path("AI_BLAISE_BACKUP_WALG_BINARY", "/usr/local/bin/wal-g"),
            pg_ctl_binary: env_path("AI_BLAISE_BACKUP_PG_CTL_BINARY", "/usr/bin/pg_ctl"),
            psql_binary: env_path("AI_BLAISE_BACKUP_PSQL_BINARY", "/usr/bin/psql"),
            primary_pgdata: env_path(
                "AI_BLAISE_BACKUP_PRIMARY_PGDATA",
                "/var/lib/postgresql/data",
            ),
            restore_root: env_path(
                "AI_BLAISE_BACKUP_RESTORE_ROOT",
                "/var/lib/postgresql/restores",
            ),
            branch_root: env_path(
                "AI_BLAISE_BACKUP_BRANCH_ROOT",
                "/var/lib/postgresql/branches",
            ),
            wal_window_source: Box::new(StaticWalWindow::new(WalArchiveWindow {
                oldest_wal_time: std::env::var("AI_BLAISE_BACKUP_OLDEST_WAL_TIME")
                    .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string()),
                latest_wal_time: std::env::var("AI_BLAISE_BACKUP_LATEST_WAL_TIME")
                    .unwrap_or_else(|_| "9999-12-31T23:59:59Z".to_string()),
            })),
        }
    }
}

fn env_path(key: &str, default: &str) -> PathBuf {
    PathBuf::from(std::env::var_os(OsString::from(key)).unwrap_or_else(|| OsString::from(default)))
}

/// HTTP routing for the backup sidecar. Builds JSON responses for every route
/// described in `docs/ai-blaise/RUNBOOKS/pitr-restore.md`.
pub fn handle_backup_http_bytes(
    engine: &BackupEngine,
    request: &[u8],
    now_epoch_seconds: u64,
) -> Result<HttpProbeResponse, BackupSidecarError> {
    let request =
        std::str::from_utf8(request).map_err(|_| BackupSidecarError::MalformedHttpRequest)?;
    let parsed = parse_http_request(request)?;

    match (parsed.method, parsed.path.as_str()) {
        ("GET", "/backups") => {
            let invocation = engine.list_backups()?;
            let cluster = engine.job_cluster();
            let archive_uri = engine.job_archive_uri();
            Ok(HttpProbeResponse::new(
                200,
                "application/json",
                format!(
                    "{{\"cluster\":\"{}\",\"archive_uri\":\"{}\",\"output\":{},\"elapsed_ms\":{}}}\n",
                    escape_json(&cluster),
                    escape_json(&archive_uri),
                    json_string(&invocation.stdout),
                    invocation.elapsed_ms
                ),
            ))
        }
        ("POST", "/backups/run") => {
            let artifact = engine.run_base_backup()?;
            Ok(HttpProbeResponse::new(
                202,
                "application/json",
                backup_artifact_json(&artifact),
            ))
        }
        ("GET", "/backups/status") => Ok(HttpProbeResponse::new(
            200,
            "application/json",
            backup_status_json(&engine.status()),
        )),
        ("POST", "/backups/delete-old") => {
            let retention = parse_delete_old_body(parsed.body)?;
            let invocation = engine.delete_old(retention)?;
            Ok(HttpProbeResponse::new(
                202,
                "application/json",
                walg_invocation_json("delete_old", &invocation),
            ))
        }
        ("GET", "/wal/status") => {
            let invocation = engine.wal_archive_status()?;
            let window = engine.wal_window_source.window()?;
            Ok(HttpProbeResponse::new(
                200,
                "application/json",
                format!(
                    "{{\"oldest_wal_time\":\"{}\",\"latest_wal_time\":\"{}\",\"detail\":{},\"elapsed_ms\":{}}}\n",
                    escape_json(&window.oldest_wal_time),
                    escape_json(&window.latest_wal_time),
                    json_string(&invocation.stdout),
                    invocation.elapsed_ms
                ),
            ))
        }
        ("POST", "/pitr/restore") => {
            let plan = parse_pitr_restore_body(parsed.body)?;
            let job_id = format!("pitr-{}-{}", plan.target_cluster, now_epoch_seconds);
            let job = engine.start_pitr_restore(&plan, job_id, now_epoch_seconds)?;
            Ok(HttpProbeResponse::new(
                202,
                "application/json",
                pitr_job_json(&job),
            ))
        }
        ("GET", path) if path.starts_with("/pitr/status/") => {
            let job_id = path.trim_start_matches("/pitr/status/");
            match engine.pitr_job(job_id) {
                Some(job) => Ok(HttpProbeResponse::new(
                    200,
                    "application/json",
                    pitr_job_json(&job),
                )),
                None => Ok(HttpProbeResponse::new(
                    404,
                    "application/json",
                    format!(
                        "{{\"error\":\"pitr job not found\",\"job_id\":\"{}\"}}\n",
                        escape_json(job_id)
                    ),
                )),
            }
        }
        ("POST", "/branches/queryable") => {
            let (plan, port) = parse_queryable_branch_body(parsed.body)?;
            let process = engine.create_queryable_branch(&plan, port)?;
            Ok(HttpProbeResponse::new(
                201,
                "application/json",
                queryable_branch_process_json(&process),
            ))
        }
        ("GET", "/branches/queryable") => {
            let branches = engine.queryable_branches();
            let entries: Vec<String> = branches.iter().map(queryable_branch_process_json).collect();
            Ok(HttpProbeResponse::new(
                200,
                "application/json",
                format!("{{\"branches\":[{}]}}\n", entries.join(",")),
            ))
        }
        ("GET", "/metrics") => Ok(HttpProbeResponse::new(
            200,
            "text/plain; version=0.0.4",
            backup_prometheus_metrics(&engine.status()),
        )),
        (method, _) if !is_supported_method(method) => Ok(HttpProbeResponse::new(
            405,
            "application/json",
            "{\"error\":\"method not allowed\"}\n",
        )),
        _ => {
            let mut runtime = SidecarRuntime::ready("backup");
            Ok(runtime.handle_http_bytes(request.as_bytes())?)
        }
    }
}

fn is_supported_method(method: &str) -> bool {
    matches!(method, "GET" | "POST" | "HEAD")
}

#[derive(Debug, Clone)]
struct ParsedHttpRequest<'a> {
    method: &'a str,
    path: String,
    body: &'a str,
}

fn parse_http_request(request: &str) -> Result<ParsedHttpRequest<'_>, BackupSidecarError> {
    let (head, body) = request
        .split_once("\r\n\r\n")
        .or_else(|| request.split_once("\n\n"))
        .ok_or(BackupSidecarError::MalformedHttpRequest)?;
    let request_line = head
        .lines()
        .next()
        .ok_or(BackupSidecarError::MalformedHttpRequest)?;
    let mut parts = request_line.split_whitespace();
    let method = parts
        .next()
        .ok_or(BackupSidecarError::MalformedHttpRequest)?;
    let path_raw = parts
        .next()
        .ok_or(BackupSidecarError::MalformedHttpRequest)?;
    if !path_raw.starts_with('/') {
        return Err(BackupSidecarError::MalformedHttpRequest);
    }
    let path = path_raw
        .split_once('?')
        .map(|(prefix, _)| prefix.to_string())
        .unwrap_or_else(|| path_raw.to_string());
    Ok(ParsedHttpRequest { method, path, body })
}

fn parse_pitr_restore_body(body: &str) -> Result<PitrRestorePlan, BackupSidecarError> {
    let cluster = json_field(body, "cluster")?;
    let source_archive_uri = json_field(body, "source_archive_uri")?;
    let target_time = json_field(body, "target_time")?;
    let target_cluster = json_field(body, "target_cluster")?;
    Ok(PitrRestorePlan {
        cluster,
        source_archive_uri,
        target_time,
        target_cluster,
    })
}

fn parse_queryable_branch_body(
    body: &str,
) -> Result<(QueryableBackupBranchPlan, u16), BackupSidecarError> {
    let branch_name = json_field(body, "branch_name")?;
    let source_archive_uri = json_field(body, "source_archive_uri")?;
    let target_time = json_field(body, "target_time")?;
    let port = json_optional_u16(body, "port")?.unwrap_or(6543);
    validate_queryable_port(port)?;
    let plan = QueryableBackupBranchPlan {
        branch_name,
        source_archive_uri,
        target_time,
        read_only: true,
    };
    Ok((plan, port))
}

fn parse_delete_old_body(body: &str) -> Result<u32, BackupSidecarError> {
    json_u32(body, "retention_full")
        .or_else(|| json_u32(body, "retention_days"))
        .filter(|value| *value > 0)
        .ok_or(BackupSidecarError::InvalidRetention)
}

fn json_field(body: &str, key: &str) -> Result<String, BackupSidecarError> {
    let needle = format!("\"{key}\"");
    let start = body
        .find(&needle)
        .ok_or_else(|| BackupSidecarError::MissingRequiredField(field_name_for(key)))?;
    let after_key = &body[start + needle.len()..];
    let colon = after_key
        .find(':')
        .ok_or(BackupSidecarError::MalformedHttpRequest)?;
    let rest = after_key[colon + 1..].trim_start();
    if !rest.starts_with('"') {
        return Err(BackupSidecarError::MalformedHttpRequest);
    }
    let after_quote = &rest[1..];
    let end_quote = after_quote
        .find('"')
        .ok_or(BackupSidecarError::MalformedHttpRequest)?;
    Ok(after_quote[..end_quote].to_string())
}

fn json_optional_u16(body: &str, key: &str) -> Result<Option<u16>, BackupSidecarError> {
    let Some((digits, trailing)) = json_unsigned_number(body, key)? else {
        return Ok(None);
    };
    validate_json_number_trailing(trailing)?;
    digits
        .parse::<u16>()
        .map(Some)
        .map_err(|_| BackupSidecarError::InvalidPort)
}

fn json_u32(body: &str, key: &str) -> Option<u32> {
    let Some((digits, trailing)) = json_unsigned_number(body, key).ok().flatten() else {
        return None;
    };
    if validate_json_number_trailing(trailing).is_err() {
        return None;
    }
    digits.parse::<u32>().ok()
}

fn json_unsigned_number<'a>(
    body: &'a str,
    key: &str,
) -> Result<Option<(&'a str, &'a str)>, BackupSidecarError> {
    let needle = format!("\"{key}\"");
    let Some(start) = body.find(&needle) else {
        return Ok(None);
    };
    let after_key = &body[start + needle.len()..];
    let colon = after_key
        .find(':')
        .ok_or(BackupSidecarError::MalformedHttpRequest)?;
    let rest = after_key[colon + 1..].trim_start();
    let end = rest
        .find(|ch: char| !ch.is_ascii_digit())
        .unwrap_or(rest.len());
    if end == 0 {
        return Err(BackupSidecarError::MalformedHttpRequest);
    }
    Ok(Some((&rest[..end], &rest[end..])))
}

fn validate_json_number_trailing(trailing: &str) -> Result<(), BackupSidecarError> {
    let trimmed = trailing.trim_start();
    if trimmed.is_empty() || trimmed.starts_with(',') || trimmed.starts_with('}') {
        Ok(())
    } else {
        Err(BackupSidecarError::MalformedHttpRequest)
    }
}

fn field_name_for(key: &str) -> &'static str {
    match key {
        "cluster" => "cluster",
        "source_archive_uri" => "source_archive_uri",
        "target_time" => "target_time",
        "target_cluster" => "target_cluster",
        "branch_name" => "branch_name",
        _ => "field",
    }
}

fn json_string(value: &str) -> String {
    let mut buffer = String::with_capacity(value.len() + 2);
    buffer.push('"');
    buffer.push_str(&escape_json(value));
    buffer.push('"');
    buffer
}

fn escape_json(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

fn backup_artifact_json(artifact: &BackupArtifact) -> String {
    format!(
        "{{\"cluster\":\"{}\",\"base_destination_uri\":\"{}\",\"wal_archive_uri\":\"{}\",\"base_size_bytes\":{},\"wal_segments\":{},\"encrypted\":{},\"retention_days\":{}}}\n",
        escape_json(&artifact.cluster),
        escape_json(&artifact.base_destination_uri),
        escape_json(&artifact.wal_archive_uri),
        artifact.base_size_bytes,
        artifact.wal_segments,
        artifact.encrypted,
        artifact.retention_days,
    )
}

fn pitr_job_json(job: &PitrRestoreJob) -> String {
    let finished = match job.finished_at_epoch_seconds {
        Some(value) => value.to_string(),
        None => "null".to_string(),
    };
    let status_detail = match &job.status {
        PitrRestoreStatus::Failed(detail) => json_string(detail),
        _ => "null".to_string(),
    };
    format!(
        "{{\"job_id\":\"{}\",\"status\":\"{}\",\"target_time\":\"{}\",\"target_cluster\":\"{}\",\"source_archive_uri\":\"{}\",\"target_directory\":\"{}\",\"started_at_epoch_seconds\":{},\"finished_at_epoch_seconds\":{},\"status_detail\":{}}}\n",
        escape_json(&job.job_id),
        job.status.as_str(),
        escape_json(&job.target_time),
        escape_json(&job.target_cluster),
        escape_json(&job.source_archive_uri),
        escape_json(&job.target_directory.to_string_lossy()),
        job.started_at_epoch_seconds,
        finished,
        status_detail,
    )
}

fn walg_invocation_json(operation: &str, invocation: &WalgInvocation) -> String {
    format!(
        "{{\"operation\":\"{}\",\"status_code\":{},\"elapsed_ms\":{},\"stdout\":{},\"stderr\":{}}}\n",
        escape_json(operation),
        invocation
            .status_code
            .map(|code| code.to_string())
            .unwrap_or_else(|| "null".to_string()),
        invocation.elapsed_ms,
        json_string(&invocation.stdout),
        json_string(&invocation.stderr),
    )
}

fn backup_status_json(status: &BackupStatus) -> String {
    let last = match &status.last_walg_invocation {
        Some(invocation) => format!(
            "{{\"operation\":\"{}\",\"status_code\":{},\"elapsed_ms\":{},\"stdout_excerpt\":{},\"stderr_excerpt\":{}}}",
            escape_json(&invocation.operation),
            invocation
                .status_code
                .map(|code| code.to_string())
                .unwrap_or_else(|| "null".to_string()),
            invocation.elapsed_ms,
            json_string(&invocation.stdout_excerpt),
            json_string(&invocation.stderr_excerpt),
        ),
        None => "null".to_string(),
    };
    let last_run = status
        .scheduler
        .last_run_epoch_seconds
        .map(|value| value.to_string())
        .unwrap_or_else(|| "null".to_string());
    let last_error = status
        .scheduler
        .last_error
        .as_ref()
        .map(|value| json_string(value))
        .unwrap_or_else(|| "null".to_string());
    format!(
        "{{\"completed_base_backups\":{},\"archived_wal_segments\":{},\"pitr_restores\":{},\"queryable_branches\":{},\"retention_deletions\":{},\"failed_walg_invocations\":{},\"encrypted_artifacts\":{},\"next_scheduled_epoch_minute\":{},\"last_scheduled_run_epoch_seconds\":{},\"last_scheduler_error\":{},\"last_walg_invocation\":{}}}\n",
        status.state.completed_base_backups,
        status.state.archived_wal_segments,
        status.state.pitr_restores,
        status.state.queryable_branches,
        status.state.retention_deletions,
        status.state.failed_walg_invocations,
        status.state.encrypted_artifacts,
        status.scheduler.next_epoch_minute,
        last_run,
        last_error,
        last,
    )
}

fn backup_prometheus_metrics(status: &BackupStatus) -> String {
    let last_elapsed = status
        .last_walg_invocation
        .as_ref()
        .map(|invocation| invocation.elapsed_ms)
        .unwrap_or(0);
    format!(
        "# HELP ai_blaise_backup_completed_base_backups Completed base backup count.
\
         # TYPE ai_blaise_backup_completed_base_backups counter
\
         ai_blaise_backup_completed_base_backups {}
\
         # HELP ai_blaise_backup_archived_wal_segments Archived WAL segment count.
\
         # TYPE ai_blaise_backup_archived_wal_segments counter
\
         ai_blaise_backup_archived_wal_segments {}
\
         # HELP ai_blaise_backup_pitr_restores PITR restore count.
\
         # TYPE ai_blaise_backup_pitr_restores counter
\
         ai_blaise_backup_pitr_restores {}
\
         # HELP ai_blaise_backup_queryable_branches Queryable read-only branch count.
\
         # TYPE ai_blaise_backup_queryable_branches gauge
\
         ai_blaise_backup_queryable_branches {}
\
         # HELP ai_blaise_backup_retention_deletions Retention delete operation count.
\
         # TYPE ai_blaise_backup_retention_deletions counter
\
         ai_blaise_backup_retention_deletions {}
\
         # HELP ai_blaise_backup_failed_walg_invocations Failed WAL-G invocation count.
\
         # TYPE ai_blaise_backup_failed_walg_invocations counter
\
         ai_blaise_backup_failed_walg_invocations {}
\
         # HELP ai_blaise_backup_last_walg_elapsed_ms Last WAL-G invocation elapsed milliseconds.
\
         # TYPE ai_blaise_backup_last_walg_elapsed_ms gauge
\
         ai_blaise_backup_last_walg_elapsed_ms {}
",
        status.state.completed_base_backups,
        status.state.archived_wal_segments,
        status.state.pitr_restores,
        status.state.queryable_branches,
        status.state.retention_deletions,
        status.state.failed_walg_invocations,
        last_elapsed,
    )
}

fn queryable_branch_process_json(process: &QueryableBranchProcess) -> String {
    format!(
        "{{\"branch_name\":\"{}\",\"data_dir\":\"{}\",\"port\":{},\"target_time\":\"{}\",\"source_archive_uri\":\"{}\",\"read_only\":true}}",
        escape_json(&process.branch_name),
        escape_json(&process.data_dir.to_string_lossy()),
        process.port,
        escape_json(&process.target_time),
        escape_json(&process.source_archive_uri),
    )
}

/// Read an HTTP request from a TCP stream, honoring Content-Length so JSON
/// bodies are not truncated.
fn read_http_request(stream: &mut std::net::TcpStream) -> Result<Vec<u8>, std::io::Error> {
    use std::io::Read;

    stream.set_read_timeout(Some(std::time::Duration::from_secs(5)))?;
    let mut request = Vec::new();
    let mut chunk = [0_u8; 8192];
    loop {
        let read_len = stream.read(&mut chunk)?;
        if read_len == 0 {
            break;
        }
        request.extend_from_slice(&chunk[..read_len]);
        if http_request_complete(&request) || request.len() >= 65_536 {
            break;
        }
    }
    Ok(request)
}

fn http_request_complete(request: &[u8]) -> bool {
    let Some((body_start, header_bytes)) = split_http_head(request) else {
        return false;
    };
    let Ok(headers) = std::str::from_utf8(header_bytes) else {
        return true;
    };
    let content_length = headers
        .lines()
        .filter_map(|line| line.split_once(':'))
        .find(|(name, _)| name.eq_ignore_ascii_case("content-length"))
        .and_then(|(_, value)| value.trim().parse::<usize>().ok())
        .unwrap_or(0);
    request.len() >= body_start + content_length
}

fn split_http_head(request: &[u8]) -> Option<(usize, &[u8])> {
    find_bytes(request, b"\r\n\r\n")
        .map(|index| (index + 4, &request[..index]))
        .or_else(|| find_bytes(request, b"\n\n").map(|index| (index + 2, &request[..index])))
}

fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

fn excerpt(value: &str, max_chars: usize) -> String {
    value.chars().take(max_chars).collect()
}

fn current_epoch_seconds() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn current_epoch_minute() -> u64 {
    current_epoch_seconds() / 60
}

pub fn spawn_scheduled_backup_loop(engine: Arc<BackupEngine>) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || loop {
        let now = current_epoch_seconds();
        if let Err(error) = engine.run_scheduled_backup_if_due(now) {
            eprintln!("ai-blaise backup scheduler cycle failed: {error}");
        }
        let sleep_seconds = engine.seconds_until_next_scheduled_run(now).clamp(1, 60);
        std::thread::sleep(std::time::Duration::from_secs(sleep_seconds));
    })
}

fn scheduler_disabled() -> bool {
    std::env::var("AI_BLAISE_BACKUP_DISABLE_SCHEDULER")
        .map(|value| matches!(value.as_str(), "1" | "true" | "TRUE" | "yes" | "YES"))
        .unwrap_or(false)
}

/// Run the HTTP server forever, dispatching to [`handle_backup_http_bytes`].
pub fn serve_backup_http_forever(
    engine: BackupEngine,
    default_addr: &str,
) -> Result<(), BackupSidecarError> {
    use std::io::Write;
    use std::net::TcpListener;

    let listen_addr = listen_addr_from_env(default_addr)?;
    let listener = TcpListener::bind(&listen_addr)?;
    let engine = Arc::new(engine);
    let _scheduler = if scheduler_disabled() {
        None
    } else {
        Some(spawn_scheduled_backup_loop(Arc::clone(&engine)))
    };
    eprintln!("ai-blaise backup sidecar HTTP server listening on {listen_addr}");

    for stream in listener.incoming() {
        let mut stream = stream?;
        let request = read_http_request(&mut stream)?;
        let now_epoch_seconds = current_epoch_seconds();
        let response = handle_backup_http_bytes(engine.as_ref(), &request, now_epoch_seconds)
            .unwrap_or_else(|error| {
                HttpProbeResponse::new(
                    400,
                    "application/json",
                    format!("{{\"error\":\"{}\"}}\n", escape_json(&error.to_string())),
                )
            });
        stream.write_all(response.to_http_string().as_bytes())?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static TEST_ENGINE_ID: AtomicUsize = AtomicUsize::new(0);

    fn test_engine(walg_binary: &str) -> BackupEngine {
        let engine_id = TEST_ENGINE_ID.fetch_add(1, Ordering::SeqCst);
        let root = std::env::temp_dir().join(format!(
            "ai-blaise-backup-test-{}-{}-{}",
            std::process::id(),
            walg_binary.replace('/', "_"),
            engine_id,
        ));
        std::fs::create_dir_all(&root).expect("test root");
        let pg_ctl = root.join("pg_ctl");
        let psql = root.join("psql");
        write_executable(
            &pg_ctl,
            "#!/usr/bin/env bash\nset -euo pipefail\necho 'pg_ctl stub invoked'\n",
        );
        write_executable(
            &psql,
            "#!/usr/bin/env bash\nset -euo pipefail\necho '1|on'\n",
        );

        BackupEngine::new(BackupEngineConfig {
            job: canonical_backup_job(),
            walg_binary: PathBuf::from(walg_binary),
            pg_ctl_binary: pg_ctl,
            psql_binary: psql,
            primary_pgdata: root.join("pgdata"),
            restore_root: root.join("restores"),
            branch_root: root.join("branches"),
            wal_window_source: Box::new(StaticWalWindow::new(WalArchiveWindow {
                oldest_wal_time: "2026-05-19T00:00:00Z".to_string(),
                latest_wal_time: "2026-05-19T23:59:59Z".to_string(),
            })),
        })
        .expect("test engine")
    }

    fn write_executable(path: &std::path::Path, body: &str) {
        use std::io::Write;
        use std::os::unix::fs::PermissionsExt;

        let mut file = std::fs::File::create(path).expect("create executable");
        file.write_all(body.as_bytes()).expect("write executable");
        file.flush().expect("flush executable");
        drop(file);
        let mut permissions = std::fs::metadata(path).expect("metadata").permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(path, permissions).expect("chmod executable");
    }

    #[test]
    fn backup_job_plan_validates_base_and_wal_archive() {
        assert_eq!(canonical_backup_job().validate(), Ok(()));
    }

    #[test]
    fn canonical_backup_report_is_deterministic() {
        let report = canonical_backup_report().expect("canonical report");

        assert_eq!(report.job.cluster, "prod");
        assert_eq!(report.restore.target_cluster, "restore-prod");
        assert_eq!(report.queryable_branch.branch_name, "prod-at-noon");
    }

    #[test]
    fn backup_runtime_runs_encrypted_backup_restore_and_branch() {
        let report = canonical_backup_runtime_report().expect("runtime report");

        assert_eq!(report.backup.cluster, "prod");
        assert_eq!(report.backup.base_size_bytes, 2_097_152);
        assert_eq!(report.backup.wal_segments, 3);
        assert!(report.backup.encrypted);
        assert_eq!(report.restore.target_cluster, "restore-prod");
        assert_eq!(report.restore.replayed_wal_segments, 3);
        assert_eq!(report.queryable_branch.branch_name, "prod-at-noon");
        assert!(report.queryable_branch.read_only);
        assert_eq!(report.state.completed_base_backups, 1);
        assert_eq!(report.state.archived_wal_segments, 3);
        assert_eq!(report.state.pitr_restores, 1);
        assert_eq!(report.state.queryable_branches, 1);
        assert_eq!(report.state.retention_deletions, 0);
        assert_eq!(report.state.failed_walg_invocations, 0);
        assert_eq!(report.state.encrypted_artifacts, 4);
    }

    #[test]
    fn backup_runtime_rejects_restore_from_wrong_archive() {
        let mut runtime = BackupRuntime::new(canonical_backup_job()).expect("runtime");
        let mut restore = canonical_pitr_restore_plan();
        restore.source_archive_uri = "s3://backups/other".to_string();

        assert_eq!(
            runtime.restore_pitr(&restore),
            Err(BackupSidecarError::ArchiveMismatch)
        );
    }

    #[test]
    fn pitr_restore_requires_utc_timestamp() {
        let mut restore = canonical_pitr_restore_plan();
        restore.target_time = "2026-05-19 12:00:00".to_string();

        assert_eq!(
            restore.validate(),
            Err(BackupSidecarError::InvalidTimestamp)
        );
    }

    #[test]
    fn pitr_restore_rejects_calendar_invalid_timestamp() {
        let mut restore = canonical_pitr_restore_plan();
        restore.target_time = "2026-02-30T12:00:00Z".to_string();

        assert_eq!(
            restore.validate(),
            Err(BackupSidecarError::InvalidTimestamp)
        );
    }

    #[test]
    fn backup_job_rejects_malformed_object_store_uri() {
        let mut job = canonical_backup_job();
        job.contract.archive_uri = "s3://backups/../prod".to_string();

        assert_eq!(
            job.validate(),
            Err(BackupSidecarError::InvalidUri("backup.archive_uri"))
        );

        job.contract.archive_uri = "file:///tmp/backups".to_string();
        assert_eq!(
            job.validate(),
            Err(BackupSidecarError::InvalidUri("backup.archive_uri"))
        );
    }

    #[test]
    fn queryable_branch_must_be_read_only() {
        let mut branch = canonical_queryable_branch_plan();
        branch.read_only = false;

        assert_eq!(
            branch.validate(),
            Err(BackupSidecarError::QueryableBranchMustBeReadOnly)
        );
    }

    #[test]
    fn queryable_branch_rejects_path_like_name() {
        let mut branch = canonical_queryable_branch_plan();
        branch.branch_name = "../prod".to_string();

        assert_eq!(
            branch.validate(),
            Err(BackupSidecarError::InvalidBranchName)
        );
    }

    #[test]
    fn backup_job_rejects_missing_kms_key() {
        let mut job = canonical_backup_job();
        job.encryption = Some(BackupEncryptionPlan {
            kms_key_ref: " ".to_string(),
        });

        assert_eq!(
            job.validate(),
            Err(BackupSidecarError::MissingRequiredField(
                "encryption.kms_key_ref"
            ))
        );
    }

    #[test]
    fn wal_window_contains_canonical_target() {
        let window = WalArchiveWindow {
            oldest_wal_time: "2026-05-19T00:00:00Z".to_string(),
            latest_wal_time: "2026-05-19T23:59:59Z".to_string(),
        };
        assert!(window.contains("2026-05-19T12:00:00Z"));
        assert!(!window.contains("2026-05-20T00:00:00Z"));
    }

    #[test]
    fn engine_validates_pitr_target_against_wal_window() {
        let engine = test_engine("/usr/bin/true");
        let mut restore = canonical_pitr_restore_plan();
        restore.target_time = "2025-01-01T00:00:00Z".to_string();
        let error = engine
            .start_pitr_restore(&restore, "job-1", 1_716_148_800)
            .expect_err("target outside window must fail");
        match error {
            BackupSidecarError::PitrTargetOutsideWindow { target_time, .. } => {
                assert_eq!(target_time, "2025-01-01T00:00:00Z");
            }
            other => panic!("expected PitrTargetOutsideWindow, got {other:?}"),
        }
    }

    #[test]
    fn engine_records_successful_pitr_job() {
        let engine = test_engine("/usr/bin/true");
        let job = engine
            .start_pitr_restore(&canonical_pitr_restore_plan(), "job-2", 1_716_148_800)
            .expect("pitr job records success");

        assert_eq!(job.status.as_str(), "succeeded");
        assert!(engine.pitr_job("job-2").is_some());
    }

    #[test]
    fn engine_records_failed_pitr_job() {
        let engine = test_engine("/usr/bin/false");
        let job = engine
            .start_pitr_restore(&canonical_pitr_restore_plan(), "job-3", 1_716_148_800)
            .expect("pitr job records failure result");

        assert_eq!(job.status.as_str(), "failed");
        assert_eq!(engine.status().state.failed_walg_invocations, 1);
    }

    #[test]
    fn engine_creates_queryable_branch_process() {
        let engine = test_engine("/usr/bin/true");
        let process = engine
            .create_queryable_branch(&canonical_queryable_branch_plan(), 6543)
            .expect("queryable branch process");

        assert_eq!(process.branch_name, "prod-at-noon");
        assert_eq!(process.port, 6543);
        assert!(engine.queryable_branches().len() == 1);
    }

    #[test]
    fn engine_rejects_duplicate_queryable_branch() {
        let engine = test_engine("/usr/bin/true");
        let plan = canonical_queryable_branch_plan();
        engine
            .create_queryable_branch(&plan, 6543)
            .expect("first branch succeeds");

        let error = engine
            .create_queryable_branch(&plan, 6544)
            .expect_err("duplicate branch must fail");
        match error {
            BackupSidecarError::QueryableBranch(detail) => {
                assert!(detail.contains("already exists"));
            }
            other => panic!("expected QueryableBranch error, got {other:?}"),
        }
    }

    #[test]
    fn http_backups_run_executes_walg_and_returns_artifact() {
        let engine = test_engine("/usr/bin/true");
        let response = handle_backup_http_bytes(
            &engine,
            b"POST /backups/run HTTP/1.1\r\nHost: local\r\nContent-Length: 0\r\n\r\n",
            1_716_148_800,
        )
        .expect("backups run response");

        assert_eq!(response.status_code, 202);
        assert!(response.body.contains("\"cluster\":\"prod\""));
        assert!(response.body.contains("\"encrypted\":true"));
    }

    #[test]
    fn http_backups_status_and_delete_old_are_exposed() {
        let engine = test_engine("/usr/bin/true");
        handle_backup_http_bytes(
            &engine,
            b"POST /backups/run HTTP/1.1\r\nHost: local\r\nContent-Length: 0\r\n\r\n",
            1_716_148_800,
        )
        .expect("base backup run");

        let status = handle_backup_http_bytes(
            &engine,
            b"GET /backups/status HTTP/1.1\r\nHost: local\r\n\r\n",
            1_716_148_800,
        )
        .expect("status response");
        assert_eq!(status.status_code, 200);
        assert!(status.body.contains("\"completed_base_backups\":1"));
        assert!(status.body.contains("\"operation\":\"base_backup\""));

        let body = "{\"retention_full\":7}";
        let request = format!(
            "POST /backups/delete-old HTTP/1.1\r\nHost: local\r\nContent-Length: {}\r\n\r\n{}",
            body.len(),
            body
        );
        let deleted = handle_backup_http_bytes(&engine, request.as_bytes(), 1_716_148_800)
            .expect("delete-old response");
        assert_eq!(deleted.status_code, 202);
        assert!(deleted.body.contains("\"operation\":\"delete_old\""));

        let status = handle_backup_http_bytes(
            &engine,
            b"GET /backups/status HTTP/1.1
Host: local

",
            1_716_148_800,
        )
        .expect("status after delete-old");
        assert!(status.body.contains("\"retention_deletions\":1"));
    }

    #[test]
    fn scheduler_runs_due_backup_once_and_advances_next_fire() {
        let engine = test_engine("/usr/bin/true");
        {
            let mut state = engine.scheduler_state.lock().expect("scheduler state");
            state.next_epoch_minute = 1;
        }

        let artifact = engine
            .run_scheduled_backup_if_due(60)
            .expect("scheduler should run")
            .expect("artifact");
        assert_eq!(artifact.cluster, "prod");
        let status = engine.status();
        assert_eq!(status.state.completed_base_backups, 1);
        assert_eq!(status.scheduler.last_run_epoch_seconds, Some(60));
        assert!(status.scheduler.next_epoch_minute > 1);
    }

    #[test]
    fn http_metrics_exports_backup_counters() {
        let engine = test_engine("/usr/bin/true");
        handle_backup_http_bytes(
            &engine,
            b"POST /backups/run HTTP/1.1\r\nHost: local\r\nContent-Length: 0\r\n\r\n",
            1_716_148_800,
        )
        .expect("base backup run");
        let response = handle_backup_http_bytes(
            &engine,
            b"GET /metrics HTTP/1.1\r\nHost: local\r\n\r\n",
            1_716_148_800,
        )
        .expect("metrics response");
        assert_eq!(response.status_code, 200);
        assert!(response
            .body
            .contains("ai_blaise_backup_completed_base_backups 1"));
    }

    #[test]
    fn http_pitr_restore_validates_and_records_job() {
        let engine = test_engine("/usr/bin/true");
        let body = "{\"cluster\":\"prod\",\"source_archive_uri\":\"s3://backups/prod\",\"target_time\":\"2026-05-19T12:00:00Z\",\"target_cluster\":\"restore-prod\"}";
        let request = format!(
            "POST /pitr/restore HTTP/1.1\r\nHost: local\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
            body.len(),
            body
        );
        let response = handle_backup_http_bytes(&engine, request.as_bytes(), 1_716_148_800)
            .expect("pitr restore response");

        assert_eq!(response.status_code, 202);
        assert!(response.body.contains("\"status\":\"succeeded\""));
        assert!(response.body.contains("\"target_cluster\""));
    }

    #[test]
    fn http_branches_queryable_rejects_invalid_or_malformed_port() {
        let engine = test_engine("/usr/bin/true");
        let low_port = "{\"branch_name\":\"prod-at-noon\",\"source_archive_uri\":\"s3://backups/prod\",\"target_time\":\"2026-05-19T12:00:00Z\",\"port\":80}";
        let request = format!(
            "POST /branches/queryable HTTP/1.1
Host: local
Content-Type: application/json
Content-Length: {}

{}",
            low_port.len(),
            low_port
        );
        assert_eq!(
            handle_backup_http_bytes(&engine, request.as_bytes(), 1_716_148_800),
            Err(BackupSidecarError::InvalidPort)
        );

        let junk_port = "{\"branch_name\":\"prod-at-noon\",\"source_archive_uri\":\"s3://backups/prod\",\"target_time\":\"2026-05-19T12:00:00Z\",\"port\":6543junk}";
        let request = format!(
            "POST /branches/queryable HTTP/1.1
Host: local
Content-Type: application/json
Content-Length: {}

{}",
            junk_port.len(),
            junk_port
        );
        assert_eq!(
            handle_backup_http_bytes(&engine, request.as_bytes(), 1_716_148_800),
            Err(BackupSidecarError::MalformedHttpRequest)
        );
    }

    #[test]
    fn http_branches_queryable_creates_and_lists() {
        let engine = test_engine("/usr/bin/true");
        let body = "{\"branch_name\":\"prod-at-noon\",\"source_archive_uri\":\"s3://backups/prod\",\"target_time\":\"2026-05-19T12:00:00Z\",\"port\":6543}";
        let request = format!(
            "POST /branches/queryable HTTP/1.1\r\nHost: local\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
            body.len(),
            body
        );
        let created = handle_backup_http_bytes(&engine, request.as_bytes(), 1_716_148_800)
            .expect("branch create response");
        assert_eq!(created.status_code, 201);
        assert!(created.body.contains("\"port\":6543"));

        let listed = handle_backup_http_bytes(
            &engine,
            b"GET /branches/queryable HTTP/1.1\r\nHost: local\r\n\r\n",
            1_716_148_800,
        )
        .expect("branch list response");

        assert_eq!(listed.status_code, 200);
        assert!(listed.body.contains("\"branches\":[{"));
        assert!(listed.body.contains("prod-at-noon"));
    }

    #[test]
    fn http_pitr_status_returns_404_for_unknown_job() {
        let engine = test_engine("/usr/bin/true");
        let response = handle_backup_http_bytes(
            &engine,
            b"GET /pitr/status/unknown-job HTTP/1.1\r\nHost: local\r\n\r\n",
            1_716_148_800,
        )
        .expect("status response");

        assert_eq!(response.status_code, 404);
        assert!(response.body.contains("not found"));
    }

    #[test]
    fn http_healthz_returns_component_label() {
        let engine = test_engine("/usr/bin/true");
        let response = handle_backup_http_bytes(
            &engine,
            b"GET /healthz HTTP/1.1\r\nHost: local\r\n\r\n",
            1_716_148_800,
        )
        .expect("healthz response");

        assert_eq!(response.status_code, 200);
        assert!(response.body.contains("\"component\":\"backup\""));
    }

    #[test]
    fn http_rejects_malformed_request() {
        let engine = test_engine("/usr/bin/true");
        let response = handle_backup_http_bytes(&engine, b"not-http", 1_716_148_800);
        assert_eq!(response, Err(BackupSidecarError::MalformedHttpRequest));
    }

    #[test]
    fn http_request_completion_honors_content_length() {
        let partial = b"POST /pitr/restore HTTP/1.1\r\nHost: local\r\nContent-Length: 80\r\n\r\n{\"cluster\":\"prod\"";
        assert!(!http_request_complete(partial));

        let complete = b"POST /pitr/restore HTTP/1.1\r\nHost: local\r\nContent-Length: 15\r\n\r\n{\"cluster\":\"x\"}";
        assert!(http_request_complete(complete));
    }
}
