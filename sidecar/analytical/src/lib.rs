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
        validate_required_list("lakehouse.projected_columns", &self.projected_columns)?;
        validate_optional_list("lakehouse.predicates", &self.predicates)
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
        validate_required_list("pushdown.projected_columns", &self.projected_columns)?;
        validate_optional_list("pushdown.predicates", &self.predicates)
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
        validate_required_list("duckdb.allowed_extensions", &self.allowed_extensions)
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
    MissingRequiredField(&'static str),
    SharedContract(String),
}

impl fmt::Display for AnalyticalSidecarError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidIdentifier(field) => write!(formatter, "{field} must be a SQL identifier"),
            Self::InvalidLsn => write!(formatter, "LSN must use the PostgreSQL HEX/HEX form"),
            Self::InvalidObjectUri(field) => write!(formatter, "{field} must be an object URI"),
            Self::MissingRequiredField(field) => write!(formatter, "{field} must not be empty"),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn analytical_sidecar_plan_validates_lakehouse_and_federation() {
        assert_eq!(valid_plan().validate(), Ok(()));
    }

    #[test]
    fn lakehouse_read_requires_projection() {
        let mut plan = valid_plan();
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
        let mut plan = valid_plan();
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
        let mut plan = valid_plan();
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

    fn valid_plan() -> AnalyticalSidecarPlan {
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
}
