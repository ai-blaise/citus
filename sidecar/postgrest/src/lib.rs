//! PostgREST sidecar contracts.

// FEATURE: API1
// FEATURE: API2
// FEATURE: API5
// FEATURE: API6

use ai_blaise_citus_sidecar_shared::{
    listen_addr_from_env, HttpProbeResponse, SidecarRuntime, SidecarRuntimeError,
};
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
    InvalidRuntimeDependency(String),
    InvalidShardCount,
    MalformedHttpRequest,
    MissingRequiredField(&'static str),
    MissingRuntimeDependency(String),
    Runtime(String),
    RuntimeDependencyUnavailable(String),
    RouteNotFound(String),
    RlsRequired,
}

impl fmt::Display for PostgrestSidecarError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidIdentifier(field) => write!(formatter, "{field} must be a SQL identifier"),
            Self::InvalidPath(field) => write!(formatter, "{field} must start with /"),
            Self::InvalidRuntimeDependency(detail) => {
                write!(formatter, "invalid runtime dependency: {detail}")
            }
            Self::InvalidShardCount => write!(formatter, "shard_count must be greater than zero"),
            Self::MalformedHttpRequest => {
                write!(formatter, "malformed PostgREST sidecar HTTP request")
            }
            Self::MissingRequiredField(field) => write!(formatter, "{field} must not be empty"),
            Self::MissingRuntimeDependency(name) => {
                write!(formatter, "missing runtime dependency: {name}")
            }
            Self::Runtime(error) => write!(formatter, "{error}"),
            Self::RuntimeDependencyUnavailable(detail) => {
                write!(formatter, "runtime dependency unavailable: {detail}")
            }
            Self::RouteNotFound(path) => write!(formatter, "no route configured for {path}"),
            Self::RlsRequired => write!(formatter, "RLS must be required for auto-API routes"),
        }
    }
}

impl Error for PostgrestSidecarError {}

impl From<SidecarRuntimeError> for PostgrestSidecarError {
    fn from(error: SidecarRuntimeError) -> Self {
        Self::Runtime(error.to_string())
    }
}

impl From<std::io::Error> for PostgrestSidecarError {
    fn from(error: std::io::Error) -> Self {
        Self::Runtime(error.to_string())
    }
}

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

pub fn canonical_postgrest_plan() -> PostgrestSidecarPlan {
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

pub fn canonical_postgrest_execution_plan() -> Result<PostgrestSidecarPlan, PostgrestSidecarError> {
    let plan = canonical_postgrest_plan();
    plan.validate()?;
    Ok(plan)
}

const POSTGREST_BINARY_ENV: &str = "AI_BLAISE_POSTGREST_BINARY";
const POSTGREST_PORT_ENV: &str = "AI_BLAISE_POSTGREST_PORT";
const POSTGREST_CONFIG_PATH_ENV: &str = "AI_BLAISE_POSTGREST_CONFIG_PATH";
const POSTGREST_UPSTREAM_ENV: &str = "AI_BLAISE_POSTGREST_UPSTREAM";
const POSTGREST_EXIT_ON_STDIN_EOF_ENV: &str = "AI_BLAISE_POSTGREST_EXIT_ON_STDIN_EOF";
const MIN_JWT_SECRET_BYTES: usize = 32;

// =============================================================================
// Runtime: process supervisor + HTTP front door
// =============================================================================

/// Connection inputs the operator hands the supervisor. They are kept opaque to
/// the rest of the codebase so secret material does not leak into reports.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PostgrestSupervisorConfig {
    pub db_uri_secret_ref: String,
    pub jwt_secret_ref: String,
    pub anon_role: String,
    pub server_port: u16,
    pub log_level: PostgrestLogLevel,
    pub binary_path: String,
}

impl PostgrestSupervisorConfig {
    pub fn validate(&self) -> Result<(), PostgrestSidecarError> {
        validate_required("supervisor.db_uri_secret_ref", &self.db_uri_secret_ref)?;
        validate_required("supervisor.jwt_secret_ref", &self.jwt_secret_ref)?;
        validate_identifier("supervisor.anon_role", &self.anon_role)?;
        if self.server_port == 0 {
            return Err(PostgrestSidecarError::MissingRequiredField(
                "supervisor.server_port",
            ));
        }
        validate_required("supervisor.binary_path", &self.binary_path)
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum PostgrestLogLevel {
    Crit,
    Error,
    Warn,
    Info,
}

impl PostgrestLogLevel {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Crit => "crit",
            Self::Error => "error",
            Self::Warn => "warn",
            Self::Info => "info",
        }
    }
}

/// Renders the `postgrest.conf` file the supervisor writes to disk before
/// launching the upstream binary. The `db-uri` and `jwt-secret` values are kept
/// as `env:<NAME>` references so the supervisor reads them from secrets at
/// process-launch time rather than baking the secret into the configuration.
pub fn render_postgrest_conf(
    plan: &PostgrestSidecarPlan,
    config: &PostgrestSupervisorConfig,
) -> Result<String, PostgrestSidecarError> {
    plan.validate()?;
    config.validate()?;

    let mut conf = String::new();
    conf.push_str("# Generated by ai-blaise/citus sidecar/postgrest.\n");
    conf.push_str("# Do not edit by hand. Re-emit through the supervisor.\n");
    conf.push_str(&format!("db-uri = \"env:{}\"\n", config.db_uri_secret_ref));
    conf.push_str(&format!("db-schemas = \"{}\"\n", plan.schemas.join(",")));
    conf.push_str(&format!("db-anon-role = \"{}\"\n", config.anon_role));
    conf.push_str(&format!("jwt-secret = \"env:{}\"\n", config.jwt_secret_ref));
    conf.push_str(&format!(
        "jwt-aud = \"{}\"\n",
        plan.openapi.title.replace('"', "")
    ));
    conf.push_str("jwt-role-claim-key = \".role\"\n");
    conf.push_str(&format!(
        "# tenant-claim consumed by PostgreSQL RLS policies: {}\n",
        plan.auth.tenant_claim
    ));
    conf.push_str(&format!("server-port = {}\n", config.server_port));
    conf.push_str("server-host = \"127.0.0.1\"\n");
    conf.push_str(&format!("log-level = \"{}\"\n", config.log_level.as_str()));
    conf.push_str("db-prepared-statements = true\n");
    conf.push_str("db-tx-end = \"commit-allow-override\"\n");
    conf.push_str(&format!(
        "openapi-mode = \"follow-privileges\"\n# openapi-path is enforced at the sidecar front door: {}\n",
        plan.openapi.path
    ));
    Ok(conf)
}

