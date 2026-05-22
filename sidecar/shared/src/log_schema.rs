//! Canonical structured-log schemas for ai-blaise sidecars and the pool proxy.
//!
//! Each sidecar emits JSON log lines with a deterministic schema. The schema
//! is described as data so that:
//!
//! * companion can synthesize a SQL view that parses the JSON column into
//!   typed columns (see `companion::log_view`),
//! * `docs/ai-blaise/OBSERVABILITY.md` can be checked against the same source
//!   of truth, and
//! * sidecars can validate their emission helpers in unit tests.
//!
//! All schemas include the common fields. Per-sidecar specializations layer
//! on top of the common base.

// FEATURE: O15

use std::error::Error;
use std::fmt;
use std::fmt::Write as _;

/// Severity levels mirror RFC 5424 / OpenTelemetry severity numbers but the
/// emitted JSON uses the canonical short names below.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub enum LogSeverity {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
    Critical,
}

impl LogSeverity {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Trace => "trace",
            Self::Debug => "debug",
            Self::Info => "info",
            Self::Warn => "warn",
            Self::Error => "error",
            Self::Critical => "critical",
        }
    }
}

/// Logical type of a structured-log field. The companion log view maps each
/// kind onto a typed SQL column.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum LogFieldKind {
    Timestamp,
    String,
    Integer,
    Float,
    Bool,
    Json,
}

impl LogFieldKind {
    pub fn pg_type(self) -> &'static str {
        match self {
            Self::Timestamp => "timestamptz",
            Self::String => "text",
            Self::Integer => "bigint",
            Self::Float => "double precision",
            Self::Bool => "boolean",
            Self::Json => "jsonb",
        }
    }
}

/// A single structured-log field declaration.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct LogField {
    pub name: &'static str,
    pub kind: LogFieldKind,
    pub required: bool,
    pub description: &'static str,
}

impl LogField {
    pub const fn required(
        name: &'static str,
        kind: LogFieldKind,
        description: &'static str,
    ) -> Self {
        Self {
            name,
            kind,
            required: true,
            description,
        }
    }

    pub const fn optional(
        name: &'static str,
        kind: LogFieldKind,
        description: &'static str,
    ) -> Self {
        Self {
            name,
            kind,
            required: false,
            description,
        }
    }
}

/// A structured-log schema. The `common` fields are inherited by every
/// sidecar; the `extensions` are specific to one sidecar.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct LogSchema {
    pub component: &'static str,
    pub common: &'static [LogField],
    pub extensions: &'static [LogField],
}

impl LogSchema {
    pub fn validate(&self) -> Result<(), LogSchemaError> {
        if self.component.trim().is_empty() {
            return Err(LogSchemaError::EmptyComponent);
        }
        // Common + extension fields must have non-overlapping names so the
        // companion log view never gets a duplicate column.
        for field in self.extensions {
            if self.common.iter().any(|common| common.name == field.name) {
                return Err(LogSchemaError::ConflictingField(field.name));
            }
        }
        // Required common fields anchor the JSON shape; without them the
        // companion view would produce all-NULL rows.
        for required in CANONICAL_COMMON_REQUIRED {
            if !self.common.iter().any(|field| field.name == *required) {
                return Err(LogSchemaError::MissingCommonField(required));
            }
        }
        Ok(())
    }

    pub fn all_fields(&self) -> impl Iterator<Item = &LogField> {
        self.common.iter().chain(self.extensions.iter())
    }
}

/// Named sidecar schema, used by `canonical_sidecar_log_schemas` to expose
/// the deterministic catalog of every sidecar's log shape.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SidecarLogSchema {
    pub sidecar: &'static str,
    pub schema: LogSchema,
}

/// A single rendered log record. The companion log view consumes a stream of
/// these records when ingesting Vector/Loki output back into the database.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct LogRecord {
    pub timestamp_rfc3339: String,
    pub level: LogSeverity,
    pub component: String,
    pub message: String,
    pub traceparent: Option<String>,
    pub tenant_id: Option<String>,
    pub request_id: Option<String>,
    pub version: Option<String>,
    pub error: Option<String>,
    pub fields_json: String,
}

