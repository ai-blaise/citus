// FEATURE: O16

//! Conversion-webhook plumbing for the ai-blaise Citus CRDs.
//!
//! Every CRD currently serves both `v1alpha1` (storage version) and
//! `v1beta1` (forward placeholder, today identical to v1alpha1). The handlers
//! in this module implement the typed half of a Kubernetes conversion webhook
//! per resource: callers hand us a strongly-typed `*Spec` value plus the
//! source and target API versions, and we return the spec re-cast to the
//! target version (or an error if the requested conversion is not supported).
//!
//! Today every handler is an identity mapping. When v1beta1 actually diverges
//! the divergent fields will be added to the corresponding handler in
//! `operator/src/conversion/<resource>.rs` and the round-trip identity test
//! will move to a kind-specific assertion.

pub mod backup;
pub mod branch;
pub mod citus_cluster;
pub mod conflict_policy;
pub mod federation;
pub mod function;
pub mod hypertable;
pub mod migration;
pub mod region;
pub mod scheduled_repack;
pub mod search_index;
pub mod shard_group;
pub mod sidecar;
pub mod survival_goal;
pub mod tenant;
pub mod vectorizer;
pub mod webhook;

use std::error::Error;
use std::fmt;

use crate::crds::{CrdKind, CRD_CATALOG, CRD_GROUP, SERVED_VERSIONS, STORAGE_VERSION};

/// The Kubernetes `Webhook` conversion strategy advertised by the CRD bundle.
/// Kept here as a single constant so the YAML manifest and the Rust handler
/// agree on the path served by the operator.
pub const CONVERSION_WEBHOOK_PATH: &str = "/convert";

/// Default HTTPS port the operator exposes the conversion webhook on. The
/// operator's `serve` mode binds the probe and webhook traffic to the same
/// listener; the cert-manager `Certificate` shipped from command-center mints
/// the serving cert for this port.
pub const CONVERSION_WEBHOOK_PORT: u16 = 8443;

/// API versions accepted by the conversion webhook. Mirrors
/// `crds::SERVED_VERSIONS` and is reasserted on the round-trip path so a
/// future divergence between the two lists is caught by the operator tests
/// rather than at deploy time.
pub const SUPPORTED_VERSIONS: &[&str] = SERVED_VERSIONS;

/// Errors produced when the webhook is asked to perform an unsupported
/// conversion or receives a payload whose embedded kind does not match the
/// request.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum ConversionError {
    /// The source version is not one this operator serves.
    UnsupportedSourceVersion(String),
    /// The target version is not one this operator serves.
    UnsupportedTargetVersion(String),
    /// The `kind` field of the request does not match the typed payload.
    KindMismatch {
        expected: &'static str,
        actual: String,
    },
    /// The request asked us to convert a CRD we do not serve.
    UnknownKind(String),
    /// The request asked us to convert between versions of different kinds.
    PayloadKindMismatch {
        request_kind: String,
        payload_kind: &'static str,
    },
}

impl fmt::Display for ConversionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedSourceVersion(version) => {
                write!(formatter, "unsupported source apiVersion: {version}")
            }
            Self::UnsupportedTargetVersion(version) => {
                write!(formatter, "unsupported target apiVersion: {version}")
            }
            Self::KindMismatch { expected, actual } => {
                write!(
                    formatter,
                    "payload kind {actual:?} does not match handler kind {expected:?}"
                )
            }
            Self::UnknownKind(kind) => {
                write!(formatter, "unknown CRD kind: {kind}")
            }
            Self::PayloadKindMismatch {
                request_kind,
                payload_kind,
            } => write!(
                formatter,
                "request kind {request_kind} does not match payload kind {payload_kind}"
            ),
        }
    }
}

impl Error for ConversionError {}

