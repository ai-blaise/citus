//! Cold-tier sidecar contracts.

// FEATURE: R1
// FEATURE: R5
// FEATURE: R9
// FEATURE: Search8

use std::error::Error;
use std::fmt;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

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
        validate_search_index_uri("search.tantivy_index_uri", &self.tantivy_index_uri)?;
        if let Some(uri) = &self.lancedb_index_uri {
            validate_object_uri("search.lancedb_index_uri", uri)?;
            validate_search_index_uri("search.lancedb_index_uri", uri)?;
        }
        validate_required_list("search.indexed_columns", &self.indexed_columns)?;
        validate_identifier_list("search.indexed_columns", &self.indexed_columns)
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
    InvalidSearchIndexUri(&'static str),
    InvalidUriPath(&'static str),
    InvalidTierThresholds,
    MissingRequiredField(&'static str),
    UnsupportedMaterializationUri(String),
    Io(String),
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
            Self::InvalidSearchIndexUri(field) => {
                write!(formatter, "{field} must end with a search index directory")
            }
            Self::InvalidUriPath(field) => write!(formatter, "{field} contains an unsafe URI path"),
            Self::InvalidTierThresholds => {
                write!(formatter, "tier thresholds must satisfy hot > warm > cold")
            }
            Self::MissingRequiredField(field) => write!(formatter, "{field} must not be empty"),
            Self::UnsupportedMaterializationUri(uri) => write!(
                formatter,
                "materialization supports only local file:// artifact URIs, got {uri}"
            ),
            Self::Io(error) => write!(formatter, "{error}"),
        }
    }
}

impl Error for ColdTierError {}

impl From<std::io::Error> for ColdTierError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error.to_string())
    }
}

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

