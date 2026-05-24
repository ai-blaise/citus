//! WAL-G CLI orchestration.

// FEATURE: B1
// FEATURE: B3
// FEATURE: B6

use std::collections::HashMap;
use std::error::Error;
use std::ffi::OsString;
use std::fmt;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use crate::BackupJobPlan;

/// Orchestrates the `wal-g` command-line interface for base backups,
/// WAL archival, PITR restore, and retention.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct WalgRunner {
    binary: PathBuf,
    base_env: HashMap<OsString, OsString>,
}

impl WalgRunner {
    /// Build a runner from a backup job plan, materializing the WAL-G
    /// environment from the validated archive URI, retention, concurrency,
    /// compression, and KMS settings.
    pub fn from_plan(binary: impl Into<PathBuf>, plan: &BackupJobPlan) -> Self {
        let mut runner = Self::new(binary);
        runner.set_env("WALG_LOG_LEVEL", "INFO");
        for (key, value) in walg_env_from_plan(plan) {
            runner.set_env(key, value);
        }
        runner
    }

    /// Construct a runner with no environment beyond the WAL-G binary path.
    pub fn new(binary: impl Into<PathBuf>) -> Self {
        Self {
            binary: binary.into(),
            base_env: HashMap::new(),
        }
    }

    /// Set or override a single environment variable that is propagated to
    /// every spawned `wal-g` invocation.
    pub fn set_env(&mut self, key: impl Into<OsString>, value: impl Into<OsString>) {
        self.base_env.insert(key.into(), value.into());
    }

    pub fn binary(&self) -> &Path {
        &self.binary
    }

    pub fn env(&self) -> &HashMap<OsString, OsString> {
        &self.base_env
    }

    /// Validate encryption-related WAL-G environment before accepting work.
    pub fn validate_encryption_env(&self, encryption_required: bool) -> Result<(), WalgError> {
        if !encryption_required {
            return Ok(());
        }
        let accepted_keys = [
            "WALG_GPG_KEY_ID",
            "WALG_PGP_KEY",
            "WALG_PGP_KEY_PATH",
            "WALG_LIBSODIUM_KEY",
            "WALG_LIBSODIUM_KEY_PATH",
        ];
        for key in accepted_keys {
            if let Some(value) = self.base_env.get(&OsString::from(key)) {
                if !value.is_empty() {
                    return Ok(());
                }
            }
        }
        Err(WalgError::MissingEncryptionEnv)
    }

    /// Build a `wal-g backup-push` command for the given pgdata directory.
    pub fn base_backup_command(&self, pgdata: &Path) -> Command {
        let mut command = self.base_command();
        command.arg("backup-push");
        command.arg(pgdata);
        command
    }

    /// Build a `wal-g wal-show` command to summarize WAL archive state.
    pub fn wal_archive_status_command(&self) -> Command {
        let mut command = self.base_command();
        command.arg("wal-show");
        command.arg("--detailed-json");
        command
    }

    /// Build a `wal-g backup-fetch` command targeting a directory and the
    /// recovery target time (RFC3339).
    pub fn pitr_restore_command(&self, target_dir: &Path, target_time: &str) -> Command {
        let mut command = self.base_command();
        command.arg("backup-fetch");
        command.arg(target_dir);
        command.arg("LATEST");
        command.arg("--target-time");
        command.arg(target_time);
        command
    }

    /// Build a `wal-g delete retain` command honoring the configured retention.
    pub fn delete_old_command(&self, retention_full: u32) -> Command {
        let argument = format!("{retention_full}");
        let mut command = self.base_command();
        command.arg("delete");
        command.arg("retain");
        command.arg("FULL");
        command.arg(argument);
        command.arg("--confirm");
        command
    }

    /// Build a `wal-g backup-list` command for `/backups` HTTP listing.
    pub fn backup_list_command(&self) -> Command {
        let mut command = self.base_command();
        command.arg("backup-list");
        command.arg("--detail");
        command.arg("--json");
        command
    }

