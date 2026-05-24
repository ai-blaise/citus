//! Repack sidecar contracts.

// FEATURE: R7

use ai_blaise_citus_sidecar_shared::{
    RepackContract, RepackExecutionStrategy, SidecarContractError,
};
use std::error::Error;
use std::fmt;
use std::process::Command;

pub const DRY_RUN_EVIDENCE_BOUNDARY: &str = "dry-run-plan-only";
pub const LIVE_PG_REPACK_EVIDENCE_BOUNDARY: &str = "live-pg-repack-execution";

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct RepackJobPlan {
    pub contract: RepackContract,
    pub schedule: String,
    pub lock_timeout_ms: u32,
    pub shard_targets: Vec<ShardRepackTarget>,
}

impl RepackJobPlan {
    pub fn validate(&self) -> Result<(), RepackSidecarError> {
        self.contract.validate()?;
        validate_qualified_name("repack.target", &self.contract.target)?;
        validate_required("schedule", &self.schedule)?;
        if self.lock_timeout_ms == 0 {
            return Err(RepackSidecarError::InvalidLockTimeout);
        }
        if self.shard_targets.is_empty() {
            return Err(RepackSidecarError::MissingRequiredField("shard_targets"));
        }
        for target in &self.shard_targets {
            target.validate()?;
        }
        Ok(())
    }

    pub fn select_strategy(
        &self,
        environment: &RepackRuntimeEnvironment,
    ) -> Result<RepackExecutionStrategy, RepackSidecarError> {
        self.validate()?;
        environment.validate()?;

        match self.contract.strategy {
            RepackExecutionStrategy::PgRepack if environment.pg_repack_available => {
                Ok(RepackExecutionStrategy::PgRepack)
            }
            RepackExecutionStrategy::PgRepack => Err(RepackSidecarError::MissingCapability(
                "pg_repack extension or binary",
            )),
            RepackExecutionStrategy::RepackConcurrentlyPg19
                if environment.pg_major >= 19 && environment.repack_concurrently_available =>
            {
                Ok(RepackExecutionStrategy::RepackConcurrentlyPg19)
            }
            RepackExecutionStrategy::RepackConcurrentlyPg19 => {
                Err(RepackSidecarError::UnsupportedStrategy(
                    "repack_concurrently_pg19 requires PostgreSQL 19+ and an explicit capability flag",
                ))
            }
        }
    }

    pub fn execution_report(
        &self,
        environment: RepackRuntimeEnvironment,
    ) -> Result<RepackExecutionReport, RepackSidecarError> {
        let selected_strategy = self.select_strategy(&environment)?;
        let mut selected_job = self.clone();
        selected_job.contract.strategy = selected_strategy;
        let command = selected_job.command_plan()?;

        Ok(RepackExecutionReport {
            job: selected_job,
            environment,
            selected_strategy,
            command,
            dry_run: true,
            executed: false,
            evidence_boundary: DRY_RUN_EVIDENCE_BOUNDARY.to_string(),
        })
    }

