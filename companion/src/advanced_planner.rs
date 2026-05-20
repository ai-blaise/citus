// FEATURE: A10
// FEATURE: A11
// FEATURE: B4
// FEATURE: Edge1
// FEATURE: Edge2
// FEATURE: F3
// FEATURE: F4
// FEATURE: L7
// FEATURE: L10
// FEATURE: M4
// FEATURE: MR3
// FEATURE: MR6
// FEATURE: R3
// FEATURE: R8
// FEATURE: R12
// FEATURE: S1
// FEATURE: S3
// FEATURE: S8
// FEATURE: S12
// FEATURE: Sto2
// FEATURE: T4
// FEATURE: T10
// FEATURE: T11
// FEATURE: T13
// FEATURE: T14
// FEATURE: TS10
// FEATURE: TS11

use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;

pub const ADVANCED_PLANNER_FEATURE_IDS: &[&str] = &[
    "A10", "A11", "B4", "Edge1", "Edge2", "F3", "F4", "L7", "L10", "M4", "MR3", "MR6", "R3", "R8",
    "R12", "S1", "S3", "S8", "S12", "Sto2", "T4", "T10", "T11", "T13", "T14", "TS10", "TS11",
];

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct AdvancedPlannerContract {
    pub surfaces: Vec<PlannerSurface>,
}