/// Renders an OpenAPI 3.0 document describing the canonical PostgREST routes.
/// The shape mirrors what upstream PostgREST emits at `/openapi.json` so that
/// the sidecar can serve a deterministic spec even before the upstream binary
/// becomes ready (the supervisor swaps to the upstream document once available).
pub fn render_openapi_document(
    plan: &PostgrestSidecarPlan,
) -> Result<String, PostgrestSidecarError> {
    plan.validate()?;

    let mut paths = String::new();
    for (index, route) in plan.routes.iter().enumerate() {
        let separator = if index == 0 { "" } else { "," };
        let path = format!("/{}", route.table);
        let methods = route
            .methods
            .iter()
            .map(|method| format!(
                "\"{}\":{{\"tags\":[\"{}.{}\"],\"summary\":\"{} {}.{}\",\"responses\":{{\"200\":{{\"description\":\"OK\"}}}}}}",
                method_lower(method),
                route.schema,
                route.table,
                method_upper(method),
                route.schema,
                route.table,
            ))
            .collect::<Vec<_>>()
            .join(",");
        paths.push_str(&format!("{separator}\"{path}\":{{{methods}}}"));
    }

    let schemas = plan
        .schemas
        .iter()
        .map(|schema| format!("\"{schema}\""))
        .collect::<Vec<_>>()
        .join(",");

    let document = format!(
        "{{\"openapi\":\"3.0.0\",\"info\":{{\"title\":\"{}\",\"version\":\"{}\"}},\"servers\":[{{\"url\":\"/\"}}],\"x-ai-blaise\":{{\"schemas\":[{}],\"tenant_claim\":\"{}\",\"rls_required\":true}},\"paths\":{{{}}}}}",
        plan.openapi.title,
        plan.openapi.version,
        schemas,
        plan.auth.tenant_claim,
        paths,
    );
    Ok(document)
}

fn method_lower(method: &RestMethod) -> &'static str {
    match method {
        RestMethod::Get => "get",
        RestMethod::Post => "post",
        RestMethod::Patch => "patch",
        RestMethod::Delete => "delete",
    }
}

