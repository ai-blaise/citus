//! Safe Model Context Protocol contracts for ai-blaise/citus operations.

// FEATURE: MCP1
// FEATURE: MCP2
// FEATURE: MCP3
// FEATURE: MCP4
// FEATURE: D11

use std::error::Error;
use std::fmt;

use native_tls::TlsConnector;
use postgres::Client;
use postgres_native_tls::MakeTlsConnector;
use serde_json::{json, Map, Value};

pub const MCP_DATABASE_URL_ENV: &str = "AI_BLAISE_MCP_DATABASE_URL";
pub const MCP_MAX_ROWS_ENV: &str = "AI_BLAISE_MCP_MAX_ROWS";
pub const MCP_MAX_ROWS_CEILING: u32 = 1_000;
pub const MCP_MAX_TIMEOUT_MS: u32 = 300_000;

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
            self.tool.validate_tenant_scope(scope)?;
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

        match self {
            Self::RunExplain { sql } | Self::QueryWithTimeout { sql, .. } => {
                validate_read_only_sql(sql)?;
            }
            _ => {}
        }

        if let Self::QueryWithTimeout { timeout_ms, .. } = self {
            if *timeout_ms == 0 {
                return Err(McpToolError::InvalidTimeout);
            }
            if *timeout_ms > MCP_MAX_TIMEOUT_MS {
                return Err(McpToolError::TimeoutTooLarge);
            }
        }

        Ok(())
    }

    fn is_destructive(&self) -> bool {
        matches!(self, Self::TenantArchive { .. })
    }

    fn validate_tenant_scope(&self, scope: &TenantScope) -> Result<(), McpToolError> {
        match self {
            Self::RunExplain { sql } | Self::QueryWithTimeout { sql, .. } => {
                validate_sql_schema_references(sql, scope)
            }
            Self::SuggestIndex { table } => validate_qualified_table(table, scope),
            _ => Ok(()),
        }
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
                .any(|schema| !is_unquoted_identifier(schema))
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
    Database(String),
    ForbiddenSchema(String),
    InvalidIdentifier(&'static str),
    InvalidTimeout,
    MissingRequiredField(&'static str),
    TimeoutTooLarge,
    UnsafeSql(String),
    UnsafeToolDenied,
}

impl fmt::Display for McpToolError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Database(error) => write!(formatter, "{error}"),
            Self::ForbiddenSchema(schema) => {
                write!(formatter, "schema {schema} is outside allowed_schemas")
            }
            Self::InvalidIdentifier(field) => {
                write!(formatter, "{field} must be an unquoted SQL identifier")
            }
            Self::InvalidTimeout => write!(formatter, "timeout_ms must be greater than zero"),
            Self::MissingRequiredField(field) => {
                write!(formatter, "{field} must not be empty")
            }
            Self::TimeoutTooLarge => {
                write!(formatter, "timeout_ms must not exceed {MCP_MAX_TIMEOUT_MS}")
            }
            Self::UnsafeSql(reason) => write!(formatter, "{reason}"),
            Self::UnsafeToolDenied => write!(formatter, "safe mode denied a destructive tool"),
        }
    }
}

impl Error for McpToolError {}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct McpDatabaseExecutionConfig {
    pub database_url: String,
    pub max_rows: u32,
}

impl McpDatabaseExecutionConfig {
    pub fn new(database_url: String, max_rows: u32) -> Result<Self, McpToolError> {
        if max_rows == 0 {
            return Err(McpToolError::Database(format!(
                "{MCP_MAX_ROWS_ENV} must be greater than zero"
            )));
        }
        if max_rows > MCP_MAX_ROWS_CEILING {
            return Err(McpToolError::Database(format!(
                "{MCP_MAX_ROWS_ENV} must not exceed {MCP_MAX_ROWS_CEILING}"
            )));
        }

        Ok(Self {
            database_url,
            max_rows,
        })
    }