/// Typed payload carried by a conversion request. The operator never carries
/// `serde_json::Value` here because we want the type checker to enforce that
/// every code path stays in sync with the Rust spec types. The HTTPS webhook
/// adapter (admission-style HTTP handler) is the layer that converts wire
/// JSON to/from these variants; that adapter lives alongside the admission
/// webhook implementation and is wired in `operator/src/main.rs` when the
/// `serve` subcommand is invoked.
#[derive(Debug, Clone, PartialEq)]
pub enum ConversionPayload {
    CitusCluster(crate::crds::citus_cluster::v1alpha1::CitusClusterSpec),
    ShardGroup(crate::crds::shard_group::v1alpha1::ShardGroupSpec),
    Hypertable(crate::crds::hypertable::v1alpha1::HypertableSpec),
    Branch(crate::crds::branch::v1alpha1::BranchSpec),
    Vectorizer(crate::crds::vectorizer::v1alpha1::VectorizerSpec),
    Sidecar(crate::crds::sidecar::v1alpha1::SidecarDeploymentSpec),
    Migration(crate::crds::migration::v1alpha1::MigrationSpec),
    ConflictPolicy(crate::crds::conflict_policy::v1alpha1::ConflictPolicySpec),
    Tenant(crate::crds::tenant::v1alpha1::TenantSpec),
    Region(crate::crds::region::v1alpha1::RegionSpec),
    SurvivalGoal(crate::crds::survival_goal::v1alpha1::SurvivalGoalSpec),
    Backup(crate::crds::backup::v1alpha1::BackupSpec),
    Federation(crate::crds::federation::v1alpha1::FederationSpec),
    SearchIndex(crate::crds::search_index::v1alpha1::SearchIndexSpec),
    Webhook(crate::crds::webhook::v1alpha1::WebhookSpec),
    Function(crate::crds::function::v1alpha1::FunctionSpec),
    ScheduledRepack(crate::crds::scheduled_repack::v1alpha1::ScheduledRepackSpec),
}

impl ConversionPayload {
    /// CRD kind the payload represents. Used to cross-check the kind field
    /// the API server placed alongside the typed object.
    pub fn kind(&self) -> CrdKind {
        match self {
            Self::CitusCluster(_) => CrdKind::CitusCluster,
            Self::ShardGroup(_) => CrdKind::ShardGroup,
            Self::Hypertable(_) => CrdKind::Hypertable,
            Self::Branch(_) => CrdKind::Branch,
            Self::Vectorizer(_) => CrdKind::Vectorizer,
            Self::Sidecar(_) => CrdKind::Sidecar,
            Self::Migration(_) => CrdKind::Migration,
            Self::ConflictPolicy(_) => CrdKind::ConflictPolicy,
            Self::Tenant(_) => CrdKind::Tenant,
            Self::Region(_) => CrdKind::Region,
            Self::SurvivalGoal(_) => CrdKind::SurvivalGoal,
            Self::Backup(_) => CrdKind::Backup,
            Self::Federation(_) => CrdKind::Federation,
            Self::SearchIndex(_) => CrdKind::SearchIndex,
            Self::Webhook(_) => CrdKind::Webhook,
            Self::Function(_) => CrdKind::Function,
            Self::ScheduledRepack(_) => CrdKind::ScheduledRepack,
        }
    }

    /// Name of the kind as it appears on the wire.
    pub fn kind_name(&self) -> &'static str {
        let kind = self.kind();
        CRD_CATALOG
            .iter()
            .find(|metadata| metadata.kind == kind)
            .expect("every ConversionPayload variant has a CRD_CATALOG entry")
            .kind_name
    }
}

/// Request as the operator sees it after decoding the conversion-webhook
/// envelope. The HTTPS adapter is responsible for turning the Kubernetes
/// `ConversionReview` JSON into a `ConversionRequest`; this module keeps the
/// pure conversion logic so tests can exercise it without spinning up a
/// server.
#[derive(Debug, Clone, PartialEq)]
pub struct ConversionRequest {
    pub source_api_version: String,
    pub target_api_version: String,
    pub kind: String,
    pub payload: ConversionPayload,
}

/// Successful conversion result. The `payload` is in `target_api_version` form
/// and ready for the operator (or the API server, when this travels back over
/// the webhook channel) to consume.
#[derive(Debug, Clone, PartialEq)]
pub struct ConversionResponse {
    pub target_api_version: String,
    pub payload: ConversionPayload,
}

