// CRD module roots. Each resource lives in its own directory with the
// implementation pinned to `v1alpha1`, served alongside an explicit (currently
// identity-mapped) `v1beta1` so the conversion-webhook pipeline can ship a
// real upgrade path without churning the schema files later.

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

/// Canonical Kubernetes API group for every ai-blaise Citus operator CRD.
pub const CRD_GROUP: &str = "ai-blaise.com";

/// Version that controllers store in etcd and that the operator deserializes
/// into Rust spec types. Conversion webhooks normalize every served version
/// down to this value before the operator sees it.
pub const STORAGE_VERSION: &str = "v1alpha1";

/// Versions currently served by the API server. The conversion webhook routes
/// between them; the storage version is the canonical home for persisted
/// objects.
pub const SERVED_VERSIONS: &[&str] = &["v1alpha1", "v1beta1"];

/// Identifier of an ai-blaise Citus CRD as advertised on the wire. The list of
/// constants below maps 1:1 to the YAML bundle shipped from
/// `ai-blaise/command-center` and to the 17 CRD modules in this directory.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum CrdKind {
    CitusCluster,
    ShardGroup,
    Hypertable,
    Branch,
    Vectorizer,
    Sidecar,
    Migration,
    ConflictPolicy,
    Tenant,
    Region,
    SurvivalGoal,
    Backup,
    Federation,
    SearchIndex,
    Webhook,
    Function,
    ScheduledRepack,
}

/// Metadata for one CRD as advertised on the wire.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct CrdMetadata {
    pub kind: CrdKind,
    pub kind_name: &'static str,
    pub plural: &'static str,
    pub singular: &'static str,
    pub module_name: &'static str,
}

impl CrdMetadata {
    /// Fully-qualified Custom Resource name used in the bundle:
    /// `<plural>.<group>`.
    pub fn fully_qualified_name(&self) -> String {
        format!("{}.{}", self.plural, CRD_GROUP)
    }
}

/// Canonical ordering of the 17 CRDs shipped by the operator. The order
/// matches the YAML bundle in
/// `command-center/helm/charts/citus-cluster/crds/ai-blaise-citus-crds.yaml`
/// so smoke tests that diff against either side stay deterministic.
pub const CRD_CATALOG: &[CrdMetadata] = &[
    CrdMetadata {
        kind: CrdKind::CitusCluster,
        kind_name: "CitusCluster",
        plural: "citusclusters",
        singular: "cituscluster",
        module_name: "citus_cluster",
    },
    CrdMetadata {
        kind: CrdKind::ShardGroup,
        kind_name: "ShardGroup",
        plural: "shardgroups",
        singular: "shardgroup",
        module_name: "shard_group",
    },
    CrdMetadata {
        kind: CrdKind::Hypertable,
        kind_name: "Hypertable",
        plural: "hypertables",
        singular: "hypertable",
        module_name: "hypertable",
    },
    CrdMetadata {
        kind: CrdKind::Branch,
        kind_name: "Branch",
        plural: "branches",
        singular: "branch",
        module_name: "branch",
    },
    CrdMetadata {
        kind: CrdKind::Vectorizer,
        kind_name: "Vectorizer",
        plural: "vectorizers",
        singular: "vectorizer",
        module_name: "vectorizer",
    },
    CrdMetadata {
        kind: CrdKind::Sidecar,
        kind_name: "Sidecar",
        plural: "sidecars",
        singular: "sidecar",
        module_name: "sidecar",
    },
    CrdMetadata {
        kind: CrdKind::Migration,
        kind_name: "Migration",
        plural: "migrations",
        singular: "migration",
        module_name: "migration",
    },
    CrdMetadata {
        kind: CrdKind::ConflictPolicy,
        kind_name: "ConflictPolicy",
        plural: "conflictpolicies",
        singular: "conflictpolicy",
        module_name: "conflict_policy",
    },
    CrdMetadata {
        kind: CrdKind::Tenant,
        kind_name: "Tenant",
        plural: "tenants",
        singular: "tenant",
        module_name: "tenant",
    },
    CrdMetadata {
        kind: CrdKind::Region,
        kind_name: "Region",
        plural: "regions",
        singular: "region",
        module_name: "region",
    },
    CrdMetadata {
        kind: CrdKind::SurvivalGoal,
        kind_name: "SurvivalGoal",
        plural: "survivalgoals",
        singular: "survivalgoal",
        module_name: "survival_goal",
    },
    CrdMetadata {
        kind: CrdKind::Backup,
        kind_name: "Backup",
        plural: "backups",
        singular: "backup",
        module_name: "backup",
    },
    CrdMetadata {
        kind: CrdKind::Federation,
        kind_name: "Federation",
        plural: "federations",
        singular: "federation",
        module_name: "federation",
    },
    CrdMetadata {
        kind: CrdKind::SearchIndex,
        kind_name: "SearchIndex",
        plural: "searchindexes",
        singular: "searchindex",
        module_name: "search_index",
    },
    CrdMetadata {
        kind: CrdKind::Webhook,
        kind_name: "Webhook",
        plural: "webhooks",
        singular: "webhook",
        module_name: "webhook",
    },
    CrdMetadata {
        kind: CrdKind::Function,
        kind_name: "Function",
        plural: "functions",
        singular: "function",
        module_name: "function",
    },
    CrdMetadata {
        kind: CrdKind::ScheduledRepack,
        kind_name: "ScheduledRepack",
        plural: "scheduledrepacks",
        singular: "scheduledrepack",
        module_name: "scheduledrepack",
    },
];

/// Look up a CRD by its `kind` name. Returns `None` if the kind is not a
/// known ai-blaise CRD.
pub fn crd_for_kind(kind_name: &str) -> Option<CrdMetadata> {
    CRD_CATALOG
        .iter()
        .copied()
        .find(|metadata| metadata.kind_name == kind_name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crd_catalog_lists_seventeen_resources() {
        assert_eq!(CRD_CATALOG.len(), 17);
    }

    #[test]
    fn crd_catalog_kind_names_are_unique() {
        let mut seen = Vec::new();
        for metadata in CRD_CATALOG {
            assert!(
                !seen.contains(&metadata.kind_name),
                "duplicate kind name: {}",
                metadata.kind_name
            );
            seen.push(metadata.kind_name);
        }
    }

    #[test]
    fn crd_catalog_plurals_match_yaml_bundle() {
        let expected = [
            "citusclusters",
            "shardgroups",
            "hypertables",
            "branches",
            "vectorizers",
            "sidecars",
            "migrations",
            "conflictpolicies",
            "tenants",
            "regions",
            "survivalgoals",
            "backups",
            "federations",
            "searchindexes",
            "webhooks",
            "functions",
            "scheduledrepacks",
        ];
        for (metadata, expected_plural) in CRD_CATALOG.iter().zip(expected.iter()) {
            assert_eq!(
                &metadata.plural, expected_plural,
                "plural mismatch for {}",
                metadata.kind_name
            );
        }
    }

    #[test]
    fn fully_qualified_name_uses_group() {
        let cituscluster = crd_for_kind("CitusCluster").expect("CitusCluster registered");
        assert_eq!(
            cituscluster.fully_qualified_name(),
            "citusclusters.ai-blaise.com"
        );
    }

    #[test]
    fn served_versions_include_storage_version() {
        assert!(SERVED_VERSIONS.contains(&STORAGE_VERSION));
        assert_eq!(SERVED_VERSIONS, &["v1alpha1", "v1beta1"]);
    }
}
