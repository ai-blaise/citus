//! GraphQL sidecar contracts.

// FEATURE: API3
// FEATURE: API4
// FEATURE: API5

use ai_blaise_citus_auth_introspection_client::{
    validate_bearer_token, AuthIntrospectionClient, AuthIntrospectionConfig,
    AuthIntrospectionError, VerifiedIdentity, AUTH_CA_CERT_PATH_ENV, AUTH_CLIENT_IDENTITY_PATH_ENV,
    AUTH_EXPECTED_AUDIENCE_ENV, AUTH_EXPECTED_ISSUER_ENV, AUTH_INTROSPECTION_URL_ENV,
    AUTH_TIMEOUT_MS_ENV,
};
use ai_blaise_citus_sidecar_shared::{
    listen_addr_from_env, HttpProbeResponse, SidecarRuntime, SidecarRuntimeError,
};
use postgres::{Client, NoTls};
use serde::Deserialize;
use serde_json::Value;
use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;
use zeroize::Zeroizing;

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
    pub auth_service_ref: String,
    pub tenant_claim: String,
    pub introspection_enabled: bool,
}

impl GraphqlAuthPolicy {
    fn validate(&self) -> Result<(), GraphqlSidecarError> {
        if !self.rls_required {
            return Err(GraphqlSidecarError::RlsRequired);
        }
        validate_required("auth.auth_service_ref", &self.auth_service_ref)?;
        validate_required("auth.tenant_claim", &self.tenant_claim)
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum GraphqlSidecarError {
    InvalidIdentifier(&'static str),
    InvalidPath(&'static str),
    InvalidRuntimeDependency(String),
    AuthenticationRejected,
    AuthenticationUnavailable,
    BodyIdentityForbidden,
    DatabaseUnavailable(&'static str),
    IntrospectionDisabled,
    MalformedHttpRequest,
    MalformedQuery(String),
    MissingRequiredField(&'static str),
    MissingRuntimeDependency(String),
    PlanResolutionFailed(String),
    Runtime(String),
    TenantClaimMissing,
    LiveExecutionRequired,
    RlsRequired,
}

impl fmt::Display for GraphqlSidecarError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidIdentifier(field) => write!(formatter, "{field} must be a SQL identifier"),
            Self::InvalidPath(field) => write!(formatter, "{field} must start with /"),
            Self::InvalidRuntimeDependency(detail) => {
                write!(formatter, "invalid runtime dependency: {detail}")
            }
            Self::AuthenticationRejected => write!(formatter, "authentication rejected"),
            Self::AuthenticationUnavailable => {
                write!(formatter, "authentication service unavailable")
            }
            Self::BodyIdentityForbidden => {
                write!(formatter, "request-body identity claims are forbidden")
            }
            Self::DatabaseUnavailable(_) => write!(formatter, "GraphQL database unavailable"),
            Self::IntrospectionDisabled => {
                write!(formatter, "GraphQL introspection is disabled by policy")
            }
            Self::MalformedHttpRequest => {
                write!(formatter, "malformed GraphQL sidecar HTTP request")
            }
            Self::MalformedQuery(detail) => write!(formatter, "malformed GraphQL query: {detail}"),
            Self::MissingRequiredField(field) => write!(formatter, "{field} must not be empty"),
            Self::MissingRuntimeDependency(name) => {
                write!(formatter, "missing runtime dependency: {name}")
            }
            Self::PlanResolutionFailed(detail) => {
                write!(formatter, "plan resolution failed: {detail}")
            }
            Self::Runtime(error) => write!(formatter, "{error}"),
            Self::TenantClaimMissing => write!(
                formatter,
                "request.jwt.claims is missing the tenant claim required for RLS"
            ),
            Self::RlsRequired => write!(formatter, "RLS must be required for GraphQL routes"),
            Self::LiveExecutionRequired => {
                write!(
                    formatter,
                    "live GraphQL execution must be explicitly enabled"
                )
            }
        }
    }
}

impl Error for GraphqlSidecarError {}

impl From<SidecarRuntimeError> for GraphqlSidecarError {
    fn from(_error: SidecarRuntimeError) -> Self {
        Self::Runtime("GraphQL runtime request rejected".to_string())
    }
}

impl From<std::io::Error> for GraphqlSidecarError {
    fn from(_error: std::io::Error) -> Self {
        Self::Runtime("GraphQL I/O unavailable".to_string())
    }
}

impl From<postgres::Error> for GraphqlSidecarError {
    fn from(_error: postgres::Error) -> Self {
        Self::DatabaseUnavailable("database-operation-unavailable")
    }
}

impl From<AuthIntrospectionError> for GraphqlSidecarError {
    fn from(error: AuthIntrospectionError) -> Self {
        match error {
            AuthIntrospectionError::MissingConfiguration(name) => {
                Self::MissingRuntimeDependency(name.to_string())
            }
            AuthIntrospectionError::InvalidConfiguration(name)
            | AuthIntrospectionError::CredentialRead(name)
            | AuthIntrospectionError::CredentialTooLarge(name)
            | AuthIntrospectionError::CredentialInvalid(name) => {
                Self::InvalidRuntimeDependency(name.to_string())
            }
            AuthIntrospectionError::Inactive | AuthIntrospectionError::InvalidToken => {
                Self::AuthenticationRejected
            }
            AuthIntrospectionError::ClientBuild
            | AuthIntrospectionError::RequestFailed
            | AuthIntrospectionError::ResponseTooLarge
            | AuthIntrospectionError::UnexpectedStatus(_)
            | AuthIntrospectionError::InvalidResponse(_) => Self::AuthenticationUnavailable,
        }
    }
}

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
            auth_service_ref: "auth3-introspection".to_string(),
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

pub const GRAPHQL_DATABASE_URL_ENV: &str = "AI_BLAISE_GRAPHQL_DATABASE_URL";
pub const GRAPHQL_LIVE_EXECUTION_ENV: &str = "AI_BLAISE_GRAPHQL_LIVE_EXECUTION";

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct GraphqlRuntimeDependencyReport {
    pub database_url_env: String,
    pub auth_introspection_url_env: String,
    pub auth_ca_cert_path_env: String,
    pub auth_client_identity_path_env: String,
    pub auth_expected_issuer_env: String,
    pub auth_expected_audience_env: String,
    pub auth_timeout_ms_env: String,
    pub endpoint_path: String,
    pub pg_graphql_extension_required: bool,
}

pub fn graphql_runtime_dependency_report_from_env(
) -> Result<GraphqlRuntimeDependencyReport, GraphqlSidecarError> {
    let plan = canonical_graphql_execution_plan()?;
    graphql_runtime_dependency_report(&plan, |name| std::env::var(name).ok())
}

pub fn graphql_runtime_dependency_report<F>(
    plan: &GraphqlSidecarPlan,
    lookup: F,
) -> Result<GraphqlRuntimeDependencyReport, GraphqlSidecarError>
where
    F: Fn(&str) -> Option<String>,
{
    plan.validate()?;
    let database_url = require_runtime_env(&lookup, GRAPHQL_DATABASE_URL_ENV)?;
    validate_postgres_url(GRAPHQL_DATABASE_URL_ENV, &database_url)?;
    AuthIntrospectionConfig::from_lookup(lookup)?;

    Ok(GraphqlRuntimeDependencyReport {
        database_url_env: GRAPHQL_DATABASE_URL_ENV.to_string(),
        auth_introspection_url_env: AUTH_INTROSPECTION_URL_ENV.to_string(),
        auth_ca_cert_path_env: AUTH_CA_CERT_PATH_ENV.to_string(),
        auth_client_identity_path_env: AUTH_CLIENT_IDENTITY_PATH_ENV.to_string(),
        auth_expected_issuer_env: AUTH_EXPECTED_ISSUER_ENV.to_string(),
        auth_expected_audience_env: AUTH_EXPECTED_AUDIENCE_ENV.to_string(),
        auth_timeout_ms_env: AUTH_TIMEOUT_MS_ENV.to_string(),
        endpoint_path: plan.endpoint_path.clone(),
        pg_graphql_extension_required: true,
    })
}

pub struct GraphqlLiveExecutor {
    client: Client,
}

impl GraphqlLiveExecutor {
    pub fn connect_from_env() -> Result<Self, GraphqlSidecarError> {
        if !graphql_live_execution_enabled_from_env() {
            return Err(GraphqlSidecarError::LiveExecutionRequired);
        }
        let report = graphql_runtime_dependency_report_from_env()?;
        Self::connect_env(&report.database_url_env)
    }

    pub fn connect_env(database_url_env: &str) -> Result<Self, GraphqlSidecarError> {
        let database_url = std::env::var(database_url_env)
            .map_err(|_| GraphqlSidecarError::MissingRuntimeDependency(database_url_env.into()))?;
        Self::connect(&database_url)
    }

    pub fn connect(database_url: &str) -> Result<Self, GraphqlSidecarError> {
        validate_postgres_url(GRAPHQL_DATABASE_URL_ENV, database_url)?;
        let mut client = Client::connect(database_url, NoTls).map_err(|_| {
            GraphqlSidecarError::DatabaseUnavailable("database-connect-unavailable")
        })?;
        let extension_exists: bool = client
            .query_one(
                "select exists (select 1 from pg_extension where extname = 'pg_graphql')",
                &[],
            )?
            .try_get(0)
            .map_err(|_| {
                GraphqlSidecarError::DatabaseUnavailable("extension-attestation-result-unavailable")
            })?;
        if !extension_exists {
            return Err(GraphqlSidecarError::MissingRuntimeDependency(
                "pg_graphql extension".to_string(),
            ));
        }
        Ok(Self { client })
    }

    fn execute(
        &mut self,
        request: &GraphqlRequest,
        identity: &GraphqlIdentity,
    ) -> Result<String, GraphqlSidecarError> {
        let mut transaction = self.client.transaction().map_err(|_| {
            GraphqlSidecarError::DatabaseUnavailable("database-transaction-unavailable")
        })?;
        transaction
            .query_one(
                "select pg_catalog.set_config('request.jwt.claims', $1, true)",
                &[&identity.claims_json],
            )
            .map_err(|_| GraphqlSidecarError::DatabaseUnavailable("claims-install-unavailable"))?;
        let variables_json = render_variables_json(&request.variables)?;
        let operation_name = request.operation_name.as_deref();
        let row = transaction
            .query_one(
                "select graphql.resolve($1, $2::text::jsonb, $3)::text",
                &[&request.query, &variables_json, &operation_name],
            )
            .map_err(|_| GraphqlSidecarError::DatabaseUnavailable("graphql-resolve-unavailable"))?;
        let response_json: String = row
            .try_get(0)
            .map_err(|_| GraphqlSidecarError::DatabaseUnavailable("graphql-result-unavailable"))?;
        transaction
            .commit()
            .map_err(|_| GraphqlSidecarError::DatabaseUnavailable("database-commit-unavailable"))?;
        Ok(response_json)
    }
}

pub fn graphql_live_execution_enabled_from_env() -> bool {
    std::env::var(GRAPHQL_LIVE_EXECUTION_ENV)
        .map(|value| matches!(value.as_str(), "1" | "true" | "TRUE" | "yes" | "YES"))
        .unwrap_or(false)
}

fn require_runtime_env<F>(lookup: &F, name: &str) -> Result<String, GraphqlSidecarError>
where
    F: Fn(&str) -> Option<String>,
{
    lookup(name)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| GraphqlSidecarError::MissingRuntimeDependency(name.to_string()))
}

fn validate_postgres_url(field: &str, value: &str) -> Result<(), GraphqlSidecarError> {
    if value.starts_with("postgres://") || value.starts_with("postgresql://") {
        Ok(())
    } else {
        Err(GraphqlSidecarError::InvalidRuntimeDependency(format!(
            "{field} must be a PostgreSQL URL"
        )))
    }
}

// =============================================================================
// Runtime: GraphQL handler with GUC-aware resolution
// =============================================================================

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct GraphqlRequest {
    pub query: String,
    pub variables: BTreeMap<String, Value>,
    pub operation_name: Option<String>,
}

impl GraphqlRequest {
    pub fn new(query: impl Into<String>) -> Self {
        Self {
            query: query.into(),
            variables: BTreeMap::new(),
            operation_name: None,
        }
    }

    pub fn with_variable(mut self, key: impl Into<String>, value: impl Into<Value>) -> Self {
        self.variables.insert(key.into(), value.into());
        self
    }
}

#[derive(Clone, Eq, PartialEq)]
struct GraphqlIdentity {
    tenant_id: String,
    claims_json: String,
}

impl GraphqlIdentity {
    fn from_verified(identity: VerifiedIdentity) -> Self {
        Self {
            tenant_id: identity.tenant_id().to_string(),
            claims_json: identity.claims_json(),
        }
    }

    fn canonical_fixture() -> Self {
        Self {
            tenant_id: "tenant-a".to_string(),
            claims_json: "{\"aud\":\"postgres\",\"exp\":4102444800,\"iat\":0,\"iss\":\"canonical-fixture\",\"jti\":\"canonical-fixture\",\"mfa_verified\":false,\"role\":\"web_anon\",\"sub\":\"canonical-fixture\",\"tenant_id\":\"tenant-a\"}".to_string(),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct GraphqlExecutionPlan {
    pub set_jwt_claims_sql: Option<String>,
    pub resolve_sql: String,
    pub binding_namespace: String,
    pub distributed_types: Vec<String>,
    pub uses_introspection: bool,
    pub uses_subscription: bool,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct GraphqlResponse {
    pub data_json: String,
    pub execution_plan: GraphqlExecutionPlan,
    pub tenant_id: Option<String>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct GraphqlSubscription {
    pub field: String,
    pub notify_channels: Vec<String>,
    pub distributed_types: Vec<String>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct GraphqlHandlerState {
    pub queries_resolved: u64,
    pub subscriptions_registered: u64,
    pub plans_persisted: u64,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct GraphqlHandler {
    plan: GraphqlSidecarPlan,
    persisted_plans: BTreeMap<String, GraphqlExecutionPlan>,
    subscriptions: BTreeMap<String, GraphqlSubscription>,
    queries_resolved: u64,
}

impl GraphqlHandler {
    pub fn new(plan: GraphqlSidecarPlan) -> Result<Self, GraphqlSidecarError> {
        plan.validate()?;
        Ok(Self {
            plan,
            persisted_plans: BTreeMap::new(),
            subscriptions: BTreeMap::new(),
            queries_resolved: 0,
        })
    }

    pub fn plan(&self) -> &GraphqlSidecarPlan {
        &self.plan
    }

    pub fn state(&self) -> GraphqlHandlerState {
        GraphqlHandlerState {
            queries_resolved: self.queries_resolved,
            subscriptions_registered: self.subscriptions.len() as u64,
            plans_persisted: self.persisted_plans.len() as u64,
        }
    }

    /// Returns the SQL the sidecar would run on a tenant-aware Postgres
    /// connection. The function does not open a Postgres connection itself; it
    /// produces the rendered statements so the deployment can run them through
    /// whatever Postgres client is provided (e.g. `tokio-postgres` upstream of
    /// a real deployment, or the canonical fixture in tests).
    fn resolve(
        &mut self,
        request: &GraphqlRequest,
        identity: &GraphqlIdentity,
    ) -> Result<GraphqlResponse, GraphqlSidecarError> {
        let trimmed = request.query.trim();
        if trimmed.is_empty() {
            return Err(GraphqlSidecarError::MalformedQuery(
                "query body must not be empty".to_string(),
            ));
        }
        let lower = trimmed.to_ascii_lowercase();
        let uses_introspection = lower.contains("__schema") || lower.contains("__type");
        if uses_introspection && !self.plan.auth.introspection_enabled {
            return Err(GraphqlSidecarError::IntrospectionDisabled);
        }
        let uses_subscription = lower.starts_with("subscription");

        if identity.tenant_id.trim().is_empty() {
            return Err(GraphqlSidecarError::TenantClaimMissing);
        }
        let tenant_id = Some(identity.tenant_id.clone());
        let distributed_types = self
            .plan
            .distributed_bindings
            .iter()
            .filter(|binding| lower.contains(&binding.type_name.to_ascii_lowercase()))
            .map(|binding| binding.type_name.clone())
            .collect::<Vec<_>>();

        let execution_plan = GraphqlExecutionPlan {
            set_jwt_claims_sql: Some(
                "select pg_catalog.set_config('request.jwt.claims', $1, true)".to_string(),
            ),
            resolve_sql: "select graphql.resolve($1, $2::text::jsonb, $3)::text".to_string(),
            binding_namespace: self.plan.schema_bindings[0].graphql_namespace.clone(),
            distributed_types: distributed_types.clone(),
            uses_introspection,
            uses_subscription,
        };
        self.persisted_plans
            .insert(query_hash(&request.query), execution_plan.clone());

        self.queries_resolved += 1;
        let data_json =
            render_canonical_response(&self.plan, &distributed_types, tenant_id.as_deref());
        Ok(GraphqlResponse {
            data_json,
            execution_plan,
            tenant_id,
        })
    }

    fn register_subscription(
        &mut self,
        request: &GraphqlRequest,
        identity: &GraphqlIdentity,
    ) -> Result<GraphqlSubscription, GraphqlSidecarError> {
        if !request
            .query
            .trim()
            .to_ascii_lowercase()
            .starts_with("subscription")
        {
            return Err(GraphqlSidecarError::MalformedQuery(
                "subscriptions must begin with `subscription`".to_string(),
            ));
        }
        if identity.tenant_id.trim().is_empty() {
            return Err(GraphqlSidecarError::TenantClaimMissing);
        }
        let field = subscription_field(&request.query);
        let notify_channels = self
            .plan
            .schema_bindings
            .iter()
            .flat_map(|binding| {
                binding.exposed_tables.iter().map(|table| {
                    format!(
                        "{}.{}.{}",
                        binding.graphql_namespace, binding.pg_schema, table
                    )
                })
            })
            .collect();
        let distributed_types = self
            .plan
            .distributed_bindings
            .iter()
            .map(|binding| binding.type_name.clone())
            .collect();
        let subscription = GraphqlSubscription {
            field,
            notify_channels,
            distributed_types,
        };
        self.subscriptions
            .insert(query_hash(&request.query), subscription.clone());
        Ok(subscription)
    }

    pub fn persisted_plans(&self) -> &BTreeMap<String, GraphqlExecutionPlan> {
        &self.persisted_plans
    }

    pub fn subscriptions(&self) -> &BTreeMap<String, GraphqlSubscription> {
        &self.subscriptions
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct GraphqlRuntimeReport {
    pub plan: GraphqlSidecarPlan,
    pub response: GraphqlResponse,
    pub subscription: GraphqlSubscription,
    pub state: GraphqlHandlerState,
}

pub fn canonical_graphql_request() -> GraphqlRequest {
    GraphqlRequest::new("query { orderCollection { edges { node { id total } } } }")
}

pub fn canonical_graphql_subscription_request() -> GraphqlRequest {
    GraphqlRequest::new("subscription { orderInserted { id total } }")
}

pub fn canonical_graphql_runtime_report() -> Result<GraphqlRuntimeReport, GraphqlSidecarError> {
    let plan = canonical_graphql_execution_plan()?;
    let mut handler = GraphqlHandler::new(plan.clone())?;
    let identity = GraphqlIdentity::canonical_fixture();
    let response = handler.resolve(&canonical_graphql_request(), &identity)?;
    let subscription =
        handler.register_subscription(&canonical_graphql_subscription_request(), &identity)?;
    Ok(GraphqlRuntimeReport {
        plan,
        response,
        subscription,
        state: handler.state(),
    })
}

fn render_variables_json(
    variables: &BTreeMap<String, Value>,
) -> Result<String, GraphqlSidecarError> {
    serde_json::to_string(variables)
        .map_err(|_| GraphqlSidecarError::PlanResolutionFailed("variables".to_string()))
}

fn render_canonical_response(
    plan: &GraphqlSidecarPlan,
    distributed_types: &[String],
    tenant_id: Option<&str>,
) -> String {
    let types = distributed_types
        .iter()
        .map(|name| format!("\"{name}\""))
        .collect::<Vec<_>>()
        .join(",");
    let tenant = tenant_id
        .map(|value| format!("\"{}\"", value))
        .unwrap_or_else(|| "null".to_string());
    format!(
        "{{\"data\":{{\"namespace\":\"{}\",\"distributed_types\":[{}],\"tenant_id\":{tenant}}}}}",
        plan.schema_bindings[0].graphql_namespace, types,
    )
}

fn subscription_field(query: &str) -> String {
    let trimmed_query = query.trim();
    let after_keyword = trimmed_query
        .split_once(char::is_whitespace)
        .map(|split| split.1)
        .unwrap_or(trimmed_query);
    let trimmed = after_keyword.trim_start_matches('{').trim();
    trimmed
        .split(['{', ' ', '\t', '\n'])
        .find(|word| !word.is_empty())
        .unwrap_or("__unknown")
        .to_string()
}

fn query_hash(query: &str) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in query.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    format!("plan-{:016x}", hash)
}

// =============================================================================
// HTTP front door
// =============================================================================

const MAX_HTTP_REQUEST_BYTES: usize = 65_536;
const MAX_HTTP_BODY_BYTES: usize = 49_152;

trait GraphqlAuthenticator {
    fn authenticate(&self, token: &str) -> Result<GraphqlIdentity, GraphqlSidecarError>;
}

impl GraphqlAuthenticator for AuthIntrospectionClient {
    fn authenticate(&self, token: &str) -> Result<GraphqlIdentity, GraphqlSidecarError> {
        self.introspect(token)
            .map(GraphqlIdentity::from_verified)
            .map_err(GraphqlSidecarError::from)
    }
}

#[derive(Debug)]
struct ParsedHttpRequest<'a> {
    raw: &'a str,
    method: &'a str,
    path: &'a str,
    body: &'a str,
    authorization: Option<&'a str>,
    content_type: Option<&'a str>,
}

impl ParsedHttpRequest<'_> {
    fn raw_bytes(&self) -> &[u8] {
        self.raw.as_bytes()
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct GraphqlHttpBody {
    query: String,
    #[serde(default)]
    variables: BTreeMap<String, Value>,
    #[serde(default, rename = "operationName")]
    operation_name: Option<String>,
}

pub fn handle_graphql_sidecar_http_bytes(
    request: &[u8],
) -> Result<HttpProbeResponse, GraphqlSidecarError> {
    let mut runtime = SidecarRuntime::ready("graphql");
    handle_graphql_sidecar_http_request(request, &mut runtime, None, None)
}

fn handle_graphql_sidecar_http_request(
    request: &[u8],
    runtime: &mut SidecarRuntime,
    authenticator: Option<&dyn GraphqlAuthenticator>,
    live_executor: Option<&mut GraphqlLiveExecutor>,
) -> Result<HttpProbeResponse, GraphqlSidecarError> {
    let request = parse_http_request(request)?;
    let plan = canonical_graphql_execution_plan()?;

    let is_graphql_route =
        matches!(request.path, "/graphql" | "/graphql/ws") || request.path == plan.endpoint_path;
    if (request.method == "POST" || request.method == "GET") && is_graphql_route {
        if request.method == "POST"
            && !request
                .content_type
                .and_then(|value| value.split(';').next())
                .is_some_and(|value| value.trim().eq_ignore_ascii_case("application/json"))
        {
            return Ok(HttpProbeResponse::new(
                415,
                "application/json",
                "{\"errors\":[{\"message\":\"application/json required\"}]}\n",
            ));
        }
        if request.path == "/graphql/ws" {
            if request.method == "POST" {
                return handle_graphql_subscription_post(&plan, &request, authenticator);
            }
            return Ok(HttpProbeResponse::new(
                426,
                "application/json",
                "{\"error\":\"upgrade required: subscriptions use WebSocket transport at /graphql/ws\"}\n",
            ));
        }
        if request.method == "GET" {
            return Ok(HttpProbeResponse::new(
                200,
                "text/html; charset=utf-8",
                render_graphiql(&plan.endpoint_path),
            ));
        }
        return handle_graphql_post(&plan, &request, authenticator, live_executor);
    }

    Ok(runtime.handle_http_bytes(request.raw_bytes())?)
}

fn handle_graphql_subscription_post(
    plan: &GraphqlSidecarPlan,
    http_request: &ParsedHttpRequest<'_>,
    authenticator: Option<&dyn GraphqlAuthenticator>,
) -> Result<HttpProbeResponse, GraphqlSidecarError> {
    let request = match parse_graphql_body(http_request.body) {
        Ok(request) => request,
        Err(error) => return Ok(graphql_error_response(&error)),
    };
    let identity = match authenticate_request(http_request.authorization, authenticator) {
        Ok(identity) => identity,
        Err(error) => {
            log_closed_request_failure("authenticate", &error);
            return Ok(graphql_error_response(&error));
        }
    };

    let mut handler = GraphqlHandler::new(plan.clone())?;
    if let Err(error) = handler.register_subscription(&request, &identity) {
        return Ok(graphql_error_response(&error));
    }
    Ok(HttpProbeResponse::new(
        501,
        "application/json",
        "{\"errors\":[{\"message\":\"subscription transport unavailable\"}]}\n",
    ))
}

fn handle_graphql_post(
    plan: &GraphqlSidecarPlan,
    http_request: &ParsedHttpRequest<'_>,
    authenticator: Option<&dyn GraphqlAuthenticator>,
    live_executor: Option<&mut GraphqlLiveExecutor>,
) -> Result<HttpProbeResponse, GraphqlSidecarError> {
    let request = match parse_graphql_body(http_request.body) {
        Ok(request) => request,
        Err(error) => return Ok(graphql_error_response(&error)),
    };
    let identity = match authenticate_request(http_request.authorization, authenticator) {
        Ok(identity) => identity,
        Err(error) => {
            log_closed_request_failure("authenticate", &error);
            return Ok(graphql_error_response(&error));
        }
    };
    let Some(executor) = live_executor else {
        return Ok(graphql_error_response(
            &GraphqlSidecarError::LiveExecutionRequired,
        ));
    };

    let mut handler = GraphqlHandler::new(plan.clone())?;
    if let Err(error) = handler.resolve(&request, &identity) {
        return Ok(graphql_error_response(&error));
    }
    match executor.execute(&request, &identity) {
        Ok(response_json) => Ok(HttpProbeResponse::new(
            200,
            "application/json",
            format!("{response_json}\n"),
        )),
        Err(error) => {
            log_closed_request_failure("execute", &error);
            Ok(graphql_error_response(&error))
        }
    }
}

fn log_closed_request_failure(stage: &'static str, error: &GraphqlSidecarError) {
    let category = match error {
        GraphqlSidecarError::AuthenticationRejected => "authentication-rejected",
        GraphqlSidecarError::AuthenticationUnavailable => "authentication-unavailable",
        GraphqlSidecarError::DatabaseUnavailable(stage) => stage,
        GraphqlSidecarError::LiveExecutionRequired => "live-execution-required",
        GraphqlSidecarError::TenantClaimMissing => "tenant-claim-missing",
        _ => "request-rejected",
    };
    eprintln!("ai-blaise graphql request rejected: stage={stage} category={category}");
}

fn parse_graphql_body(body: &str) -> Result<GraphqlRequest, GraphqlSidecarError> {
    let body = body.trim();
    if body.is_empty() || body.len() > MAX_HTTP_BODY_BYTES {
        return Err(GraphqlSidecarError::MalformedQuery(
            "invalid request body".to_string(),
        ));
    }
    let value: Value = serde_json::from_str(body)
        .map_err(|_| GraphqlSidecarError::MalformedQuery("invalid JSON body".to_string()))?;
    let object = value.as_object().ok_or_else(|| {
        GraphqlSidecarError::MalformedQuery("request body must be an object".to_string())
    })?;
    if object.contains_key("jwt_claims") || object.contains_key("tenant_id") {
        return Err(GraphqlSidecarError::BodyIdentityForbidden);
    }
    let body: GraphqlHttpBody = serde_json::from_str(body)
        .map_err(|_| GraphqlSidecarError::MalformedQuery("invalid request fields".to_string()))?;
    let mut request = GraphqlRequest::new(body.query);
    request.variables = body.variables;
    request.operation_name = body.operation_name;
    Ok(request)
}

fn authenticate_request(
    authorization: Option<&str>,
    authenticator: Option<&dyn GraphqlAuthenticator>,
) -> Result<GraphqlIdentity, GraphqlSidecarError> {
    let authenticator = authenticator.ok_or(GraphqlSidecarError::AuthenticationUnavailable)?;
    let authorization = authorization.ok_or(GraphqlSidecarError::AuthenticationRejected)?;
    let mut fields = authorization.split_ascii_whitespace();
    let scheme = fields
        .next()
        .ok_or(GraphqlSidecarError::AuthenticationRejected)?;
    let token = fields
        .next()
        .ok_or(GraphqlSidecarError::AuthenticationRejected)?;
    if !scheme.eq_ignore_ascii_case("Bearer") || fields.next().is_some() {
        return Err(GraphqlSidecarError::AuthenticationRejected);
    }
    validate_bearer_token(token).map_err(|_| GraphqlSidecarError::AuthenticationRejected)?;
    authenticator.authenticate(token)
}

fn graphql_error_response(error: &GraphqlSidecarError) -> HttpProbeResponse {
    let (status, message) = match error {
        GraphqlSidecarError::AuthenticationRejected => (401, "authentication rejected"),
        GraphqlSidecarError::AuthenticationUnavailable
        | GraphqlSidecarError::DatabaseUnavailable(_)
        | GraphqlSidecarError::LiveExecutionRequired
        | GraphqlSidecarError::MissingRuntimeDependency(_)
        | GraphqlSidecarError::InvalidRuntimeDependency(_) => (503, "service unavailable"),
        GraphqlSidecarError::BodyIdentityForbidden => {
            (400, "request-body identity claims are forbidden")
        }
        GraphqlSidecarError::IntrospectionDisabled => (400, "introspection disabled"),
        GraphqlSidecarError::TenantClaimMissing => (401, "authentication rejected"),
        _ => (400, "invalid GraphQL request"),
    };
    HttpProbeResponse::new(
        status,
        "application/json",
        format!("{{\"errors\":[{{\"message\":\"{message}\"}}]}}\n"),
    )
}

pub fn serve_graphql_sidecar_http_forever(default_addr: &str) -> Result<(), GraphqlSidecarError> {
    use std::io::Write;
    use std::net::TcpListener;

    canonical_graphql_execution_plan()?;
    let mut runtime = SidecarRuntime::ready("graphql");
    let authenticator = AuthIntrospectionClient::from_env()?;
    let mut live_executor = GraphqlLiveExecutor::connect_from_env()?;
    let listen_addr = listen_addr_from_env(default_addr)?;
    let listener = TcpListener::bind(&listen_addr)?;
    eprintln!("ai-blaise graphql sidecar listening on {listen_addr}");

    for stream in listener.incoming() {
        let mut stream = stream?;
        let request = read_http_request(&mut stream)?;
        let response = handle_graphql_sidecar_http_request(
            request.as_slice(),
            &mut runtime,
            Some(&authenticator),
            Some(&mut live_executor),
        )
        .unwrap_or_else(|error| graphql_error_response(&error));
        stream.write_all(response.to_http_string().as_bytes())?;
    }
    Ok(())
}

fn render_graphiql(endpoint: &str) -> String {
    format!(
        "<!doctype html><html><head><title>ai-blaise GraphQL</title></head><body><pre>POST a JSON {{\\\"query\\\":...}} with an Authorization: Bearer header to {endpoint}.</pre></body></html>\n"
    )
}

fn read_http_request(
    stream: &mut std::net::TcpStream,
) -> Result<Zeroizing<Vec<u8>>, std::io::Error> {
    use std::io::Read;
    stream.set_read_timeout(Some(std::time::Duration::from_secs(5)))?;
    let mut request = Zeroizing::new(Vec::new());
    let mut chunk = Zeroizing::new([0_u8; 8192]);
    loop {
        let read_len = stream.read(&mut chunk[..])?;
        if read_len == 0 {
            break;
        }
        request.extend_from_slice(&chunk[..read_len]);
        if http_request_complete(&request) || request.len() >= 65_536 {
            break;
        }
    }
    Ok(request)
}

fn http_request_complete(request: &[u8]) -> bool {
    let Some((body_start, header_bytes)) = split_http_head(request) else {
        return false;
    };
    let Ok(headers) = std::str::from_utf8(header_bytes) else {
        return true;
    };
    let content_length = headers
        .lines()
        .filter_map(|line| line.split_once(':'))
        .find(|(name, _)| name.eq_ignore_ascii_case("content-length"))
        .and_then(|(_, value)| value.trim().parse::<usize>().ok())
        .unwrap_or(0);
    request.len() >= body_start + content_length
}

fn split_http_head(request: &[u8]) -> Option<(usize, &[u8])> {
    find_bytes(request, b"\r\n\r\n")
        .map(|index| (index + 4, &request[..index]))
        .or_else(|| find_bytes(request, b"\n\n").map(|index| (index + 2, &request[..index])))
}

fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

fn parse_http_request(request: &[u8]) -> Result<ParsedHttpRequest<'_>, GraphqlSidecarError> {
    if request.len() > MAX_HTTP_REQUEST_BYTES {
        return Err(GraphqlSidecarError::MalformedHttpRequest);
    }
    let request =
        std::str::from_utf8(request).map_err(|_| GraphqlSidecarError::MalformedHttpRequest)?;
    let (head, body) = request
        .split_once("\r\n\r\n")
        .or_else(|| request.split_once("\n\n"))
        .ok_or(GraphqlSidecarError::MalformedHttpRequest)?;
    let request_line = head
        .lines()
        .next()
        .ok_or(GraphqlSidecarError::MalformedHttpRequest)?;
    let mut parts = request_line.split_whitespace();
    let method = parts
        .next()
        .ok_or(GraphqlSidecarError::MalformedHttpRequest)?;
    let path = parts
        .next()
        .ok_or(GraphqlSidecarError::MalformedHttpRequest)?;
    let version = parts
        .next()
        .ok_or(GraphqlSidecarError::MalformedHttpRequest)?;
    if parts.next().is_some()
        || !path.starts_with('/')
        || !matches!(version, "HTTP/1.0" | "HTTP/1.1")
    {
        return Err(GraphqlSidecarError::MalformedHttpRequest);
    }
    let mut authorization = None;
    let mut content_length = None;
    let mut content_type = None;
    for line in head.lines().skip(1) {
        if line.is_empty()
            || line.starts_with([' ', '\t'])
            || line
                .bytes()
                .any(|byte| byte.is_ascii_control() && byte != b'\t')
        {
            return Err(GraphqlSidecarError::MalformedHttpRequest);
        }
        let (name, value) = line
            .split_once(':')
            .ok_or(GraphqlSidecarError::MalformedHttpRequest)?;
        if name.is_empty()
            || !name
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        {
            return Err(GraphqlSidecarError::MalformedHttpRequest);
        }
        let value = value.trim();
        if name.eq_ignore_ascii_case("authorization") && authorization.replace(value).is_some() {
            return Err(GraphqlSidecarError::MalformedHttpRequest);
        }
        if name.eq_ignore_ascii_case("content-length")
            && content_length
                .replace(
                    value
                        .parse::<usize>()
                        .map_err(|_| GraphqlSidecarError::MalformedHttpRequest)?,
                )
                .is_some()
        {
            return Err(GraphqlSidecarError::MalformedHttpRequest);
        }
        if name.eq_ignore_ascii_case("content-type") && content_type.replace(value).is_some() {
            return Err(GraphqlSidecarError::MalformedHttpRequest);
        }
        if name.eq_ignore_ascii_case("transfer-encoding") {
            return Err(GraphqlSidecarError::MalformedHttpRequest);
        }
    }
    if body.len() > MAX_HTTP_BODY_BYTES
        || content_length.is_some_and(|length| length != body.len())
        || (method == "POST" && content_length.is_none())
    {
        return Err(GraphqlSidecarError::MalformedHttpRequest);
    }
    Ok(ParsedHttpRequest {
        raw: request,
        method,
        path,
        body,
        authorization,
        content_type,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    struct AcceptingAuthenticator;

    impl GraphqlAuthenticator for AcceptingAuthenticator {
        fn authenticate(&self, token: &str) -> Result<GraphqlIdentity, GraphqlSidecarError> {
            if token != "header.token.value" {
                return Err(GraphqlSidecarError::AuthenticationRejected);
            }
            Ok(GraphqlIdentity::canonical_fixture())
        }
    }

    struct UnavailableAuthenticator;

    impl GraphqlAuthenticator for UnavailableAuthenticator {
        fn authenticate(&self, _token: &str) -> Result<GraphqlIdentity, GraphqlSidecarError> {
            Err(GraphqlSidecarError::AuthenticationUnavailable)
        }
    }

    fn runtime_dependency(name: &str) -> Option<String> {
        match name {
            GRAPHQL_DATABASE_URL_ENV => {
                Some("postgresql://postgres@127.0.0.1/postgres".to_string())
            }
            AUTH_INTROSPECTION_URL_ENV => {
                Some("https://auth.example.com/auth/introspect".to_string())
            }
            AUTH_CA_CERT_PATH_ENV => Some("/run/secrets/auth-ca.pem".to_string()),
            AUTH_CLIENT_IDENTITY_PATH_ENV => Some("/run/secrets/graphql-client.pem".to_string()),
            AUTH_EXPECTED_ISSUER_ENV => Some("https://issuer.example.com".to_string()),
            AUTH_EXPECTED_AUDIENCE_ENV => Some("postgres".to_string()),
            _ => None,
        }
    }

    fn post_request(path: &str, body: &str, authorization: Option<&str>) -> Vec<u8> {
        let authorization = authorization
            .map(|value| format!("Authorization: {value}\r\n"))
            .unwrap_or_default();
        format!(
            "POST {path} HTTP/1.1\r\nHost: local\r\nContent-Type: application/json\r\n{authorization}Content-Length: {}\r\n\r\n{body}",
            body.len()
        )
        .into_bytes()
    }

    fn handle_with_auth(
        request: &[u8],
        authenticator: &dyn GraphqlAuthenticator,
    ) -> HttpProbeResponse {
        let mut runtime = SidecarRuntime::ready("graphql");
        handle_graphql_sidecar_http_request(request, &mut runtime, Some(authenticator), None)
            .expect("HTTP response")
    }

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

    #[test]
    fn runtime_dependency_report_fails_closed_without_database_url() {
        let plan = canonical_graphql_plan();

        assert_eq!(
            graphql_runtime_dependency_report(&plan, |_| None),
            Err(GraphqlSidecarError::MissingRuntimeDependency(
                "AI_BLAISE_GRAPHQL_DATABASE_URL".to_string()
            ))
        );
    }

    #[test]
    fn runtime_dependency_report_rejects_invalid_database_url() {
        let plan = canonical_graphql_plan();
        let err = graphql_runtime_dependency_report(&plan, |name| match name {
            GRAPHQL_DATABASE_URL_ENV => Some("http://postgres".to_string()),
            _ => None,
        })
        .expect_err("invalid database url");
        assert!(err.to_string().contains("must be a PostgreSQL URL"));
    }

    #[test]
    fn runtime_dependency_report_names_pg_graphql_boundary() {
        let plan = canonical_graphql_plan();
        let report =
            graphql_runtime_dependency_report(&plan, runtime_dependency).expect("dependencies");

        assert_eq!(report.database_url_env, GRAPHQL_DATABASE_URL_ENV);
        assert_eq!(
            report.auth_introspection_url_env,
            AUTH_INTROSPECTION_URL_ENV
        );
        assert_eq!(report.auth_ca_cert_path_env, AUTH_CA_CERT_PATH_ENV);
        assert_eq!(
            report.auth_client_identity_path_env,
            AUTH_CLIENT_IDENTITY_PATH_ENV
        );
        assert_eq!(report.auth_expected_issuer_env, AUTH_EXPECTED_ISSUER_ENV);
        assert_eq!(
            report.auth_expected_audience_env,
            AUTH_EXPECTED_AUDIENCE_ENV
        );
        assert_eq!(report.auth_timeout_ms_env, AUTH_TIMEOUT_MS_ENV);
        assert_eq!(report.endpoint_path, "/graphql/v1");
        assert!(report.pg_graphql_extension_required);
    }

    #[test]
    fn resolve_renders_set_claims_and_resolve_sql() {
        let plan = canonical_graphql_plan();
        let mut handler = GraphqlHandler::new(plan).expect("handler");
        let identity = GraphqlIdentity::canonical_fixture();

        let response = handler
            .resolve(&canonical_graphql_request(), &identity)
            .expect("resolve");

        let set_claims = response
            .execution_plan
            .set_jwt_claims_sql
            .as_deref()
            .expect("set_claims");
        assert_eq!(
            set_claims,
            "select pg_catalog.set_config('request.jwt.claims', $1, true)"
        );
        assert_eq!(
            response.execution_plan.resolve_sql,
            "select graphql.resolve($1, $2::text::jsonb, $3)::text"
        );
        assert!(!set_claims.contains("tenant-a"));
        assert!(!response.execution_plan.resolve_sql.contains("tenant-a"));
        assert_eq!(response.tenant_id.as_deref(), Some("tenant-a"));
        assert!(response
            .execution_plan
            .distributed_types
            .contains(&"Order".to_string()));
        assert_eq!(handler.state().queries_resolved, 1);
        assert_eq!(handler.state().plans_persisted, 1);
    }

    #[test]
    fn resolve_rejects_introspection_when_disabled() {
        let plan = canonical_graphql_plan();
        let mut handler = GraphqlHandler::new(plan).expect("handler");
        let identity = GraphqlIdentity::canonical_fixture();

        let request = GraphqlRequest::new("query { __schema { types { name } } }");

        assert_eq!(
            handler.resolve(&request, &identity),
            Err(GraphqlSidecarError::IntrospectionDisabled)
        );
    }

    #[test]
    fn resolve_requires_tenant_claim_when_rls_required() {
        let plan = canonical_graphql_plan();
        let mut handler = GraphqlHandler::new(plan).expect("handler");

        let request = GraphqlRequest::new("query { orderCollection { edges { node { id } } } }");
        let empty_identity = GraphqlIdentity {
            tenant_id: String::new(),
            claims_json: "{}".to_string(),
        };

        assert_eq!(
            handler.resolve(&request, &empty_identity),
            Err(GraphqlSidecarError::TenantClaimMissing)
        );
    }

    #[test]
    fn register_subscription_records_notify_channels() {
        let plan = canonical_graphql_plan();
        let mut handler = GraphqlHandler::new(plan).expect("handler");
        let identity = GraphqlIdentity::canonical_fixture();

        let subscription = handler
            .register_subscription(&canonical_graphql_subscription_request(), &identity)
            .expect("subscription");

        assert!(subscription
            .notify_channels
            .contains(&"public_api.public.orders".to_string()));
        assert!(subscription
            .distributed_types
            .contains(&"Order".to_string()));
        assert_eq!(handler.state().subscriptions_registered, 1);
    }

    #[test]
    fn canonical_graphql_runtime_report_is_deterministic() {
        let report = canonical_graphql_runtime_report().expect("report");

        assert_eq!(report.state.queries_resolved, 1);
        assert_eq!(report.state.subscriptions_registered, 1);
        assert!(report
            .response
            .data_json
            .contains("\"namespace\":\"public_api\""));
        assert_eq!(report.subscription.field, "orderInserted");
    }

    #[test]
    fn public_http_front_door_serves_graphiql_but_fails_post_closed() {
        let get =
            handle_graphql_sidecar_http_bytes(b"GET /graphql HTTP/1.1\r\nHost: local\r\n\r\n")
                .expect("get");
        assert_eq!(get.status_code, 200);
        assert!(get.body.contains("ai-blaise GraphQL"));
        assert!(get.body.contains("Authorization: Bearer"));
        assert!(!get.body.contains("jwt_claims"));

        let request = post_request(
            "/graphql/v1",
            r#"{"query":"query { orderCollection { edges { node { id } } } }"}"#,
            Some("Bearer header.token.value"),
        );
        let response = handle_graphql_sidecar_http_bytes(&request).expect("post");
        assert_eq!(response.status_code, 503);
        assert_eq!(
            response.body,
            "{\"errors\":[{\"message\":\"service unavailable\"}]}\n"
        );
    }

    #[test]
    fn http_front_door_returns_healthz() {
        let response =
            handle_graphql_sidecar_http_bytes(b"GET /healthz HTTP/1.1\r\nHost: local\r\n\r\n")
                .expect("healthz");
        assert_eq!(response.status_code, 200);
        assert!(response.body.contains("\"component\":\"graphql\""));
    }

    #[test]
    fn body_identity_is_forbidden_before_authentication() {
        for field in ["jwt_claims", "tenant_id"] {
            let body = format!(
                "{{\"query\":\"query {{ orderCollection {{ edges {{ node {{ id }} }} }} }}\",\"{field}\":\"attacker\"}}"
            );
            let request = post_request("/graphql/v1", &body, Some("Bearer header.token.value"));
            let response = handle_with_auth(&request, &AcceptingAuthenticator);
            assert_eq!(response.status_code, 400);
            assert!(response.body.contains("identity claims are forbidden"));
            assert!(!response.body.contains("attacker"));
            assert!(!response.body.contains("header.token.value"));
        }
    }

    #[test]
    fn query_routes_require_one_validated_bearer_and_live_execution() {
        let body = r#"{"query":"query { orderCollection { edges { node { id } } } }","variables":{"count":3,"nested":{"safe":true}}}"#;

        let missing = post_request("/graphql/v1", body, None);
        let response = handle_with_auth(&missing, &AcceptingAuthenticator);
        assert_eq!(response.status_code, 401);

        for value in [
            "Basic abc",
            "Bearer",
            "Bearer bad token",
            "Bearer bad\\token",
        ] {
            let request = post_request("/graphql/v1", body, Some(value));
            let response = handle_with_auth(&request, &AcceptingAuthenticator);
            assert_eq!(response.status_code, 401);
            assert!(!response.body.contains(value));
        }

        let accepted = post_request("/graphql/v1", body, Some("Bearer header.token.value"));
        let response = handle_with_auth(&accepted, &AcceptingAuthenticator);
        assert_eq!(response.status_code, 503);
        assert!(!response.body.contains("tenant-a"));
        assert!(!response.body.contains("header.token.value"));

        let unavailable = handle_with_auth(&accepted, &UnavailableAuthenticator);
        assert_eq!(unavailable.status_code, 503);
        assert_eq!(unavailable.body, response.body);
    }

    #[test]
    fn websocket_boundary_authenticates_then_fails_unimplemented_transport_closed() {
        let body = r#"{"query":"subscription { orderInserted { id total } }"}"#;
        let unauthorized = post_request("/graphql/ws", body, None);
        let unauthorized = handle_with_auth(&unauthorized, &AcceptingAuthenticator);
        assert_eq!(unauthorized.status_code, 401);

        let request = post_request("/graphql/ws", body, Some("Bearer header.token.value"));
        let response = handle_with_auth(&request, &AcceptingAuthenticator);

        assert_eq!(response.status_code, 501);
        assert_eq!(
            response.body,
            "{\"errors\":[{\"message\":\"subscription transport unavailable\"}]}\n"
        );
        assert!(!response.body.contains("tenant-a"));
    }

    #[test]
    fn http_parser_rejects_duplicate_authority_and_ambiguous_framing() {
        let duplicate = b"POST /graphql/v1 HTTP/1.1\r\nAuthorization: Bearer first.token\r\nAuthorization: Bearer second.token\r\nContent-Length: 2\r\n\r\n{}";
        assert_eq!(
            handle_graphql_sidecar_http_bytes(duplicate),
            Err(GraphqlSidecarError::MalformedHttpRequest)
        );
        let duplicate_length =
            b"POST /graphql/v1 HTTP/1.1\r\nContent-Length: 2\r\nContent-Length: 2\r\n\r\n{}";
        assert_eq!(
            handle_graphql_sidecar_http_bytes(duplicate_length),
            Err(GraphqlSidecarError::MalformedHttpRequest)
        );
        let mismatched_length = b"POST /graphql/v1 HTTP/1.1\r\nContent-Length: 9\r\n\r\n{}";
        assert_eq!(
            handle_graphql_sidecar_http_bytes(mismatched_length),
            Err(GraphqlSidecarError::MalformedHttpRequest)
        );
        let transfer_encoding = b"POST /graphql/v1 HTTP/1.1\r\nTransfer-Encoding: chunked\r\nContent-Type: application/json\r\n\r\n2\r\n{}\r\n0\r\n\r\n";
        assert_eq!(
            handle_graphql_sidecar_http_bytes(transfer_encoding),
            Err(GraphqlSidecarError::MalformedHttpRequest)
        );
        let missing_length =
            b"POST /graphql/v1 HTTP/1.1\r\nContent-Type: application/json\r\n\r\n{}";
        assert_eq!(
            handle_graphql_sidecar_http_bytes(missing_length),
            Err(GraphqlSidecarError::MalformedHttpRequest)
        );
        let missing_content_type = b"POST /graphql/v1 HTTP/1.1\r\nContent-Length: 2\r\n\r\n{}";
        let response =
            handle_graphql_sidecar_http_bytes(missing_content_type).expect("HTTP response");
        assert_eq!(response.status_code, 415);
    }

    #[test]
    fn graphql_body_parser_rejects_unknown_and_duplicate_fields() {
        for body in [
            r#"{"query":"query { ok }","unknown":true}"#,
            r#"{"query":"query { first }","query":"query { second }"}"#,
            r#"["query { ok }"]"#,
        ] {
            let result = parse_graphql_body(body);
            assert!(
                matches!(&result, Err(GraphqlSidecarError::MalformedQuery(_))),
                "unexpected parse result for {body:?}: {result:?}"
            );
        }
    }
}
