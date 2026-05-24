//! Live CDC runtime: replication source -> anonymization -> sink dispatch ->
//! DLQ on failure.
//!
//! The runtime here is the inner engine. The binary in `main.rs` wraps it
//! with a TCP control-plane: callers POST a wal2json frame (or a JSON list
//! of pre-decoded events) and receive a [`CdcDispatchReport`] in return.

// FEATURE: C1
// FEATURE: C2
// FEATURE: C3
// FEATURE: C14
// FEATURE: C15
// FEATURE: WH3

use crate::sinks::{encode_sink_frame, CdcEventPayload, SinkDispatchReport, SinkWireFrame};
use crate::source::{decode_replication_frame, ReplicationFrame};
use crate::{
    anon, build_dlq_record, decode_wal2json_frame, CdcDeliveryPlan, CdcEventEnvelope, CdcRuntime,
    CdcSidecarError, CdcSidecarPlan, CdcSinkPlan, Dlq, DlqRecord, LogicalReplicationFrame,
    SinkDeliveryOutcome, WalOutputPlugin,
};

/// Source of replication frames. The default is wal2json over a logical
/// replication slot; tests can substitute a deterministic frame source.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum CdcReplicationSource {
    Wal2Json,
    PgOutput,
}

/// Runtime configuration injected by the binary. The `Dlq` lives outside
/// this struct so it can be shared across batches.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct CdcRuntimeConfig {
    pub plan: CdcSidecarPlan,
    pub source: CdcReplicationSource,
    pub dispatch_live: bool,
    pub dlq_path: Option<String>,
}

impl CdcRuntimeConfig {
    pub fn canonical() -> Self {
        Self {
            plan: crate::canonical_cdc_plan(),
            source: CdcReplicationSource::Wal2Json,
            dispatch_live: false,
            dlq_path: None,
        }
    }
}

/// One dispatched CDC event: the post-anon envelope, the sink-by-sink
/// dispatch reports, and the LSN the runtime acked back to PostgreSQL.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct CdcDispatchedEvent {
    pub event: CdcEventEnvelope,
    pub anonymized_columns: Vec<String>,
    pub plan: CdcDeliveryPlan,
    pub frames: Vec<SinkWireFrame>,
    pub outcomes: Vec<SinkDeliveryOutcome>,
    pub dlq_entries: Vec<DlqRecord>,
}

/// Aggregate of a full WAL frame dispatch.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct CdcDispatchReport {
    pub start_lsn: String,
    pub end_lsn: String,
    pub events: Vec<CdcDispatchedEvent>,
    pub dlq_total: usize,
    pub bytes_total: u64,
}

/// In-process live runtime. Holds the plan + DLQ + running state.
pub struct CdcLiveRuntime {
    runtime: CdcRuntime,
    plan: CdcSidecarPlan,
    dlq: Dlq,
    config: CdcRuntimeConfig,
}

impl CdcLiveRuntime {
    pub fn new(config: CdcRuntimeConfig) -> Result<Self, CdcSidecarError> {
        let runtime = CdcRuntime::new(config.plan.clone())?;
        let dlq = match &config.dlq_path {
            Some(path) => Dlq::with_file(path),
            None => Dlq::in_memory(),
        };
        Ok(Self {
            plan: config.plan.clone(),
            runtime,
            dlq,
            config,
        })
    }

    pub fn dlq(&self) -> &Dlq {
        &self.dlq
    }

    pub fn plan(&self) -> &CdcSidecarPlan {
        &self.plan
    }

    pub fn config(&self) -> &CdcRuntimeConfig {
        &self.config
    }

    pub fn runtime(&self) -> &CdcRuntime {
        &self.runtime
    }

    /// Decode a generic logical replication frame, then run the same
    /// anonymization, sink encoding, DLQ, and ack path used by the wal2json
    /// control-plane endpoint.
    pub fn ingest_replication_frame(
        &mut self,
        frame: &ReplicationFrame,
    ) -> Result<CdcDispatchReport, CdcSidecarError> {
        let expected = match self.config.source {
            CdcReplicationSource::Wal2Json => WalOutputPlugin::Wal2Json,
            CdcReplicationSource::PgOutput => WalOutputPlugin::PgOutput,
        };
        if frame.plugin != expected {
            return Err(CdcSidecarError::UnsupportedOperation(
                "replication frame plugin does not match runtime source".to_string(),
            ));
        }
        let raw_events = decode_replication_frame(frame)?;
        let logical = LogicalReplicationFrame {
            start_lsn: frame.start_lsn.clone(),
            end_lsn: frame.end_lsn.clone(),
            payload: String::from_utf8_lossy(&frame.payload).into_owned(),
        };
        self.ingest_decoded_events(&logical, raw_events)
    }