impl LogRecord {
    /// Render the log record as a single-line JSON object. The implementation
    /// is intentionally manual to keep `serde_json` out of the shared crate's
    /// dependency graph; the format is deterministic and round-trippable.
    pub fn to_json_line(&self) -> String {
        let mut output = String::with_capacity(256);
        output.push('{');
        write_json_field(&mut output, "timestamp", &self.timestamp_rfc3339, false);
        write_json_field(&mut output, "level", self.level.as_str(), true);
        write_json_field(&mut output, "sidecar", &self.component, true);
        write_json_field(&mut output, "message", &self.message, true);
        if let Some(traceparent) = &self.traceparent {
            write_json_field(&mut output, "traceparent", traceparent, true);
        }
        if let Some(tenant_id) = &self.tenant_id {
            write_json_field(&mut output, "tenant_id", tenant_id, true);
        }
        if let Some(request_id) = &self.request_id {
            write_json_field(&mut output, "request_id", request_id, true);
        }
        if let Some(version) = &self.version {
            write_json_field(&mut output, "version", version, true);
        }
        if let Some(error) = &self.error {
            write_json_field(&mut output, "error", error, true);
        }
        if !self.fields_json.is_empty() {
            output.push(',');
            output.push_str("\"fields\":");
            output.push_str(&self.fields_json);
        }
        output.push('}');
        output
    }
}

fn write_json_field(output: &mut String, name: &str, value: &str, leading_comma: bool) {
    if leading_comma {
        output.push(',');
    }
    output.push('"');
    output.push_str(name);
    output.push_str("\":\"");
    output.push_str(&escape_json_string(value));
    output.push('"');
}

fn escape_json_string(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                let _ = write!(output, "\\u{:04x}", c as u32);
            }
            c => output.push(c),
        }
    }
    output
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum LogSchemaError {
    EmptyComponent,
    ConflictingField(&'static str),
    MissingCommonField(&'static str),
}

impl fmt::Display for LogSchemaError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyComponent => write!(formatter, "log schema component must not be empty"),
            Self::ConflictingField(name) => {
                write!(
                    formatter,
                    "extension field '{name}' conflicts with a common field"
                )
            }
            Self::MissingCommonField(name) => {
                write!(
                    formatter,
                    "log schema is missing required common field '{name}'"
                )
            }
        }
    }
}

impl Error for LogSchemaError {}

const CANONICAL_COMMON_REQUIRED: &[&str] = &["timestamp", "level", "sidecar", "message"];

const COMMON_FIELDS: &[LogField] = &[
    LogField::required(
        "timestamp",
        LogFieldKind::Timestamp,
        "RFC3339 wall-clock timestamp at log emission, including the timezone offset.",
    ),
    LogField::required(
        "level",
        LogFieldKind::String,
        "Severity short name: trace, debug, info, warn, error, or critical.",
    ),
    LogField::required(
        "sidecar",
        LogFieldKind::String,
        "Component name of the emitter, e.g. vectorizer or postgrest.",
    ),
    LogField::required(
        "message",
        LogFieldKind::String,
        "Human-readable single-line message.",
    ),
    LogField::optional(
        "traceparent",
        LogFieldKind::String,
        "W3C trace-context traceparent of the inbound or outbound RPC, if any.",
    ),
    LogField::optional(
        "tenant_id",
        LogFieldKind::String,
        "Tenant identifier when the operation is tenant-scoped.",
    ),
    LogField::optional(
        "request_id",
        LogFieldKind::String,
        "Application-level request identifier; correlates with the access log.",
    ),
    LogField::optional(
        "version",
        LogFieldKind::String,
        "Sidecar binary version (semver), useful when rolling out.",
    ),
    LogField::optional(
        "error",
        LogFieldKind::String,
        "Top-level error message; present only when level is error or critical.",
    ),
    LogField::optional(
        "fields",
        LogFieldKind::Json,
        "Sidecar-specific structured fields (see per-sidecar schema below).",
    ),
];

