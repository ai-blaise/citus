// FEATURE: RT1

//! Realtime CDC ingest hook.
//!
//! The pool tails the logical-replication stream populated by the placement
//! subscriber and re-emits change events to the `sidecar/realtime` ingest UDS
//! at `/var/run/citus/cdc.sock` (configurable). This module owns the framing
//! contract: each event is serialized as length-prefixed JSON exactly as
//! `sidecar/realtime` expects on the receive side.

use std::error::Error;
use std::fmt;

/// Logical-replication event flavors the pool re-emits.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum CdcOperation {
    Insert,
    Update,
    Delete,
    Truncate,
}

impl CdcOperation {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Insert => "INSERT",
            Self::Update => "UPDATE",
            Self::Delete => "DELETE",
            Self::Truncate => "TRUNCATE",
        }
    }
}

/// One row-level CDC event.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct CdcEvent {
    pub tenant_id: String,
    pub schema: String,
    pub table: String,
    pub operation: CdcOperation,
    pub commit_lsn: u64,
    pub primary_key_json: String,
    pub row_json: String,
}

impl CdcEvent {
    pub fn validate(&self) -> Result<(), RealtimeHookError> {
        if self.tenant_id.trim().is_empty() {
            return Err(RealtimeHookError::MissingField("tenant_id"));
        }
        if self.schema.trim().is_empty() {
            return Err(RealtimeHookError::MissingField("schema"));
        }
        if self.table.trim().is_empty() {
            return Err(RealtimeHookError::MissingField("table"));
        }
        if self.primary_key_json.trim().is_empty() {
            return Err(RealtimeHookError::MissingField("primary_key_json"));
        }
        if self.row_json.trim().is_empty() {
            return Err(RealtimeHookError::MissingField("row_json"));
        }
        Ok(())
    }

    /// Serialize the event into the UDS line framing expected by
    /// `sidecar/realtime`:
    ///
    /// `<length>\n<json>\n`
    pub fn encode_frame(&self) -> Result<Vec<u8>, RealtimeHookError> {
        self.validate()?;
        let json = self.to_json();
        let frame = format!("{}\n{}\n", json.len(), json);
        Ok(frame.into_bytes())
    }

    fn to_json(&self) -> String {
        format!(
            "{{\"tenant_id\":\"{}\",\"schema\":\"{}\",\"table\":\"{}\",\"op\":\"{}\",\"lsn\":{},\"pk\":{},\"row\":{}}}",
            escape_json(&self.tenant_id),
            escape_json(&self.schema),
            escape_json(&self.table),
            self.operation.as_str(),
            self.commit_lsn,
            self.primary_key_json,
            self.row_json,
        )
    }
}

/// Configuration for the realtime hook.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct RealtimeHookConfig {
    pub uds_path: String,
    pub max_queue_depth: u32,
}

impl Default for RealtimeHookConfig {
    fn default() -> Self {
        Self {
            uds_path: "/var/run/citus/cdc.sock".to_string(),
            max_queue_depth: 4_096,
        }
    }
}

impl RealtimeHookConfig {
    pub fn validate(&self) -> Result<(), RealtimeHookError> {
        if self.uds_path.trim().is_empty() {
            return Err(RealtimeHookError::MissingField("uds_path"));
        }
        if !self.uds_path.starts_with('/') {
            return Err(RealtimeHookError::InvalidUdsPath(self.uds_path.clone()));
        }
        if self.max_queue_depth == 0 {
            return Err(RealtimeHookError::InvalidQueueDepth);
        }
        Ok(())
    }
}

/// In-process queue used by the proxy hot path to hand events to the writer
/// task. Exposing a deterministic API lets tests exercise back-pressure
/// without spinning a real UDS.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct RealtimeHookQueue {
    config: RealtimeHookConfig,
    buffer: std::collections::VecDeque<CdcEvent>,
    dropped: u64,
}

impl RealtimeHookQueue {
    pub fn new(config: RealtimeHookConfig) -> Result<Self, RealtimeHookError> {
        config.validate()?;
        Ok(Self {
            config,
            buffer: std::collections::VecDeque::new(),
            dropped: 0,
        })
    }

