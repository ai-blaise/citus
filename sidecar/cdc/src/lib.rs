//! CDC sidecar contracts and real-runtime entry points.
//!
//! The lib exposes three layers:
//! 1. Validation contracts ([`CdcSidecarPlan`], [`LogicalSlotPlan`],
//!    [`CdcSinkPlan`], [`AnonymizationRule`]). Pure data structures with
//!    eager validation and deterministic delivery plans.
//! 2. Wire-level sink encoders ([`sinks`]) that turn an event envelope plus
//!    a sink plan into the exact bytes a real Kafka broker, Kinesis stream,
//!    GCP Pub/Sub topic, NATS subject, webhook endpoint, or HTTP/2 server
//!    would observe.
//! 3. A live runtime ([`live`]) that wires logical replication ingest to
//!    the encoders, applies PII anonymization, and tracks DLQ state.

// FEATURE: C1
// FEATURE: C2
// FEATURE: C3
// FEATURE: C9
// FEATURE: C14
// FEATURE: C15
// FEATURE: L8
// FEATURE: WH3

pub mod anon;
pub mod dlq;
pub mod live;
pub mod nats_sink;
pub mod replication;
pub mod runtime;
pub mod sinks;
pub mod source;

pub use anon::{apply_anonymization, hash_value};
pub use dlq::{build_dlq_record, Dlq, DlqRecord};
pub use live::{
    cdc_runtime_dispatch, CdcDispatchReport, CdcDispatchedEvent, CdcLiveRuntime,
    CdcReplicationSource, CdcRuntimeConfig,
};
pub use sinks::{
    dispatch_http1, dispatch_nats_pub, encode_sink_frame, CdcEventPayload, SinkDeliveryOutcome,
    SinkDispatchReport, SinkWireFrame, SinkWireKind,
};
pub use source::{
    decode_replication_frame, InMemoryReplicationClient, LogicalReplicationClient,
    PgOutputLogicalDecoder, ReplicationCheckpoint, ReplicationFrame, ReplicationStreamConfig,
    Wal2JsonDecoder, WalDecoder,
};

use ai_blaise_citus_sidecar_shared::{
    CdcSink, CdcStreamContract, DeliveryRetryPolicy, SidecarContractError,
};
use serde_json::Value;
use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct CdcSidecarPlan {
    pub stream: CdcStreamContract,
    pub slot: LogicalSlotPlan,
    pub sinks: Vec<CdcSinkPlan>,
    pub schema_capture: Option<SchemaCapturePlan>,
    pub anonymization: Vec<AnonymizationRule>,
}

impl CdcSidecarPlan {
    pub fn validate(&self) -> Result<(), CdcSidecarError> {
        self.stream.validate()?;
        self.slot.validate()?;
        if self.sinks.is_empty() {
            return Err(CdcSidecarError::MissingRequiredField("sinks"));
        }
        for sink in &self.sinks {
            sink.validate()?;
        }
        if let Some(schema_capture) = &self.schema_capture {
            schema_capture.validate()?;
        }
        for rule in &self.anonymization {
            rule.validate()?;
        }
        Ok(())
    }

