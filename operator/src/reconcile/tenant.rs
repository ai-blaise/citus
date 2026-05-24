// FEATURE: TO1
// FEATURE: TO2
// FEATURE: TO5

use ai_blaise_citus_companion::{TenantArchivePlan, TenantOperationError, TenantQuotaPlan};
use std::error::Error;
use std::fmt;

use crate::crds::tenant::{TenantSpec, TenantSpecError};

const POOL_CONFIGMAP_PREFIX: &str = "ai-blaise-citus-pool-tenant";
const TENANT_ARCHIVE_URI_TEMPLATE: &str = "s3://ai-blaise-citus-archives";

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TenantReconcilePlan {
    pub spec: TenantSpec,
    pub quota_plan: TenantQuotaPlan,
    pub archive_plan: TenantArchivePlan,
    pub steps: Vec<TenantApplyStep>,
}

impl TryFrom<&TenantSpec> for TenantReconcilePlan {
    type Error = TenantReconcileError;

    fn try_from(spec: &TenantSpec) -> Result<Self, Self::Error> {
        spec.validate()?;

        let quota_plan = TenantQuotaPlan {
            tenant_name: spec.name.clone(),
            max_connections: spec.quotas.max_connections,
            max_qps: spec.quotas.max_qps,
        };
        quota_plan.validate()?;

        let archive_plan = TenantArchivePlan {
            tenant_name: spec.name.clone(),
            destination_uri: archive_destination_uri(&spec.name),
            retention_days: TENANT_ARCHIVE_DEFAULT_RETENTION_DAYS,
        };
        archive_plan.validate()?;

        let steps = build_tenant_apply_steps(spec, &quota_plan, &archive_plan);

        Ok(Self {
            spec: spec.clone(),
            quota_plan,
            archive_plan,
            steps,
        })
    }
}