    /// Enqueue an event. Returns `false` if the queue is full (event dropped
    /// + `dropped` counter incremented) and `true` if accepted.
    pub fn enqueue(&mut self, event: CdcEvent) -> Result<bool, RealtimeHookError> {
        event.validate()?;
        if self.buffer.len() as u32 >= self.config.max_queue_depth {
            self.dropped += 1;
            return Ok(false);
        }
        self.buffer.push_back(event);
        Ok(true)
    }

    pub fn drain(&mut self) -> Vec<CdcEvent> {
        self.buffer.drain(..).collect()
    }

    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    pub fn dropped(&self) -> u64 {
        self.dropped
    }

    pub fn config(&self) -> &RealtimeHookConfig {
        &self.config
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum RealtimeHookError {
    InvalidQueueDepth,
    InvalidUdsPath(String),
    MissingField(&'static str),
}

impl fmt::Display for RealtimeHookError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidQueueDepth => {
                write!(formatter, "max_queue_depth must be greater than zero")
            }
            Self::InvalidUdsPath(path) => write!(formatter, "uds_path must be absolute: {path}"),
            Self::MissingField(field) => write!(formatter, "{field} must not be empty"),
        }
    }
}

impl Error for RealtimeHookError {}

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

    fn sample_event(operation: CdcOperation) -> CdcEvent {
        CdcEvent {
            tenant_id: "tenant-a".to_string(),
            schema: "public".to_string(),
            table: "orders".to_string(),
            operation,
            commit_lsn: 42,
            primary_key_json: "{\"id\":7}".to_string(),
            row_json: "{\"id\":7,\"total\":100}".to_string(),
        }
    }

    #[test]
    fn config_defaults_to_pool_uds_path() {
        let config = RealtimeHookConfig::default();
        assert_eq!(config.uds_path, "/var/run/citus/cdc.sock");
        assert!(config.validate().is_ok());
    }

    #[test]
    fn relative_uds_path_rejected() {
        let config = RealtimeHookConfig {
            uds_path: "cdc.sock".to_string(),
            ..RealtimeHookConfig::default()
        };
        assert!(matches!(
            config.validate(),
            Err(RealtimeHookError::InvalidUdsPath(_))
        ));
    }

    #[test]
    fn encode_frame_emits_length_prefix() {
        let frame = sample_event(CdcOperation::Insert)
            .encode_frame()
            .expect("frame");
        let text = std::str::from_utf8(&frame).expect("utf8");
        let mut parts = text.splitn(2, '\n');
        let length: usize = parts.next().expect("length").parse().expect("usize");
        let json_with_newline = parts.next().expect("json");
        // strip trailing newline
        let json = json_with_newline
            .strip_suffix('\n')
            .expect("trailing newline");
        assert_eq!(length, json.len());
        assert!(json.contains("\"op\":\"INSERT\""));
        assert!(json.contains("\"tenant_id\":\"tenant-a\""));
    }

    #[test]
    fn queue_drops_when_full() {
        let mut queue = RealtimeHookQueue::new(RealtimeHookConfig {
            uds_path: "/tmp/test.sock".to_string(),
            max_queue_depth: 1,
        })
        .expect("queue");
        assert_eq!(queue.enqueue(sample_event(CdcOperation::Insert)), Ok(true));
        assert_eq!(queue.enqueue(sample_event(CdcOperation::Update)), Ok(false));
        assert_eq!(queue.dropped(), 1);
    }

    #[test]
    fn drain_returns_in_order() {
        let mut queue = RealtimeHookQueue::new(RealtimeHookConfig {
            uds_path: "/tmp/test.sock".to_string(),
            max_queue_depth: 4,
        })
        .expect("queue");
        queue
            .enqueue(sample_event(CdcOperation::Insert))
            .expect("a");
        queue
            .enqueue(sample_event(CdcOperation::Update))
            .expect("b");
        let drained = queue.drain();
        assert_eq!(drained.len(), 2);
        assert_eq!(drained[0].operation, CdcOperation::Insert);
        assert_eq!(drained[1].operation, CdcOperation::Update);
        assert!(queue.is_empty());
    }
}
