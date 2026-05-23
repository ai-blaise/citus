// FEATURE: B2
// FEATURE: B6

use ai_blaise_citus_sidecar_shared::BackupRestoreContract;
use std::error::Error;
use std::fmt;

use crate::crds::backup::{BackupProvider, BackupSpec, BackupSpecError};

const SIDECAR_DEPLOYMENT_PREFIX: &str = "citus-sidecar-backup";
const CONFIGMAP_PREFIX: &str = "citus-backup";
const WAL_STATUS_PATH: &str = "/wal/status";
const BACKUPS_PATH: &str = "/backups";

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct BackupReconcilePlan {
    pub resource_name: String,
    pub spec: BackupSpec,
    pub contract: BackupRestoreContract,
    pub archive_uri: String,
    pub sidecar_deployment_name: String,
    pub configmap_name: String,
    pub steps: Vec<BackupApplyStep>,
}

impl TryFrom<&BackupSpec> for BackupReconcilePlan {
    type Error = BackupReconcileError;

    fn try_from(spec: &BackupSpec) -> Result<Self, Self::Error> {
        Self::from_resource_name("default", spec)
    }
}

impl BackupReconcilePlan {
    pub fn from_resource_name(
        resource_name: &str,
        spec: &BackupSpec,
    ) -> Result<Self, BackupReconcileError> {
        spec.validate()?;
        let resource_name = sanitize_dns_label(resource_name, 40);
        let archive_uri = object_store_uri(spec);
        let sidecar_deployment_name = format!("{SIDECAR_DEPLOYMENT_PREFIX}-{resource_name}");
        let configmap_name = format!("{CONFIGMAP_PREFIX}-{resource_name}");

        let contract = BackupRestoreContract {
            schedule: spec.schedule.clone(),
            archive_uri: archive_uri.clone(),
            pitr_target: None,
            queryable_branch_name: None,
        };
        contract
            .validate()
            .map_err(|error| BackupReconcileError::InvalidContract(error.to_string()))?;

        let steps = build_backup_apply_steps(
            spec,
            &archive_uri,
            &sidecar_deployment_name,
            &configmap_name,
        );

        Ok(Self {
            resource_name,
            spec: spec.clone(),
            contract,
            archive_uri,
            sidecar_deployment_name,
            configmap_name,
            steps,
        })
    }

    pub fn sidecar_deployment_name(&self) -> &str {
        &self.sidecar_deployment_name
    }

    pub fn sidecar_configmap_payload(&self) -> String {
        let mut lines = Vec::new();
        lines.push(format!("schedule={}", self.spec.schedule));
        lines.push(format!("retention_days={}", self.spec.retention_days));
        lines.push(format!(
            "provider={}",
            provider_label(self.spec.target.provider)
        ));
        lines.push(format!("bucket={}", self.spec.target.bucket));
        lines.push(format!("prefix={}", self.spec.target.prefix));
        lines.push(format!("archive_uri={}", self.archive_uri));
        if let Some(encryption) = &self.spec.encryption {
            lines.push(format!("kms_key_ref={}", encryption.kms_key_ref));
        }
        lines.push(String::new());
        lines.join("\n")
    }

    pub fn status_endpoints(&self) -> [String; 2] {
        [
            format!("GET {WAL_STATUS_PATH}"),
            format!("GET {BACKUPS_PATH}"),
        ]
    }

    pub fn delete_plan(&self) -> BackupDeletePlan {
        BackupDeletePlan {
            sidecar_deployment_name: self.sidecar_deployment_name.clone(),
            archive_uri: self.archive_uri.clone(),
            retention_days: self.spec.retention_days,
            stop_sidecar: true,
            delete_archives: false,
        }
    }
}

