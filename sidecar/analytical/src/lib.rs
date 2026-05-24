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
use std::error::Error;
use std::fmt;

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
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct AnalyticalRuntimeReport {
    pub read: AnalyticalRuntimeRead,
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

        let pushed_down = true;
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
            snapshot_commit,
            federated_catalogs,
            duckdb_extensions,
            motherduck_database,
            state: self.state.clone(),
            runtime_policy: self.policy.clone(),
            external_io_attempted: false,
            query_engine_executed: false,
            evidence_boundary: "deterministic-runtime-report-only".to_string(),
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
            limit: Some(10_000),
        },
        snapshot_commit: Some(IcebergSnapshotCommitPlan {
            transaction_id: "tx-1".to_string(),
            snapshot_id: "snapshot-1".to_string(),
            prepare_lsn: "16/B374D848".to_string(),
            manifest_uri: "s3://lake/warehouse/orders/metadata/manifest.avro".to_string(),
        }),
        federated_catalogs: vec![FederatedCatalog {
            name: "databricks".to_string(),
            target: FederationTarget::Databricks,
            iceberg_catalog_uri: "s3://lake/catalog".to_string(),
        }],
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn analytical_sidecar_plan_validates_lakehouse_and_federation() {
        assert_eq!(canonical_analytical_plan().validate(), Ok(()));
    }

    #[test]
    fn canonical_analytical_execution_plan_is_deterministic() {
        let plan = canonical_analytical_execution_plan().expect("canonical plan");

        assert_eq!(plan.mirror.mirror_name, "orders_mirror");
        assert_eq!(plan.pushdown.plan_id, "orders-scan");
        assert_eq!(plan.federated_catalogs[0].name, "databricks");
    }

    #[test]
    fn analytical_runtime_executes_lakehouse_pushdown_and_catalogs() {
        let report = canonical_analytical_runtime_report().expect("runtime report");

        assert_eq!(report.read.mirror_name, "orders_mirror");
        assert_eq!(report.read.engine, AnalyticalEngine::DataFusion);
        assert_eq!(report.read.format, LakehouseFormat::Iceberg);
        assert_eq!(report.read.pushdown_plan_id, "orders-scan");
        assert!(report.read.pushed_down);
        assert_eq!(report.read.estimated_rows, 9_750);
        assert_eq!(report.read.mirrored_cdc_events, 30);
        assert_eq!(
            report
                .snapshot_commit
                .as_ref()
                .map(|snapshot| snapshot.snapshot_id.as_str()),
            Some("snapshot-1")
        );
        assert_eq!(report.federated_catalogs[0].catalog, "databricks");
        assert_eq!(report.duckdb_extensions, ["httpfs", "iceberg"]);
        assert_eq!(report.motherduck_database.as_deref(), Some("analytics"));
        assert_eq!(report.state.lakehouse_reads, 1);
        assert_eq!(report.state.pushed_down_plans, 1);
        assert_eq!(report.state.mirrored_cdc_events, 30);
        assert_eq!(report.state.snapshot_commits, 1);
        assert_eq!(report.state.federated_catalog_publications, 1);
        assert_eq!(report.state.duckdb_extension_loads, 2);
        assert_eq!(report.state.motherduck_sessions, 1);
    }

    #[test]
    fn analytical_runtime_reports_non_live_boundary() {
        let report = canonical_analytical_runtime_report().expect("runtime report");

        assert!(!report.external_io_attempted);
        assert!(!report.query_engine_executed);
        assert_eq!(
            report.evidence_boundary,
            "deterministic-runtime-report-only"
        );
        assert_eq!(
            report.runtime_policy.allowed_engines,
            [AnalyticalEngine::DataFusion]
        );
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
        policy.max_pushdown_limit = 1_000;
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
