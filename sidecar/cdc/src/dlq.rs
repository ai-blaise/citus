//! Dead-letter queue for CDC deliveries that exhaust their retry budget.
//!
//! The DLQ is intentionally minimal: it owns an append-only in-memory log
//! plus an optional on-disk sidecar file. The runtime calls [`Dlq::enqueue`]
//! when a sink dispatch exceeds the retry policy max attempts.
//! Operators drain the DLQ by reading the file (one JSON record per line)
//! or by querying the in-memory log through `GET /dlq` on the control plane.

// FEATURE: WH3

use crate::sinks::{SinkWireFrame, SinkWireKind};
use crate::{CdcEventEnvelope, CdcSidecarError, SinkDeliveryPlan};
use serde_json::{json, Value};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct DlqRecord {
    pub queue: String,
    pub sink: String,
    pub target: String,
    pub event_lsn: String,
    pub tenant_id: String,
    pub reason: String,
    pub attempts: u32,
    pub frame_bytes: usize,
}

impl DlqRecord {
    pub fn as_json(&self) -> Value {
        json!({
            "queue": self.queue,
            "sink": self.sink,
            "target": self.target,
            "event_lsn": self.event_lsn,
            "tenant_id": self.tenant_id,
            "reason": self.reason,
            "attempts": self.attempts,
            "frame_bytes": self.frame_bytes,
        })
    }
}

#[derive(Debug)]
pub struct Dlq {
    in_memory: Mutex<Vec<DlqRecord>>,
    file_path: Option<PathBuf>,
}

impl Dlq {
    pub fn in_memory() -> Self {
        Self {
            in_memory: Mutex::new(Vec::new()),
            file_path: None,
        }
    }

    pub fn with_file(path: impl Into<PathBuf>) -> Self {
        Self {
            in_memory: Mutex::new(Vec::new()),
            file_path: Some(path.into()),
        }
    }

    /// Path the DLQ persists to, if any.
    pub fn file_path(&self) -> Option<&Path> {
        self.file_path.as_deref()
    }

    /// Append a record. Persists to disk too if `with_file` was used.
    pub fn enqueue(&self, record: DlqRecord) -> Result<(), CdcSidecarError> {
        if let Some(path) = &self.file_path {
            let mut file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)
                .map_err(|e| {
                    CdcSidecarError::SharedContract(format!(
                        "dlq.open: {path}: {e}",
                        path = path.display()
                    ))
                })?;
            let mut line = record.as_json().to_string();
            line.push('\n');
            file.write_all(line.as_bytes())
                .map_err(|e| CdcSidecarError::SharedContract(format!("dlq.write: {e}")))?;
        }
        self.in_memory
            .lock()
            .map_err(|_| CdcSidecarError::SharedContract("dlq.lock poisoned".to_string()))?
            .push(record);
        Ok(())
    }

    pub fn records(&self) -> Result<Vec<DlqRecord>, CdcSidecarError> {
        self.in_memory
            .lock()
            .map(|guard| guard.clone())
            .map_err(|_| CdcSidecarError::SharedContract("dlq.lock poisoned".to_string()))
    }

    pub fn len(&self) -> usize {
        self.in_memory.lock().map(|guard| guard.len()).unwrap_or(0)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Build a [`DlqRecord`] for a failed dispatch. The caller supplies the
/// frame, the plan, the event, and a human-readable failure reason.
pub fn build_dlq_record(
    plan: &SinkDeliveryPlan,
    frame: &SinkWireFrame,
    event: &CdcEventEnvelope,
    attempts: u32,
    reason: &str,
) -> DlqRecord {
    DlqRecord {
        queue: plan.retry_policy.dead_letter_queue.clone(),
        sink: frame.sink.clone(),
        target: sink_target(&frame.kind),
        event_lsn: event.lsn.clone(),
        tenant_id: event.tenant_id.clone(),
        reason: reason.to_string(),
        attempts,
        frame_bytes: frame.bytes.len(),
    }
}

fn sink_target(kind: &SinkWireKind) -> String {
    match kind {
        SinkWireKind::KafkaProduce { topic, .. } => topic.clone(),
        SinkWireKind::KinesisPutRecord { stream_name, .. } => stream_name.clone(),
        SinkWireKind::PubSubPublish { topic, .. } => topic.clone(),
        SinkWireKind::NatsPub { subject } => subject.clone(),
        SinkWireKind::Http1Request { url } => url.clone(),
        SinkWireKind::Http2Request { url, .. } => url.clone(),
        SinkWireKind::RealtimeFanout { topic_prefix } => topic_prefix.clone(),
        SinkWireKind::AnalyticalAppend { stream_name } => stream_name.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sinks::{encode_sink_frame, CdcEventPayload};
    use crate::{canonical_cdc_event, canonical_cdc_plan, canonical_retry_policy, CdcSinkPlan};

    #[test]
    fn enqueue_records_persist_to_memory() {
        let dlq = Dlq::in_memory();
        let record = sample_record();
        dlq.enqueue(record.clone()).expect("enqueue");
        let records = dlq.records().expect("records");
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].sink, record.sink);
        assert!(!dlq.is_empty());
    }

    #[test]
    fn build_dlq_record_uses_sink_target() {
        let event = canonical_cdc_event();
        let payload = CdcEventPayload::encode(&event, &[]);
        let plan = canonical_cdc_plan();
        let sink = plan
            .sinks
            .iter()
            .find(|s| matches!(s, CdcSinkPlan::Nats { .. }))
            .cloned()
            .unwrap();
        let frame = encode_sink_frame(&sink, &payload, &event).expect("encode");
        let delivery_plan = SinkDeliveryPlan {
            sink: "nats".to_string(),
            target: "tenant.orders".to_string(),
            retry_policy: canonical_retry_policy(),
        };
        let record = build_dlq_record(&delivery_plan, &frame, &event, 5, "broker unreachable");
        assert_eq!(record.target, "tenant.orders");
        assert_eq!(record.attempts, 5);
        assert_eq!(record.reason, "broker unreachable");
        assert!(record.frame_bytes > 0);
    }

    #[test]
    fn enqueue_persists_to_file_when_configured() {
        let tmp = tempfile_path();
        let dlq = Dlq::with_file(&tmp);
        dlq.enqueue(sample_record()).expect("enqueue");
        let contents = std::fs::read_to_string(&tmp).expect("read");
        assert!(contents.contains("\"queue\":\"cdc.dead_letters\""));
        std::fs::remove_file(&tmp).ok();
    }

    fn tempfile_path() -> PathBuf {
        let mut path = std::env::temp_dir();
        path.push(format!("ai-blaise-cdc-dlq-test-{}.log", std::process::id()));
        let _ = std::fs::remove_file(&path);
        path
    }

    fn sample_record() -> DlqRecord {
        DlqRecord {
            queue: "cdc.dead_letters".to_string(),
            sink: "kafka".to_string(),
            target: "orders".to_string(),
            event_lsn: "16/B374D848".to_string(),
            tenant_id: "tenant-a".to_string(),
            reason: "broker timeout".to_string(),
            attempts: 5,
            frame_bytes: 256,
        }
    }
}