    /// Decode a wal2json frame, anonymize the events, encode the wire
    /// frames for each sink, dispatch them, and record DLQ entries for
    /// failures.
    pub fn ingest_wal2json(
        &mut self,
        frame: &LogicalReplicationFrame,
    ) -> Result<CdcDispatchReport, CdcSidecarError> {
        if !matches!(self.config.source, CdcReplicationSource::Wal2Json) {
            return Err(CdcSidecarError::UnsupportedOperation(
                "non-wal2json source ingested via wal2json path".to_string(),
            ));
        }
        let raw_events = decode_wal2json_frame(frame)?;
        self.ingest_decoded_events(frame, raw_events)
    }

    fn ingest_decoded_events(
        &mut self,
        frame: &LogicalReplicationFrame,
        raw_events: Vec<CdcEventEnvelope>,
    ) -> Result<CdcDispatchReport, CdcSidecarError> {
        let batch = self.runtime.advance_with_events(frame, &raw_events)?;
        let mut dispatched = Vec::with_capacity(raw_events.len());
        let mut dlq_total = 0_usize;
        let mut bytes_total = 0_u64;
        for (event, delivery_report) in raw_events.into_iter().zip(batch.deliveries.iter()) {
            let mut event = event;
            let applied = anon::apply_anonymization(&self.plan.anonymization, &mut event);
            let payload = CdcEventPayload::encode(&event, &applied);
            let mut frames = Vec::with_capacity(self.plan.sinks.len());
            let mut outcomes = Vec::with_capacity(self.plan.sinks.len());
            let mut dlq_entries = Vec::new();
            for (sink_index, sink) in self.plan.sinks.iter().enumerate() {
                let frame = encode_sink_frame(sink, &payload, &event)?;
                bytes_total += frame.bytes.len() as u64;
                let outcome = self.dispatch_one(sink, &frame);
                if let SinkDeliveryOutcome::DeadLettered { reason } = &outcome {
                    let plan = &delivery_report.delivery.routed_sinks[sink_index];
                    let record = build_dlq_record(
                        plan,
                        &frame,
                        &event,
                        plan.retry_policy.max_attempts,
                        reason,
                    );
                    self.dlq.enqueue(record.clone())?;
                    dlq_entries.push(record);
                    dlq_total += 1;
                }
                frames.push(frame);
                outcomes.push(outcome);
            }
            dispatched.push(CdcDispatchedEvent {
                event,
                anonymized_columns: applied,
                plan: delivery_report.delivery.clone(),
                frames,
                outcomes,
                dlq_entries,
            });
        }

        Ok(CdcDispatchReport {
            start_lsn: batch.start_lsn,
            end_lsn: batch.end_lsn,
            events: dispatched,
            dlq_total,
            bytes_total,
        })
    }

    fn dispatch_one(&self, plan: &CdcSinkPlan, frame: &SinkWireFrame) -> SinkDeliveryOutcome {
        if !self.config.dispatch_live {
            return SinkDeliveryOutcome::Encoded;
        }
        match plan {
            CdcSinkPlan::Webhook { url, .. } | CdcSinkPlan::Http2 { url, .. } => {
                match crate::dispatch_http1(url, &frame.bytes, &Default::default()) {
                    Ok(summary) => SinkDeliveryOutcome::Delivered {
                        response_summary: summary,
                    },
                    Err(reason) => SinkDeliveryOutcome::DeadLettered { reason },
                }
            }
            CdcSinkPlan::Nats {
                subject,
                server_url,
                ..
            } => match crate::dispatch_nats_pub(server_url, subject, &frame.bytes) {
                Ok(summary) => SinkDeliveryOutcome::Delivered {
                    response_summary: summary,
                },
                Err(reason) => SinkDeliveryOutcome::DeadLettered { reason },
            },
            // Kafka / Kinesis / Pub/Sub require client SDKs that are not
            // bundled into the no-deps build profile. The dispatcher reports
            // them as `Encoded` (the wire frame is real and ready for an
            // operator-provided shipper sidecar to forward); when the
            // operator wires the sidecar to a real broker the dispatcher
            // is swapped out for a network-aware variant.
            _ => SinkDeliveryOutcome::Encoded,
        }
    }
}

