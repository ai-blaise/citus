//! kube-rs controllers for the V2 operator catalog.
//!
//! Each CRD owns a sub-module that registers a typed kube `CustomResource`
//! wrapping the corresponding `*Spec` type defined in `crate::crds`, plus a
//! `reconcile`/`error_policy` pair driven by `kube::runtime::Controller`.
//!
//! `controllers::serve(client, selection)` spawns the selected controllers on
//! the supplied tokio runtime and fails closed once any controller exits.

pub mod backup;
pub mod boundary;
pub mod citus_cluster;
pub mod conflict_policy;
pub mod federation;
pub mod function;
pub mod hypertable;
pub mod migration;
pub mod region;
pub mod scheduled_repack;
pub mod search_index;
pub mod sidecar;
pub mod survival_goal;
pub mod tenant;
pub mod webhook;

use boundary::{execution_mode_from_env, BoundaryError, ExecutionMode};
use k8s_openapi::apiextensions_apiserver::pkg::apis::apiextensions::v1::CustomResourceDefinition;
use kube::{Client, CustomResourceExt};
use std::collections::HashSet;
use std::env;
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;
use tokio::task::{JoinError, JoinSet};
use tracing::{info, warn};

pub const CONTROLLER_COUNT: usize = 14;

/// Typed identity for every controller with a live kube-rs watch loop.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ControllerKind {
    CitusCluster,
    Migration,
    Tenant,
    Region,
    SurvivalGoal,
    Backup,
    Hypertable,
    Federation,
    SearchIndex,
    Webhook,
    Function,
    ScheduledRepack,
    ConflictPolicy,
    Sidecar,
}

impl ControllerKind {
    pub const ALL: [Self; CONTROLLER_COUNT] = [
        Self::CitusCluster,
        Self::Migration,
        Self::Tenant,
        Self::Region,
        Self::SurvivalGoal,
        Self::Backup,
        Self::Hypertable,
        Self::Federation,
        Self::SearchIndex,
        Self::Webhook,
        Self::Function,
        Self::ScheduledRepack,
        Self::ConflictPolicy,
        Self::Sidecar,
    ];

