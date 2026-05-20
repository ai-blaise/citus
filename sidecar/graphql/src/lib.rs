//! GraphQL sidecar contracts.

// FEATURE: API3
// FEATURE: API4
// FEATURE: API5

use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct GraphqlSidecarPlan {
    pub endpoint_path: String,
    pub schema_bindings: Vec<GraphqlSchemaBinding>,
    pub distributed_bindings: Vec<DistributedGraphqlBinding>,
    pub auth: GraphqlAuthPolicy,
}

impl GraphqlSidecarPlan {
    pub fn validate(&self) -> Result<(), GraphqlSidecarError> {
        validate_path("endpoint_path", &self.endpoint_path)?;
        if self.schema_bindings.is_empty() {
            return Err(GraphqlSidecarError::MissingRequiredField("schema_bindings"));
        }
        for binding in &self.schema_bindings {
            binding.validate()?;
        }
        for binding in &self.distributed_bindings {
            binding.validate()?;
        }
        self.auth.validate()
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct GraphqlSchemaBinding {
    pub pg_schema: String,
    pub graphql_namespace: String,
    pub exposed_tables: Vec<String>,
}

impl GraphqlSchemaBinding {
    fn validate(&self) -> Result<(), GraphqlSidecarError> {
        validate_identifier("schema.pg_schema", &self.pg_schema)?;
        validate_identifier("schema.graphql_namespace", &self.graphql_namespace)?;
        validate_required_list("schema.exposed_tables", &self.exposed_tables)
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct DistributedGraphqlBinding {
    pub type_name: String,
    pub table: String,
    pub distribution_column: String,
    pub route_function: String,
}

impl DistributedGraphqlBinding {
    fn validate(&self) -> Result<(), GraphqlSidecarError> {
        validate_identifier("distributed.type_name", &self.type_name)?;
        validate_qualified_name("distributed.table", &self.table)?;
        validate_identifier("distributed.distribution_column", &self.distribution_column)?;
        validate_qualified_name("distributed.route_function", &self.route_function)
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct GraphqlAuthPolicy {
    pub rls_required: bool,
    pub jwt_secret_ref: String,
    pub tenant_claim: String,
    pub introspection_enabled: bool,
}

impl GraphqlAuthPolicy {
    fn validate(&self) -> Result<(), GraphqlSidecarError> {
        if !self.rls_required {
            return Err(GraphqlSidecarError::RlsRequired);
        }
        validate_required("auth.jwt_secret_ref", &self.jwt_secret_ref)?;
        validate_required("auth.tenant_claim", &self.tenant_claim)
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum GraphqlSidecarError {
    InvalidIdentifier(&'static str),
    InvalidPath(&'static str),
    MissingRequiredField(&'static str),
    RlsRequired,
}

impl fmt::Display for GraphqlSidecarError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidIdentifier(field) => write!(formatter, "{field} must be a SQL identifier"),
            Self::InvalidPath(field) => write!(formatter, "{field} must start with /"),
            Self::MissingRequiredField(field) => write!(formatter, "{field} must not be empty"),
            Self::RlsRequired => write!(formatter, "RLS must be required for GraphQL routes"),
        }
    }
}

impl Error for GraphqlSidecarError {}

fn validate_required(field: &'static str, value: &str) -> Result<(), GraphqlSidecarError> {
    if value.trim().is_empty() {
        return Err(GraphqlSidecarError::MissingRequiredField(field));
    }
    Ok(())
}

fn validate_required_list(
    field: &'static str,
    values: &[String],
) -> Result<(), GraphqlSidecarError> {
    if values.is_empty() || values.iter().any(|value| value.trim().is_empty()) {
        return Err(GraphqlSidecarError::MissingRequiredField(field));
    }
    Ok(())
}

fn validate_identifier(field: &'static str, value: &str) -> Result<(), GraphqlSidecarError> {
    validate_required(field, value)?;
    if value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
    {
        Ok(())
    } else {
        Err(GraphqlSidecarError::InvalidIdentifier(field))
    }
}

fn validate_qualified_name(field: &'static str, value: &str) -> Result<(), GraphqlSidecarError> {
    validate_required(field, value)?;
    let parts: Vec<_> = value.split('.').collect();
    if parts.len() == 2
        && parts
            .iter()
            .all(|part| validate_identifier(field, part).is_ok())
    {
        Ok(())
    } else {
        Err(GraphqlSidecarError::InvalidIdentifier(field))
    }
}

fn validate_path(field: &'static str, value: &str) -> Result<(), GraphqlSidecarError> {
    validate_required(field, value)?;
    if value.starts_with('/') {
        Ok(())
    } else {
        Err(GraphqlSidecarError::InvalidPath(field))
    }
}

pub fn canonical_graphql_plan() -> GraphqlSidecarPlan {
    GraphqlSidecarPlan {
        endpoint_path: "/graphql/v1".to_string(),
        schema_bindings: vec![GraphqlSchemaBinding {
            pg_schema: "public".to_string(),
            graphql_namespace: "public_api".to_string(),
            exposed_tables: vec!["orders".to_string(), "customers".to_string()],
        }],
        distributed_bindings: vec![DistributedGraphqlBinding {
            type_name: "Order".to_string(),
            table: "public.orders".to_string(),
            distribution_column: "tenant_id".to_string(),
            route_function: "companion.route_distributed_graphql".to_string(),
        }],
        auth: GraphqlAuthPolicy {
            rls_required: true,
            jwt_secret_ref: "graphql-jwt-secret".to_string(),
            tenant_claim: "tenant_id".to_string(),
            introspection_enabled: false,
        },
    }
}

pub fn canonical_graphql_execution_plan() -> Result<GraphqlSidecarPlan, GraphqlSidecarError> {
    let plan = canonical_graphql_plan();
    plan.validate()?;
    Ok(plan)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn graphql_plan_validates_distributed_binding() {
        assert_eq!(canonical_graphql_plan().validate(), Ok(()));
    }

    #[test]
    fn canonical_graphql_execution_plan_is_deterministic() {
        let plan = canonical_graphql_execution_plan().expect("canonical plan");

        assert_eq!(plan.endpoint_path, "/graphql/v1");
        assert_eq!(plan.schema_bindings[0].graphql_namespace, "public_api");
        assert_eq!(plan.distributed_bindings[0].type_name, "Order");
    }

    #[test]
    fn graphql_route_function_must_be_qualified() {
        let mut plan = canonical_graphql_plan();
        plan.distributed_bindings[0].route_function = "route_orders".to_string();

        assert_eq!(
            plan.validate(),
            Err(GraphqlSidecarError::InvalidIdentifier(
                "distributed.route_function"
            ))
        );
    }

    #[test]
    fn graphql_auth_requires_rls() {
        let mut plan = canonical_graphql_plan();
        plan.auth.rls_required = false;

        assert_eq!(plan.validate(), Err(GraphqlSidecarError::RlsRequired));
    }

    #[test]
    fn graphql_endpoint_path_must_be_absolute() {
        let mut plan = canonical_graphql_plan();
        plan.endpoint_path = "graphql/v1".to_string();

        assert_eq!(
            plan.validate(),
            Err(GraphqlSidecarError::InvalidPath("endpoint_path"))
        );
    }
}
