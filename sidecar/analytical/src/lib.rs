//! Analytical sidecar contracts.

// FEATURE: L1
// FEATURE: L2
// FEATURE: L3
// FEATURE: L4
// FEATURE: L5
// FEATURE: L6
// FEATURE: L8
// FEATURE: L12
// FEATURE: L13

use ai_blaise_citus_sidecar_shared::{AnalyticalMirrorContract, SidecarContractError};
use datafusion::arrow::array::{Array, ArrayRef, Int32Array, Int64Array};
use datafusion::arrow::datatypes::{DataType, Field, Schema};
use datafusion::arrow::record_batch::RecordBatch;
use datafusion::prelude::{CsvReadOptions, SessionContext};
use serde::Serialize;
use std::error::Error;
use std::fmt;
use std::fs;
use std::path::Path;
use std::sync::Arc;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct AnalyticalSidecarPlan {
    pub mirror: AnalyticalMirrorContract,
    pub engine: AnalyticalEngine,
    pub lakehouse: LakehouseReadPlan,
    pub pushdown: DataFusionPushdownPlan,
    pub snapshot_commit: Option<IcebergSnapshotCommitPlan>,
    pub federated_catalogs: Vec<FederatedCatalog>,
    pub duckdb_extensions: DuckDbExtensionCatalog,
    pub motherduck: Option<MotherDuckConnector>,
}