const ANALYTICAL_FIELDS: &[LogField] = &[
    LogField::optional(
        "query_queue_depth",
        LogFieldKind::Integer,
        "Pending analytical-query queue depth at emission.",
    ),
    LogField::optional(
        "iceberg_snapshot_seconds",
        LogFieldKind::Integer,
        "Seconds since the most recent Iceberg snapshot.",
    ),
    LogField::optional(
        "mirror_stream",
        LogFieldKind::String,
        "Source mirror stream name.",
    ),
];

const AUTH_FIELDS: &[LogField] = &[
    LogField::optional(
        "issuer",
        LogFieldKind::String,
        "JWT issuer that signed the inbound token.",
    ),
    LogField::optional("subject", LogFieldKind::String, "JWT subject claim."),
    LogField::optional(
        "denial_reason",
        LogFieldKind::String,
        "Reason an auth decision was a denial (e.g. expired_token).",
    ),
];

const BACKUP_FIELDS: &[LogField] = &[
    LogField::optional(
        "wal_archive_lag_seconds",
        LogFieldKind::Integer,
        "Lag between newest WAL and archive at emission.",
    ),
    LogField::optional(
        "last_backup_age_seconds",
        LogFieldKind::Integer,
        "Age of the most recent base backup.",
    ),
    LogField::optional("archive_uri", LogFieldKind::String, "Backup archive URI."),
];

const CDC_FIELDS: &[LogField] = &[
    LogField::optional(
        "slot_name",
        LogFieldKind::String,
        "Logical replication slot name.",
    ),
    LogField::optional(
        "sink",
        LogFieldKind::String,
        "Sink that received the change (webhook/realtime/kafka/...).",
    ),
    LogField::optional(
        "lag_seconds",
        LogFieldKind::Integer,
        "Lag in seconds between source LSN and acknowledged LSN.",
    ),
    LogField::optional(
        "delivered_count",
        LogFieldKind::Integer,
        "Number of change records delivered in this batch.",
    ),
];

const COLDTIER_FIELDS: &[LogField] = &[
    LogField::optional(
        "object_count",
        LogFieldKind::Integer,
        "Number of cold-tier objects examined.",
    ),
    LogField::optional(
        "bytes_demoted",
        LogFieldKind::Integer,
        "Bytes demoted to cold tier in this run.",
    ),
];

const EDGE_FUNCTIONS_FIELDS: &[LogField] = &[
    LogField::optional(
        "runtime",
        LogFieldKind::String,
        "Runtime name: deno or bun.",
    ),
    LogField::optional(
        "function",
        LogFieldKind::String,
        "Edge function identifier.",
    ),
    LogField::optional(
        "invocation_id",
        LogFieldKind::String,
        "Per-invocation identifier for correlation.",
    ),
    LogField::optional(
        "language_error",
        LogFieldKind::String,
        "JavaScript/TypeScript error class, if any.",
    ),
];

const GRAPHQL_FIELDS: &[LogField] = &[
    LogField::optional(
        "operation_name",
        LogFieldKind::String,
        "Named GraphQL operation.",
    ),
    LogField::optional(
        "operation_kind",
        LogFieldKind::String,
        "query, mutation, or subscription.",
    ),
    LogField::optional(
        "language_error",
        LogFieldKind::String,
        "Server-side resolver error class, if any.",
    ),
];

const HLC_FIELDS: &[LogField] = &[
    LogField::optional(
        "logical_time",
        LogFieldKind::Integer,
        "Hybrid logical clock logical component at emission.",
    ),
    LogField::optional(
        "clock_skew_ms",
        LogFieldKind::Integer,
        "Observed clock skew vs cluster majority.",
    ),
];

