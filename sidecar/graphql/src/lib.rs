//! GraphQL sidecar contracts.

// FEATURE: API3
// FEATURE: API4
// FEATURE: API5

use ai_blaise_citus_sidecar_shared::{
    listen_addr_from_env, HttpProbeResponse, SidecarRuntime, SidecarRuntimeError,
};
use postgres::{Client, NoTls};
use std::collections::BTreeMap;
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
    InvalidRuntimeDependency(String),
    IntrospectionDisabled,
    MalformedHttpRequest,
    MalformedQuery(String),
    MissingRequiredField(&'static str),
    MissingRuntimeDependency(String),
    PlanResolutionFailed(String),
    Runtime(String),
    TenantClaimMissing,
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
        }
    }
}

impl Error for GraphqlSidecarError {}

impl From<SidecarRuntimeError> for GraphqlSidecarError {
    fn from(error: SidecarRuntimeError) -> Self {
        Self::Runtime(error.to_string())
    }
}

impl From<std::io::Error> for GraphqlSidecarError {
    fn from(error: std::io::Error) -> Self {
        Self::Runtime(error.to_string())
    }
}

impl From<postgres::Error> for GraphqlSidecarError {
    fn from(error: postgres::Error) -> Self {
        if let Some(db_error) = error.as_db_error() {
            return Self::Runtime(format!(
                "{}: {}",
                db_error.code().code(),
                db_error.message()
            ));
        }
        Self::Runtime(error.to_string())
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

pub const GRAPHQL_DATABASE_URL_ENV: &str = "AI_BLAISE_GRAPHQL_DATABASE_URL";
pub const GRAPHQL_JWT_SECRET_ENV: &str = "AI_BLAISE_GRAPHQL_JWT_SECRET";
pub const GRAPHQL_LIVE_EXECUTION_ENV: &str = "AI_BLAISE_GRAPHQL_LIVE_EXECUTION";
const MIN_JWT_SECRET_BYTES: usize = 32;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct GraphqlRuntimeDependencyReport {
    pub database_url_env: String,
    pub jwt_secret_env: String,
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
    let jwt_secret = require_runtime_env(&lookup, GRAPHQL_JWT_SECRET_ENV)?;
    validate_jwt_secret(GRAPHQL_JWT_SECRET_ENV, &jwt_secret)?;

    Ok(GraphqlRuntimeDependencyReport {
        database_url_env: GRAPHQL_DATABASE_URL_ENV.to_string(),
        jwt_secret_env: GRAPHQL_JWT_SECRET_ENV.to_string(),
        endpoint_path: plan.endpoint_path.clone(),
        pg_graphql_extension_required: true,
    })
}

pub struct GraphqlLiveExecutor {
    client: Client,
}

impl GraphqlLiveExecutor {
    pub fn connect_from_env() -> Result<Option<Self>, GraphqlSidecarError> {
        if !graphql_live_execution_enabled_from_env() {
            return Ok(None);
        }
        let report = graphql_runtime_dependency_report_from_env()?;
        Ok(Some(Self::connect_env(&report.database_url_env)?))
    }

    pub fn connect_env(database_url_env: &str) -> Result<Self, GraphqlSidecarError> {
        let database_url = std::env::var(database_url_env)
            .map_err(|_| GraphqlSidecarError::MissingRuntimeDependency(database_url_env.into()))?;
        Self::connect(&database_url)
    }

    pub fn connect(database_url: &str) -> Result<Self, GraphqlSidecarError> {
        validate_postgres_url(GRAPHQL_DATABASE_URL_ENV, database_url)?;
        let mut client = Client::connect(database_url, NoTls)?;
        let extension_exists: bool = client
            .query_one(
                "select exists (select 1 from pg_extension where extname = 'pg_graphql')",
                &[],
            )?
            .get(0);
        if !extension_exists {
            return Err(GraphqlSidecarError::MissingRuntimeDependency(
                "pg_graphql extension".to_string(),
            ));
        }
        Ok(Self { client })
    }

    pub fn execute(&mut self, request: &GraphqlRequest) -> Result<String, GraphqlSidecarError> {
        let mut transaction = self.client.transaction()?;
        if let Some(claims_json) = request.jwt_claims_json.as_deref() {
            transaction.simple_query(&render_set_claims_sql(claims_json))?;
        }
        let row = transaction.query_one(&format!("{}::text", render_resolve_sql(request)), &[])?;
        let response_json: String = row.get(0);
        transaction.commit()?;
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

fn validate_jwt_secret(field: &str, value: &str) -> Result<(), GraphqlSidecarError> {
    if value.len() >= MIN_JWT_SECRET_BYTES {
        Ok(())
    } else {
        Err(GraphqlSidecarError::InvalidRuntimeDependency(format!(
            "{field} must be at least {MIN_JWT_SECRET_BYTES} bytes"
        )))
    }
}

// =============================================================================
// Runtime: GraphQL handler with GUC-aware resolution
// =============================================================================

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct GraphqlRequest {
    pub query: String,
    pub variables: BTreeMap<String, String>,
    pub operation_name: Option<String>,
    pub jwt_claims_json: Option<String>,
}

impl GraphqlRequest {
    pub fn new(query: impl Into<String>) -> Self {
        Self {
            query: query.into(),
            variables: BTreeMap::new(),
            operation_name: None,
            jwt_claims_json: None,
        }
    }

    pub fn with_variable(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.variables.insert(key.into(), value.into());
        self
    }

    pub fn with_jwt_claims_json(mut self, claims: impl Into<String>) -> Self {
        self.jwt_claims_json = Some(claims.into());
        self
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
    pub fn resolve(
        &mut self,
        request: &GraphqlRequest,
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

        let tenant_id = self.extract_tenant_id(request.jwt_claims_json.as_deref())?;
        let set_jwt_claims_sql = request
            .jwt_claims_json
            .as_deref()
            .map(render_set_claims_sql);

        let resolve_sql = render_resolve_sql(request);
        let distributed_types = self
            .plan
            .distributed_bindings
            .iter()
            .filter(|binding| lower.contains(&binding.type_name.to_ascii_lowercase()))
            .map(|binding| binding.type_name.clone())
            .collect::<Vec<_>>();

        let execution_plan = GraphqlExecutionPlan {
            set_jwt_claims_sql,
            resolve_sql,
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

    pub fn register_subscription(
        &mut self,
        request: &GraphqlRequest,
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
        let _ = self.extract_tenant_id(request.jwt_claims_json.as_deref())?;
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

    fn extract_tenant_id(
        &self,
        claims_json: Option<&str>,
    ) -> Result<Option<String>, GraphqlSidecarError> {
        if !self.plan.auth.rls_required {
            return Ok(None);
        }
        let Some(claims) = claims_json else {
            return Err(GraphqlSidecarError::TenantClaimMissing);
        };
        let tenant_id = extract_string_field(claims, &self.plan.auth.tenant_claim)
            .ok_or(GraphqlSidecarError::TenantClaimMissing)?;
        Ok(Some(tenant_id))
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
        .with_jwt_claims_json("{\"tenant_id\":\"tenant-a\",\"role\":\"web_anon\"}")
}

pub fn canonical_graphql_subscription_request() -> GraphqlRequest {
    GraphqlRequest::new("subscription { orderInserted { id total } }")
        .with_jwt_claims_json("{\"tenant_id\":\"tenant-a\",\"role\":\"web_anon\"}")
}

pub fn canonical_graphql_runtime_report() -> Result<GraphqlRuntimeReport, GraphqlSidecarError> {
    let plan = canonical_graphql_execution_plan()?;
    let mut handler = GraphqlHandler::new(plan.clone())?;
    let response = handler.resolve(&canonical_graphql_request())?;
    let subscription = handler.register_subscription(&canonical_graphql_subscription_request())?;
    Ok(GraphqlRuntimeReport {
        plan,
        response,
        subscription,
        state: handler.state(),
    })
}

fn render_set_claims_sql(claims_json: &str) -> String {
    format!(
        "select set_config('request.jwt.claims', '{}', true)",
        claims_json.replace('\'', "''")
    )
}

fn render_resolve_sql(request: &GraphqlRequest) -> String {
    let escaped_query = request.query.replace('\'', "''");
    let variables_json = render_variables_json(&request.variables);
    let operation = request
        .operation_name
        .as_deref()
        .map(|name| format!("'{}'", name.replace('\'', "''")))
        .unwrap_or_else(|| "null".to_string());
    format!("select graphql.resolve('{escaped_query}', '{variables_json}'::jsonb, {operation})")
}

fn render_variables_json(variables: &BTreeMap<String, String>) -> String {
    if variables.is_empty() {
        return "{}".to_string();
    }
    let entries = variables
        .iter()
        .map(|(key, value)| format!("\"{}\":\"{}\"", key, value.replace('\'', "''")))
        .collect::<Vec<_>>()
        .join(",");
    format!("{{{entries}}}")
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

fn extract_string_field(claims_json: &str, field: &str) -> Option<String> {
    let needle = format!("\"{field}\":");
    let start = claims_json.find(&needle)? + needle.len();
    let mut chars = claims_json[start..].chars().peekable();
    while let Some(ch) = chars.peek() {
        if ch.is_whitespace() {
            chars.next();
        } else {
            break;
        }
    }
    if chars.peek() == Some(&'"') {
        chars.next();
        let mut value = String::new();
        while let Some(ch) = chars.next() {
            if ch == '\\' {
                if let Some(next) = chars.next() {
                    match next {
                        '"' => value.push('"'),
                        '\\' => value.push('\\'),
                        '/' => value.push('/'),
                        'n' => value.push('\n'),
                        'r' => value.push('\r'),
                        't' => value.push('\t'),
                        other => value.push(other),
                    }
                }
                continue;
            }
            if ch == '"' {
                return Some(value);
            }
            value.push(ch);
        }
        None
    } else {
        let mut value = String::new();
        for ch in chars {
            if ch == ',' || ch == '}' {
                let trimmed = value.trim();
                if trimmed.is_empty() {
                    return None;
                }
                return Some(trimmed.to_string());
            }
            value.push(ch);
        }
        None
    }
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

pub fn handle_graphql_sidecar_http_bytes(
    request: &[u8],
) -> Result<HttpProbeResponse, GraphqlSidecarError> {
    let mut runtime = SidecarRuntime::ready("graphql");
    handle_graphql_sidecar_http_request(request, &mut runtime, None)
}

fn handle_graphql_sidecar_http_request(
    request: &[u8],
    runtime: &mut SidecarRuntime,
    live_executor: Option<&mut GraphqlLiveExecutor>,
) -> Result<HttpProbeResponse, GraphqlSidecarError> {
    let request =
        std::str::from_utf8(request).map_err(|_| GraphqlSidecarError::MalformedHttpRequest)?;
    let (method, path, body) = parse_http_request(request)?;
    let plan = canonical_graphql_execution_plan()?;

    if (method == "POST" || method == "GET") && path.starts_with("/graphql") {
        if path == "/graphql/ws" {
            if method == "POST" {
                return handle_graphql_subscription_post(&plan, body);
            }
            return Ok(HttpProbeResponse::new(
                426,
                "application/json",
                "{\"error\":\"upgrade required: subscriptions use WebSocket transport at /graphql/ws\"}\n",
            ));
        }
        if method == "GET" {
            return Ok(HttpProbeResponse::new(
                200,
                "text/html; charset=utf-8",
                render_graphiql(&plan.endpoint_path),
            ));
        }
        return handle_graphql_post(&plan, body, live_executor);
    }

    Ok(runtime.handle_http_bytes(request.as_bytes())?)
}

fn handle_graphql_subscription_post(
    plan: &GraphqlSidecarPlan,
    body: &str,
) -> Result<HttpProbeResponse, GraphqlSidecarError> {
    let body = body.trim();
    if body.is_empty() {
        return Ok(HttpProbeResponse::new(
            400,
            "application/json",
            "{\"errors\":[{\"message\":\"empty GraphQL subscription body\"}]}\n",
        ));
    }
    let query = extract_string_field(body, "query").ok_or_else(|| {
        GraphqlSidecarError::MalformedQuery("missing subscription query field".to_string())
    })?;
    let claims_json = extract_string_field(body, "jwt_claims");
    let mut request = GraphqlRequest::new(query);
    if let Some(claims) = claims_json {
        request = request.with_jwt_claims_json(claims);
    }

    let mut handler = GraphqlHandler::new(plan.clone())?;
    match handler.register_subscription(&request) {
        Ok(subscription) => Ok(HttpProbeResponse::new(
            200,
            "application/json",
            render_subscription_boundary(&subscription),
        )),
        Err(error) => Ok(HttpProbeResponse::new(
            400,
            "application/json",
            format!(
                "{{\"errors\":[{{\"message\":\"{}\"}}]}}\n",
                escape_json(&error.to_string())
            ),
        )),
    }
}

fn handle_graphql_post(
    plan: &GraphqlSidecarPlan,
    body: &str,
    live_executor: Option<&mut GraphqlLiveExecutor>,
) -> Result<HttpProbeResponse, GraphqlSidecarError> {
    let body = body.trim();
    if body.is_empty() {
        return Ok(HttpProbeResponse::new(
            400,
            "application/json",
            "{\"errors\":[{\"message\":\"empty GraphQL body\"}]}\n",
        ));
    }
    let query = extract_string_field(body, "query")
        .ok_or_else(|| GraphqlSidecarError::MalformedQuery("missing query field".to_string()))?;
    let claims_json = extract_string_field(body, "jwt_claims");
    let operation_name = extract_string_field(body, "operationName");

    let mut request = GraphqlRequest::new(query);
    if let Some(claims) = claims_json {
        request = request.with_jwt_claims_json(claims);
    }
    if let Some(name) = operation_name {
        request.operation_name = Some(name);
    }

    let mut handler = GraphqlHandler::new(plan.clone())?;
    if let Some(executor) = live_executor {
        if let Err(error) = handler.resolve(&request) {
            return Ok(HttpProbeResponse::new(
                400,
                "application/json",
                format!(
                    "{{\"errors\":[{{\"message\":\"{}\"}}]}}\n",
                    escape_json(&error.to_string())
                ),
            ));
        }
        return match executor.execute(&request) {
            Ok(response_json) => Ok(HttpProbeResponse::new(
                200,
                "application/json",
                format!("{response_json}\n"),
            )),
            Err(error) => Ok(HttpProbeResponse::new(
                502,
                "application/json",
                format!(
                    "{{\"errors\":[{{\"message\":\"{}\"}}]}}\n",
                    escape_json(&error.to_string())
                ),
            )),
        };
    }

    match handler.resolve(&request) {
        Ok(response) => Ok(HttpProbeResponse::new(
            200,
            "application/json",
            format!("{}\n", response.data_json),
        )),
        Err(error) => Ok(HttpProbeResponse::new(
            400,
            "application/json",
            format!(
                "{{\"errors\":[{{\"message\":\"{}\"}}]}}\n",
                escape_json(&error.to_string())
            ),
        )),
    }
}

pub fn serve_graphql_sidecar_http_forever(default_addr: &str) -> Result<(), GraphqlSidecarError> {
    use std::io::Write;
    use std::net::TcpListener;

    canonical_graphql_execution_plan()?;
    let mut runtime = SidecarRuntime::ready("graphql");
    let mut live_executor = GraphqlLiveExecutor::connect_from_env()?;
    let listen_addr = listen_addr_from_env(default_addr)?;
    let listener = TcpListener::bind(&listen_addr)?;
    eprintln!("ai-blaise graphql sidecar listening on {listen_addr}");

    for stream in listener.incoming() {
        let mut stream = stream?;
        let request = read_http_request(&mut stream)?;
        let response =
            handle_graphql_sidecar_http_request(&request, &mut runtime, live_executor.as_mut())
                .unwrap_or_else(|error| {
                    HttpProbeResponse::new(
                        400,
                        "application/json",
                        format!(
                            "{{\"errors\":[{{\"message\":\"{}\"}}]}}\n",
                            escape_json(&error.to_string())
                        ),
                    )
                });
        stream.write_all(response.to_http_string().as_bytes())?;
    }
    Ok(())
}

fn render_subscription_boundary(subscription: &GraphqlSubscription) -> String {
    let channels = subscription
        .notify_channels
        .iter()
        .map(|channel| format!("\"{}\"", escape_json(channel)))
        .collect::<Vec<_>>()
        .join(",");
    let distributed_types = subscription
        .distributed_types
        .iter()
        .map(|type_name| format!("\"{}\"", escape_json(type_name)))
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "{{\"transport\":\"websocket\",\"protocol\":\"graphql-transport-ws\",\"subscription_field\":\"{}\",\"notify_channels\":[{}],\"distributed_types\":[{}]}}\n",
        escape_json(&subscription.field),
        channels,
        distributed_types,
    )
}

fn render_graphiql(endpoint: &str) -> String {
    format!(
        "<!doctype html><html><head><title>ai-blaise GraphQL</title></head><body><pre>POST a JSON {{\\\"query\\\":...,\\\"jwt_claims\\\":...}} to {endpoint} to execute queries.</pre></body></html>\n"
    )
}

fn read_http_request(stream: &mut std::net::TcpStream) -> Result<Vec<u8>, std::io::Error> {
    use std::io::Read;
    stream.set_read_timeout(Some(std::time::Duration::from_secs(5)))?;
    let mut request = Vec::new();
    let mut chunk = [0_u8; 8192];
    loop {
        let read_len = stream.read(&mut chunk)?;
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

fn parse_http_request(request: &str) -> Result<(&str, &str, &str), GraphqlSidecarError> {
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
    if !path.starts_with('/') {
        return Err(GraphqlSidecarError::MalformedHttpRequest);
    }
    Ok((method, path, body))
}

fn escape_json(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
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
            GRAPHQL_JWT_SECRET_ENV => Some("01234567890123456789012345678901".to_string()),
            _ => None,
        })
        .expect_err("invalid database url");
        assert!(err.to_string().contains("must be a PostgreSQL URL"));
    }

    #[test]
    fn runtime_dependency_report_names_pg_graphql_boundary() {
        let plan = canonical_graphql_plan();
        let report = graphql_runtime_dependency_report(&plan, |name| match name {
            GRAPHQL_DATABASE_URL_ENV => {
                Some("postgresql://postgres@127.0.0.1/postgres".to_string())
            }
            GRAPHQL_JWT_SECRET_ENV => Some("01234567890123456789012345678901".to_string()),
            _ => None,
        })
        .expect("runtime dependencies");

        assert_eq!(report.database_url_env, GRAPHQL_DATABASE_URL_ENV);
        assert_eq!(report.jwt_secret_env, GRAPHQL_JWT_SECRET_ENV);
        assert_eq!(report.endpoint_path, "/graphql/v1");
        assert!(report.pg_graphql_extension_required);
    }

    #[test]
    fn resolve_renders_set_claims_and_resolve_sql() {
        let plan = canonical_graphql_plan();
        let mut handler = GraphqlHandler::new(plan).expect("handler");

        let response = handler
            .resolve(&canonical_graphql_request())
            .expect("resolve");

        let set_claims = response
            .execution_plan
            .set_jwt_claims_sql
            .as_deref()
            .expect("set_claims");
        assert!(set_claims.starts_with("select set_config('request.jwt.claims',"));
        assert!(response
            .execution_plan
            .resolve_sql
            .contains("graphql.resolve"));
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

        let request = GraphqlRequest::new("query { __schema { types { name } } }")
            .with_jwt_claims_json("{\"tenant_id\":\"tenant-a\"}");

        assert_eq!(
            handler.resolve(&request),
            Err(GraphqlSidecarError::IntrospectionDisabled)
        );
    }

    #[test]
    fn resolve_requires_tenant_claim_when_rls_required() {
        let plan = canonical_graphql_plan();
        let mut handler = GraphqlHandler::new(plan).expect("handler");

        let request = GraphqlRequest::new("query { orderCollection { edges { node { id } } } }");

        assert_eq!(
            handler.resolve(&request),
            Err(GraphqlSidecarError::TenantClaimMissing)
        );
    }

    #[test]
    fn register_subscription_records_notify_channels() {
        let plan = canonical_graphql_plan();
        let mut handler = GraphqlHandler::new(plan).expect("handler");

        let subscription = handler
            .register_subscription(&canonical_graphql_subscription_request())
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
    fn http_front_door_serves_graphql_query_and_graphiql() {
        let body = r#"{"query":"query { orderCollection { edges { node { id } } } }","jwt_claims":"{\"tenant_id\":\"tenant-a\"}"}"#;
        let request = format!(
            "POST /graphql/v1 HTTP/1.1\r\ncontent-type: application/json\r\ncontent-length: {}\r\n\r\n{}",
            body.len(),
            body,
        );
        let response = handle_graphql_sidecar_http_bytes(request.as_bytes()).expect("post");
        assert_eq!(response.status_code, 200);
        assert!(response.body.contains("\"namespace\":\"public_api\""));

        let get =
            handle_graphql_sidecar_http_bytes(b"GET /graphql HTTP/1.1\r\nHost: local\r\n\r\n")
                .expect("get");
        assert_eq!(get.status_code, 200);
        assert!(get.body.contains("ai-blaise GraphQL"));
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
    fn websocket_boundary_registers_subscription() {
        let body = r#"{"query":"subscription { orderInserted { id total } }","jwt_claims":"{\"tenant_id\":\"tenant-a\"}"}"#;
        let request = format!(
            "POST /graphql/ws HTTP/1.1\r\ncontent-type: application/json\r\ncontent-length: {}\r\n\r\n{}",
            body.len(),
            body,
        );
        let response = handle_graphql_sidecar_http_bytes(request.as_bytes()).expect("ws boundary");

        assert_eq!(response.status_code, 200);
        assert!(response.body.contains("\"transport\":\"websocket\""));
        assert!(response
            .body
            .contains("\"subscription_field\":\"orderInserted\""));
        assert!(response.body.contains("public_api.public.orders"));
    }

    #[test]
    fn extract_string_field_parses_nested_quotes_after_double_quote() {
        let claims = "{\"tenant_id\":\"tenant-a\",\"role\":\"web_anon\"}";
        assert_eq!(
            extract_string_field(claims, "tenant_id"),
            Some("tenant-a".to_string())
        );
        assert_eq!(
            extract_string_field(claims, "role"),
            Some("web_anon".to_string())
        );
        assert_eq!(extract_string_field(claims, "missing"), None);
    }
}
