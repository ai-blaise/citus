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
pub enum RepackSidecarError {
    InvalidIdentifier(&'static str),
    InvalidLockTimeout,
    InvalidShardId,
    MissingRequiredField(&'static str),
    SharedContract(String),
}

impl fmt::Display for RepackSidecarError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidIdentifier(field) => write!(formatter, "{field} must be a SQL identifier"),
            Self::InvalidLockTimeout => {
                write!(formatter, "lock_timeout_ms must be greater than zero")
            }
            Self::InvalidShardId => write!(formatter, "shard_id must be greater than zero"),
            Self::MissingRequiredField(field) => write!(formatter, "{field} must not be empty"),
            Self::SharedContract(error) => write!(formatter, "{error}"),
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
    fn shard_target_requires_qualified_table() {
        let mut job = valid_job();
        job.shard_targets[0].table = "orders_102008".to_string();

        assert_eq!(
            job.validate(),
            Err(RepackSidecarError::InvalidIdentifier("shard.table"))
        );
    }

    fn valid_job() -> RepackJobPlan {
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
}
