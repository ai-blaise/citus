//! Edge function sidecar contracts.

// FEATURE: EF1
// FEATURE: EF2
// FEATURE: EF4
// FEATURE: EF5

use ai_blaise_citus_sidecar_cdc::CdcOperation;
use postgres::{Config, NoTls, SimpleQueryMessage};
use serde_json::Value;
use std::env;
use std::error::Error;
use std::fmt;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const MAX_INLINE_SOURCE_BYTES: usize = 262_144;
const MAX_ENTRYPOINT_BYTES: usize = 256;
const MAX_HTTP_PATH_BYTES: usize = 256;
const MAX_SECRET_REFS: usize = 32;
const MAX_INVOCATION_PAYLOAD_BYTES: u64 = 1_048_576;
const MAX_INVOCATION_TIMEOUT_MS: u32 = 30_000;
const MAX_UDS_PATH_BYTES: usize = 107;
const MAX_RUNTIME_STDOUT_BYTES: usize = 65_536;

pub const EDGE_DB_CALLBACK_EXECUTION_ENV: &str = "AI_BLAISE_EDGE_DB_CALLBACK_EXECUTION";
pub const EDGE_RUNTIME_EXECUTION_ENV: &str = "AI_BLAISE_EDGE_RUNTIME_EXECUTION";
pub const EDGE_DENO_BIN_ENV: &str = "AI_BLAISE_DENO_BIN";
pub const EDGE_BUN_BIN_ENV: &str = "AI_BLAISE_BUN_BIN";

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
        validate_env_secret_refs(&self.env_secret_refs)?;
        if let Some(callback) = &self.db_callback {
            callback.validate()?;
        }
        Ok(())
    }

    pub fn launch_plan(&self) -> Result<RuntimeLaunchPlan, EdgeFunctionError> {
        self.validate()?;

        let network_policy = if self.db_callback.is_some() {
            RuntimeNetworkPolicy::UnixOnly
        } else {
            RuntimeNetworkPolicy::None
        };
        let mut args = match self.runtime {
            EdgeFunctionRuntime::Deno => {
                let mut args = vec!["run".to_string(), "--no-prompt".to_string()];
                if !self.env_secret_refs.is_empty() {
                    args.push("--allow-env".to_string());
                }
                if matches!(network_policy, RuntimeNetworkPolicy::UnixOnly) {
                    args.push("--allow-net=unix".to_string());
                }
                args
            }
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
            sandbox: RuntimeSandboxPlan {
                execution_mode: RuntimeExecutionMode::PlanOnly,
                external_runtime_spawned: false,
                user_code_executed: false,
                filesystem_write_allowed: false,
                subprocess_allowed: false,
                network_policy,
            },
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
            Self::Inline { code } => validate_inline_source(code),
            Self::BundleUri { uri, entrypoint } => {
                validate_object_uri("source.bundle_uri", uri)?;
                validate_entrypoint_path("source.entrypoint", entrypoint)
            }
            Self::GitRef {
                repository,
                reference,
                path,
            } => {
                validate_http_url("source.repository", repository)?;
                validate_required("source.reference", reference)?;
                validate_entrypoint_path("source.path", path)
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
            Self::Http { path } => validate_http_path(path),
            Self::Scheduled { schedule } => validate_schedule(schedule),
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
        if !self.uds_path.starts_with('/')
            || self.uds_path.contains('\0')
            || self.uds_path.len() > MAX_UDS_PATH_BYTES
        {
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
    pub sandbox: RuntimeSandboxPlan,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct RuntimeSandboxPlan {
    pub execution_mode: RuntimeExecutionMode,
    pub external_runtime_spawned: bool,
    pub user_code_executed: bool,
    pub filesystem_write_allowed: bool,
    pub subprocess_allowed: bool,
    pub network_policy: RuntimeNetworkPolicy,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum RuntimeExecutionMode {
    PlanOnly,
    Live,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum RuntimeNetworkPolicy {
    None,
    UnixOnly,
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
        if self.payload_bytes > MAX_INVOCATION_PAYLOAD_BYTES {
            return Err(EdgeFunctionError::PayloadTooLarge {
                bytes: self.payload_bytes,
                max_bytes: MAX_INVOCATION_PAYLOAD_BYTES,
            });
        }
        if self.timeout_ms == 0 {
            return Err(EdgeFunctionError::InvalidInvocationTimeout);
        }
        if self.timeout_ms > MAX_INVOCATION_TIMEOUT_MS {
            return Err(EdgeFunctionError::InvocationTimeoutTooLarge {
                timeout_ms: self.timeout_ms,
                max_timeout_ms: MAX_INVOCATION_TIMEOUT_MS,
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum EdgeFunctionError {
    DbCallbackExecutionDisabled,
    FunctionNameMismatch,
    InvalidDbCallbackSocket,
    FunctionNotFound,
    InvalidEntrypoint(&'static str),
    InvalidHttpPath,
    InvalidIdentifier(&'static str),
    InvalidInvocationTimeout,
    InvalidObjectUri(&'static str),
    InvalidPayloadSize,
    InvalidSchedule,
    InvalidSecretRef,
    InvalidSource(&'static str),
    InvalidStatementTimeout,
    InvalidUdsPath,
    InvalidUrl(&'static str),
    InvocationTimeoutExceedsPlan {
        timeout_ms: u32,
        max_timeout_ms: u32,
    },
    InvocationTimeoutTooLarge {
        timeout_ms: u32,
        max_timeout_ms: u32,
    },
    MalformedHttpRequest,
    MissingRequiredField(&'static str),
    PayloadTooLarge {
        bytes: u64,
        max_bytes: u64,
    },
    Runtime(String),
    SourceTooLarge {
        bytes: usize,
        max_bytes: usize,
    },
    TooManySecretRefs {
        count: usize,
        max_count: usize,
    },
    TriggerNotAllowed,
    UnsafeDbCallbackStatement,
    RuntimeExecutionDisabled,
    RuntimeOutputTooLarge {
        bytes: usize,
        max_bytes: usize,
    },
    RuntimeTimedOut {
        timeout_ms: u32,
    },
    UnsupportedRuntimeSource,
    UnsupportedExecutionMode,
}

impl fmt::Display for EdgeFunctionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DbCallbackExecutionDisabled => write!(
                formatter,
                "database callback execution requires AI_BLAISE_EDGE_DB_CALLBACK_EXECUTION=1"
            ),
            Self::FunctionNameMismatch => {
                write!(formatter, "invocation function_name does not match plan")
            }
            Self::InvalidDbCallbackSocket => {
                write!(
                    formatter,
                    "database callback UDS path must point at .s.PGSQL.<port>"
                )
            }
            Self::FunctionNotFound => write!(formatter, "no registered function matches the name"),
            Self::InvalidEntrypoint(field) => {
                write!(formatter, "{field} must be a safe relative entrypoint path")
            }
            Self::InvalidHttpPath => write!(
                formatter,
                "HTTP trigger path must start with / and stay within a single safe path"
            ),
            Self::InvalidIdentifier(field) => write!(formatter, "{field} must be a SQL identifier"),
            Self::InvalidInvocationTimeout => {
                write!(formatter, "timeout_ms must be greater than zero")
            }
            Self::InvalidObjectUri(field) => write!(formatter, "{field} must be an object URI"),
            Self::InvalidPayloadSize => {
                write!(formatter, "payload_bytes must be greater than zero")
            }
            Self::InvalidSchedule => write!(
                formatter,
                "schedule must be a supported five-field cron expression"
            ),
            Self::InvalidSecretRef => {
                write!(formatter, "env_secret_refs must be Kubernetes secret names")
            }
            Self::InvalidSource(field) => write!(formatter, "{field} contains unsupported bytes"),
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
            Self::InvocationTimeoutTooLarge {
                timeout_ms,
                max_timeout_ms,
            } => write!(
                formatter,
                "invocation timeout {timeout_ms} exceeds max runtime bound {max_timeout_ms}"
            ),
            Self::MalformedHttpRequest => {
                write!(formatter, "malformed edge-functions sidecar HTTP request")
            }
            Self::MissingRequiredField(field) => write!(formatter, "{field} must not be empty"),
            Self::PayloadTooLarge { bytes, max_bytes } => write!(
                formatter,
                "payload_bytes {bytes} exceeds max runtime bound {max_bytes}"
            ),
            Self::Runtime(error) => write!(formatter, "{error}"),
            Self::SourceTooLarge { bytes, max_bytes } => write!(
                formatter,
                "inline source bytes {bytes} exceeds max runtime bound {max_bytes}"
            ),
            Self::TooManySecretRefs { count, max_count } => write!(
                formatter,
                "env_secret_refs count {count} exceeds max runtime bound {max_count}"
            ),
            Self::TriggerNotAllowed => write!(formatter, "invocation trigger is not configured"),
            Self::UnsafeDbCallbackStatement => write!(
                formatter,
                "database callback statement must be a single safe DML/SELECT/CALL statement"
            ),
            Self::RuntimeExecutionDisabled => write!(
                formatter,
                "external Deno user-code execution requires AI_BLAISE_EDGE_RUNTIME_EXECUTION=1"
            ),
            Self::RuntimeOutputTooLarge { bytes, max_bytes } => write!(
                formatter,
                "runtime stdout bytes {bytes} exceeds max runtime bound {max_bytes}"
            ),
            Self::RuntimeTimedOut { timeout_ms } => {
                write!(
                    formatter,
                    "runtime execution exceeded timeout_ms {timeout_ms}"
                )
            }
            Self::UnsupportedRuntimeSource => write!(
                formatter,
                "live runtime execution currently requires an inline Deno source"
            ),
            Self::UnsupportedExecutionMode => write!(
                formatter,
                "external Deno/Bun user-code execution is not enabled by this sidecar boundary"
            ),
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

fn validate_env_secret_refs(values: &[String]) -> Result<(), EdgeFunctionError> {
    if values.len() > MAX_SECRET_REFS {
        return Err(EdgeFunctionError::TooManySecretRefs {
            count: values.len(),
            max_count: MAX_SECRET_REFS,
        });
    }
    for value in values {
        validate_required("env_secret_refs", value)?;
        if !is_valid_secret_ref(value) {
            return Err(EdgeFunctionError::InvalidSecretRef);
        }
    }
    Ok(())
}

fn validate_identifier(field: &'static str, value: &str) -> Result<(), EdgeFunctionError> {
    validate_required(field, value)?;
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return Err(EdgeFunctionError::MissingRequiredField(field));
    };
    if !(first.is_ascii_alphabetic() || first == '_') {
        return Err(EdgeFunctionError::InvalidIdentifier(field));
    }
    if chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_') {
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

fn validate_inline_source(code: &str) -> Result<(), EdgeFunctionError> {
    validate_required("source.inline.code", code)?;
    if code.contains('\0') {
        return Err(EdgeFunctionError::InvalidSource("source.inline.code"));
    }
    if code.len() > MAX_INLINE_SOURCE_BYTES {
        return Err(EdgeFunctionError::SourceTooLarge {
            bytes: code.len(),
            max_bytes: MAX_INLINE_SOURCE_BYTES,
        });
    }
    Ok(())
}

fn validate_entrypoint_path(field: &'static str, value: &str) -> Result<(), EdgeFunctionError> {
    validate_required(field, value)?;
    if value.len() > MAX_ENTRYPOINT_BYTES
        || value.starts_with('/')
        || value.contains('\0')
        || value
            .split('/')
            .any(|part| part.is_empty() || part == "." || part == "..")
    {
        return Err(EdgeFunctionError::InvalidEntrypoint(field));
    }
    Ok(())
}

fn validate_http_path(path: &str) -> Result<(), EdgeFunctionError> {
    validate_required("trigger.http.path", path)?;
    if path.len() > MAX_HTTP_PATH_BYTES
        || !path.starts_with('/')
        || path.contains(char::is_whitespace)
        || path.split('/').any(|part| part == "..")
    {
        return Err(EdgeFunctionError::InvalidHttpPath);
    }
    Ok(())
}

fn validate_schedule(schedule: &str) -> Result<(), EdgeFunctionError> {
    validate_required("trigger.schedule", schedule)?;
    let parts = schedule.split_whitespace().collect::<Vec<_>>();
    if parts.len() != 5 {
        return Err(EdgeFunctionError::InvalidSchedule);
    }
    let minute = parts[0];
    let minute_supported = minute == "*"
        || minute
            .strip_prefix("*/")
            .and_then(|value| value.parse::<u64>().ok())
            .is_some_and(|value| value > 0 && value <= 59);
    if minute_supported && parts[1..].iter().all(|part| *part == "*") {
        Ok(())
    } else {
        Err(EdgeFunctionError::InvalidSchedule)
    }
}

fn is_valid_secret_ref(value: &str) -> bool {
    if value.is_empty() || value.len() > 253 {
        return false;
    }
    let bytes = value.as_bytes();
    let edge_ok = |byte: u8| byte.is_ascii_lowercase() || byte.is_ascii_digit();
    if !edge_ok(bytes[0]) || !edge_ok(*bytes.last().unwrap_or(&0)) {
        return false;
    }
    value
        .split('.')
        .all(|part| !part.is_empty() && part.bytes().all(|byte| edge_ok(byte) || byte == b'-'))
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
    Planned,
    DbCallbackExecuted,
    Executed,
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
    pub db_callback_statement_executed: bool,
    pub db_callback_rows: Option<u64>,
    pub user_code_executed: bool,
    pub runtime_response_json: Option<String>,
    pub status: InvocationStatus,
    pub execution_mode: RuntimeExecutionMode,
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
        let db_callback_used = self.validate_invocation(request)?;
        self.record_invocation(db_callback_used);

        Ok(self.render_invocation_execution(
            request,
            db_callback_used,
            false,
            None,
            false,
            None,
            None,
            InvocationStatus::Planned,
            RuntimeExecutionMode::PlanOnly,
        ))
    }

    pub fn invoke_with_db_callback(
        &mut self,
        request: &InvocationRequest,
        db_statement: Option<&str>,
        db_executor: Option<&mut DbCallbackExecutor>,
    ) -> Result<EdgeFunctionExecution, EdgeFunctionError> {
        let mut db_callback_used = self.validate_invocation(request)?;
        let mut db_callback_statement_executed = false;
        let mut db_callback_rows = None;
        let mut status = InvocationStatus::Planned;

        if let Some(statement) = db_statement {
            let callback = self
                .plan
                .db_callback
                .as_ref()
                .ok_or(EdgeFunctionError::MissingRequiredField("db_callback"))?;
            let executor = db_executor.ok_or(EdgeFunctionError::DbCallbackExecutionDisabled)?;
            db_callback_rows = Some(executor.execute(callback, statement, request.timeout_ms)?);
            db_callback_used = true;
            db_callback_statement_executed = true;
            status = InvocationStatus::DbCallbackExecuted;
        }

        self.record_invocation(db_callback_used);
        Ok(self.render_invocation_execution(
            request,
            db_callback_used,
            db_callback_statement_executed,
            db_callback_rows,
            false,
            None,
            None,
            status,
            RuntimeExecutionMode::PlanOnly,
        ))
    }

    pub fn invoke_external_runtime(
        &mut self,
        request: &InvocationRequest,
        payload: &Value,
        runtime_executor: Option<&mut ExternalRuntimeExecutor>,
    ) -> Result<EdgeFunctionExecution, EdgeFunctionError> {
        let db_callback_used = self.validate_invocation(request)?;
        let executor = runtime_executor.ok_or(EdgeFunctionError::RuntimeExecutionDisabled)?;
        if db_callback_used {
            return Err(EdgeFunctionError::UnsupportedRuntimeSource);
        }
        let result = executor.execute(&self.plan, request, payload)?;
        let response_bytes = result.response_json.len() as u64;
        self.record_invocation(false);
        Ok(self.render_invocation_execution(
            request,
            false,
            false,
            None,
            true,
            Some(result.response_json),
            Some(response_bytes),
            InvocationStatus::Executed,
            RuntimeExecutionMode::Live,
        ))
    }

    fn validate_invocation(&self, request: &InvocationRequest) -> Result<bool, EdgeFunctionError> {
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
        Ok(self.plan.db_callback.is_some())
    }

    fn record_invocation(&mut self, db_callback_used: bool) {
        self.state.invocations += 1;
        if db_callback_used {
            self.state.db_callbacks += 1;
        }
    }
    #[allow(clippy::too_many_arguments)] // internal helper consumed by render_invocation; refactor planned with the EF6 alpha follow-up
    fn render_invocation_execution(
        &self,
        request: &InvocationRequest,
        db_callback_used: bool,
        db_callback_statement_executed: bool,
        db_callback_rows: Option<u64>,
        user_code_executed: bool,
        runtime_response_json: Option<String>,
        response_bytes: Option<u64>,
        status: InvocationStatus,
        execution_mode: RuntimeExecutionMode,
    ) -> EdgeFunctionExecution {
        let mut command = vec![self.launch.executable.clone()];
        command.extend(self.launch.args.clone());

        EdgeFunctionExecution {
            function_name: request.function_name.clone(),
            runtime: self.plan.runtime,
            command,
            trigger: request.trigger.clone(),
            tenant_id: request.tenant_id.clone(),
            payload_bytes: request.payload_bytes,
            response_bytes: response_bytes
                .unwrap_or_else(|| deterministic_response_bytes(request.payload_bytes)),
            db_callback_used,
            db_callback_statement_executed,
            db_callback_rows,
            user_code_executed,
            runtime_response_json,
            status,
            execution_mode,
        }
    }
}

fn deterministic_response_bytes(payload_bytes: u64) -> u64 {
    (payload_bytes / 2) + 64
}

#[derive(Debug)]
pub struct ExternalRuntimeExecutor {
    deno_bin: PathBuf,
    bun_bin: PathBuf,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ExternalRuntimeResult {
    pub response_json: String,
}

impl ExternalRuntimeExecutor {
    pub fn connect_from_env() -> Result<Option<Self>, EdgeFunctionError> {
        if !runtime_execution_enabled_from_env() {
            return Ok(None);
        }
        let deno_bin = env::var(EDGE_DENO_BIN_ENV).unwrap_or_else(|_| "deno".to_string());
        let bun_bin = env::var(EDGE_BUN_BIN_ENV).unwrap_or_else(|_| "bun".to_string());
        Ok(Some(Self {
            deno_bin: PathBuf::from(deno_bin),
            bun_bin: PathBuf::from(bun_bin),
        }))
    }

    pub fn execute(
        &mut self,
        plan: &EdgeFunctionPlan,
        request: &InvocationRequest,
        payload: &Value,
    ) -> Result<ExternalRuntimeResult, EdgeFunctionError> {
        plan.validate()?;
        let FunctionSource::Inline { code } = &plan.source else {
            return Err(EdgeFunctionError::UnsupportedRuntimeSource);
        };
        if plan.db_callback.is_some() {
            return Err(EdgeFunctionError::UnsupportedRuntimeSource);
        }

        let workdir = create_runtime_workdir(&plan.name)?;
        let result = match plan.runtime {
            EdgeFunctionRuntime::Deno => self.execute_deno_inline(&workdir, code, request, payload),
            EdgeFunctionRuntime::Bun => self.execute_bun_inline(&workdir, code, request, payload),
        };
        let _ = fs::remove_dir_all(&workdir);
        result
    }

    fn execute_deno_inline(
        &self,
        workdir: &Path,
        code: &str,
        request: &InvocationRequest,
        payload: &Value,
    ) -> Result<ExternalRuntimeResult, EdgeFunctionError> {
        let inline_path = workdir.join("inline.ts");
        let runner_path = workdir.join("runner.ts");
        fs::write(&inline_path, code)?;
        fs::write(&runner_path, deno_runner_source())?;

        let input = serde_json::json!({
            "function_name": request.function_name,
            "tenant_id": request.tenant_id,
            "payload_bytes": request.payload_bytes,
            "timeout_ms": request.timeout_ms,
            "payload": payload,
        })
        .to_string();

        let mut child = Command::new(&self.deno_bin)
            .arg("run")
            .arg("--no-prompt")
            .arg(&runner_path)
            .current_dir(workdir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| EdgeFunctionError::Runtime("Deno stdout pipe missing".to_string()))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| EdgeFunctionError::Runtime("Deno stderr pipe missing".to_string()))?;
        let stdout_reader = spawn_bounded_output_reader(stdout, MAX_RUNTIME_STDOUT_BYTES + 1);
        let stderr_reader = spawn_bounded_output_reader(stderr, 4096);
        if let Some(stdin) = child.stdin.as_mut() {
            stdin.write_all(input.as_bytes())?;
        }
        drop(child.stdin.take());

        let deadline = Instant::now() + Duration::from_millis(u64::from(request.timeout_ms));
        loop {
            if child.try_wait()?.is_some() {
                break;
            }
            if Instant::now() >= deadline {
                let _ = child.kill();
                let _ = child.wait();
                let _ = stdout_reader.join();
                let _ = stderr_reader.join();
                return Err(EdgeFunctionError::RuntimeTimedOut {
                    timeout_ms: request.timeout_ms,
                });
            }
            thread::sleep(Duration::from_millis(10));
        }

        let status = child.wait()?;
        let stdout = join_output_reader(stdout_reader)?;
        let stderr = join_output_reader(stderr_reader)?;
        if !status.success() {
            let stderr = String::from_utf8_lossy(&stderr.bytes);
            return Err(EdgeFunctionError::Runtime(format!(
                "Deno runtime exited with {}: {}",
                status,
                sanitize_runtime_stderr(&stderr)
            )));
        }
        if stdout.total_bytes > MAX_RUNTIME_STDOUT_BYTES {
            return Err(EdgeFunctionError::RuntimeOutputTooLarge {
                bytes: stdout.total_bytes,
                max_bytes: MAX_RUNTIME_STDOUT_BYTES,
            });
        }
        let response_json = String::from_utf8_lossy(&stdout.bytes).trim().to_string();
        if response_json.is_empty() || serde_json::from_str::<Value>(&response_json).is_err() {
            return Err(EdgeFunctionError::Runtime(
                "Deno runtime must write one JSON value to stdout".to_string(),
            ));
        }
        Ok(ExternalRuntimeResult { response_json })
    }

    fn execute_bun_inline(
        &self,
        workdir: &Path,
        code: &str,
        request: &InvocationRequest,
        payload: &Value,
    ) -> Result<ExternalRuntimeResult, EdgeFunctionError> {
        let inline_path = workdir.join("inline.ts");
        let runner_path = workdir.join("runner.ts");
        fs::write(&inline_path, code)?;
        fs::write(&runner_path, bun_runner_source())?;

        let input = serde_json::json!({
            "function_name": request.function_name,
            "tenant_id": request.tenant_id,
            "payload_bytes": request.payload_bytes,
            "timeout_ms": request.timeout_ms,
            "payload": payload,
        })
        .to_string();

        let mut command = Command::new(&self.bun_bin);
        command
            .arg(&runner_path)
            .current_dir(workdir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .env_clear();
        let mut child = command.spawn()?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| EdgeFunctionError::Runtime("Bun stdout pipe missing".to_string()))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| EdgeFunctionError::Runtime("Bun stderr pipe missing".to_string()))?;
        let stdout_reader = spawn_bounded_output_reader(stdout, MAX_RUNTIME_STDOUT_BYTES + 1);
        let stderr_reader = spawn_bounded_output_reader(stderr, 4096);
        if let Some(stdin) = child.stdin.as_mut() {
            stdin.write_all(input.as_bytes())?;
        }
        drop(child.stdin.take());

        let deadline = Instant::now() + Duration::from_millis(u64::from(request.timeout_ms));
        loop {
            if child.try_wait()?.is_some() {
                break;
            }
            if Instant::now() >= deadline {
                let _ = child.kill();
                let _ = child.wait();
                let _ = stdout_reader.join();
                let _ = stderr_reader.join();
                return Err(EdgeFunctionError::RuntimeTimedOut {
                    timeout_ms: request.timeout_ms,
                });
            }
            thread::sleep(Duration::from_millis(10));
        }

        let status = child.wait()?;
        let stdout = join_output_reader(stdout_reader)?;
        let stderr = join_output_reader(stderr_reader)?;
        if !status.success() {
            let stderr = String::from_utf8_lossy(&stderr.bytes);
            return Err(EdgeFunctionError::Runtime(format!(
                "Bun runtime exited with {}: {}",
                status,
                sanitize_runtime_stderr(&stderr)
            )));
        }
        if stdout.total_bytes > MAX_RUNTIME_STDOUT_BYTES {
            return Err(EdgeFunctionError::RuntimeOutputTooLarge {
                bytes: stdout.total_bytes,
                max_bytes: MAX_RUNTIME_STDOUT_BYTES,
            });
        }
        let response_json = String::from_utf8_lossy(&stdout.bytes).trim().to_string();
        if response_json.is_empty() || serde_json::from_str::<Value>(&response_json).is_err() {
            return Err(EdgeFunctionError::Runtime(
                "Bun runtime must write one JSON value to stdout".to_string(),
            ));
        }
        Ok(ExternalRuntimeResult { response_json })
    }
}

#[derive(Debug)]
struct BoundedOutput {
    bytes: Vec<u8>,
    total_bytes: usize,
}

fn spawn_bounded_output_reader<R>(
    mut reader: R,
    max_stored_bytes: usize,
) -> thread::JoinHandle<std::io::Result<BoundedOutput>>
where
    R: Read + Send + 'static,
{
    thread::spawn(move || {
        let mut bytes = Vec::new();
        let mut total_bytes = 0usize;
        let mut buffer = [0u8; 8192];
        loop {
            let read = reader.read(&mut buffer)?;
            if read == 0 {
                break;
            }
            total_bytes = total_bytes.saturating_add(read);
            if bytes.len() < max_stored_bytes {
                let remaining = max_stored_bytes - bytes.len();
                bytes.extend_from_slice(&buffer[..read.min(remaining)]);
            }
        }
        Ok(BoundedOutput { bytes, total_bytes })
    })
}

fn join_output_reader(
    reader: thread::JoinHandle<std::io::Result<BoundedOutput>>,
) -> Result<BoundedOutput, EdgeFunctionError> {
    reader
        .join()
        .map_err(|_| EdgeFunctionError::Runtime("runtime output reader panicked".to_string()))?
        .map_err(EdgeFunctionError::from)
}

fn runtime_execution_enabled_from_env() -> bool {
    env::var(EDGE_RUNTIME_EXECUTION_ENV)
        .map(|value| matches!(value.as_str(), "1" | "true" | "TRUE" | "yes" | "on"))
        .unwrap_or(false)
}

fn create_runtime_workdir(function_name: &str) -> Result<PathBuf, EdgeFunctionError> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| EdgeFunctionError::Runtime(error.to_string()))?
        .as_nanos();
    let dir = env::temp_dir().join(format!(
        "ai-blaise-edge-runtime-{}-{}-{now}",
        std::process::id(),
        function_name
    ));
    fs::create_dir(&dir)?;
    Ok(dir)
}

fn deno_runner_source() -> &'static str {
    r#"import handler from "./inline.ts";

const decoder = new TextDecoder();
const chunks = [];
for await (const chunk of Deno.stdin.readable) {
  chunks.push(chunk);
}
let total = 0;
for (const chunk of chunks) total += chunk.length;
const bytes = new Uint8Array(total);
let offset = 0;
for (const chunk of chunks) {
  bytes.set(chunk, offset);
  offset += chunk.length;
}
const input = JSON.parse(decoder.decode(bytes));
const result = await handler(input);
if (result instanceof Response) {
  const body = await result.text();
  console.log(JSON.stringify({status: result.status, body}));
} else {
  console.log(JSON.stringify(result ?? null));
}
"#
}

fn bun_runner_source() -> &'static str {
    r#"import handler from "./inline.ts";

const chunks = [];
for await (const chunk of process.stdin) {
  chunks.push(Buffer.isBuffer(chunk) ? chunk : Buffer.from(chunk));
}
const input = JSON.parse(Buffer.concat(chunks).toString("utf8"));
const result = await handler(input);
if (result instanceof Response) {
  const body = await result.text();
  console.log(JSON.stringify({status: result.status, body}));
} else {
  console.log(JSON.stringify(result ?? null));
}
"#
}

fn sanitize_runtime_stderr(stderr: &str) -> String {
    stderr
        .lines()
        .take(4)
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(512)
        .collect()
}

#[derive(Debug)]
pub struct DbCallbackExecutor;

impl DbCallbackExecutor {
    pub fn connect_from_env() -> Result<Option<Self>, EdgeFunctionError> {
        if !db_callback_execution_enabled_from_env() {
            return Ok(None);
        }
        Ok(Some(Self))
    }

    pub fn execute(
        &mut self,
        callback: &DbCallbackPlan,
        statement: &str,
        invocation_timeout_ms: u32,
    ) -> Result<u64, EdgeFunctionError> {
        callback.validate()?;
        if !is_safe_statement(statement) {
            return Err(EdgeFunctionError::UnsafeDbCallbackStatement);
        }
        let target = PostgresUdsTarget::parse(&callback.uds_path)?;
        let mut config = Config::new();
        config.user(&callback.role);
        config.dbname(&callback.database);
        config.host_path(&target.socket_dir);
        config.port(target.port);
        config.connect_timeout(Duration::from_millis(u64::from(
            invocation_timeout_ms.min(callback.statement_timeout_ms),
        )));

        let mut client = config.connect(NoTls)?;
        let mut transaction = client.transaction()?;
        let timeout_ms = invocation_timeout_ms.min(callback.statement_timeout_ms);
        transaction.batch_execute(&format!(
            "SET LOCAL statement_timeout = '{}ms'; SET LOCAL idle_in_transaction_session_timeout = '{}ms'",
            timeout_ms, timeout_ms
        ))?;
        let rows = count_simple_query_rows(transaction.simple_query(statement)?);
        transaction.commit()?;
        Ok(rows)
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct PostgresUdsTarget {
    socket_dir: PathBuf,
    port: u16,
}

impl PostgresUdsTarget {
    fn parse(uds_path: &str) -> Result<Self, EdgeFunctionError> {
        let path = Path::new(uds_path);
        let Some(file_name) = path.file_name().and_then(|value| value.to_str()) else {
            return Err(EdgeFunctionError::InvalidDbCallbackSocket);
        };
        let Some(port_text) = file_name.strip_prefix(".s.PGSQL.") else {
            return Err(EdgeFunctionError::InvalidDbCallbackSocket);
        };
        let port = port_text
            .parse::<u16>()
            .map_err(|_| EdgeFunctionError::InvalidDbCallbackSocket)?;
        let Some(socket_dir) = path.parent() else {
            return Err(EdgeFunctionError::InvalidDbCallbackSocket);
        };
        Ok(Self {
            socket_dir: socket_dir.to_path_buf(),
            port,
        })
    }
}

fn db_callback_execution_enabled_from_env() -> bool {
    env::var(EDGE_DB_CALLBACK_EXECUTION_ENV)
        .map(|value| matches!(value.as_str(), "1" | "true" | "TRUE" | "yes" | "on"))
        .unwrap_or(false)
}

fn count_simple_query_rows(messages: Vec<SimpleQueryMessage>) -> u64 {
    messages
        .into_iter()
        .map(|message| match message {
            SimpleQueryMessage::Row(_) => 1,
            SimpleQueryMessage::CommandComplete(rows) => rows,
            _ => 0,
        })
        .sum()
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
        self.invoke_with_db_callback(request, None, None)
    }

    pub fn invoke_with_db_callback(
        &mut self,
        request: &InvocationRequest,
        db_statement: Option<&str>,
        db_executor: Option<&mut DbCallbackExecutor>,
    ) -> Result<EdgeFunctionExecution, EdgeFunctionError> {
        let host = self
            .functions
            .get_mut(&request.function_name)
            .ok_or(EdgeFunctionError::FunctionNotFound)?;
        let execution = host.invoke_with_db_callback(request, db_statement, db_executor)?;
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

    pub fn invoke_external_runtime(
        &mut self,
        request: &InvocationRequest,
        payload: &Value,
        runtime_executor: Option<&mut ExternalRuntimeExecutor>,
    ) -> Result<EdgeFunctionExecution, EdgeFunctionError> {
        let host = self
            .functions
            .get_mut(&request.function_name)
            .ok_or(EdgeFunctionError::FunctionNotFound)?;
        let execution = host.invoke_external_runtime(request, payload, runtime_executor)?;
        self.invocations += 1;
        if matches!(
            request.trigger,
            FunctionTrigger::Scheduled { .. } | FunctionTrigger::CdcEvent { .. }
        ) {
            self.triggered_invocations += 1;
        }
        Ok(execution)
    }

    pub fn dispatch_due_schedules(
        &mut self,
        epoch_seconds: u64,
        envelope: &TriggerDispatchEnvelope,
        runtime_executor: Option<&mut ExternalRuntimeExecutor>,
    ) -> Result<Vec<EdgeFunctionExecution>, EdgeFunctionError> {
        let due = TriggerScheduler::new(self, epoch_seconds).due_schedules();
        let mut runtime_executor = runtime_executor;
        let mut executions = Vec::new();
        for tick in due {
            let request = InvocationRequest {
                function_name: tick.function_name,
                tenant_id: envelope.tenant_id.clone(),
                trigger: FunctionTrigger::Scheduled {
                    schedule: tick.schedule,
                },
                payload_bytes: envelope.payload_bytes,
                timeout_ms: envelope.timeout_ms,
            };
            let execution = match envelope.execution_mode {
                RuntimeExecutionMode::PlanOnly => self.invoke(&request)?,
                RuntimeExecutionMode::Live => self.invoke_external_runtime(
                    &request,
                    &envelope.payload,
                    runtime_executor.as_deref_mut(),
                )?,
            };
            executions.push(execution);
        }
        Ok(executions)
    }

    pub fn dispatch_cdc_event(
        &mut self,
        table: &str,
        operation: CdcOperation,
        envelope: &TriggerDispatchEnvelope,
        runtime_executor: Option<&mut ExternalRuntimeExecutor>,
    ) -> Result<Vec<EdgeFunctionExecution>, EdgeFunctionError> {
        let events = TriggerScheduler::new(self, 0).matching_events(table, &operation);
        let mut runtime_executor = runtime_executor;
        let mut executions = Vec::new();
        for event in events {
            let request = InvocationRequest {
                function_name: event.function_name,
                tenant_id: envelope.tenant_id.clone(),
                trigger: FunctionTrigger::CdcEvent {
                    table: event.table,
                    operation: event.operation,
                },
                payload_bytes: envelope.payload_bytes,
                timeout_ms: envelope.timeout_ms,
            };
            let execution = match envelope.execution_mode {
                RuntimeExecutionMode::PlanOnly => self.invoke(&request)?,
                RuntimeExecutionMode::Live => self.invoke_external_runtime(
                    &request,
                    &envelope.payload,
                    runtime_executor.as_deref_mut(),
                )?,
            };
            executions.push(execution);
        }
        Ok(executions)
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
        self.plan.runtime
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
            return Err(EdgeFunctionError::UnsafeDbCallbackStatement);
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
    let trimmed = lower.trim();
    if trimmed.is_empty()
        || trimmed.contains('\0')
        || trimmed.contains(';')
        || trimmed.contains("--")
        || trimmed.contains("/*")
        || trimmed.contains("*/")
    {
        return false;
    }
    let first_token = trimmed
        .split(|ch: char| ch.is_ascii_whitespace() || ch == '(')
        .next()
        .unwrap_or("");
    matches!(
        first_token,
        "select" | "insert" | "update" | "delete" | "with" | "call"
    )
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

fn cdc_operation_from_str(operation: &str) -> Result<CdcOperation, EdgeFunctionError> {
    match operation {
        "insert" => Ok(CdcOperation::Insert),
        "update" => Ok(CdcOperation::Update),
        "delete" => Ok(CdcOperation::Delete),
        "truncate" => Ok(CdcOperation::Truncate),
        _ => Err(EdgeFunctionError::InvalidIdentifier("cdc_operation")),
    }
}

// -----------------------------------------------------------------------------
// HTTP front door
// -----------------------------------------------------------------------------

pub fn handle_edge_functions_sidecar_http_bytes(
    request: &[u8],
) -> Result<HttpProbeResponse, EdgeFunctionError> {
    let mut runtime = SidecarRuntime::ready("edge-functions");
    let mut registry = canonical_edge_function_registry()?;
    handle_edge_functions_sidecar_http_request(request, &mut runtime, &mut registry, None, None)
}

fn handle_edge_functions_sidecar_http_request(
    request: &[u8],
    runtime: &mut SidecarRuntime,
    registry: &mut EdgeFunctionRegistry,
    db_executor: Option<&mut DbCallbackExecutor>,
    runtime_executor: Option<&mut ExternalRuntimeExecutor>,
) -> Result<HttpProbeResponse, EdgeFunctionError> {
    let request =
        std::str::from_utf8(request).map_err(|_| EdgeFunctionError::MalformedHttpRequest)?;
    let (method, path, body) = parse_http_request(request)?;

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
                Err(error) => Ok(error_response(error)),
            },
            Err(error) => Ok(error_response(error)),
        };
    }
    if method == "POST" && path == "/triggers/scheduled" {
        let envelope = match trigger_dispatch_envelope_from_body(body) {
            Ok(envelope) => envelope,
            Err(error) => return Ok(error_response(error)),
        };
        let epoch_seconds = match trigger_epoch_seconds_from_body(body) {
            Ok(epoch_seconds) => epoch_seconds,
            Err(error) => return Ok(error_response(error)),
        };
        return match registry.dispatch_due_schedules(epoch_seconds, &envelope, runtime_executor) {
            Ok(executions) => Ok(HttpProbeResponse::new(
                200,
                "application/json",
                render_trigger_dispatch("scheduled", executions),
            )),
            Err(error) => Ok(error_response(error)),
        };
    }
    if method == "POST" && path == "/triggers/cdc" {
        let envelope = match trigger_dispatch_envelope_from_body(body) {
            Ok(envelope) => envelope,
            Err(error) => return Ok(error_response(error)),
        };
        let (table, operation) = match cdc_trigger_from_body(body) {
            Ok(trigger) => trigger,
            Err(error) => return Ok(error_response(error)),
        };
        return match registry.dispatch_cdc_event(&table, operation, &envelope, runtime_executor) {
            Ok(executions) => Ok(HttpProbeResponse::new(
                200,
                "application/json",
                render_trigger_dispatch("cdc", executions),
            )),
            Err(error) => Ok(error_response(error)),
        };
    }
    if method == "POST" && path.starts_with("/functions/") {
        let name = path.trim_start_matches("/functions/");
        let envelope = match invocation_envelope_for_registered(registry, name, body) {
            Ok(envelope) => envelope,
            Err(error) => return Ok(error_response(error)),
        };
        if envelope.execution_mode == RuntimeExecutionMode::Live {
            return match registry.invoke_external_runtime(
                &envelope.request,
                &envelope.payload,
                runtime_executor,
            ) {
                Ok(execution) => Ok(HttpProbeResponse::new(
                    200,
                    "application/json",
                    render_execution(&execution),
                )),
                Err(error) => Ok(error_response(error)),
            };
        }
        let db_statement = match db_statement_from_body(body) {
            Ok(statement) => statement,
            Err(error) => return Ok(error_response(error)),
        };
        return match registry.invoke_with_db_callback(
            &envelope.request,
            db_statement.as_deref(),
            db_executor,
        ) {
            Ok(execution) => Ok(HttpProbeResponse::new(
                200,
                "application/json",
                render_execution(&execution),
            )),
            Err(error) => Ok(error_response(error)),
        };
    }

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

fn render_trigger_dispatch(kind: &str, executions: Vec<EdgeFunctionExecution>) -> String {
    let rendered = executions
        .iter()
        .map(|execution| render_execution(execution).trim().to_string())
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "{{\"trigger\":\"{}\",\"matched\":{},\"dispatched\":{},\"executions\":[{}]}}\n",
        escape_json(kind),
        executions.len(),
        executions.len(),
        rendered,
    )
}

fn render_execution(execution: &EdgeFunctionExecution) -> String {
    let db_callback_rows = execution
        .db_callback_rows
        .map(|rows| rows.to_string())
        .unwrap_or_else(|| "null".to_string());
    let runtime_response_json = execution
        .runtime_response_json
        .as_deref()
        .map(|value| format!("\"{}\"", escape_json(value)))
        .unwrap_or_else(|| "null".to_string());
    format!(
        "{{\"function\":\"{}\",\"runtime\":\"{}\",\"trigger\":\"{}\",\"tenant_id\":\"{}\",\"payload_bytes\":{},\"response_bytes\":{},\"db_callback_used\":{},\"db_callback_statement_executed\":{},\"db_callback_rows\":{},\"user_code_executed\":{},\"runtime_response_json\":{},\"status\":\"{}\",\"execution_mode\":\"{}\"}}\n",
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
        execution.db_callback_statement_executed,
        db_callback_rows,
        execution.user_code_executed,
        runtime_response_json,
        match execution.status {
            InvocationStatus::Planned => "planned",
            InvocationStatus::DbCallbackExecuted => "db_callback_executed",
            InvocationStatus::Executed => "executed",
        },
        match execution.execution_mode {
            RuntimeExecutionMode::PlanOnly => "plan_only",
            RuntimeExecutionMode::Live => "live",
        },
    )
}

fn register_plan_from_body(body: &str) -> Result<EdgeFunctionPlan, EdgeFunctionError> {
    let json = parse_json_body(body)?;
    let name =
        json_string_field(&json, "name").ok_or(EdgeFunctionError::MissingRequiredField("name"))?;
    let runtime_str = json_string_field(&json, "runtime").unwrap_or_else(|| "deno".to_string());
    let runtime = match runtime_str.as_str() {
        "deno" => EdgeFunctionRuntime::Deno,
        "bun" => EdgeFunctionRuntime::Bun,
        _ => {
            return Err(EdgeFunctionError::InvalidIdentifier("runtime"));
        }
    };
    let code =
        json_string_field(&json, "code").ok_or(EdgeFunctionError::MissingRequiredField("code"))?;
    let trigger_path = json_string_field(&json, "http_path");
    let schedule = json_string_field(&json, "schedule");
    let cdc_table = json_string_field(&json, "cdc_table");
    let cdc_operation = match json_string_field(&json, "cdc_operation") {
        Some(operation) => Some(cdc_operation_from_str(&operation)?),
        None => None,
    };
    let env_secret_refs = json_string_array_field(&json, "env_secret_refs")?;
    let db_callback = match json_string_field(&json, "db_callback_socket") {
        Some(uds_path) => Some(DbCallbackPlan {
            uds_path,
            database: json_string_field(&json, "db_callback_database")
                .unwrap_or_else(|| "app".to_string()),
            role: json_string_field(&json, "db_callback_role")
                .unwrap_or_else(|| "edge_runtime".to_string()),
            statement_timeout_ms: json_u32_field(&json, "db_callback_statement_timeout_ms")?
                .unwrap_or(1_000),
        }),
        None => None,
    };

    let mut triggers = Vec::new();
    if let Some(path) = trigger_path {
        triggers.push(FunctionTrigger::Http { path });
    }
    if let Some(schedule) = schedule {
        triggers.push(FunctionTrigger::Scheduled { schedule });
    }
    match (cdc_table, cdc_operation) {
        (Some(table), Some(operation)) => {
            triggers.push(FunctionTrigger::CdcEvent { table, operation })
        }
        (None, None) => {}
        (Some(_), None) => return Err(EdgeFunctionError::MissingRequiredField("cdc_operation")),
        (None, Some(_)) => return Err(EdgeFunctionError::MissingRequiredField("cdc_table")),
    }
    if triggers.is_empty() {
        triggers.push(FunctionTrigger::Http {
            path: "/".to_string(),
        });
    }

    let plan = EdgeFunctionPlan {
        name,
        runtime,
        source: FunctionSource::Inline { code },
        triggers,
        env_secret_refs,
        db_callback,
    };
    plan.validate()?;
    Ok(plan)
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct InvocationEnvelope {
    request: InvocationRequest,
    execution_mode: RuntimeExecutionMode,
    payload: Value,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TriggerDispatchEnvelope {
    pub execution_mode: RuntimeExecutionMode,
    pub tenant_id: String,
    pub payload_bytes: u64,
    pub timeout_ms: u32,
    pub payload: Value,
}

fn invocation_envelope_for_registered(
    registry: &EdgeFunctionRegistry,
    name: &str,
    body: &str,
) -> Result<InvocationEnvelope, EdgeFunctionError> {
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
    let json = parse_json_body(body)?;
    let execution_mode = match json_string_field(&json, "execution_mode")
        .unwrap_or_else(|| "plan_only".to_string())
        .as_str()
    {
        "plan_only" => RuntimeExecutionMode::PlanOnly,
        "live" => RuntimeExecutionMode::Live,
        _ => return Err(EdgeFunctionError::UnsupportedExecutionMode),
    };
    let tenant_id = json_string_field(&json, "tenant_id").unwrap_or_else(|| "tenant-a".to_string());
    let payload_bytes = json_u64_field(&json, "payload_bytes")?.unwrap_or(64);
    let timeout_ms = json_u32_field(&json, "timeout_ms")?.unwrap_or(500);
    let payload = json.get("payload").cloned().unwrap_or(Value::Null);
    Ok(InvocationEnvelope {
        request: InvocationRequest {
            function_name: name.to_string(),
            tenant_id,
            trigger,
            payload_bytes,
            timeout_ms,
        },
        execution_mode,
        payload,
    })
}

fn trigger_dispatch_envelope_from_body(
    body: &str,
) -> Result<TriggerDispatchEnvelope, EdgeFunctionError> {
    let json = parse_json_body(body)?;
    let execution_mode = match json_string_field(&json, "execution_mode")
        .unwrap_or_else(|| "plan_only".to_string())
        .as_str()
    {
        "plan_only" => RuntimeExecutionMode::PlanOnly,
        "live" => RuntimeExecutionMode::Live,
        _ => return Err(EdgeFunctionError::UnsupportedExecutionMode),
    };
    let tenant_id = json_string_field(&json, "tenant_id").unwrap_or_else(|| "tenant-a".to_string());
    let payload_bytes = json_u64_field(&json, "payload_bytes")?.unwrap_or(64);
    let timeout_ms = json_u32_field(&json, "timeout_ms")?.unwrap_or(500);
    let request = InvocationRequest {
        function_name: "trigger_validation".to_string(),
        tenant_id: tenant_id.clone(),
        trigger: FunctionTrigger::Http {
            path: "/trigger-validation".to_string(),
        },
        payload_bytes,
        timeout_ms,
    };
    request.validate()?;
    Ok(TriggerDispatchEnvelope {
        execution_mode,
        tenant_id,
        payload_bytes,
        timeout_ms,
        payload: json.get("payload").cloned().unwrap_or(Value::Null),
    })
}

fn trigger_epoch_seconds_from_body(body: &str) -> Result<u64, EdgeFunctionError> {
    let json = parse_json_body(body)?;
    Ok(json_u64_field(&json, "epoch_seconds")?.unwrap_or_else(current_epoch_seconds))
}

fn cdc_trigger_from_body(body: &str) -> Result<(String, CdcOperation), EdgeFunctionError> {
    let json = parse_json_body(body)?;
    let table = json_string_field(&json, "table")
        .ok_or(EdgeFunctionError::MissingRequiredField("table"))?;
    validate_qualified_name("table", &table)?;
    let operation = json_string_field(&json, "operation")
        .ok_or(EdgeFunctionError::MissingRequiredField("operation"))?;
    Ok((table, cdc_operation_from_str(&operation)?))
}

fn current_epoch_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn parse_json_body(body: &str) -> Result<Value, EdgeFunctionError> {
    let trimmed = body.trim();
    if trimmed.is_empty() {
        return Ok(Value::Object(Default::default()));
    }
    serde_json::from_str(trimmed).map_err(|_| EdgeFunctionError::MalformedHttpRequest)
}

fn json_string_field(value: &Value, field: &str) -> Option<String> {
    value.get(field).and_then(Value::as_str).map(str::to_string)
}

fn json_u64_field(value: &Value, field: &str) -> Result<Option<u64>, EdgeFunctionError> {
    match value.get(field) {
        None => Ok(None),
        Some(raw) => raw
            .as_u64()
            .map(Some)
            .ok_or(EdgeFunctionError::MalformedHttpRequest),
    }
}

fn json_u32_field(value: &Value, field: &str) -> Result<Option<u32>, EdgeFunctionError> {
    match json_u64_field(value, field)? {
        None => Ok(None),
        Some(raw) => u32::try_from(raw)
            .map(Some)
            .map_err(|_| EdgeFunctionError::MalformedHttpRequest),
    }
}

fn json_string_array_field(value: &Value, field: &str) -> Result<Vec<String>, EdgeFunctionError> {
    match value.get(field) {
        None => Ok(Vec::new()),
        Some(Value::Array(values)) => values
            .iter()
            .map(|value| {
                value
                    .as_str()
                    .map(str::to_string)
                    .ok_or(EdgeFunctionError::MalformedHttpRequest)
            })
            .collect(),
        Some(_) => Err(EdgeFunctionError::MalformedHttpRequest),
    }
}

fn db_statement_from_body(body: &str) -> Result<Option<String>, EdgeFunctionError> {
    let json = parse_json_body(body)?;
    Ok(json_string_field(&json, "db_statement"))
}

fn error_response(error: EdgeFunctionError) -> HttpProbeResponse {
    let status_code = http_status_for_error(&error);
    HttpProbeResponse::new(
        status_code,
        "application/json",
        format!("{{\"error\":\"{}\"}}\n", escape_json(&error.to_string())),
    )
}
fn http_status_for_error(error: &EdgeFunctionError) -> u16 {
    match error {
        EdgeFunctionError::DbCallbackExecutionDisabled => 501,
        EdgeFunctionError::FunctionNotFound => 404,
        EdgeFunctionError::RuntimeExecutionDisabled => 501,
        EdgeFunctionError::RuntimeTimedOut { .. } => 504,
        EdgeFunctionError::UnsupportedRuntimeSource => 501,
        EdgeFunctionError::UnsupportedExecutionMode => 501,
        _ => 400,
    }
}

pub fn serve_edge_functions_sidecar_http_forever(
    default_addr: &str,
) -> Result<(), EdgeFunctionError> {
    use std::net::TcpListener;

    let mut registry = canonical_edge_function_registry()?;
    let mut runtime = SidecarRuntime::ready("edge-functions");
    let mut db_executor = DbCallbackExecutor::connect_from_env()?;
    let mut runtime_executor = ExternalRuntimeExecutor::connect_from_env()?;
    let listen_addr = listen_addr_from_env(default_addr)?;
    let listener = TcpListener::bind(&listen_addr)?;
    eprintln!("ai-blaise edge-functions sidecar listening on {listen_addr}");

    for stream in listener.incoming() {
        let mut stream = stream?;
        let request = read_http_request(&mut stream)?;
        let response = handle_edge_functions_sidecar_http_request(
            &request,
            &mut runtime,
            &mut registry,
            db_executor.as_mut(),
            runtime_executor.as_mut(),
        )
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

impl From<postgres::Error> for EdgeFunctionError {
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
                "--no-prompt".to_string(),
                "--allow-env".to_string(),
                "--allow-net=unix".to_string(),
                "inline.ts".to_string(),
            ]
        );
        assert_eq!(report.execution.response_bytes, 320);
        assert!(report.execution.db_callback_used);
        assert_eq!(report.execution.status, InvocationStatus::Planned);
        assert_eq!(
            report.execution.execution_mode,
            RuntimeExecutionMode::PlanOnly
        );
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
            Err(EdgeFunctionError::UnsafeDbCallbackStatement)
        );
    }

    #[test]
    fn postgres_uds_target_parses_socket_path() {
        let target = PostgresUdsTarget::parse("/tmp/edge/.s.PGSQL.5432").expect("target");
        assert_eq!(target.socket_dir, PathBuf::from("/tmp/edge"));
        assert_eq!(target.port, 5432);
        assert_eq!(
            PostgresUdsTarget::parse("/tmp/edge/postgresql.sock"),
            Err(EdgeFunctionError::InvalidDbCallbackSocket)
        );
    }

    #[test]
    fn db_callback_statement_guard_rejects_multi_statement_input() {
        assert!(is_safe_statement(
            "insert into edge_callback_events(tenant_id) values ('tenant-a')"
        ));
        assert!(!is_safe_statement(
            "select 1; drop table edge_callback_events"
        ));
        assert!(!is_safe_statement("select 1 -- comment"));
        assert!(!is_safe_statement(
            "alter table edge_callback_events add column x int"
        ));
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
    fn http_front_door_dispatches_scheduled_and_cdc_triggers() {
        let mut runtime = SidecarRuntime::ready("edge-functions");
        let mut registry = EdgeFunctionRegistry::new();

        let scheduled = r#"{"name":"scheduled_report","runtime":"deno","code":"export default async () => ({ok:true})","schedule":"*/5 * * * *"}"#;
        let request = format!(
            "POST /functions HTTP/1.1\r\ncontent-type: application/json\r\ncontent-length: {}\r\n\r\n{}",
            scheduled.len(),
            scheduled
        );
        let response = handle_edge_functions_sidecar_http_request(
            request.as_bytes(),
            &mut runtime,
            &mut registry,
            None,
            None,
        )
        .expect("register scheduled");
        assert_eq!(response.status_code, 201);

        let cdc = r#"{"name":"order_event","runtime":"deno","code":"export default async () => ({ok:true})","cdc_table":"public.orders","cdc_operation":"insert"}"#;
        let request = format!(
            "POST /functions HTTP/1.1\r\ncontent-type: application/json\r\ncontent-length: {}\r\n\r\n{}",
            cdc.len(),
            cdc
        );
        let response = handle_edge_functions_sidecar_http_request(
            request.as_bytes(),
            &mut runtime,
            &mut registry,
            None,
            None,
        )
        .expect("register cdc");
        assert_eq!(response.status_code, 201);

        let scheduled_dispatch =
            r#"{"epoch_seconds":0,"tenant_id":"tenant-a","payload_bytes":64,"timeout_ms":250}"#;
        let request = format!(
            "POST /triggers/scheduled HTTP/1.1\r\ncontent-type: application/json\r\ncontent-length: {}\r\n\r\n{}",
            scheduled_dispatch.len(),
            scheduled_dispatch
        );
        let response = handle_edge_functions_sidecar_http_request(
            request.as_bytes(),
            &mut runtime,
            &mut registry,
            None,
            None,
        )
        .expect("dispatch scheduled");
        assert_eq!(response.status_code, 200);
        assert!(response.body.contains("\"trigger\":\"scheduled\""));
        assert!(response.body.contains("\"dispatched\":1"));
        assert!(response.body.contains("\"function\":\"scheduled_report\""));
        assert!(response.body.contains("\"execution_mode\":\"plan_only\""));

        let cdc_dispatch = r#"{"table":"public.orders","operation":"insert","tenant_id":"tenant-a","payload_bytes":64,"timeout_ms":250}"#;
        let request = format!(
            "POST /triggers/cdc HTTP/1.1\r\ncontent-type: application/json\r\ncontent-length: {}\r\n\r\n{}",
            cdc_dispatch.len(),
            cdc_dispatch
        );
        let response = handle_edge_functions_sidecar_http_request(
            request.as_bytes(),
            &mut runtime,
            &mut registry,
            None,
            None,
        )
        .expect("dispatch cdc");
        assert_eq!(response.status_code, 200);
        assert!(response.body.contains("\"trigger\":\"cdc\""));
        assert!(response.body.contains("\"dispatched\":1"));
        assert!(response.body.contains("\"function\":\"order_event\""));
        assert_eq!(registry.snapshot().invocations, 2);
        assert_eq!(registry.snapshot().triggered_invocations, 2);
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
        assert!(response.body.contains("\"status\":\"planned\""));
        assert!(response.body.contains("\"execution_mode\":\"plan_only\""));
        assert!(response
            .body
            .contains("\"db_callback_statement_executed\":false"));
    }

    #[test]
    fn http_front_door_rejects_db_callback_statement_without_live_executor() {
        let body = r#"{"tenant_id":"tenant-a","payload_bytes":256,"timeout_ms":250,"db_statement":"select 1"}"#;
        let request = format!(
            "POST /functions/order_created HTTP/1.1\r\ncontent-type: application/json\r\ncontent-length: {}\r\n\r\n{}",
            body.len(),
            body
        );
        let response =
            handle_edge_functions_sidecar_http_bytes(request.as_bytes()).expect("invoke");
        assert_eq!(response.status_code, 501);
        assert!(response.body.contains(EDGE_DB_CALLBACK_EXECUTION_ENV));
    }

    #[test]
    fn function_plan_rejects_unsafe_runtime_inputs() {
        let mut plan = canonical_edge_function_plan();
        plan.source = FunctionSource::Inline {
            code: "x".repeat(MAX_INLINE_SOURCE_BYTES + 1),
        };
        assert_eq!(
            plan.validate(),
            Err(EdgeFunctionError::SourceTooLarge {
                bytes: MAX_INLINE_SOURCE_BYTES + 1,
                max_bytes: MAX_INLINE_SOURCE_BYTES,
            })
        );

        plan.source = FunctionSource::Inline {
            code: "bad\0source".to_string(),
        };
        assert_eq!(
            plan.validate(),
            Err(EdgeFunctionError::InvalidSource("source.inline.code"))
        );

        let plan = EdgeFunctionPlan {
            name: "bad_entry".to_string(),
            runtime: EdgeFunctionRuntime::Deno,
            source: FunctionSource::BundleUri {
                uri: "s3://bucket/function.tgz".to_string(),
                entrypoint: "../index.ts".to_string(),
            },
            triggers: vec![FunctionTrigger::Http {
                path: "/bad".to_string(),
            }],
            env_secret_refs: Vec::new(),
            db_callback: None,
        };
        assert_eq!(
            plan.validate(),
            Err(EdgeFunctionError::InvalidEntrypoint("source.entrypoint"))
        );
    }

    #[test]
    fn function_plan_rejects_invalid_env_secret_refs() {
        let mut plan = canonical_edge_function_plan();
        plan.env_secret_refs = vec!["Uppercase-secret".to_string()];
        assert_eq!(plan.validate(), Err(EdgeFunctionError::InvalidSecretRef));

        plan.env_secret_refs = (0..=MAX_SECRET_REFS)
            .map(|index| format!("secret-{index}"))
            .collect();
        assert_eq!(
            plan.validate(),
            Err(EdgeFunctionError::TooManySecretRefs {
                count: MAX_SECRET_REFS + 1,
                max_count: MAX_SECRET_REFS,
            })
        );
    }

    #[test]
    fn invocation_request_enforces_size_and_timeout_bounds() {
        let mut request = canonical_invocation_request();
        request.payload_bytes = MAX_INVOCATION_PAYLOAD_BYTES + 1;
        assert_eq!(
            request.validate(),
            Err(EdgeFunctionError::PayloadTooLarge {
                bytes: MAX_INVOCATION_PAYLOAD_BYTES + 1,
                max_bytes: MAX_INVOCATION_PAYLOAD_BYTES,
            })
        );

        request = canonical_invocation_request();
        request.timeout_ms = MAX_INVOCATION_TIMEOUT_MS + 1;
        assert_eq!(
            request.validate(),
            Err(EdgeFunctionError::InvocationTimeoutTooLarge {
                timeout_ms: MAX_INVOCATION_TIMEOUT_MS + 1,
                max_timeout_ms: MAX_INVOCATION_TIMEOUT_MS,
            })
        );
    }

    #[test]
    fn external_runtime_execution_fails_closed() {
        let mut runtime =
            EdgeFunctionRuntimeHost::new(canonical_edge_function_plan()).expect("runtime");
        assert_eq!(
            runtime.invoke_external_runtime(&canonical_invocation_request(), &Value::Null, None),
            Err(EdgeFunctionError::RuntimeExecutionDisabled)
        );
    }

    #[test]
    fn http_front_door_rejects_unsupported_live_execution() {
        let body = r#"{"tenant_id":"tenant-a","payload_bytes":256,"timeout_ms":250,"execution_mode":"live"}"#;
        let request = format!(
            "POST /functions/order_created HTTP/1.1
content-type: application/json
content-length: {}

{}",
            body.len(),
            body
        );
        let response =
            handle_edge_functions_sidecar_http_bytes(request.as_bytes()).expect("invoke");
        assert_eq!(response.status_code, 501);
        assert!(response.body.contains("AI_BLAISE_EDGE_RUNTIME_EXECUTION"));
    }

    #[test]
    fn http_front_door_rejects_invalid_registration_and_invocation_bounds() {
        let bad_register = r#"{"name":"bad","runtime":"deno","code":"ok","http_path":"admin"}"#;
        let request = format!(
            "POST /functions HTTP/1.1
content-type: application/json
content-length: {}

{}",
            bad_register.len(),
            bad_register
        );
        let response =
            handle_edge_functions_sidecar_http_bytes(request.as_bytes()).expect("register");
        assert_eq!(response.status_code, 400);
        assert!(response.body.contains("HTTP trigger path"));

        let bad_invoke = r#"{"tenant_id":"tenant-a","payload_bytes":1048577,"timeout_ms":250}"#;
        let request = format!(
            "POST /functions/order_created HTTP/1.1
content-type: application/json
content-length: {}

{}",
            bad_invoke.len(),
            bad_invoke
        );
        let response =
            handle_edge_functions_sidecar_http_bytes(request.as_bytes()).expect("invoke");
        assert_eq!(response.status_code, 400);
        assert!(response.body.contains("payload_bytes"));
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