impl TenantReconcilePlan {
    pub fn sql_script(&self) -> String {
        self.steps
            .iter()
            .filter(|step| step.kind == TenantApplyStepKind::Sql)
            .map(|step| step.payload.trim_end().to_string())
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub fn sql_step_count(&self) -> usize {
        self.steps
            .iter()
            .filter(|step| step.kind == TenantApplyStepKind::Sql)
            .count()
    }

    pub fn pool_configmap_name(&self) -> String {
        tenant_pool_configmap_name(&self.spec.name)
    }

    pub fn pool_configmap_payload(&self) -> String {
        let mut lines = Vec::new();
        lines.push(format!("tenant_name={}", self.spec.name));
        lines.push(format!("schema_name={}", self.spec.schema_name));
        lines.push(format!(
            "max_connections={}",
            self.spec.quotas.max_connections
        ));
        lines.push(format!("max_qps={}", self.spec.quotas.max_qps));
        lines.push(format!(
            "max_storage_bytes={}",
            self.spec.quotas.max_storage_bytes
        ));
        if let Some(region) = &self.spec.region_affinity {
            lines.push(format!("region_affinity={region}"));
        }
        lines.push(String::new());
        lines.join("\n")
    }

    pub fn delete_plan(&self) -> TenantDeletePlan {
        TenantDeletePlan {
            archive_plan: self.archive_plan.clone(),
            sql: format!(
                "SELECT companion_internal.plan_tenant_archive({tenant}, {destination}, {retention});",
                tenant = sql_literal(&self.archive_plan.tenant_name),
                destination = sql_literal(&self.archive_plan.destination_uri),
                retention = self.archive_plan.retention_days
            ),
        }
    }
}

pub const TENANT_ARCHIVE_DEFAULT_RETENTION_DAYS: u32 = 30;

fn archive_destination_uri(tenant_name: &str) -> String {
    format!(
        "{TENANT_ARCHIVE_URI_TEMPLATE}/{}",
        sanitize_dns_label(tenant_name, 63)
    )
}

fn build_tenant_apply_steps(
    spec: &TenantSpec,
    quota_plan: &TenantQuotaPlan,
    archive_plan: &TenantArchivePlan,
) -> Vec<TenantApplyStep> {
    let mut steps = Vec::new();

    steps.push(TenantApplyStep::sql(
        "ensure_tenant_schema",
        "TO1",
        format!(
            "CREATE SCHEMA IF NOT EXISTS {schema};",
            schema = quote_identifier(&spec.schema_name)
        ),
        true,
    ));

    steps.push(TenantApplyStep::sql(
        "set_tenant_quota",
        "TO2",
        format!(
            "SELECT companion_internal.set_tenant_quota({tenant}, {connections}, {qps});",
            tenant = sql_literal(&quota_plan.tenant_name),
            connections = quota_plan.max_connections,
            qps = quota_plan.max_qps
        ),
        true,
    ));

    if let Some(region) = &spec.region_affinity {
        steps.push(TenantApplyStep::sql(
            "set_tenant_region_affinity",
            "TO5",
            format!(
                "SELECT companion_internal.set_tenant_region_affinity({tenant}, {region});",
                tenant = sql_literal(&spec.name),
                region = sql_literal(region)
            ),
            true,
        ));
    }

    steps.push(TenantApplyStep::configmap(
        "publish_tenant_pool_configmap",
        "TO2",
        tenant_pool_configmap_name(&spec.name),
        true,
    ));

    steps.push(TenantApplyStep::archive(
        "register_tenant_archive_target",
        "TO1",
        archive_plan.destination_uri.clone(),
        true,
    ));

    steps
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TenantApplyStep {
    pub name: String,
    pub feature_id: String,
    pub kind: TenantApplyStepKind,
    pub payload: String,
    pub idempotent: bool,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum TenantApplyStepKind {
    Sql,
    PoolConfigMap,
    ArchiveTarget,
}

impl TenantApplyStep {
    fn sql(
        name: impl Into<String>,
        feature_id: impl Into<String>,
        sql: impl Into<String>,
        idempotent: bool,
    ) -> Self {
        Self {
            name: name.into(),
            feature_id: feature_id.into(),
            kind: TenantApplyStepKind::Sql,
            payload: sql.into(),
            idempotent,
        }
    }

    fn configmap(
        name: impl Into<String>,
        feature_id: impl Into<String>,
        configmap_name: impl Into<String>,
        idempotent: bool,
    ) -> Self {
        Self {
            name: name.into(),
            feature_id: feature_id.into(),
            kind: TenantApplyStepKind::PoolConfigMap,
            payload: configmap_name.into(),
            idempotent,
        }
    }

    fn archive(
        name: impl Into<String>,
        feature_id: impl Into<String>,
        destination_uri: impl Into<String>,
        idempotent: bool,
    ) -> Self {
        Self {
            name: name.into(),
            feature_id: feature_id.into(),
            kind: TenantApplyStepKind::ArchiveTarget,
            payload: destination_uri.into(),
            idempotent,
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TenantDeletePlan {
    pub archive_plan: TenantArchivePlan,
    pub sql: String,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum TenantReconcileError {
    InvalidSpec(TenantSpecError),
    InvalidPlan(TenantOperationError),
}

impl fmt::Display for TenantReconcileError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSpec(error) => write!(formatter, "{error}"),
            Self::InvalidPlan(error) => write!(formatter, "{error}"),
        }
    }
}

impl Error for TenantReconcileError {}

impl From<TenantSpecError> for TenantReconcileError {
    fn from(error: TenantSpecError) -> Self {
        Self::InvalidSpec(error)
    }
}

impl From<TenantOperationError> for TenantReconcileError {
    fn from(error: TenantOperationError) -> Self {
        Self::InvalidPlan(error)
    }
}

fn tenant_pool_configmap_name(tenant_name: &str) -> String {
    let suffix_limit = 63_usize.saturating_sub(POOL_CONFIGMAP_PREFIX.len() + 1);
    format!(
        "{POOL_CONFIGMAP_PREFIX}-{}",
        sanitize_dns_label(tenant_name, suffix_limit)
    )
}

fn sql_literal(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

fn quote_identifier(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\""))
}

fn sanitize_dns_label(value: &str, max_len: usize) -> String {
    let mut out = String::with_capacity(value.len());
    for byte in value.bytes() {
        let ch = byte as char;
        let lower = ch.to_ascii_lowercase();
        if lower.is_ascii_alphanumeric() {
            out.push(lower);
        } else {
            out.push('-');
        }
    }
    let mut trimmed = out.trim_matches('-').to_string();
    if trimmed.len() > max_len {
        trimmed.truncate(max_len);
        trimmed = trimmed.trim_matches('-').to_string();
    }
    if trimmed.is_empty() {
        "tenant".to_string()
    } else {
        trimmed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crds::tenant::TenantQuotas;

    fn spec_with_affinity(region: Option<&str>) -> TenantSpec {
        TenantSpec {
            name: "tenant-a".to_string(),
            schema_name: "tenant_a".to_string(),
            quotas: TenantQuotas {
                max_connections: 32,
                max_qps: 5_000,
                max_storage_bytes: 1_099_511_627_776,
            },
            region_affinity: region.map(str::to_string),
        }
    }

    #[test]
    fn reconcile_plan_with_affinity_emits_five_steps() {
        let spec = spec_with_affinity(Some("us-east-1"));
        let plan = TenantReconcilePlan::try_from(&spec).expect("valid plan");

        assert_eq!(plan.steps.len(), 5);
        assert_eq!(plan.sql_step_count(), 3);
        assert_eq!(plan.steps[0].feature_id, "TO1");
        assert_eq!(plan.steps[1].feature_id, "TO2");
        assert_eq!(plan.steps[2].feature_id, "TO5");
        assert_eq!(plan.steps[3].kind, TenantApplyStepKind::PoolConfigMap);
        assert_eq!(plan.steps[4].kind, TenantApplyStepKind::ArchiveTarget);
        assert!(plan
            .sql_script()
            .contains("CREATE SCHEMA IF NOT EXISTS \"tenant_a\""));
        assert!(plan
            .sql_script()
            .contains("companion_internal.set_tenant_quota('tenant-a', 32, 5000)"));
        assert!(plan
            .sql_script()
            .contains("companion_internal.set_tenant_region_affinity('tenant-a', 'us-east-1')"));
    }

    #[test]
    fn reconcile_plan_without_affinity_skips_region_step() {
        let plan = TenantReconcilePlan::try_from(&spec_with_affinity(None)).expect("valid plan");

        assert_eq!(plan.steps.len(), 4);
        assert!(plan.steps.iter().all(|step| step.feature_id != "TO5"));
    }

    #[test]
    fn pool_configmap_payload_carries_quotas_and_affinity() {
        let plan = TenantReconcilePlan::try_from(&spec_with_affinity(Some("us-east-1")))
            .expect("valid plan");

        let payload = plan.pool_configmap_payload();
        assert!(payload.contains("tenant_name=tenant-a"));
        assert!(payload.contains("schema_name=tenant_a"));
        assert!(payload.contains("max_connections=32"));
        assert!(payload.contains("max_qps=5000"));
        assert!(payload.contains("max_storage_bytes=1099511627776"));
        assert!(payload.contains("region_affinity=us-east-1"));
        assert_eq!(
            plan.pool_configmap_name(),
            "ai-blaise-citus-pool-tenant-tenant-a"
        );
    }

    #[test]
    fn delete_plan_queues_archive_without_dropping_schema() {
        let plan = TenantReconcilePlan::try_from(&spec_with_affinity(None)).expect("valid plan");
        let delete_plan = plan.delete_plan();

        assert_eq!(delete_plan.archive_plan.tenant_name, "tenant-a");
        assert!(delete_plan
            .sql
            .contains("companion_internal.plan_tenant_archive('tenant-a'"));
        assert!(!delete_plan.sql.to_lowercase().contains("drop schema"));
    }

    #[test]
    fn reconcile_plan_rejects_zero_quota() {
        let mut spec = spec_with_affinity(None);
        spec.quotas.max_connections = 0;

        assert!(matches!(
            TenantReconcilePlan::try_from(&spec),
            Err(TenantReconcileError::InvalidSpec(_))
        ));
    }

    #[test]
    fn configmap_name_obeys_dns_label_limit() {
        let name = tenant_pool_configmap_name(&"Tenant_".repeat(20));
        assert!(name.len() <= 63);
        assert!(name.starts_with(POOL_CONFIGMAP_PREFIX));
    }
}
