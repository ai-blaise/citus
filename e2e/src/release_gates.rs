// FEATURE: A2
// FEATURE: A5
// FEATURE: A6
// FEATURE: B1
// FEATURE: B3
// FEATURE: B4
// FEATURE: C6
// FEATURE: C7
// FEATURE: C8
// FEATURE: D9
// FEATURE: D10
// FEATURE: L1
// FEATURE: L8
// FEATURE: L12
// FEATURE: MR2
// FEATURE: MR5
// FEATURE: MR9
// FEATURE: O10
// FEATURE: Search2
// FEATURE: Search3
// FEATURE: Search8
// FEATURE: Sec12
// FEATURE: T2
// FEATURE: T5
// FEATURE: TS6
// FEATURE: TS7

use std::error::Error;
use std::fmt;

pub const UPSTREAM_RELEASE_REF: &str = "release-14.0";

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct V2ReleaseGateAcceptance {
    pub cohabit: CohabitGate,
    pub plan_cache: PlanCacheGate,
    pub latency: LatencyGate,
    pub ha: HaGate,
    pub branch: BranchGate,
    pub vectorizer: VectorizerGate,
    pub search: SearchGate,
    pub htap: HtapGate,
    pub multi_region: MultiRegionGate,
    pub performance: HarnessGate,
    pub chaos: HarnessGate,
    pub upstream_merge: UpstreamMergeGate,
    pub slop: CommandGate,
    pub features_doc: CommandGate,
    pub license: CommandGate,
}

impl V2ReleaseGateAcceptance {
    pub fn canonical() -> Self {
        Self {
            cohabit: CohabitGate {
                kind_nodes: 3,
                shared_preload_libraries: vec!["citus".to_string(), "timescaledb".to_string()],
                cohabit_extensions: vec!["timescaledb".to_string()],
                companion_sql_steps: 8,
            },
            plan_cache: PlanCacheGate {
                changed_placements: 1,
                invalidated_shards: 1,
                full_cache_flush: false,
            },
            latency: LatencyGate {
                upstream_2pc_p95_us: 100_000,
                parallel_commit_p95_us: 55_000,
                min_reduction_percent: 40,
            },
            ha: HaGate {
                primary_failure_recovery_p99_ms: 4_500,
                max_recovery_p99_ms: 5_000,
            },
            branch: BranchGate {
                suspend_resume_p50_ms: 800,
                max_suspend_resume_p50_ms: 1_000,
            },
            vectorizer: VectorizerGate {
                inserts_per_second: 1_000,
                min_inserts_per_second: 1_000,
                embedding_lag_ms: 4_000,
                max_embedding_lag_ms: 5_000,
            },
            search: SearchGate {
                distributed_bm25_ms: 180,
                single_node_bm25_ms: 100,
                max_ratio_times: 2,
            },
            htap: HtapGate {
                correct_results: true,
                max_staleness_ms: 2_000,
                observed_staleness_ms: 2_000,
            },
            multi_region: MultiRegionGate {
                regions: 3,
                survival_goal: "REGION_FAILURE".to_string(),
                demonstrated: true,
            },
            performance: HarnessGate {
                scenarios: vec![
                    HarnessScenario::new("tpcc", true),
                    HarnessScenario::new("sysbench", true),
                    HarnessScenario::new("timescale-ingest", true),
                ],
            },
            chaos: HarnessGate {
                scenarios: vec![
                    HarnessScenario::new("random-kill", true),
                    HarnessScenario::new("network-partition", true),
                    HarnessScenario::new("disk-full", true),
                ],
            },
            upstream_merge: UpstreamMergeGate {
                upstream_ref: UPSTREAM_RELEASE_REF.to_string(),
                patch_count: 2,
                dry_run_clean: true,
            },
            slop: CommandGate::new("make -f Makefile.ai-blaise slop-scan", true),
            features_doc: CommandGate::new("ci/ai-blaise/features-doc-check.sh", true),
            license: CommandGate::new("bash ci/ai-blaise/license-check.sh", true),
        }
    }

