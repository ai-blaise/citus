//! Safe Model Context Protocol contracts for ai-blaise/citus operations.

// FEATURE: MCP1
// FEATURE: MCP2
// FEATURE: MCP3
// FEATURE: D11

use std::error::Error;
use std::fmt;

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

#[cfg(test)]
mod tests {
    use super::*;

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
}
