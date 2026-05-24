// FEATURE: MR1
// FEATURE: MR3
// FEATURE: MR4
// FEATURE: MR8

use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;

use crate::crds::region::{RegionSpec, RegionSpecError};
use crate::crds::shard_group::ShardGroupSpec;

const TABLESPACE_BASE_PATH: &str = "/var/lib/postgresql/tablespaces";
const NODE_AFFINITY_KEY: &str = "topology.kubernetes.io/zone";
const CNPG_LEADER_AFFINITY_KEY: &str = "ai-blaise.com/leader-region";

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct RegionReconcilePlan {
    pub spec: RegionSpec,
    pub tablespace_path: String,
    pub steps: Vec<RegionApplyStep>,
}

impl TryFrom<&RegionSpec> for RegionReconcilePlan {
    type Error = RegionReconcileError;

    fn try_from(spec: &RegionSpec) -> Result<Self, Self::Error> {
        spec.validate()?;
        let tablespace_path = format!(
            "{TABLESPACE_BASE_PATH}/{}",
            sanitize_path(&spec.tablespace_name)
        );
        let steps = build_region_apply_steps(spec, &tablespace_path);
        Ok(Self {
            spec: spec.clone(),
            tablespace_path,
            steps,
        })
    }
}

impl RegionReconcilePlan {
    pub fn sql_script(&self) -> String {
        self.steps
            .iter()
            .filter(|step| {
                matches!(
                    step.kind,
                    RegionApplyStepKind::Sql | RegionApplyStepKind::SqlPreflight
                )
            })
            .map(|step| step.payload.trim_end().to_string())
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub fn sql_step_count(&self) -> usize {
        self.steps
            .iter()
            .filter(|step| {
                matches!(
                    step.kind,
                    RegionApplyStepKind::Sql | RegionApplyStepKind::SqlPreflight
                )
            })
            .count()
    }

    pub fn node_affinity_label(&self) -> String {
        format!("{NODE_AFFINITY_KEY}={}", self.spec.kubernetes_zone)
    }

    pub fn leader_affinity_label(&self) -> Option<String> {
        if self.spec.leader_pinned {
            Some(format!("{CNPG_LEADER_AFFINITY_KEY}={}", self.spec.name))
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct RegionalRowPlacementSpec {
    pub table: String,
    pub region_column: String,
    pub distribution_column: String,
    pub allowed_regions: Vec<String>,
    pub min_region_replicas: u32,
}

impl RegionalRowPlacementSpec {
    pub fn validate(&self) -> Result<(), RegionReconcileError> {
        validate_schema_qualified_table("placement.table", &self.table)?;
        validate_sql_identifier("placement.region_column", &self.region_column)?;
        validate_sql_identifier("placement.distribution_column", &self.distribution_column)?;
        if self.region_column == self.distribution_column {
            return Err(RegionReconcileError::AmbiguousPlacementKey);
        }
        if self.allowed_regions.is_empty() {
            return Err(RegionReconcileError::MissingPlacementRegions);
        }
        if self.min_region_replicas == 0 {
            return Err(RegionReconcileError::InvalidPlacementReplicaCount);
        }
        if self.min_region_replicas as usize > self.allowed_regions.len() {
            return Err(RegionReconcileError::PlacementReplicaCountExceedsRegions);
        }
        validate_distinct_region_names(&self.allowed_regions)?;
        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct RegionalRowPlacementPlan {
    pub spec: RegionalRowPlacementSpec,
    pub steps: Vec<RegionalRowPlacementStep>,
}

impl RegionalRowPlacementPlan {
    pub fn new(
        spec: &RegionalRowPlacementSpec,
        regions: &[RegionSpec],
        shard_group: &ShardGroupSpec,
    ) -> Result<Self, RegionReconcileError> {
        spec.validate()?;
        validate_region_inventory(regions)?;
        validate_placement_regions_declared(&spec.allowed_regions, regions)?;
        validate_shard_group_for_placement(spec, shard_group)?;

        Ok(Self {
            spec: spec.clone(),
            steps: vec![
                RegionalRowPlacementStep::new(
                    "validate_region_prefix_column",
                    "MR3",
                    format!("column={}", spec.region_column),
                    true,
                ),
                RegionalRowPlacementStep::new(
                    "verify_distribution_key",
                    "MR3",
                    format!(
                        "table={};distribution_column={}",
                        spec.table, spec.distribution_column
                    ),
                    true,
                ),
                RegionalRowPlacementStep::new(
                    "verify_allowed_regions",
                    "MR3",
                    format!("regions={}", spec.allowed_regions.join(",")),
                    true,
                ),
                RegionalRowPlacementStep::new(
                    "require_region_topology_spread",
                    "MR3",
                    "topology_key=topology.kubernetes.io/region;max_skew=1".to_string(),
                    true,
                ),
            ],
        })
    }

    pub fn step_count(&self) -> usize {
        self.steps.len()
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct RegionalRowPlacementStep {
    pub name: String,
    pub feature_id: String,
    pub payload: String,
    pub idempotent: bool,
}

impl RegionalRowPlacementStep {
    fn new(
        name: impl Into<String>,
        feature_id: impl Into<String>,
        payload: impl Into<String>,
        idempotent: bool,
    ) -> Self {
        Self {
            name: name.into(),
            feature_id: feature_id.into(),
            payload: payload.into(),
            idempotent,
        }
    }
}

fn build_region_apply_steps(spec: &RegionSpec, tablespace_path: &str) -> Vec<RegionApplyStep> {
    let mut steps = Vec::new();

    steps.push(RegionApplyStep::sql_preflight(
        "inspect_region_tablespace",
        "MR4",
        format!(
            "SELECT spcname, pg_tablespace_location(oid) AS location FROM pg_tablespace WHERE spcname = {name};",
            name = sql_literal(&spec.tablespace_name)
        ),
        true,
    ));

    steps.push(RegionApplyStep::sql(
        "create_region_tablespace_if_missing",
        "MR4",
        format!(
            "CREATE TABLESPACE {ident} LOCATION {location};",
            ident = quote_identifier(&spec.tablespace_name),
            location = sql_literal(tablespace_path)
        ),
        false,
    ));

    steps.push(RegionApplyStep::node_affinity(
        "set_region_node_affinity",
        "MR1",
        format!("{NODE_AFFINITY_KEY}={}", spec.kubernetes_zone),
        true,
    ));

    if spec.leader_pinned {
        steps.push(RegionApplyStep::cnpg_leader_pin(
            "pin_cnpg_leader",
            "MR8",
            format!("{CNPG_LEADER_AFFINITY_KEY}={}", spec.name),
            true,
        ));
    }

    steps
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct RegionApplyStep {
    pub name: String,
    pub feature_id: String,
    pub kind: RegionApplyStepKind,
    pub payload: String,
    pub idempotent: bool,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum RegionApplyStepKind {
    SqlPreflight,
    Sql,
    NodeAffinity,
    CnpgLeaderPin,
}

impl RegionApplyStep {
    fn sql_preflight(
        name: impl Into<String>,
        feature_id: impl Into<String>,
        sql: impl Into<String>,
        idempotent: bool,
    ) -> Self {
        Self {
            name: name.into(),
            feature_id: feature_id.into(),
            kind: RegionApplyStepKind::SqlPreflight,
            payload: sql.into(),
            idempotent,
        }
    }

    fn sql(
        name: impl Into<String>,
        feature_id: impl Into<String>,
        sql: impl Into<String>,
        idempotent: bool,
    ) -> Self {
        Self {
            name: name.into(),
            feature_id: feature_id.into(),
            kind: RegionApplyStepKind::Sql,
            payload: sql.into(),
            idempotent,
        }
    }

    fn node_affinity(
        name: impl Into<String>,
        feature_id: impl Into<String>,
        label: impl Into<String>,
        idempotent: bool,
    ) -> Self {
        Self {
            name: name.into(),
            feature_id: feature_id.into(),
            kind: RegionApplyStepKind::NodeAffinity,
            payload: label.into(),
            idempotent,
        }
    }

    fn cnpg_leader_pin(
        name: impl Into<String>,
        feature_id: impl Into<String>,
        label: impl Into<String>,
        idempotent: bool,
    ) -> Self {
        Self {
            name: name.into(),
            feature_id: feature_id.into(),
            kind: RegionApplyStepKind::CnpgLeaderPin,
            payload: label.into(),
            idempotent,
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum RegionReconcileError {
    InvalidSpec(RegionSpecError),
    AmbiguousPlacementKey,
    DuplicatePlacementRegion(String),
    DuplicateRegionInventory(String),
    InvalidPlacementReplicaCount,
    InvalidPlacementTable(&'static str),
    InvalidPlacementIdentifier(&'static str),
    MissingPlacementRegions,
    PlacementRegionUndeclared(String),
    PlacementReplicaCountExceedsRegions,
    ShardGroupDistributionMismatch {
        expected: String,
        actual: String,
    },
    ShardGroupTableMismatch {
        expected: String,
        actual: String,
    },
    ShardGroupMissingRegionTopology,
    ShardGroupRegionSkewTooLoose(u32),
    ShardGroupReplicationTooLow {
        replication_factor: u32,
        required: u32,
    },
}

impl fmt::Display for RegionReconcileError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSpec(error) => write!(formatter, "{error}"),
            Self::AmbiguousPlacementKey => write!(formatter, "region_column and distribution_column must differ"),
            Self::DuplicatePlacementRegion(region) => write!(formatter, "placement region {region} appears more than once"),
            Self::DuplicateRegionInventory(region) => write!(formatter, "Region inventory contains duplicate region {region}"),
            Self::InvalidPlacementReplicaCount => write!(formatter, "min_region_replicas must be greater than zero"),
            Self::InvalidPlacementTable(field) => write!(formatter, "{field} must be a schema-qualified SQL table name"),
            Self::InvalidPlacementIdentifier(field) => write!(formatter, "{field} must be a safe SQL identifier"),
            Self::MissingPlacementRegions => write!(formatter, "allowed_regions must not be empty"),
            Self::PlacementRegionUndeclared(region) => write!(formatter, "placement region {region} has no Region CR"),
            Self::PlacementReplicaCountExceedsRegions => write!(formatter, "min_region_replicas cannot exceed allowed_regions"),
            Self::ShardGroupDistributionMismatch { expected, actual } => write!(formatter, "ShardGroup distribution column {actual} does not match placement distribution column {expected}"),
            Self::ShardGroupTableMismatch { expected, actual } => write!(formatter, "ShardGroup table {actual} does not match placement table {expected}"),
            Self::ShardGroupMissingRegionTopology => write!(formatter, "ShardGroup must include topology.kubernetes.io/region placement policy"),
            Self::ShardGroupRegionSkewTooLoose(max_skew) => write!(formatter, "ShardGroup region max_skew={max_skew} is too loose; require max_skew=1"),
            Self::ShardGroupReplicationTooLow { replication_factor, required } => write!(formatter, "ShardGroup replication_factor={replication_factor} is below min_region_replicas={required}"),
        }
    }
}

impl Error for RegionReconcileError {}

impl From<RegionSpecError> for RegionReconcileError {
    fn from(error: RegionSpecError) -> Self {
        Self::InvalidSpec(error)
    }
}

fn validate_schema_qualified_table(
    field: &'static str,
    value: &str,
) -> Result<(), RegionReconcileError> {
    let parts = value.split('.').collect::<Vec<_>>();
    if parts.len() != 2
        || parts
            .iter()
            .any(|part| validate_sql_identifier(field, part).is_err())
    {
        return Err(RegionReconcileError::InvalidPlacementTable(field));
    }
    Ok(())
}

fn validate_sql_identifier(field: &'static str, value: &str) -> Result<(), RegionReconcileError> {
    let bytes = value.as_bytes();
    if bytes.is_empty()
        || bytes.len() > 63
        || !(bytes[0].is_ascii_lowercase() || bytes[0] == b'_')
        || !bytes
            .iter()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || *byte == b'_')
    {
        return Err(RegionReconcileError::InvalidPlacementIdentifier(field));
    }
    Ok(())
}

fn validate_distinct_region_names(regions: &[String]) -> Result<(), RegionReconcileError> {
    let mut seen = BTreeSet::new();
    for region in regions {
        if !seen.insert(region) {
            return Err(RegionReconcileError::DuplicatePlacementRegion(
                region.clone(),
            ));
        }
    }
    Ok(())
}

fn validate_region_inventory(regions: &[RegionSpec]) -> Result<(), RegionReconcileError> {
    let mut seen = BTreeSet::new();
    for region in regions {
        region.validate()?;
        if !seen.insert(&region.name) {
            return Err(RegionReconcileError::DuplicateRegionInventory(
                region.name.clone(),
            ));
        }
    }
    Ok(())
}

fn validate_placement_regions_declared(
    allowed_regions: &[String],
    regions: &[RegionSpec],
) -> Result<(), RegionReconcileError> {
    for allowed in allowed_regions {
        if !regions.iter().any(|region| &region.name == allowed) {
            return Err(RegionReconcileError::PlacementRegionUndeclared(
                allowed.clone(),
            ));
        }
    }
    Ok(())
}

fn validate_shard_group_for_placement(
    spec: &RegionalRowPlacementSpec,
    shard_group: &ShardGroupSpec,
) -> Result<(), RegionReconcileError> {
    if shard_group.parent_table != spec.table {
        return Err(RegionReconcileError::ShardGroupTableMismatch {
            expected: spec.table.clone(),
            actual: shard_group.parent_table.clone(),
        });
    }
    if shard_group.distribution_column != spec.distribution_column {
        return Err(RegionReconcileError::ShardGroupDistributionMismatch {
            expected: spec.distribution_column.clone(),
            actual: shard_group.distribution_column.clone(),
        });
    }
    if shard_group.replication_factor < spec.min_region_replicas {
        return Err(RegionReconcileError::ShardGroupReplicationTooLow {
            replication_factor: shard_group.replication_factor,
            required: spec.min_region_replicas,
        });
    }
    let Some(policy) = shard_group
        .placement_policy
        .iter()
        .find(|policy| policy.topology_key == "topology.kubernetes.io/region")
    else {
        return Err(RegionReconcileError::ShardGroupMissingRegionTopology);
    };
    if policy.max_skew > 1 {
        return Err(RegionReconcileError::ShardGroupRegionSkewTooLoose(
            policy.max_skew,
        ));
    }
    Ok(())
}

fn sql_literal(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

fn quote_identifier(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\""))
}

fn sanitize_path(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for byte in value.bytes() {
        let ch = byte as char;
        let lower = ch.to_ascii_lowercase();
        if lower.is_ascii_alphanumeric() || lower == '_' || lower == '-' {
            out.push(lower);
        } else {
            out.push('_');
        }
    }
    if out.is_empty() {
        "region".to_string()
    } else {
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crds::shard_group::{PlacementPolicy, ShardGroupSpec, UnsatisfiablePlacementAction};

    fn region(leader_pinned: bool) -> RegionSpec {
        RegionSpec {
            name: "us-east-1".to_string(),
            kubernetes_zone: "us-east-1a".to_string(),
            tablespace_name: "ts_us_east_1".to_string(),
            leader_pinned,
        }
    }

    fn west_region() -> RegionSpec {
        RegionSpec {
            name: "us-west-2".to_string(),
            kubernetes_zone: "us-west-2a".to_string(),
            tablespace_name: "ts_us_west_2".to_string(),
            leader_pinned: false,
        }
    }

    fn placement_spec() -> RegionalRowPlacementSpec {
        RegionalRowPlacementSpec {
            table: "public.orders".to_string(),
            region_column: "region_id".to_string(),
            distribution_column: "tenant_id".to_string(),
            allowed_regions: vec!["us-east-1".to_string(), "us-west-2".to_string()],
            min_region_replicas: 2,
        }
    }

    fn shard_group() -> ShardGroupSpec {
        ShardGroupSpec {
            parent_table: "public.orders".to_string(),
            distribution_column: "tenant_id".to_string(),
            num_shards: 32,
            colocation_group: Some("orders".to_string()),
            replication_factor: 3,
            placement_policy: vec![PlacementPolicy {
                topology_key: "topology.kubernetes.io/region".to_string(),
                max_skew: 1,
                when_unsatisfiable: UnsatisfiablePlacementAction::DoNotSchedule,
            }],
        }
    }

    #[test]
    fn regional_row_placement_plan_emits_deterministic_contract_steps() {
        let plan = RegionalRowPlacementPlan::new(
            &placement_spec(),
            &[region(true), west_region()],
            &shard_group(),
        )
        .expect("valid placement plan");

        assert_eq!(plan.step_count(), 4);
        assert_eq!(plan.steps[0].name, "validate_region_prefix_column");
        assert!(plan.steps[1]
            .payload
            .contains("distribution_column=tenant_id"));
        assert!(plan.steps[2].payload.contains("us-east-1,us-west-2"));
        assert!(plan.steps[3]
            .payload
            .contains("topology.kubernetes.io/region"));
    }

    #[test]
    fn regional_row_placement_fails_closed_for_bad_inventory_and_shard_policy() {
        let result = RegionalRowPlacementPlan::new(
            &placement_spec(),
            &[region(true), region(false)],
            &shard_group(),
        );
        assert!(matches!(
            result,
            Err(RegionReconcileError::DuplicateRegionInventory(region)) if region == "us-east-1"
        ));

        let mut bad_shard_group = shard_group();
        bad_shard_group.placement_policy[0].max_skew = 2;
        let result = RegionalRowPlacementPlan::new(
            &placement_spec(),
            &[region(true), west_region()],
            &bad_shard_group,
        );
        assert_eq!(
            result,
            Err(RegionReconcileError::ShardGroupRegionSkewTooLoose(2))
        );
    }

    #[test]
    fn regional_row_placement_rejects_ambiguous_keys_and_undeclared_region() {
        let mut spec = placement_spec();
        spec.region_column = "tenant_id".to_string();
        assert_eq!(
            RegionalRowPlacementPlan::new(&spec, &[region(true), west_region()], &shard_group()),
            Err(RegionReconcileError::AmbiguousPlacementKey)
        );

        let mut spec = placement_spec();
        spec.allowed_regions.push("eu-central-1".to_string());
        let result =
            RegionalRowPlacementPlan::new(&spec, &[region(true), west_region()], &shard_group());
        assert!(matches!(
            result,
            Err(RegionReconcileError::PlacementRegionUndeclared(region)) if region == "eu-central-1"
        ));
    }

    #[test]
    fn reconcile_plan_with_leader_pin_emits_four_steps() {
        let plan = RegionReconcilePlan::try_from(&region(true)).expect("valid plan");

        assert_eq!(plan.steps.len(), 4);
        assert_eq!(plan.sql_step_count(), 2);
        assert_eq!(plan.steps[0].kind, RegionApplyStepKind::SqlPreflight);
        assert_eq!(plan.steps[1].feature_id, "MR4");
        assert!(!plan.steps[1].idempotent);
        assert_eq!(plan.steps[2].kind, RegionApplyStepKind::NodeAffinity);
        assert_eq!(plan.steps[3].feature_id, "MR8");
        assert!(plan
            .sql_script()
            .contains("CREATE TABLESPACE \"ts_us_east_1\""));
        assert!(plan
            .sql_script()
            .contains("LOCATION '/var/lib/postgresql/tablespaces/ts_us_east_1'"));
        assert_eq!(
            plan.node_affinity_label(),
            "topology.kubernetes.io/zone=us-east-1a"
        );
        assert_eq!(
            plan.leader_affinity_label().as_deref(),
            Some("ai-blaise.com/leader-region=us-east-1")
        );
    }

    #[test]
    fn reconcile_plan_without_leader_pin_skips_cnpg_step() {
        let plan = RegionReconcilePlan::try_from(&region(false)).expect("valid plan");

        assert_eq!(plan.steps.len(), 3);
        assert!(plan
            .steps
            .iter()
            .all(|step| step.kind != RegionApplyStepKind::CnpgLeaderPin));
        assert_eq!(plan.leader_affinity_label(), None);
    }

    #[test]
    fn reconcile_plan_rejects_empty_zone() {
        let spec = RegionSpec {
            name: "us-east-1".to_string(),
            kubernetes_zone: " ".to_string(),
            tablespace_name: "ts_us_east_1".to_string(),
            leader_pinned: false,
        };

        assert!(matches!(
            RegionReconcilePlan::try_from(&spec),
            Err(RegionReconcileError::InvalidSpec(_))
        ));
    }
}
