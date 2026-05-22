//! Cold-tier sidecar contracts.

// FEATURE: R1
// FEATURE: R5
// FEATURE: R9
// FEATURE: Search8

use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ColdTierPlan {
    pub policy: TierPolicy,
    pub shards: Vec<ColdShard>,
    pub search: Option<SearchColdTierPlan>,
}

impl ColdTierPlan {
    pub fn validate(&self) -> Result<(), ColdTierError> {
        self.policy.validate()?;
        if self.shards.is_empty() {
            return Err(ColdTierError::MissingRequiredField("shards"));
        }
        for shard in &self.shards {
            shard.validate()?;
        }
        if let Some(search) = &self.search {
            search.validate()?;
        }
        Ok(())
    }

    pub fn move_plans(&self) -> Result<Vec<TierMovePlan>, ColdTierError> {
        self.validate()?;
        Ok(self
            .shards
            .iter()
            .filter_map(|shard| {
                let target = self.policy.target_tier(shard.temperature_score);
                (target != shard.current_tier).then(|| TierMovePlan {
                    shard_id: shard.shard_id,
                    table: shard.table.clone(),
                    from: shard.current_tier,
                    to: target,
                    object_uri: shard.object_uri.clone(),
                })
            })
            .collect())
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct TierPolicy {
    pub hot_min_score: u8,
    pub warm_min_score: u8,
    pub cold_max_score: u8,
}

impl TierPolicy {
    fn validate(&self) -> Result<(), ColdTierError> {
        if self.hot_min_score <= self.warm_min_score || self.warm_min_score <= self.cold_max_score {
            return Err(ColdTierError::InvalidTierThresholds);
        }
        Ok(())
    }

    fn target_tier(&self, score: u8) -> StorageTier {
        if score >= self.hot_min_score {
            StorageTier::Hot
        } else if score >= self.warm_min_score {
            StorageTier::Warm
        } else {
            StorageTier::Cold
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ColdShard {
    pub shard_id: u64,
    pub table: String,
    pub current_tier: StorageTier,
    pub temperature_score: u8,
    pub object_uri: String,
    pub format: ColdTierFormat,
    pub layers: Vec<LayerFile>,
}

impl ColdShard {
    fn validate(&self) -> Result<(), ColdTierError> {
        if self.shard_id == 0 {
            return Err(ColdTierError::InvalidShardId);
        }
        validate_qualified_name("shard.table", &self.table)?;
        validate_object_uri("shard.object_uri", &self.object_uri)?;
        if self.layers.is_empty() {
            return Err(ColdTierError::MissingRequiredField("shard.layers"));
        }
        for layer in &self.layers {
            layer.validate()?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum StorageTier {
    Hot,
    Warm,
    Cold,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum ColdTierFormat {
    Iceberg,
    Parquet,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct LayerFile {
    pub uri: String,
    pub kind: LayerKind,
    pub bytes: u64,
}

impl LayerFile {
    fn validate(&self) -> Result<(), ColdTierError> {
        validate_object_uri("layer.uri", &self.uri)?;
        if self.bytes == 0 {
            return Err(ColdTierError::InvalidLayerSize);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum LayerKind {
    Image,
    Delta,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SearchColdTierPlan {
    pub tantivy_index_uri: String,
    pub lancedb_index_uri: Option<String>,
    pub indexed_columns: Vec<String>,
}

impl SearchColdTierPlan {
    fn validate(&self) -> Result<(), ColdTierError> {
        validate_object_uri("search.tantivy_index_uri", &self.tantivy_index_uri)?;
        if let Some(uri) = &self.lancedb_index_uri {
            validate_object_uri("search.lancedb_index_uri", uri)?;
        }
        validate_required_list("search.indexed_columns", &self.indexed_columns)
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TierMovePlan {
    pub shard_id: u64,
    pub table: String,
    pub from: StorageTier,
    pub to: StorageTier,
    pub object_uri: String,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum ColdTierError {
    InvalidIdentifier(&'static str),
    InvalidLayerSize,
    LayerObjectMismatch,
    InvalidObjectUri(&'static str),
    InvalidShardId,
    InvalidTierThresholds,
    MissingRequiredField(&'static str),
}

impl fmt::Display for ColdTierError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidIdentifier(field) => write!(formatter, "{field} must be a SQL identifier"),
            Self::InvalidLayerSize => write!(formatter, "layer bytes must be greater than zero"),
            Self::LayerObjectMismatch => {
                write!(
                    formatter,
                    "layer URI must be stored below the shard object URI"
                )
            }
            Self::InvalidObjectUri(field) => write!(formatter, "{field} must be an object URI"),
            Self::InvalidShardId => write!(formatter, "shard_id must be greater than zero"),
            Self::InvalidTierThresholds => {
                write!(formatter, "tier thresholds must satisfy hot > warm > cold")
            }
            Self::MissingRequiredField(field) => write!(formatter, "{field} must not be empty"),
        }
    }
}

impl Error for ColdTierError {}

fn validate_required(field: &'static str, value: &str) -> Result<(), ColdTierError> {
    if value.trim().is_empty() {
        return Err(ColdTierError::MissingRequiredField(field));
    }
    Ok(())
}

fn validate_required_list(field: &'static str, values: &[String]) -> Result<(), ColdTierError> {
    if values.is_empty() || values.iter().any(|value| value.trim().is_empty()) {
        return Err(ColdTierError::MissingRequiredField(field));
    }
    Ok(())
}

fn validate_identifier(field: &'static str, value: &str) -> Result<(), ColdTierError> {
    validate_required(field, value)?;
    if value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
    {
        Ok(())
    } else {
        Err(ColdTierError::InvalidIdentifier(field))
    }
}

fn validate_qualified_name(field: &'static str, value: &str) -> Result<(), ColdTierError> {
    validate_required(field, value)?;
    let parts: Vec<_> = value.split('.').collect();
    if parts.len() == 2
        && parts
            .iter()
            .all(|part| validate_identifier(field, part).is_ok())
    {
        Ok(())
    } else {
        Err(ColdTierError::InvalidIdentifier(field))
    }
}

fn validate_object_uri(field: &'static str, value: &str) -> Result<(), ColdTierError> {
    validate_required(field, value)?;
    if value.starts_with("s3://") || value.starts_with("gs://") || value.starts_with("az://") {
        Ok(())
    } else {
        Err(ColdTierError::InvalidObjectUri(field))
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ColdTierMoveExecution {
    pub shard_id: u64,
    pub table: String,
    pub from: StorageTier,
    pub to: StorageTier,
    pub object_uri: String,
    pub format: ColdTierFormat,
    pub layer_count: usize,
    pub bytes_moved: u64,
    pub image_layers: usize,
    pub delta_layers: usize,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ColdTierPlannerRoute {
    pub shard_id: u64,
    pub table: String,
    pub tier: StorageTier,
    pub object_uri: String,
    pub format: ColdTierFormat,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SearchIndexMaterialization {
    pub index_uris: Vec<String>,
    pub indexed_columns: Vec<String>,
    pub searchable_shards: u64,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ColdTierRuntimeState {
    pub moved_shards: u64,
    pub materialized_layer_files: u64,
    pub object_bytes_written: u64,
    pub search_indexes_materialized: u64,
    pub planner_routes_refreshed: u64,
    pub cold_tier_reads: u64,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ColdTierRuntimeReport {
    pub moves: Vec<ColdTierMoveExecution>,
    pub planner_routes: Vec<ColdTierPlannerRoute>,
    pub search: Option<SearchIndexMaterialization>,
    pub state: ColdTierRuntimeState,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ColdTierRuntime {
    plan: ColdTierPlan,
    state: ColdTierRuntimeState,
}

impl ColdTierRuntime {
    pub fn new(plan: ColdTierPlan) -> Result<Self, ColdTierError> {
        plan.validate()?;

        Ok(Self {
            plan,
            state: ColdTierRuntimeState {
                moved_shards: 0,
                materialized_layer_files: 0,
                object_bytes_written: 0,
                search_indexes_materialized: 0,
                planner_routes_refreshed: 0,
                cold_tier_reads: 0,
            },
        })
    }

    pub fn state(&self) -> &ColdTierRuntimeState {
        &self.state
    }

    pub fn execute_tier_cycle(&mut self) -> Result<ColdTierRuntimeReport, ColdTierError> {
        self.plan.validate()?;
        self.ensure_layers_under_shards()?;

        let move_plans = self.plan.move_plans()?;
        let moves = move_plans
            .iter()
            .map(|move_plan| {
                let shard = self.find_shard(move_plan.shard_id);
                ColdTierMoveExecution {
                    shard_id: move_plan.shard_id,
                    table: move_plan.table.clone(),
                    from: move_plan.from,
                    to: move_plan.to,
                    object_uri: move_plan.object_uri.clone(),
                    format: shard.format,
                    layer_count: shard.layers.len(),
                    bytes_moved: shard.layers.iter().map(|layer| layer.bytes).sum(),
                    image_layers: shard
                        .layers
                        .iter()
                        .filter(|layer| layer.kind == LayerKind::Image)
                        .count(),
                    delta_layers: shard
                        .layers
                        .iter()
                        .filter(|layer| layer.kind == LayerKind::Delta)
                        .count(),
                }
            })
            .collect::<Vec<_>>();
        let planner_routes = moves
            .iter()
            .map(|move_execution| ColdTierPlannerRoute {
                shard_id: move_execution.shard_id,
                table: move_execution.table.clone(),
                tier: move_execution.to,
                object_uri: move_execution.object_uri.clone(),
                format: move_execution.format,
            })
            .collect::<Vec<_>>();
        let search = self
            .plan
            .search
            .as_ref()
            .map(|search_plan| SearchIndexMaterialization {
                index_uris: search_index_uris(search_plan),
                indexed_columns: search_plan.indexed_columns.clone(),
                searchable_shards: moves.len() as u64,
            });

        self.state.moved_shards += moves.len() as u64;
        self.state.materialized_layer_files += moves
            .iter()
            .map(|move_execution| move_execution.layer_count as u64)
            .sum::<u64>();
        self.state.object_bytes_written += moves
            .iter()
            .map(|move_execution| move_execution.bytes_moved)
            .sum::<u64>();
        self.state.search_indexes_materialized += search
            .as_ref()
            .map_or(0, |materialization| materialization.index_uris.len() as u64);
        self.state.planner_routes_refreshed += planner_routes.len() as u64;
        self.state.cold_tier_reads += planner_routes
            .iter()
            .filter(|route| route.tier == StorageTier::Cold)
            .count() as u64;

        Ok(ColdTierRuntimeReport {
            moves,
            planner_routes,
            search,
            state: self.state.clone(),
        })
    }

    fn find_shard(&self, shard_id: u64) -> &ColdShard {
        self.plan
            .shards
            .iter()
            .find(|shard| shard.shard_id == shard_id)
            .expect("move plan references a validated shard")
    }

    fn ensure_layers_under_shards(&self) -> Result<(), ColdTierError> {
        for shard in &self.plan.shards {
            let object_prefix = format!("{}/", shard.object_uri);
            if shard
                .layers
                .iter()
                .any(|layer| !layer.uri.starts_with(&object_prefix))
            {
                return Err(ColdTierError::LayerObjectMismatch);
            }
        }
        Ok(())
    }
}

fn search_index_uris(search_plan: &SearchColdTierPlan) -> Vec<String> {
    let mut uris = vec![search_plan.tantivy_index_uri.clone()];
    if let Some(uri) = &search_plan.lancedb_index_uri {
        uris.push(uri.clone());
    }
    uris
}

pub fn canonical_cold_tier_plan() -> ColdTierPlan {
    ColdTierPlan {
        policy: TierPolicy {
            hot_min_score: 80,
            warm_min_score: 40,
            cold_max_score: 20,
        },
        shards: vec![ColdShard {
            shard_id: 42,
            table: "public.events".to_string(),
            current_tier: StorageTier::Hot,
            temperature_score: 10,
            object_uri: "s3://cold-tier/events/42".to_string(),
            format: ColdTierFormat::Iceberg,
            layers: vec![
                LayerFile {
                    uri: "s3://cold-tier/events/42/image.parquet".to_string(),
                    kind: LayerKind::Image,
                    bytes: 1024,
                },
                LayerFile {
                    uri: "s3://cold-tier/events/42/delta-1.parquet".to_string(),
                    kind: LayerKind::Delta,
                    bytes: 128,
                },
            ],
        }],
        search: Some(SearchColdTierPlan {
            tantivy_index_uri: "s3://cold-tier/indexes/events".to_string(),
            lancedb_index_uri: Some("s3://cold-tier/indexes/events-vector".to_string()),
            indexed_columns: vec!["body".to_string(), "embedding".to_string()],
        }),
    }
}

pub fn canonical_move_plans() -> Result<Vec<TierMovePlan>, ColdTierError> {
    canonical_cold_tier_plan().move_plans()
}

pub fn canonical_cold_tier_runtime_report() -> Result<ColdTierRuntimeReport, ColdTierError> {
    let mut runtime = ColdTierRuntime::new(canonical_cold_tier_plan())?;
    runtime.execute_tier_cycle()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cold_tier_plan_calculates_move_plans() {
        let moves = canonical_cold_tier_plan().move_plans().expect("move plans");

        assert_eq!(
            moves,
            vec![TierMovePlan {
                shard_id: 42,
                table: "public.events".to_string(),
                from: StorageTier::Hot,
                to: StorageTier::Cold,
                object_uri: "s3://cold-tier/events/42".to_string(),
            }]
        );
    }

    #[test]
    fn canonical_move_plans_are_deterministic() {
        let moves = canonical_move_plans().expect("canonical moves");

        assert_eq!(moves.len(), 1);
        assert_eq!(moves[0].shard_id, 42);
        assert_eq!(moves[0].to, StorageTier::Cold);
    }

    #[test]
    fn cold_tier_runtime_materializes_pageserver_layers_and_search_indexes() {
        let report = canonical_cold_tier_runtime_report().expect("runtime report");

        assert_eq!(report.moves.len(), 1);
        assert_eq!(report.moves[0].shard_id, 42);
        assert_eq!(report.moves[0].from, StorageTier::Hot);
        assert_eq!(report.moves[0].to, StorageTier::Cold);
        assert_eq!(report.moves[0].format, ColdTierFormat::Iceberg);
        assert_eq!(report.moves[0].layer_count, 2);
        assert_eq!(report.moves[0].bytes_moved, 1_152);
        assert_eq!(report.moves[0].image_layers, 1);
        assert_eq!(report.moves[0].delta_layers, 1);
        assert_eq!(report.planner_routes[0].tier, StorageTier::Cold);
        assert_eq!(report.search.as_ref().expect("search").index_uris.len(), 2);
        assert_eq!(
            report.search.as_ref().expect("search").indexed_columns,
            ["body", "embedding"]
        );
        assert_eq!(report.state.moved_shards, 1);
        assert_eq!(report.state.materialized_layer_files, 2);
        assert_eq!(report.state.object_bytes_written, 1_152);
        assert_eq!(report.state.search_indexes_materialized, 2);
        assert_eq!(report.state.planner_routes_refreshed, 1);
        assert_eq!(report.state.cold_tier_reads, 1);
    }

    #[test]
    fn cold_tier_runtime_rejects_layer_outside_shard_object_prefix() {
        let mut plan = canonical_cold_tier_plan();
        plan.shards[0].layers[0].uri = "s3://cold-tier/other/image.parquet".to_string();
        let mut runtime = ColdTierRuntime::new(plan).expect("runtime");

        assert_eq!(
            runtime.execute_tier_cycle(),
            Err(ColdTierError::LayerObjectMismatch)
        );
    }

    #[test]
    fn policy_requires_ordered_thresholds() {
        let mut plan = canonical_cold_tier_plan();
        plan.policy = TierPolicy {
            hot_min_score: 50,
            warm_min_score: 75,
            cold_max_score: 20,
        };

        assert_eq!(plan.validate(), Err(ColdTierError::InvalidTierThresholds));
    }

    #[test]
    fn layer_requires_positive_size() {
        let mut plan = canonical_cold_tier_plan();
        plan.shards[0].layers[0].bytes = 0;

        assert_eq!(plan.validate(), Err(ColdTierError::InvalidLayerSize));
    }

    #[test]
    fn search_plan_requires_indexed_columns() {
        let mut plan = canonical_cold_tier_plan();
        plan.search = Some(SearchColdTierPlan {
            tantivy_index_uri: "s3://cold-tier/indexes/events".to_string(),
            lancedb_index_uri: None,
            indexed_columns: Vec::new(),
        });

        assert_eq!(
            plan.validate(),
            Err(ColdTierError::MissingRequiredField(
                "search.indexed_columns"
            ))
        );
    }
}
