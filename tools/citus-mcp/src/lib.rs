//! Safe Model Context Protocol contracts for ai-blaise/citus operations.

// FEATURE: MCP1
// FEATURE: MCP2
// FEATURE: MCP3
// FEATURE: D11

use std::error::Error;
use std::fmt;

use serde_json::{json, Map, Value};

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct McpToolRequest {
    pub tool: McpTool,
    pub tenant_scope: Option<TenantScope>,
    pub safe_mode: SafeMode,
}

impl McpToolRequest {
    pub fn validate(&self) -> Result<(), McpToolError> {
        self.tool.validate()?;
        if let Some(scope) = &self.tenant_scope {
            scope.validate()?;
        }
        if self.safe_mode == SafeMode::Required && self.tool.is_destructive() {
            return Err(McpToolError::UnsafeToolDenied);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum McpTool {
    ListShards,
    ListHypertables,
    RunExplain { sql: String },
    RebalanceDryRun { shard_group: String },
    SuggestIndex { table: String },
    QueryWithTimeout { sql: String, timeout_ms: u32 },
    CurrentLag,
    CurrentReplicationStatus,
    TenantArchive { tenant_name: String },
}

impl McpTool {
    fn validate(&self) -> Result<(), McpToolError> {
        match self {
            Self::RunExplain { sql } | Self::QueryWithTimeout { sql, .. } => {
                validate_required("sql", sql)?;
            }
            Self::RebalanceDryRun { shard_group } => {
                validate_required("shard_group", shard_group)?;
            }
            Self::SuggestIndex { table } => {
                validate_required("table", table)?;
            }
            Self::TenantArchive { tenant_name } => {
                validate_required("tenant_name", tenant_name)?;
            }
            _ => {}
        }

        if let Self::QueryWithTimeout { timeout_ms, .. } = self {
            if *timeout_ms == 0 {
                return Err(McpToolError::InvalidTimeout);
            }
        }

        Ok(())
    }

    fn is_destructive(&self) -> bool {
        matches!(self, Self::TenantArchive { .. })
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TenantScope {
    pub tenant_id: String,
    pub allowed_schemas: Vec<String>,
}

impl TenantScope {
    fn validate(&self) -> Result<(), McpToolError> {
        validate_required("tenant_id", &self.tenant_id)?;
        if self.allowed_schemas.is_empty()
            || self
                .allowed_schemas
                .iter()
                .any(|schema| schema.trim().is_empty())
        {
            return Err(McpToolError::MissingRequiredField("allowed_schemas"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum SafeMode {
    Required,
    Disabled,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum McpToolError {
    InvalidTimeout,
    MissingRequiredField(&'static str),
    UnsafeToolDenied,
}

impl fmt::Display for McpToolError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidTimeout => write!(formatter, "timeout_ms must be greater than zero"),
            Self::MissingRequiredField(field) => {
                write!(formatter, "{field} must not be empty")
            }
            Self::UnsafeToolDenied => write!(formatter, "safe mode denied a destructive tool"),
        }
    }
}

impl Error for McpToolError {}

fn validate_required(field: &'static str, value: &str) -> Result<(), McpToolError> {
    if value.trim().is_empty() {
        return Err(McpToolError::MissingRequiredField(field));
    }
    Ok(())
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct McpExecutionReport {
    pub requests: usize,
    pub tenant_scoped_requests: usize,
    pub safe_mode_required: usize,
    pub destructive_denials: usize,
}

pub fn canonical_mcp_requests() -> Vec<McpToolRequest> {
    vec![
        McpToolRequest {
            tool: McpTool::ListShards,
            tenant_scope: Some(canonical_tenant_scope()),
            safe_mode: SafeMode::Required,
        },
        McpToolRequest {
            tool: McpTool::QueryWithTimeout {
                sql: "SELECT count(*) FROM tenant_a.orders".to_string(),
                timeout_ms: 1_000,
            },
            tenant_scope: Some(canonical_tenant_scope()),
            safe_mode: SafeMode::Required,
        },
        McpToolRequest {
            tool: McpTool::RebalanceDryRun {
                shard_group: "tenant".to_string(),
            },
            tenant_scope: None,
            safe_mode: SafeMode::Required,
        },
    ]
}

pub fn canonical_mcp_execution_report() -> Result<McpExecutionReport, McpToolError> {
    let requests = canonical_mcp_requests();
    for request in &requests {
        request.validate()?;
    }

    let denied_request = McpToolRequest {
        tool: McpTool::TenantArchive {
            tenant_name: "tenant-a".to_string(),
        },
        tenant_scope: Some(canonical_tenant_scope()),
        safe_mode: SafeMode::Required,
    };
    let destructive_denials =
        usize::from(denied_request.validate() == Err(McpToolError::UnsafeToolDenied));

    Ok(McpExecutionReport {
        requests: requests.len(),
        tenant_scoped_requests: requests
            .iter()
            .filter(|request| request.tenant_scope.is_some())
            .count(),
        safe_mode_required: requests
            .iter()
            .filter(|request| request.safe_mode == SafeMode::Required)
            .count(),
        destructive_denials,
    })
}

pub fn handle_mcp_stdio_request(line: &str) -> String {
    handle_mcp_stdio_request_with_server_info(line, "ai-blaise-citus-mcp")
}

pub fn handle_mcp_stdio_request_with_server_info(line: &str, server_name: &str) -> String {
    let response = match serde_json::from_str::<Value>(line) {
        Ok(request) => handle_jsonrpc_request(&request, server_name),
        Err(error) => jsonrpc_error(Value::Null, -32700, format!("parse error: {error}")),
    };

    response.to_string()
}

fn canonical_tenant_scope() -> TenantScope {
    TenantScope {
        tenant_id: "tenant-a".to_string(),
        allowed_schemas: vec!["tenant_a".to_string()],
    }
}

fn handle_jsonrpc_request(request: &Value, server_name: &str) -> Value {
    let id = request.get("id").cloned().unwrap_or(Value::Null);
    let Some(method) = request.get("method").and_then(Value::as_str) else {
        return jsonrpc_error(id, -32600, "method must be a string");
    };

    match method {
        "initialize" => jsonrpc_result(
            id,
            json!({
                "protocolVersion": "2024-11-05",
                "serverInfo": {
                    "name": server_name,
                    "version": env!("CARGO_PKG_VERSION"),
                },
                "capabilities": {
                    "tools": {
                        "listChanged": false,
                    },
                },
            }),
        ),
        "tools/list" => jsonrpc_result(id, json!({ "tools": mcp_tool_descriptors() })),
        "tools/call" => handle_tools_call(id, request.get("params")),
        _ => jsonrpc_error(id, -32601, format!("unknown method: {method}")),
    }
}

fn handle_tools_call(id: Value, params: Option<&Value>) -> Value {
    let Some(params) = params.and_then(Value::as_object) else {
        return jsonrpc_error(id, -32602, "tools/call params must be an object");
    };
    let Some(name) = params.get("name").and_then(Value::as_str) else {
        return jsonrpc_error(id, -32602, "tools/call name must be a string");
    };
    let arguments = params
        .get("arguments")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();

    match mcp_request_from_call(name, &arguments).and_then(validate_tenant_policy) {
        Ok(request) => jsonrpc_result(
            id,
            json!({
                "content": [{
                    "type": "text",
                    "text": format!(
                        "accepted {} safe_mode={} tenant_scope={}",
                        name,
                        safe_mode_label(request.safe_mode),
                        request.tenant_scope.as_ref().map(|scope| scope.tenant_id.as_str()).unwrap_or("none"),
                    ),
                }],
                "isError": false,
            }),
        ),
        Err(message) => jsonrpc_result(
            id,
            json!({
                "content": [{
                    "type": "text",
                    "text": message,
                }],
                "isError": true,
            }),
        ),
    }
}

fn mcp_request_from_call(
    name: &str,
    arguments: &Map<String, Value>,
) -> Result<McpToolRequest, String> {
    let tool = match name {
        "list_shards" => McpTool::ListShards,
        "list_hypertables" => McpTool::ListHypertables,
        "run_explain" => McpTool::RunExplain {
            sql: required_string(arguments, "sql")?,
        },
        "rebalance_dry_run" => McpTool::RebalanceDryRun {
            shard_group: required_string(arguments, "shard_group")?,
        },
        "suggest_index" => McpTool::SuggestIndex {
            table: required_string(arguments, "table")?,
        },
        "query_with_timeout" => McpTool::QueryWithTimeout {
            sql: required_string(arguments, "sql")?,
            timeout_ms: required_u32(arguments, "timeout_ms")?,
        },
        "current_lag" => McpTool::CurrentLag,
        "current_replication_status" => McpTool::CurrentReplicationStatus,
        "tenant_archive" => McpTool::TenantArchive {
            tenant_name: required_string(arguments, "tenant_name")?,
        },
        _ => return Err(format!("unknown MCP tool: {name}")),
    };

    Ok(McpToolRequest {
        tool,
        tenant_scope: tenant_scope_from_arguments(arguments)?,
        safe_mode: SafeMode::Required,
    })
}

fn validate_tenant_policy(request: McpToolRequest) -> Result<McpToolRequest, String> {
    if requires_tenant_scope(&request.tool) && request.tenant_scope.is_none() {
        return Err("tenant_scope is required for tenant-scoped MCP tools".to_string());
    }
    request.validate().map_err(|error| error.to_string())?;
    Ok(request)
}

fn tenant_scope_from_arguments(
    arguments: &Map<String, Value>,
) -> Result<Option<TenantScope>, String> {
    let tenant_id = arguments.get("tenant_id").and_then(Value::as_str);
    let allowed_schemas = arguments.get("allowed_schemas");
    if tenant_id.is_none() && allowed_schemas.is_none() {
        return Ok(None);
    }

    let Some(tenant_id) = tenant_id else {
        return Err("tenant_id is required when allowed_schemas is provided".to_string());
    };
    let Some(allowed_schemas) = allowed_schemas.and_then(Value::as_array) else {
        return Err("allowed_schemas must be an array".to_string());
    };

    let mut schemas = Vec::new();
    for value in allowed_schemas {
        let Some(schema) = value.as_str() else {
            return Err("allowed_schemas entries must be strings".to_string());
        };
        schemas.push(schema.to_string());
    }

    Ok(Some(TenantScope {
        tenant_id: tenant_id.to_string(),
        allowed_schemas: schemas,
    }))
}

fn required_string(arguments: &Map<String, Value>, field: &'static str) -> Result<String, String> {
    let Some(value) = arguments.get(field).and_then(Value::as_str) else {
        return Err(format!("{field} must be a string"));
    };
    Ok(value.to_string())
}

fn required_u32(arguments: &Map<String, Value>, field: &'static str) -> Result<u32, String> {
    let Some(value) = arguments.get(field).and_then(Value::as_u64) else {
        return Err(format!("{field} must be a positive integer"));
    };
    u32::try_from(value).map_err(|_| format!("{field} is too large"))
}

fn requires_tenant_scope(tool: &McpTool) -> bool {
    matches!(
        tool,
        McpTool::ListShards
            | McpTool::ListHypertables
            | McpTool::RunExplain { .. }
            | McpTool::SuggestIndex { .. }
            | McpTool::QueryWithTimeout { .. }
            | McpTool::TenantArchive { .. }
    )
}

fn mcp_tool_descriptors() -> Vec<Value> {
    [
        ("list_shards", "List tenant-visible shard metadata"),
        (
            "list_hypertables",
            "List tenant-visible distributed hypertables",
        ),
        (
            "run_explain",
            "Dry-run EXPLAIN for a tenant-scoped SQL statement",
        ),
        (
            "rebalance_dry_run",
            "Plan a non-mutating rebalance operation",
        ),
        (
            "suggest_index",
            "Suggest an index for a tenant-scoped table",
        ),
        (
            "query_with_timeout",
            "Run a bounded tenant-scoped read query",
        ),
        ("current_lag", "Report replication lag"),
        (
            "current_replication_status",
            "Report replication health and timeline status",
        ),
        (
            "tenant_archive",
            "Destructive tenant archive request denied while safe mode is required",
        ),
    ]
    .into_iter()
    .map(|(name, description)| {
        json!({
            "name": name,
            "description": description,
            "inputSchema": {
                "type": "object",
                "properties": {
                    "tenant_id": { "type": "string" },
                    "allowed_schemas": {
                        "type": "array",
                        "items": { "type": "string" },
                    },
                    "sql": { "type": "string" },
                    "timeout_ms": { "type": "integer", "minimum": 1 },
                    "shard_group": { "type": "string" },
                    "table": { "type": "string" },
                    "tenant_name": { "type": "string" },
                },
            },
        })
    })
    .collect()
}

fn jsonrpc_result(id: Value, result: Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": result,
    })
}

fn jsonrpc_error(id: Value, code: i64, message: impl Into<String>) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": {
            "code": code,
            "message": message.into(),
        },
    })
}

fn safe_mode_label(safe_mode: SafeMode) -> &'static str {
    match safe_mode {
        SafeMode::Required => "required",
        SafeMode::Disabled => "disabled",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    #[test]
    fn canonical_mcp_execution_report_is_deterministic() {
        assert_eq!(
            canonical_mcp_execution_report(),
            Ok(McpExecutionReport {
                requests: 3,
                tenant_scoped_requests: 2,
                safe_mode_required: 3,
                destructive_denials: 1,
            })
        );
    }

    #[test]
    fn safe_tenant_scoped_query_passes() {
        let request = McpToolRequest {
            tool: McpTool::QueryWithTimeout {
                sql: "SELECT count(*) FROM tenant_a.orders".to_string(),
                timeout_ms: 1_000,
            },
            tenant_scope: Some(TenantScope {
                tenant_id: "tenant-a".to_string(),
                allowed_schemas: vec!["tenant_a".to_string()],
            }),
            safe_mode: SafeMode::Required,
        };

        assert_eq!(request.validate(), Ok(()));
    }

    #[test]
    fn safe_mode_rejects_destructive_tool() {
        let request = McpToolRequest {
            tool: McpTool::TenantArchive {
                tenant_name: "tenant-a".to_string(),
            },
            tenant_scope: Some(TenantScope {
                tenant_id: "tenant-a".to_string(),
                allowed_schemas: vec!["tenant_a".to_string()],
            }),
            safe_mode: SafeMode::Required,
        };

        assert_eq!(request.validate(), Err(McpToolError::UnsafeToolDenied));
    }

    #[test]
    fn scoped_tool_requires_allowed_schemas() {
        let request = McpToolRequest {
            tool: McpTool::ListShards,
            tenant_scope: Some(TenantScope {
                tenant_id: "tenant-a".to_string(),
                allowed_schemas: Vec::new(),
            }),
            safe_mode: SafeMode::Required,
        };

        assert_eq!(
            request.validate(),
            Err(McpToolError::MissingRequiredField("allowed_schemas"))
        );
    }

    #[test]
    fn stdio_initialize_reports_mcp_capabilities() {
        let response = handle_mcp_stdio_request(
            r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05"}}"#,
        );
        let value: Value = serde_json::from_str(&response).expect("json response");

        assert_eq!(value["jsonrpc"], "2.0");
        assert_eq!(value["result"]["serverInfo"]["name"], "ai-blaise-citus-mcp");
        assert_eq!(
            value["result"]["capabilities"]["tools"]["listChanged"],
            false
        );
    }

    #[test]
    fn stdio_tools_call_accepts_tenant_scoped_safe_query() {
        let response = handle_mcp_stdio_request(
            r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"query_with_timeout","arguments":{"sql":"SELECT 1","timeout_ms":1000,"tenant_id":"tenant-a","allowed_schemas":["tenant_a"]}}}"#,
        );
        let value: Value = serde_json::from_str(&response).expect("json response");

        assert_eq!(value["result"]["isError"], false);
        assert!(value["result"]["content"][0]["text"]
            .as_str()
            .expect("text")
            .contains("accepted query_with_timeout"));
    }

    #[test]
    fn stdio_tools_call_denies_destructive_archive() {
        let response = handle_mcp_stdio_request(
            r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"tenant_archive","arguments":{"tenant_name":"tenant-a","tenant_id":"tenant-a","allowed_schemas":["tenant_a"]}}}"#,
        );
        let value: Value = serde_json::from_str(&response).expect("json response");

        assert_eq!(value["result"]["isError"], true);
        assert_eq!(
            value["result"]["content"][0]["text"],
            "safe mode denied a destructive tool"
        );
    }

    #[test]
    fn stdio_tools_call_requires_tenant_scope_for_queries() {
        let response = handle_mcp_stdio_request(
            r#"{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"query_with_timeout","arguments":{"sql":"SELECT 1","timeout_ms":1000}}}"#,
        );
        let value: Value = serde_json::from_str(&response).expect("json response");

        assert_eq!(value["result"]["isError"], true);
        assert_eq!(
            value["result"]["content"][0]["text"],
            "tenant_scope is required for tenant-scoped MCP tools"
        );
    }
}
