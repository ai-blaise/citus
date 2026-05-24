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
            if !seen.insert(surface.feature_id) {
                return Err(AdvancedPlannerError::DuplicateFeature(surface.feature_id));
            }
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

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct AdvancedPlannerExecutionReport {
    pub surface_count: usize,
    pub lookup_surfaces: usize,
    pub lookup_min_partitions: u32,
    pub max_batch_rows: u32,
    pub distributed_sql_worker_tasks: u32,
    pub transaction_state_surfaces: usize,
    pub transaction_shard_budget: u32,
    pub policy_surfaces: usize,
    pub policy_required_inputs: usize,
    pub storage_domains: usize,
    pub research_guards: usize,
}

impl AdvancedPlannerExecutionReport {
    fn from_contract(contract: &AdvancedPlannerContract) -> Result<Self, AdvancedPlannerError> {
        contract.validate()?;

        let mut report = Self {
            surface_count: contract.surfaces.len(),
            lookup_surfaces: 0,
            lookup_min_partitions: 0,
            max_batch_rows: 0,
            distributed_sql_worker_tasks: 0,
            transaction_state_surfaces: 0,
            transaction_shard_budget: 0,
            policy_surfaces: 0,
            policy_required_inputs: 0,
            storage_domains: 0,
            research_guards: 0,
        };

        for surface in &contract.surfaces {
            match &surface.kind {
                PlannerSurfaceKind::Lookup { min_partitions } => {
                    report.lookup_surfaces += 1;
                    report.lookup_min_partitions += min_partitions;
                }
                PlannerSurfaceKind::BatchTransfer { max_batch_rows } => {
                    report.max_batch_rows = report.max_batch_rows.max(*max_batch_rows);
                }
                PlannerSurfaceKind::DistributedSql { worker_tasks } => {
                    report.distributed_sql_worker_tasks += worker_tasks;
                }
                PlannerSurfaceKind::TransactionState { max_open_shards } => {
                    report.transaction_state_surfaces += 1;
                    report.transaction_shard_budget += max_open_shards;
                }
                PlannerSurfaceKind::Policy { required_inputs } => {
                    report.policy_surfaces += 1;
                    report.policy_required_inputs += required_inputs.len();
                }
                PlannerSurfaceKind::StorageDomain { .. } => {
                    report.storage_domains += 1;
                }
                PlannerSurfaceKind::ResearchGuard { .. } => {
                    report.research_guards += 1;
                }
            }
        }

        Ok(report)
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct AdvancedPlannerRuntimeScenario {
    pub feature_id: &'static str,
    pub scenario_name: String,
    pub required_evidence: Vec<String>,
    pub contract_checks: Vec<String>,
    pub execution_boundary: PlannerExecutionBoundary,
}

impl AdvancedPlannerRuntimeScenario {
    fn validate(
        &self,
        contract_features: &BTreeSet<&'static str>,
    ) -> Result<(), AdvancedPlannerError> {
        validate_required("runtime.feature_id", self.feature_id)?;
        validate_required("runtime.scenario_name", &self.scenario_name)?;
        validate_required_list("runtime.required_evidence", &self.required_evidence)?;
        validate_required_list("runtime.contract_checks", &self.contract_checks)?;

        if !contract_features.contains(self.feature_id) {
            return Err(AdvancedPlannerError::UnknownRuntimeFeature(self.feature_id));
        }

        if self.execution_boundary == PlannerExecutionBoundary::LiveDistributedExecution {
            return Err(AdvancedPlannerError::UnsupportedLiveExecutionClaim(
                self.feature_id,
            ));
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum PlannerExecutionBoundary {
    DeterministicContract,
    PatchSmoke,
    PlanOnly,
    ResearchGuard,
    LiveDistributedExecution,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct AdvancedPlannerRuntimeReport {
    pub scenario_count: usize,
    pub covered_features: usize,
    pub contract_checks: usize,
    pub fail_closed_checks: usize,
    pub live_execution_claims: usize,
    pub patch_smoke_boundaries: usize,
    pub plan_only_boundaries: usize,
    pub deterministic_boundaries: usize,
    pub research_guard_boundaries: usize,
}

impl AdvancedPlannerRuntimeReport {
    fn from_contract(contract: &AdvancedPlannerContract) -> Result<Self, AdvancedPlannerError> {
        contract.validate()?;
        let scenarios = contract
            .surfaces
            .iter()
            .map(runtime_scenario_for_surface)
            .collect::<Vec<_>>();

        Self::from_scenarios(contract, &scenarios)
    }

    fn from_scenarios(
        contract: &AdvancedPlannerContract,
        scenarios: &[AdvancedPlannerRuntimeScenario],
    ) -> Result<Self, AdvancedPlannerError> {
        contract.validate()?;
        if scenarios.is_empty() {
            return Err(AdvancedPlannerError::MissingRequiredField(
                "runtime.scenarios",
            ));
        }

        let contract_features = contract
            .surfaces
            .iter()
            .map(|surface| surface.feature_id)
            .collect::<BTreeSet<_>>();
        let mut scenario_features = BTreeSet::new();
        let mut contract_checks = 0;
        let mut live_execution_claims = 0;
        let mut patch_smoke_boundaries = 0;
        let mut plan_only_boundaries = 0;
        let mut deterministic_boundaries = 0;
        let mut research_guard_boundaries = 0;

        for scenario in scenarios {
            scenario.validate(&contract_features)?;
            if !scenario_features.insert(scenario.feature_id) {
                return Err(AdvancedPlannerError::DuplicateFeature(scenario.feature_id));
            }
            contract_checks += scenario.contract_checks.len();
            match scenario.execution_boundary {
                PlannerExecutionBoundary::DeterministicContract => deterministic_boundaries += 1,
                PlannerExecutionBoundary::PatchSmoke => patch_smoke_boundaries += 1,
                PlannerExecutionBoundary::PlanOnly => plan_only_boundaries += 1,
                PlannerExecutionBoundary::ResearchGuard => research_guard_boundaries += 1,
                PlannerExecutionBoundary::LiveDistributedExecution => live_execution_claims += 1,
            }
        }

        for feature_id in &contract_features {
            if !scenario_features.contains(feature_id) {
                return Err(AdvancedPlannerError::MissingRuntimeScenario(feature_id));
            }
        }

        Ok(Self {
            scenario_count: scenarios.len(),
            covered_features: scenario_features.len(),
            contract_checks,
            fail_closed_checks: canonical_advanced_planner_fail_closed_checks(),
            live_execution_claims,
            patch_smoke_boundaries,
            plan_only_boundaries,
            deterministic_boundaries,
            research_guard_boundaries,
        })
    }
}

pub fn canonical_advanced_planner_runtime_report(
) -> Result<AdvancedPlannerRuntimeReport, AdvancedPlannerError> {
    AdvancedPlannerRuntimeReport::from_contract(&canonical_advanced_planner_contract())
}

pub fn canonical_advanced_planner_fail_closed_checks() -> usize {
    let mut checks = 0;

    if (AdvancedPlannerContract { surfaces: vec![] })
        .validate()
        .is_err()
    {
        checks += 1;
    }

    let mut duplicate = canonical_advanced_planner_contract();
    duplicate.surfaces.push(duplicate.surfaces[0].clone());
    if matches!(
        duplicate.validate(),
        Err(AdvancedPlannerError::DuplicateFeature(_))
    ) {
        checks += 1;
    }

    if (AdvancedPlannerContract {
        surfaces: vec![PlannerSurface {
            feature_id: "T10",
            name: "bulk fetch".to_string(),
            references: vec!["executor/bulk_fetch".to_string()],
            kind: PlannerSurfaceKind::BatchTransfer { max_batch_rows: 0 },
        }],
    })
    .validate()
    .is_err()
    {
        checks += 1;
    }

    let contract_features = canonical_advanced_planner_contract()
        .surfaces
        .iter()
        .map(|surface| surface.feature_id)
        .collect::<BTreeSet<_>>();
    if runtime_probe("Nope", PlannerExecutionBoundary::PlanOnly)
        .validate(&contract_features)
        .is_err()
    {
        checks += 1;
    }
    if runtime_probe("T11", PlannerExecutionBoundary::LiveDistributedExecution)
        .validate(&contract_features)
        .is_err()
    {
        checks += 1;
    }

    checks
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

pub fn canonical_advanced_planner_execution_report(
) -> Result<AdvancedPlannerExecutionReport, AdvancedPlannerError> {
    AdvancedPlannerExecutionReport::from_contract(&canonical_advanced_planner_contract())
}

fn runtime_scenario_for_surface(surface: &PlannerSurface) -> AdvancedPlannerRuntimeScenario {
    let mut required_evidence = surface.references.clone();
    required_evidence.push("docs/ai-blaise/NEW_FEATURES.md".to_string());
    required_evidence.push("ci/ai-blaise/companion-advanced-planner-smoke.sh".to_string());

    let (execution_boundary, contract_checks) = match &surface.kind {
        PlannerSurfaceKind::Lookup { min_partitions } => (
            PlannerExecutionBoundary::PatchSmoke,
            vec![
                format!("min_partitions={min_partitions}"),
                "router_planner_algorithm_smoke_required".to_string(),
            ],
        ),
        PlannerSurfaceKind::BatchTransfer { max_batch_rows } => (
            PlannerExecutionBoundary::PlanOnly,
            vec![
                format!("max_batch_rows={max_batch_rows}"),
                "external_protocol_backpressure_not_claimed".to_string(),
            ],
        ),
        PlannerSurfaceKind::DistributedSql { worker_tasks } => (
            PlannerExecutionBoundary::PlanOnly,
            vec![
                format!("worker_tasks={worker_tasks}"),
                "physical_worker_execution_not_claimed".to_string(),
            ],
        ),
        PlannerSurfaceKind::TransactionState { max_open_shards } => (
            PlannerExecutionBoundary::PlanOnly,
            vec![
                format!("max_open_shards={max_open_shards}"),
                "distributed_cleanup_not_claimed".to_string(),
            ],
        ),
        PlannerSurfaceKind::Policy { required_inputs } => {
            let mut checks = vec!["policy_contract_present".to_string()];
            checks.extend(
                required_inputs
                    .iter()
                    .map(|input| format!("required_input:{input}")),
            );
            (PlannerExecutionBoundary::DeterministicContract, checks)
        }
        PlannerSurfaceKind::StorageDomain {
            domain_name,
            backing_table,
        } => (
            PlannerExecutionBoundary::DeterministicContract,
            vec![
                format!("domain_name={domain_name}"),
                format!("backing_table={backing_table}"),
            ],
        ),
        PlannerSurfaceKind::ResearchGuard { decision_record } => (
            PlannerExecutionBoundary::ResearchGuard,
            vec![format!("decision_record={decision_record}")],
        ),
    };

    AdvancedPlannerRuntimeScenario {
        feature_id: surface.feature_id,
        scenario_name: format!("{} runtime boundary", surface.name),
        required_evidence,
        contract_checks,
        execution_boundary,
    }
}

fn runtime_probe(
    feature_id: &'static str,
    execution_boundary: PlannerExecutionBoundary,
) -> AdvancedPlannerRuntimeScenario {
    AdvancedPlannerRuntimeScenario {
        feature_id,
        scenario_name: "probe".to_string(),
        required_evidence: vec!["ci/ai-blaise/companion-advanced-planner-smoke.sh".to_string()],
        contract_checks: vec!["probe_check".to_string()],
        execution_boundary,
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
    DuplicateFeature(&'static str),
    MissingFeature(&'static str),
    MissingRuntimeScenario(&'static str),
    MissingRequiredField(&'static str),
    UnknownRuntimeFeature(&'static str),
    UnsupportedLiveExecutionClaim(&'static str),
}

impl fmt::Display for AdvancedPlannerError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidPositive(field) => write!(formatter, "{field} must be greater than zero"),
            Self::DuplicateFeature(feature_id) => {
                write!(formatter, "advanced planner contract duplicates feature {feature_id}")
            }
            Self::MissingFeature(feature_id) => {
                write!(
                    formatter,
                    "advanced planner contract missing feature {feature_id}"
                )
            }
            Self::MissingRuntimeScenario(feature_id) => {
                write!(
                    formatter,
                    "advanced planner runtime report missing scenario for {feature_id}"
                )
            }
            Self::MissingRequiredField(field) => write!(formatter, "{field} must not be empty"),
            Self::UnknownRuntimeFeature(feature_id) => {
                write!(formatter, "advanced planner runtime scenario references unknown feature {feature_id}")
            }
            Self::UnsupportedLiveExecutionClaim(feature_id) => write!(
                formatter,
                "advanced planner runtime scenario for {feature_id} claims live distributed execution"
            ),
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
    fn advanced_planner_execution_report_is_deterministic() {
        let report = canonical_advanced_planner_execution_report().expect("execution report");

        assert_eq!(report.surface_count, 27);
        assert_eq!(report.lookup_surfaces, 1);
        assert_eq!(report.lookup_min_partitions, 1);
        assert_eq!(report.max_batch_rows, 4096);
        assert_eq!(report.distributed_sql_worker_tasks, 2);
        assert_eq!(report.transaction_state_surfaces, 2);
        assert_eq!(report.transaction_shard_budget, 256);
        assert_eq!(report.policy_surfaces, 19);
        assert_eq!(report.policy_required_inputs, 40);
        assert_eq!(report.storage_domains, 1);
        assert_eq!(report.research_guards, 2);
    }

    #[test]
    fn advanced_planner_runtime_report_is_deterministic_and_bounded() {
        let report = canonical_advanced_planner_runtime_report().expect("runtime report");

        assert_eq!(report.scenario_count, 27);
        assert_eq!(report.covered_features, 27);
        assert_eq!(report.contract_checks, 73);
        assert_eq!(report.fail_closed_checks, 5);
        assert_eq!(report.live_execution_claims, 0);
        assert_eq!(report.patch_smoke_boundaries, 1);
        assert_eq!(report.plan_only_boundaries, 4);
        assert_eq!(report.deterministic_boundaries, 20);
        assert_eq!(report.research_guard_boundaries, 2);
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

    #[test]
    fn advanced_planner_rejects_duplicate_feature_surface() {
        let mut contract = canonical_advanced_planner_contract();
        contract.surfaces.push(contract.surfaces[0].clone());

        assert_eq!(
            contract.validate(),
            Err(AdvancedPlannerError::DuplicateFeature("T4"))
        );
    }

    #[test]
    fn advanced_planner_runtime_rejects_unknown_feature() {
        let contract = canonical_advanced_planner_contract();
        let scenarios = vec![runtime_probe("Unknown", PlannerExecutionBoundary::PlanOnly)];

        assert_eq!(
            AdvancedPlannerRuntimeReport::from_scenarios(&contract, &scenarios),
            Err(AdvancedPlannerError::UnknownRuntimeFeature("Unknown"))
        );
    }

    #[test]
    fn advanced_planner_runtime_rejects_live_execution_overclaim() {
        let contract = canonical_advanced_planner_contract();
        let scenarios = vec![runtime_probe(
            "T11",
            PlannerExecutionBoundary::LiveDistributedExecution,
        )];

        assert_eq!(
            AdvancedPlannerRuntimeReport::from_scenarios(&contract, &scenarios),
            Err(AdvancedPlannerError::UnsupportedLiveExecutionClaim("T11"))
        );
    }
}
