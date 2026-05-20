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
    InvalidHttpPath,
    InvalidIdentifier(&'static str),
    InvalidInvocationTimeout,
    InvalidObjectUri(&'static str),
    InvalidPayloadSize,
    InvalidStatementTimeout,
    InvalidUdsPath,
    InvalidUrl(&'static str),
    MissingRequiredField(&'static str),
}

impl fmt::Display for EdgeFunctionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
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
            Self::MissingRequiredField(field) => write!(formatter, "{field} must not be empty"),
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
}