    pub fn delivery_plan(
        &self,
        event: &CdcEventEnvelope,
    ) -> Result<CdcDeliveryPlan, CdcSidecarError> {
        self.validate()?;
        event.validate()?;

        let mut routed_sinks = Vec::with_capacity(self.sinks.len());
        for sink in &self.sinks {
            routed_sinks.push(SinkDeliveryPlan {
                sink: sink.name().to_string(),
                target: sink.target().to_string(),
                retry_policy: sink.retry_policy().clone(),
            });
        }

        Ok(CdcDeliveryPlan {
            event_lsn: event.lsn.clone(),
            table: format!("{}.{}", event.schema, event.table),
            operation: event.operation,
            routed_sinks,
            anonymized_columns: self
                .anonymization
                .iter()
                .filter(|rule| rule.matches_event(event))
                .map(|rule| rule.column.clone())
                .collect(),
        })
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct LogicalSlotPlan {
    pub slot_name: String,
    pub publication_name: String,
    pub plugin: WalOutputPlugin,
    pub confirmed_flush_lsn: Option<String>,
}

impl LogicalSlotPlan {
    fn validate(&self) -> Result<(), CdcSidecarError> {
        validate_identifier("slot.slot_name", &self.slot_name)?;
        validate_identifier("slot.publication_name", &self.publication_name)?;
        if let Some(lsn) = &self.confirmed_flush_lsn {
            validate_lsn(lsn)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum WalOutputPlugin {
    Wal2Json,
    PgOutput,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum CdcSinkPlan {
    Webhook {
        name: String,
        url: String,
        retry_policy: DeliveryRetryPolicy,
    },
    Realtime {
        name: String,
        topic_prefix: String,
        retry_policy: DeliveryRetryPolicy,
    },
    AnalyticalMirror {
        name: String,
        mirror_name: String,
        storage_uri: String,
        retry_policy: DeliveryRetryPolicy,
    },
    Kafka {
        name: String,
        topic: String,
        bootstrap_servers: String,
        retry_policy: DeliveryRetryPolicy,
    },
    Nats {
        name: String,
        subject: String,
        server_url: String,
        retry_policy: DeliveryRetryPolicy,
    },
    PubSub {
        name: String,
        project_id: String,
        topic: String,
        retry_policy: DeliveryRetryPolicy,
    },
    Kinesis {
        name: String,
        stream_name: String,
        region: String,
        retry_policy: DeliveryRetryPolicy,
    },
    Http2 {
        name: String,
        url: String,
        retry_policy: DeliveryRetryPolicy,
    },
}

impl CdcSinkPlan {
    fn validate(&self) -> Result<(), CdcSidecarError> {
        validate_required("sink.name", self.name())?;
        self.retry_policy().validate()?;

        match self {
            Self::Webhook { url, .. } => validate_http_url("sink.webhook.url", url),
            Self::Realtime { topic_prefix, .. } => {
                validate_required("sink.realtime.topic_prefix", topic_prefix)
            }
            Self::AnalyticalMirror {
                mirror_name,
                storage_uri,
                ..
            } => {
                validate_required("sink.analytical_mirror.mirror_name", mirror_name)?;
                validate_object_uri("sink.analytical_mirror.storage_uri", storage_uri)
            }
            Self::Kafka {
                topic,
                bootstrap_servers,
                ..
            } => {
                validate_required("sink.kafka.topic", topic)?;
                validate_required("sink.kafka.bootstrap_servers", bootstrap_servers)
            }
            Self::Nats {
                subject,
                server_url,
                ..
            } => {
                validate_nats_subject("sink.nats.subject", subject)?;
                validate_nats_url(server_url)
            }
            Self::PubSub {
                project_id, topic, ..
            } => {
                validate_pubsub_project_id("sink.pubsub.project_id", project_id)?;
                validate_pubsub_topic("sink.pubsub.topic", topic)
            }
            Self::Kinesis {
                stream_name,
                region,
                ..
            } => {
                validate_required("sink.kinesis.stream_name", stream_name)?;
                validate_required("sink.kinesis.region", region)
            }
            Self::Http2 { url, .. } => validate_http_url("sink.http2.url", url),
        }
    }

    fn name(&self) -> &str {
        match self {
            Self::Webhook { name, .. }
            | Self::Realtime { name, .. }
            | Self::AnalyticalMirror { name, .. }
            | Self::Kafka { name, .. }
            | Self::Nats { name, .. }
            | Self::PubSub { name, .. }
            | Self::Kinesis { name, .. }
            | Self::Http2 { name, .. } => name,
        }
    }

    fn target(&self) -> &str {
        match self {
            Self::Webhook { url, .. } => url,
            Self::Realtime { topic_prefix, .. } => topic_prefix,
            Self::AnalyticalMirror { mirror_name, .. } => mirror_name,
            Self::Kafka { topic, .. } => topic,
            Self::Nats { subject, .. } => subject,
            Self::PubSub { topic, .. } => topic,
            Self::Kinesis { stream_name, .. } => stream_name,
            Self::Http2 { url, .. } => url,
        }
    }

    fn retry_policy(&self) -> &DeliveryRetryPolicy {
        match self {
            Self::Webhook { retry_policy, .. }
            | Self::Realtime { retry_policy, .. }
            | Self::AnalyticalMirror { retry_policy, .. }
            | Self::Kafka { retry_policy, .. }
            | Self::Nats { retry_policy, .. }
            | Self::PubSub { retry_policy, .. }
            | Self::Kinesis { retry_policy, .. }
            | Self::Http2 { retry_policy, .. } => retry_policy,
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SchemaCapturePlan {
    pub ddl_stream_table: String,
    pub include_schemas: Vec<String>,
    pub fail_on_parse_error: bool,
}

impl SchemaCapturePlan {
    fn validate(&self) -> Result<(), CdcSidecarError> {
        validate_qualified_name("schema_capture.ddl_stream_table", &self.ddl_stream_table)?;
        validate_required_list("schema_capture.include_schemas", &self.include_schemas)
    }

    /// FEATURE: C2 -- parse rows from the configured DDL stream table after
    /// they traverse the same wal2json/pgoutput replication boundary as row
    /// changes. Missing or malformed DDL rows fail closed when the plan asks
    /// for strict parsing, so consumers do not silently miss schema changes.
    pub fn parse_ddl_stream_event(
        &self,
        event: &CdcEventEnvelope,
    ) -> Result<Option<DdlStreamEvent>, CdcSidecarError> {
        self.validate()?;
        let Some((stream_schema, stream_table)) = self.ddl_stream_table.split_once('.') else {
            return Err(CdcSidecarError::InvalidIdentifier(
                "schema_capture.ddl_stream_table",
            ));
        };
        if event.schema != stream_schema || event.table != stream_table {
            return Ok(None);
        }

        let parsed = DdlStreamEvent::from_cdc_event(event, &self.ddl_stream_table);
        match parsed {
            Ok(ddl_event) => {
                if self
                    .include_schemas
                    .iter()
                    .any(|schema| schema == &ddl_event.object_schema)
                {
                    Ok(Some(ddl_event))
                } else {
                    Ok(None)
                }
            }
            Err(error) if self.fail_on_parse_error => Err(error),
            Err(_) => Ok(None),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct DdlStreamEvent {
    pub lsn: String,
    pub ddl_stream_table: String,
    pub command_tag: String,
    pub object_schema: String,
    pub object_identity: String,
    pub ddl: String,
    pub occurred_at: String,
}

impl DdlStreamEvent {
    fn from_cdc_event(
        event: &CdcEventEnvelope,
        ddl_stream_table: &str,
    ) -> Result<Self, CdcSidecarError> {
        let ddl_event = Self {
            lsn: event.lsn.clone(),
            ddl_stream_table: ddl_stream_table.to_string(),
            command_tag: required_column(event, "command_tag")?,
            object_schema: required_column(event, "object_schema")?,
            object_identity: required_column(event, "object_identity")?,
            ddl: required_column(event, "ddl")?,
            occurred_at: required_column(event, "occurred_at")?,
        };
        ddl_event.validate()?;
        Ok(ddl_event)
    }

    fn validate(&self) -> Result<(), CdcSidecarError> {
        validate_lsn(&self.lsn)?;
        validate_qualified_name("ddl_event.ddl_stream_table", &self.ddl_stream_table)?;
        validate_required("ddl_event.command_tag", &self.command_tag)?;
        validate_identifier("ddl_event.object_schema", &self.object_schema)?;
        validate_required("ddl_event.object_identity", &self.object_identity)?;
        validate_required("ddl_event.ddl", &self.ddl)?;
        validate_required("ddl_event.occurred_at", &self.occurred_at)
    }
}

fn required_column(
    event: &CdcEventEnvelope,
    name: &'static str,
) -> Result<String, CdcSidecarError> {
    event
        .columns
        .iter()
        .find(|column| column.name == name)
        .and_then(|column| column.value.clone())
        .filter(|value| !value.trim().is_empty())
        .ok_or(CdcSidecarError::MissingRequiredField(name))
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct AnonymizationRule {
    pub schema: String,
    pub table: String,
    pub column: String,
    pub strategy: AnonymizationStrategy,
}

impl AnonymizationRule {
    fn validate(&self) -> Result<(), CdcSidecarError> {
        validate_identifier("anonymization.schema", &self.schema)?;
        validate_identifier("anonymization.table", &self.table)?;
        validate_identifier("anonymization.column", &self.column)
    }

    fn matches_event(&self, event: &CdcEventEnvelope) -> bool {
        self.schema == event.schema
            && self.table == event.table
            && event
                .columns
                .iter()
                .any(|column| column.name == self.column)
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum AnonymizationStrategy {
    Redact,
    Hash,
    Null,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct CdcEventEnvelope {
    pub lsn: String,
    pub schema: String,
    pub table: String,
    pub tenant_id: String,
    pub operation: CdcOperation,
    pub columns: Vec<CdcColumnValue>,
}

impl CdcEventEnvelope {
    pub fn validate(&self) -> Result<(), CdcSidecarError> {
        validate_lsn(&self.lsn)?;
        validate_identifier("event.schema", &self.schema)?;
        validate_identifier("event.table", &self.table)?;
        validate_required("event.tenant_id", &self.tenant_id)?;
        if self.columns.is_empty() {
            return Err(CdcSidecarError::MissingRequiredField("event.columns"));
        }
        for column in &self.columns {
            column.validate()?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum CdcOperation {
    Insert,
    Update,
    Delete,
    Truncate,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct CdcColumnValue {
    pub name: String,
    pub value: Option<String>,
}

impl CdcColumnValue {
    fn validate(&self) -> Result<(), CdcSidecarError> {
        validate_identifier("event.column.name", &self.name)
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct CdcDeliveryPlan {
    pub event_lsn: String,
    pub table: String,
    pub operation: CdcOperation,
    pub routed_sinks: Vec<SinkDeliveryPlan>,
    pub anonymized_columns: Vec<String>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SinkDeliveryPlan {
    pub sink: String,
    pub target: String,
    pub retry_policy: DeliveryRetryPolicy,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct LogicalReplicationFrame {
    pub start_lsn: String,
    pub end_lsn: String,
    pub payload: String,
}

impl LogicalReplicationFrame {
    pub fn validate(&self) -> Result<(), CdcSidecarError> {
        validate_lsn(&self.start_lsn)?;
        validate_lsn(&self.end_lsn)?;
        validate_required("replication.payload", &self.payload)?;
        if lsn_to_u64(&self.end_lsn)? < lsn_to_u64(&self.start_lsn)? {
            return Err(CdcSidecarError::InvalidLsn);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ReplicationAck {
    pub write_lsn: String,
    pub flush_lsn: String,
    pub apply_lsn: String,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct CdcRuntimeState {
    pub slot_name: String,
    pub last_received_lsn: String,
    pub last_delivered_lsn: String,
    pub acked_flush_lsn: String,
    pub delivered_events: u64,
    pub delivered_sink_writes: u64,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct CdcRuntimeDelivery {
    pub event: CdcEventEnvelope,
    pub delivery: CdcDeliveryPlan,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct CdcRuntimeBatch {
    pub start_lsn: String,
    pub end_lsn: String,
    pub deliveries: Vec<CdcRuntimeDelivery>,
    pub ack: ReplicationAck,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct CdcRuntimeReport {
    pub batch: CdcRuntimeBatch,
    pub state: CdcRuntimeState,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct CdcRuntime {
    plan: CdcSidecarPlan,
    state: CdcRuntimeState,
}

impl CdcRuntime {
    pub fn new(plan: CdcSidecarPlan) -> Result<Self, CdcSidecarError> {
        plan.validate()?;
        let checkpoint = plan
            .slot
            .confirmed_flush_lsn
            .clone()
            .unwrap_or_else(|| "0/0".to_string());
        validate_lsn(&checkpoint)?;

        Ok(Self {
            state: CdcRuntimeState {
                slot_name: plan.slot.slot_name.clone(),
                last_received_lsn: checkpoint.clone(),
                last_delivered_lsn: checkpoint.clone(),
                acked_flush_lsn: checkpoint,
                delivered_events: 0,
                delivered_sink_writes: 0,
            },
            plan,
        })
    }

    pub fn state(&self) -> &CdcRuntimeState {
        &self.state
    }

    pub fn apply_wal2json_frame(
        &mut self,
        frame: &LogicalReplicationFrame,
    ) -> Result<CdcRuntimeBatch, CdcSidecarError> {
        frame.validate()?;
        let events = decode_wal2json_frame(frame)?;
        if events.is_empty() {
            return Err(CdcSidecarError::MissingRequiredField("wal2json.change"));
        }

        let mut deliveries = Vec::with_capacity(events.len());
        let mut delivered_sink_writes = 0_u64;
        for mut event in events {
            // Client-side defense in depth: rewrite values before the dispatch
            // plan is materialized, so the runtime cannot accidentally encode
            // raw PII for any downstream sink.
            anon::apply_anonymization(&self.plan.anonymization, &mut event);
            let delivery = self.plan.delivery_plan(&event)?;
            delivered_sink_writes += delivery.routed_sinks.len() as u64;
            deliveries.push(CdcRuntimeDelivery { event, delivery });
        }

        self.state.last_received_lsn = frame.end_lsn.clone();
        self.state.last_delivered_lsn = frame.end_lsn.clone();
        self.state.acked_flush_lsn = frame.end_lsn.clone();
        self.state.delivered_events += deliveries.len() as u64;
        self.state.delivered_sink_writes += delivered_sink_writes;

        Ok(CdcRuntimeBatch {
            start_lsn: frame.start_lsn.clone(),
            end_lsn: frame.end_lsn.clone(),
            deliveries,
            ack: ReplicationAck {
                write_lsn: frame.end_lsn.clone(),
                flush_lsn: frame.end_lsn.clone(),
                apply_lsn: frame.end_lsn.clone(),
            },
        })
    }

    /// Advance the runtime using a pre-decoded set of events. The live
    /// runtime uses this to feed already-anonymized events through the
    /// contract layer without re-decoding the WAL frame twice.
    pub fn advance_with_events(
        &mut self,
        frame: &LogicalReplicationFrame,
        events: &[CdcEventEnvelope],
    ) -> Result<CdcRuntimeBatch, CdcSidecarError> {
        frame.validate()?;
        if events.is_empty() {
            return Err(CdcSidecarError::MissingRequiredField("wal2json.change"));
        }
        let mut deliveries = Vec::with_capacity(events.len());
        let mut delivered_sink_writes = 0_u64;
        for event in events {
            let delivery = self.plan.delivery_plan(event)?;
            delivered_sink_writes += delivery.routed_sinks.len() as u64;
            deliveries.push(CdcRuntimeDelivery {
                event: event.clone(),
                delivery,
            });
        }
        self.state.last_received_lsn = frame.end_lsn.clone();
        self.state.last_delivered_lsn = frame.end_lsn.clone();
        self.state.acked_flush_lsn = frame.end_lsn.clone();
        self.state.delivered_events += deliveries.len() as u64;
        self.state.delivered_sink_writes += delivered_sink_writes;
        Ok(CdcRuntimeBatch {
            start_lsn: frame.start_lsn.clone(),
            end_lsn: frame.end_lsn.clone(),
            deliveries,
            ack: ReplicationAck {
                write_lsn: frame.end_lsn.clone(),
                flush_lsn: frame.end_lsn.clone(),
                apply_lsn: frame.end_lsn.clone(),
            },
        })
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum CdcSidecarError {
    ColumnValueMismatch,
    InvalidHttpUrl(&'static str),
    InvalidIdentifier(&'static str),
    InvalidLsn,
    InvalidNatsUrl,
    InvalidSinkConfig(&'static str),
    InvalidObjectUri(&'static str),
    InvalidPgOutput(String),
    InvalidWal2Json(String),
    MissingRequiredField(&'static str),
    SharedContract(String),
    UnsupportedOperation(String),
}

impl fmt::Display for CdcSidecarError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ColumnValueMismatch => {
                write!(
                    formatter,
                    "wal2json columnnames and columnvalues lengths differ"
                )
            }
            Self::InvalidHttpUrl(field) => {
                write!(formatter, "{field} must start with http:// or https://")
            }
            Self::InvalidIdentifier(field) => {
                write!(formatter, "{field} must be a non-empty SQL identifier")
            }
            Self::InvalidLsn => write!(formatter, "LSN must use the PostgreSQL HEX/HEX form"),
            Self::InvalidNatsUrl => write!(formatter, "NATS server URL must start with nats://"),
            Self::InvalidSinkConfig(field) => {
                write!(formatter, "{field} contains invalid sink routing syntax")
            }
            Self::InvalidObjectUri(field) => write!(formatter, "{field} must be an object URI"),
            Self::InvalidPgOutput(error) => write!(formatter, "invalid pgoutput payload: {error}"),
            Self::InvalidWal2Json(error) => write!(formatter, "invalid wal2json payload: {error}"),
            Self::MissingRequiredField(field) => write!(formatter, "{field} must not be empty"),
            Self::SharedContract(error) => write!(formatter, "{error}"),
            Self::UnsupportedOperation(operation) => {
                write!(formatter, "unsupported replication operation: {operation}")
            }
        }
    }
}

impl Error for CdcSidecarError {}

impl From<SidecarContractError> for CdcSidecarError {
    fn from(error: SidecarContractError) -> Self {
        Self::SharedContract(error.to_string())
    }
}

fn validate_required(field: &'static str, value: &str) -> Result<(), CdcSidecarError> {
    if value.trim().is_empty() {
        return Err(CdcSidecarError::MissingRequiredField(field));
    }
    Ok(())
}

fn validate_required_list(field: &'static str, values: &[String]) -> Result<(), CdcSidecarError> {
    if values.is_empty() || values.iter().any(|value| value.trim().is_empty()) {
        return Err(CdcSidecarError::MissingRequiredField(field));
    }
    Ok(())
}

fn validate_identifier(field: &'static str, value: &str) -> Result<(), CdcSidecarError> {
    validate_required(field, value)?;
    if value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
    {
        Ok(())
    } else {
        Err(CdcSidecarError::InvalidIdentifier(field))
    }
}

fn validate_qualified_name(field: &'static str, value: &str) -> Result<(), CdcSidecarError> {
    validate_required(field, value)?;
    let parts: Vec<_> = value.split('.').collect();
    if parts.len() == 2
        && parts
            .iter()
            .all(|part| validate_identifier(field, part).is_ok())
    {
        Ok(())
    } else {
        Err(CdcSidecarError::InvalidIdentifier(field))
    }
}

fn validate_lsn(value: &str) -> Result<(), CdcSidecarError> {
    validate_required("lsn", value)?;
    let parts: Vec<_> = value.split('/').collect();
    if parts.len() == 2
        && parts
            .iter()
            .all(|part| !part.is_empty() && part.chars().all(|ch| ch.is_ascii_hexdigit()))
    {
        Ok(())
    } else {
        Err(CdcSidecarError::InvalidLsn)
    }
}

fn lsn_to_u64(value: &str) -> Result<u64, CdcSidecarError> {
    validate_lsn(value)?;
    let (high, low) = value.split_once('/').ok_or(CdcSidecarError::InvalidLsn)?;
    let high = u64::from_str_radix(high, 16).map_err(|_| CdcSidecarError::InvalidLsn)?;
    let low = u64::from_str_radix(low, 16).map_err(|_| CdcSidecarError::InvalidLsn)?;
    high.checked_shl(32)
        .and_then(|shifted| shifted.checked_add(low))
        .ok_or(CdcSidecarError::InvalidLsn)
}

fn validate_http_url(field: &'static str, value: &str) -> Result<(), CdcSidecarError> {
    validate_required(field, value)?;
    if value.starts_with("http://") || value.starts_with("https://") {
        Ok(())
    } else {
        Err(CdcSidecarError::InvalidHttpUrl(field))
    }
}

fn validate_object_uri(field: &'static str, value: &str) -> Result<(), CdcSidecarError> {
    validate_required(field, value)?;
    if value.starts_with("s3://") || value.starts_with("gs://") || value.starts_with("az://") {
        Ok(())
    } else {
        Err(CdcSidecarError::InvalidObjectUri(field))
    }
}

fn validate_nats_url(value: &str) -> Result<(), CdcSidecarError> {
    validate_required("sink.nats.server_url", value)?;
    let Some(host_port) = value.strip_prefix("nats://") else {
        return Err(CdcSidecarError::InvalidNatsUrl);
    };
    if host_port.is_empty()
        || host_port.contains('@')
        || host_port.contains('/')
        || host_port.contains('?')
        || host_port.chars().any(char::is_whitespace)
    {
        Err(CdcSidecarError::InvalidNatsUrl)
    } else {
        Ok(())
    }
}

fn validate_nats_subject(field: &'static str, value: &str) -> Result<(), CdcSidecarError> {
    validate_required(field, value)?;
    let tokens = value.split('.').collect::<Vec<_>>();
    if tokens.iter().any(|token| token.is_empty()) {
        return Err(CdcSidecarError::InvalidSinkConfig(field));
    }
    if value.chars().all(is_nats_subject_char)
        && !tokens.iter().any(|token| *token == "*" || *token == ">")
    {
        Ok(())
    } else {
        Err(CdcSidecarError::InvalidSinkConfig(field))
    }
}

fn is_nats_subject_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.' | '/' | ':')
}

fn validate_pubsub_project_id(field: &'static str, value: &str) -> Result<(), CdcSidecarError> {
    validate_required(field, value)?;
    let bytes = value.as_bytes();
    if !(6..=30).contains(&bytes.len()) {
        return Err(CdcSidecarError::InvalidSinkConfig(field));
    }
    let last = bytes[bytes.len() - 1];
    if !bytes[0].is_ascii_lowercase() || !(last.is_ascii_lowercase() || last.is_ascii_digit()) {
        return Err(CdcSidecarError::InvalidSinkConfig(field));
    }
    if bytes
        .iter()
        .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || *byte == b'-')
    {
        Ok(())
    } else {
        Err(CdcSidecarError::InvalidSinkConfig(field))
    }
}

fn validate_pubsub_topic(field: &'static str, value: &str) -> Result<(), CdcSidecarError> {
    validate_required(field, value)?;
    let bytes = value.as_bytes();
    if !(3..=255).contains(&bytes.len()) || value.starts_with("goog") {
        return Err(CdcSidecarError::InvalidSinkConfig(field));
    }
    if bytes[0].is_ascii_alphabetic()
        && bytes.iter().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(*byte, b'-' | b'_' | b'.' | b'~' | b'+' | b'%')
        })
    {
        Ok(())
    } else {
        Err(CdcSidecarError::InvalidSinkConfig(field))
    }
}

pub fn decode_wal2json_frame(
    frame: &LogicalReplicationFrame,
) -> Result<Vec<CdcEventEnvelope>, CdcSidecarError> {
    frame.validate()?;
    let root: Value = serde_json::from_str(&frame.payload)
        .map_err(|error| CdcSidecarError::InvalidWal2Json(error.to_string()))?;
    let changes = json_array(&root, "change")?;
    let mut events = Vec::with_capacity(changes.len());

    for change in changes {
        let operation = operation_from_wal2json(json_string(change, "kind")?)?;
        let schema = json_string(change, "schema")?.to_string();
        let table = json_string(change, "table")?.to_string();
        let column_names = json_string_array(change, "columnnames")?;
        let column_values = json_value_array(change, "columnvalues")?;
        if column_names.len() != column_values.len() {
            return Err(CdcSidecarError::ColumnValueMismatch);
        }

        let mut columns = Vec::with_capacity(column_names.len());
        let mut tenant_id = None;
        for (name, value) in column_names.into_iter().zip(column_values) {
            let value = json_scalar_to_string(value)?;
            if name == "tenant_id" {
                tenant_id = value.clone();
            }
            columns.push(CdcColumnValue { name, value });
        }

        let event = CdcEventEnvelope {
            lsn: frame.end_lsn.clone(),
            schema,
            table,
            tenant_id: tenant_id.ok_or(CdcSidecarError::MissingRequiredField("event.tenant_id"))?,
            operation,
            columns,
        };
        event.validate()?;
        events.push(event);
    }

    Ok(events)
}

fn operation_from_wal2json(operation: &str) -> Result<CdcOperation, CdcSidecarError> {
    match operation {
        "insert" => Ok(CdcOperation::Insert),
        "update" => Ok(CdcOperation::Update),
        "delete" => Ok(CdcOperation::Delete),
        "truncate" => Ok(CdcOperation::Truncate),
        other => Err(CdcSidecarError::UnsupportedOperation(other.to_string())),
    }
}

fn json_array<'a>(value: &'a Value, field: &'static str) -> Result<&'a [Value], CdcSidecarError> {
    value
        .get(field)
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .ok_or(CdcSidecarError::MissingRequiredField(field))
}

fn json_string<'a>(value: &'a Value, field: &'static str) -> Result<&'a str, CdcSidecarError> {
    value
        .get(field)
        .and_then(Value::as_str)
        .ok_or(CdcSidecarError::MissingRequiredField(field))
}

fn json_string_array(value: &Value, field: &'static str) -> Result<Vec<String>, CdcSidecarError> {
    json_array(value, field)?
        .iter()
        .map(|item| {
            item.as_str()
                .map(ToString::to_string)
                .ok_or(CdcSidecarError::MissingRequiredField(field))
        })
        .collect()
}

fn json_value_array<'a>(
    value: &'a Value,
    field: &'static str,
) -> Result<Vec<&'a Value>, CdcSidecarError> {
    Ok(json_array(value, field)?.iter().collect())
}

fn json_scalar_to_string(value: &Value) -> Result<Option<String>, CdcSidecarError> {
    Ok(match value {
        Value::Null => None,
        Value::Bool(value) => Some(value.to_string()),
        Value::Number(value) => Some(value.to_string()),
        Value::String(value) => Some(value.clone()),
        _ => {
            return Err(CdcSidecarError::InvalidWal2Json(
                "columnvalues must contain scalar values".to_string(),
            ))
        }
    })
}

pub fn sink_plan_from_shared(
    sink: CdcSink,
    retry_policy: DeliveryRetryPolicy,
) -> Result<CdcSinkPlan, CdcSidecarError> {
    let plan = match sink {
        CdcSink::Webhook { url } => CdcSinkPlan::Webhook {
            name: "webhook".to_string(),
            url,
            retry_policy,
        },
        CdcSink::Realtime { topic_prefix } => CdcSinkPlan::Realtime {
            name: "realtime".to_string(),
            topic_prefix,
            retry_policy,
        },
        CdcSink::AnalyticalMirror { stream_name } => CdcSinkPlan::AnalyticalMirror {
            name: "analytical-mirror".to_string(),
            mirror_name: stream_name,
            storage_uri: "s3://analytical-mirror/default".to_string(),
            retry_policy,
        },
        CdcSink::Kafka { topic } => CdcSinkPlan::Kafka {
            name: "kafka".to_string(),
            topic,
            bootstrap_servers: "localhost:9092".to_string(),
            retry_policy,
        },
        CdcSink::Nats { subject } => CdcSinkPlan::Nats {
            name: "nats".to_string(),
            subject,
            server_url: "nats://localhost:4222".to_string(),
            retry_policy,
        },
        CdcSink::PubSub { project_id, topic } => CdcSinkPlan::PubSub {
            name: "pubsub".to_string(),
            project_id,
            topic,
            retry_policy,
        },
    };
    plan.validate()?;
    Ok(plan)
}

pub fn canonical_retry_policy() -> DeliveryRetryPolicy {
    DeliveryRetryPolicy {
        max_attempts: 5,
        dead_letter_queue: "cdc.dead_letters".to_string(),
    }
}

pub fn canonical_slot_plan() -> LogicalSlotPlan {
    LogicalSlotPlan {
        slot_name: "ai_blaise_cdc".to_string(),
        publication_name: "ai_blaise_publication".to_string(),
        plugin: WalOutputPlugin::Wal2Json,
        confirmed_flush_lsn: Some("16/B374D848".to_string()),
    }
}

pub fn canonical_cdc_plan() -> CdcSidecarPlan {
    CdcSidecarPlan {
        stream: CdcStreamContract {
            slot_name: "ai_blaise_cdc".to_string(),
            publication_name: "ai_blaise_publication".to_string(),
            sinks: vec![
                CdcSink::Nats {
                    subject: "tenant.orders".to_string(),
                },
                CdcSink::PubSub {
                    project_id: "analytics-prod".to_string(),
                    topic: "orders".to_string(),
                },
            ],
            retry_policy: canonical_retry_policy(),
        },
        slot: canonical_slot_plan(),
        sinks: vec![
            CdcSinkPlan::Webhook {
                name: "webhook".to_string(),
                url: "https://hooks.example.com/orders".to_string(),
                retry_policy: canonical_retry_policy(),
            },
            CdcSinkPlan::Realtime {
                name: "realtime".to_string(),
                topic_prefix: "tenant.orders".to_string(),
                retry_policy: canonical_retry_policy(),
            },
            CdcSinkPlan::Nats {
                name: "nats".to_string(),
                subject: "tenant.orders".to_string(),
                server_url: "nats://nats.cdc.svc:4222".to_string(),
                retry_policy: canonical_retry_policy(),
            },
            CdcSinkPlan::PubSub {
                name: "pubsub".to_string(),
                project_id: "analytics-prod".to_string(),
                topic: "orders".to_string(),
                retry_policy: canonical_retry_policy(),
            },
            CdcSinkPlan::Kafka {
                name: "kafka".to_string(),
                topic: "cdc.orders".to_string(),
                bootstrap_servers: "kafka.cdc.svc:9092".to_string(),
                retry_policy: canonical_retry_policy(),
            },
            CdcSinkPlan::Kinesis {
                name: "kinesis".to_string(),
                stream_name: "cdc-orders".to_string(),
                region: "us-east-1".to_string(),
                retry_policy: canonical_retry_policy(),
            },
            CdcSinkPlan::Http2 {
                name: "http2".to_string(),
                url: "https://h2.example.com/cdc/orders".to_string(),
                retry_policy: canonical_retry_policy(),
            },
        ],
        schema_capture: Some(SchemaCapturePlan {
            ddl_stream_table: "cdc.ddl_events".to_string(),
            include_schemas: vec!["public".to_string()],
            fail_on_parse_error: true,
        }),
        anonymization: vec![AnonymizationRule {
            schema: "public".to_string(),
            table: "orders".to_string(),
            column: "email".to_string(),
            strategy: AnonymizationStrategy::Hash,
        }],
    }
}

pub fn canonical_cdc_event() -> CdcEventEnvelope {
    CdcEventEnvelope {
        lsn: "16/B374D848".to_string(),
        schema: "public".to_string(),
        table: "orders".to_string(),
        tenant_id: "tenant-a".to_string(),
        operation: CdcOperation::Insert,
        columns: vec![
            CdcColumnValue {
                name: "id".to_string(),
                value: Some("1".to_string()),
            },
            CdcColumnValue {
                name: "status".to_string(),
                value: Some("paid".to_string()),
            },
            CdcColumnValue {
                name: "email".to_string(),
                value: Some("person@example.com".to_string()),
            },
        ],
    }
}

pub fn canonical_delivery_plan() -> Result<CdcDeliveryPlan, CdcSidecarError> {
    canonical_cdc_plan().delivery_plan(&canonical_cdc_event())
}

pub fn canonical_wal2json_frame() -> LogicalReplicationFrame {
    LogicalReplicationFrame {
        start_lsn: "16/B374D848".to_string(),
        end_lsn: "16/B374D900".to_string(),
        payload: r#"{"change":[{"kind":"insert","schema":"public","table":"orders","columnnames":["id","tenant_id","status","email"],"columnvalues":[1,"tenant-a","paid","person@example.com"]}]}"#.to_string(),
    }
}

pub fn canonical_cdc_runtime_report() -> Result<CdcRuntimeReport, CdcSidecarError> {
    let mut runtime = CdcRuntime::new(canonical_cdc_plan())?;
    let batch = runtime.apply_wal2json_frame(&canonical_wal2json_frame())?;

    Ok(CdcRuntimeReport {
        batch,
        state: runtime.state().clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cdc_sidecar_plan_routes_event_to_all_sinks() {
        let plan = canonical_cdc_plan();
        let delivery = plan
            .delivery_plan(&canonical_cdc_event())
            .expect("delivery plan should route");

        assert_eq!(delivery.event_lsn, "16/B374D848");
        assert_eq!(delivery.table, "public.orders");
        assert_eq!(delivery.routed_sinks.len(), 7);
        assert_eq!(delivery.anonymized_columns, vec!["email".to_string()]);
    }

    #[test]
    fn canonical_delivery_plan_is_deterministic() {
        let delivery = canonical_delivery_plan().expect("canonical delivery");

        assert_eq!(
            delivery
                .routed_sinks
                .iter()
                .map(|sink| sink.sink.as_str())
                .collect::<Vec<_>>(),
            vec!["webhook", "realtime", "nats", "pubsub", "kafka", "kinesis", "http2"]
        );
        assert_eq!(delivery.anonymized_columns, vec!["email".to_string()]);
    }

    #[test]
    fn kinesis_sink_requires_stream_name_and_region() {
        let bad_stream = CdcSinkPlan::Kinesis {
            name: "kinesis".to_string(),
            stream_name: " ".to_string(),
            region: "us-east-1".to_string(),
            retry_policy: canonical_retry_policy(),
        };
        assert_eq!(
            bad_stream.validate(),
            Err(CdcSidecarError::MissingRequiredField(
                "sink.kinesis.stream_name"
            ))
        );
        let bad_region = CdcSinkPlan::Kinesis {
            name: "kinesis".to_string(),
            stream_name: "orders".to_string(),
            region: " ".to_string(),
            retry_policy: canonical_retry_policy(),
        };
        assert_eq!(
            bad_region.validate(),
            Err(CdcSidecarError::MissingRequiredField("sink.kinesis.region"))
        );
    }

    #[test]
    fn http2_sink_requires_http_url() {
        let plan = CdcSinkPlan::Http2 {
            name: "http2".to_string(),
            url: "ftp://example.com".to_string(),
            retry_policy: canonical_retry_policy(),
        };
        assert_eq!(
            plan.validate(),
            Err(CdcSidecarError::InvalidHttpUrl("sink.http2.url"))
        );
    }

    #[test]
    fn wal2json_frame_decodes_to_cdc_event() {
        let events = decode_wal2json_frame(&canonical_wal2json_frame()).expect("decode frame");

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].lsn, "16/B374D900");
        assert_eq!(events[0].tenant_id, "tenant-a");
        assert_eq!(events[0].operation, CdcOperation::Insert);
        assert_eq!(events[0].columns[2].value.as_deref(), Some("paid"));
    }

    #[test]
    fn runtime_applies_wal2json_frame_and_advances_ack() {
        let report = canonical_cdc_runtime_report().expect("runtime report");

        assert_eq!(report.batch.deliveries.len(), 1);
        assert_eq!(report.batch.deliveries[0].delivery.routed_sinks.len(), 7);
        assert_eq!(report.batch.ack.flush_lsn, "16/B374D900");
        assert_eq!(report.state.acked_flush_lsn, "16/B374D900");
        assert_eq!(report.state.delivered_events, 1);
        assert_eq!(report.state.delivered_sink_writes, 7);
    }

    #[test]
    fn runtime_applies_anonymization_to_event_before_delivery() {
        let mut runtime = CdcRuntime::new(canonical_cdc_plan()).expect("runtime");
        let batch = runtime
            .apply_wal2json_frame(&canonical_wal2json_frame())
            .expect("apply frame");
        let email_column = batch.deliveries[0]
            .event
            .columns
            .iter()
            .find(|column| column.name == "email")
            .expect("email column");
        assert!(
            email_column
                .value
                .as_deref()
                .unwrap_or("")
                .starts_with("anon_"),
            "expected anonymized email, got {:?}",
            email_column.value
        );
    }

    #[test]
    fn wal2json_frame_rejects_mismatched_column_values() {
        let mut frame = canonical_wal2json_frame();
        frame.payload = r#"{"change":[{"kind":"insert","schema":"public","table":"orders","columnnames":["id","tenant_id"],"columnvalues":[1]}]}"#.to_string();

        assert_eq!(
            decode_wal2json_frame(&frame),
            Err(CdcSidecarError::ColumnValueMismatch)
        );
    }

    #[test]
    fn logical_slot_rejects_invalid_lsn() {
        let mut plan = canonical_slot_plan();
        plan.confirmed_flush_lsn = Some("not-an-lsn".to_string());

        assert_eq!(plan.validate(), Err(CdcSidecarError::InvalidLsn));
    }

    #[test]
    fn nats_sink_requires_nats_url() {
        let sink = CdcSinkPlan::Nats {
            name: "nats".to_string(),
            subject: "tenant.orders".to_string(),
            server_url: "http://nats".to_string(),
            retry_policy: canonical_retry_policy(),
        };

        assert_eq!(sink.validate(), Err(CdcSidecarError::InvalidNatsUrl));
    }

    #[test]
    fn nats_sink_rejects_unsupported_auth_urls() {
        let sink = CdcSinkPlan::Nats {
            name: "nats".to_string(),
            subject: "tenant.orders".to_string(),
            server_url: "nats://user:pass@nats:4222".to_string(),
            retry_policy: canonical_retry_policy(),
        };

        assert_eq!(sink.validate(), Err(CdcSidecarError::InvalidNatsUrl));
    }

    #[test]
    fn nats_sink_rejects_protocol_injection_subjects() {
        for subject in [
            "tenant.orders\r\nPING",
            "tenant orders",
            "tenant.*",
            "tenant.>",
            ".tenant.orders",
            "tenant..orders",
        ] {
            let sink = CdcSinkPlan::Nats {
                name: "nats".to_string(),
                subject: subject.to_string(),
                server_url: "nats://nats:4222".to_string(),
                retry_policy: canonical_retry_policy(),
            };

            assert_eq!(
                sink.validate(),
                Err(CdcSidecarError::InvalidSinkConfig("sink.nats.subject")),
                "subject {subject:?} should fail closed"
            );
        }
    }

    #[test]
    fn pubsub_sink_requires_project_id() {
        let sink = CdcSinkPlan::PubSub {
            name: "pubsub".to_string(),
            project_id: " ".to_string(),
            topic: "orders".to_string(),
            retry_policy: canonical_retry_policy(),
        };

        assert_eq!(
            sink.validate(),
            Err(CdcSidecarError::MissingRequiredField(
                "sink.pubsub.project_id"
            ))
        );
    }

    #[test]
    fn pubsub_sink_rejects_invalid_project_and_topic_names() {
        let bad_project = CdcSinkPlan::PubSub {
            name: "pubsub".to_string(),
            project_id: "AnalyticsProd".to_string(),
            topic: "orders".to_string(),
            retry_policy: canonical_retry_policy(),
        };

        assert_eq!(
            bad_project.validate(),
            Err(CdcSidecarError::InvalidSinkConfig("sink.pubsub.project_id"))
        );

        let bad_topic = CdcSinkPlan::PubSub {
            name: "pubsub".to_string(),
            project_id: "analytics-prod".to_string(),
            topic: "googManaged".to_string(),
            retry_policy: canonical_retry_policy(),
        };

        assert_eq!(
            bad_topic.validate(),
            Err(CdcSidecarError::InvalidSinkConfig("sink.pubsub.topic"))
        );
    }

    #[test]
    fn schema_capture_requires_qualified_table() {
        let schema_capture = SchemaCapturePlan {
            ddl_stream_table: "ddl_events".to_string(),
            include_schemas: vec!["public".to_string()],
            fail_on_parse_error: true,
        };

        assert_eq!(
            schema_capture.validate(),
            Err(CdcSidecarError::InvalidIdentifier(
                "schema_capture.ddl_stream_table"
            ))
        );
    }

    #[test]
    fn schema_capture_parses_ddl_stream_event() {
        let plan = SchemaCapturePlan {
            ddl_stream_table: "cdc.ddl_events".to_string(),
            include_schemas: vec!["public".to_string()],
            fail_on_parse_error: true,
        };
        let event = CdcEventEnvelope {
            lsn: "16/B374DA00".to_string(),
            schema: "cdc".to_string(),
            table: "ddl_events".to_string(),
            tenant_id: "schema-capture".to_string(),
            operation: CdcOperation::Insert,
            columns: vec![
                CdcColumnValue {
                    name: "tenant_id".to_string(),
                    value: Some("schema-capture".to_string()),
                },
                CdcColumnValue {
                    name: "command_tag".to_string(),
                    value: Some("CREATE TABLE".to_string()),
                },
                CdcColumnValue {
                    name: "object_schema".to_string(),
                    value: Some("public".to_string()),
                },
                CdcColumnValue {
                    name: "object_identity".to_string(),
                    value: Some("public.cdc_schema_smoke".to_string()),
                },
                CdcColumnValue {
                    name: "ddl".to_string(),
                    value: Some("CREATE TABLE public.cdc_schema_smoke(id bigint)".to_string()),
                },
                CdcColumnValue {
                    name: "occurred_at".to_string(),
                    value: Some("2026-05-24T18:00:00Z".to_string()),
                },
            ],
        };

        let parsed = plan
            .parse_ddl_stream_event(&event)
            .expect("parse")
            .expect("ddl event");
        assert_eq!(parsed.command_tag, "CREATE TABLE");
        assert_eq!(parsed.object_schema, "public");
        assert_eq!(parsed.object_identity, "public.cdc_schema_smoke");
    }
}
