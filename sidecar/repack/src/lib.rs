//! Repack sidecar contracts.

// FEATURE: R7

use ai_blaise_citus_sidecar_shared::{
    RepackContract, RepackExecutionStrategy, SidecarContractError,
};
use std::error::Error;
use std::fmt;

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
            evidence_boundary: "dry-run-plan-only".to_string(),
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
    InvalidShardId,
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
            Self::InvalidShardId => write!(formatter, "shard_id must be greater than zero"),
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
