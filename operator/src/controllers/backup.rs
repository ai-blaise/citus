//! `Backup` controller.

use super::{Context, ControllerError};
use crate::crds::backup::{BackupEncryption, BackupProvider, BackupSpec, BackupTarget};
use crate::reconcile::backup::BackupReconcilePlan;
use futures::StreamExt;
use kube::{
    api::Api,
    runtime::{controller::Action, watcher, Controller},
    CustomResource, ResourceExt,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, error, info};

/// Kube-rs typed resource for the Backup CRD.
#[derive(CustomResource, Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[kube(
    group = "citus.ai-blaise.io",
    version = "v2",
    kind = "Backup",
    namespaced
)]
#[serde(rename_all = "camelCase")]
pub struct BackupCrSpec {
    pub schedule: String,
    pub retention_days: u32,
    pub target: BackupTargetCr,
    #[serde(default)]
    pub encryption: Option<BackupEncryptionCr>,
}

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct BackupTargetCr {
    #[serde(default = "default_provider")]
    pub provider: String,
    pub bucket: String,
    pub prefix: String,
}

fn default_provider() -> String {
    "S3".to_string()
}

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct BackupEncryptionCr {
    pub kms_key_ref: String,
}

impl BackupCrSpec {
    pub fn to_authoritative(&self) -> Result<BackupSpec, String> {
        Ok(BackupSpec {
            schedule: self.schedule.clone(),
            retention_days: self.retention_days,
            target: BackupTarget {
                provider: parse_provider(&self.target.provider)?,
                bucket: self.target.bucket.clone(),
                prefix: self.target.prefix.clone(),
            },
            encryption: self.encryption.as_ref().map(|encryption| BackupEncryption {
                kms_key_ref: encryption.kms_key_ref.clone(),
            }),
        })
    }
}

fn parse_provider(provider: &str) -> Result<BackupProvider, String> {
    match provider {
        "S3" | "s3" => Ok(BackupProvider::S3),
        "Gcs" | "GCS" | "gcs" => Ok(BackupProvider::Gcs),
        "Azure" | "azure" => Ok(BackupProvider::Azure),
        other => Err(format!("unknown backup provider: {other}")),
    }
}

pub async fn run(ctx: Arc<Context>) -> Result<(), ControllerError> {
    let api: Api<Backup> = Api::default_namespaced(ctx.client.clone());
    info!("Backup controller starting");
    Controller::new(api, watcher::Config::default())
        .run(reconcile, error_policy, ctx)
        .for_each(|res| async move {
            match res {
                Ok((object, _action)) => debug!(?object, "reconciled Backup"),
                Err(error) => error!(?error, "Backup reconcile error"),
            }
        })
        .await;
    Ok(())
}

async fn reconcile(backup: Arc<Backup>, ctx: Arc<Context>) -> Result<Action, ControllerError> {
    let authoritative = backup
        .spec
        .to_authoritative()
        .map_err(ControllerError::InvalidSpec)?;
    let plan = BackupReconcilePlan::from_resource_name(&backup.name_any(), &authoritative)
        .map_err(|error| ControllerError::InvalidSpec(error.to_string()))?;
    info!(
        backup = ?backup.metadata.name,
        archive_uri = %plan.archive_uri,
        apply_steps = plan.steps.len(),
        sidecar_deployment = %plan.sidecar_deployment_name(),
        configmap = %plan.configmap_name,
        "Backup reconcile plan built"
    );
    Ok(Action::requeue(ctx.default_requeue))
}

fn error_policy(_backup: Arc<Backup>, error: &ControllerError, ctx: Arc<Context>) -> Action {
    error!(?error, "Backup controller backoff");
    Action::requeue(ctx.default_requeue)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cr_spec_round_trips_into_reconcile_plan() {
        let cr = BackupCrSpec {
            schedule: "0 */6 * * *".to_string(),
            retention_days: 30,
            target: BackupTargetCr {
                provider: "S3".to_string(),
                bucket: "ai-blaise-citus-backups".to_string(),
                prefix: "prod/us-east-1".to_string(),
            },
            encryption: Some(BackupEncryptionCr {
                kms_key_ref: "aws-kms-prod".to_string(),
            }),
        };
        let authoritative = cr.to_authoritative().expect("backup spec");
        let plan = BackupReconcilePlan::from_resource_name("nightly", &authoritative)
            .expect("backup plan");
        assert_eq!(plan.steps.len(), 4);
        assert_eq!(
            plan.archive_uri,
            "s3://ai-blaise-citus-backups/prod/us-east-1"
        );
    }

    #[test]
    fn cr_spec_rejects_unknown_provider() {
        let cr = BackupCrSpec {
            schedule: "0 */6 * * *".to_string(),
            retention_days: 30,
            target: BackupTargetCr {
                provider: "Tape".to_string(),
                bucket: "ai-blaise-citus-backups".to_string(),
                prefix: "prod/us-east-1".to_string(),
            },
            encryption: None,
        };
        assert!(cr.to_authoritative().is_err());
    }
}