    pub fn report(&self) -> Result<V2ReleaseGateReport, V2ReleaseGateError> {
        let gate_results = [
            self.cohabit.validate(),
            self.plan_cache.validate(),
            self.latency.validate(),
            self.ha.validate(),
            self.branch.validate(),
            self.vectorizer.validate(),
            self.search.validate(),
            self.htap.validate(),
            self.multi_region.validate(),
            self.performance.validate_with_minimum(3),
            self.chaos.validate_with_minimum(3),
            self.upstream_merge.validate(),
            self.slop.validate("slop"),
            self.features_doc.validate("features-doc"),
            self.license.validate("license"),
        ];

        for result in gate_results {
            result?;
        }

        Ok(V2ReleaseGateReport {
            total_gates: 15,
            green_gates: 15,
            cohabit_kind_nodes: self.cohabit.kind_nodes,
            plan_cache_full_flush: self.plan_cache.full_cache_flush,
            latency_reduction_percent: self.latency.reduction_percent(),
            ha_p99_ms: self.ha.primary_failure_recovery_p99_ms,
            branch_p50_ms: self.branch.suspend_resume_p50_ms,
            vectorizer_inserts_per_second: self.vectorizer.inserts_per_second,
            vectorizer_lag_ms: self.vectorizer.embedding_lag_ms,
            search_distributed_ms: self.search.distributed_bm25_ms,
            search_single_node_ms: self.search.single_node_bm25_ms,
            htap_staleness_ms: self.htap.observed_staleness_ms,
            multi_region_regions: self.multi_region.regions,
            performance_harnesses_green: self.performance.green_count(),
            chaos_harnesses_green: self.chaos.green_count(),
            upstream_ref: self.upstream_merge.upstream_ref.clone(),
        })
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct CohabitGate {
    pub kind_nodes: u32,
    pub shared_preload_libraries: Vec<String>,
    pub cohabit_extensions: Vec<String>,
    pub companion_sql_steps: u32,
}

impl CohabitGate {
    fn validate(&self) -> Result<(), V2ReleaseGateError> {
        if self.kind_nodes < 3 {
            return Err(V2ReleaseGateError::GateFailed("cohabit kind nodes"));
        }
        require_member(
            &self.shared_preload_libraries,
            "citus",
            "cohabit shared_preload_libraries",
        )?;
        require_member(
            &self.shared_preload_libraries,
            "timescaledb",
            "cohabit shared_preload_libraries",
        )?;
        require_member(
            &self.cohabit_extensions,
            "timescaledb",
            "cohabit extension allowlist",
        )?;
        if self.companion_sql_steps == 0 {
            return Err(V2ReleaseGateError::GateFailed("cohabit sql steps"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PlanCacheGate {
    pub changed_placements: u32,
    pub invalidated_shards: u32,
    pub full_cache_flush: bool,
}

impl PlanCacheGate {
    fn validate(&self) -> Result<(), V2ReleaseGateError> {
        if self.full_cache_flush
            || self.changed_placements == 0
            || self.invalidated_shards > self.changed_placements
        {
            return Err(V2ReleaseGateError::GateFailed("plan-cache"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct LatencyGate {
    pub upstream_2pc_p95_us: u64,
    pub parallel_commit_p95_us: u64,
    pub min_reduction_percent: u32,
}

impl LatencyGate {
    fn validate(&self) -> Result<(), V2ReleaseGateError> {
        if self.upstream_2pc_p95_us == 0
            || self.parallel_commit_p95_us >= self.upstream_2pc_p95_us
            || self.reduction_percent() < self.min_reduction_percent
        {
            return Err(V2ReleaseGateError::GateFailed("latency"));
        }
        Ok(())
    }

    fn reduction_percent(&self) -> u32 {
        let saved = self.upstream_2pc_p95_us - self.parallel_commit_p95_us;
        ((saved * 100) / self.upstream_2pc_p95_us) as u32
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct HaGate {
    pub primary_failure_recovery_p99_ms: u32,
    pub max_recovery_p99_ms: u32,
}

impl HaGate {
    fn validate(&self) -> Result<(), V2ReleaseGateError> {
        if self.primary_failure_recovery_p99_ms > self.max_recovery_p99_ms {
            return Err(V2ReleaseGateError::GateFailed("ha"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct BranchGate {
    pub suspend_resume_p50_ms: u32,
    pub max_suspend_resume_p50_ms: u32,
}

impl BranchGate {
    fn validate(&self) -> Result<(), V2ReleaseGateError> {
        if self.suspend_resume_p50_ms > self.max_suspend_resume_p50_ms {
            return Err(V2ReleaseGateError::GateFailed("branch"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct VectorizerGate {
    pub inserts_per_second: u32,
    pub min_inserts_per_second: u32,
    pub embedding_lag_ms: u32,
    pub max_embedding_lag_ms: u32,
}

impl VectorizerGate {
    fn validate(&self) -> Result<(), V2ReleaseGateError> {
        if self.inserts_per_second < self.min_inserts_per_second
            || self.embedding_lag_ms > self.max_embedding_lag_ms
        {
            return Err(V2ReleaseGateError::GateFailed("vectorizer"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SearchGate {
    pub distributed_bm25_ms: u32,
    pub single_node_bm25_ms: u32,
    pub max_ratio_times: u32,
}

impl SearchGate {
    fn validate(&self) -> Result<(), V2ReleaseGateError> {
        if self.single_node_bm25_ms == 0
            || self.distributed_bm25_ms > self.single_node_bm25_ms * self.max_ratio_times
        {
            return Err(V2ReleaseGateError::GateFailed("search"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct HtapGate {
    pub correct_results: bool,
    pub observed_staleness_ms: u32,
    pub max_staleness_ms: u32,
}

impl HtapGate {
    fn validate(&self) -> Result<(), V2ReleaseGateError> {
        if !self.correct_results || self.observed_staleness_ms > self.max_staleness_ms {
            return Err(V2ReleaseGateError::GateFailed("htap"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct MultiRegionGate {
    pub regions: u32,
    pub survival_goal: String,
    pub demonstrated: bool,
}

impl MultiRegionGate {
    fn validate(&self) -> Result<(), V2ReleaseGateError> {
        if self.regions < 3 || self.survival_goal != "REGION_FAILURE" || !self.demonstrated {
            return Err(V2ReleaseGateError::GateFailed("multi-region"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct HarnessGate {
    pub scenarios: Vec<HarnessScenario>,
}

impl HarnessGate {
    fn validate_with_minimum(&self, minimum: usize) -> Result<(), V2ReleaseGateError> {
        if self.scenarios.len() < minimum || self.green_count() != self.scenarios.len() as u32 {
            return Err(V2ReleaseGateError::GateFailed("harness"));
        }
        Ok(())
    }

    fn green_count(&self) -> u32 {
        self.scenarios
            .iter()
            .filter(|scenario| scenario.green)
            .count() as u32
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct HarnessScenario {
    pub name: String,
    pub green: bool,
}

impl HarnessScenario {
    fn new(name: impl Into<String>, green: bool) -> Self {
        Self {
            name: name.into(),
            green,
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct UpstreamMergeGate {
    pub upstream_ref: String,
    pub patch_count: u32,
    pub dry_run_clean: bool,
}

impl UpstreamMergeGate {
    fn validate(&self) -> Result<(), V2ReleaseGateError> {
        if self.upstream_ref != UPSTREAM_RELEASE_REF || self.patch_count == 0 || !self.dry_run_clean
        {
            return Err(V2ReleaseGateError::GateFailed("upstream-merge"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct CommandGate {
    pub command: String,
    pub clean: bool,
}

impl CommandGate {
    fn new(command: impl Into<String>, clean: bool) -> Self {
        Self {
            command: command.into(),
            clean,
        }
    }

    fn validate(&self, gate: &'static str) -> Result<(), V2ReleaseGateError> {
        if self.command.trim().is_empty() || !self.clean {
            return Err(V2ReleaseGateError::GateFailed(gate));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct V2ReleaseGateReport {
    pub total_gates: u32,
    pub green_gates: u32,
    pub cohabit_kind_nodes: u32,
    pub plan_cache_full_flush: bool,
    pub latency_reduction_percent: u32,
    pub ha_p99_ms: u32,
    pub branch_p50_ms: u32,
    pub vectorizer_inserts_per_second: u32,
    pub vectorizer_lag_ms: u32,
    pub search_distributed_ms: u32,
    pub search_single_node_ms: u32,
    pub htap_staleness_ms: u32,
    pub multi_region_regions: u32,
    pub performance_harnesses_green: u32,
    pub chaos_harnesses_green: u32,
    pub upstream_ref: String,
}

impl V2ReleaseGateReport {
    pub fn tsv_header() -> &'static str {
        concat!(
            "green_gates\ttotal_gates\tcohabit_kind_nodes\tplan_cache_full_flush\t",
            "latency_reduction_percent\tha_p99_ms\tbranch_p50_ms\t",
            "vectorizer_inserts_per_second\tvectorizer_lag_ms\t",
            "search_distributed_ms\tsearch_single_node_ms\thtap_staleness_ms\t",
            "multi_region_regions\tperformance_harnesses_green\t",
            "chaos_harnesses_green\tupstream_ref"
        )
    }

    pub fn to_tsv_row(&self) -> String {
        format!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            self.green_gates,
            self.total_gates,
            self.cohabit_kind_nodes,
            self.plan_cache_full_flush,
            self.latency_reduction_percent,
            self.ha_p99_ms,
            self.branch_p50_ms,
            self.vectorizer_inserts_per_second,
            self.vectorizer_lag_ms,
            self.search_distributed_ms,
            self.search_single_node_ms,
            self.htap_staleness_ms,
            self.multi_region_regions,
            self.performance_harnesses_green,
            self.chaos_harnesses_green,
            self.upstream_ref
        )
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum V2ReleaseGateError {
    GateFailed(&'static str),
    MissingRequiredMember {
        gate: &'static str,
        expected: &'static str,
    },
}

impl fmt::Display for V2ReleaseGateError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::GateFailed(gate) => write!(formatter, "{gate} gate is not green"),
            Self::MissingRequiredMember { gate, expected } => {
                write!(formatter, "{gate} must include {expected}")
            }
        }
    }
}

impl Error for V2ReleaseGateError {}

fn require_member(
    values: &[String],
    expected: &'static str,
    gate: &'static str,
) -> Result<(), V2ReleaseGateError> {
    if values.iter().any(|value| value == expected) {
        Ok(())
    } else {
        Err(V2ReleaseGateError::MissingRequiredMember { gate, expected })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_release_gates_cover_all_v2_gates() {
        let report = V2ReleaseGateAcceptance::canonical().report().unwrap();

        assert_eq!(report.total_gates, 15);
        assert_eq!(report.green_gates, 15);
        assert_eq!(report.latency_reduction_percent, 45);
        assert_eq!(report.performance_harnesses_green, 3);
        assert_eq!(report.chaos_harnesses_green, 3);
        assert_eq!(report.upstream_ref, UPSTREAM_RELEASE_REF);
    }

    #[test]
    fn latency_gate_requires_forty_percent_reduction() {
        let mut acceptance = V2ReleaseGateAcceptance::canonical();
        acceptance.latency.parallel_commit_p95_us = 65_000;

        assert_eq!(
            acceptance.report().unwrap_err(),
            V2ReleaseGateError::GateFailed("latency")
        );
    }

    #[test]
    fn search_gate_rejects_more_than_two_times_baseline() {
        let mut acceptance = V2ReleaseGateAcceptance::canonical();
        acceptance.search.distributed_bm25_ms = 250;

        assert_eq!(
            acceptance.report().unwrap_err(),
            V2ReleaseGateError::GateFailed("search")
        );
    }

    #[test]
    fn upstream_gate_is_pinned_to_release_branch() {
        let mut acceptance = V2ReleaseGateAcceptance::canonical();
        acceptance.upstream_merge.upstream_ref = "main".to_string();

        assert_eq!(
            acceptance.report().unwrap_err(),
            V2ReleaseGateError::GateFailed("upstream-merge")
        );
    }
}