    pub fn from_env() -> Result<Option<Self>, McpToolError> {
        let database_url = match std::env::var(MCP_DATABASE_URL_ENV) {
            Ok(value) if !value.trim().is_empty() => value,
            _ => return Ok(None),
        };
        let max_rows = match std::env::var(MCP_MAX_ROWS_ENV) {
            Ok(value) => value.parse::<u32>().map_err(|_| {
                McpToolError::Database(format!("{MCP_MAX_ROWS_ENV} must be a positive integer"))
            })?,
            Err(_) => 50,
        };

        Self::new(database_url, max_rows).map(Some)
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct McpDatabaseExecutionReport {
    pub tool_name: String,
    pub message: String,
    pub columns: Vec<String>,
    pub rows: Vec<Value>,
}

impl McpDatabaseExecutionReport {
    fn content_text(&self) -> String {
        format!(
            "executed {} rows={} columns={} message={} result={}",
            self.tool_name,
            self.rows.len(),
            self.columns.len(),
            self.message,
            Value::Array(self.rows.clone())
        )
    }
}

fn validate_required(field: &'static str, value: &str) -> Result<(), McpToolError> {
    if value.trim().is_empty() {
        return Err(McpToolError::MissingRequiredField(field));
    }
    Ok(())
}

fn validate_read_only_sql(sql: &str) -> Result<(), McpToolError> {
    let trimmed = sql.trim();
    let lower = trimmed.to_ascii_lowercase();
    let first_token = lower
        .split(|character: char| character.is_whitespace() || character == '(')
        .find(|token| !token.is_empty())
        .unwrap_or("");

    if !matches!(first_token, "select" | "with" | "explain") {
        return Err(McpToolError::UnsafeSql(
            "safe-mode MCP SQL must start with SELECT, WITH, or EXPLAIN".to_string(),
        ));
    }
    if first_token == "explain" && contains_keyword(&lower, "analyze") {
        return Err(McpToolError::UnsafeSql(
            "safe-mode MCP EXPLAIN must not use ANALYZE".to_string(),
        ));
    }

    let without_trailing_semicolon = trimmed.trim_end_matches(';').trim_end();
    if without_trailing_semicolon.contains(';') {
        return Err(McpToolError::UnsafeSql(
            "safe-mode MCP SQL must contain exactly one read-only statement".to_string(),
        ));
    }

    for forbidden in [
        "alter", "begin", "call", "commit", "copy", "create", "delete", "do", "drop", "execute",
        "grant", "insert", "lock", "merge", "reset", "revoke", "rollback", "set", "truncate",
        "update", "vacuum",
    ] {
        if contains_keyword(&lower, forbidden) {
            return Err(McpToolError::UnsafeSql(format!(
                "safe-mode MCP SQL rejects {forbidden} statements"
            )));
        }
    }

    Ok(())
}

fn validate_sql_schema_references(sql: &str, scope: &TenantScope) -> Result<(), McpToolError> {
    for schema in schema_references(sql) {
        if !scope
            .allowed_schemas
            .iter()
            .any(|allowed| allowed == &schema)
        {
            return Err(McpToolError::ForbiddenSchema(schema));
        }
    }
    Ok(())
}

fn validate_qualified_table(table: &str, scope: &TenantScope) -> Result<(), McpToolError> {
    let Some((schema, relation)) = table.split_once('.') else {
        return Err(McpToolError::InvalidIdentifier("table"));
    };
    if relation.contains('.')
        || !is_unquoted_identifier(schema)
        || !is_unquoted_identifier(relation)
    {
        return Err(McpToolError::InvalidIdentifier("table"));
    }
    if !scope
        .allowed_schemas
        .iter()
        .any(|allowed| allowed == schema)
    {
        return Err(McpToolError::ForbiddenSchema(schema.to_string()));
    }
    Ok(())
}

fn contains_keyword(sql: &str, keyword: &str) -> bool {
    sql.match_indices(keyword).any(|(index, _)| {
        let before = sql[..index].chars().next_back();
        let after = sql[index + keyword.len()..].chars().next();
        !before.is_some_and(is_identifier_character) && !after.is_some_and(is_identifier_character)
    })
}

fn schema_references(sql: &str) -> Vec<String> {
    let bytes = sql.as_bytes();
    let mut schemas = Vec::new();
    let mut index = 0;

    while index < bytes.len() {
        if bytes[index] == b'\'' {
            index += 1;
            while index < bytes.len() {
                if bytes[index] == b'\'' {
                    index += 1;
                    if index < bytes.len() && bytes[index] == b'\'' {
                        index += 1;
                        continue;
                    }
                    break;
                }
                index += 1;
            }
            continue;
        }

        if index + 1 < bytes.len() && bytes[index] == b'-' && bytes[index + 1] == b'-' {
            index += 2;
            while index < bytes.len() && bytes[index] != b'\n' {
                index += 1;
            }
            continue;
        }

        if index + 1 < bytes.len() && bytes[index] == b'/' && bytes[index + 1] == b'*' {
            index += 2;
            while index + 1 < bytes.len() && !(bytes[index] == b'*' && bytes[index + 1] == b'/') {
                index += 1;
            }
            index = (index + 2).min(bytes.len());
            continue;
        }

        if !is_identifier_start(bytes[index] as char) {
            index += 1;
            continue;
        }

        let start = index;
        index += 1;
        while index < bytes.len() && is_identifier_character(bytes[index] as char) {
            index += 1;
        }
        let identifier = &sql[start..index];

        let mut lookahead = index;
        while lookahead < bytes.len() && (bytes[lookahead] as char).is_ascii_whitespace() {
            lookahead += 1;
        }
        if lookahead < bytes.len() && bytes[lookahead] == b'.' {
            let mut relation_start = lookahead + 1;
            while relation_start < bytes.len()
                && (bytes[relation_start] as char).is_ascii_whitespace()
            {
                relation_start += 1;
            }
            if relation_start < bytes.len()
                && is_identifier_start(bytes[relation_start] as char)
                && !schemas.iter().any(|schema| schema == identifier)
            {
                schemas.push(identifier.to_string());
            }
        }
    }

    schemas
}

fn is_unquoted_identifier(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    is_identifier_start(first) && chars.all(is_identifier_character)
}

fn is_identifier_start(character: char) -> bool {
    character == '_' || character.is_ascii_alphabetic()
}

fn is_identifier_character(character: char) -> bool {
    character == '_' || character.is_ascii_alphanumeric()
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

pub fn execute_mcp_tool_against_database(
    request: &McpToolRequest,
    config: &McpDatabaseExecutionConfig,
) -> Result<McpDatabaseExecutionReport, McpToolError> {
    request.validate()?;

    match &request.tool {
        McpTool::QueryWithTimeout { sql, timeout_ms } => {
            let bounded_sql = bounded_readonly_query(sql, config.max_rows);
            run_database_query(
                config,
                "query_with_timeout",
                "bounded read-only query",
                &bounded_sql,
                *timeout_ms,
            )
        }
        McpTool::RunExplain { sql } => {
            let explain_sql = explain_query(sql);
            run_explain_query(
                config,
                "run_explain",
                "read-only explain plan",
                &explain_sql,
                5_000,
            )
        }
        McpTool::ListShards => {
            if relation_exists(config, "pg_dist_shard")? {
                run_database_query(
                    config,
                    "list_shards",
                    "pg_dist_shard catalog rows",
                    &format!(
                        "SELECT shardid::text AS shardid, logicalrelid::text AS relation_name \
                         FROM pg_dist_shard ORDER BY shardid LIMIT {}",
                        config.max_rows
                    ),
                    5_000,
                )
            } else {
                Ok(empty_database_report(
                    "list_shards",
                    "pg_dist_shard catalog is absent on this database",
                ))
            }
        }
        McpTool::ListHypertables => {
            if relation_exists(config, "_timescaledb_catalog.hypertable")? {
                run_database_query(
                    config,
                    "list_hypertables",
                    "TimescaleDB hypertable catalog rows",
                    &format!(
                        "SELECT schema_name::text, table_name::text \
                         FROM _timescaledb_catalog.hypertable \
                         ORDER BY schema_name, table_name LIMIT {}",
                        config.max_rows
                    ),
                    5_000,
                )
            } else {
                Ok(empty_database_report(
                    "list_hypertables",
                    "TimescaleDB hypertable catalog is absent on this database",
                ))
            }
        }
        McpTool::SuggestIndex { table } => {
            let (schema, relation) = qualified_table_parts(table)?;
            run_database_query(
                config,
                "suggest_index",
                "existing index inventory for tenant-scoped table",
                &format!(
                    "SELECT indexname::text, indexdef::text FROM pg_indexes \
                     WHERE schemaname = '{}' AND tablename = '{}' \
                     ORDER BY indexname LIMIT {}",
                    escape_sql_literal(schema),
                    escape_sql_literal(relation),
                    config.max_rows
                ),
                5_000,
            )
        }
        McpTool::CurrentLag => run_database_query(
            config,
            "current_lag",
            "replication lag snapshot",
            "SELECT COALESCE(max(replay_lag), interval '0 seconds')::text AS max_replay_lag \
                 FROM pg_stat_replication",
            5_000,
        ),
        McpTool::CurrentReplicationStatus => run_database_query(
            config,
            "current_replication_status",
            "replication connection snapshot",
            "SELECT count(*)::text AS replication_connections FROM pg_stat_replication",
            5_000,
        ),
        McpTool::RebalanceDryRun { .. } => Ok(empty_database_report(
            "rebalance_dry_run",
            "rebalance dry-run remains validation-only until Citus rebalance planning is wired",
        )),
        McpTool::TenantArchive { .. } => Err(McpToolError::UnsafeToolDenied),
    }
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
        Ok(request) => match handle_valid_tool_request(name, request) {
            Ok(text) => jsonrpc_result(
                id,
                json!({
                    "content": [{
                        "type": "text",
                        "text": text,
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
        },
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

fn handle_valid_tool_request(name: &str, request: McpToolRequest) -> Result<String, String> {
    match McpDatabaseExecutionConfig::from_env() {
        Ok(Some(config)) => execute_mcp_tool_against_database(&request, &config)
            .map(|report| report.content_text())
            .map_err(|error| error.to_string()),
        Ok(None) => Ok(format!(
            "validated {} safe_mode={} tenant_scope={}",
            name,
            safe_mode_label(request.safe_mode),
            request
                .tenant_scope
                .as_ref()
                .map_or("none", |scope| scope.tenant_id.as_str()),
        )),
        Err(error) => Err(error.to_string()),
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
        (
            "list_shards",
            "Validate a tenant-scoped shard metadata request",
        ),
        (
            "list_hypertables",
            "Validate a tenant-scoped distributed hypertable request",
        ),
        (
            "run_explain",
            "Validate a read-only EXPLAIN request for tenant-scoped SQL",
        ),
        (
            "rebalance_dry_run",
            "Validate a non-mutating rebalance dry-run request",
        ),
        (
            "suggest_index",
            "Validate an index suggestion request for a tenant-scoped table",
        ),
        (
            "query_with_timeout",
            "Validate a bounded tenant-scoped read-query request",
        ),
        ("current_lag", "Validate a replication-lag report request"),
        (
            "current_replication_status",
            "Validate a replication health and timeline status request",
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

fn bounded_readonly_query(sql: &str, max_rows: u32) -> String {
    format!(
        "SELECT * FROM ({}) AS ai_blaise_mcp_readonly_result LIMIT {max_rows}",
        strip_trailing_statement_terminator(sql)
    )
}

fn explain_query(sql: &str) -> String {
    let stripped = strip_trailing_statement_terminator(sql);
    if stripped
        .trim_start()
        .to_ascii_lowercase()
        .starts_with("explain")
    {
        stripped
    } else {
        format!("EXPLAIN (FORMAT TEXT) {stripped}")
    }
}

fn strip_trailing_statement_terminator(sql: &str) -> String {
    sql.trim().trim_end_matches(';').trim_end().to_string()
}

fn qualified_table_parts(table: &str) -> Result<(&str, &str), McpToolError> {
    let Some((schema, relation)) = table.split_once('.') else {
        return Err(McpToolError::InvalidIdentifier("table"));
    };
    Ok((schema, relation))
}

fn escape_sql_literal(value: &str) -> String {
    value.replace('\'', "''")
}

fn run_database_query(
    config: &McpDatabaseExecutionConfig,
    tool_name: &str,
    message: &str,
    sql: &str,
    timeout_ms: u32,
) -> Result<McpDatabaseExecutionReport, McpToolError> {
    if timeout_ms == 0 {
        return Err(McpToolError::InvalidTimeout);
    }
    let mut client = connect_database(config)?;
    begin_readonly_transaction(&mut client, timeout_ms, tool_name)?;
    let rows = match query_rows_as_json(&mut client, sql, config.max_rows) {
        Ok(rows) => rows,
        Err(error) => {
            rollback_best_effort(&mut client);
            return Err(McpToolError::Database(format!(
                "{tool_name} execution failed: {error}"
            )));
        }
    };
    commit_transaction(&mut client, tool_name)?;

    Ok(McpDatabaseExecutionReport {
        tool_name: tool_name.to_string(),
        message: message.to_string(),
        columns: derive_columns(&rows),
        rows,
    })
}

fn run_explain_query(
    config: &McpDatabaseExecutionConfig,
    tool_name: &str,
    message: &str,
    sql: &str,
    timeout_ms: u32,
) -> Result<McpDatabaseExecutionReport, McpToolError> {
    if timeout_ms == 0 {
        return Err(McpToolError::InvalidTimeout);
    }
    let mut client = connect_database(config)?;
    begin_readonly_transaction(&mut client, timeout_ms, tool_name)?;
    let plan_rows = match client.query(sql, &[]) {
        Ok(rows) => rows,
        Err(error) => {
            rollback_best_effort(&mut client);
            return Err(McpToolError::Database(format!(
                "{tool_name} execution failed: {error}"
            )));
        }
    };
    let mut rows = Vec::new();
    for row in plan_rows.into_iter().take(config.max_rows as usize) {
        let plan: String = match row.try_get(0) {
            Ok(plan) => plan,
            Err(error) => {
                rollback_best_effort(&mut client);
                return Err(McpToolError::Database(format!(
                    "{tool_name} result decoding failed: {error}"
                )));
            }
        };
        rows.push(json!({ "QUERY PLAN": plan }));
    }
    commit_transaction(&mut client, tool_name)?;

    Ok(McpDatabaseExecutionReport {
        tool_name: tool_name.to_string(),
        message: message.to_string(),
        columns: vec!["QUERY PLAN".to_string()],
        rows,
    })
}

fn relation_exists(
    config: &McpDatabaseExecutionConfig,
    relation: &str,
) -> Result<bool, McpToolError> {
    let result = run_database_query(
        config,
        "relation_exists",
        "relation existence check",
        &format!(
            "SELECT to_regclass('{}')::text AS relation_name",
            escape_sql_literal(relation)
        ),
        5_000,
    )?;
    Ok(result
        .rows
        .first()
        .and_then(Value::as_object)
        .and_then(|row| row.get("relation_name"))
        .and_then(Value::as_str)
        .is_some())
}

fn connect_database(config: &McpDatabaseExecutionConfig) -> Result<Client, McpToolError> {
    let connector = TlsConnector::builder()
        .build()
        .map_err(|error| McpToolError::Database(format!("TLS connector setup failed: {error}")))?;
    let connector = MakeTlsConnector::new(connector);
    Client::connect(&config.database_url, connector).map_err(|error| {
        McpToolError::Database(format!("{MCP_DATABASE_URL_ENV} connection failed: {error}"))
    })
}

fn begin_readonly_transaction(
    client: &mut Client,
    timeout_ms: u32,
    tool_name: &str,
) -> Result<(), McpToolError> {
    client
        .batch_execute(&format!(
            "BEGIN READ ONLY; \
             SET LOCAL statement_timeout = {timeout_ms}; \
             SET LOCAL idle_in_transaction_session_timeout = {timeout_ms};"
        ))
        .map_err(|error| {
            McpToolError::Database(format!(
                "{tool_name} read-only transaction setup failed: {error}"
            ))
        })
}

fn commit_transaction(client: &mut Client, tool_name: &str) -> Result<(), McpToolError> {
    client.batch_execute("COMMIT").map_err(|error| {
        McpToolError::Database(format!(
            "{tool_name} read-only transaction commit failed: {error}"
        ))
    })
}

fn rollback_best_effort(client: &mut Client) {
    let _ = client.batch_execute("ROLLBACK");
}

fn query_rows_as_json(client: &mut Client, sql: &str, max_rows: u32) -> Result<Vec<Value>, String> {
    let row = client
        .query_one(&json_aggregation_sql(sql, max_rows), &[])
        .map_err(|error| error.to_string())?;
    let json_text: String = row.try_get(0).map_err(|error| error.to_string())?;
    let value: Value = serde_json::from_str(&json_text).map_err(|error| error.to_string())?;
    value
        .as_array()
        .cloned()
        .ok_or_else(|| "database JSON aggregation did not return an array".to_string())
}

fn json_aggregation_sql(sql: &str, max_rows: u32) -> String {
    format!(
        "WITH ai_blaise_mcp_limited AS ( \
             SELECT * FROM ({}) AS ai_blaise_mcp_result LIMIT {max_rows} \
         ) \
         SELECT COALESCE(jsonb_agg(to_jsonb(ai_blaise_mcp_limited)), '[]'::jsonb)::text \
         FROM ai_blaise_mcp_limited",
        strip_trailing_statement_terminator(sql)
    )
}

fn derive_columns(rows: &[Value]) -> Vec<String> {
    rows.first()
        .and_then(Value::as_object)
        .map(|object| object.keys().cloned().collect())
        .unwrap_or_default()
}

fn empty_database_report(tool_name: &str, message: &str) -> McpDatabaseExecutionReport {
    McpDatabaseExecutionReport {
        tool_name: tool_name.to_string(),
        message: message.to_string(),
        columns: Vec::new(),
        rows: Vec::new(),
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
        std::env::remove_var(MCP_DATABASE_URL_ENV);
        let response = handle_mcp_stdio_request(
            r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"query_with_timeout","arguments":{"sql":"SELECT 1","timeout_ms":1000,"tenant_id":"tenant-a","allowed_schemas":["tenant_a"]}}}"#,
        );
        let value: Value = serde_json::from_str(&response).expect("json response");

        assert_eq!(value["result"]["isError"], false);
        assert!(value["result"]["content"][0]["text"]
            .as_str()
            .expect("text")
            .contains("validated query_with_timeout"));
    }

    #[test]
    fn stdio_tools_call_denies_destructive_archive() {
        std::env::remove_var(MCP_DATABASE_URL_ENV);
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
        std::env::remove_var(MCP_DATABASE_URL_ENV);
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

    #[test]
    fn tenant_scope_rejects_cross_schema_sql() {
        let request = McpToolRequest {
            tool: McpTool::QueryWithTimeout {
                sql: "SELECT count(*) FROM tenant_b.orders".to_string(),
                timeout_ms: 1_000,
            },
            tenant_scope: Some(TenantScope {
                tenant_id: "tenant-a".to_string(),
                allowed_schemas: vec!["tenant_a".to_string()],
            }),
            safe_mode: SafeMode::Required,
        };

        assert_eq!(
            request.validate(),
            Err(McpToolError::ForbiddenSchema("tenant_b".to_string()))
        );
    }

    #[test]
    fn safe_mode_rejects_mutating_sql() {
        let request = McpToolRequest {
            tool: McpTool::QueryWithTimeout {
                sql: "UPDATE tenant_a.orders SET total = 0".to_string(),
                timeout_ms: 1_000,
            },
            tenant_scope: Some(TenantScope {
                tenant_id: "tenant-a".to_string(),
                allowed_schemas: vec!["tenant_a".to_string()],
            }),
            safe_mode: SafeMode::Required,
        };

        assert_eq!(
            request.validate(),
            Err(McpToolError::UnsafeSql(
                "safe-mode MCP SQL must start with SELECT, WITH, or EXPLAIN".to_string()
            ))
        );
    }

    #[test]
    fn safe_mode_rejects_explain_analyze() {
        let request = McpToolRequest {
            tool: McpTool::RunExplain {
                sql: "EXPLAIN ANALYZE SELECT count(*) FROM tenant_a.orders".to_string(),
            },
            tenant_scope: Some(TenantScope {
                tenant_id: "tenant-a".to_string(),
                allowed_schemas: vec!["tenant_a".to_string()],
            }),
            safe_mode: SafeMode::Required,
        };

        assert_eq!(
            request.validate(),
            Err(McpToolError::UnsafeSql(
                "safe-mode MCP EXPLAIN must not use ANALYZE".to_string()
            ))
        );
    }

    #[test]
    fn query_timeout_has_a_production_ceiling() {
        let request = McpToolRequest {
            tool: McpTool::QueryWithTimeout {
                sql: "SELECT count(*) FROM tenant_a.orders".to_string(),
                timeout_ms: MCP_MAX_TIMEOUT_MS + 1,
            },
            tenant_scope: Some(TenantScope {
                tenant_id: "tenant-a".to_string(),
                allowed_schemas: vec!["tenant_a".to_string()],
            }),
            safe_mode: SafeMode::Required,
        };

        assert_eq!(request.validate(), Err(McpToolError::TimeoutTooLarge));
    }

    #[test]
    fn stdio_tools_call_denies_cross_schema_query() {
        std::env::remove_var(MCP_DATABASE_URL_ENV);
        let response = handle_mcp_stdio_request(
            r#"{"jsonrpc":"2.0","id":6,"method":"tools/call","params":{"name":"query_with_timeout","arguments":{"sql":"SELECT count(*) FROM tenant_b.orders","timeout_ms":1000,"tenant_id":"tenant-a","allowed_schemas":["tenant_a"]}}}"#,
        );
        let value: Value = serde_json::from_str(&response).expect("json response");

        assert_eq!(value["result"]["isError"], true);
        assert_eq!(
            value["result"]["content"][0]["text"],
            "schema tenant_b is outside allowed_schemas"
        );
    }

    #[test]
    fn builds_bounded_readonly_query() {
        assert_eq!(
            bounded_readonly_query("SELECT 1;", 25),
            "SELECT * FROM (SELECT 1) AS ai_blaise_mcp_readonly_result LIMIT 25"
        );
    }

    #[test]
    fn builds_explain_query_without_double_explain() {
        assert_eq!(explain_query("SELECT 1;"), "EXPLAIN (FORMAT TEXT) SELECT 1");
        assert_eq!(explain_query("EXPLAIN SELECT 1;"), "EXPLAIN SELECT 1");
    }

    #[test]
    fn database_config_rejects_unbounded_row_limits() {
        let config = McpDatabaseExecutionConfig::new(
            "postgresql://postgres@127.0.0.1/postgres".to_string(),
            MCP_MAX_ROWS_CEILING + 1,
        );

        assert_eq!(
            config,
            Err(McpToolError::Database(format!(
                "{MCP_MAX_ROWS_ENV} must not exceed {MCP_MAX_ROWS_CEILING}"
            )))
        );
    }
}
