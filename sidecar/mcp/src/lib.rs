//! MCP sidecar service contracts.

// FEATURE: MCP1
// FEATURE: MCP2
// FEATURE: MCP3
// FEATURE: D11

use ai_blaise_citus_mcp::{
    handle_mcp_stdio_request_with_server_info, McpTool, McpToolError, McpToolRequest, SafeMode,
    TenantScope,
};
use ai_blaise_citus_sidecar_shared::{
    listen_addr_from_env, HttpProbeResponse, SidecarRuntime, SidecarRuntimeError,
};
use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct McpSidecarPlan {
    pub listen_addr: String,
    pub auth: McpAuthPlan,
    pub session_policy: McpSessionPolicy,
    pub allowed_requests: Vec<McpToolRequest>,
}

impl McpSidecarPlan {
    pub fn validate(&self) -> Result<(), McpSidecarError> {
        validate_required("listen_addr", &self.listen_addr)?;
        self.auth.validate()?;
        self.session_policy.validate()?;
        if self.allowed_requests.is_empty() {
            return Err(McpSidecarError::MissingRequiredField("allowed_requests"));
        }
        for request in &self.allowed_requests {
            request.validate()?;
            if self.session_policy.safe_mode == SafeMode::Required
                && request.safe_mode != SafeMode::Required
            {
                return Err(McpSidecarError::UnsafeSessionPolicy);
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct McpAuthPlan {
    pub issuer: String,
    pub audience: String,
    pub tenant_claim: String,
}

impl McpAuthPlan {
    fn validate(&self) -> Result<(), McpSidecarError> {
        validate_required("auth.issuer", &self.issuer)?;
        validate_required("auth.audience", &self.audience)?;
        validate_required("auth.tenant_claim", &self.tenant_claim)
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct McpSessionPolicy {
    pub safe_mode: SafeMode,
    pub max_concurrent_sessions: u32,
    pub idle_timeout_seconds: u32,
}

impl McpSessionPolicy {
    fn validate(&self) -> Result<(), McpSidecarError> {
        if self.max_concurrent_sessions == 0 {
            return Err(McpSidecarError::InvalidConcurrency);
        }
        if self.idle_timeout_seconds == 0 {
            return Err(McpSidecarError::InvalidIdleTimeout);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum McpSidecarError {
    InvalidConcurrency,
    InvalidIdleTimeout,
    MalformedHttpRequest,
    MissingRequiredField(&'static str),
    Runtime(String),
    Tool(String),
    UnsafeSessionPolicy,
}

impl fmt::Display for McpSidecarError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidConcurrency => {
                write!(
                    formatter,
                    "max_concurrent_sessions must be greater than zero"
                )
            }
            Self::InvalidIdleTimeout => {
                write!(formatter, "idle_timeout_seconds must be greater than zero")
            }
            Self::MalformedHttpRequest => write!(formatter, "malformed MCP sidecar HTTP request"),
            Self::MissingRequiredField(field) => write!(formatter, "{field} must not be empty"),
            Self::Runtime(error) => write!(formatter, "{error}"),
            Self::Tool(error) => write!(formatter, "{error}"),
            Self::UnsafeSessionPolicy => {
                write!(formatter, "safe-mode sessions require safe-mode requests")
            }
        }
    }
}

impl Error for McpSidecarError {}

impl From<McpToolError> for McpSidecarError {
    fn from(error: McpToolError) -> Self {
        Self::Tool(error.to_string())
    }
}

impl From<SidecarRuntimeError> for McpSidecarError {
    fn from(error: SidecarRuntimeError) -> Self {
        Self::Runtime(error.to_string())
    }
}

impl From<std::io::Error> for McpSidecarError {
    fn from(error: std::io::Error) -> Self {
        Self::Runtime(error.to_string())
    }
}

fn validate_required(field: &'static str, value: &str) -> Result<(), McpSidecarError> {
    if value.trim().is_empty() {
        return Err(McpSidecarError::MissingRequiredField(field));
    }
    Ok(())
}

pub fn canonical_mcp_plan() -> McpSidecarPlan {
    McpSidecarPlan {
        listen_addr: "127.0.0.1:8088".to_string(),
        auth: McpAuthPlan {
            issuer: "https://auth.example.com".to_string(),
            audience: "citus-mcp".to_string(),
            tenant_claim: "tenant_id".to_string(),
        },
        session_policy: McpSessionPolicy {
            safe_mode: SafeMode::Required,
            max_concurrent_sessions: 16,
            idle_timeout_seconds: 300,
        },
        allowed_requests: vec![McpToolRequest {
            tool: McpTool::RunExplain {
                sql: "select * from tenant_a.orders where tenant_id = $1".to_string(),
            },
            tenant_scope: Some(TenantScope {
                tenant_id: "tenant-a".to_string(),
                allowed_schemas: vec!["tenant_a".to_string()],
            }),
            safe_mode: SafeMode::Required,
        }],
    }
}

pub fn canonical_mcp_execution_plan() -> Result<McpSidecarPlan, McpSidecarError> {
    let plan = canonical_mcp_plan();
    plan.validate()?;
    Ok(plan)
}

pub fn handle_mcp_sidecar_stdio_request(line: &str) -> Result<String, McpSidecarError> {
    canonical_mcp_execution_plan()?;
    Ok(handle_mcp_stdio_request_with_server_info(
        line,
        "ai-blaise-citus-mcp-sidecar",
    ))
}

pub fn handle_mcp_sidecar_http_bytes(request: &[u8]) -> Result<HttpProbeResponse, McpSidecarError> {
    let mut runtime = SidecarRuntime::ready("mcp-sidecar");
    handle_mcp_sidecar_http_bytes_with_runtime(&mut runtime, request)
}

pub fn handle_mcp_sidecar_http_bytes_with_runtime(
    runtime: &mut SidecarRuntime,
    request: &[u8],
) -> Result<HttpProbeResponse, McpSidecarError> {
    let request =
        std::str::from_utf8(request).map_err(|_| McpSidecarError::MalformedHttpRequest)?;
    let (method, path, body) = parse_http_request(request)?;

    match (method, path) {
        ("POST", "/mcp") => {
            let response = handle_mcp_sidecar_stdio_request(body.trim())?;
            Ok(HttpProbeResponse::new(
                200,
                "application/json",
                format!("{response}\n"),
            ))
        }
        (_, "/mcp") => Ok(HttpProbeResponse::new(
            405,
            "application/json",
            "{\"error\":\"method not allowed\"}\n",
        )),
        _ => Ok(runtime.handle_http_bytes(request.as_bytes())?),
    }
}

pub fn serve_mcp_sidecar_http_forever(default_addr: &str) -> Result<(), McpSidecarError> {
    use std::io::Write;
    use std::net::TcpListener;

    canonical_mcp_execution_plan()?;
    let listen_addr = listen_addr_from_env(default_addr)?;
    let listener = TcpListener::bind(&listen_addr)?;
    eprintln!("ai-blaise mcp-sidecar HTTP server listening on {listen_addr}");
    let mut runtime = SidecarRuntime::ready("mcp-sidecar");

    for stream in listener.incoming() {
        let mut stream = stream?;
        let request = read_http_request(&mut stream)?;
        let response = handle_mcp_sidecar_http_bytes_with_runtime(&mut runtime, &request)
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

fn parse_http_request(request: &str) -> Result<(&str, &str, &str), McpSidecarError> {
    let (head, body) = request
        .split_once("\r\n\r\n")
        .or_else(|| request.split_once("\n\n"))
        .ok_or(McpSidecarError::MalformedHttpRequest)?;
    let request_line = head
        .lines()
        .next()
        .ok_or(McpSidecarError::MalformedHttpRequest)?;
    let mut parts = request_line.split_whitespace();
    let method = parts.next().ok_or(McpSidecarError::MalformedHttpRequest)?;
    let path = parts.next().ok_or(McpSidecarError::MalformedHttpRequest)?;
    if !path.starts_with('/') {
        return Err(McpSidecarError::MalformedHttpRequest);
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
    fn mcp_sidecar_plan_validates_safe_tenant_scoped_requests() {
        assert_eq!(canonical_mcp_plan().validate(), Ok(()));
    }

    #[test]
    fn canonical_mcp_execution_plan_is_deterministic() {
        let plan = canonical_mcp_execution_plan().expect("canonical plan");

        assert_eq!(plan.listen_addr, "127.0.0.1:8088");
        assert_eq!(plan.session_policy.max_concurrent_sessions, 16);
        assert_eq!(plan.allowed_requests.len(), 1);
    }

    #[test]
    fn safe_session_policy_rejects_non_safe_request() {
        let mut plan = canonical_mcp_plan();
        plan.allowed_requests[0].safe_mode = SafeMode::Disabled;

        assert_eq!(plan.validate(), Err(McpSidecarError::UnsafeSessionPolicy));
    }

    #[test]
    fn destructive_tool_is_denied_by_tool_contract() {
        let mut plan = canonical_mcp_plan();
        plan.allowed_requests.push(McpToolRequest {
            tool: McpTool::TenantArchive {
                tenant_name: "tenant-a".to_string(),
            },
            tenant_scope: plan.allowed_requests[0].tenant_scope.clone(),
            safe_mode: SafeMode::Required,
        });

        assert_eq!(
            plan.validate(),
            Err(McpSidecarError::Tool(
                "safe mode denied a destructive tool".to_string()
            ))
        );
    }

    #[test]
    fn sidecar_stdio_reports_malformed_jsonrpc_as_response() {
        let response = handle_mcp_sidecar_stdio_request(r#"{"#).expect("sidecar stdio response");

        assert!(response.contains(r#""code":-32700"#));
        assert!(response.contains("parse error"));
    }

    #[test]
    fn sidecar_stdio_reports_unknown_method_as_response() {
        let response = handle_mcp_sidecar_stdio_request(
            r#"{"jsonrpc":"2.0","id":9,"method":"resources/list"}"#,
        )
        .expect("sidecar stdio response");

        assert!(response.contains(r#""id":9"#));
        assert!(response.contains(r#""code":-32601"#));
        assert!(response.contains("unknown method: resources/list"));
    }

    #[test]
    fn sidecar_stdio_lists_complete_tool_registry() {
        let response =
            handle_mcp_sidecar_stdio_request(r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#)
                .expect("sidecar stdio response");

        assert_eq!(response.matches(r#""name":"#).count(), 9);
        for expected in [
            "list_shards",
            "list_hypertables",
            "run_explain",
            "rebalance_dry_run",
            "suggest_index",
            "query_with_timeout",
            "current_lag",
            "current_replication_status",
            "tenant_archive",
        ] {
            assert!(response.contains(expected), "tools/list missing {expected}");
        }
        assert!(response.contains("inputSchema"));
    }

    #[test]
    fn sidecar_stdio_initialize_identifies_sidecar_server() {
        let response =
            handle_mcp_sidecar_stdio_request(r#"{"jsonrpc":"2.0","id":1,"method":"initialize"}"#)
                .expect("sidecar stdio response");

        assert!(response.contains(r#""name":"ai-blaise-citus-mcp-sidecar""#));
        assert!(response.contains(r#""tools":{"listChanged":false}"#));
    }

    #[test]
    fn sidecar_stdio_accepts_safe_tenant_query() {
        let response = handle_mcp_sidecar_stdio_request(
            r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"query_with_timeout","arguments":{"sql":"SELECT count(*) FROM tenant_a.orders","timeout_ms":1000,"tenant_id":"tenant-a","allowed_schemas":["tenant_a"]}}}"#,
        )
        .expect("sidecar stdio response");

        assert!(response.contains(r#""isError":false"#));
        assert!(response.contains("validated query_with_timeout"));
        assert!(response.contains("tenant-a"));
    }

    #[test]
    fn sidecar_stdio_denies_destructive_tool() {
        let response = handle_mcp_sidecar_stdio_request(
            r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"tenant_archive","arguments":{"tenant_name":"tenant-a","tenant_id":"tenant-a","allowed_schemas":["tenant_a"]}}}"#,
        )
        .expect("sidecar stdio response");

        assert!(response.contains(r#""isError":true"#));
        assert!(response.contains("safe mode denied a destructive tool"));
    }

    #[test]
    fn sidecar_stdio_requires_tenant_scope() {
        let response = handle_mcp_sidecar_stdio_request(
            r#"{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"query_with_timeout","arguments":{"sql":"SELECT 1","timeout_ms":1000}}}"#,
        )
        .expect("sidecar stdio response");

        assert!(response.contains(r#""isError":true"#));
        assert!(response.contains("tenant_scope is required"));
    }

    #[test]
    fn sidecar_http_serves_health_probes() {
        let response =
            handle_mcp_sidecar_http_bytes(b"GET /healthz HTTP/1.1\r\nHost: local\r\n\r\n")
                .expect("health response");

        assert_eq!(response.status_code, 200);
        assert!(response.body.contains(r#""component":"mcp-sidecar""#));
    }

    #[test]
    fn sidecar_http_serves_readiness_and_metrics() {
        let ready = handle_mcp_sidecar_http_bytes(b"GET /readyz HTTP/1.1\r\nHost: local\r\n\r\n")
            .expect("ready response");
        assert_eq!(ready.status_code, 200);
        assert!(ready.body.contains(r#""component":"mcp-sidecar""#));

        let metrics =
            handle_mcp_sidecar_http_bytes(b"GET /metrics HTTP/1.1\r\nHost: local\r\n\r\n")
                .expect("metrics response");
        assert_eq!(metrics.status_code, 200);
        assert_eq!(metrics.content_type, "text/plain; version=0.0.4");
        assert!(metrics
            .body
            .contains("ai_blaise_sidecar_ready{component=\"mcp-sidecar\"} 1"));
    }

    #[test]
    fn sidecar_http_persists_probe_runtime_state() {
        let mut runtime = SidecarRuntime::ready("mcp-sidecar");
        let drain = handle_mcp_sidecar_http_bytes_with_runtime(
            &mut runtime,
            b"POST /drain HTTP/1.1\r\nHost: local\r\n\r\n",
        )
        .expect("drain response");
        assert_eq!(drain.status_code, 202);

        let ready = handle_mcp_sidecar_http_bytes_with_runtime(
            &mut runtime,
            b"GET /readyz HTTP/1.1\r\nHost: local\r\n\r\n",
        )
        .expect("ready response");
        assert_eq!(ready.status_code, 503);

        let metrics = handle_mcp_sidecar_http_bytes_with_runtime(
            &mut runtime,
            b"GET /metrics HTTP/1.1\r\nHost: local\r\n\r\n",
        )
        .expect("metrics response");
        assert!(metrics
            .body
            .contains("ai_blaise_sidecar_ready{component=\"mcp-sidecar\"} 0"));
    }

    #[test]
    fn sidecar_http_posts_mcp_jsonrpc() {
        let response = handle_mcp_sidecar_http_bytes(
            b"POST /mcp HTTP/1.1\r\nHost: local\r\nContent-Type: application/json\r\n\r\n{\"jsonrpc\":\"2.0\",\"id\":7,\"method\":\"initialize\"}",
        )
        .expect("MCP HTTP response");

        assert_eq!(response.status_code, 200);
        assert_eq!(response.content_type, "application/json");
        assert!(response
            .body
            .contains(r#""name":"ai-blaise-citus-mcp-sidecar""#));
    }

    #[test]
    fn sidecar_http_preserves_jsonrpc_parse_errors() {
        let response = handle_mcp_sidecar_http_bytes(
            b"POST /mcp HTTP/1.1\r\nHost: local\r\nContent-Type: application/json\r\n\r\n{",
        )
        .expect("MCP HTTP response");

        assert_eq!(response.status_code, 200);
        assert!(response.body.contains(r#""code":-32700"#));
        assert!(response.body.contains("parse error"));
    }

    #[test]
    fn sidecar_http_preserves_jsonrpc_unknown_method_errors() {
        let response = handle_mcp_sidecar_http_bytes(
            b"POST /mcp HTTP/1.1\r\nHost: local\r\nContent-Type: application/json\r\n\r\n{\"jsonrpc\":\"2.0\",\"id\":8,\"method\":\"resources/list\"}",
        )
        .expect("MCP HTTP response");

        assert_eq!(response.status_code, 200);
        assert!(response.body.contains(r#""id":8"#));
        assert!(response.body.contains(r#""code":-32601"#));
    }

    #[test]
    fn sidecar_http_rejects_wrong_mcp_method() {
        let response = handle_mcp_sidecar_http_bytes(b"GET /mcp HTTP/1.1\r\nHost: local\r\n\r\n")
            .expect("MCP method response");

        assert_eq!(response.status_code, 405);
        assert!(response.body.contains("method not allowed"));
    }

    #[test]
    fn sidecar_http_rejects_malformed_request() {
        assert_eq!(
            handle_mcp_sidecar_http_bytes(b"not-http"),
            Err(McpSidecarError::MalformedHttpRequest)
        );
    }

    #[test]
    fn http_request_completion_honors_content_length() {
        let partial =
            b"POST /mcp HTTP/1.1\r\nHost: local\r\nContent-Length: 17\r\n\r\n{\"jsonrpc\":\"2.0\"";
        assert!(!http_request_complete(partial));

        let complete =
            b"POST /mcp HTTP/1.1\r\nHost: local\r\nContent-Length: 16\r\n\r\n{\"jsonrpc\":\"2.0\"";
        assert!(http_request_complete(complete));
    }
}
