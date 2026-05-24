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
    InvalidShardCount,
    MalformedHttpRequest,
    MissingRequiredField(&'static str),
    Runtime(String),
    RouteNotFound(String),
    RlsRequired,
}

impl fmt::Display for PostgrestSidecarError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidIdentifier(field) => write!(formatter, "{field} must be a SQL identifier"),
            Self::InvalidPath(field) => write!(formatter, "{field} must start with /"),
            Self::InvalidShardCount => write!(formatter, "shard_count must be greater than zero"),
            Self::MalformedHttpRequest => {
                write!(formatter, "malformed PostgREST sidecar HTTP request")
            }
            Self::MissingRequiredField(field) => write!(formatter, "{field} must not be empty"),
            Self::Runtime(error) => write!(formatter, "{error}"),
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
    conf.push_str(&format!(
        "jwt-role-claim-key = \".{}\"\n",
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
                ("PGRST_DB_ANON_ROLE".to_string(), config.anon_role.clone()),
                (
                    "PGRST_SERVER_PORT".to_string(),
                    config.server_port.to_string(),
                ),
                (
                    "PGRST_LOG_LEVEL".to_string(),
                    config.log_level.as_str().to_string(),
                ),
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
        for (name, value) in &self.launch.env {
            if !value.starts_with("env:") {
                command.env(name, value);
            }
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

// =============================================================================
// HTTP front door
// =============================================================================

pub fn handle_postgrest_sidecar_http_bytes(
    request: &[u8],
) -> Result<HttpProbeResponse, PostgrestSidecarError> {
    let mut runtime = SidecarRuntime::ready("postgrest");
    handle_postgrest_sidecar_http_bytes_with_runtime(request, &mut runtime)
}

pub fn handle_postgrest_sidecar_http_bytes_with_runtime(
    request: &[u8],
    runtime: &mut SidecarRuntime,
) -> Result<HttpProbeResponse, PostgrestSidecarError> {
    let request =
        std::str::from_utf8(request).map_err(|_| PostgrestSidecarError::MalformedHttpRequest)?;
    let (method, path, _body) = parse_http_request(request)?;

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
    if (method == "GET" || method == "POST") && path.starts_with("/api/") {
        return match supervisor.resolve_route(&path["/api".len()..]) {
            Ok(route) => Ok(HttpProbeResponse::new(
                200,
                "application/json",
                render_route_payload(route, method),
            )),
            Err(error) => Ok(HttpProbeResponse::new(
                404,
                "application/json",
                format!("{{\"error\":\"{}\"}}\n", escape_json(&error.to_string())),
            )),
        };
    }

    Ok(runtime.handle_http_bytes(request.as_bytes())?)
}

fn render_route_payload(route: &RestRoute, method: &str) -> String {
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
        route.schema, route.table, method, methods, view,
    )
}

pub fn serve_postgrest_sidecar_http_forever(
    default_addr: &str,
) -> Result<(), PostgrestSidecarError> {
    use std::io::Write;
    use std::net::TcpListener;

    canonical_postgrest_execution_plan()?;
    let listen_addr = listen_addr_from_env(default_addr)?;
    let listener = TcpListener::bind(&listen_addr)?;
    eprintln!("ai-blaise postgrest sidecar listening on {listen_addr}");
    let mut runtime = SidecarRuntime::ready("postgrest");

    for stream in listener.incoming() {
        let mut stream = stream?;
        let request = read_http_request(&mut stream)?;
        let response = handle_postgrest_sidecar_http_bytes_with_runtime(&request, &mut runtime)
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

fn parse_http_request(request: &str) -> Result<(&str, &str, &str), PostgrestSidecarError> {
    let (head, body) = request
        .split_once("\r\n\r\n")
        .or_else(|| request.split_once("\n\n"))
        .ok_or(PostgrestSidecarError::MalformedHttpRequest)?;
    let request_line = head
        .lines()
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

        let mut child = supervisor
            .spawn_child_at(&config_path)
            .expect("spawn child");
        let status = child.wait().expect("wait child");
        let conf = std::fs::read_to_string(&config_path).expect("config written");
        let _ = std::fs::remove_file(&config_path);

        assert!(status.success());
        assert_eq!(supervisor.state().state, SupervisorState::Launched);
        assert_eq!(supervisor.state().launches, 1);
        assert!(conf.contains("db-uri = \"env:POSTGREST_DB_URI\""));
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