impl AdvancedPlannerContract {
    pub fn validate(&self) -> Result<(), AdvancedPlannerError> {
        if self.surfaces.is_empty() {
            return Err(AdvancedPlannerError::MissingRequiredField("surfaces"));
        }

        let mut seen = BTreeSet::new();
        for surface in &self.surfaces {
            surface.validate()?;
            seen.insert(surface.feature_id);
        }

        for feature_id in ADVANCED_PLANNER_FEATURE_IDS {
            if !seen.contains(feature_id) {
                return Err(AdvancedPlannerError::MissingFeature(feature_id));
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PlannerSurface {
    pub feature_id: &'static str,
    pub name: String,
    pub references: Vec<String>,
    pub kind: PlannerSurfaceKind,
}

impl PlannerSurface {
    fn validate(&self) -> Result<(), AdvancedPlannerError> {
        validate_required("surface.feature_id", self.feature_id)?;
        validate_required("surface.name", &self.name)?;
        validate_required_list("surface.references", &self.references)?;
        self.kind.validate()
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum PlannerSurfaceKind {
    Lookup {
        min_partitions: u32,
    },
    BatchTransfer {
        max_batch_rows: u32,
    },
    DistributedSql {
        worker_tasks: u32,
    },
    TransactionState {
        max_open_shards: u32,
    },
    Policy {
        required_inputs: Vec<String>,
    },
    StorageDomain {
        domain_name: String,
        backing_table: String,
    },
    ResearchGuard {
        decision_record: String,
    },
}

impl PlannerSurfaceKind {
    fn validate(&self) -> Result<(), AdvancedPlannerError> {
        match self {
            Self::Lookup { min_partitions } if *min_partitions == 0 => {
                Err(AdvancedPlannerError::InvalidPositive("min_partitions"))
            }
            Self::BatchTransfer { max_batch_rows } if *max_batch_rows == 0 => {
                Err(AdvancedPlannerError::InvalidPositive("max_batch_rows"))
            }
            Self::DistributedSql { worker_tasks } if *worker_tasks == 0 => {
                Err(AdvancedPlannerError::InvalidPositive("worker_tasks"))
            }
            Self::TransactionState { max_open_shards } if *max_open_shards == 0 => {
                Err(AdvancedPlannerError::InvalidPositive("max_open_shards"))
            }
            Self::Policy { required_inputs } => {
                validate_required_list("policy.required_inputs", required_inputs)
            }
            Self::StorageDomain {
                domain_name,
                backing_table,
            } => {
                validate_required("storage.domain_name", domain_name)?;
                validate_required("storage.backing_table", backing_table)
            }
            Self::ResearchGuard { decision_record } => {
                validate_required("research.decision_record", decision_record)
            }
            _ => Ok(()),
        }
    }
}

pub fn canonical_advanced_planner_contract() -> AdvancedPlannerContract {
    AdvancedPlannerContract {
        surfaces: vec![
            lookup("T4", "hash-table planner hot path", "planner/shard_lookup"),
            batch("T10", "bulk protocol fetch path", "executor/bulk_fetch"),
            distributed_sql("T11", "DistSQL physical pushdown", "planner/distsql"),
            txn("T13", "distributed cursor state", "txn_coord/cursors"),
            txn("T14", "distributed savepoint state", "txn_coord/savepoints"),
            policy(
                "TS10",
                "hierarchical continuous aggregate fanout",
                &["source_cagg", "target_cagg"],
            ),
            policy(
                "TS11",
                "segmentby bloom filter fanout",
                &["table", "segmentby_column"],
            ),
            policy(
                "A10",
                "streaming chat completion SRF",
                &["provider", "model", "tenant_budget"],
            ),
            policy(
                "A11",
                "semantic catalog text-to-SQL",
                &["catalog", "tenant_scope"],
            ),
            policy(
                "M4",
                "schema drift reconciliation",
                &["observed_schema", "desired_schema"],
            ),
            policy(
                "L7",
                "citus columnar analytical path",
                &["table", "columnar_policy"],
            ),
            policy("L10", "cross-tier HTAP planner", &["hot", "warm", "cold"]),
            policy(
                "R3",
                "worker columnstore policy",
                &["table", "age_threshold"],
            ),
            policy(
                "R8",
                "non-hypertable cold columnar path",
                &["table", "tier"],
            ),
            policy(
                "R12",
                "per-shard temperature ranking",
                &["shard_id", "temperature_score"],
            ),
            policy(
                "S1",
                "automatic shard split intent",
                &["shard_group", "threshold"],
            ),
            policy(
                "S3",
                "clone-node scale-out intent",
                &["source_worker", "target_worker"],
            ),
            policy(
                "S8",
                "locality-prefixed primary keys",
                &["region", "tenant_id"],
            ),
            policy(
                "S12",
                "tablespace-aware placement",
                &["region", "tablespace"],
            ),
            policy(
                "MR3",
                "regional row placement",
                &["region_prefix", "distribution_key"],
            ),
            policy(
                "MR6",
                "closed timestamp time travel",
                &["timestamp", "max_staleness"],
            ),
            policy(
                "B4",
                "backup as read-only data source",
                &["backup_id", "branch"],
            ),
            policy(
                "F3",
                "Iceberg federation catalog bridge",
                &["catalog", "warehouse"],
            ),
            policy(
                "F4",
                "rotating postgres_fdw credentials",
                &["server", "secret_ref"],
            ),
            storage_domain("Sto2", "file_attachment", "storage.file_attachment_refs"),
            research_guard("Edge1", "edge bounded-staleness replica contract"),
            research_guard("Edge2", "libsql read-tier research guard"),
        ],
    }
}

fn lookup(feature_id: &'static str, name: &str, reference: &str) -> PlannerSurface {
    surface(
        feature_id,
        name,
        reference,
        PlannerSurfaceKind::Lookup { min_partitions: 1 },
    )
}

fn batch(feature_id: &'static str, name: &str, reference: &str) -> PlannerSurface {
    surface(
        feature_id,
        name,
        reference,
        PlannerSurfaceKind::BatchTransfer {
            max_batch_rows: 4096,
        },
    )
}

fn distributed_sql(feature_id: &'static str, name: &str, reference: &str) -> PlannerSurface {
    surface(
        feature_id,
        name,
        reference,
        PlannerSurfaceKind::DistributedSql { worker_tasks: 2 },
    )
}

fn txn(feature_id: &'static str, name: &str, reference: &str) -> PlannerSurface {
    surface(
        feature_id,
        name,
        reference,
        PlannerSurfaceKind::TransactionState {
            max_open_shards: 128,
        },
    )
}

fn policy(feature_id: &'static str, name: &str, required_inputs: &[&str]) -> PlannerSurface {
    PlannerSurface {
        feature_id,
        name: name.to_string(),
        references: vec!["docs/ai-blaise/ARCHITECTURE.md".to_string()],
        kind: PlannerSurfaceKind::Policy {
            required_inputs: required_inputs
                .iter()
                .map(|input| (*input).to_string())
                .collect(),
        },
    }
}

fn storage_domain(
    feature_id: &'static str,
    domain_name: &str,
    backing_table: &str,
) -> PlannerSurface {
    PlannerSurface {
        feature_id,
        name: "file attachment domain type".to_string(),
        references: vec!["docs/ai-blaise/ARCHITECTURE.md".to_string()],
        kind: PlannerSurfaceKind::StorageDomain {
            domain_name: domain_name.to_string(),
            backing_table: backing_table.to_string(),
        },
    }
}

fn research_guard(feature_id: &'static str, name: &str) -> PlannerSurface {
    PlannerSurface {
        feature_id,
        name: name.to_string(),
        references: vec!["docs/ai-blaise/ARCHITECTURE.md".to_string()],
        kind: PlannerSurfaceKind::ResearchGuard {
            decision_record: "docs/ai-blaise/ADR/0001-fork-not-rewrite.md".to_string(),
        },
    }
}

fn surface(
    feature_id: &'static str,
    name: &str,
    reference: &str,
    kind: PlannerSurfaceKind,
) -> PlannerSurface {
    PlannerSurface {
        feature_id,
        name: name.to_string(),
        references: vec![reference.to_string()],
        kind,
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum AdvancedPlannerError {
    InvalidPositive(&'static str),
    MissingFeature(&'static str),
    MissingRequiredField(&'static str),
}

impl fmt::Display for AdvancedPlannerError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidPositive(field) => write!(formatter, "{field} must be greater than zero"),
            Self::MissingFeature(feature_id) => {
                write!(
                    formatter,
                    "advanced planner contract missing feature {feature_id}"
                )
            }
            Self::MissingRequiredField(field) => write!(formatter, "{field} must not be empty"),
        }
    }
}

impl Error for AdvancedPlannerError {}

fn validate_required(field: &'static str, value: &str) -> Result<(), AdvancedPlannerError> {
    if value.trim().is_empty() {
        return Err(AdvancedPlannerError::MissingRequiredField(field));
    }
    Ok(())
}

fn validate_required_list(
    field: &'static str,
    values: &[String],
) -> Result<(), AdvancedPlannerError> {
    if values.is_empty() || values.iter().any(|value| value.trim().is_empty()) {
        return Err(AdvancedPlannerError::MissingRequiredField(field));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn advanced_planner_contract_covers_remaining_surfaces() {
        let contract = canonical_advanced_planner_contract();

        assert_eq!(contract.validate(), Ok(()));
        assert_eq!(contract.surfaces.len(), ADVANCED_PLANNER_FEATURE_IDS.len());
    }

    #[test]
    fn advanced_planner_rejects_invalid_batch_size() {
        let contract = AdvancedPlannerContract {
            surfaces: vec![PlannerSurface {
                feature_id: "T10",
                name: "bulk fetch".to_string(),
                references: vec!["executor/bulk_fetch".to_string()],
                kind: PlannerSurfaceKind::BatchTransfer { max_batch_rows: 0 },
            }],
        };

        assert_eq!(
            contract.validate(),
            Err(AdvancedPlannerError::InvalidPositive("max_batch_rows"))
        );
    }
}