    pub const fn selector_name(self) -> &'static str {
        match self {
            Self::CitusCluster => "citus_cluster",
            Self::Migration => "migration",
            Self::Tenant => "tenant",
            Self::Region => "region",
            Self::SurvivalGoal => "survival_goal",
            Self::Backup => "backup",
            Self::Hypertable => "hypertable",
            Self::Federation => "federation",
            Self::SearchIndex => "search_index",
            Self::Webhook => "webhook",
            Self::Function => "function",
            Self::ScheduledRepack => "scheduled_repack",
            Self::ConflictPolicy => "conflict_policy",
            Self::Sidecar => "sidecar",
        }
    }

    pub const fn resource_kind(self) -> &'static str {
        match self {
            Self::CitusCluster => "CitusCluster",
            Self::Migration => "Migration",
            Self::Tenant => "Tenant",
            Self::Region => "Region",
            Self::SurvivalGoal => "SurvivalGoal",
            Self::Backup => "Backup",
            Self::Hypertable => "Hypertable",
            Self::Federation => "Federation",
            Self::SearchIndex => "SearchIndex",
            Self::Webhook => "Webhook",
            Self::Function => "Function",
            Self::ScheduledRepack => "ScheduledRepack",
            Self::ConflictPolicy => "ConflictPolicy",
            Self::Sidecar => "Sidecar",
        }
    }

    pub const fn resource_plural(self) -> &'static str {
        match self {
            Self::CitusCluster => "citusclusters",
            Self::Migration => "migrations",
            Self::Tenant => "tenants",
            Self::Region => "regions",
            Self::SurvivalGoal => "survivalgoals",
            Self::Backup => "backups",
            Self::Hypertable => "hypertables",
            Self::Federation => "federations",
            Self::SearchIndex => "searchindexes",
            Self::Webhook => "webhooks",
            Self::Function => "functions",
            Self::ScheduledRepack => "scheduledrepacks",
            Self::ConflictPolicy => "conflictpolicies",
            Self::Sidecar => "sidecars",
        }
    }

    fn parse(token: &str) -> Option<Self> {
        match token.trim().to_ascii_lowercase().as_str() {
            "citus_cluster" | "cituscluster" => Some(Self::CitusCluster),
            "migration" => Some(Self::Migration),
            "tenant" => Some(Self::Tenant),
            "region" => Some(Self::Region),
            "survival_goal" => Some(Self::SurvivalGoal),
            "backup" => Some(Self::Backup),
            "hypertable" => Some(Self::Hypertable),
            "federation" => Some(Self::Federation),
            "search_index" => Some(Self::SearchIndex),
            "webhook" => Some(Self::Webhook),
            "function" => Some(Self::Function),
            "scheduled_repack" => Some(Self::ScheduledRepack),
            "conflict_policy" => Some(Self::ConflictPolicy),
            "sidecar" => Some(Self::Sidecar),
            _ => None,
        }
    }

    fn crd(self) -> CustomResourceDefinition {
        match self {
            Self::CitusCluster => citus_cluster::CitusCluster::crd(),
            Self::Migration => migration::Migration::crd(),
            Self::Tenant => tenant::Tenant::crd(),
            Self::Region => region::Region::crd(),
            Self::SurvivalGoal => survival_goal::SurvivalGoal::crd(),
            Self::Backup => backup::Backup::crd(),
            Self::Hypertable => hypertable::Hypertable::crd(),
            Self::Federation => federation::Federation::crd(),
            Self::SearchIndex => search_index::SearchIndex::crd(),
            Self::Webhook => webhook::Webhook::crd(),
            Self::Function => function::Function::crd(),
            Self::ScheduledRepack => scheduled_repack::ScheduledRepack::crd(),
            Self::ConflictPolicy => conflict_policy::ConflictPolicy::crd(),
            Self::Sidecar => sidecar::Sidecar::crd(),
        }
    }
}

/// Validated controller subset selected for one operator process.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ControllerSelection {
    controllers: Vec<ControllerKind>,
}

impl ControllerSelection {
    pub fn all() -> Self {
        Self {
            controllers: ControllerKind::ALL.to_vec(),
        }
    }

    pub fn from_env() -> Result<Self, ControllerSelectionError> {
        match env::var("AI_BLAISE_OPERATOR_CONTROLLERS") {
            Ok(value) => Self::parse(&value),
            Err(env::VarError::NotPresent) => Ok(Self::all()),
            Err(env::VarError::NotUnicode(_)) => {
                Err(ControllerSelectionError::InvalidSelectorEncoding)
            }
        }
    }

    fn parse(value: &str) -> Result<Self, ControllerSelectionError> {
        if value.trim().is_empty() {
            return Err(ControllerSelectionError::EmptySelection);
        }
        if value.trim().eq_ignore_ascii_case("all") {
            return Ok(Self::all());
        }

        let mut controllers = Vec::new();
        let mut seen = HashSet::new();
        for raw in value.split(',') {
            let token = raw.trim();
            if token.is_empty() {
                return Err(ControllerSelectionError::EmptySelector);
            }
            if token.eq_ignore_ascii_case("all") {
                return Err(ControllerSelectionError::MixedAllSelection);
            }
            let controller = ControllerKind::parse(token)
                .ok_or_else(|| ControllerSelectionError::UnknownController(token.to_string()))?;
            if !seen.insert(controller) {
                return Err(ControllerSelectionError::DuplicateController(
                    controller.selector_name(),
                ));
            }
            controllers.push(controller);
        }

        Ok(Self { controllers })
    }

    pub fn as_slice(&self) -> &[ControllerKind] {
        &self.controllers
    }
}

