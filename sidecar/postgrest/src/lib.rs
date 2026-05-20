//! PostgREST sidecar contracts.

// FEATURE: API1
// FEATURE: API2
// FEATURE: API5
// FEATURE: API6

use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PostgrestSidecarPlan {
    pub schemas: Vec<String>,
    pub routes: Vec<RestRoute>,
    pub auth: ApiAuthPolicy,
    pub openapi: OpenApiPlan,
}

impl PostgrestSidecarPlan {
    pub fn validate(&self) -> Result<(), PostgrestSidecarError> {
        validate_required_list("schemas", &self.schemas)?;
        if self.routes.is_empty() {
            return Err(PostgrestSidecarError::MissingRequiredField("routes"));
        }
        for route in &self.routes {
            route.validate()?;
        }
        self.auth.validate()?;
        self.openapi.validate()
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct RestRoute {
    pub schema: String,
    pub table: String,
    pub methods: Vec<RestMethod>,
    pub distributed_view: Option<DistributedViewBinding>,
}

impl RestRoute {
    fn validate(&self) -> Result<(), PostgrestSidecarError> {
        validate_identifier("route.schema", &self.schema)?;
        validate_identifier("route.table", &self.table)?;
        if self.methods.is_empty() {
            return Err(PostgrestSidecarError::MissingRequiredField("route.methods"));
        }
        if let Some(binding) = &self.distributed_view {
            binding.validate()?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum RestMethod {
    Get,
    Post,
    Patch,
    Delete,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct DistributedViewBinding {
    pub view_name: String,
    pub distribution_column: String,
    pub shard_count: u32,
}

impl DistributedViewBinding {
    fn validate(&self) -> Result<(), PostgrestSidecarError> {
        validate_qualified_name("distributed_view.view_name", &self.view_name)?;
        validate_identifier(
            "distributed_view.distribution_column",
            &self.distribution_column,
        )?;
        if self.shard_count == 0 {
            return Err(PostgrestSidecarError::InvalidShardCount);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ApiAuthPolicy {
    pub rls_required: bool,
    pub jwt_secret_ref: String,
    pub tenant_claim: String,
}

impl ApiAuthPolicy {
    fn validate(&self) -> Result<(), PostgrestSidecarError> {
        if !self.rls_required {
            return Err(PostgrestSidecarError::RlsRequired);
        }
        validate_required("auth.jwt_secret_ref", &self.jwt_secret_ref)?;
        validate_required("auth.tenant_claim", &self.tenant_claim)
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct OpenApiPlan {
    pub path: String,
    pub title: String,
    pub version: String,
}

impl OpenApiPlan {
    fn validate(&self) -> Result<(), PostgrestSidecarError> {
        validate_path("openapi.path", &self.path)?;
        validate_required("openapi.title", &self.title)?;
        validate_required("openapi.version", &self.version)
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum PostgrestSidecarError {
    InvalidIdentifier(&'static str),
    InvalidPath(&'static str),
    InvalidShardCount,
    MissingRequiredField(&'static str),
    RlsRequired,
}

impl fmt::Display for PostgrestSidecarError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidIdentifier(field) => write!(formatter, "{field} must be a SQL identifier"),
            Self::InvalidPath(field) => write!(formatter, "{field} must start with /"),
            Self::InvalidShardCount => write!(formatter, "shard_count must be greater than zero"),
            Self::MissingRequiredField(field) => write!(formatter, "{field} must not be empty"),
            Self::RlsRequired => write!(formatter, "RLS must be required for auto-API routes"),
        }
    }
}

impl Error for PostgrestSidecarError {}

fn validate_required(field: &'static str, value: &str) -> Result<(), PostgrestSidecarError> {
    if value.trim().is_empty() {
        return Err(PostgrestSidecarError::MissingRequiredField(field));
    }
    Ok(())
}

fn validate_required_list(
    field: &'static str,
    values: &[String],
) -> Result<(), PostgrestSidecarError> {
    if values.is_empty() || values.iter().any(|value| value.trim().is_empty()) {
        return Err(PostgrestSidecarError::MissingRequiredField(field));
    }
    Ok(())
}

fn validate_identifier(field: &'static str, value: &str) -> Result<(), PostgrestSidecarError> {
    validate_required(field, value)?;
    if value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
    {
        Ok(())
    } else {
        Err(PostgrestSidecarError::InvalidIdentifier(field))
    }
}

fn validate_qualified_name(field: &'static str, value: &str) -> Result<(), PostgrestSidecarError> {
    validate_required(field, value)?;
    let parts: Vec<_> = value.split('.').collect();
    if parts.len() == 2
        && parts
            .iter()
            .all(|part| validate_identifier(field, part).is_ok())
    {
        Ok(())
    } else {
        Err(PostgrestSidecarError::InvalidIdentifier(field))
    }
}

fn validate_path(field: &'static str, value: &str) -> Result<(), PostgrestSidecarError> {
    validate_required(field, value)?;
    if value.starts_with('/') {
        Ok(())
    } else {
        Err(PostgrestSidecarError::InvalidPath(field))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn postgrest_plan_validates_distributed_route_and_openapi() {
        assert_eq!(valid_plan().validate(), Ok(()));
    }

    #[test]
    fn distributed_view_requires_shard_count() {
        let mut plan = valid_plan();
        plan.routes[0]
            .distributed_view
            .as_mut()
            .expect("binding")
            .shard_count = 0;

        assert_eq!(
            plan.validate(),
            Err(PostgrestSidecarError::InvalidShardCount)
        );
    }

    #[test]
    fn auto_api_requires_rls() {
        let mut plan = valid_plan();
        plan.auth.rls_required = false;

        assert_eq!(plan.validate(), Err(PostgrestSidecarError::RlsRequired));
    }

    #[test]
    fn openapi_path_must_be_absolute() {
        let mut plan = valid_plan();
        plan.openapi.path = "openapi.json".to_string();

        assert_eq!(
            plan.validate(),
            Err(PostgrestSidecarError::InvalidPath("openapi.path"))
        );
    }

    fn valid_plan() -> PostgrestSidecarPlan {
        PostgrestSidecarPlan {
            schemas: vec!["public".to_string(), "api".to_string()],
            routes: vec![RestRoute {
                schema: "public".to_string(),
                table: "orders".to_string(),
                methods: vec![RestMethod::Get, RestMethod::Post],
                distributed_view: Some(DistributedViewBinding {
                    view_name: "api.orders".to_string(),
                    distribution_column: "tenant_id".to_string(),
                    shard_count: 32,
                }),
            }],
            auth: ApiAuthPolicy {
                rls_required: true,
                jwt_secret_ref: "postgrest-jwt-secret".to_string(),
                tenant_claim: "tenant_id".to_string(),
            },
            openapi: OpenApiPlan {
                path: "/openapi.json".to_string(),
                title: "ai-blaise Citus API".to_string(),
                version: "v1alpha1".to_string(),
            },
        }
    }
}