fn validate_identifier_list(field: &'static str, values: &[String]) -> Result<(), ColdTierError> {
    for value in values {
        validate_identifier(field, value)?;
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
    let Some((scheme, path)) = value.split_once("://") else {
        return Err(ColdTierError::InvalidObjectUri(field));
    };
    if !matches!(scheme, "file" | "s3" | "gs" | "az") {
        return Err(ColdTierError::InvalidObjectUri(field));
    }
    if path.trim().is_empty()
        || path.contains("..")
        || path.contains(' ')
        || path.contains("//")
        || path.contains('\0')
        || path.chars().any(|ch| ch.is_ascii_control())
    {
        return Err(ColdTierError::InvalidUriPath(field));
    }
    if scheme == "file" && !path.starts_with('/') {
        return Err(ColdTierError::InvalidUriPath(field));
    }
    Ok(())
}

fn validate_search_index_uri(field: &'static str, value: &str) -> Result<(), ColdTierError> {
    if value.ends_with(".tantivy")
        || value.ends_with(".lance")
        || value.ends_with(".lancedb")
        || value.contains("/indexes/")
    {
        Ok(())
    } else {
        Err(ColdTierError::InvalidSearchIndexUri(field))
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

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum ColdTierArtifactKind {
    Layer,
    SearchIndex,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ColdTierArtifact {
    pub uri: String,
    pub kind: ColdTierArtifactKind,
    pub bytes: u64,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ColdTierRuntimeState {
    pub moved_shards: u64,
    pub materialized_layer_files: u64,
    pub object_bytes_written: u64,
    pub search_indexes_materialized: u64,
    pub planner_routes_refreshed: u64,
    pub cold_tier_reads: u64,
    pub search_index_bytes_written: u64,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ColdTierRuntimeReport {
    pub moves: Vec<ColdTierMoveExecution>,
    pub planner_routes: Vec<ColdTierPlannerRoute>,
    pub search: Option<SearchIndexMaterialization>,
    pub artifacts: Vec<ColdTierArtifact>,
    pub state: ColdTierRuntimeState,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ColdTierMaterializationReport {
    pub artifact_count: u64,
    pub bytes_written: u64,
    pub file_paths: Vec<String>,
}

pub fn materialize_file_artifacts(
    report: &ColdTierRuntimeReport,
) -> Result<ColdTierMaterializationReport, ColdTierError> {
    let mut materialized = ColdTierMaterializationReport {
        artifact_count: 0,
        bytes_written: 0,
        file_paths: Vec::new(),
    };

    for artifact in &report.artifacts {
        let path = file_path_from_uri(&artifact.uri)?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        write_deterministic_artifact(&path, artifact)?;
        let bytes_written = fs::metadata(&path)?.len();
        if bytes_written != artifact.bytes {
            return Err(ColdTierError::Io(format!(
                "materialized artifact {} has {} bytes, expected {}",
                artifact.uri, bytes_written, artifact.bytes
            )));
        }
        materialized.artifact_count += 1;
        materialized.bytes_written += bytes_written;
        materialized
            .file_paths
            .push(path.to_string_lossy().to_string());
    }

    Ok(materialized)
}

fn file_path_from_uri(uri: &str) -> Result<PathBuf, ColdTierError> {
    validate_object_uri("artifact.uri", uri)?;
    let Some(path) = uri.strip_prefix("file://") else {
        return Err(ColdTierError::UnsupportedMaterializationUri(
            uri.to_string(),
        ));
    };
    if path.ends_with('/') {
        return Err(ColdTierError::InvalidUriPath("artifact.uri"));
    }
    Ok(PathBuf::from(path))
}

fn write_deterministic_artifact(
    path: &Path,
    artifact: &ColdTierArtifact,
) -> Result<(), ColdTierError> {
    let mut file = fs::File::create(path)?;
    let kind = match artifact.kind {
        ColdTierArtifactKind::Layer => "layer",
        ColdTierArtifactKind::SearchIndex => "search-index",
    };
    let marker = format!("ai-blaise-coldtier\nkind={kind}\nuri={}\n", artifact.uri);
    let marker = marker.as_bytes();
    let mut remaining = artifact.bytes;
    while remaining > 0 {
        let chunk_len = marker.len().min(remaining as usize);
        file.write_all(&marker[..chunk_len])?;
        remaining -= chunk_len as u64;
    }
    file.sync_all()?;
    Ok(())
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
                search_index_bytes_written: 0,
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

        let mut artifacts = Vec::new();
        for move_execution in &moves {
            let shard = self.find_shard(move_execution.shard_id);
            artifacts.extend(shard.layers.iter().map(|layer| ColdTierArtifact {
                uri: layer.uri.clone(),
                kind: ColdTierArtifactKind::Layer,
                bytes: layer.bytes,
            }));
        }
        if let Some(search_materialization) = &search {
            artifacts.extend(search_materialization.index_uris.iter().map(|uri| {
                ColdTierArtifact {
                    uri: uri.clone(),
                    kind: ColdTierArtifactKind::SearchIndex,
                    bytes: search_artifact_bytes(search_materialization),
                }
            }));
        }

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
            .map(|materialization| materialization.index_uris.len() as u64)
            .unwrap_or(0);
        self.state.planner_routes_refreshed += planner_routes.len() as u64;
        self.state.cold_tier_reads += planner_routes
            .iter()
            .filter(|route| route.tier == StorageTier::Cold)
            .count() as u64;
        self.state.search_index_bytes_written += artifacts
            .iter()
            .filter(|artifact| artifact.kind == ColdTierArtifactKind::SearchIndex)
            .map(|artifact| artifact.bytes)
            .sum::<u64>();

        Ok(ColdTierRuntimeReport {
            moves,
            planner_routes,
            search,
            artifacts,
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

fn search_artifact_bytes(search: &SearchIndexMaterialization) -> u64 {
    64 * search.indexed_columns.len() as u64 * search.searchable_shards.max(1)
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
            object_uri: "file:///tmp/ai-blaise-coldtier/events/42".to_string(),
            format: ColdTierFormat::Iceberg,
            layers: vec![
                LayerFile {
                    uri: "file:///tmp/ai-blaise-coldtier/events/42/image.parquet".to_string(),
                    kind: LayerKind::Image,
                    bytes: 1024,
                },
                LayerFile {
                    uri: "file:///tmp/ai-blaise-coldtier/events/42/delta-1.parquet".to_string(),
                    kind: LayerKind::Delta,
                    bytes: 128,
                },
            ],
        }],
        search: Some(SearchColdTierPlan {
            tantivy_index_uri: "file:///tmp/ai-blaise-coldtier/indexes/events.tantivy".to_string(),
            lancedb_index_uri: Some(
                "file:///tmp/ai-blaise-coldtier/indexes/events.lance".to_string(),
            ),
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
                object_uri: "file:///tmp/ai-blaise-coldtier/events/42".to_string(),
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
        assert_eq!(report.artifacts.len(), 4);
        assert_eq!(
            report
                .artifacts
                .iter()
                .filter(|artifact| artifact.kind == ColdTierArtifactKind::SearchIndex)
                .count(),
            2
        );
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
        assert_eq!(report.state.search_index_bytes_written, 256);
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
            tantivy_index_uri: "file:///tmp/ai-blaise-coldtier/indexes/events.tantivy".to_string(),
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

    #[test]
    fn cold_tier_rejects_unsafe_file_uri_paths() {
        let mut plan = canonical_cold_tier_plan();
        plan.shards[0].object_uri = "file://relative/events/42".to_string();

        assert_eq!(
            plan.validate(),
            Err(ColdTierError::InvalidUriPath("shard.object_uri"))
        );

        plan.shards[0].object_uri = "file:///tmp/ai-blaise-coldtier/events\n42".to_string();
        assert_eq!(
            plan.validate(),
            Err(ColdTierError::InvalidUriPath("shard.object_uri"))
        );
    }

    #[test]
    fn search_plan_rejects_non_identifier_columns() {
        let mut plan = canonical_cold_tier_plan();
        plan.search.as_mut().expect("search").indexed_columns = vec!["body;drop".to_string()];

        assert_eq!(
            plan.validate(),
            Err(ColdTierError::InvalidIdentifier("search.indexed_columns"))
        );
    }

    #[test]
    fn search_plan_rejects_untyped_index_uri() {
        let mut plan = canonical_cold_tier_plan();
        plan.search.as_mut().expect("search").tantivy_index_uri =
            "file:///tmp/cold/events".to_string();

        assert_eq!(
            plan.validate(),
            Err(ColdTierError::InvalidSearchIndexUri(
                "search.tantivy_index_uri"
            ))
        );
    }

    #[test]
    fn cold_tier_materializes_local_file_artifacts() {
        let root =
            std::env::temp_dir().join(format!("ai-blaise-coldtier-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let root_uri = format!("file://{}", root.to_string_lossy());
        let mut plan = canonical_cold_tier_plan();
        plan.shards[0].object_uri = format!("{root_uri}/events/42");
        plan.shards[0].layers[0].uri = format!("{root_uri}/events/42/image.parquet");
        plan.shards[0].layers[1].uri = format!("{root_uri}/events/42/delta-1.parquet");
        let search = plan.search.as_mut().expect("search plan");
        search.tantivy_index_uri = format!("{root_uri}/indexes/events.tantivy");
        search.lancedb_index_uri = Some(format!("{root_uri}/indexes/events.lance"));

        let mut runtime = ColdTierRuntime::new(plan).expect("runtime");
        let report = runtime.execute_tier_cycle().expect("cycle");
        let materialized = materialize_file_artifacts(&report).expect("materialized");

        assert_eq!(materialized.artifact_count, 4);
        assert_eq!(materialized.bytes_written, 1_408);
        assert_eq!(
            fs::metadata(root.join("events/42/image.parquet"))
                .unwrap()
                .len(),
            1_024
        );
        assert_eq!(
            fs::metadata(root.join("events/42/delta-1.parquet"))
                .unwrap()
                .len(),
            128
        );
        assert_eq!(
            fs::metadata(root.join("indexes/events.tantivy"))
                .unwrap()
                .len(),
            128
        );
        assert_eq!(
            fs::metadata(root.join("indexes/events.lance"))
                .unwrap()
                .len(),
            128
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn cold_tier_materialization_rejects_directory_artifact_paths() {
        let report = ColdTierRuntimeReport {
            moves: Vec::new(),
            planner_routes: Vec::new(),
            search: None,
            artifacts: vec![ColdTierArtifact {
                uri: "file:///tmp/ai-blaise-coldtier/events/42/".to_string(),
                kind: ColdTierArtifactKind::Layer,
                bytes: 1,
            }],
            state: ColdTierRuntimeState {
                moved_shards: 0,
                materialized_layer_files: 0,
                object_bytes_written: 0,
                search_indexes_materialized: 0,
                planner_routes_refreshed: 0,
                cold_tier_reads: 0,
                search_index_bytes_written: 0,
            },
        };

        assert_eq!(
            materialize_file_artifacts(&report),
            Err(ColdTierError::InvalidUriPath("artifact.uri"))
        );
    }

    #[test]
    fn cold_tier_materialization_rejects_non_file_artifacts() {
        let report = ColdTierRuntimeReport {
            moves: Vec::new(),
            planner_routes: Vec::new(),
            search: None,
            artifacts: vec![ColdTierArtifact {
                uri: "s3://bucket/events/42/image.parquet".to_string(),
                kind: ColdTierArtifactKind::Layer,
                bytes: 1,
            }],
            state: ColdTierRuntimeState {
                moved_shards: 0,
                materialized_layer_files: 0,
                object_bytes_written: 0,
                search_indexes_materialized: 0,
                planner_routes_refreshed: 0,
                cold_tier_reads: 0,
                search_index_bytes_written: 0,
            },
        };

        assert!(matches!(
            materialize_file_artifacts(&report),
            Err(ColdTierError::UnsupportedMaterializationUri(_))
        ));
    }
}