fn build_backup_apply_steps(
    spec: &BackupSpec,
    archive_uri: &str,
    deployment_name: &str,
    configmap_name: &str,
) -> Vec<BackupApplyStep> {
    let mut steps = Vec::new();

    steps.push(BackupApplyStep::sidecar_deployment(
        "deploy_backup_sidecar",
        "B2",
        deployment_name.to_string(),
        true,
    ));

    steps.push(BackupApplyStep::sidecar_config(
        "publish_backup_sidecar_config",
        "B2",
        format!(
            "configmap={configmap_name};schedule={};archive_uri={archive_uri};retention_days={}",
            spec.schedule, spec.retention_days
        ),
        true,
    ));

    if let Some(encryption) = &spec.encryption {
        steps.push(BackupApplyStep::kms_binding(
            "bind_backup_kms_key",
            "B6",
            encryption.kms_key_ref.clone(),
            true,
        ));
    }

    steps.push(BackupApplyStep::status_probe(
        "register_backup_status_probes",
        "B2",
        format!("wal={WAL_STATUS_PATH};backups={BACKUPS_PATH}"),
        true,
    ));

    steps
}

fn object_store_uri(spec: &BackupSpec) -> String {
    format!(
        "{scheme}://{bucket}/{prefix}",
        scheme = provider_scheme(spec.target.provider),
        bucket = spec.target.bucket.trim_matches('/'),
        prefix = spec.target.prefix.trim_matches('/')
    )
}

fn provider_scheme(provider: BackupProvider) -> &'static str {
    match provider {
        BackupProvider::S3 => "s3",
        BackupProvider::Gcs => "gs",
        BackupProvider::Azure => "az",
    }
}