const MCP_FIELDS: &[LogField] = &[
    LogField::optional("tool", LogFieldKind::String, "MCP tool name."),
    LogField::optional(
        "denial_kind",
        LogFieldKind::String,
        "tenant_denial or destructive_denial when a request is rejected.",
    ),
];

const POSTGREST_FIELDS: &[LogField] = &[
    LogField::optional("route", LogFieldKind::String, "REST route pattern."),
    LogField::optional("method", LogFieldKind::String, "HTTP method."),
    LogField::optional(
        "status_code",
        LogFieldKind::Integer,
        "HTTP status code returned.",
    ),
    LogField::optional(
        "language_error",
        LogFieldKind::String,
        "Plv8/plrust error class, if any.",
    ),
];

const RAFT_FIELDS: &[LogField] = &[
    LogField::optional("term", LogFieldKind::Integer, "Raft term at emission."),
    LogField::optional(
        "leader_id",
        LogFieldKind::String,
        "Current leader node identifier.",
    ),
    LogField::optional(
        "follower_lag_index",
        LogFieldKind::Integer,
        "Highest committed index minus follower match index.",
    ),
];

const REALTIME_FIELDS: &[LogField] = &[
    LogField::optional("topic", LogFieldKind::String, "Channel/topic identifier."),
    LogField::optional(
        "ws_connections",
        LogFieldKind::Integer,
        "Active WebSocket connections.",
    ),
    LogField::optional(
        "broadcast_fanout",
        LogFieldKind::Integer,
        "Recipients reached in this broadcast.",
    ),
];

const REPACK_FIELDS: &[LogField] = &[
    LogField::optional(
        "target",
        LogFieldKind::String,
        "Table or index being repacked.",
    ),
    LogField::optional(
        "strategy",
        LogFieldKind::String,
        "pg_repack or repack-concurrently.",
    ),
    LogField::optional(
        "bytes_compacted",
        LogFieldKind::Integer,
        "Bytes reclaimed in this run.",
    ),
];

const SCHEMA_JOB_FIELDS: &[LogField] = &[
    LogField::optional("job_name", LogFieldKind::String, "Schema-job identifier."),
    LogField::optional(
        "attempt",
        LogFieldKind::Integer,
        "Retry attempt counter (1-indexed).",
    ),
    LogField::optional(
        "dialect",
        LogFieldKind::String,
        "SQL dialect for the job (postgres/citus).",
    ),
];

const STORAGE_FIELDS: &[LogField] = &[
    LogField::optional(
        "bucket",
        LogFieldKind::String,
        "Object-storage bucket name.",
    ),
    LogField::optional(
        "operation",
        LogFieldKind::String,
        "PUT, GET, DELETE, or presign.",
    ),
    LogField::optional(
        "object_bytes",
        LogFieldKind::Integer,
        "Object size in bytes.",
    ),
];

const TXN_STATUS_FIELDS: &[LogField] = &[
    LogField::optional("xid", LogFieldKind::Integer, "PostgreSQL transaction id."),
    LogField::optional(
        "commit_state",
        LogFieldKind::String,
        "committed, aborted, or in_progress.",
    ),
];

const VECTORIZER_FIELDS: &[LogField] = &[
    LogField::optional(
        "provider",
        LogFieldKind::String,
        "Embedding provider (openai, azure_openai, voyage, ...).",
    ),
    LogField::optional("model", LogFieldKind::String, "Embedding model identifier."),
    LogField::optional(
        "embedding_count",
        LogFieldKind::Integer,
        "Number of embeddings produced.",
    ),
    LogField::optional("tokens", LogFieldKind::Integer, "Tokens consumed."),
    LogField::optional(
        "cost_usd",
        LogFieldKind::Float,
        "Estimated USD cost for this batch.",
    ),
];

