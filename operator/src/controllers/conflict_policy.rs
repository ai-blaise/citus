//! `ConflictPolicy` controller.

use super::{Context, ControllerError};
use crate::crds::conflict_policy::{ConflictClass, ConflictPolicySpec, ConflictResolution};
use crate::reconcile::conflict_policy::ConflictPolicyReconcilePlan;
use futures::StreamExt;
use kube::{
    api::Api,
    runtime::{controller::Action, watcher, Controller},
    CustomResource,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, error, info};

#[derive(CustomResource, Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[kube(
    group = "citus.ai-blaise.io",
    version = "v2",
    kind = "ConflictPolicy",
    namespaced,
    status = "ConflictPolicyStatus"
)]
#[serde(rename_all = "camelCase")]
pub struct ConflictPolicyCrSpec {
    pub table: String,
    pub class: String,
    pub resolution: String,
    #[serde(default)]
    pub custom_function: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ConflictPolicyStatus {
    pub policy_name: String,
    pub conflict_class: String,
    pub resolution: String,
    pub apply_steps: usize,
}

impl ConflictPolicyCrSpec {
    pub fn to_authoritative(&self) -> Result<ConflictPolicySpec, String> {
        Ok(ConflictPolicySpec {
            table: self.table.clone(),
            class: parse_conflict_class(&self.class)?,
            resolution: parse_resolution(&self.resolution)?,
            custom_function: self.custom_function.clone(),
        })
    }
}

fn parse_conflict_class(value: &str) -> Result<ConflictClass, String> {
    match normalize_token(value).as_str() {
        "insertinsert" | "insertexists" => Ok(ConflictClass::InsertInsert),
        "insertupdate" | "updateexists" => Ok(ConflictClass::InsertUpdate),
        "updateupdate" | "updateorigindiffers" => Ok(ConflictClass::UpdateUpdate),
        "updatedelete" | "updatemissing" => Ok(ConflictClass::UpdateDelete),
        "deleteupdate" | "deleteorigindiffers" => Ok(ConflictClass::DeleteUpdate),
        "deletedelete" | "deletemissing" => Ok(ConflictClass::DeleteDelete),
        "constraint" | "deleteexists" => Ok(ConflictClass::Constraint),
        other => Err(format!("unsupported conflict class: {other}")),
    }
}

fn parse_resolution(value: &str) -> Result<ConflictResolution, String> {
    match normalize_token(value).as_str() {
        "lastwritewins" | "applyremoteifnewer" => Ok(ConflictResolution::LastWriteWins),
        "originwins" | "applyremote" => Ok(ConflictResolution::OriginWins),
        "targetwins" | "keeplocal" => Ok(ConflictResolution::TargetWins),
        "skip" => Ok(ConflictResolution::Skip),
        "error" => Ok(ConflictResolution::Error),
        "customfunction" | "mergefunction" => Ok(ConflictResolution::CustomFunction),
        other => Err(format!("unsupported conflict resolution: {other}")),
    }
}

fn normalize_token(value: &str) -> String {
    value
        .trim()
        .chars()
        .filter(|character| !matches!(character, '-' | '_'))
        .collect::<String>()
        .to_ascii_lowercase()
}

pub async fn run(ctx: Arc<Context>) -> Result<(), ControllerError> {
    let api: Api<ConflictPolicy> = Api::default_namespaced(ctx.client.clone());
    info!("ConflictPolicy controller starting");
    Controller::new(api, watcher::Config::default())
        .run(reconcile, error_policy, ctx)
        .for_each(|res| async move {
            match res {
                Ok((object, _action)) => debug!(?object, "reconciled ConflictPolicy"),
                Err(error) => error!(?error, "ConflictPolicy reconcile error"),
            }
        })
        .await;
    Ok(())
}

async fn reconcile(
    conflict_policy: Arc<ConflictPolicy>,
    ctx: Arc<Context>,
) -> Result<Action, ControllerError> {
    let resource_name = conflict_policy
        .metadata
        .name
        .as_deref()
        .unwrap_or("conflict-policy");
    let authoritative = conflict_policy
        .spec
        .to_authoritative()
        .map_err(ControllerError::InvalidSpec)?;
    let plan = ConflictPolicyReconcilePlan::from_spec(resource_name, &authoritative)
        .map_err(|error| ControllerError::InvalidSpec(error.to_string()))?;
    info!(
        conflict_policy = ?conflict_policy.metadata.name,
        table = %plan.table,
        class = %plan.class_str(),
        resolution = %plan.resolution_str(),
        apply_steps = plan.apply_plan().steps.len(),
        "ConflictPolicy reconcile plan built"
    );
    Ok(Action::requeue(ctx.default_requeue))
}

fn error_policy(
    _conflict_policy: Arc<ConflictPolicy>,
    error: &ControllerError,
    ctx: Arc<Context>,
) -> Action {
    error!(?error, "ConflictPolicy controller backoff");
    Action::requeue(ctx.default_requeue)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cr_spec_round_trips_into_reconcile_plan() {
        let cr = ConflictPolicyCrSpec {
            table: "public.reference_accounts".to_string(),
            class: "UpdateUpdate".to_string(),
            resolution: "LastWriteWins".to_string(),
            custom_function: None,
        };
        let authoritative = cr.to_authoritative().expect("valid spec");
        let plan = ConflictPolicyReconcilePlan::from_spec("accounts-lww", &authoritative)
            .expect("valid plan");
        assert_eq!(plan.class_str(), "update_origin_differs");
        assert_eq!(plan.resolution_str(), "apply_remote_if_newer");
    }

    #[test]
    fn unsupported_class_is_rejected() {
        assert_eq!(
            parse_conflict_class("split-brain"),
            Err("unsupported conflict class: splitbrain".to_string())
        );
    }
}