    /// Execute a previously built command, returning the captured output and
    /// the duration in milliseconds. Any non-zero exit translates into
    /// [`WalgError::NonZeroExit`].
    pub fn run(&self, mut command: Command) -> Result<WalgInvocation, WalgError> {
        let start = std::time::Instant::now();
        command.stdin(Stdio::null());
        command.stdout(Stdio::piped());
        command.stderr(Stdio::piped());
        let output: Output = command
            .output()
            .map_err(|error| WalgError::Spawn(error.to_string()))?;
        let elapsed = start.elapsed();
        let invocation = WalgInvocation {
            status_code: output.status.code(),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            elapsed_ms: u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX),
        };
        if !output.status.success() {
            return Err(WalgError::NonZeroExit(invocation));
        }
        Ok(invocation)
    }

    /// Trigger a base backup via `wal-g backup-push`.
    pub fn base_backup(&self, pgdata: &Path) -> Result<WalgInvocation, WalgError> {
        let command = self.base_backup_command(pgdata);
        self.run(command)
    }

    /// Summarize WAL archive state via `wal-g wal-show`.
    pub fn wal_archive_status(&self) -> Result<WalgInvocation, WalgError> {
        let command = self.wal_archive_status_command();
        self.run(command)
    }

    /// Run a PITR restore via `wal-g backup-fetch`.
    pub fn pitr_restore(
        &self,
        target_dir: &Path,
        target_time: &str,
    ) -> Result<WalgInvocation, WalgError> {
        let command = self.pitr_restore_command(target_dir, target_time);
        self.run(command)
    }

    /// Prune retained base backups via `wal-g delete retain FULL`.
    pub fn delete_old(&self, retention_full: u32) -> Result<WalgInvocation, WalgError> {
        if retention_full == 0 {
            return Err(WalgError::InvalidRetention);
        }
        let command = self.delete_old_command(retention_full);
        self.run(command)
    }

    /// List base backups via `wal-g backup-list --json`.
    pub fn backup_list(&self) -> Result<WalgInvocation, WalgError> {
        let command = self.backup_list_command();
        self.run(command)
    }

    fn base_command(&self) -> Command {
        let mut command = Command::new(&self.binary);
        for (key, value) in &self.base_env {
            command.env(key, value);
        }
        command
    }
}

/// Render the deterministic WAL-G environment map for a backup job plan.
pub fn walg_env_from_plan(plan: &BackupJobPlan) -> Vec<(&'static str, String)> {
    let mut env = Vec::with_capacity(8);
    env.push((
        walg_provider_prefix_key(&plan.contract.archive_uri),
        plan.contract.archive_uri.clone(),
    ));
    env.push((
        "WALG_FILE_PREFIX_OVERRIDE",
        plan.base_backup.destination_uri.clone(),
    ));
    env.push((
        "WALG_WAL_PREFIX_OVERRIDE",
        plan.wal_archive.archive_uri.clone(),
    ));
    env.push((
        "WALG_COMPRESSION_METHOD",
        match plan.wal_archive.compression {
            crate::WalCompression::None => "none".to_string(),
            crate::WalCompression::Gzip => "gzip".to_string(),
            crate::WalCompression::Zstd => "zstd".to_string(),
        },
    ));
    env.push((
        "WALG_UPLOAD_CONCURRENCY",
        plan.base_backup.concurrency.to_string(),
    ));
    env.push((
        "WALG_RETENTION_FULL",
        plan.base_backup.retention_days.to_string(),
    ));
    if let Some(encryption) = &plan.encryption {
        env.push(("WALG_GPG_KEY_ID", encryption.kms_key_ref.clone()));
    }
    env
}

fn walg_provider_prefix_key(uri: &str) -> &'static str {
    if uri.starts_with("gs://") {
        "WALG_GS_PREFIX"
    } else if uri.starts_with("az://") {
        "WALG_AZ_PREFIX"
    } else {
        "WALG_S3_PREFIX"
    }
}

/// Captured output of a successful (or failed) WAL-G invocation.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct WalgInvocation {
    pub status_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub elapsed_ms: u64,
}

/// Errors raised when orchestrating the `wal-g` CLI.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum WalgError {
    InvalidRetention,
    MissingEncryptionEnv,
    NonZeroExit(WalgInvocation),
    Spawn(String),
}

impl fmt::Display for WalgError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRetention => {
                write!(formatter, "retain count must be greater than zero")
            }
            Self::MissingEncryptionEnv => write!(
                formatter,
                "encrypted backups require WALG_GPG_KEY_ID, WALG_PGP_KEY, WALG_PGP_KEY_PATH, WALG_LIBSODIUM_KEY, or WALG_LIBSODIUM_KEY_PATH"
            ),
            Self::NonZeroExit(invocation) => write!(
                formatter,
                "wal-g exited with status {} after {} ms: {}",
                invocation
                    .status_code
                    .map(|code| code.to_string())
                    .unwrap_or_else(|| "signal".to_string()),
                invocation.elapsed_ms,
                invocation.stderr.trim()
            ),
            Self::Spawn(error) => write!(formatter, "failed to spawn wal-g: {error}"),
        }
    }
}

impl Error for WalgError {}

