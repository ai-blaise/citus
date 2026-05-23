// FEATURE: MR1
// FEATURE: MR4
// FEATURE: MR8

use std::error::Error;
use std::fmt;

use crate::crds::region::{RegionSpec, RegionSpecError};

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
}

impl fmt::Display for RegionReconcileError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSpec(error) => write!(formatter, "{error}"),
        }
    }
}

impl Error for RegionReconcileError {}

impl From<RegionSpecError> for RegionReconcileError {
    fn from(error: RegionSpecError) -> Self {
        Self::InvalidSpec(error)
    }
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

    fn region(leader_pinned: bool) -> RegionSpec {
        RegionSpec {
            name: "us-east-1".to_string(),
            kubernetes_zone: "us-east-1a".to_string(),
            tablespace_name: "ts_us_east_1".to_string(),
            leader_pinned,
        }
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