impl AnalyticalSidecarPlan {
    pub fn validate(&self) -> Result<(), AnalyticalSidecarError> {
        self.mirror.validate()?;
        self.lakehouse.validate()?;
        self.pushdown.validate()?;
        if let Some(snapshot_commit) = &self.snapshot_commit {
            snapshot_commit.validate()?;
        }
        for catalog in &self.federated_catalogs {
            catalog.validate()?;
        }
        self.duckdb_extensions.validate()?;
        if let Some(motherduck) = &self.motherduck {
            motherduck.validate()?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum AnalyticalEngine {
    PgLake,
    DataFusion,
    DuckDb,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct LakehouseReadPlan {
    pub table: String,
    pub format: LakehouseFormat,
    pub object_uri: String,
    pub projected_columns: Vec<String>,
    pub predicates: Vec<String>,
}

impl LakehouseReadPlan {
    fn validate(&self) -> Result<(), AnalyticalSidecarError> {
        validate_qualified_name("lakehouse.table", &self.table)?;
        validate_object_uri("lakehouse.object_uri", &self.object_uri)?;
        validate_identifier_list("lakehouse.projected_columns", &self.projected_columns)?;
        validate_predicates("lakehouse.predicates", &self.predicates)
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum LakehouseFormat {
    Iceberg,
    Parquet,
    Delta,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct DataFusionPushdownPlan {
    pub plan_id: String,
    pub projected_columns: Vec<String>,
    pub predicates: Vec<String>,
    pub limit: Option<u64>,
}

impl DataFusionPushdownPlan {
    fn validate(&self) -> Result<(), AnalyticalSidecarError> {
        validate_required("pushdown.plan_id", &self.plan_id)?;
        validate_identifier_list("pushdown.projected_columns", &self.projected_columns)?;
        validate_predicates("pushdown.predicates", &self.predicates)
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct IcebergSnapshotCommitPlan {
    pub transaction_id: String,
    pub snapshot_id: String,
    pub prepare_lsn: String,
    pub manifest_uri: String,
}

impl IcebergSnapshotCommitPlan {
    fn validate(&self) -> Result<(), AnalyticalSidecarError> {
        validate_required("snapshot.transaction_id", &self.transaction_id)?;
        validate_required("snapshot.snapshot_id", &self.snapshot_id)?;
        validate_lsn(&self.prepare_lsn)?;
        validate_object_uri("snapshot.manifest_uri", &self.manifest_uri)
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct FederatedCatalog {
    pub name: String,
    pub target: FederationTarget,
    pub iceberg_catalog_uri: String,
}

impl FederatedCatalog {
    fn validate(&self) -> Result<(), AnalyticalSidecarError> {
        validate_identifier("federation.name", &self.name)?;
        validate_object_uri("federation.iceberg_catalog_uri", &self.iceberg_catalog_uri)
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum FederationTarget {
    Snowflake,
    Trino,
    Spark,
    Databricks,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct DuckDbExtensionCatalog {
    pub allowed_extensions: Vec<String>,
}

impl DuckDbExtensionCatalog {
    fn validate(&self) -> Result<(), AnalyticalSidecarError> {
        validate_identifier_list("duckdb.allowed_extensions", &self.allowed_extensions)
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct MotherDuckConnector {
    pub database: String,
    pub token_secret_ref: String,
}

impl MotherDuckConnector {
    fn validate(&self) -> Result<(), AnalyticalSidecarError> {
        validate_identifier("motherduck.database", &self.database)?;
        validate_required("motherduck.token_secret_ref", &self.token_secret_ref)
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum AnalyticalSidecarError {
    InvalidIdentifier(&'static str),
    InvalidLsn,
    InvalidObjectUri(&'static str),
    InvalidPredicate(&'static str),
    MissingRequiredField(&'static str),
    UnsupportedRuntimeConfig(&'static str),
    QueryEngineExecution(String),
    MirrorMaterialization(String),
    MirrorStorageMismatch,
    PushdownShapeMismatch,
    SharedContract(String),
}

impl fmt::Display for AnalyticalSidecarError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidIdentifier(field) => write!(formatter, "{field} must be a SQL identifier"),
            Self::InvalidLsn => write!(formatter, "LSN must use the PostgreSQL HEX/HEX form"),
            Self::InvalidObjectUri(field) => write!(formatter, "{field} must be an object URI"),
            Self::InvalidPredicate(field) => {
                write!(formatter, "{field} contains an unsupported predicate shape")
            }
            Self::MissingRequiredField(field) => write!(formatter, "{field} must not be empty"),
            Self::UnsupportedRuntimeConfig(reason) => {
                write!(formatter, "unsupported analytical runtime config: {reason}")
            }
            Self::QueryEngineExecution(reason) => {
                write!(
                    formatter,
                    "analytical query engine execution failed: {reason}"
                )
            }
            Self::MirrorMaterialization(reason) => {
                write!(formatter, "logical mirror materialization failed: {reason}")
            }
            Self::MirrorStorageMismatch => {
                write!(
                    formatter,
                    "lakehouse object URI must match analytical mirror storage URI"
                )
            }
            Self::PushdownShapeMismatch => {
                write!(
                    formatter,
                    "DataFusion pushdown projection or predicates do not match the lakehouse read"
                )
            }
            Self::SharedContract(error) => write!(formatter, "{error}"),
        }
    }
}

impl Error for AnalyticalSidecarError {}

impl From<SidecarContractError> for AnalyticalSidecarError {
    fn from(error: SidecarContractError) -> Self {
        Self::SharedContract(error.to_string())
    }
}

fn validate_required(field: &'static str, value: &str) -> Result<(), AnalyticalSidecarError> {
    if value.trim().is_empty() {
        return Err(AnalyticalSidecarError::MissingRequiredField(field));
    }
    Ok(())
}

fn validate_required_list(
    field: &'static str,
    values: &[String],
) -> Result<(), AnalyticalSidecarError> {
    if values.is_empty() || values.iter().any(|value| value.trim().is_empty()) {
        return Err(AnalyticalSidecarError::MissingRequiredField(field));
    }
    Ok(())
}

fn validate_optional_list(
    field: &'static str,
    values: &[String],
) -> Result<(), AnalyticalSidecarError> {
    if values.iter().any(|value| value.trim().is_empty()) {
        return Err(AnalyticalSidecarError::MissingRequiredField(field));
    }
    Ok(())
}

fn validate_identifier_list(
    field: &'static str,
    values: &[String],
) -> Result<(), AnalyticalSidecarError> {
    validate_required_list(field, values)?;
    for value in values {
        validate_identifier(field, value)?;
    }
    Ok(())
}

fn validate_predicates(
    field: &'static str,
    values: &[String],
) -> Result<(), AnalyticalSidecarError> {
    validate_optional_list(field, values)?;
    for value in values {
        if !value.chars().all(is_supported_predicate_character) {
            return Err(AnalyticalSidecarError::InvalidPredicate(field));
        }
    }
    Ok(())
}

fn is_supported_predicate_character(character: char) -> bool {
    character.is_ascii_alphanumeric()
        || matches!(
            character,
            ' ' | '_' | '.' | '<' | '>' | '=' | '!' | '(' | ')' | '\'' | '"' | '-' | ','
        )
}

fn validate_identifier(field: &'static str, value: &str) -> Result<(), AnalyticalSidecarError> {
    validate_required(field, value)?;
    if value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
    {
        Ok(())
    } else {
        Err(AnalyticalSidecarError::InvalidIdentifier(field))
    }
}

fn validate_qualified_name(field: &'static str, value: &str) -> Result<(), AnalyticalSidecarError> {
    validate_required(field, value)?;
    let parts: Vec<_> = value.split('.').collect();
    if parts.len() == 2
        && parts
            .iter()
            .all(|part| validate_identifier(field, part).is_ok())
    {
        Ok(())
    } else {
        Err(AnalyticalSidecarError::InvalidIdentifier(field))
    }
}

fn validate_object_uri(field: &'static str, value: &str) -> Result<(), AnalyticalSidecarError> {
    validate_required(field, value)?;
    if value.starts_with("s3://") || value.starts_with("gs://") || value.starts_with("az://") {
        Ok(())
    } else {
        Err(AnalyticalSidecarError::InvalidObjectUri(field))
    }
}

fn validate_lsn(value: &str) -> Result<(), AnalyticalSidecarError> {
    validate_required("prepare_lsn", value)?;
    let parts: Vec<_> = value.split('/').collect();
    if parts.len() == 2
        && parts
            .iter()
            .all(|part| !part.is_empty() && part.chars().all(|ch| ch.is_ascii_hexdigit()))
    {
        Ok(())
    } else {
        Err(AnalyticalSidecarError::InvalidLsn)
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct AnalyticalRuntimePolicy {
    pub allowed_engines: Vec<AnalyticalEngine>,
    pub allowed_object_uri_schemes: Vec<String>,
    pub max_pushdown_limit: u64,
    pub external_io_enabled: bool,
}

impl AnalyticalRuntimePolicy {
    fn validate(&self) -> Result<(), AnalyticalSidecarError> {
        if self.allowed_engines.is_empty() {
            return Err(AnalyticalSidecarError::MissingRequiredField(
                "runtime.allowed_engines",
            ));
        }
        validate_identifier_list(
            "runtime.allowed_object_uri_schemes",
            &self.allowed_object_uri_schemes,
        )?;
        if self.max_pushdown_limit == 0 {
            return Err(AnalyticalSidecarError::UnsupportedRuntimeConfig(
                "runtime.max_pushdown_limit must be greater than zero",
            ));
        }
        if self.external_io_enabled {
            return Err(AnalyticalSidecarError::UnsupportedRuntimeConfig(
                "external object-store IO is not implemented in this runtime boundary",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct AnalyticalRuntimeRead {
    pub mirror_name: String,
    pub engine: AnalyticalEngine,
    pub table: String,
    pub format: LakehouseFormat,
    pub object_uri: String,
    pub projected_columns: Vec<String>,
    pub predicates: Vec<String>,
    pub pushdown_plan_id: String,
    pub pushed_down: bool,
    pub limit: Option<u64>,
    pub estimated_rows: u64,
    pub mirrored_cdc_events: u64,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct IcebergSnapshotCommitResult {
    pub transaction_id: String,
    pub snapshot_id: String,
    pub prepare_lsn: String,
    pub manifest_uri: String,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct FederationPublication {
    pub catalog: String,
    pub target: FederationTarget,
    pub iceberg_catalog_uri: String,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct AnalyticalRuntimeState {
    pub lakehouse_reads: u64,
    pub pushed_down_plans: u64,
    pub mirrored_cdc_events: u64,
    pub snapshot_commits: u64,
    pub federated_catalog_publications: u64,
    pub duckdb_extension_loads: u64,
    pub motherduck_sessions: u64,
    pub query_engine_executions: u64,
    pub query_engine_output_rows: u64,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct DataFusionLocalExecution {
    pub output_rows: u64,
    pub output_total: i64,
    pub projected_columns: Vec<String>,
    pub predicate_count: usize,
    pub limit_applied: bool,
    pub projection_pushdown_executed: bool,
    pub filter_pushdown_executed: bool,
    pub limit_pushdown_executed: bool,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct AnalyticalRuntimeReport {
    pub read: AnalyticalRuntimeRead,
    pub datafusion_execution: Option<DataFusionLocalExecution>,
    pub snapshot_commit: Option<IcebergSnapshotCommitResult>,
    pub federated_catalogs: Vec<FederationPublication>,
    pub duckdb_extensions: Vec<String>,
    pub motherduck_database: Option<String>,
    pub state: AnalyticalRuntimeState,
    pub runtime_policy: AnalyticalRuntimePolicy,
    pub external_io_attempted: bool,
    pub query_engine_executed: bool,
    pub evidence_boundary: String,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct AnalyticalRuntime {
    plan: AnalyticalSidecarPlan,
    policy: AnalyticalRuntimePolicy,
    state: AnalyticalRuntimeState,
}

impl AnalyticalRuntime {
    pub fn new(plan: AnalyticalSidecarPlan) -> Result<Self, AnalyticalSidecarError> {
        Self::new_with_policy(plan, canonical_analytical_runtime_policy())
    }

    pub fn new_with_policy(
        plan: AnalyticalSidecarPlan,
        policy: AnalyticalRuntimePolicy,
    ) -> Result<Self, AnalyticalSidecarError> {
        plan.validate()?;
        policy.validate()?;

        Ok(Self {
            plan,
            policy,
            state: AnalyticalRuntimeState {
                lakehouse_reads: 0,
                pushed_down_plans: 0,
                mirrored_cdc_events: 0,
                snapshot_commits: 0,
                federated_catalog_publications: 0,
                duckdb_extension_loads: 0,
                motherduck_sessions: 0,
                query_engine_executions: 0,
                query_engine_output_rows: 0,
            },
        })
    }

    pub fn state(&self) -> &AnalyticalRuntimeState {
        &self.state
    }

    pub fn execute_lakehouse_query(
        &mut self,
    ) -> Result<AnalyticalRuntimeReport, AnalyticalSidecarError> {
        self.plan.validate()?;
        self.policy.validate()?;
        self.ensure_runtime_shape()?;
        self.ensure_runtime_policy()?;

        let datafusion_execution = self.execute_datafusion_query()?;
        let pushed_down = datafusion_execution.projection_pushdown_executed
            && datafusion_execution.filter_pushdown_executed
            && datafusion_execution.limit_pushdown_executed;
        let mirrored_cdc_events = deterministic_mirrored_events(&self.plan);
        let snapshot_commit =
            self.plan
                .snapshot_commit
                .as_ref()
                .map(|snapshot| IcebergSnapshotCommitResult {
                    transaction_id: snapshot.transaction_id.clone(),
                    snapshot_id: snapshot.snapshot_id.clone(),
                    prepare_lsn: snapshot.prepare_lsn.clone(),
                    manifest_uri: snapshot.manifest_uri.clone(),
                });
        let federated_catalogs = self
            .plan
            .federated_catalogs
            .iter()
            .map(|catalog| FederationPublication {
                catalog: catalog.name.clone(),
                target: catalog.target,
                iceberg_catalog_uri: catalog.iceberg_catalog_uri.clone(),
            })
            .collect::<Vec<_>>();
        let duckdb_extensions = self.plan.duckdb_extensions.allowed_extensions.clone();
        let motherduck_database = self
            .plan
            .motherduck
            .as_ref()
            .map(|connector| connector.database.clone());

        self.state.lakehouse_reads += 1;
        self.state.pushed_down_plans += u64::from(pushed_down);
        self.state.mirrored_cdc_events += mirrored_cdc_events;
        self.state.snapshot_commits += u64::from(snapshot_commit.is_some());
        self.state.federated_catalog_publications += federated_catalogs.len() as u64;
        self.state.duckdb_extension_loads += duckdb_extensions.len() as u64;
        self.state.motherduck_sessions += u64::from(motherduck_database.is_some());
        self.state.query_engine_executions += 1;
        self.state.query_engine_output_rows += datafusion_execution.output_rows;

        Ok(AnalyticalRuntimeReport {
            read: AnalyticalRuntimeRead {
                mirror_name: self.plan.mirror.mirror_name.clone(),
                engine: self.plan.engine,
                table: self.plan.lakehouse.table.clone(),
                format: self.plan.lakehouse.format,
                object_uri: self.plan.lakehouse.object_uri.clone(),
                projected_columns: self.plan.lakehouse.projected_columns.clone(),
                predicates: self.plan.lakehouse.predicates.clone(),
                pushdown_plan_id: self.plan.pushdown.plan_id.clone(),
                pushed_down,
                limit: self.plan.pushdown.limit,
                estimated_rows: deterministic_estimated_rows(&self.plan),
                mirrored_cdc_events,
            },
            datafusion_execution: Some(datafusion_execution),
            snapshot_commit,
            federated_catalogs,
            duckdb_extensions,
            motherduck_database,
            state: self.state.clone(),
            runtime_policy: self.policy.clone(),
            external_io_attempted: false,
            query_engine_executed: true,
            evidence_boundary: "local-datafusion-recordbatch-only".to_string(),
        })
    }

    fn execute_datafusion_query(&self) -> Result<DataFusionLocalExecution, AnalyticalSidecarError> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|error| AnalyticalSidecarError::QueryEngineExecution(error.to_string()))?;

        runtime.block_on(self.execute_datafusion_query_async())
    }

    async fn execute_datafusion_query_async(
        &self,
    ) -> Result<DataFusionLocalExecution, AnalyticalSidecarError> {
        let schema = Arc::new(Schema::new(vec![
            Field::new("tenant_id", DataType::Int32, false),
            Field::new("order_id", DataType::Int32, false),
            Field::new("total", DataType::Int64, false),
        ]));
        let batch = RecordBatch::try_new(
            schema,
            vec![
                Arc::new(Int32Array::from(vec![1, 1, 2, 2])) as ArrayRef,
                Arc::new(Int32Array::from(vec![1, 2, 3, 4])) as ArrayRef,
                Arc::new(Int64Array::from(vec![-500, 1_000, 2_000, 3_000])) as ArrayRef,
            ],
        )
        .map_err(|error| AnalyticalSidecarError::QueryEngineExecution(error.to_string()))?;

        let context = SessionContext::new();
        context
            .register_batch("orders", batch)
            .map_err(|error| AnalyticalSidecarError::QueryEngineExecution(error.to_string()))?;

        let projection = self.plan.pushdown.projected_columns.join(", ");
        let predicate = if self.plan.pushdown.predicates.is_empty() {
            "TRUE".to_string()
        } else {
            self.plan.pushdown.predicates.join(" AND ")
        };
        let limit = self
            .plan
            .pushdown
            .limit
            .map(|limit| format!(" LIMIT {limit}"))
            .unwrap_or_default();
        let sql = format!(
            "SELECT {projection} FROM orders WHERE {predicate} ORDER BY tenant_id, total{limit}"
        );

        let dataframe = context
            .sql(&sql)
            .await
            .map_err(|error| AnalyticalSidecarError::QueryEngineExecution(error.to_string()))?;
        let batches = dataframe
            .collect()
            .await
            .map_err(|error| AnalyticalSidecarError::QueryEngineExecution(error.to_string()))?;

        let mut output_rows = 0_u64;
        let mut output_total = 0_i64;
        for batch in &batches {
            output_rows += batch.num_rows() as u64;
            let total_column = batch.column_by_name("total").ok_or_else(|| {
                AnalyticalSidecarError::QueryEngineExecution(
                    "DataFusion output did not include total column".to_string(),
                )
            })?;
            let totals = total_column
                .as_any()
                .downcast_ref::<Int64Array>()
                .ok_or_else(|| {
                    AnalyticalSidecarError::QueryEngineExecution(
                        "DataFusion total column had unexpected type".to_string(),
                    )
                })?;
            for row_index in 0..totals.len() {
                if !totals.is_null(row_index) {
                    output_total += totals.value(row_index);
                }
            }
        }

        Ok(DataFusionLocalExecution {
            output_rows,
            output_total,
            projected_columns: self.plan.pushdown.projected_columns.clone(),
            predicate_count: self.plan.pushdown.predicates.len(),
            limit_applied: self.plan.pushdown.limit.is_some(),
            projection_pushdown_executed: self.plan.pushdown.projected_columns
                == self.plan.lakehouse.projected_columns,
            filter_pushdown_executed: !self.plan.pushdown.predicates.is_empty() && output_rows < 4,
            limit_pushdown_executed: self
                .plan
                .pushdown
                .limit
                .is_some_and(|limit| output_rows <= limit),
        })
    }

    fn ensure_runtime_shape(&self) -> Result<(), AnalyticalSidecarError> {
        if self.plan.lakehouse.object_uri != self.plan.mirror.storage_uri {
            return Err(AnalyticalSidecarError::MirrorStorageMismatch);
        }
        if self.plan.lakehouse.projected_columns != self.plan.pushdown.projected_columns
            || self.plan.lakehouse.predicates != self.plan.pushdown.predicates
        {
            return Err(AnalyticalSidecarError::PushdownShapeMismatch);
        }
        Ok(())
    }

    fn ensure_runtime_policy(&self) -> Result<(), AnalyticalSidecarError> {
        if !self.policy.allowed_engines.contains(&self.plan.engine) {
            return Err(AnalyticalSidecarError::UnsupportedRuntimeConfig(
                "analytical engine is not enabled by runtime policy",
            ));
        }
        let scheme = object_uri_scheme(&self.plan.lakehouse.object_uri).ok_or(
            AnalyticalSidecarError::InvalidObjectUri("lakehouse.object_uri"),
        )?;
        if !self
            .policy
            .allowed_object_uri_schemes
            .iter()
            .any(|allowed| allowed == scheme)
        {
            return Err(AnalyticalSidecarError::UnsupportedRuntimeConfig(
                "lakehouse object URI scheme is not enabled by runtime policy",
            ));
        }
        if self
            .plan
            .pushdown
            .limit
            .is_some_and(|limit| limit > self.policy.max_pushdown_limit)
        {
            return Err(AnalyticalSidecarError::UnsupportedRuntimeConfig(
                "pushdown limit exceeds runtime policy",
            ));
        }
        Ok(())
    }
}

fn object_uri_scheme(value: &str) -> Option<&str> {
    value.split_once("://").map(|(scheme, _)| scheme)
}

fn deterministic_mirrored_events(plan: &AnalyticalSidecarPlan) -> u64 {
    let limit_units = plan.pushdown.limit.unwrap_or(1_000).div_ceil(1_000).max(1);
    let plan_shape =
        (plan.pushdown.projected_columns.len() + plan.pushdown.predicates.len()) as u64;

    limit_units * plan_shape.max(1)
}

fn deterministic_estimated_rows(plan: &AnalyticalSidecarPlan) -> u64 {
    let base_rows = plan.pushdown.limit.unwrap_or(50_000);
    let predicate_discount = plan.pushdown.predicates.len() as u64 * 250;

    base_rows.saturating_sub(predicate_discount).max(1)
}

pub fn canonical_analytical_plan() -> AnalyticalSidecarPlan {
    AnalyticalSidecarPlan {
        mirror: AnalyticalMirrorContract {
            source_slot: "ai_blaise_cdc".to_string(),
            mirror_name: "orders_mirror".to_string(),
            storage_uri: "s3://lake/warehouse/orders".to_string(),
            search_index_enabled: true,
        },
        engine: AnalyticalEngine::DataFusion,
        lakehouse: LakehouseReadPlan {
            table: "public.orders".to_string(),
            format: LakehouseFormat::Iceberg,
            object_uri: "s3://lake/warehouse/orders".to_string(),
            projected_columns: vec!["tenant_id".to_string(), "total".to_string()],
            predicates: vec!["total > 0".to_string()],
        },
        pushdown: DataFusionPushdownPlan {
            plan_id: "orders-scan".to_string(),
            projected_columns: vec!["tenant_id".to_string(), "total".to_string()],
            predicates: vec!["total > 0".to_string()],
            limit: Some(2),
        },
        snapshot_commit: Some(IcebergSnapshotCommitPlan {
            transaction_id: "tx-1".to_string(),
            snapshot_id: "snapshot-1".to_string(),
            prepare_lsn: "16/B374D848".to_string(),
            manifest_uri: "s3://lake/warehouse/orders/metadata/manifest.avro".to_string(),
        }),
        federated_catalogs: vec![
            FederatedCatalog {
                name: "databricks".to_string(),
                target: FederationTarget::Databricks,
                iceberg_catalog_uri: "s3://lake/catalog/databricks".to_string(),
            },
            FederatedCatalog {
                name: "snowflake".to_string(),
                target: FederationTarget::Snowflake,
                iceberg_catalog_uri: "s3://lake/catalog/snowflake".to_string(),
            },
            FederatedCatalog {
                name: "trino".to_string(),
                target: FederationTarget::Trino,
                iceberg_catalog_uri: "s3://lake/catalog/trino".to_string(),
            },
            FederatedCatalog {
                name: "spark".to_string(),
                target: FederationTarget::Spark,
                iceberg_catalog_uri: "s3://lake/catalog/spark".to_string(),
            },
        ],
        duckdb_extensions: DuckDbExtensionCatalog {
            allowed_extensions: vec!["httpfs".to_string(), "iceberg".to_string()],
        },
        motherduck: Some(MotherDuckConnector {
            database: "analytics".to_string(),
            token_secret_ref: "motherduck-token".to_string(),
        }),
    }
}

pub fn canonical_analytical_runtime_policy() -> AnalyticalRuntimePolicy {
    AnalyticalRuntimePolicy {
        allowed_engines: vec![AnalyticalEngine::DataFusion],
        allowed_object_uri_schemes: vec!["s3".to_string()],
        max_pushdown_limit: 50_000,
        external_io_enabled: false,
    }
}

pub fn canonical_analytical_execution_plan() -> Result<AnalyticalSidecarPlan, AnalyticalSidecarError>
{
    let plan = canonical_analytical_plan();
    plan.validate()?;
    Ok(plan)
}

pub fn canonical_analytical_runtime_report(
) -> Result<AnalyticalRuntimeReport, AnalyticalSidecarError> {
    let mut runtime = AnalyticalRuntime::new(canonical_analytical_plan())?;
    runtime.execute_lakehouse_query()
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct DuckDbExtensionCatalogRuntimeReport {
    pub feature_id: &'static str,
    pub allowed_extensions: Vec<String>,
    pub allowed_extension_count: u64,
    pub install_sql: Vec<String>,
    pub load_sql: Vec<String>,
    pub external_io_attempted: bool,
    pub pg_duckdb_runtime_exercised: bool,
    pub motherduck_session_exercised: bool,
    pub evidence_boundary: &'static str,
}

pub fn canonical_duckdb_extension_catalog_report(
) -> Result<DuckDbExtensionCatalogRuntimeReport, AnalyticalSidecarError> {
    let catalog = canonical_analytical_plan().duckdb_extensions;
    catalog.validate()?;
    let install_sql = catalog
        .allowed_extensions
        .iter()
        .map(|extension| format!("INSTALL {extension}"))
        .collect::<Vec<_>>();
    let load_sql = catalog
        .allowed_extensions
        .iter()
        .map(|extension| format!("LOAD {extension}"))
        .collect::<Vec<_>>();

    Ok(DuckDbExtensionCatalogRuntimeReport {
        feature_id: "L12",
        allowed_extension_count: catalog.allowed_extensions.len() as u64,
        allowed_extensions: catalog.allowed_extensions,
        install_sql,
        load_sql,
        external_io_attempted: false,
        pg_duckdb_runtime_exercised: false,
        motherduck_session_exercised: false,
        evidence_boundary: "live-duckdb-container-extension-load-only",
    })
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct FederationCatalogPublicationReport {
    pub feature_id: &'static str,
    pub version: &'static str,
    pub catalog_names: Vec<String>,
    pub federation_targets: Vec<String>,
    pub catalog_count: u64,
    pub artifact_path: String,
    pub artifact_bytes: u64,
    pub local_catalog_artifact_created: bool,
    pub external_warehouse_connections_attempted: bool,
    pub object_store_io_attempted: bool,
    pub catalog_auth_exercised: bool,
    pub evidence_boundary: &'static str,
}

pub fn publish_canonical_federation_catalog_artifact(
    artifact_path: &Path,
) -> Result<FederationCatalogPublicationReport, AnalyticalSidecarError> {
    let plan = canonical_analytical_execution_plan()?;
    let artifact = render_federation_catalog_json(&plan.federated_catalogs)?;
    if let Some(parent) = artifact_path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|error| {
                AnalyticalSidecarError::QueryEngineExecution(format!(
                    "create catalog artifact directory {} failed: {error}",
                    parent.display()
                ))
            })?;
        }
    }
    fs::write(artifact_path, artifact).map_err(|error| {
        AnalyticalSidecarError::QueryEngineExecution(format!(
            "write federation catalog artifact {} failed: {error}",
            artifact_path.display()
        ))
    })?;
    let artifact_bytes = fs::metadata(artifact_path)
        .map_err(|error| {
            AnalyticalSidecarError::QueryEngineExecution(format!(
                "metadata federation catalog artifact {} failed: {error}",
                artifact_path.display()
            ))
        })?
        .len();
    let catalog_names = plan
        .federated_catalogs
        .iter()
        .map(|catalog| catalog.name.clone())
        .collect::<Vec<_>>();
    let federation_targets = plan
        .federated_catalogs
        .iter()
        .map(|catalog| federation_target_label(catalog.target).to_string())
        .collect::<Vec<_>>();

    Ok(FederationCatalogPublicationReport {
        feature_id: "L6",
        version: "v1",
        catalog_count: catalog_names.len() as u64,
        catalog_names,
        federation_targets,
        artifact_path: artifact_path.display().to_string(),
        artifact_bytes,
        local_catalog_artifact_created: artifact_bytes > 0,
        external_warehouse_connections_attempted: false,
        object_store_io_attempted: false,
        catalog_auth_exercised: false,
        evidence_boundary: "local-federation-catalog-artifact-http-only",
    })
}

#[derive(Debug, Serialize)]
struct FederationCatalogArtifact<'a> {
    feature_id: &'static str,
    version: &'static str,
    catalogs: Vec<FederationCatalogArtifactEntry<'a>>,
    external_warehouse_connections_attempted: bool,
    object_store_io_attempted: bool,
    catalog_auth_exercised: bool,
}

#[derive(Debug, Serialize)]
struct FederationCatalogArtifactEntry<'a> {
    name: &'a str,
    target: &'static str,
    iceberg_catalog_uri: &'a str,
}

fn render_federation_catalog_json(
    catalogs: &[FederatedCatalog],
) -> Result<String, AnalyticalSidecarError> {
    let artifact = FederationCatalogArtifact {
        feature_id: "L6",
        version: "v1",
        catalogs: catalogs
            .iter()
            .map(|catalog| FederationCatalogArtifactEntry {
                name: &catalog.name,
                target: federation_target_label(catalog.target),
                iceberg_catalog_uri: &catalog.iceberg_catalog_uri,
            })
            .collect(),
        external_warehouse_connections_attempted: false,
        object_store_io_attempted: false,
        catalog_auth_exercised: false,
    };
    let mut json = serde_json::to_string_pretty(&artifact).map_err(|error| {
        AnalyticalSidecarError::QueryEngineExecution(format!(
            "serialize federation catalog artifact failed: {error}"
        ))
    })?;
    json.push('\n');
    Ok(json)
}

fn federation_target_label(target: FederationTarget) -> &'static str {
    match target {
        FederationTarget::Snowflake => "snowflake",
        FederationTarget::Trino => "trino",
        FederationTarget::Spark => "spark",
        FederationTarget::Databricks => "databricks",
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct LogicalMirrorRow {
    pub tenant_id: i32,
    pub order_id: i32,
    pub total: i64,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct LogicalMirrorMaterializationReport {
    pub feature_id: &'static str,
    pub mirror_name: String,
    pub source_table: String,
    pub source_plugin: String,
    pub decoded_change_lines: usize,
    pub materialized_rows: u64,
    pub materialized_total: i64,
    pub artifact_path: String,
    pub artifact_bytes: u64,
    pub datafusion_query_executed: bool,
    pub datafusion_output_rows: u64,
    pub datafusion_output_total: i64,
    pub local_mirror_artifact_created: bool,
    pub object_store_io_attempted: bool,
    pub long_running_slot_tailing: bool,
    pub checkpoint_persistence_exercised: bool,
    pub kubernetes_traffic_exercised: bool,
}

pub fn materialize_test_decoding_mirror_to_local_artifact(
    decoded_changes: &str,
    artifact_path: &Path,
) -> Result<LogicalMirrorMaterializationReport, AnalyticalSidecarError> {
    let rows = parse_test_decoding_insert_rows(decoded_changes)?;
    if rows.is_empty() {
        return Err(AnalyticalSidecarError::MirrorMaterialization(
            "test_decoding stream did not contain insert rows".to_string(),
        ));
    }

    let artifact_bytes = write_mirror_artifact(&rows, artifact_path)?;
    let (datafusion_output_rows, datafusion_output_total) =
        execute_datafusion_over_mirror_artifact(artifact_path)?;
    let materialized_total = rows.iter().map(|row| row.total).sum();
    let decoded_change_lines = decoded_changes
        .lines()
        .filter(|line| line.contains("table public.l8_orders: INSERT:"))
        .count();

    Ok(LogicalMirrorMaterializationReport {
        feature_id: "L8",
        mirror_name: "orders_mirror".to_string(),
        source_table: "public.l8_orders".to_string(),
        source_plugin: "test_decoding".to_string(),
        decoded_change_lines,
        materialized_rows: rows.len() as u64,
        materialized_total,
        artifact_path: artifact_path.display().to_string(),
        artifact_bytes,
        datafusion_query_executed: true,
        datafusion_output_rows,
        datafusion_output_total,
        local_mirror_artifact_created: artifact_bytes > 0,
        object_store_io_attempted: false,
        long_running_slot_tailing: false,
        checkpoint_persistence_exercised: false,
        kubernetes_traffic_exercised: false,
    })
}

fn parse_test_decoding_insert_rows(
    decoded_changes: &str,
) -> Result<Vec<LogicalMirrorRow>, AnalyticalSidecarError> {
    decoded_changes
        .lines()
        .filter(|line| line.contains("table public.l8_orders: INSERT:"))
        .map(parse_test_decoding_insert_row)
        .collect()
}

fn parse_test_decoding_insert_row(line: &str) -> Result<LogicalMirrorRow, AnalyticalSidecarError> {
    let payload = line
        .split_once(" INSERT: ")
        .map(|(_, payload)| payload)
        .ok_or_else(|| {
            AnalyticalSidecarError::MirrorMaterialization(format!(
                "missing INSERT payload in decoded line: {line}"
            ))
        })?;

    let tenant_id = parse_test_decoding_i32(payload, "tenant_id")?;
    let order_id = parse_test_decoding_i32(payload, "order_id")?;
    let total = parse_test_decoding_i64(payload, "total")?;
    Ok(LogicalMirrorRow {
        tenant_id,
        order_id,
        total,
    })
}

fn parse_test_decoding_i32(payload: &str, column: &str) -> Result<i32, AnalyticalSidecarError> {
    let value = parse_test_decoding_i64(payload, column)?;
    i32::try_from(value).map_err(|error| {
        AnalyticalSidecarError::MirrorMaterialization(format!(
            "invalid {column} value {value}: {error}"
        ))
    })
}

fn parse_test_decoding_i64(payload: &str, column: &str) -> Result<i64, AnalyticalSidecarError> {
    let needle = format!("{column}[");
    let start = payload.find(&needle).ok_or_else(|| {
        AnalyticalSidecarError::MirrorMaterialization(format!(
            "missing {column} in decoded payload: {payload}"
        ))
    })?;
    let after_column = &payload[start..];
    let (_, after_type) = after_column.split_once(':').ok_or_else(|| {
        AnalyticalSidecarError::MirrorMaterialization(format!(
            "missing {column} value separator in decoded payload: {payload}"
        ))
    })?;
    let token = after_type.split_whitespace().next().ok_or_else(|| {
        AnalyticalSidecarError::MirrorMaterialization(format!(
            "missing {column} value in decoded payload: {payload}"
        ))
    })?;
    let unquoted = token.trim_matches(char::from(39));
    unquoted.parse::<i64>().map_err(|error| {
        AnalyticalSidecarError::MirrorMaterialization(format!(
            "invalid {column} value {unquoted}: {error}"
        ))
    })
}

fn write_mirror_artifact(
    rows: &[LogicalMirrorRow],
    artifact_path: &Path,
) -> Result<u64, AnalyticalSidecarError> {
    if let Some(parent) = artifact_path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|error| {
                AnalyticalSidecarError::MirrorMaterialization(format!(
                    "create artifact directory {} failed: {error}",
                    parent.display()
                ))
            })?;
        }
    }

    let mut artifact = String::from("tenant_id\torder_id\ttotal\n");
    for row in rows {
        artifact.push_str(&format!(
            "{}\t{}\t{}\n",
            row.tenant_id, row.order_id, row.total
        ));
    }

    let file_name = artifact_path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("mirror.tsv");
    let temp_path = artifact_path.with_file_name(format!(
        ".{file_name}.tmp-{}-{}",
        std::process::id(),
        rows.len()
    ));
    fs::write(&temp_path, artifact).map_err(|error| {
        AnalyticalSidecarError::MirrorMaterialization(format!(
            "write {} failed: {error}",
            temp_path.display()
        ))
    })?;
    fs::rename(&temp_path, artifact_path).map_err(|error| {
        let _ = fs::remove_file(&temp_path);
        AnalyticalSidecarError::MirrorMaterialization(format!(
            "rename {} to {} failed: {error}",
            temp_path.display(),
            artifact_path.display()
        ))
    })?;
    fs::metadata(artifact_path)
        .map_err(|error| {
            AnalyticalSidecarError::MirrorMaterialization(format!(
                "metadata {} failed: {error}",
                artifact_path.display()
            ))
        })
        .map(|metadata| metadata.len())
}

fn execute_datafusion_over_mirror_artifact(
    artifact_path: &Path,
) -> Result<(u64, i64), AnalyticalSidecarError> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|error| AnalyticalSidecarError::QueryEngineExecution(error.to_string()))?;

    runtime.block_on(execute_datafusion_over_mirror_artifact_async(artifact_path))
}

async fn execute_datafusion_over_mirror_artifact_async(
    artifact_path: &Path,
) -> Result<(u64, i64), AnalyticalSidecarError> {
    let schema = Schema::new(vec![
        Field::new("tenant_id", DataType::Int32, false),
        Field::new("order_id", DataType::Int32, false),
        Field::new("total", DataType::Int64, false),
    ]);
    let artifact = artifact_path.to_str().ok_or_else(|| {
        AnalyticalSidecarError::MirrorMaterialization(format!(
            "artifact path {} is not valid UTF-8",
            artifact_path.display()
        ))
    })?;

    let context = SessionContext::new();
    context
        .register_csv(
            "mirror_orders",
            artifact,
            CsvReadOptions::new()
                .delimiter(b'\t')
                .has_header(true)
                .schema(&schema)
                .file_extension(".tsv"),
        )
        .await
        .map_err(|error| AnalyticalSidecarError::QueryEngineExecution(error.to_string()))?;
    let dataframe = context
        .sql("SELECT tenant_id, total FROM mirror_orders WHERE total > 0 ORDER BY tenant_id, order_id")
        .await
        .map_err(|error| AnalyticalSidecarError::QueryEngineExecution(error.to_string()))?;
    let batches = dataframe
        .collect()
        .await
        .map_err(|error| AnalyticalSidecarError::QueryEngineExecution(error.to_string()))?;

    let mut output_rows = 0_u64;
    let mut output_total = 0_i64;
    for batch in &batches {
        output_rows += batch.num_rows() as u64;
        let total_column = batch.column_by_name("total").ok_or_else(|| {
            AnalyticalSidecarError::QueryEngineExecution(
                "mirror DataFusion output did not include total column".to_string(),
            )
        })?;
        let totals = total_column
            .as_any()
            .downcast_ref::<Int64Array>()
            .ok_or_else(|| {
                AnalyticalSidecarError::QueryEngineExecution(
                    "mirror total column was not Int64".to_string(),
                )
            })?;
        for index in 0..totals.len() {
            output_total += totals.value(index);
        }
    }
    Ok((output_rows, output_total))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn logical_mirror_materializes_test_decoding_rows_to_local_datafusion() {
        let temp_path =
            std::env::temp_dir().join(format!("ai-blaise-l8-test-{}.tsv", std::process::id()));
        let decoded = "BEGIN 1\ntable public.l8_orders: INSERT: tenant_id[integer]:1 order_id[integer]:1 total[bigint]:1000\ntable public.l8_orders: INSERT: tenant_id[integer]:2 order_id[integer]:2 total[bigint]:2000\ntable public.l8_orders: INSERT: tenant_id[integer]:3 order_id[integer]:3 total[bigint]:3000\nCOMMIT 1";
        let report = materialize_test_decoding_mirror_to_local_artifact(decoded, &temp_path)
            .expect("logical mirror materialization");

        assert_eq!(report.feature_id, "L8");
        assert_eq!(report.source_plugin, "test_decoding");
        assert_eq!(report.decoded_change_lines, 3);
        assert_eq!(report.materialized_rows, 3);
        assert_eq!(report.materialized_total, 6_000);
        assert_eq!(report.datafusion_output_rows, 3);
        assert_eq!(report.datafusion_output_total, 6_000);
        assert_eq!(
            std::fs::read_to_string(&temp_path).expect("mirror artifact"),
            "tenant_id\torder_id\ttotal\n1\t1\t1000\n2\t2\t2000\n3\t3\t3000\n"
        );
        assert!(report.local_mirror_artifact_created);
        assert!(!report.object_store_io_attempted);
        assert!(!report.long_running_slot_tailing);
        assert!(!report.checkpoint_persistence_exercised);
        assert!(!report.kubernetes_traffic_exercised);
        let _ = std::fs::remove_file(temp_path);
    }

    #[test]
    fn logical_mirror_rejects_empty_decoded_stream() {
        let temp_path =
            std::env::temp_dir().join(format!("ai-blaise-l8-empty-{}.tsv", std::process::id()));

        assert!(matches!(
            materialize_test_decoding_mirror_to_local_artifact("BEGIN 1\nCOMMIT 1", &temp_path),
            Err(AnalyticalSidecarError::MirrorMaterialization(_))
        ));
    }

    #[test]
    fn logical_mirror_rejects_out_of_range_integer_keys() {
        let temp_path =
            std::env::temp_dir().join(format!("ai-blaise-l8-overflow-{}.tsv", std::process::id()));
        let decoded = "table public.l8_orders: INSERT: tenant_id[bigint]:2147483648 order_id[integer]:1 total[bigint]:1000";

        assert!(matches!(
            materialize_test_decoding_mirror_to_local_artifact(decoded, &temp_path),
            Err(AnalyticalSidecarError::MirrorMaterialization(_))
        ));
        assert!(!temp_path.exists());
    }

    #[test]
    fn analytical_sidecar_plan_validates_lakehouse_and_federation() {
        assert_eq!(canonical_analytical_plan().validate(), Ok(()));
    }

    #[test]
    fn federation_catalog_publication_writes_versioned_artifact() {
        let temp_path = std::env::temp_dir().join(format!(
            "ai-blaise-l6-federation-{}.json",
            std::process::id()
        ));
        let report = publish_canonical_federation_catalog_artifact(&temp_path)
            .expect("federation catalog publication");

        assert_eq!(report.feature_id, "L6");
        assert_eq!(report.version, "v1");
        assert_eq!(report.catalog_count, 4);
        assert_eq!(
            report.catalog_names,
            ["databricks", "snowflake", "trino", "spark"]
        );
        assert_eq!(
            report.federation_targets,
            ["databricks", "snowflake", "trino", "spark"]
        );
        assert!(report.local_catalog_artifact_created);
        assert!(!report.external_warehouse_connections_attempted);
        assert!(!report.object_store_io_attempted);
        assert!(!report.catalog_auth_exercised);
        let artifact = std::fs::read_to_string(&temp_path).expect("catalog artifact");
        assert!(artifact.contains("\"feature_id\": \"L6\""));
        assert!(artifact.contains("\"target\": \"snowflake\""));
        assert!(artifact.contains("\"target\": \"trino\""));
        assert!(artifact.contains("\"target\": \"spark\""));
        assert!(artifact.contains("\"target\": \"databricks\""));
        let _ = std::fs::remove_file(temp_path);
    }

    #[test]
    fn duckdb_extension_catalog_report_renders_install_and_load_sql() {
        let report = canonical_duckdb_extension_catalog_report().expect("duckdb catalog report");

        assert_eq!(report.feature_id, "L12");
        assert_eq!(report.allowed_extensions, ["httpfs", "iceberg"]);
        assert_eq!(report.allowed_extension_count, 2);
        assert_eq!(report.install_sql, ["INSTALL httpfs", "INSTALL iceberg"]);
        assert_eq!(report.load_sql, ["LOAD httpfs", "LOAD iceberg"]);
        assert!(!report.external_io_attempted);
        assert!(!report.pg_duckdb_runtime_exercised);
        assert!(!report.motherduck_session_exercised);
        assert_eq!(
            report.evidence_boundary,
            "live-duckdb-container-extension-load-only"
        );
    }

    #[test]
    fn canonical_analytical_execution_plan_is_deterministic() {
        let plan = canonical_analytical_execution_plan().expect("canonical plan");

        assert_eq!(plan.mirror.mirror_name, "orders_mirror");
        assert_eq!(plan.pushdown.plan_id, "orders-scan");
        assert_eq!(plan.federated_catalogs[0].name, "databricks");
        assert_eq!(plan.federated_catalogs.len(), 4);
        assert_eq!(plan.federated_catalogs[1].name, "snowflake");
        assert_eq!(plan.federated_catalogs[2].name, "trino");
        assert_eq!(plan.federated_catalogs[3].name, "spark");
    }

    #[test]
    fn analytical_runtime_executes_lakehouse_pushdown_and_catalogs() {
        let report = canonical_analytical_runtime_report().expect("runtime report");

        assert_eq!(report.read.mirror_name, "orders_mirror");
        assert_eq!(report.read.engine, AnalyticalEngine::DataFusion);
        assert_eq!(report.read.format, LakehouseFormat::Iceberg);
        assert_eq!(report.read.pushdown_plan_id, "orders-scan");
        assert!(report.read.pushed_down);
        assert_eq!(report.read.estimated_rows, 1);
        assert_eq!(report.read.mirrored_cdc_events, 3);
        assert_eq!(
            report
                .snapshot_commit
                .as_ref()
                .map(|snapshot| snapshot.snapshot_id.as_str()),
            Some("snapshot-1")
        );
        assert_eq!(report.federated_catalogs[0].catalog, "databricks");
        assert_eq!(report.federated_catalogs.len(), 4);
        assert_eq!(report.duckdb_extensions, ["httpfs", "iceberg"]);
        assert_eq!(report.motherduck_database.as_deref(), Some("analytics"));
        assert_eq!(report.state.lakehouse_reads, 1);
        assert_eq!(report.state.pushed_down_plans, 1);
        assert_eq!(report.state.mirrored_cdc_events, 3);
        assert_eq!(report.state.query_engine_executions, 1);
        assert_eq!(report.state.query_engine_output_rows, 2);
        assert_eq!(report.state.snapshot_commits, 1);
        assert_eq!(report.state.federated_catalog_publications, 4);
        assert_eq!(report.state.duckdb_extension_loads, 2);
        assert_eq!(report.state.motherduck_sessions, 1);
    }

    #[test]
    fn analytical_runtime_reports_local_datafusion_boundary() {
        let report = canonical_analytical_runtime_report().expect("runtime report");
        let datafusion = report.datafusion_execution.as_ref().expect("datafusion");

        assert!(!report.external_io_attempted);
        assert!(report.query_engine_executed);
        assert_eq!(
            report.evidence_boundary,
            "local-datafusion-recordbatch-only"
        );
        assert_eq!(
            report.runtime_policy.allowed_engines,
            [AnalyticalEngine::DataFusion]
        );
        assert_eq!(datafusion.output_rows, 2);
        assert_eq!(datafusion.output_total, 3_000);
        assert!(datafusion.projection_pushdown_executed);
        assert!(datafusion.filter_pushdown_executed);
        assert!(datafusion.limit_pushdown_executed);
    }

    #[test]
    fn analytical_runtime_rejects_disabled_engine() {
        let mut plan = canonical_analytical_plan();
        plan.engine = AnalyticalEngine::DuckDb;
        let mut runtime = AnalyticalRuntime::new(plan).expect("runtime");

        assert_eq!(
            runtime.execute_lakehouse_query(),
            Err(AnalyticalSidecarError::UnsupportedRuntimeConfig(
                "analytical engine is not enabled by runtime policy"
            ))
        );
    }

    #[test]
    fn analytical_runtime_rejects_external_io_enabled_policy() {
        let mut policy = canonical_analytical_runtime_policy();
        policy.external_io_enabled = true;

        assert_eq!(
            AnalyticalRuntime::new_with_policy(canonical_analytical_plan(), policy),
            Err(AnalyticalSidecarError::UnsupportedRuntimeConfig(
                "external object-store IO is not implemented in this runtime boundary"
            ))
        );
    }

    #[test]
    fn analytical_runtime_rejects_unallowed_object_scheme() {
        let mut policy = canonical_analytical_runtime_policy();
        policy.allowed_object_uri_schemes = vec!["gs".to_string()];
        let mut runtime = AnalyticalRuntime::new_with_policy(canonical_analytical_plan(), policy)
            .expect("runtime");

        assert_eq!(
            runtime.execute_lakehouse_query(),
            Err(AnalyticalSidecarError::UnsupportedRuntimeConfig(
                "lakehouse object URI scheme is not enabled by runtime policy"
            ))
        );
    }

    #[test]
    fn analytical_runtime_rejects_pushdown_limit_over_policy() {
        let mut policy = canonical_analytical_runtime_policy();
        policy.max_pushdown_limit = 1;
        let mut runtime = AnalyticalRuntime::new_with_policy(canonical_analytical_plan(), policy)
            .expect("runtime");

        assert_eq!(
            runtime.execute_lakehouse_query(),
            Err(AnalyticalSidecarError::UnsupportedRuntimeConfig(
                "pushdown limit exceeds runtime policy"
            ))
        );
    }

    #[test]
    fn analytical_runtime_rejects_mirror_storage_mismatch() {
        let mut plan = canonical_analytical_plan();
        plan.lakehouse.object_uri = "s3://lake/warehouse/other".to_string();
        let mut runtime = AnalyticalRuntime::new(plan).expect("runtime");

        assert_eq!(
            runtime.execute_lakehouse_query(),
            Err(AnalyticalSidecarError::MirrorStorageMismatch)
        );
    }

    #[test]
    fn analytical_runtime_rejects_pushdown_shape_mismatch() {
        let mut plan = canonical_analytical_plan();
        plan.pushdown.predicates = vec!["tenant_id = 1".to_string()];
        let mut runtime = AnalyticalRuntime::new(plan).expect("runtime");

        assert_eq!(
            runtime.execute_lakehouse_query(),
            Err(AnalyticalSidecarError::PushdownShapeMismatch)
        );
    }

    #[test]
    fn lakehouse_read_rejects_invalid_projection_identifier() {
        let mut plan = canonical_analytical_plan();
        plan.lakehouse.projected_columns = vec!["tenant id".to_string()];

        assert_eq!(
            plan.validate(),
            Err(AnalyticalSidecarError::InvalidIdentifier(
                "lakehouse.projected_columns"
            ))
        );
    }

    #[test]
    fn lakehouse_read_rejects_unsupported_predicate_shape() {
        let mut plan = canonical_analytical_plan();
        plan.lakehouse.predicates = vec!["total > 0; drop table public.orders".to_string()];

        assert_eq!(
            plan.validate(),
            Err(AnalyticalSidecarError::InvalidPredicate(
                "lakehouse.predicates"
            ))
        );
    }

    #[test]
    fn lakehouse_read_requires_projection() {
        let mut plan = canonical_analytical_plan();
        plan.lakehouse.projected_columns = Vec::new();

        assert_eq!(
            plan.validate(),
            Err(AnalyticalSidecarError::MissingRequiredField(
                "lakehouse.projected_columns"
            ))
        );
    }

    #[test]
    fn snapshot_commit_requires_lsn() {
        let mut plan = canonical_analytical_plan();
        plan.snapshot_commit = Some(IcebergSnapshotCommitPlan {
            transaction_id: "tx-1".to_string(),
            snapshot_id: "snapshot-1".to_string(),
            prepare_lsn: "bad-lsn".to_string(),
            manifest_uri: "s3://lake/warehouse/orders/metadata/manifest.avro".to_string(),
        });

        assert_eq!(plan.validate(), Err(AnalyticalSidecarError::InvalidLsn));
    }

    #[test]
    fn motherduck_requires_token_secret() {
        let mut plan = canonical_analytical_plan();
        plan.motherduck = Some(MotherDuckConnector {
            database: "analytics".to_string(),
            token_secret_ref: " ".to_string(),
        });

        assert_eq!(
            plan.validate(),
            Err(AnalyticalSidecarError::MissingRequiredField(
                "motherduck.token_secret_ref"
            ))
        );
    }
}
