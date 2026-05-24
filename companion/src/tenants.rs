// FEATURE: S14
// FEATURE: TO3
// FEATURE: TO4
// FEATURE: TO5

use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TenantMovePlan {
    pub tenant_name: String,
    pub source_worker: String,
    pub target_worker: String,
    pub region_affinity: Option<String>,
}

impl TenantMovePlan {
    pub fn validate(&self) -> Result<(), TenantOperationError> {
        validate_required("tenant_name", &self.tenant_name)?;
        validate_required("source_worker", &self.source_worker)?;
        validate_required("target_worker", &self.target_worker)?;
        if self.source_worker.trim() == self.target_worker.trim() {
            return Err(TenantOperationError::SameWorkerMove);
        }
        validate_optional("region_affinity", &self.region_affinity)
    }

    pub fn to_sql_plan(&self) -> Result<TenantSqlPlan, TenantOperationError> {
        self.validate()?;
        let region = match &self.region_affinity {
            Some(region) => sql_literal(region),
            None => "NULL".to_string(),
        };

        TenantSqlPlan::new(
            "S14",
            vec![
                format!(
                    "SELECT companion_internal.plan_tenant_move({}, {}, {}, {});",
                    sql_literal(&self.tenant_name),
                    sql_literal(&self.source_worker),
                    sql_literal(&self.target_worker),
                    region
                ),
                format!(
                    "SELECT move_id, status FROM companion_tenant_moves WHERE tenant_name = {};",
                    sql_literal(&self.tenant_name)
                ),
            ],
        )
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TenantQuotaPlan {
    pub tenant_name: String,
    pub max_connections: u32,
    pub max_qps: u32,
}

impl TenantQuotaPlan {
    pub fn validate(&self) -> Result<(), TenantOperationError> {
        validate_required("tenant_name", &self.tenant_name)?;
        if self.max_connections == 0 {
            return Err(TenantOperationError::InvalidQuota("max_connections"));
        }
        if self.max_qps == 0 {
            return Err(TenantOperationError::InvalidQuota("max_qps"));
        }
        Ok(())
    }

    pub fn to_sql_plan(&self) -> Result<TenantSqlPlan, TenantOperationError> {
        self.validate()?;
        TenantSqlPlan::new(
            "TO5",
            vec![format!(
                "SELECT companion_internal.set_tenant_quota({}, {}, {});",
                sql_literal(&self.tenant_name),
                self.max_connections,
                self.max_qps
            )],
        )
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TenantArchivePlan {
    pub tenant_name: String,
    pub destination_uri: String,
    pub retention_days: u32,
}

impl TenantArchivePlan {
    pub fn validate(&self) -> Result<(), TenantOperationError> {
        validate_required("tenant_name", &self.tenant_name)?;
        validate_required("destination_uri", &self.destination_uri)?;
        if self.retention_days == 0 {
            return Err(TenantOperationError::InvalidRetention);
        }
        Ok(())
    }

    pub fn to_sql_plan(&self) -> Result<TenantSqlPlan, TenantOperationError> {
        self.validate()?;
        TenantSqlPlan::new(
            "TO4",
            vec![format!(
                "SELECT companion_internal.plan_tenant_archive({}, {}, {});",
                sql_literal(&self.tenant_name),
                sql_literal(&self.destination_uri),
                self.retention_days
            )],
        )
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TenantRegionAffinityPlan {
    pub tenant_name: String,
    pub region_affinity: String,
}

impl TenantRegionAffinityPlan {
    pub fn validate(&self) -> Result<(), TenantOperationError> {
        validate_required("tenant_name", &self.tenant_name)?;
        validate_required("region_affinity", &self.region_affinity)
    }

    pub fn to_sql_plan(&self) -> Result<TenantSqlPlan, TenantOperationError> {
        self.validate()?;
        TenantSqlPlan::new(
            "TO5",
            vec![format!(
                "SELECT companion_internal.set_tenant_region_affinity({}, {});",
                sql_literal(&self.tenant_name),
                sql_literal(&self.region_affinity)
            )],
        )
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TenantSqlPlan {
    pub feature_id: &'static str,
    pub commands: Vec<String>,
}

impl TenantSqlPlan {
    fn new(feature_id: &'static str, commands: Vec<String>) -> Result<Self, TenantOperationError> {
        if commands.is_empty() || commands.iter().any(|command| command.trim().is_empty()) {
            return Err(TenantOperationError::MissingRequiredField("commands"));
        }
        Ok(Self {
            feature_id,
            commands,
        })
    }

    pub fn script(&self) -> String {
        self.commands.join("\n")
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum TenantOperationError {
    InvalidQuota(&'static str),
    InvalidRetention,
    SameWorkerMove,
    MissingRequiredField(&'static str),
}

impl fmt::Display for TenantOperationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidQuota(field) => write!(formatter, "{field} must be greater than zero"),
            Self::InvalidRetention => {
                write!(formatter, "retention_days must be greater than zero")
            }
            Self::SameWorkerMove => {
                write!(formatter, "source_worker and target_worker must differ")
            }
            Self::MissingRequiredField(field) => {
                write!(formatter, "{field} must not be empty")
            }
        }
    }
}

impl Error for TenantOperationError {}

fn validate_required(field: &'static str, value: &str) -> Result<(), TenantOperationError> {
    if value.trim().is_empty() {
        return Err(TenantOperationError::MissingRequiredField(field));
    }
    Ok(())
}

fn validate_optional(
    field: &'static str,
    value: &Option<String>,
) -> Result<(), TenantOperationError> {
    if matches!(value, Some(value) if value.trim().is_empty()) {
        return Err(TenantOperationError::MissingRequiredField(field));
    }
    Ok(())
}

fn sql_literal(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_tenant_move_passes() {
        let plan = TenantMovePlan {
            tenant_name: "tenant-a".to_string(),
            source_worker: "worker-1".to_string(),
            target_worker: "worker-2".to_string(),
            region_affinity: Some("us-east-1".to_string()),
        };

        assert_eq!(plan.validate(), Ok(()));
    }

    #[test]
    fn tenant_move_rejects_same_worker() {
        let plan = TenantMovePlan {
            tenant_name: "tenant-a".to_string(),
            source_worker: "worker-1".to_string(),
            target_worker: "worker-1".to_string(),
            region_affinity: None,
        };

        assert_eq!(plan.validate(), Err(TenantOperationError::SameWorkerMove));
    }

    #[test]
    fn tenant_move_sql_plan_contains_action_and_readback() {
        let plan = TenantMovePlan {
            tenant_name: "tenant-a".to_string(),
            source_worker: "worker-1".to_string(),
            target_worker: "worker-2".to_string(),
            region_affinity: Some("us-east-1".to_string()),
        }
        .to_sql_plan()
        .unwrap();

        assert_eq!(plan.feature_id, "S14");
        assert_eq!(plan.commands.len(), 2);
        assert!(plan.script().contains("plan_tenant_move"));
        assert!(plan.script().contains("companion_tenant_moves"));
    }

    #[test]
    fn tenant_move_sql_plan_uses_null_region_when_absent() {
        let plan = TenantMovePlan {
            tenant_name: "tenant-a".to_string(),
            source_worker: "worker-1".to_string(),
            target_worker: "worker-2".to_string(),
            region_affinity: None,
        }
        .to_sql_plan()
        .unwrap();

        assert!(plan.commands[0].ends_with("'worker-2', NULL);"));
    }

    #[test]
    fn tenant_quota_sql_plan_materializes_limits() {
        let plan = TenantQuotaPlan {
            tenant_name: "tenant-a".to_string(),
            max_connections: 100,
            max_qps: 1_000,
        }
        .to_sql_plan()
        .unwrap();

        assert_eq!(plan.feature_id, "TO5");
        assert!(plan.script().contains("set_tenant_quota"));
    }

    #[test]
    fn tenant_archive_sql_plan_materializes_archive_request() {
        let plan = TenantArchivePlan {
            tenant_name: "tenant-a".to_string(),
            destination_uri: "s3://archives/tenant-a".to_string(),
            retention_days: 30,
        }
        .to_sql_plan()
        .unwrap();

        assert_eq!(plan.feature_id, "TO4");
        assert!(plan.script().contains("plan_tenant_archive"));
    }

    #[test]
    fn tenant_region_affinity_sql_plan_rejects_empty_region() {
        let err = TenantRegionAffinityPlan {
            tenant_name: "tenant-a".to_string(),
            region_affinity: " ".to_string(),
        }
        .to_sql_plan()
        .unwrap_err();

        assert!(matches!(
            err,
            TenantOperationError::MissingRequiredField("region_affinity")
        ));
    }

    #[test]
    fn tenant_archive_requires_retention() {
        let plan = TenantArchivePlan {
            tenant_name: "tenant-a".to_string(),
            destination_uri: "s3://archives/tenant-a".to_string(),
            retention_days: 0,
        };

        assert_eq!(plan.validate(), Err(TenantOperationError::InvalidRetention));
    }
}