/// Convert one payload between two served versions. Today every handler is an
/// identity round-trip; when a v1beta1 schema actually diverges the handler
/// file for that resource will replace the identity body with a typed mapping.
pub fn convert(request: &ConversionRequest) -> Result<ConversionResponse, ConversionError> {
    validate_version(&request.source_api_version, |v| {
        ConversionError::UnsupportedSourceVersion(v)
    })?;
    validate_version(&request.target_api_version, |v| {
        ConversionError::UnsupportedTargetVersion(v)
    })?;

    let payload_kind = request.payload.kind_name();
    if request.kind != payload_kind {
        return Err(ConversionError::PayloadKindMismatch {
            request_kind: request.kind.clone(),
            payload_kind,
        });
    }

    let converted = match &request.payload {
        ConversionPayload::CitusCluster(spec) => {
            ConversionPayload::CitusCluster(citus_cluster::identity(spec))
        }
        ConversionPayload::ShardGroup(spec) => {
            ConversionPayload::ShardGroup(shard_group::identity(spec))
        }
        ConversionPayload::Hypertable(spec) => {
            ConversionPayload::Hypertable(hypertable::identity(spec))
        }
        ConversionPayload::Branch(spec) => ConversionPayload::Branch(branch::identity(spec)),
        ConversionPayload::Vectorizer(spec) => {
            ConversionPayload::Vectorizer(vectorizer::identity(spec))
        }
        ConversionPayload::Sidecar(spec) => ConversionPayload::Sidecar(sidecar::identity(spec)),
        ConversionPayload::Migration(spec) => {
            ConversionPayload::Migration(migration::identity(spec))
        }
        ConversionPayload::ConflictPolicy(spec) => {
            ConversionPayload::ConflictPolicy(conflict_policy::identity(spec))
        }
        ConversionPayload::Tenant(spec) => ConversionPayload::Tenant(tenant::identity(spec)),
        ConversionPayload::Region(spec) => ConversionPayload::Region(region::identity(spec)),
        ConversionPayload::SurvivalGoal(spec) => {
            ConversionPayload::SurvivalGoal(survival_goal::identity(spec))
        }
        ConversionPayload::Backup(spec) => ConversionPayload::Backup(backup::identity(spec)),
        ConversionPayload::Federation(spec) => {
            ConversionPayload::Federation(federation::identity(spec))
        }
        ConversionPayload::SearchIndex(spec) => {
            ConversionPayload::SearchIndex(search_index::identity(spec))
        }
        ConversionPayload::Webhook(spec) => ConversionPayload::Webhook(webhook::identity(spec)),
        ConversionPayload::Function(spec) => ConversionPayload::Function(function::identity(spec)),
        ConversionPayload::ScheduledRepack(spec) => {
            ConversionPayload::ScheduledRepack(scheduled_repack::identity(spec))
        }
    };

    Ok(ConversionResponse {
        target_api_version: request.target_api_version.clone(),
        payload: converted,
    })
}

fn validate_version(
    version: &str,
    error_ctor: impl FnOnce(String) -> ConversionError,
) -> Result<(), ConversionError> {
    if SUPPORTED_VERSIONS.contains(&version) {
        Ok(())
    } else {
        Err(error_ctor(version.to_string()))
    }
}

/// Build the JSON-style conversion-webhook target URL the YAML bundle points
/// at. Kept here so the cert-manager Certificate, the CRD bundle, and the
/// operator agree on a single path. Service DNS is interpolated by the helm
/// chart; we expose only the path here.
pub fn conversion_webhook_url(service_dns: &str) -> String {
    format!("https://{service_dns}:{CONVERSION_WEBHOOK_PORT}{CONVERSION_WEBHOOK_PATH}")
}

/// Convenience: number of distinct CRD kinds the registry knows how to
/// convert. Equals `crds::CRD_CATALOG.len()` (17).
pub fn registered_kind_count() -> usize {
    CRD_CATALOG.len()
}

/// Group the conversion webhook operates on. Mirrored from `crds::CRD_GROUP`.
pub fn webhook_group() -> &'static str {
    CRD_GROUP
}