/// Structural CRDs for exactly the controllers in [`ControllerKind::ALL`].
pub fn controller_crds() -> Vec<CustomResourceDefinition> {
    ControllerKind::ALL
        .iter()
        .copied()
        .map(ControllerKind::crd)
        .collect()
}

#[derive(Debug, Error)]
pub enum ControllerSelectionError {
    #[error("AI_BLAISE_OPERATOR_CONTROLLERS must select at least one controller")]
    EmptySelection,
    #[error("AI_BLAISE_OPERATOR_CONTROLLERS contains an empty selector")]
    EmptySelector,
    #[error("AI_BLAISE_OPERATOR_CONTROLLERS is not valid Unicode")]
    InvalidSelectorEncoding,
    #[error("AI_BLAISE_OPERATOR_CONTROLLERS cannot mix 'all' with named controllers")]
    MixedAllSelection,
    #[error("unknown controller selector '{0}'")]
    UnknownController(String),
    #[error("duplicate controller selector '{0}'")]
    DuplicateController(&'static str),
}

/// Shared context handed to every reconciler.
#[derive(Clone)]
pub struct Context {
    pub client: Client,
    pub default_requeue: Duration,
    pub execution_mode: ExecutionMode,
}

impl Context {
    pub fn new(client: Client) -> Result<Arc<Self>, ControllerError> {
        Ok(Arc::new(Self {
            client,
            default_requeue: Duration::from_secs(30),
            execution_mode: execution_mode_from_env()?,
        }))
    }
}

/// Errors common to every reconciler.
#[derive(Debug, Error)]
pub enum ControllerError {
    #[error("kube-rs client error: {0}")]
    Kube(#[from] kube::Error),
    #[error("companion error: {0}")]
    Companion(String),
    #[error("invalid spec: {0}")]
    InvalidSpec(String),
    #[error("controller boundary error: {0}")]
    Boundary(#[from] BoundaryError),
    #[error("finalizer error: {0}")]
    Finalizer(String),
}

#[derive(Debug, Error)]
pub enum ServeError {
    #[error("invalid controller selection: {0}")]
    Selection(#[from] ControllerSelectionError),
    #[error("controller setup failed: {0}")]
    Setup(#[source] Box<ControllerError>),
    #[error("no controller tasks were started")]
    NoControllerTasks,
    #[error("{0} controller exited unexpectedly")]
    ControllerExited(&'static str),
    #[error("{controller} controller failed: {source}")]
    ControllerFailed {
        controller: &'static str,
        #[source]
        source: Box<ControllerError>,
    },
    #[error("controller task terminated unexpectedly: {0}")]
    ControllerTask(#[source] JoinError),
}

/// Parse the environment selector and serve the resulting controller set.
pub async fn serve_all(client: Client) -> Result<(), ServeError> {
    serve(client, ControllerSelection::from_env()?).await
}

/// Spawn the validated controller subset concurrently. A kube
/// `Controller::run` stream is infinite under normal conditions, so every
/// task exit is terminal and returned as an error.
pub async fn serve(client: Client, selection: ControllerSelection) -> Result<(), ServeError> {
    let ctx = Context::new(client).map_err(|source| ServeError::Setup(Box::new(source)))?;
    let selected_names = selection
        .as_slice()
        .iter()
        .map(|controller| controller.selector_name())
        .collect::<Vec<_>>()
        .join(",");
    info!(
        mode = ctx.execution_mode.as_str(),
        controllers = %selected_names,
        "operator serving validated controller selection"
    );

    let mut tasks = JoinSet::new();
    for controller in selection.as_slice().iter().copied() {
        let controller_ctx = ctx.clone();
        tasks.spawn(async move {
            let result = run_controller(controller, controller_ctx).await;
            (controller, result)
        });
    }

    let terminal = tasks
        .join_next()
        .await
        .ok_or(ServeError::NoControllerTasks)?
        .map_err(ServeError::ControllerTask)?;
    terminal_controller_result(terminal.0, terminal.1)
}

async fn run_controller(
    controller: ControllerKind,
    ctx: Arc<Context>,
) -> Result<(), ControllerError> {
    match controller {
        ControllerKind::CitusCluster => citus_cluster::run(ctx).await,
        ControllerKind::Migration => migration::run(ctx).await,
        ControllerKind::Tenant => tenant::run(ctx).await,
        ControllerKind::Region => region::run(ctx).await,
        ControllerKind::SurvivalGoal => survival_goal::run(ctx).await,
        ControllerKind::Backup => backup::run(ctx).await,
        ControllerKind::Hypertable => hypertable::run(ctx).await,
        ControllerKind::Federation => federation::run(ctx).await,
        ControllerKind::SearchIndex => search_index::run(ctx).await,
        ControllerKind::Webhook => webhook::run(ctx).await,
        ControllerKind::Function => function::run(ctx).await,
        ControllerKind::ScheduledRepack => scheduled_repack::run(ctx).await,
        ControllerKind::ConflictPolicy => conflict_policy::run(ctx).await,
        ControllerKind::Sidecar => sidecar::run(ctx).await,
    }
}

fn terminal_controller_result(
    controller: ControllerKind,
    result: Result<(), ControllerError>,
) -> Result<(), ServeError> {
    let name = controller.selector_name();
    match result {
        Ok(()) => {
            warn!(controller = name, "controller exited unexpectedly");
            Err(ServeError::ControllerExited(name))
        }
        Err(source) => {
            warn!(controller = name, ?source, "controller failed");
            Err(ServeError::ControllerFailed {
                controller: name,
                source: Box::new(source),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selector_catalog_is_typed_and_complete() {
        let selection = ControllerSelection::parse(
            "citus_cluster,migration,tenant,region,survival_goal,backup,hypertable,\
             federation,search_index,webhook,function,scheduled_repack,conflict_policy,sidecar",
        )
        .expect("complete selector catalog");
        assert_eq!(selection.as_slice(), ControllerKind::ALL);
        assert_eq!(ControllerSelection::parse("all").unwrap(), selection);
        assert_eq!(ControllerSelection::all(), selection);
    }

    #[test]
    fn selector_preserves_existing_citus_alias_and_supports_subsets() {
        assert_eq!(
            ControllerSelection::parse("CitusCluster")
                .unwrap()
                .as_slice(),
            &[ControllerKind::CitusCluster]
        );
        assert_eq!(
            ControllerSelection::parse("backup,sidecar")
                .unwrap()
                .as_slice(),
            &[ControllerKind::Backup, ControllerKind::Sidecar]
        );
    }

    #[test]
    fn selector_rejects_typos_empty_segments_duplicates_and_mixed_all() {
        assert!(matches!(
            ControllerSelection::parse("sideacr"),
            Err(ControllerSelectionError::UnknownController(value)) if value == "sideacr"
        ));
        assert!(matches!(
            ControllerSelection::parse("sidecar,,backup"),
            Err(ControllerSelectionError::EmptySelector)
        ));
        assert!(matches!(
            ControllerSelection::parse("sidecar,sidecar"),
            Err(ControllerSelectionError::DuplicateController("sidecar"))
        ));
        assert!(matches!(
            ControllerSelection::parse("all,sidecar"),
            Err(ControllerSelectionError::MixedAllSelection)
        ));
        assert!(matches!(
            ControllerSelection::parse("  "),
            Err(ControllerSelectionError::EmptySelection)
        ));
    }

    #[test]
    fn controller_catalog_exports_exact_structural_crds() {
        use std::collections::BTreeSet;

        let crds = controller_crds();
        assert_eq!(crds.len(), CONTROLLER_COUNT);
        for (crd, controller) in crds.iter().zip(ControllerKind::ALL) {
            assert_eq!(crd.spec.group, "citus.ai-blaise.io");
            assert_eq!(crd.spec.scope, "Namespaced");
            assert_eq!(crd.spec.names.kind, controller.resource_kind());
            assert_eq!(crd.spec.names.plural, controller.resource_plural());
            assert_eq!(
                crd.metadata.name.as_deref(),
                Some(format!("{}.citus.ai-blaise.io", controller.resource_plural()).as_str())
            );
            assert_eq!(crd.spec.versions.len(), 1);
            assert_eq!(crd.spec.versions[0].name, "v2");
            assert!(crd.spec.versions[0].served);
            assert!(crd.spec.versions[0].storage);

            let value = serde_json::to_value(crd).expect("serialize controller CRD");
            assert_eq!(
                value
                    .pointer("/spec/versions/0/schema/openAPIV3Schema/type")
                    .and_then(serde_json::Value::as_str),
                Some("object")
            );
            assert!(value
                .pointer("/spec/versions/0/schema/openAPIV3Schema/properties/spec")
                .is_some());
            let expected_status = matches!(
                controller,
                ControllerKind::CitusCluster
                    | ControllerKind::Migration
                    | ControllerKind::Hypertable
                    | ControllerKind::ScheduledRepack
                    | ControllerKind::ConflictPolicy
                    | ControllerKind::Sidecar
            );
            assert_eq!(
                value
                    .pointer("/spec/versions/0/subresources/status")
                    .is_some(),
                expected_status,
                "unexpected status subresource for {}",
                controller.resource_kind()
            );
            assert_eq!(
                value
                    .pointer("/spec/versions/0/schema/openAPIV3Schema/properties/status/type")
                    .and_then(serde_json::Value::as_str),
                expected_status.then_some("object"),
                "unexpected status schema for {}",
                controller.resource_kind()
            );
            if expected_status {
                let actual_fields = value
                    .pointer("/spec/versions/0/schema/openAPIV3Schema/properties/status/properties")
                    .and_then(serde_json::Value::as_object)
                    .expect("status properties")
                    .keys()
                    .map(String::as_str)
                    .collect::<BTreeSet<_>>();
                let expected_fields = expected_status_fields(controller)
                    .iter()
                    .copied()
                    .collect::<BTreeSet<_>>();
                assert_eq!(
                    actual_fields,
                    expected_fields,
                    "status fields changed for {}",
                    controller.resource_kind()
                );
            }
        }
    }

    fn expected_status_fields(controller: ControllerKind) -> &'static [&'static str] {
        match controller {
            ControllerKind::CitusCluster => &[
                "appliedSpecHash",
                "bootstrapJob",
                "cnpgClusters",
                "conditions",
                "expectedExtensions",
                "lastError",
                "nodeConninfo",
                "observedGeneration",
                "phase",
            ],
            ControllerKind::Migration => &["applySteps", "phase", "targetPhase"],
            ControllerKind::Hypertable => &[
                "appliedStepCount",
                "applyStepCount",
                "conditions",
                "lastAppliedSqlHash",
                "lastAppliedUnixSeconds",
                "lastError",
                "observedGeneration",
                "phase",
                "skippedStepCount",
                "sqlPlanCount",
                "table",
            ],
            ControllerKind::ScheduledRepack => &["applySteps", "bloatEstimateSql", "jobName"],
            ControllerKind::ConflictPolicy => {
                &["applySteps", "conflictClass", "policyName", "resolution"]
            }
            ControllerKind::Sidecar => &["deploymentName", "metricsUrl", "readyz", "serviceName"],
            _ => &[],
        }
    }

    #[test]
    fn every_controller_terminal_result_is_an_error() {
        for controller in ControllerKind::ALL {
            assert!(matches!(
                terminal_controller_result(controller, Ok(())),
                Err(ServeError::ControllerExited(name)) if name == controller.selector_name()
            ));
        }
        assert!(matches!(
            terminal_controller_result(
                ControllerKind::Backup,
                Err(ControllerError::InvalidSpec("invalid".to_string()))
            ),
            Err(ServeError::ControllerFailed {
                controller: "backup",
                ..
            })
        ));
    }
}