fn method_upper(method: &RestMethod) -> &'static str {
    match method {
        RestMethod::Get => "GET",
        RestMethod::Post => "POST",
        RestMethod::Patch => "PATCH",
        RestMethod::Delete => "DELETE",
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum SupervisorState {
    Pending,
    Launched,
    CrashedAndRestarted,
    Drained,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PostgrestLaunchPlan {
    pub binary_path: String,
    pub config_path: String,
    pub args: Vec<String>,
    pub env: Vec<(String, String)>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PostgrestSupervisorState {
    pub state: SupervisorState,
    pub launches: u32,
    pub restarts: u32,
    pub config_bytes: usize,
    pub openapi_bytes: usize,
}

/// Stateful supervisor for the upstream PostgREST child process. Tests can
/// exercise the lifecycle deterministically, while production callers use
/// `spawn_child_at` to write the generated config and launch the configured
/// PostgREST binary.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PostgrestSupervisor {
    plan: PostgrestSidecarPlan,
    config: PostgrestSupervisorConfig,
    launch: PostgrestLaunchPlan,
    conf_text: String,
    openapi_text: String,
    state: SupervisorState,
    launches: u32,
    restarts: u32,
}

impl PostgrestSupervisor {
    pub fn new(
        plan: PostgrestSidecarPlan,
        config: PostgrestSupervisorConfig,
    ) -> Result<Self, PostgrestSidecarError> {
        plan.validate()?;
        config.validate()?;
        let conf_text = render_postgrest_conf(&plan, &config)?;
        let openapi_text = render_openapi_document(&plan)?;
        let launch = PostgrestLaunchPlan {
            binary_path: config.binary_path.clone(),
            config_path: "/etc/postgrest/postgrest.conf".to_string(),
            args: vec!["/etc/postgrest/postgrest.conf".to_string()],
            env: vec![
                (
                    "PGRST_DB_URI".to_string(),
                    format!("env:{}", config.db_uri_secret_ref),
                ),
                (
                    "PGRST_JWT_SECRET".to_string(),
                    format!("env:{}", config.jwt_secret_ref),
                ),
                ("PGRST_DB_SCHEMAS".to_string(), plan.schemas.join(",")),
                ("PGRST_DB_ANON_ROLE".to_string(), config.anon_role.clone()),
                (
                    "PGRST_SERVER_PORT".to_string(),
                    config.server_port.to_string(),
                ),
                (
                    "PGRST_LOG_LEVEL".to_string(),
                    config.log_level.as_str().to_string(),
                ),
                ("PGRST_JWT_AUD".to_string(), plan.openapi.title.clone()),
                ("PGRST_JWT_ROLE_CLAIM_KEY".to_string(), ".role".to_string()),
            ],
        };
        Ok(Self {
            plan,
            config,
            launch,
            conf_text,
            openapi_text,
            state: SupervisorState::Pending,
            launches: 0,
            restarts: 0,
        })
    }

    pub fn plan(&self) -> &PostgrestSidecarPlan {
        &self.plan
    }

    pub fn config(&self) -> &PostgrestSupervisorConfig {
        &self.config
    }

    pub fn launch_plan(&self) -> &PostgrestLaunchPlan {
        &self.launch
    }

    pub fn postgrest_conf(&self) -> &str {
        &self.conf_text
    }

    pub fn openapi_document(&self) -> &str {
        &self.openapi_text
    }

    pub fn launch(&mut self) -> &PostgrestLaunchPlan {
        self.launches += 1;
        self.state = SupervisorState::Launched;
        &self.launch
    }

    pub fn resolved_launch_env(&self) -> Result<Vec<(String, String)>, PostgrestSidecarError> {
        resolve_launch_env(&self.launch.env)
    }

    pub fn write_config_at(
        &self,
        config_path: impl AsRef<std::path::Path>,
    ) -> Result<(), PostgrestSidecarError> {
        let config_path = config_path.as_ref();
        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(config_path, &self.conf_text)?;
        Ok(())
    }

    pub fn spawn_child_at(
        &mut self,
        config_path: impl AsRef<std::path::Path>,
    ) -> Result<std::process::Child, PostgrestSidecarError> {
        let config_path = config_path.as_ref();
        self.write_config_at(config_path)?;
        let mut command = std::process::Command::new(&self.launch.binary_path);
        command.arg(config_path);
        for (name, value) in self.resolved_launch_env()? {
            command.env(name, value);
        }
        let child = command.spawn()?;
        self.launches += 1;
        self.state = SupervisorState::Launched;
        Ok(child)
    }

    pub fn report_crash(&mut self) {
        self.restarts += 1;
        self.state = SupervisorState::CrashedAndRestarted;
        self.launches += 1;
    }

    pub fn drain(&mut self) {
        self.state = SupervisorState::Drained;
    }

    pub fn state(&self) -> PostgrestSupervisorState {
        PostgrestSupervisorState {
            state: self.state,
            launches: self.launches,
            restarts: self.restarts,
            config_bytes: self.conf_text.len(),
            openapi_bytes: self.openapi_text.len(),
        }
    }

    /// Resolves a REST request path of shape `/<table>` (or `/<schema>/<table>`)
    /// to the configured route entry. Used by the HTTP front door to decide
    /// whether a request matches the canonical plan before forwarding upstream.
    pub fn resolve_route(&self, path: &str) -> Result<&RestRoute, PostgrestSidecarError> {
        let trimmed = path.trim_start_matches('/');
        let (schema, table) = match trimmed.split_once('/') {
            Some((schema, rest)) => {
                let table = rest.split('?').next().unwrap_or(rest);
                (Some(schema), table)
            }
            None => {
                let table = trimmed.split('?').next().unwrap_or(trimmed);
                (None, table)
            }
        };
        self.plan
            .routes
            .iter()
            .find(|route| {
                route.table == table
                    && schema
                        .map(|requested| requested == route.schema)
                        .unwrap_or(true)
            })
            .ok_or_else(|| PostgrestSidecarError::RouteNotFound(path.to_string()))
    }
}

fn resolve_launch_env(
    env: &[(String, String)],
) -> Result<Vec<(String, String)>, PostgrestSidecarError> {
    env.iter()
        .map(|(name, value)| {
            if let Some(secret_env) = value.strip_prefix("env:") {
                let resolved = std::env::var(secret_env).map_err(|_| {
                    PostgrestSidecarError::MissingRuntimeDependency(secret_env.to_string())
                })?;
                Ok((name.clone(), resolved))
            } else {
                Ok((name.clone(), value.clone()))
            }
        })
        .collect()
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PostgrestRuntimeReport {
    pub plan: PostgrestSidecarPlan,
    pub launch: PostgrestLaunchPlan,
    pub state: PostgrestSupervisorState,
    pub openapi: String,
    pub conf: String,
}

pub fn canonical_postgrest_supervisor_config() -> PostgrestSupervisorConfig {
    PostgrestSupervisorConfig {
        db_uri_secret_ref: "POSTGREST_DB_URI".to_string(),
        jwt_secret_ref: "POSTGREST_JWT_SECRET".to_string(),
        anon_role: "web_anon".to_string(),
        server_port: 3000,
        log_level: PostgrestLogLevel::Info,
        binary_path: "/usr/local/bin/postgrest".to_string(),
    }
}

pub fn postgrest_supervisor_config_from_env(
) -> Result<PostgrestSupervisorConfig, PostgrestSidecarError> {
    let mut config = canonical_postgrest_supervisor_config();
    if let Ok(binary_path) = std::env::var(POSTGREST_BINARY_ENV) {
        config.binary_path = binary_path;
    }
    if let Ok(port) = std::env::var(POSTGREST_PORT_ENV) {
        config.server_port = port.parse::<u16>().map_err(|_| {
            PostgrestSidecarError::InvalidRuntimeDependency(format!(
                "{POSTGREST_PORT_ENV} must be a non-zero TCP port"
            ))
        })?;
        if config.server_port == 0 {
            return Err(PostgrestSidecarError::InvalidRuntimeDependency(format!(
                "{POSTGREST_PORT_ENV} must be a non-zero TCP port"
            )));
        }
    }
    config.validate()?;
    Ok(config)
}

pub fn postgrest_config_path_from_env() -> String {
    std::env::var(POSTGREST_CONFIG_PATH_ENV)
        .ok()
        .filter(|path| !path.trim().is_empty())
        .unwrap_or_else(|| "/etc/postgrest/postgrest.conf".to_string())
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PostgrestRuntimeDependencyReport {
    pub db_uri_env: String,
    pub jwt_secret_env: String,
    pub binary_path: String,
    pub config_path: String,
    pub schemas: Vec<String>,
    pub route_count: usize,
}

pub fn postgrest_runtime_dependency_report_from_env(
) -> Result<PostgrestRuntimeDependencyReport, PostgrestSidecarError> {
    let plan = canonical_postgrest_execution_plan()?;
    let config = postgrest_supervisor_config_from_env()?;
    postgrest_runtime_dependency_report(&plan, &config, |name| std::env::var(name).ok())
}

pub fn postgrest_runtime_dependency_report<F>(
    plan: &PostgrestSidecarPlan,
    config: &PostgrestSupervisorConfig,
    lookup: F,
) -> Result<PostgrestRuntimeDependencyReport, PostgrestSidecarError>
where
    F: Fn(&str) -> Option<String>,
{
    plan.validate()?;
    config.validate()?;
    let db_uri = require_runtime_env(&lookup, &config.db_uri_secret_ref)?;
    validate_postgres_url(&config.db_uri_secret_ref, &db_uri)?;
    let jwt_secret = require_runtime_env(&lookup, &config.jwt_secret_ref)?;
    validate_jwt_secret(&config.jwt_secret_ref, &jwt_secret)?;
    let binary_path = std::path::Path::new(&config.binary_path);
    if !binary_path.is_file() {
        return Err(PostgrestSidecarError::RuntimeDependencyUnavailable(
            format!(
                "PostgREST binary is not available at {}",
                config.binary_path
            ),
        ));
    }

    let supervisor = PostgrestSupervisor::new(plan.clone(), config.clone())?;
    Ok(PostgrestRuntimeDependencyReport {
        db_uri_env: config.db_uri_secret_ref.clone(),
        jwt_secret_env: config.jwt_secret_ref.clone(),
        binary_path: config.binary_path.clone(),
        config_path: supervisor.launch_plan().config_path.clone(),
        schemas: plan.schemas.clone(),
        route_count: plan.routes.len(),
    })
}

fn require_runtime_env<F>(lookup: &F, name: &str) -> Result<String, PostgrestSidecarError>
where
    F: Fn(&str) -> Option<String>,
{
    lookup(name)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| PostgrestSidecarError::MissingRuntimeDependency(name.to_string()))
}

fn validate_postgres_url(field: &str, value: &str) -> Result<(), PostgrestSidecarError> {
    if value.starts_with("postgres://") || value.starts_with("postgresql://") {
        Ok(())
    } else {
        Err(PostgrestSidecarError::InvalidRuntimeDependency(format!(
            "{field} must be a PostgreSQL URL"
        )))
    }
}

fn validate_jwt_secret(field: &str, value: &str) -> Result<(), PostgrestSidecarError> {
    if value.len() >= MIN_JWT_SECRET_BYTES {
        Ok(())
    } else {
        Err(PostgrestSidecarError::InvalidRuntimeDependency(format!(
            "{field} must be at least {MIN_JWT_SECRET_BYTES} bytes"
        )))
    }
}

pub fn canonical_postgrest_runtime_report() -> Result<PostgrestRuntimeReport, PostgrestSidecarError>
{
    let plan = canonical_postgrest_plan();
    let config = canonical_postgrest_supervisor_config();
    let mut supervisor = PostgrestSupervisor::new(plan.clone(), config)?;
    let launch = supervisor.launch().clone();
    Ok(PostgrestRuntimeReport {
        plan,
        launch,
        state: supervisor.state(),
        openapi: supervisor.openapi_document().to_string(),
        conf: supervisor.postgrest_conf().to_string(),
    })
}

pub fn run_supervised_postgrest_until_stopped() -> Result<(), PostgrestSidecarError> {
    let plan = canonical_postgrest_execution_plan()?;
    let config = postgrest_supervisor_config_from_env()?;
    postgrest_runtime_dependency_report(&plan, &config, |name| std::env::var(name).ok())?;
    let config_path = postgrest_config_path_from_env();
    let mut supervisor = PostgrestSupervisor::new(plan, config)?;
    let mut child = supervisor.spawn_child_at(&config_path)?;
    eprintln!(
        "ai-blaise postgrest supervisor launched child pid {} with config {}",
        child.id(),
        config_path
    );

    let exit_on_stdin_eof = std::env::var(POSTGREST_EXIT_ON_STDIN_EOF_ENV)
        .ok()
        .as_deref()
        == Some("1");
    let stdin_rx = if exit_on_stdin_eof {
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            use std::io::Read as _;
            let mut buffer = Vec::new();
            let _ = std::io::stdin().read_to_end(&mut buffer);
            let _ = tx.send(());
        });
        Some(rx)
    } else {
        None
    };

    loop {
        if let Some(status) = child.try_wait()? {
            if status.success() {
                return Ok(());
            }
            return Err(PostgrestSidecarError::Runtime(format!(
                "PostgREST child exited with {status}"
            )));
        }
        if stdin_rx
            .as_ref()
            .map(|rx| rx.try_recv().is_ok())
            .unwrap_or(false)
        {
            child.kill()?;
            let _ = child.wait();
            return Ok(());
        }
        std::thread::sleep(std::time::Duration::from_millis(250));
    }
}

// =============================================================================
// HTTP front door
// =============================================================================

#[derive(Debug, Clone, Eq, PartialEq)]
struct ParsedPostgrestHttpRequest {
    method: String,
    path: String,
    headers: Vec<(String, String)>,
    body: String,
}

pub fn handle_postgrest_sidecar_http_bytes(
    request: &[u8],
) -> Result<HttpProbeResponse, PostgrestSidecarError> {
    let mut runtime = SidecarRuntime::ready("postgrest");
    handle_postgrest_sidecar_http_request(request, &mut runtime)
}

fn handle_postgrest_sidecar_http_request(
    request: &[u8],
    runtime: &mut SidecarRuntime,
) -> Result<HttpProbeResponse, PostgrestSidecarError> {
    let request =
        std::str::from_utf8(request).map_err(|_| PostgrestSidecarError::MalformedHttpRequest)?;
    let request = parse_http_request(request)?;
    let method = request.method.as_str();
    let path = request.path.as_str();

    let plan = canonical_postgrest_execution_plan()?;
    let config = canonical_postgrest_supervisor_config();
    let supervisor = PostgrestSupervisor::new(plan.clone(), config)?;

    if method == "GET" && path == "/openapi.json" {
        return Ok(HttpProbeResponse::new(
            200,
            "application/openapi+json",
            format!("{}\n", supervisor.openapi_document()),
        ));
    }
    if method == "GET" && path == "/postgrest.conf" {
        return Ok(HttpProbeResponse::new(
            200,
            "text/plain; charset=utf-8",
            supervisor.postgrest_conf().to_string(),
        ));
    }
    if path.starts_with("/api/") {
        return match supervisor.resolve_route(&path["/api".len()..]) {
            Ok(route) => match rest_method_from_http(method) {
                Some(rest_method) if route.methods.contains(&rest_method) => {
                    if let Some(upstream) = postgrest_upstream_from_env()? {
                        proxy_postgrest_request(&upstream, &request, route, rest_method)
                    } else {
                        Ok(HttpProbeResponse::new(
                            200,
                            "application/json",
                            render_route_payload(route, rest_method),
                        ))
                    }
                }
                _ => Ok(HttpProbeResponse::new(
                    405,
                    "application/json",
                    "{\"error\":\"method not allowed for route\"}\n".to_string(),
                )),
            },
            Err(error) => Ok(HttpProbeResponse::new(
                404,
                "application/json",
                format!("{{\"error\":\"{}\"}}\n", escape_json(&error.to_string())),
            )),
        };
    }

    Ok(runtime.handle_http_bytes(request_to_runtime_bytes(&request).as_bytes())?)
}

fn request_to_runtime_bytes(request: &ParsedPostgrestHttpRequest) -> String {
    format!(
        "{} {} HTTP/1.1\r\nHost: sidecar\r\n\r\n{}",
        request.method, request.path, request.body
    )
}

fn rest_method_from_http(method: &str) -> Option<RestMethod> {
    match method {
        "GET" => Some(RestMethod::Get),
        "POST" => Some(RestMethod::Post),
        "PATCH" => Some(RestMethod::Patch),
        "DELETE" => Some(RestMethod::Delete),
        _ => None,
    }
}

fn postgrest_upstream_from_env() -> Result<Option<String>, PostgrestSidecarError> {
    let Ok(raw) = std::env::var(POSTGREST_UPSTREAM_ENV) else {
        return Ok(None);
    };
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    let upstream = if let Some(rest) = trimmed.strip_prefix("http://") {
        rest
    } else if trimmed.contains("://") {
        return Err(PostgrestSidecarError::InvalidRuntimeDependency(format!(
            "{POSTGREST_UPSTREAM_ENV} only supports http:// or host:port loopback upstreams"
        )));
    } else {
        trimmed
    };
    if upstream.contains('/') || !upstream.contains(':') {
        return Err(PostgrestSidecarError::InvalidRuntimeDependency(format!(
            "{POSTGREST_UPSTREAM_ENV} must be host:port without a path"
        )));
    }
    let Some((host, port)) = upstream.rsplit_once(':') else {
        return Err(PostgrestSidecarError::InvalidRuntimeDependency(format!(
            "{POSTGREST_UPSTREAM_ENV} must be host:port without a path"
        )));
    };
    let host = host.trim_matches(|ch| ch == '[' || ch == ']');
    if !matches!(host, "127.0.0.1" | "localhost" | "::1") {
        return Err(PostgrestSidecarError::InvalidRuntimeDependency(format!(
            "{POSTGREST_UPSTREAM_ENV} must point at a loopback PostgREST process"
        )));
    }
    let port = port.parse::<u16>().map_err(|_| {
        PostgrestSidecarError::InvalidRuntimeDependency(format!(
            "{POSTGREST_UPSTREAM_ENV} must use a non-zero TCP port"
        ))
    })?;
    if port == 0 {
        return Err(PostgrestSidecarError::InvalidRuntimeDependency(format!(
            "{POSTGREST_UPSTREAM_ENV} must use a non-zero TCP port"
        )));
    }
    Ok(Some(upstream.to_string()))
}

fn proxy_postgrest_request(
    upstream: &str,
    request: &ParsedPostgrestHttpRequest,
    route: &RestRoute,
    method: RestMethod,
) -> Result<HttpProbeResponse, PostgrestSidecarError> {
    let target = proxy_target(route, &request.path);
    let mut stream = std::net::TcpStream::connect(upstream)?;
    stream.set_read_timeout(Some(std::time::Duration::from_secs(15)))?;
    stream.set_write_timeout(Some(std::time::Duration::from_secs(15)))?;

    let mut outbound = format!(
        "{} {} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n",
        request.method, target.path, upstream
    );
    for (name, value) in request
        .headers
        .iter()
        .filter(|(name, _)| passthrough_header(name))
    {
        if !profile_header(name) && !name.eq_ignore_ascii_case("content-length") {
            outbound.push_str(&format!("{}: {}\r\n", name, value));
        }
    }
    match method {
        RestMethod::Get => outbound.push_str(&format!("Accept-Profile: {}\r\n", target.schema)),
        RestMethod::Post | RestMethod::Patch | RestMethod::Delete => {
            outbound.push_str(&format!("Content-Profile: {}\r\n", target.schema));
            outbound.push_str(&format!("Accept-Profile: {}\r\n", target.schema));
        }
    }
    if !request.body.is_empty() && !has_header(&request.headers, "content-type") {
        outbound.push_str("Content-Type: application/json\r\n");
    }
    outbound.push_str(&format!(
        "Content-Length: {}\r\n\r\n{}",
        request.body.len(),
        request.body
    ));

    use std::io::{Read as _, Write as _};
    stream.write_all(outbound.as_bytes())?;
    let mut response = Vec::new();
    stream.read_to_end(&mut response)?;
    parse_upstream_response(&response)
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct ProxyTarget {
    schema: String,
    path: String,
}

fn proxy_target(route: &RestRoute, request_path: &str) -> ProxyTarget {
    let after_api = request_path.strip_prefix("/api").unwrap_or(request_path);
    let trimmed = after_api.trim_start_matches('/');
    let (path_without_query, query) = split_query(trimmed);
    let explicit_schema = path_without_query
        .split_once('/')
        .map(|(schema, _)| schema.to_string());
    let schema = explicit_schema
        .clone()
        .or_else(|| {
            route.distributed_view.as_ref().and_then(|binding| {
                binding
                    .view_name
                    .split_once('.')
                    .map(|(schema, _)| schema.to_string())
            })
        })
        .unwrap_or_else(|| route.schema.clone());
    let table = explicit_schema
        .and_then(|_| {
            path_without_query
                .split_once('/')
                .map(|(_, table)| table.to_string())
        })
        .or_else(|| {
            route.distributed_view.as_ref().and_then(|binding| {
                binding
                    .view_name
                    .split_once('.')
                    .map(|(_, table)| table.to_string())
            })
        })
        .unwrap_or_else(|| route.table.clone());
    let mut path = format!("/{table}");
    if let Some(query) = query {
        path.push('?');
        path.push_str(query);
    }
    ProxyTarget { schema, path }
}

fn split_query(value: &str) -> (&str, Option<&str>) {
    match value.split_once('?') {
        Some((path, query)) => (path, Some(query)),
        None => (value, None),
    }
}

fn passthrough_header(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        "authorization" | "accept" | "prefer" | "content-type"
    )
}

fn profile_header(name: &str) -> bool {
    name.eq_ignore_ascii_case("accept-profile") || name.eq_ignore_ascii_case("content-profile")
}

fn has_header(headers: &[(String, String)], needle: &str) -> bool {
    headers
        .iter()
        .any(|(name, _)| name.eq_ignore_ascii_case(needle))
}

fn parse_upstream_response(response: &[u8]) -> Result<HttpProbeResponse, PostgrestSidecarError> {
    let (body_start, head_bytes) =
        split_http_head(response).ok_or(PostgrestSidecarError::MalformedHttpRequest)?;
    let head =
        std::str::from_utf8(head_bytes).map_err(|_| PostgrestSidecarError::MalformedHttpRequest)?;
    let mut lines = head.lines();
    let status_line = lines
        .next()
        .ok_or(PostgrestSidecarError::MalformedHttpRequest)?;
    let status_code = status_line
        .split_whitespace()
        .nth(1)
        .ok_or(PostgrestSidecarError::MalformedHttpRequest)?
        .parse::<u16>()
        .map_err(|_| PostgrestSidecarError::MalformedHttpRequest)?;
    let mut content_type = "application/octet-stream".to_string();
    let mut chunked = false;
    for line in lines {
        if let Some((name, value)) = line.split_once(':') {
            if name.eq_ignore_ascii_case("content-type") {
                content_type = value.trim().to_string();
            }
            if name.eq_ignore_ascii_case("transfer-encoding")
                && value.to_ascii_lowercase().contains("chunked")
            {
                chunked = true;
            }
        }
    }
    let body_bytes = if chunked {
        decode_chunked_body(&response[body_start..])?
    } else {
        response[body_start..].to_vec()
    };
    let body = String::from_utf8_lossy(&body_bytes).into_owned();
    Ok(HttpProbeResponse::new(status_code, content_type, body))
}

fn decode_chunked_body(mut body: &[u8]) -> Result<Vec<u8>, PostgrestSidecarError> {
    let mut decoded = Vec::new();
    loop {
        let Some(line_end) = find_bytes(body, b"\r\n") else {
            return Err(PostgrestSidecarError::MalformedHttpRequest);
        };
        let size_text = std::str::from_utf8(&body[..line_end])
            .map_err(|_| PostgrestSidecarError::MalformedHttpRequest)?;
        let size_hex = size_text.split(';').next().unwrap_or(size_text).trim();
        let size = usize::from_str_radix(size_hex, 16)
            .map_err(|_| PostgrestSidecarError::MalformedHttpRequest)?;
        body = &body[line_end + 2..];
        if size == 0 {
            return Ok(decoded);
        }
        if body.len() < size + 2 {
            return Err(PostgrestSidecarError::MalformedHttpRequest);
        }
        decoded.extend_from_slice(&body[..size]);
        body = &body[size + 2..];
    }
}

fn render_route_payload(route: &RestRoute, method: RestMethod) -> String {
    let methods = route
        .methods
        .iter()
        .map(|method| format!("\"{}\"", method_upper(method)))
        .collect::<Vec<_>>()
        .join(",");
    let view = route
        .distributed_view
        .as_ref()
        .map(|binding| {
            format!(
                "{{\"view\":\"{}\",\"distribution_column\":\"{}\",\"shard_count\":{}}}",
                binding.view_name, binding.distribution_column, binding.shard_count,
            )
        })
        .unwrap_or_else(|| "null".to_string());
    format!(
        "{{\"schema\":\"{}\",\"table\":\"{}\",\"method\":\"{}\",\"allowed_methods\":[{}],\"distributed_view\":{}}}\n",
        route.schema,
        route.table,
        method_upper(&method),
        methods,
        view,
    )
}

pub fn serve_postgrest_sidecar_http_forever(
    default_addr: &str,
) -> Result<(), PostgrestSidecarError> {
    use std::io::Write;
    use std::net::TcpListener;

    canonical_postgrest_execution_plan()?;
    let mut runtime = SidecarRuntime::ready("postgrest");
    let listen_addr = listen_addr_from_env(default_addr)?;
    let listener = TcpListener::bind(&listen_addr)?;
    eprintln!("ai-blaise postgrest sidecar listening on {listen_addr}");

    for stream in listener.incoming() {
        let mut stream = stream?;
        let request = read_http_request(&mut stream)?;
        let response = handle_postgrest_sidecar_http_request(&request, &mut runtime)
            .unwrap_or_else(|error| {
                HttpProbeResponse::new(
                    400,
                    "application/json",
                    format!("{{\"error\":\"{}\"}}\n", escape_json(&error.to_string())),
                )
            });
        stream.write_all(response.to_http_string().as_bytes())?;
    }

    Ok(())
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

fn parse_http_request(request: &str) -> Result<ParsedPostgrestHttpRequest, PostgrestSidecarError> {
    let (head, body) = request
        .split_once("\r\n\r\n")
        .or_else(|| request.split_once("\n\n"))
        .ok_or(PostgrestSidecarError::MalformedHttpRequest)?;
    let mut lines = head.lines();
    let request_line = lines
        .next()
        .ok_or(PostgrestSidecarError::MalformedHttpRequest)?;
    let mut parts = request_line.split_whitespace();
    let method = parts
        .next()
        .ok_or(PostgrestSidecarError::MalformedHttpRequest)?;
    let path = parts
        .next()
        .ok_or(PostgrestSidecarError::MalformedHttpRequest)?;
    if !path.starts_with('/') {
        return Err(PostgrestSidecarError::MalformedHttpRequest);
    }
    let mut headers = Vec::new();
    for line in lines {
        if let Some((name, value)) = line.split_once(':') {
            headers.push((name.trim().to_string(), value.trim().to_string()));
        }
    }
    Ok(ParsedPostgrestHttpRequest {
        method: method.to_string(),
        path: path.to_string(),
        headers,
        body: body.to_string(),
    })
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
    fn postgrest_plan_validates_distributed_route_and_openapi() {
        assert_eq!(canonical_postgrest_plan().validate(), Ok(()));
    }

    #[test]
    fn canonical_postgrest_execution_plan_is_deterministic() {
        let plan = canonical_postgrest_execution_plan().expect("canonical plan");

        assert_eq!(plan.openapi.path, "/openapi.json");
        assert_eq!(plan.routes[0].table, "orders");
        assert_eq!(plan.auth.tenant_claim, "tenant_id");
    }

    #[test]
    fn distributed_view_requires_shard_count() {
        let mut plan = canonical_postgrest_plan();
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
        let mut plan = canonical_postgrest_plan();
        plan.auth.rls_required = false;

        assert_eq!(plan.validate(), Err(PostgrestSidecarError::RlsRequired));
    }

    #[test]
    fn openapi_path_must_be_absolute() {
        let mut plan = canonical_postgrest_plan();
        plan.openapi.path = "openapi.json".to_string();

        assert_eq!(
            plan.validate(),
            Err(PostgrestSidecarError::InvalidPath("openapi.path"))
        );
    }

    #[test]
    fn supervisor_config_validates_required_fields() {
        let mut config = canonical_postgrest_supervisor_config();
        config.anon_role = String::new();
        assert_eq!(
            config.validate(),
            Err(PostgrestSidecarError::MissingRequiredField(
                "supervisor.anon_role",
            ))
        );
    }

    #[test]
    fn runtime_dependency_report_fails_closed_without_db_uri() {
        let plan = canonical_postgrest_plan();
        let mut config = canonical_postgrest_supervisor_config();
        config.binary_path = std::env::current_exe()
            .expect("current exe")
            .to_string_lossy()
            .into_owned();

        assert_eq!(
            postgrest_runtime_dependency_report(&plan, &config, |_| None),
            Err(PostgrestSidecarError::MissingRuntimeDependency(
                "POSTGREST_DB_URI".to_string()
            ))
        );
    }

    #[test]
    fn runtime_dependency_report_rejects_invalid_db_uri() {
        let plan = canonical_postgrest_plan();
        let mut config = canonical_postgrest_supervisor_config();
        config.binary_path = std::env::current_exe()
            .expect("current exe")
            .to_string_lossy()
            .into_owned();

        let err = postgrest_runtime_dependency_report(&plan, &config, |name| match name {
            "POSTGREST_DB_URI" => Some("http://postgres".to_string()),
            "POSTGREST_JWT_SECRET" => Some("01234567890123456789012345678901".to_string()),
            _ => None,
        })
        .expect_err("invalid db uri");
        assert!(err.to_string().contains("must be a PostgreSQL URL"));
    }

    #[test]
    fn runtime_dependency_report_uses_binary_and_secret_env_names() {
        let plan = canonical_postgrest_plan();
        let mut config = canonical_postgrest_supervisor_config();
        config.binary_path = std::env::current_exe()
            .expect("current exe")
            .to_string_lossy()
            .into_owned();

        let report = postgrest_runtime_dependency_report(&plan, &config, |name| match name {
            "POSTGREST_DB_URI" => Some("postgresql://postgres@127.0.0.1/postgres".to_string()),
            "POSTGREST_JWT_SECRET" => Some("01234567890123456789012345678901".to_string()),
            _ => None,
        })
        .expect("runtime dependencies");

        assert_eq!(report.db_uri_env, "POSTGREST_DB_URI");
        assert_eq!(report.jwt_secret_env, "POSTGREST_JWT_SECRET");
        assert_eq!(report.route_count, 1);
        assert_eq!(
            report.schemas,
            vec!["public".to_string(), "api".to_string()]
        );
    }

    #[test]
    fn render_postgrest_conf_uses_secret_refs_and_schemas() {
        let plan = canonical_postgrest_plan();
        let config = canonical_postgrest_supervisor_config();
        let conf = render_postgrest_conf(&plan, &config).expect("conf");

        assert!(conf.contains("db-uri = \"env:POSTGREST_DB_URI\""));
        assert!(conf.contains("jwt-secret = \"env:POSTGREST_JWT_SECRET\""));
        assert!(conf.contains("db-schemas = \"public,api\""));
        assert!(conf.contains("server-port = 3000"));
        assert!(conf.contains("db-anon-role = \"web_anon\""));
    }

    #[test]
    fn render_openapi_document_includes_routes_and_tenant_claim() {
        let plan = canonical_postgrest_plan();
        let document = render_openapi_document(&plan).expect("openapi");

        assert!(document.contains("\"openapi\":\"3.0.0\""));
        assert!(document.contains("\"title\":\"ai-blaise Citus API\""));
        assert!(document.contains("\"/orders\""));
        assert!(document.contains("\"public.orders\""));
        assert!(document.contains("\"tenant_claim\":\"tenant_id\""));
        assert!(document.contains("\"rls_required\":true"));
    }

    #[test]
    fn supervisor_records_launch_restart_and_drain() {
        let plan = canonical_postgrest_plan();
        let config = canonical_postgrest_supervisor_config();
        let mut supervisor = PostgrestSupervisor::new(plan, config).expect("supervisor");

        let state = supervisor.state();
        assert_eq!(state.state, SupervisorState::Pending);
        assert_eq!(state.launches, 0);

        let launch = supervisor.launch().clone();
        assert_eq!(launch.binary_path, "/usr/local/bin/postgrest");
        assert_eq!(supervisor.state().state, SupervisorState::Launched);
        assert_eq!(supervisor.state().launches, 1);

        supervisor.report_crash();
        assert_eq!(
            supervisor.state().state,
            SupervisorState::CrashedAndRestarted
        );
        assert_eq!(supervisor.state().restarts, 1);
        assert_eq!(supervisor.state().launches, 2);

        supervisor.drain();
        assert_eq!(supervisor.state().state, SupervisorState::Drained);
    }

    #[test]
    fn supervisor_writes_config_and_spawns_configured_child() {
        let plan = canonical_postgrest_plan();
        let mut config = canonical_postgrest_supervisor_config();
        config.binary_path = "/bin/true".to_string();
        let mut supervisor = PostgrestSupervisor::new(plan, config).expect("supervisor");
        let config_path = std::env::temp_dir().join(format!(
            "ai-blaise-postgrest-{}-{}.conf",
            std::process::id(),
            supervisor.state().config_bytes,
        ));

        std::env::set_var(
            "POSTGREST_DB_URI",
            "postgresql://postgres@127.0.0.1/postgres",
        );
        std::env::set_var("POSTGREST_JWT_SECRET", "01234567890123456789012345678901");
        let mut child = supervisor
            .spawn_child_at(&config_path)
            .expect("spawn child");
        let status = child.wait().expect("wait child");
        std::env::remove_var("POSTGREST_DB_URI");
        std::env::remove_var("POSTGREST_JWT_SECRET");
        let conf = std::fs::read_to_string(&config_path).expect("config written");
        let _ = std::fs::remove_file(&config_path);

        assert!(status.success());
        assert_eq!(supervisor.state().state, SupervisorState::Launched);
        assert_eq!(supervisor.state().launches, 1);
        assert!(conf.contains("db-uri = \"env:POSTGREST_DB_URI\""));
        assert!(conf.contains("jwt-role-claim-key = \".role\""));
    }

    #[test]
    fn supervisor_resolves_canonical_route() {
        let plan = canonical_postgrest_plan();
        let config = canonical_postgrest_supervisor_config();
        let supervisor = PostgrestSupervisor::new(plan, config).expect("supervisor");

        let route = supervisor.resolve_route("/orders").expect("route");
        assert_eq!(route.schema, "public");
        assert_eq!(route.table, "orders");

        let qualified = supervisor.resolve_route("/public/orders").expect("route");
        assert_eq!(qualified.table, "orders");

        assert!(matches!(
            supervisor.resolve_route("/missing"),
            Err(PostgrestSidecarError::RouteNotFound(_))
        ));
    }

    #[test]
    fn canonical_postgrest_runtime_report_is_deterministic() {
        let report = canonical_postgrest_runtime_report().expect("runtime");

        assert_eq!(report.state.launches, 1);
        assert_eq!(report.state.restarts, 0);
        assert_eq!(report.state.state, SupervisorState::Launched);
        assert!(report.openapi.contains("\"/orders\""));
        assert!(report.conf.contains("server-port = 3000"));
    }

    #[test]
    fn proxy_target_uses_distributed_view_for_unqualified_api_path() {
        let plan = canonical_postgrest_plan();
        let route = &plan.routes[0];

        let target = proxy_target(route, "/api/orders?select=id,tenant_id");

        assert_eq!(target.schema, "api");
        assert_eq!(target.path, "/orders?select=id,tenant_id");
    }

    #[test]
    fn proxy_target_respects_explicit_public_schema() {
        let plan = canonical_postgrest_plan();
        let route = &plan.routes[0];

        let target = proxy_target(route, "/api/public/orders?select=id");

        assert_eq!(target.schema, "public");
        assert_eq!(target.path, "/orders?select=id");
    }

    #[test]
    fn upstream_response_parser_decodes_chunked_postgrest_body() {
        let response = b"HTTP/1.1 201 Created\r\ncontent-type: application/json\r\ntransfer-encoding: chunked\r\n\r\nB\r\n[{\"id\":1}]\n\r\n0\r\n\r\n";

        let parsed = parse_upstream_response(response).expect("parsed response");

        assert_eq!(parsed.status_code, 201);
        assert_eq!(parsed.content_type, "application/json");
        assert_eq!(parsed.body, "[{\"id\":1}]\n");
    }

    #[test]
    fn http_front_door_serves_openapi_and_route_descriptor() {
        let response = handle_postgrest_sidecar_http_bytes(
            b"GET /openapi.json HTTP/1.1\r\nHost: local\r\n\r\n",
        )
        .expect("openapi response");
        assert_eq!(response.status_code, 200);
        assert_eq!(response.content_type, "application/openapi+json");
        assert!(response.body.contains("\"/orders\""));

        let route_response =
            handle_postgrest_sidecar_http_bytes(b"GET /api/orders HTTP/1.1\r\nHost: local\r\n\r\n")
                .expect("route response");
        assert_eq!(route_response.status_code, 200);
        assert!(route_response.body.contains("\"table\":\"orders\""));
        assert!(route_response
            .body
            .contains("\"distribution_column\":\"tenant_id\""));

        let unknown_route = handle_postgrest_sidecar_http_bytes(
            b"GET /api/missing HTTP/1.1\r\nHost: local\r\n\r\n",
        )
        .expect("unknown response");
        assert_eq!(unknown_route.status_code, 404);
    }

    #[test]
    fn http_front_door_serves_healthz_and_metrics() {
        let healthz =
            handle_postgrest_sidecar_http_bytes(b"GET /healthz HTTP/1.1\r\nHost: local\r\n\r\n")
                .expect("healthz");
        assert_eq!(healthz.status_code, 200);
        assert!(healthz.body.contains("\"component\":\"postgrest\""));

        let metrics =
            handle_postgrest_sidecar_http_bytes(b"GET /metrics HTTP/1.1\r\nHost: local\r\n\r\n")
                .expect("metrics");
        assert_eq!(metrics.status_code, 200);
        assert!(metrics.body.contains("ai_blaise_sidecar_ready"));
    }

    #[test]
    fn http_front_door_rejects_malformed_request() {
        assert!(matches!(
            handle_postgrest_sidecar_http_bytes(b"not-http"),
            Err(PostgrestSidecarError::MalformedHttpRequest)
        ));
    }
}