/// Operator's storage version. Mirrored from `crds::STORAGE_VERSION`.
pub fn storage_version() -> &'static str {
    STORAGE_VERSION
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crds::hypertable::v1alpha1::HypertableSpec;
    use crate::crds::migration::v1alpha1::{MigrationConflictAction, MigrationSpec, MigrationType};

    #[test]
    fn converts_v1alpha1_to_v1beta1_identity() {
        let spec = MigrationSpec {
            migration_type: MigrationType::Pgroll,
            yaml: "operations: []".to_string(),
            on_conflict: MigrationConflictAction::Fail,
        };
        let request = ConversionRequest {
            source_api_version: "v1alpha1".to_string(),
            target_api_version: "v1beta1".to_string(),
            kind: "Migration".to_string(),
            payload: ConversionPayload::Migration(spec.clone()),
        };

        let response = convert(&request).expect("identity conversion succeeds");
        assert_eq!(response.target_api_version, "v1beta1");
        match response.payload {
            ConversionPayload::Migration(round_tripped) => assert_eq!(round_tripped, spec),
            other => panic!("unexpected payload variant: {other:?}"),
        }
    }

    #[test]
    fn converts_v1beta1_back_to_v1alpha1_identity() {
        let spec = HypertableSpec {
            table: "metrics".to_string(),
            time_column: "ts".to_string(),
            distribution_column: "tenant_id".to_string(),
            chunk_time_interval: "1 day".to_string(),
            num_shards: 4,
            compression: None,
            retention: None,
            continuous_aggregates: Vec::new(),
        };
        let request = ConversionRequest {
            source_api_version: "v1beta1".to_string(),
            target_api_version: "v1alpha1".to_string(),
            kind: "Hypertable".to_string(),
            payload: ConversionPayload::Hypertable(spec.clone()),
        };

        let response = convert(&request).expect("identity conversion succeeds");
        assert_eq!(response.target_api_version, "v1alpha1");
        match response.payload {
            ConversionPayload::Hypertable(round_tripped) => assert_eq!(round_tripped, spec),
            other => panic!("unexpected payload variant: {other:?}"),
        }
    }

    #[test]
    fn rejects_unsupported_source_version() {
        let request = ConversionRequest {
            source_api_version: "v0".to_string(),
            target_api_version: "v1beta1".to_string(),
            kind: "Migration".to_string(),
            payload: ConversionPayload::Migration(MigrationSpec {
                migration_type: MigrationType::Pgroll,
                yaml: "x".to_string(),
                on_conflict: MigrationConflictAction::Skip,
            }),
        };

        assert_eq!(
            convert(&request),
            Err(ConversionError::UnsupportedSourceVersion("v0".to_string()))
        );
    }

    #[test]
    fn rejects_unsupported_target_version() {
        let request = ConversionRequest {
            source_api_version: "v1alpha1".to_string(),
            target_api_version: "v2".to_string(),
            kind: "Migration".to_string(),
            payload: ConversionPayload::Migration(MigrationSpec {
                migration_type: MigrationType::Pgroll,
                yaml: "x".to_string(),
                on_conflict: MigrationConflictAction::Skip,
            }),
        };

        assert_eq!(
            convert(&request),
            Err(ConversionError::UnsupportedTargetVersion("v2".to_string()))
        );
    }

    #[test]
    fn rejects_kind_payload_mismatch() {
        let request = ConversionRequest {
            source_api_version: "v1alpha1".to_string(),
            target_api_version: "v1beta1".to_string(),
            kind: "Hypertable".to_string(),
            payload: ConversionPayload::Migration(MigrationSpec {
                migration_type: MigrationType::Pgroll,
                yaml: "x".to_string(),
                on_conflict: MigrationConflictAction::Skip,
            }),
        };

        assert_eq!(
            convert(&request),
            Err(ConversionError::PayloadKindMismatch {
                request_kind: "Hypertable".to_string(),
                payload_kind: "Migration",
            })
        );
    }

    #[test]
    fn registered_kind_count_matches_catalog() {
        assert_eq!(registered_kind_count(), 17);
    }

    #[test]
    fn conversion_webhook_url_uses_constants() {
        assert_eq!(
            conversion_webhook_url("ai-blaise-citus-operator.ai-blaise-system.svc"),
            "https://ai-blaise-citus-operator.ai-blaise-system.svc:8443/convert"
        );
    }
}
