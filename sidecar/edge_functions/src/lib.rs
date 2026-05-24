//! Edge function sidecar contracts.

// FEATURE: EF1
// FEATURE: EF2
// FEATURE: EF4
// FEATURE: EF5

use ai_blaise_citus_sidecar_cdc::CdcOperation;
use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct EdgeFunctionPlan {
    pub name: String,
    pub runtime: EdgeFunctionRuntime,
    pub source: FunctionSource,
    pub triggers: Vec<FunctionTrigger>,
    pub env_secret_refs: Vec<String>,
    pub db_callback: Option<DbCallbackPlan>,
}

impl EdgeFunctionPlan {
    pub fn validate(&self) -> Result<(), EdgeFunctionError> {
        validate_identifier("name", &self.name)?;
        self.source.validate()?;
        if self.triggers.is_empty() {
            return Err(EdgeFunctionError::MissingRequiredField("triggers"));
        }
        for trigger in &self.triggers {
            trigger.validate()?;
        }
        validate_optional_list("env_secret_refs", &self.env_secret_refs)?;
        if let Some(callback) = &self.db_callback {
            callback.validate()?;
        }
        Ok(())
    }

    pub fn launch_plan(&self) -> Result<RuntimeLaunchPlan, EdgeFunctionError> {
        self.validate()?;

        let mut args = match self.runtime {
            EdgeFunctionRuntime::Deno => vec![
                "run".to_string(),
                "--allow-env".to_string(),
                "--allow-net=unix".to_string(),
            ],
            EdgeFunctionRuntime::Bun => vec!["run".to_string()],
        };
        args.push(self.source.entrypoint().to_string());

        Ok(RuntimeLaunchPlan {
            function_name: self.name.clone(),
            executable: self.runtime.executable().to_string(),
            args,
            env_secret_refs: self.env_secret_refs.clone(),
            db_callback_socket: self
                .db_callback
                .as_ref()
                .map(|callback| callback.uds_path.clone()),
        })
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum EdgeFunctionRuntime {
    Deno,
    Bun,
}

impl EdgeFunctionRuntime {
    fn executable(self) -> &'static str {
        match self {
            Self::Deno => "deno",
            Self::Bun => "bun",
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum FunctionSource {
    Inline {
        code: String,
    },
    BundleUri {
        uri: String,
        entrypoint: String,
    },
    GitRef {
        repository: String,
        reference: String,
        path: String,
    },
}

impl FunctionSource {
    fn validate(&self) -> Result<(), EdgeFunctionError> {
        match self {
            Self::Inline { code } => validate_required("source.inline.code", code),
            Self::BundleUri { uri, entrypoint } => {
                validate_object_uri("source.bundle_uri", uri)?;
                validate_required("source.entrypoint", entrypoint)
            }
            Self::GitRef {
                repository,
                reference,
                path,
            } => {
                validate_http_url("source.repository", repository)?;
                validate_required("source.reference", reference)?;
                validate_required("source.path", path)
            }
        }
    }

    fn entrypoint(&self) -> &str {
        match self {
            Self::Inline { .. } => "inline.ts",
            Self::BundleUri { entrypoint, .. } => entrypoint,
            Self::GitRef { path, .. } => path,
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum FunctionTrigger {
    Http {
        path: String,
    },
    Scheduled {
        schedule: String,
    },
    CdcEvent {
        table: String,
        operation: CdcOperation,
    },
}

impl FunctionTrigger {
    fn validate(&self) -> Result<(), EdgeFunctionError> {
        match self {
            Self::Http { path } => {
                validate_required("trigger.http.path", path)?;
                if path.starts_with('/') {
                    Ok(())
                } else {
                    Err(EdgeFunctionError::InvalidHttpPath)
                }
            }
            Self::Scheduled { schedule } => validate_required("trigger.schedule", schedule),
            Self::CdcEvent { table, .. } => validate_qualified_name("trigger.table", table),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct DbCallbackPlan {
    pub uds_path: String,
    pub database: String,
    pub role: String,
    pub statement_timeout_ms: u32,
}

impl DbCallbackPlan {
    fn validate(&self) -> Result<(), EdgeFunctionError> {
        validate_required("db_callback.uds_path", &self.uds_path)?;
        if !self.uds_path.starts_with('/') {
            return Err(EdgeFunctionError::InvalidUdsPath);
        }
        validate_identifier("db_callback.database", &self.database)?;
        validate_identifier("db_callback.role", &self.role)?;
        if self.statement_timeout_ms == 0 {
            return Err(EdgeFunctionError::InvalidStatementTimeout);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct RuntimeLaunchPlan {
    pub function_name: String,
    pub executable: String,
    pub args: Vec<String>,
    pub env_secret_refs: Vec<String>,
    pub db_callback_socket: Option<String>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct InvocationRequest {
    pub function_name: String,
    pub tenant_id: String,
    pub trigger: FunctionTrigger,
    pub payload_bytes: u64,
    pub timeout_ms: u32,
}

impl InvocationRequest {
    pub fn validate(&self) -> Result<(), EdgeFunctionError> {
        validate_identifier("invocation.function_name", &self.function_name)?;
        validate_required("invocation.tenant_id", &self.tenant_id)?;
        self.trigger.validate()?;
        if self.payload_bytes == 0 {
            return Err(EdgeFunctionError::InvalidPayloadSize);
        }
        if self.timeout_ms == 0 {
            return Err(EdgeFunctionError::InvalidInvocationTimeout);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum EdgeFunctionError {
    FunctionNameMismatch,
    FunctionNotFound,
    InvalidHttpPath,
    InvalidIdentifier(&'static str),
    InvalidInvocationTimeout,
    InvalidObjectUri(&'static str),
    InvalidPayloadSize,
    InvalidStatementTimeout,
    InvalidUdsPath,
    InvalidUrl(&'static str),
    InvocationTimeoutExceedsPlan {
        timeout_ms: u32,
        max_timeout_ms: u32,
    },
    MalformedHttpRequest,
    MissingRequiredField(&'static str),
    Runtime(String),
    TriggerNotAllowed,
}

impl fmt::Display for EdgeFunctionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FunctionNameMismatch => {
                write!(formatter, "invocation function_name does not match plan")
            }
            Self::FunctionNotFound => write!(formatter, "no registered function matches the name"),
            Self::InvalidHttpPath => write!(formatter, "HTTP trigger path must start with /"),
            Self::InvalidIdentifier(field) => write!(formatter, "{field} must be a SQL identifier"),
            Self::InvalidInvocationTimeout => {
                write!(formatter, "timeout_ms must be greater than zero")
            }
            Self::InvalidObjectUri(field) => write!(formatter, "{field} must be an object URI"),
            Self::InvalidPayloadSize => {
                write!(formatter, "payload_bytes must be greater than zero")
            }
            Self::InvalidStatementTimeout => {
                write!(formatter, "statement_timeout_ms must be greater than zero")
            }
            Self::InvalidUdsPath => write!(formatter, "UDS path must be absolute"),
            Self::InvalidUrl(field) => {
                write!(formatter, "{field} must start with http:// or https://")
            }
            Self::InvocationTimeoutExceedsPlan {
                timeout_ms,
                max_timeout_ms,
            } => write!(
                formatter,
                "invocation timeout {timeout_ms} exceeds runtime callback bound {max_timeout_ms}"
            ),
            Self::MalformedHttpRequest => {
                write!(formatter, "malformed edge-functions sidecar HTTP request")
            }
            Self::MissingRequiredField(field) => write!(formatter, "{field} must not be empty"),
            Self::Runtime(error) => write!(formatter, "{error}"),
            Self::TriggerNotAllowed => write!(formatter, "invocation trigger is not configured"),
        }
    }
}

impl Error for EdgeFunctionError {}

fn validate_required(field: &'static str, value: &str) -> Result<(), EdgeFunctionError> {
    if value.trim().is_empty() {
        return Err(EdgeFunctionError::MissingRequiredField(field));
    }
    Ok(())
}

fn validate_optional_list(field: &'static str, values: &[String]) -> Result<(), EdgeFunctionError> {
    if values.iter().any(|value| value.trim().is_empty()) {
        return Err(EdgeFunctionError::MissingRequiredField(field));
    }
    Ok(())
}

fn validate_identifier(field: &'static str, value: &str) -> Result<(), EdgeFunctionError> {
    validate_required(field, value)?;
    if value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
    {
        Ok(())
    } else {
        Err(EdgeFunctionError::InvalidIdentifier(field))
    }
}

fn validate_qualified_name(field: &'static str, value: &str) -> Result<(), EdgeFunctionError> {
    validate_required(field, value)?;
    let parts: Vec<_> = value.split('.').collect();
    if parts.len() == 2
        && parts
            .iter()
            .all(|part| validate_identifier(field, part).is_ok())
    {
        Ok(())
    } else {
        Err(EdgeFunctionError::InvalidIdentifier(field))
    }
}

fn validate_http_url(field: &'static str, value: &str) -> Result<(), EdgeFunctionError> {
    validate_required(field, value)?;
    if value.starts_with("http://") || value.starts_with("https://") {
        Ok(())
    } else {
        Err(EdgeFunctionError::InvalidUrl(field))
    }
}

fn validate_object_uri(field: &'static str, value: &str) -> Result<(), EdgeFunctionError> {
    validate_required(field, value)?;
    if value.starts_with("s3://") || value.starts_with("gs://") || value.starts_with("az://") {
        Ok(())
    } else {
        Err(EdgeFunctionError::InvalidObjectUri(field))
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct EdgeFunctionCanonicalReport {
    pub plan: EdgeFunctionPlan,
    pub launch: RuntimeLaunchPlan,
    pub invocation: InvocationRequest,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum InvocationStatus {
    Succeeded,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct EdgeFunctionExecution {
    pub function_name: String,
    pub runtime: EdgeFunctionRuntime,
    pub command: Vec<String>,
    pub trigger: FunctionTrigger,
    pub tenant_id: String,
    pub payload_bytes: u64,
    pub response_bytes: u64,
    pub db_callback_used: bool,
    pub status: InvocationStatus,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct EdgeFunctionRuntimeState {
    pub launched_functions: u64,
    pub invocations: u64,
    pub db_callbacks: u64,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct EdgeFunctionRuntimeReport {
    pub launch: RuntimeLaunchPlan,
    pub execution: EdgeFunctionExecution,
    pub state: EdgeFunctionRuntimeState,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct EdgeFunctionRuntimeHost {
    plan: EdgeFunctionPlan,
    launch: RuntimeLaunchPlan,
    state: EdgeFunctionRuntimeState,
}

impl EdgeFunctionRuntimeHost {
    pub fn new(plan: EdgeFunctionPlan) -> Result<Self, EdgeFunctionError> {
        let launch = plan.launch_plan()?;

        Ok(Self {
            plan,
            launch,
            state: EdgeFunctionRuntimeState {
                launched_functions: 1,
                invocations: 0,
                db_callbacks: 0,
            },
        })
    }

    pub fn state(&self) -> &EdgeFunctionRuntimeState {
        &self.state
    }

    pub fn launch(&self) -> &RuntimeLaunchPlan {
        &self.launch
    }

    pub fn invoke(
        &mut self,
        request: &InvocationRequest,
    ) -> Result<EdgeFunctionExecution, EdgeFunctionError> {
        request.validate()?;
        if request.function_name != self.plan.name {
            return Err(EdgeFunctionError::FunctionNameMismatch);
        }
        if !self
            .plan
            .triggers
            .iter()
            .any(|trigger| trigger == &request.trigger)
        {
            return Err(EdgeFunctionError::TriggerNotAllowed);
        }
        if let Some(callback) = &self.plan.db_callback {
            if request.timeout_ms > callback.statement_timeout_ms {
                return Err(EdgeFunctionError::InvocationTimeoutExceedsPlan {
                    timeout_ms: request.timeout_ms,
                    max_timeout_ms: callback.statement_timeout_ms,
                });
            }
        }

        let db_callback_used = self.plan.db_callback.is_some();
        self.state.invocations += 1;
        if db_callback_used {
            self.state.db_callbacks += 1;
        }

        let mut command = vec![self.launch.executable.clone()];
        command.extend(self.launch.args.clone());

        Ok(EdgeFunctionExecution {
            function_name: request.function_name.clone(),
            runtime: self.plan.runtime,
            command,
            trigger: request.trigger.clone(),
            tenant_id: request.tenant_id.clone(),
            payload_bytes: request.payload_bytes,
            response_bytes: deterministic_response_bytes(request.payload_bytes),
            db_callback_used,
            status: InvocationStatus::Succeeded,
        })
    }
}

fn deterministic_response_bytes(payload_bytes: u64) -> u64 {
    (payload_bytes / 2) + 64
}

pub fn canonical_edge_function_plan() -> EdgeFunctionPlan {
    EdgeFunctionPlan {
        name: "order_created".to_string(),
        runtime: EdgeFunctionRuntime::Deno,
        source: FunctionSource::Inline {
            code: "export default async function handler(request) { return Response.json({ ok: true }); }"
                .to_string(),
        },
        triggers: vec![
            FunctionTrigger::Http {
                path: "/orders".to_string(),
            },
            FunctionTrigger::CdcEvent {
                table: "public.orders".to_string(),
                operation: CdcOperation::Insert,
            },
        ],
        env_secret_refs: vec!["orders-api-key".to_string()],
        db_callback: Some(DbCallbackPlan {
            uds_path: "/var/run/postgresql/.s.PGSQL.5432".to_string(),
            database: "app".to_string(),
            role: "edge_runtime".to_string(),
            statement_timeout_ms: 1_000,
        }),
    }
}

pub fn canonical_bun_edge_function_plan() -> EdgeFunctionPlan {
    EdgeFunctionPlan {
        name: "invoice_sync".to_string(),
        runtime: EdgeFunctionRuntime::Bun,
        source: FunctionSource::BundleUri {
            uri: "s3://functions/invoice-sync.tgz".to_string(),
            entrypoint: "index.ts".to_string(),
        },
        triggers: vec![FunctionTrigger::Scheduled {
            schedule: "*/5 * * * *".to_string(),
        }],
        env_secret_refs: Vec::new(),
        db_callback: None,
    }
}

pub fn canonical_invocation_request() -> InvocationRequest {
    InvocationRequest {
        function_name: "order_created".to_string(),
        tenant_id: "tenant-a".to_string(),
        trigger: FunctionTrigger::CdcEvent {
            table: "public.orders".to_string(),
            operation: CdcOperation::Insert,
        },
        payload_bytes: 512,
        timeout_ms: 1_000,
    }
}

pub fn canonical_bun_invocation_request() -> InvocationRequest {
    InvocationRequest {
        function_name: "invoice_sync".to_string(),
        tenant_id: "tenant-a".to_string(),
        trigger: FunctionTrigger::Scheduled {
            schedule: "*/5 * * * *".to_string(),
        },
        payload_bytes: 256,
        timeout_ms: 500,
    }
}

pub fn canonical_edge_function_report() -> Result<EdgeFunctionCanonicalReport, EdgeFunctionError> {
    let plan = canonical_edge_function_plan();
    let launch = plan.launch_plan()?;
    let invocation = canonical_invocation_request();
    invocation.validate()?;

    Ok(EdgeFunctionCanonicalReport {
        plan,
        launch,
        invocation,
    })
}

pub fn canonical_edge_function_runtime_report(
) -> Result<EdgeFunctionRuntimeReport, EdgeFunctionError> {
    let mut runtime = EdgeFunctionRuntimeHost::new(canonical_edge_function_plan())?;
    let execution = runtime.invoke(&canonical_invocation_request())?;

    Ok(EdgeFunctionRuntimeReport {
        launch: runtime.launch().clone(),
        execution,
        state: runtime.state().clone(),
    })
}

pub fn canonical_bun_edge_function_runtime_report(
) -> Result<EdgeFunctionRuntimeReport, EdgeFunctionError> {
    let mut runtime = EdgeFunctionRuntimeHost::new(canonical_bun_edge_function_plan())?;
    let execution = runtime.invoke(&canonical_bun_invocation_request())?;

    Ok(EdgeFunctionRuntimeReport {
        launch: runtime.launch().clone(),
        execution,
        state: runtime.state().clone(),
    })
}

// =============================================================================
// FEATURE: EF3 (runtime CRD-mirror)
// FEATURE: EF6 (UDF substrate runtime hook)
// Registry + HTTP front door + UDS callback + trigger scheduler
// =============================================================================

use ai_blaise_citus_sidecar_shared::{
    listen_addr_from_env, HttpProbeResponse, SidecarRuntime, SidecarRuntimeError,
};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct EdgeFunctionRegistry {
    functions: BTreeMap<String, EdgeFunctionRuntimeHost>,
    invocations: u64,
    triggered_invocations: u64,
    db_callbacks: u64,
}

impl Default for EdgeFunctionRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl EdgeFunctionRegistry {
    pub fn new() -> Self {
        Self {
            functions: BTreeMap::new(),
            invocations: 0,
            triggered_invocations: 0,
            db_callbacks: 0,
        }
    }

    pub fn register(&mut self, plan: EdgeFunctionPlan) -> Result<(), EdgeFunctionError> {
        let host = EdgeFunctionRuntimeHost::new(plan)?;
        self.functions.insert(host.plan_name(), host);
        Ok(())
    }

    pub fn list(&self) -> Vec<EdgeFunctionRegistrySummary> {
        self.functions
            .values()
            .map(|host| EdgeFunctionRegistrySummary {
                name: host.plan_name(),
                runtime: host.runtime_kind(),
                triggers: host.trigger_kinds(),
                db_callback_socket: host.db_callback_socket().map(str::to_string),
            })
            .collect()
    }

    pub fn invoke(
        &mut self,
        request: &InvocationRequest,
    ) -> Result<EdgeFunctionExecution, EdgeFunctionError> {
        let host = self
            .functions
            .get_mut(&request.function_name)
            .ok_or(EdgeFunctionError::FunctionNotFound)?;
        let execution = host.invoke(request)?;
        self.invocations += 1;
        if matches!(
            request.trigger,
            FunctionTrigger::Scheduled { .. } | FunctionTrigger::CdcEvent { .. }
        ) {
            self.triggered_invocations += 1;
        }
        if execution.db_callback_used {
            self.db_callbacks += 1;
        }
        Ok(execution)
    }

    pub fn snapshot(&self) -> EdgeFunctionRegistrySnapshot {
        EdgeFunctionRegistrySnapshot {
            functions: self.list(),
            invocations: self.invocations,
            triggered_invocations: self.triggered_invocations,
            db_callbacks: self.db_callbacks,
        }
    }

    pub fn invocations(&self) -> u64 {
        self.invocations
    }
}

impl EdgeFunctionRuntimeHost {
    pub fn plan_name(&self) -> String {
        self.launch.function_name.clone()
    }

    pub fn runtime_kind(&self) -> EdgeFunctionRuntime {
        match self.launch.executable.as_str() {
            "deno" => EdgeFunctionRuntime::Deno,
            "bun" => EdgeFunctionRuntime::Bun,
            _ => EdgeFunctionRuntime::Deno,
        }
    }

    pub fn trigger_kinds(&self) -> Vec<EdgeFunctionTriggerKind> {
        self.plan
            .triggers
            .iter()
            .map(|trigger| match trigger {
                FunctionTrigger::Http { .. } => EdgeFunctionTriggerKind::Http,
                FunctionTrigger::Scheduled { .. } => EdgeFunctionTriggerKind::Scheduled,
                FunctionTrigger::CdcEvent { .. } => EdgeFunctionTriggerKind::CdcEvent,
            })
            .collect()
    }

    pub fn db_callback_socket(&self) -> Option<&str> {
        self.launch.db_callback_socket.as_deref()
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub enum EdgeFunctionTriggerKind {
    Http,
    Scheduled,
    CdcEvent,
}

impl EdgeFunctionTriggerKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Http => "http",
            Self::Scheduled => "scheduled",
            Self::CdcEvent => "cdc_event",
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct EdgeFunctionRegistrySummary {
    pub name: String,
    pub runtime: EdgeFunctionRuntime,
    pub triggers: Vec<EdgeFunctionTriggerKind>,
    pub db_callback_socket: Option<String>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct EdgeFunctionRegistrySnapshot {
    pub functions: Vec<EdgeFunctionRegistrySummary>,
    pub invocations: u64,
    pub triggered_invocations: u64,
    pub db_callbacks: u64,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct EdgeFunctionUdsCallback {
    pub socket_path: String,
    pub statement_timeout_ms: u32,
    pub statements: Vec<String>,
}

impl EdgeFunctionUdsCallback {
    pub fn for_plan(plan: &EdgeFunctionPlan) -> Result<Self, EdgeFunctionError> {
        let callback = plan
            .db_callback
            .as_ref()
            .ok_or(EdgeFunctionError::MissingRequiredField("db_callback"))?;
        callback.validate()?;
        Ok(Self {
            socket_path: callback.uds_path.clone(),
            statement_timeout_ms: callback.statement_timeout_ms,
            statements: Vec::new(),
        })
    }

    pub fn execute(&mut self, statement: &str) -> Result<&str, EdgeFunctionError> {
        if statement.trim().is_empty() {
            return Err(EdgeFunctionError::MissingRequiredField("statement"));
        }
        if !is_safe_statement(statement) {
            return Err(EdgeFunctionError::TriggerNotAllowed);
        }
        self.statements.push(statement.to_string());
        Ok("ok")
    }

    pub fn statements(&self) -> &[String] {
        &self.statements
    }
}

fn is_safe_statement(statement: &str) -> bool {
    let lower = statement.to_ascii_lowercase();
    let trimmed = lower.trim_start();
    trimmed.starts_with("select")
        || trimmed.starts_with("insert")
        || trimmed.starts_with("update")
        || trimmed.starts_with("delete")
        || trimmed.starts_with("with")
        || trimmed.starts_with("call")
}

// -----------------------------------------------------------------------------
// Trigger scheduler (deterministic; cron lite + event dispatch)
// -----------------------------------------------------------------------------

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ScheduledTriggerTick {
    pub function_name: String,
    pub schedule: String,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct EventTriggerNotice {
    pub function_name: String,
    pub table: String,
    pub operation: CdcOperation,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TriggerScheduler<'registry> {
    registry: &'registry EdgeFunctionRegistry,
    epoch_seconds: u64,
}

impl<'registry> TriggerScheduler<'registry> {
    pub fn new(registry: &'registry EdgeFunctionRegistry, epoch_seconds: u64) -> Self {
        Self {
            registry,
            epoch_seconds,
        }
    }

    pub fn due_schedules(&self) -> Vec<ScheduledTriggerTick> {
        let mut due = Vec::new();
        for host in self.registry.functions.values() {
            for trigger in &host.plan.triggers {
                if let FunctionTrigger::Scheduled { schedule } = trigger {
                    if schedule_due(schedule, self.epoch_seconds) {
                        due.push(ScheduledTriggerTick {
                            function_name: host.plan.name.clone(),
                            schedule: schedule.clone(),
                        });
                    }
                }
            }
        }
        due
    }

    pub fn matching_events(&self, table: &str, op: &CdcOperation) -> Vec<EventTriggerNotice> {
        let mut notices = Vec::new();
        for host in self.registry.functions.values() {
            for trigger in &host.plan.triggers {
                if let FunctionTrigger::CdcEvent {
                    table: trigger_table,
                    operation,
                } = trigger
                {
                    if trigger_table == table && operation == op {
                        notices.push(EventTriggerNotice {
                            function_name: host.plan.name.clone(),
                            table: trigger_table.clone(),
                            operation: *operation,
                        });
                    }
                }
            }
        }
        notices
    }
}

fn schedule_due(schedule: &str, epoch_seconds: u64) -> bool {
    // Minimal cron-style match: `*/<N> * * * *` means every N minutes.
    let mut parts = schedule.split_whitespace();
    let minute = parts.next().unwrap_or("*");
    if let Some(stripped) = minute.strip_prefix("*/") {
        if let Ok(interval) = stripped.parse::<u64>() {
            if interval > 0 {
                return (epoch_seconds / 60).is_multiple_of(interval);
            }
        }
    }
    minute == "*"
}

// -----------------------------------------------------------------------------
// HTTP front door
// -----------------------------------------------------------------------------

pub fn handle_edge_functions_sidecar_http_bytes(
    request: &[u8],
) -> Result<HttpProbeResponse, EdgeFunctionError> {
    let request =
        std::str::from_utf8(request).map_err(|_| EdgeFunctionError::MalformedHttpRequest)?;
    let (method, path, body) = parse_http_request(request)?;
    let mut registry = canonical_edge_function_registry()?;

    if method == "GET" && path == "/functions" {
        let snapshot = registry.snapshot();
        return Ok(HttpProbeResponse::new(
            200,
            "application/json",
            render_registry_snapshot(&snapshot),
        ));
    }
    if method == "POST" && path == "/functions" {
        return match register_plan_from_body(body) {
            Ok(plan) => match registry.register(plan.clone()) {
                Ok(()) => Ok(HttpProbeResponse::new(
                    201,
                    "application/json",
                    format!("{{\"registered\":\"{}\"}}\n", plan.name),
                )),
                Err(error) => Ok(HttpProbeResponse::new(
                    400,
                    "application/json",
                    format!("{{\"error\":\"{}\"}}\n", escape_json(&error.to_string())),
                )),
            },
            Err(error) => Ok(HttpProbeResponse::new(
                400,
                "application/json",
                format!("{{\"error\":\"{}\"}}\n", escape_json(&error.to_string())),
            )),
        };
    }
    if method == "POST" && path.starts_with("/functions/") {
        let name = path.trim_start_matches("/functions/");
        let invocation = canonical_invocation_for_registered(&registry, name, body)?;
        return match registry.invoke(&invocation) {
            Ok(execution) => Ok(HttpProbeResponse::new(
                200,
                "application/json",
                render_execution(&execution),
            )),
            Err(error) => Ok(HttpProbeResponse::new(
                400,
                "application/json",
                format!("{{\"error\":\"{}\"}}\n", escape_json(&error.to_string())),
            )),
        };
    }

    let mut runtime = SidecarRuntime::ready("edge-functions");
    Ok(runtime.handle_http_bytes(request.as_bytes())?)
}

pub fn canonical_edge_function_registry() -> Result<EdgeFunctionRegistry, EdgeFunctionError> {
    let mut registry = EdgeFunctionRegistry::new();
    registry.register(canonical_edge_function_plan())?;
    Ok(registry)
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct EdgeFunctionRegistryReport {
    pub snapshot: EdgeFunctionRegistrySnapshot,
    pub execution: EdgeFunctionExecution,
    pub due_schedules: Vec<ScheduledTriggerTick>,
    pub events: Vec<EventTriggerNotice>,
    pub uds_callback: EdgeFunctionUdsCallback,
}

pub fn canonical_edge_function_registry_report(
) -> Result<EdgeFunctionRegistryReport, EdgeFunctionError> {
    let mut registry = canonical_edge_function_registry()?;
    let execution = registry.invoke(&canonical_invocation_request())?;
    let scheduler = TriggerScheduler::new(&registry, 0);
    let due_schedules = scheduler.due_schedules();
    let events = scheduler.matching_events("public.orders", &CdcOperation::Insert);
    let mut uds_callback = EdgeFunctionUdsCallback::for_plan(&canonical_edge_function_plan())?;
    uds_callback.execute("select 1")?;
    Ok(EdgeFunctionRegistryReport {
        snapshot: registry.snapshot(),
        execution,
        due_schedules,
        events,
        uds_callback,
    })
}

fn render_registry_snapshot(snapshot: &EdgeFunctionRegistrySnapshot) -> String {
    let functions = snapshot
        .functions
        .iter()
        .map(|summary| {
            let triggers = summary
                .triggers
                .iter()
                .map(|trigger| format!("\"{}\"", trigger.as_str()))
                .collect::<Vec<_>>()
                .join(",");
            let socket = summary
                .db_callback_socket
                .as_deref()
                .map(|value| format!("\"{}\"", escape_json(value)))
                .unwrap_or_else(|| "null".to_string());
            format!(
                "{{\"name\":\"{}\",\"runtime\":\"{}\",\"triggers\":[{}],\"db_callback_socket\":{}}}",
                summary.name,
                match summary.runtime {
                    EdgeFunctionRuntime::Deno => "deno",
                    EdgeFunctionRuntime::Bun => "bun",
                },
                triggers,
                socket,
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "{{\"functions\":[{}],\"invocations\":{},\"triggered_invocations\":{},\"db_callbacks\":{}}}\n",
        functions, snapshot.invocations, snapshot.triggered_invocations, snapshot.db_callbacks,
    )
}

fn render_execution(execution: &EdgeFunctionExecution) -> String {
    format!(
        "{{\"function\":\"{}\",\"runtime\":\"{}\",\"trigger\":\"{}\",\"tenant_id\":\"{}\",\"payload_bytes\":{},\"response_bytes\":{},\"db_callback_used\":{},\"status\":\"{}\"}}\n",
        execution.function_name,
        match execution.runtime {
            EdgeFunctionRuntime::Deno => "deno",
            EdgeFunctionRuntime::Bun => "bun",
        },
        match &execution.trigger {
            FunctionTrigger::Http { path } => format!("http:{path}"),
            FunctionTrigger::Scheduled { schedule } => format!("scheduled:{schedule}"),
            FunctionTrigger::CdcEvent { table, operation } => {
                format!(
                    "cdc:{table}:{}",
                    match operation {
                        CdcOperation::Insert => "insert",
                        CdcOperation::Update => "update",
                        CdcOperation::Delete => "delete",
                        CdcOperation::Truncate => "truncate",
                    }
                )
            }
        },
        execution.tenant_id,
        execution.payload_bytes,
        execution.response_bytes,
        execution.db_callback_used,
        match execution.status {
            InvocationStatus::Succeeded => "succeeded",
        },
    )
}

fn register_plan_from_body(body: &str) -> Result<EdgeFunctionPlan, EdgeFunctionError> {
    let name = body_field(body, "name").ok_or(EdgeFunctionError::MissingRequiredField("name"))?;
    let runtime_str = body_field(body, "runtime").unwrap_or_else(|| "deno".to_string());
    let runtime = match runtime_str.as_str() {
        "deno" => EdgeFunctionRuntime::Deno,
        "bun" => EdgeFunctionRuntime::Bun,
        _ => {
            return Err(EdgeFunctionError::InvalidIdentifier("runtime"));
        }
    };
    let code = body_field(body, "code").ok_or(EdgeFunctionError::MissingRequiredField("code"))?;
    let trigger_path = body_field(body, "http_path").unwrap_or_else(|| "/".to_string());

    let plan = EdgeFunctionPlan {
        name,
        runtime,
        source: FunctionSource::Inline { code },
        triggers: vec![FunctionTrigger::Http { path: trigger_path }],
        env_secret_refs: Vec::new(),
        db_callback: None,
    };
    plan.validate()?;
    Ok(plan)
}

fn canonical_invocation_for_registered(
    registry: &EdgeFunctionRegistry,
    name: &str,
    body: &str,
) -> Result<InvocationRequest, EdgeFunctionError> {
    let host = registry
        .functions
        .get(name)
        .ok_or(EdgeFunctionError::FunctionNotFound)?;
    let trigger = host
        .plan
        .triggers
        .iter()
        .find(|trigger| matches!(trigger, FunctionTrigger::Http { .. }))
        .or_else(|| host.plan.triggers.first())
        .ok_or(EdgeFunctionError::TriggerNotAllowed)?
        .clone();
    let tenant_id = body_field(body, "tenant_id").unwrap_or_else(|| "tenant-a".to_string());
    let payload_bytes = body_field(body, "payload_bytes")
        .and_then(|raw| raw.parse::<u64>().ok())
        .unwrap_or(64);
    let timeout_ms = body_field(body, "timeout_ms")
        .and_then(|raw| raw.parse::<u32>().ok())
        .unwrap_or(500);
    Ok(InvocationRequest {
        function_name: name.to_string(),
        tenant_id,
        trigger,
        payload_bytes,
        timeout_ms,
    })
}

fn body_field(body: &str, field: &str) -> Option<String> {
    let needle = format!("\"{field}\":");
    let start = body.find(&needle)? + needle.len();
    let mut chars = body[start..].chars().peekable();
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

pub fn serve_edge_functions_sidecar_http_forever(
    default_addr: &str,
) -> Result<(), EdgeFunctionError> {
    use std::io::Write;
    use std::net::TcpListener;

    canonical_edge_function_registry()?;
    let listen_addr = listen_addr_from_env(default_addr)?;
    let listener = TcpListener::bind(&listen_addr)?;
    eprintln!("ai-blaise edge-functions sidecar listening on {listen_addr}");

    for stream in listener.incoming() {
        let mut stream = stream?;
        let request = read_http_request(&mut stream)?;
        let response = handle_edge_functions_sidecar_http_bytes(&request).unwrap_or_else(|error| {
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

fn parse_http_request(request: &str) -> Result<(&str, &str, &str), EdgeFunctionError> {
    let (head, body) = request
        .split_once("\r\n\r\n")
        .or_else(|| request.split_once("\n\n"))
        .ok_or(EdgeFunctionError::MalformedHttpRequest)?;
    let request_line = head
        .lines()
        .next()
        .ok_or(EdgeFunctionError::MalformedHttpRequest)?;
    let mut parts = request_line.split_whitespace();
    let method = parts
        .next()
        .ok_or(EdgeFunctionError::MalformedHttpRequest)?;
    let path = parts
        .next()
        .ok_or(EdgeFunctionError::MalformedHttpRequest)?;
    if !path.starts_with('/') {
        return Err(EdgeFunctionError::MalformedHttpRequest);
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

impl From<SidecarRuntimeError> for EdgeFunctionError {
    fn from(error: SidecarRuntimeError) -> Self {
        Self::Runtime(error.to_string())
    }
}

impl From<std::io::Error> for EdgeFunctionError {
    fn from(error: std::io::Error) -> Self {
        Self::Runtime(error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deno_function_renders_launch_plan_with_db_callback() {
        let launch = canonical_edge_function_plan()
            .launch_plan()
            .expect("launch plan");

        assert_eq!(launch.executable, "deno");
        assert_eq!(launch.args[0], "run");
        assert_eq!(
            launch.db_callback_socket.as_deref(),
            Some("/var/run/postgresql/.s.PGSQL.5432")
        );
    }

    #[test]
    fn canonical_edge_function_report_is_deterministic() {
        let report = canonical_edge_function_report().expect("canonical report");

        assert_eq!(report.launch.function_name, "order_created");
        assert_eq!(report.launch.executable, "deno");
        assert_eq!(report.invocation.payload_bytes, 512);
    }

    #[test]
    fn edge_function_runtime_invokes_cdc_trigger_and_tracks_state() {
        let report = canonical_edge_function_runtime_report().expect("runtime report");

        assert_eq!(report.execution.function_name, "order_created");
        assert_eq!(report.execution.runtime, EdgeFunctionRuntime::Deno);
        assert_eq!(
            report.execution.command,
            vec![
                "deno".to_string(),
                "run".to_string(),
                "--allow-env".to_string(),
                "--allow-net=unix".to_string(),
                "inline.ts".to_string(),
            ]
        );
        assert_eq!(report.execution.response_bytes, 320);
        assert!(report.execution.db_callback_used);
        assert_eq!(report.execution.status, InvocationStatus::Succeeded);
        assert_eq!(report.state.launched_functions, 1);
        assert_eq!(report.state.invocations, 1);
        assert_eq!(report.state.db_callbacks, 1);
    }

    #[test]
    fn edge_function_runtime_rejects_unconfigured_trigger() {
        let mut runtime =
            EdgeFunctionRuntimeHost::new(canonical_edge_function_plan()).expect("runtime");
        let mut request = canonical_invocation_request();
        request.trigger = FunctionTrigger::Http {
            path: "/admin".to_string(),
        };

        assert_eq!(
            runtime.invoke(&request),
            Err(EdgeFunctionError::TriggerNotAllowed)
        );
    }

    #[test]
    fn edge_function_runtime_rejects_callback_timeout_over_plan() {
        let mut runtime =
            EdgeFunctionRuntimeHost::new(canonical_edge_function_plan()).expect("runtime");
        let mut request = canonical_invocation_request();
        request.timeout_ms = 1_001;

        assert_eq!(
            runtime.invoke(&request),
            Err(EdgeFunctionError::InvocationTimeoutExceedsPlan {
                timeout_ms: 1_001,
                max_timeout_ms: 1_000,
            })
        );
    }

    #[test]
    fn bun_bundle_renders_bun_launch_plan() {
        let plan = EdgeFunctionPlan {
            name: "invoice_sync".to_string(),
            runtime: EdgeFunctionRuntime::Bun,
            source: FunctionSource::BundleUri {
                uri: "s3://functions/invoice-sync.tgz".to_string(),
                entrypoint: "index.ts".to_string(),
            },
            triggers: vec![FunctionTrigger::Scheduled {
                schedule: "*/5 * * * *".to_string(),
            }],
            env_secret_refs: Vec::new(),
            db_callback: None,
        };

        let launch = plan.launch_plan().expect("launch plan");

        assert_eq!(launch.executable, "bun");
        assert_eq!(launch.args, vec!["run".to_string(), "index.ts".to_string()]);
    }

    #[test]
    fn db_callback_requires_absolute_uds_path() {
        let mut plan = canonical_edge_function_plan();
        plan.db_callback = Some(DbCallbackPlan {
            uds_path: "postgres/.s.PGSQL.5432".to_string(),
            database: "app".to_string(),
            role: "edge_runtime".to_string(),
            statement_timeout_ms: 1_000,
        });

        assert_eq!(plan.validate(), Err(EdgeFunctionError::InvalidUdsPath));
    }

    #[test]
    fn cdc_trigger_requires_qualified_table() {
        let trigger = FunctionTrigger::CdcEvent {
            table: "orders".to_string(),
            operation: CdcOperation::Insert,
        };

        assert_eq!(
            trigger.validate(),
            Err(EdgeFunctionError::InvalidIdentifier("trigger.table"))
        );
    }

    #[test]
    fn invocation_request_requires_payload_and_timeout() {
        let request = InvocationRequest {
            function_name: "order_created".to_string(),
            tenant_id: "tenant-a".to_string(),
            trigger: FunctionTrigger::Http {
                path: "/orders".to_string(),
            },
            payload_bytes: 0,
            timeout_ms: 1000,
        };

        assert_eq!(
            request.validate(),
            Err(EdgeFunctionError::InvalidPayloadSize)
        );
    }

    #[test]
    fn registry_registers_and_invokes_canonical_function() {
        let mut registry = canonical_edge_function_registry().expect("registry");
        let execution = registry
            .invoke(&canonical_invocation_request())
            .expect("invocation");

        assert_eq!(execution.function_name, "order_created");
        let summaries = registry.list();
        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].name, "order_created");
        assert!(summaries[0]
            .triggers
            .contains(&EdgeFunctionTriggerKind::CdcEvent));
        assert_eq!(registry.invocations(), 1);
    }

    #[test]
    fn registry_invocation_rejects_unknown_function() {
        let mut registry = EdgeFunctionRegistry::new();
        let request = canonical_invocation_request();

        assert_eq!(
            registry.invoke(&request),
            Err(EdgeFunctionError::FunctionNotFound)
        );
    }

    #[test]
    fn trigger_scheduler_collects_due_schedules() {
        let mut registry = EdgeFunctionRegistry::new();
        let plan = EdgeFunctionPlan {
            name: "scheduled_report".to_string(),
            runtime: EdgeFunctionRuntime::Bun,
            source: FunctionSource::Inline {
                code: "export default { fetch: () => new Response('ok') }".to_string(),
            },
            triggers: vec![FunctionTrigger::Scheduled {
                schedule: "*/5 * * * *".to_string(),
            }],
            env_secret_refs: Vec::new(),
            db_callback: None,
        };
        registry.register(plan).expect("register");
        let scheduler = TriggerScheduler::new(&registry, 0);

        let due = scheduler.due_schedules();
        assert_eq!(due.len(), 1);
        assert_eq!(due[0].function_name, "scheduled_report");
    }

    #[test]
    fn trigger_scheduler_matches_cdc_events() {
        let registry = canonical_edge_function_registry().expect("registry");
        let scheduler = TriggerScheduler::new(&registry, 0);

        let events = scheduler.matching_events("public.orders", &CdcOperation::Insert);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].function_name, "order_created");
        assert_eq!(events[0].operation, CdcOperation::Insert);
    }

    #[test]
    fn uds_callback_validates_statement_safety() {
        let plan = canonical_edge_function_plan();
        let mut callback = EdgeFunctionUdsCallback::for_plan(&plan).expect("callback");

        callback.execute("select count(*) from orders").expect("ok");
        assert_eq!(callback.statements().len(), 1);
        assert_eq!(
            callback.execute("drop table orders"),
            Err(EdgeFunctionError::TriggerNotAllowed)
        );
    }

    #[test]
    fn canonical_edge_function_registry_report_is_deterministic() {
        let report = canonical_edge_function_registry_report().expect("report");

        assert_eq!(report.snapshot.functions.len(), 1);
        assert_eq!(report.snapshot.invocations, 1);
        assert_eq!(report.snapshot.triggered_invocations, 1);
        assert_eq!(report.execution.function_name, "order_created");
        assert_eq!(report.events.len(), 1);
        assert_eq!(report.uds_callback.statements().len(), 1);
    }

    #[test]
    fn http_front_door_lists_registered_functions() {
        let response = handle_edge_functions_sidecar_http_bytes(
            b"GET /functions HTTP/1.1\r\nHost: local\r\n\r\n",
        )
        .expect("list");
        assert_eq!(response.status_code, 200);
        assert!(response.body.contains("\"order_created\""));
        assert!(response.body.contains("\"cdc_event\""));
    }

    #[test]
    fn http_front_door_registers_new_inline_function() {
        let body = r#"{"name":"hello","runtime":"deno","code":"export default async () => Response.json({ok:true})","http_path":"/hello"}"#;
        let request = format!(
            "POST /functions HTTP/1.1\r\ncontent-type: application/json\r\ncontent-length: {}\r\n\r\n{}",
            body.len(),
            body
        );
        let response =
            handle_edge_functions_sidecar_http_bytes(request.as_bytes()).expect("register");
        assert_eq!(response.status_code, 201);
        assert!(response.body.contains("hello"));
    }

    #[test]
    fn http_front_door_invokes_canonical_function() {
        let body = r#"{"tenant_id":"tenant-a","payload_bytes":256,"timeout_ms":250}"#;
        let request = format!(
            "POST /functions/order_created HTTP/1.1\r\ncontent-type: application/json\r\ncontent-length: {}\r\n\r\n{}",
            body.len(),
            body
        );
        let response =
            handle_edge_functions_sidecar_http_bytes(request.as_bytes()).expect("invoke");
        assert_eq!(response.status_code, 200);
        assert!(response.body.contains("\"function\":\"order_created\""));
        assert!(response.body.contains("\"status\":\"succeeded\""));
    }

    #[test]
    fn http_front_door_serves_healthz() {
        let response = handle_edge_functions_sidecar_http_bytes(
            b"GET /healthz HTTP/1.1\r\nHost: local\r\n\r\n",
        )
        .expect("healthz");
        assert_eq!(response.status_code, 200);
        assert!(response.body.contains("\"component\":\"edge-functions\""));
    }
}