const ANALYTICAL_SCHEMA: LogSchema = LogSchema {
    component: "analytical",
    common: COMMON_FIELDS,
    extensions: ANALYTICAL_FIELDS,
};
const AUTH_SCHEMA: LogSchema = LogSchema {
    component: "auth",
    common: COMMON_FIELDS,
    extensions: AUTH_FIELDS,
};
const BACKUP_SCHEMA: LogSchema = LogSchema {
    component: "backup",
    common: COMMON_FIELDS,
    extensions: BACKUP_FIELDS,
};
const CDC_SCHEMA: LogSchema = LogSchema {
    component: "cdc",
    common: COMMON_FIELDS,
    extensions: CDC_FIELDS,
};
const COLDTIER_SCHEMA: LogSchema = LogSchema {
    component: "coldtier",
    common: COMMON_FIELDS,
    extensions: COLDTIER_FIELDS,
};
const EDGE_FUNCTIONS_SCHEMA: LogSchema = LogSchema {
    component: "edge_functions",
    common: COMMON_FIELDS,
    extensions: EDGE_FUNCTIONS_FIELDS,
};
const GRAPHQL_SCHEMA: LogSchema = LogSchema {
    component: "graphql",
    common: COMMON_FIELDS,
    extensions: GRAPHQL_FIELDS,
};
const HLC_SCHEMA: LogSchema = LogSchema {
    component: "hlc",
    common: COMMON_FIELDS,
    extensions: HLC_FIELDS,
};
const MCP_SCHEMA: LogSchema = LogSchema {
    component: "mcp",
    common: COMMON_FIELDS,
    extensions: MCP_FIELDS,
};
const POSTGREST_SCHEMA: LogSchema = LogSchema {
    component: "postgrest",
    common: COMMON_FIELDS,
    extensions: POSTGREST_FIELDS,
};
const RAFT_SCHEMA: LogSchema = LogSchema {
    component: "raft",
    common: COMMON_FIELDS,
    extensions: RAFT_FIELDS,
};
const REALTIME_SCHEMA: LogSchema = LogSchema {
    component: "realtime",
    common: COMMON_FIELDS,
    extensions: REALTIME_FIELDS,
};
const REPACK_SCHEMA: LogSchema = LogSchema {
    component: "repack",
    common: COMMON_FIELDS,
    extensions: REPACK_FIELDS,
};
const SCHEMA_JOB_SCHEMA: LogSchema = LogSchema {
    component: "schema_job",
    common: COMMON_FIELDS,
    extensions: SCHEMA_JOB_FIELDS,
};
const STORAGE_SCHEMA: LogSchema = LogSchema {
    component: "storage",
    common: COMMON_FIELDS,
    extensions: STORAGE_FIELDS,
};
const TXN_STATUS_SCHEMA: LogSchema = LogSchema {
    component: "txn_status",
    common: COMMON_FIELDS,
    extensions: TXN_STATUS_FIELDS,
};
const VECTORIZER_SCHEMA: LogSchema = LogSchema {
    component: "vectorizer",
    common: COMMON_FIELDS,
    extensions: VECTORIZER_FIELDS,
};

const CANONICAL_SCHEMAS: &[SidecarLogSchema] = &[
    SidecarLogSchema {
        sidecar: "analytical",
        schema: ANALYTICAL_SCHEMA,
    },
    SidecarLogSchema {
        sidecar: "auth",
        schema: AUTH_SCHEMA,
    },
    SidecarLogSchema {
        sidecar: "backup",
        schema: BACKUP_SCHEMA,
    },
    SidecarLogSchema {
        sidecar: "cdc",
        schema: CDC_SCHEMA,
    },
    SidecarLogSchema {
        sidecar: "coldtier",
        schema: COLDTIER_SCHEMA,
    },
    SidecarLogSchema {
        sidecar: "edge_functions",
        schema: EDGE_FUNCTIONS_SCHEMA,
    },
    SidecarLogSchema {
        sidecar: "graphql",
        schema: GRAPHQL_SCHEMA,
    },
    SidecarLogSchema {
        sidecar: "hlc",
        schema: HLC_SCHEMA,
    },
    SidecarLogSchema {
        sidecar: "mcp",
        schema: MCP_SCHEMA,
    },
    SidecarLogSchema {
        sidecar: "postgrest",
        schema: POSTGREST_SCHEMA,
    },
    SidecarLogSchema {
        sidecar: "raft",
        schema: RAFT_SCHEMA,
    },
    SidecarLogSchema {
        sidecar: "realtime",
        schema: REALTIME_SCHEMA,
    },
    SidecarLogSchema {
        sidecar: "repack",
        schema: REPACK_SCHEMA,
    },
    SidecarLogSchema {
        sidecar: "schema_job",
        schema: SCHEMA_JOB_SCHEMA,
    },
    SidecarLogSchema {
        sidecar: "storage",
        schema: STORAGE_SCHEMA,
    },
    SidecarLogSchema {
        sidecar: "txn_status",
        schema: TXN_STATUS_SCHEMA,
    },
    SidecarLogSchema {
        sidecar: "vectorizer",
        schema: VECTORIZER_SCHEMA,
    },
];