impl From<io::Error> for WalgError {
    fn from(error: io::Error) -> Self {
        Self::Spawn(error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canonical_backup_job;

    #[test]
    fn runner_from_plan_renders_walg_env_from_archive_uri_and_retention() {
        let plan = canonical_backup_job();
        let runner = WalgRunner::from_plan("/usr/bin/wal-g", &plan);

        let env = runner.env();
        assert_eq!(
            env.get(&OsString::from("WALG_S3_PREFIX")),
            Some(&OsString::from("s3://backups/prod"))
        );
        assert_eq!(
            env.get(&OsString::from("WALG_FILE_PREFIX_OVERRIDE")),
            Some(&OsString::from("s3://backups/prod/base"))
        );
        assert_eq!(
            env.get(&OsString::from("WALG_WAL_PREFIX_OVERRIDE")),
            Some(&OsString::from("s3://backups/prod/wal"))
        );
        assert_eq!(
            env.get(&OsString::from("WALG_COMPRESSION_METHOD")),
            Some(&OsString::from("zstd"))
        );
        assert_eq!(
            env.get(&OsString::from("WALG_UPLOAD_CONCURRENCY")),
            Some(&OsString::from("2"))
        );
        assert_eq!(
            env.get(&OsString::from("WALG_RETENTION_FULL")),
            Some(&OsString::from("30"))
        );
        assert_eq!(
            env.get(&OsString::from("WALG_GPG_KEY_ID")),
            Some(&OsString::from("aws-kms-prod"))
        );
        assert_eq!(runner.validate_encryption_env(true), Ok(()));
    }

    #[test]
    fn encrypted_runner_requires_encryption_env() {
        let runner = WalgRunner::new("/usr/bin/wal-g");
        assert_eq!(
            runner.validate_encryption_env(true),
            Err(WalgError::MissingEncryptionEnv)
        );
        assert_eq!(runner.validate_encryption_env(false), Ok(()));
    }

    #[test]
    fn runner_uses_provider_specific_walg_prefix_env() {
        let mut plan = canonical_backup_job();
        plan.contract.archive_uri = "gs://backups/prod".to_string();
        let gcs = WalgRunner::from_plan("/usr/bin/wal-g", &plan);
        assert_eq!(
            gcs.env().get(&OsString::from("WALG_GS_PREFIX")),
            Some(&OsString::from("gs://backups/prod"))
        );
        assert_eq!(gcs.env().get(&OsString::from("WALG_S3_PREFIX")), None);

        plan.contract.archive_uri = "az://backups/prod".to_string();
        let azure = WalgRunner::from_plan("/usr/bin/wal-g", &plan);
        assert_eq!(
            azure.env().get(&OsString::from("WALG_AZ_PREFIX")),
            Some(&OsString::from("az://backups/prod"))
        );
    }

    #[test]
    fn base_backup_command_targets_pgdata_directory() {
        let runner = WalgRunner::new("/usr/bin/wal-g");
        let command = runner.base_backup_command(Path::new("/var/lib/postgresql/data"));
        let program = command.get_program().to_string_lossy().into_owned();
        let args: Vec<String> = command
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect();

        assert_eq!(program, "/usr/bin/wal-g");
        assert_eq!(args, vec!["backup-push", "/var/lib/postgresql/data"]);
    }

    #[test]
    fn pitr_restore_command_passes_target_time() {
        let runner = WalgRunner::new("/usr/bin/wal-g");
        let command =
            runner.pitr_restore_command(Path::new("/tmp/restore"), "2026-05-19T12:00:00Z");
        let args: Vec<String> = command
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect();

        assert_eq!(
            args,
            vec![
                "backup-fetch",
                "/tmp/restore",
                "LATEST",
                "--target-time",
                "2026-05-19T12:00:00Z"
            ]
        );
    }

    #[test]
    fn delete_old_rejects_zero_retention() {
        let runner = WalgRunner::new("/usr/bin/wal-g");
        assert_eq!(runner.delete_old(0), Err(WalgError::InvalidRetention));
    }

    #[test]
    fn delete_old_command_renders_retain_argument() {
        let runner = WalgRunner::new("/usr/bin/wal-g");
        let command = runner.delete_old_command(7);
        let args: Vec<String> = command
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect();

        assert_eq!(args, vec!["delete", "retain", "FULL", "7", "--confirm"]);
    }

    #[test]
    fn run_succeeds_for_zero_exit_command() {
        let runner = WalgRunner::new("/usr/bin/true");
        let command = Command::new(runner.binary());
        let invocation = runner.run(command).expect("zero-exit invocation");

        assert_eq!(invocation.status_code, Some(0));
    }

    #[test]
    fn run_propagates_non_zero_exit() {
        let runner = WalgRunner::new("/usr/bin/false");
        let command = Command::new(runner.binary());
        match runner.run(command) {
            Err(WalgError::NonZeroExit(invocation)) => {
                assert_eq!(invocation.status_code, Some(1));
            }
            other => panic!("expected non-zero exit, got {other:?}"),
        }
    }
}
