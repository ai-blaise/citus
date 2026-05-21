//! MCP sidecar service contracts.

// FEATURE: MCP1
// FEATURE: MCP2
// FEATURE: MCP3
// FEATURE: D11

use ai_blaise_citus_mcp::{
    handle_mcp_stdio_request_with_server_info, McpTool, McpToolError, McpToolRequest, SafeMode,
    TenantScope,
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
    MissingRequiredField(&'static str),
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
            Self::MissingRequiredField(field) => write!(formatter, "{field} must not be empty"),
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
                sql: "select * from public.orders where tenant_id = $1".to_string(),
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
        assert!(response.contains("accepted query_with_timeout"));
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
}