/// Return the canonical, deterministic catalog of sidecar log schemas. The
/// order is stable and matches the workspace's sidecar enumeration.
pub fn canonical_sidecar_log_schemas() -> &'static [SidecarLogSchema] {
    CANONICAL_SCHEMAS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_canonical_schema_validates() {
        for sidecar in canonical_sidecar_log_schemas() {
            sidecar
                .schema
                .validate()
                .unwrap_or_else(|error| panic!("schema {} failed: {error}", sidecar.sidecar));
        }
    }

    #[test]
    fn extensions_must_not_shadow_common_fields() {
        const CLASHING: &[LogField] = &[LogField::required(
            "timestamp",
            LogFieldKind::String,
            "would shadow timestamp",
        )];
        let schema = LogSchema {
            component: "broken",
            common: COMMON_FIELDS,
            extensions: CLASHING,
        };
        assert_eq!(
            schema.validate().unwrap_err(),
            LogSchemaError::ConflictingField("timestamp"),
        );
    }

    #[test]
    fn schema_requires_canonical_common_fields() {
        const MISSING_COMMON: &[LogField] = &[];
        let schema = LogSchema {
            component: "broken",
            common: MISSING_COMMON,
            extensions: &[],
        };
        assert_eq!(
            schema.validate().unwrap_err(),
            LogSchemaError::MissingCommonField("timestamp"),
        );
    }

    #[test]
    fn log_record_to_json_line_is_one_line_and_escapes_quotes() {
        let record = LogRecord {
            timestamp_rfc3339: "2026-05-22T05:30:00Z".to_string(),
            level: LogSeverity::Info,
            component: "vectorizer".to_string(),
            message: "embed batch \"complete\"".to_string(),
            traceparent: Some(
                "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01".to_string(),
            ),
            tenant_id: Some("tenant-a".to_string()),
            request_id: Some("req-1".to_string()),
            version: Some("0.1.0".to_string()),
            error: None,
            fields_json: "{\"provider\":\"openai\",\"tokens\":2048}".to_string(),
        };
        let rendered = record.to_json_line();
        assert!(rendered.starts_with('{') && rendered.ends_with('}'));
        assert!(!rendered.contains('\n'));
        assert!(rendered.contains("\"sidecar\":\"vectorizer\""));
        assert!(rendered.contains("\"message\":\"embed batch \\\"complete\\\"\""));
        assert!(rendered.contains("\"fields\":{\"provider\":\"openai\",\"tokens\":2048}"));
    }

    #[test]
    fn canonical_schemas_cover_every_sidecar_in_workspace() {
        let sidecars: Vec<&str> = canonical_sidecar_log_schemas()
            .iter()
            .map(|item| item.sidecar)
            .collect();
        assert!(sidecars.contains(&"vectorizer"));
        assert!(sidecars.contains(&"realtime"));
        assert!(sidecars.contains(&"mcp"));
        assert!(sidecars.contains(&"raft"));
        // The workspace ships 17 sidecars; the schema catalog must cover them all.
        assert_eq!(sidecars.len(), 17);
    }
}