/// Convenience function for callers that just want to convert a single
/// pre-decoded event into per-sink frames without instantiating a runtime.
pub fn cdc_runtime_dispatch(
    plan: &CdcSidecarPlan,
    event: &CdcEventEnvelope,
) -> Result<Vec<SinkDispatchReport>, CdcSidecarError> {
    let mut event = event.clone();
    let applied = anon::apply_anonymization(&plan.anonymization, &mut event);
    let payload = CdcEventPayload::encode(&event, &applied);
    let delivery = plan.delivery_plan(&event)?;
    let mut reports = Vec::with_capacity(plan.sinks.len());
    for (sink, sink_plan) in plan.sinks.iter().zip(delivery.routed_sinks.iter()) {
        let frame = encode_sink_frame(sink, &payload, &event)?;
        reports.push(SinkDispatchReport {
            plan: sink_plan.clone(),
            frame,
            outcome: SinkDeliveryOutcome::Encoded,
        });
    }
    Ok(reports)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{canonical_cdc_event, canonical_cdc_plan, canonical_wal2json_frame};

    #[test]
    fn live_runtime_ingests_wal2json_and_emits_per_sink_frames() {
        let config = CdcRuntimeConfig::canonical();
        let mut runtime = CdcLiveRuntime::new(config).expect("runtime");
        let report = runtime
            .ingest_wal2json(&canonical_wal2json_frame())
            .expect("ingest");
        assert_eq!(report.events.len(), 1);
        let event = &report.events[0];
        assert_eq!(event.anonymized_columns, vec!["email".to_string()]);
        assert_eq!(event.frames.len(), 7);
        // Every sink reports an outcome.
        assert_eq!(event.outcomes.len(), 7);
        assert!(event.frames.iter().any(|f| f.sink == "kafka"));
        assert!(event.frames.iter().any(|f| f.sink == "kinesis"));
        assert!(event.frames.iter().any(|f| f.sink == "http2"));
        assert!(report.bytes_total > 0);
    }

    #[test]
    fn live_runtime_dead_letters_unreachable_webhook() {
        let mut config = CdcRuntimeConfig::canonical();
        config.dispatch_live = true;
        // Point the webhook at an unroutable address so the dispatcher fails
        // fast. The DLQ should now have at least one entry.
        for sink in config.plan.sinks.iter_mut() {
            if let CdcSinkPlan::Webhook { url, .. } = sink {
                *url = "http://127.0.0.1:1/cdc".to_string();
            }
            if let CdcSinkPlan::Http2 { url, .. } = sink {
                *url = "http://127.0.0.1:1/cdc".to_string();
            }
            if let CdcSinkPlan::Nats { server_url, .. } = sink {
                *server_url = "nats://127.0.0.1:1".to_string();
            }
        }
        let mut runtime = CdcLiveRuntime::new(config).expect("runtime");
        let report = runtime
            .ingest_wal2json(&canonical_wal2json_frame())
            .expect("ingest");
        assert!(report.dlq_total >= 1);
        assert!(runtime.dlq().len() >= 1);
    }

    #[test]
    fn live_runtime_ingests_pgoutput_logical_frame() {
        let mut config = CdcRuntimeConfig::canonical();
        config.source = CdcReplicationSource::PgOutput;
        let mut runtime = CdcLiveRuntime::new(config).expect("runtime");
        let frame = ReplicationFrame {
            plugin: WalOutputPlugin::PgOutput,
            start_lsn: "16/B374D848".to_string(),
            end_lsn: "16/B374D900".to_string(),
            payload: br#"{"messages":[{"op":"I","schema":"public","table":"orders","columns":[{"name":"id","value":1},{"name":"tenant_id","value":"tenant-a"},{"name":"email","value":"person@example.com"}]}]}"#.to_vec(),
        };
        let report = runtime.ingest_replication_frame(&frame).expect("ingest");
        assert_eq!(report.events.len(), 1);
        assert_eq!(report.events[0].event.tenant_id, "tenant-a");
        assert_eq!(
            report.events[0].anonymized_columns,
            vec!["email".to_string()]
        );
    }

    #[test]
    fn cdc_runtime_dispatch_returns_per_sink_reports() {
        let reports =
            cdc_runtime_dispatch(&canonical_cdc_plan(), &canonical_cdc_event()).expect("dispatch");
        assert_eq!(reports.len(), 7);
        for report in &reports {
            assert_eq!(report.outcome, SinkDeliveryOutcome::Encoded);
            assert!(!report.frame.bytes.is_empty());
        }
    }
}