fn provider_label(provider: BackupProvider) -> &'static str {
    match provider {
        BackupProvider::S3 => "s3",
        BackupProvider::Gcs => "gcs",
        BackupProvider::Azure => "azure",
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct BackupApplyStep {
    pub name: String,
    pub feature_id: String,
    pub kind: BackupApplyStepKind,
    pub payload: String,
    pub idempotent: bool,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum BackupApplyStepKind {
    SidecarDeployment,
    SidecarConfig,
    KmsBinding,
    StatusProbe,
}

impl BackupApplyStep {
    fn sidecar_deployment(
        name: impl Into<String>,
        feature_id: impl Into<String>,
        deployment_name: impl Into<String>,
        idempotent: bool,
    ) -> Self {
        Self {
            name: name.into(),
            feature_id: feature_id.into(),
            kind: BackupApplyStepKind::SidecarDeployment,
            payload: deployment_name.into(),
            idempotent,
        }
    }

    fn sidecar_config(
        name: impl Into<String>,
        feature_id: impl Into<String>,
        config: impl Into<String>,
        idempotent: bool,
    ) -> Self {
        Self {
            name: name.into(),
            feature_id: feature_id.into(),
            kind: BackupApplyStepKind::SidecarConfig,
            payload: config.into(),
            idempotent,
        }
    }

    fn kms_binding(
        name: impl Into<String>,
        feature_id: impl Into<String>,
        kms_key_ref: impl Into<String>,
        idempotent: bool,
    ) -> Self {
        Self {
            name: name.into(),
            feature_id: feature_id.into(),
            kind: BackupApplyStepKind::KmsBinding,
            payload: kms_key_ref.into(),
            idempotent,
        }
    }

    fn status_probe(
        name: impl Into<String>,
        feature_id: impl Into<String>,
        endpoints: impl Into<String>,
        idempotent: bool,
    ) -> Self {
        Self {
            name: name.into(),
            feature_id: feature_id.into(),
            kind: BackupApplyStepKind::StatusProbe,
            payload: endpoints.into(),
            idempotent,
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct BackupDeletePlan {
    pub sidecar_deployment_name: String,
    pub archive_uri: String,
    pub retention_days: u32,
    pub stop_sidecar: bool,
    pub delete_archives: bool,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum BackupReconcileError {
    InvalidSpec(BackupSpecError),
    InvalidContract(String),
}

impl fmt::Display for BackupReconcileError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSpec(error) => write!(formatter, "{error}"),
            Self::InvalidContract(message) => write!(formatter, "{message}"),
        }
    }
}

impl Error for BackupReconcileError {}

impl From<BackupSpecError> for BackupReconcileError {
    fn from(error: BackupSpecError) -> Self {
        Self::InvalidSpec(error)
    }
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
        "backup".to_string()
    } else {
        trimmed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crds::backup::{BackupEncryption, BackupTarget};

    fn encrypted_spec() -> BackupSpec {
        BackupSpec {
            schedule: "0 */6 * * *".to_string(),
            retention_days: 30,
            target: BackupTarget {
                provider: BackupProvider::S3,
                bucket: "ai-blaise-citus-backups".to_string(),
                prefix: "prod/us-east-1".to_string(),
            },
            encryption: Some(BackupEncryption {
                kms_key_ref: "aws-kms-prod".to_string(),
            }),
        }
    }

    #[test]
    fn encrypted_backup_plan_emits_four_steps() {
        let plan = BackupReconcilePlan::from_resource_name("nightly", &encrypted_spec())
            .expect("valid plan");

        assert_eq!(plan.steps.len(), 4);
        assert_eq!(plan.steps[0].kind, BackupApplyStepKind::SidecarDeployment);
        assert_eq!(plan.steps[0].payload, "citus-sidecar-backup-nightly");
        assert_eq!(plan.steps[1].kind, BackupApplyStepKind::SidecarConfig);
        assert!(plan.steps[1].payload.contains("schedule=0 */6 * * *"));
        assert!(plan.steps[1]
            .payload
            .contains("archive_uri=s3://ai-blaise-citus-backups/prod/us-east-1"));
        assert_eq!(plan.steps[2].feature_id, "B6");
        assert_eq!(plan.steps[2].payload, "aws-kms-prod");
        assert_eq!(plan.steps[3].kind, BackupApplyStepKind::StatusProbe);
        assert!(plan.steps[3].payload.contains("/wal/status"));
        assert!(plan.steps[3].payload.contains("/backups"));
        assert_eq!(
            plan.archive_uri,
            "s3://ai-blaise-citus-backups/prod/us-east-1"
        );
        assert_eq!(plan.configmap_name, "citus-backup-nightly");
    }

    #[test]
    fn unencrypted_backup_plan_skips_kms_step() {
        let mut spec = encrypted_spec();
        spec.encryption = None;

        let plan = BackupReconcilePlan::from_resource_name("nightly", &spec).expect("valid plan");
        assert_eq!(plan.steps.len(), 3);
        assert!(plan.steps.iter().all(|step| step.feature_id != "B6"));
    }

    #[test]
    fn sidecar_configmap_payload_includes_provider_and_kms() {
        let plan = BackupReconcilePlan::from_resource_name("nightly", &encrypted_spec())
            .expect("valid plan");

        let payload = plan.sidecar_configmap_payload();
        assert!(payload.contains("schedule=0 */6 * * *"));
        assert!(payload.contains("retention_days=30"));
        assert!(payload.contains("provider=s3"));
        assert!(payload.contains("bucket=ai-blaise-citus-backups"));
        assert!(payload.contains("prefix=prod/us-east-1"));
        assert!(payload.contains("archive_uri=s3://ai-blaise-citus-backups/prod/us-east-1"));
        assert!(payload.contains("kms_key_ref=aws-kms-prod"));
    }

    #[test]
    fn delete_plan_stops_sidecar_but_keeps_archives() {
        let plan = BackupReconcilePlan::from_resource_name("nightly", &encrypted_spec())
            .expect("valid plan");
        let delete_plan = plan.delete_plan();

        assert_eq!(
            delete_plan.sidecar_deployment_name,
            "citus-sidecar-backup-nightly"
        );
        assert_eq!(
            delete_plan.archive_uri,
            "s3://ai-blaise-citus-backups/prod/us-east-1"
        );
        assert_eq!(delete_plan.retention_days, 30);
        assert!(delete_plan.stop_sidecar);
        assert!(!delete_plan.delete_archives);
    }

    #[test]
    fn backup_plan_rejects_zero_retention() {
        let mut spec = encrypted_spec();
        spec.retention_days = 0;

        let result = BackupReconcilePlan::from_resource_name("nightly", &spec);
        assert!(matches!(
            result,
            Err(BackupReconcileError::InvalidSpec(
                BackupSpecError::InvalidRetention
            ))
        ));
    }

    #[test]
    fn gcs_provider_produces_gs_scheme() {
        let mut spec = encrypted_spec();
        spec.target.provider = BackupProvider::Gcs;

        let plan = BackupReconcilePlan::from_resource_name("nightly", &spec).expect("valid plan");
        assert!(plan.archive_uri.starts_with("gs://"));
    }
}