    pub fn command_plan(&self) -> Result<RepackCommandPlan, RepackSidecarError> {
        self.validate()?;
        let executable = match self.contract.strategy {
            RepackExecutionStrategy::PgRepack => "pg_repack",
            RepackExecutionStrategy::RepackConcurrentlyPg19 => "psql",
        };
        let args = match self.contract.strategy {
            RepackExecutionStrategy::PgRepack => vec![
                "--table".to_string(),
                self.contract.target.clone(),
                "--jobs".to_string(),
                self.contract.max_concurrency.to_string(),
            ],
            RepackExecutionStrategy::RepackConcurrentlyPg19 => vec![
                "-c".to_string(),
                format!("REPACK TABLE {} CONCURRENTLY", self.contract.target),
            ],
        };

        Ok(RepackCommandPlan {
            executable: executable.to_string(),
            args,
            lock_timeout_ms: self.lock_timeout_ms,
            shard_count: self.shard_targets.len() as u32,
        })
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ShardRepackTarget {
    pub shard_id: u64,
    pub worker: String,
    pub table: String,
}

impl ShardRepackTarget {
    fn validate(&self) -> Result<(), RepackSidecarError> {
        if self.shard_id == 0 {
            return Err(RepackSidecarError::InvalidShardId);
        }
        validate_required("shard.worker", &self.worker)?;
        validate_qualified_name("shard.table", &self.table)
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct RepackCommandPlan {
    pub executable: String,
    pub args: Vec<String>,
    pub lock_timeout_ms: u32,
    pub shard_count: u32,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct RepackLiveExecutionRequest {
    pub job: RepackJobPlan,
    pub environment: RepackRuntimeEnvironment,
    pub database_url: String,
    pub executable: String,
    pub wait_timeout_secs: u32,
}

impl RepackLiveExecutionRequest {
    pub fn validate(&self) -> Result<(), RepackSidecarError> {
        self.job.validate()?;
        self.environment.validate()?;
        validate_required("repack.database_url", &self.database_url)?;
        validate_required("repack.executable", &self.executable)?;
        if self.wait_timeout_secs == 0 {
            return Err(RepackSidecarError::InvalidWaitTimeout);
        }
        let selected_strategy = self.job.select_strategy(&self.environment)?;
        if selected_strategy != RepackExecutionStrategy::PgRepack {
            return Err(RepackSidecarError::UnsupportedStrategy(
                "live sidecar execution currently supports pg_repack only",
            ));
        }
        Ok(())
    }

    pub fn redacted_args(&self) -> Vec<String> {
        redact_command_args(&live_pg_repack_args(
            &self.job,
            &self.database_url,
            self.wait_timeout_secs,
        ))
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct RepackLiveExecutionReport {
    pub target: String,
    pub strategy: RepackExecutionStrategy,
    pub dry_run: bool,
    pub executed: bool,
    pub exit_code: i32,
    pub evidence_boundary: String,
    pub executable: String,
    pub redacted_args: Vec<String>,
    pub stdout: String,
    pub stderr: String,
}

pub fn execute_live_pg_repack(
    request: &RepackLiveExecutionRequest,
) -> Result<RepackLiveExecutionReport, RepackSidecarError> {
    request.validate()?;
    let args = live_pg_repack_args(
        &request.job,
        &request.database_url,
        request.wait_timeout_secs,
    );
    let output = Command::new(&request.executable)
        .args(&args)
        .output()
        .map_err(|error| RepackSidecarError::CommandSpawnFailed(error.to_string()))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    if !output.status.success() {
        return Err(RepackSidecarError::CommandExited {
            code: output.status.code(),
            stderr,
        });
    }

    Ok(RepackLiveExecutionReport {
        target: request.job.contract.target.clone(),
        strategy: RepackExecutionStrategy::PgRepack,
        dry_run: false,
        executed: true,
        exit_code: output.status.code().unwrap_or(0),
        evidence_boundary: LIVE_PG_REPACK_EVIDENCE_BOUNDARY.to_string(),
        executable: request.executable.clone(),
        redacted_args: redact_command_args(&args),
        stdout,
        stderr,
    })
}

fn live_pg_repack_args(
    job: &RepackJobPlan,
    database_url: &str,
    wait_timeout_secs: u32,
) -> Vec<String> {
    vec![
        "--dbname".to_string(),
        database_url.to_string(),
        "--table".to_string(),
        job.contract.target.clone(),
        "--jobs".to_string(),
        job.contract.max_concurrency.to_string(),
        "--wait-timeout".to_string(),
        wait_timeout_secs.to_string(),
        "--no-superuser-check".to_string(),
    ]
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct RepackRuntimeEnvironment {
    pub pg_major: u16,
    pub pg_repack_available: bool,
    pub repack_concurrently_available: bool,
}

impl RepackRuntimeEnvironment {
    fn validate(&self) -> Result<(), RepackSidecarError> {
        if self.pg_major == 0 {
            return Err(RepackSidecarError::InvalidPostgresMajor);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct RepackExecutionReport {
    pub job: RepackJobPlan,
    pub environment: RepackRuntimeEnvironment,
    pub selected_strategy: RepackExecutionStrategy,
    pub command: RepackCommandPlan,
    pub dry_run: bool,
    pub executed: bool,
    pub evidence_boundary: String,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum RepackSidecarError {
    InvalidIdentifier(&'static str),
    InvalidLockTimeout,
    InvalidPostgresMajor,
    InvalidWaitTimeout,
    InvalidShardId,
    CommandExited { code: Option<i32>, stderr: String },
    CommandSpawnFailed(String),
    MissingCapability(&'static str),
    MissingRequiredField(&'static str),
    SharedContract(String),
    UnsupportedStrategy(&'static str),
}

impl fmt::Display for RepackSidecarError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidIdentifier(field) => write!(formatter, "{field} must be a SQL identifier"),
            Self::InvalidLockTimeout => {
                write!(formatter, "lock_timeout_ms must be greater than zero")
            }
            Self::InvalidPostgresMajor => {
                write!(formatter, "pg_major must be greater than zero")
            }
            Self::InvalidWaitTimeout => {
                write!(formatter, "wait_timeout_secs must be greater than zero")
            }
            Self::InvalidShardId => write!(formatter, "shard_id must be greater than zero"),
            Self::CommandExited { code, stderr } => write!(
                formatter,
                "pg_repack exited with status {:?}: {}",
                code, stderr
            ),
            Self::CommandSpawnFailed(error) => write!(formatter, "pg_repack spawn failed: {error}"),
            Self::MissingCapability(capability) => write!(
                formatter,
                "required repack capability is unavailable: {capability}"
            ),
            Self::MissingRequiredField(field) => write!(formatter, "{field} must not be empty"),
            Self::SharedContract(error) => write!(formatter, "{error}"),
            Self::UnsupportedStrategy(reason) => {
                write!(formatter, "unsupported strategy: {reason}")
            }
        }
    }
}

impl Error for RepackSidecarError {}

impl From<SidecarContractError> for RepackSidecarError {
    fn from(error: SidecarContractError) -> Self {
        Self::SharedContract(error.to_string())
    }
}

fn validate_required(field: &'static str, value: &str) -> Result<(), RepackSidecarError> {
    if value.trim().is_empty() {
        return Err(RepackSidecarError::MissingRequiredField(field));
    }
    Ok(())
}

fn validate_identifier(field: &'static str, value: &str) -> Result<(), RepackSidecarError> {
    validate_required(field, value)?;
    if value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
    {
        Ok(())
    } else {
        Err(RepackSidecarError::InvalidIdentifier(field))
    }
}

fn validate_qualified_name(field: &'static str, value: &str) -> Result<(), RepackSidecarError> {
    validate_required(field, value)?;
    let parts: Vec<_> = value.split('.').collect();
    if parts.len() == 2
        && parts
            .iter()
            .all(|part| validate_identifier(field, part).is_ok())
    {
        Ok(())
    } else {
        Err(RepackSidecarError::InvalidIdentifier(field))
    }
}

fn redact_command_args(args: &[String]) -> Vec<String> {
    let mut redacted = Vec::with_capacity(args.len());
    let mut redact_next = false;
    for arg in args {
        if redact_next {
            redacted.push(redact_database_url(arg));
            redact_next = false;
        } else if arg == "--dbname" {
            redacted.push(arg.clone());
            redact_next = true;
        } else if let Some(value) = arg.strip_prefix("--dbname=") {
            redacted.push(format!("--dbname={}", redact_database_url(value)));
        } else {
            redacted.push(arg.clone());
        }
    }
    redacted
}

fn redact_database_url(value: &str) -> String {
    if let Some(scheme_end) = value.find("://") {
        let auth_start = scheme_end + 3;
        if let Some(at_offset) = value[auth_start..].find('@') {
            let auth_end = auth_start + at_offset;
            if let Some(password_start) = value[auth_start..auth_end].rfind(':') {
                let user = &value[auth_start..auth_start + password_start];
                return format!("{}{}:***{}", &value[..auth_start], user, &value[auth_end..]);
            }
        }
    }
    value.to_string()
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct RepackCanonicalReport {
    pub job: RepackJobPlan,
    pub command: RepackCommandPlan,
    pub environment: RepackRuntimeEnvironment,
    pub execution: RepackExecutionReport,
}

pub fn canonical_repack_job() -> RepackJobPlan {
    RepackJobPlan {
        contract: RepackContract {
            target: "public.orders".to_string(),
            strategy: RepackExecutionStrategy::PgRepack,
            max_concurrency: 2,
        },
        schedule: "0 3 * * 0".to_string(),
        lock_timeout_ms: 500,
        shard_targets: vec![
            ShardRepackTarget {
                shard_id: 102_008,
                worker: "worker-a".to_string(),
                table: "public.orders_102008".to_string(),
            },
            ShardRepackTarget {
                shard_id: 102_009,
                worker: "worker-b".to_string(),
                table: "public.orders_102009".to_string(),
            },
        ],
    }
}

pub fn canonical_repack_environment() -> RepackRuntimeEnvironment {
    RepackRuntimeEnvironment {
        pg_major: 18,
        pg_repack_available: true,
        repack_concurrently_available: false,
    }
}

pub fn canonical_repack_report() -> Result<RepackCanonicalReport, RepackSidecarError> {
    let job = canonical_repack_job();
    let environment = canonical_repack_environment();
    let execution = job.execution_report(environment)?;
    let command = execution.command.clone();

    Ok(RepackCanonicalReport {
        job,
        command,
        environment,
        execution,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pg_repack_job_renders_command_plan() {
        let command = valid_job()
            .command_plan()
            .expect("command plan should render");

        assert_eq!(command.executable, "pg_repack");
        assert_eq!(command.shard_count, 2);
        assert!(command.args.contains(&"--jobs".to_string()));
    }

    #[test]
    fn pg19_strategy_renders_repack_concurrently_command() {
        let mut job = valid_job();
        job.contract.strategy = RepackExecutionStrategy::RepackConcurrentlyPg19;

        let command = job.command_plan().expect("command plan");

        assert_eq!(command.executable, "psql");
        assert_eq!(
            command.args,
            vec![
                "-c".to_string(),
                "REPACK TABLE public.orders CONCURRENTLY".to_string(),
            ]
        );
    }

    #[test]
    fn missing_pg_repack_capability_fails_closed() {
        let job = valid_job();
        let environment = RepackRuntimeEnvironment {
            pg_major: 18,
            pg_repack_available: false,
            repack_concurrently_available: false,
        };

        assert_eq!(
            job.select_strategy(&environment),
            Err(RepackSidecarError::MissingCapability(
                "pg_repack extension or binary"
            ))
        );
    }

    #[test]
    fn pg19_strategy_requires_pg19_capability() {
        let mut job = valid_job();
        job.contract.strategy = RepackExecutionStrategy::RepackConcurrentlyPg19;
        let environment = RepackRuntimeEnvironment {
            pg_major: 18,
            pg_repack_available: true,
            repack_concurrently_available: true,
        };

        assert_eq!(
            job.select_strategy(&environment),
            Err(RepackSidecarError::UnsupportedStrategy(
                "repack_concurrently_pg19 requires PostgreSQL 19+ and an explicit capability flag"
            ))
        );
    }

    #[test]
    fn pg19_strategy_can_be_selected_when_capability_is_declared() {
        let mut job = valid_job();
        job.contract.strategy = RepackExecutionStrategy::RepackConcurrentlyPg19;
        let environment = RepackRuntimeEnvironment {
            pg_major: 19,
            pg_repack_available: false,
            repack_concurrently_available: true,
        };

        assert_eq!(
            job.select_strategy(&environment),
            Ok(RepackExecutionStrategy::RepackConcurrentlyPg19)
        );
    }

    #[test]
    fn execution_report_is_dry_run_and_non_executing() {
        let report = valid_job()
            .execution_report(canonical_repack_environment())
            .expect("execution report");

        assert!(report.dry_run);
        assert!(!report.executed);
        assert_eq!(report.evidence_boundary, "dry-run-plan-only");
        assert_eq!(report.command.executable, "pg_repack");
    }

    #[test]
    fn live_pg_repack_request_requires_database_url() {
        let request = RepackLiveExecutionRequest {
            job: valid_job(),
            environment: canonical_repack_environment(),
            database_url: String::new(),
            executable: "pg_repack".to_string(),
            wait_timeout_secs: 5,
        };

        assert_eq!(
            request.validate(),
            Err(RepackSidecarError::MissingRequiredField(
                "repack.database_url"
            ))
        );
    }

    #[test]
    fn live_pg_repack_request_renders_redacted_args() {
        let request = RepackLiveExecutionRequest {
            job: valid_job(),
            environment: canonical_repack_environment(),
            database_url: "postgresql://alice:secret@db.example/postgres".to_string(),
            executable: "pg_repack".to_string(),
            wait_timeout_secs: 5,
        };

        assert_eq!(
            live_pg_repack_args(
                &request.job,
                &request.database_url,
                request.wait_timeout_secs
            ),
            vec![
                "--dbname".to_string(),
                "postgresql://alice:secret@db.example/postgres".to_string(),
                "--table".to_string(),
                "public.orders".to_string(),
                "--jobs".to_string(),
                "2".to_string(),
                "--wait-timeout".to_string(),
                "5".to_string(),
                "--no-superuser-check".to_string(),
            ]
        );
        assert_eq!(
            request.redacted_args()[1],
            "postgresql://alice:***@db.example/postgres"
        );
    }

    #[test]
    fn shard_target_requires_qualified_table() {
        let mut job = valid_job();
        job.shard_targets[0].table = "orders_102008".to_string();

        assert_eq!(
            job.validate(),
            Err(RepackSidecarError::InvalidIdentifier("shard.table"))
        );
    }

    #[test]
    fn canonical_report_is_deterministic() {
        let report = canonical_repack_report().expect("canonical report");

        assert_eq!(report.job.contract.target, "public.orders");
        assert_eq!(report.command.executable, "pg_repack");
        assert_eq!(report.command.shard_count, 2);
        assert_eq!(report.environment.pg_major, 18);
        assert!(!report.execution.executed);
    }

    fn valid_job() -> RepackJobPlan {
        canonical_repack_job()
    }
}
