//! Logical replication source boundaries for the CDC runtime.
//!
//! The live sidecar consumes WAL through this trait boundary. The deterministic
//! implementation in this module is intentionally network-free so unit and CI
//! tests can prove checkpoint/ack behavior and wal2json/pgoutput decoding
//! without a local PostgreSQL replication slot. Production deployments wire a
//! `LogicalReplicationClient` implementation backed by PostgreSQL replication
//! protocol frames into the same `ReplicationFrame` + `WalDecoder` surface.

// FEATURE: C1
// FEATURE: C2

use crate::{
    decode_wal2json_frame, CdcColumnValue, CdcEventEnvelope, CdcOperation, CdcSidecarError,
    LogicalReplicationFrame, ReplicationAck, WalOutputPlugin,
};
use serde_json::Value;
use std::collections::VecDeque;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ReplicationStreamConfig {
    pub target_id: String,
    pub conninfo: String,
    pub slot_name: String,
    pub publication_name: String,
    pub plugin: WalOutputPlugin,
    pub start_lsn: String,
}

impl ReplicationStreamConfig {
    pub fn validate(&self) -> Result<(), CdcSidecarError> {
        validate_name("replication.target_id", &self.target_id)?;
        validate_required("replication.conninfo", &self.conninfo)?;
        validate_name("replication.slot_name", &self.slot_name)?;
        validate_name("replication.publication_name", &self.publication_name)?;
        validate_lsn_text(&self.start_lsn)
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ReplicationCheckpoint {
    pub target_id: String,
    pub restart_lsn: String,
    pub confirmed_flush_lsn: String,
    pub apply_lsn: String,
}

impl ReplicationCheckpoint {
    pub fn new(target_id: impl Into<String>, lsn: impl Into<String>) -> Self {
        let lsn = lsn.into();
        Self {
            target_id: target_id.into(),
            restart_lsn: lsn.clone(),
            confirmed_flush_lsn: lsn.clone(),
            apply_lsn: lsn,
        }
    }

    pub fn apply_ack(&mut self, ack: &ReplicationAck) -> Result<(), CdcSidecarError> {
        validate_lsn_text(&ack.write_lsn)?;
        validate_lsn_text(&ack.flush_lsn)?;
        validate_lsn_text(&ack.apply_lsn)?;
        self.confirmed_flush_lsn = ack.flush_lsn.clone();
        self.apply_lsn = ack.apply_lsn.clone();
        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ReplicationFrame {
    pub plugin: WalOutputPlugin,
    pub start_lsn: String,
    pub end_lsn: String,
    pub payload: Vec<u8>,
}

impl ReplicationFrame {
    pub fn validate(&self) -> Result<(), CdcSidecarError> {
        validate_lsn_text(&self.start_lsn)?;
        validate_lsn_text(&self.end_lsn)?;
        if self.payload.is_empty() {
            return Err(CdcSidecarError::MissingRequiredField("replication.payload"));
        }
        Ok(())
    }
}

pub trait LogicalReplicationClient {
    fn config(&self) -> &ReplicationStreamConfig;
    fn checkpoint(&self) -> &ReplicationCheckpoint;
    fn next_frame(&mut self) -> Result<Option<ReplicationFrame>, CdcSidecarError>;
    fn ack(&mut self, ack: ReplicationAck) -> Result<(), CdcSidecarError>;
}

pub trait WalDecoder {
    fn decode(&self, frame: &ReplicationFrame) -> Result<Vec<CdcEventEnvelope>, CdcSidecarError>;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Wal2JsonDecoder;

impl WalDecoder for Wal2JsonDecoder {
    fn decode(&self, frame: &ReplicationFrame) -> Result<Vec<CdcEventEnvelope>, CdcSidecarError> {
        frame.validate()?;
        if frame.plugin != WalOutputPlugin::Wal2Json {
            return Err(CdcSidecarError::UnsupportedOperation(
                "wal2json decoder received non-wal2json frame".to_string(),
            ));
        }
        let payload = String::from_utf8(frame.payload.clone()).map_err(|error| {
            CdcSidecarError::InvalidWal2Json(format!("payload must be utf8: {error}"))
        })?;
        let logical = LogicalReplicationFrame {
            start_lsn: frame.start_lsn.clone(),
            end_lsn: frame.end_lsn.clone(),
            payload,
        };
        decode_wal2json_frame(&logical)
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct PgOutputLogicalDecoder;

impl WalDecoder for PgOutputLogicalDecoder {
    fn decode(&self, frame: &ReplicationFrame) -> Result<Vec<CdcEventEnvelope>, CdcSidecarError> {
        frame.validate()?;
        if frame.plugin != WalOutputPlugin::PgOutput {
            return Err(CdcSidecarError::UnsupportedOperation(
                "pgoutput decoder received non-pgoutput frame".to_string(),
            ));
        }
        let root: Value = serde_json::from_slice(&frame.payload)
            .map_err(|error| CdcSidecarError::InvalidPgOutput(error.to_string()))?;
        let messages = root
            .get("messages")
            .and_then(Value::as_array)
            .ok_or(CdcSidecarError::MissingRequiredField("pgoutput.messages"))?;
        let mut events = Vec::with_capacity(messages.len());
        for message in messages {
            let operation = operation_from_pgoutput(
                message
                    .get("op")
                    .and_then(Value::as_str)
                    .ok_or(CdcSidecarError::MissingRequiredField("pgoutput.op"))?,
            )?;
            let schema = message
                .get("schema")
                .and_then(Value::as_str)
                .ok_or(CdcSidecarError::MissingRequiredField("pgoutput.schema"))?
                .to_string();
            let table = message
                .get("table")
                .and_then(Value::as_str)
                .ok_or(CdcSidecarError::MissingRequiredField("pgoutput.table"))?
                .to_string();
            let columns_json = message
                .get("columns")
                .and_then(Value::as_array)
                .ok_or(CdcSidecarError::MissingRequiredField("pgoutput.columns"))?;
            let mut tenant_id = None;
            let mut columns = Vec::with_capacity(columns_json.len());
            for column in columns_json {
                let name = column
                    .get("name")
                    .and_then(Value::as_str)
                    .ok_or(CdcSidecarError::MissingRequiredField(
                        "pgoutput.column.name",
                    ))?
                    .to_string();
                let value = column
                    .get("value")
                    .map(json_scalar_to_string)
                    .transpose()?
                    .flatten();
                if name == "tenant_id" {
                    tenant_id = value.clone();
                }
                columns.push(CdcColumnValue { name, value });
            }
            let event = CdcEventEnvelope {
                lsn: frame.end_lsn.clone(),
                schema,
                table,
                tenant_id: tenant_id
                    .ok_or(CdcSidecarError::MissingRequiredField("event.tenant_id"))?,
                operation,
                columns,
            };
            event.validate()?;
            events.push(event);
        }
        Ok(events)
    }
}

pub fn decode_replication_frame(
    frame: &ReplicationFrame,
) -> Result<Vec<CdcEventEnvelope>, CdcSidecarError> {
    match frame.plugin {
        WalOutputPlugin::Wal2Json => Wal2JsonDecoder.decode(frame),
        WalOutputPlugin::PgOutput => PgOutputLogicalDecoder.decode(frame),
    }
}

#[derive(Debug, Clone)]
pub struct InMemoryReplicationClient {
    config: ReplicationStreamConfig,
    checkpoint: ReplicationCheckpoint,
    frames: VecDeque<ReplicationFrame>,
    acks: Vec<ReplicationAck>,
}

impl InMemoryReplicationClient {
    pub fn new(
        config: ReplicationStreamConfig,
        frames: impl IntoIterator<Item = ReplicationFrame>,
    ) -> Result<Self, CdcSidecarError> {
        config.validate()?;
        Ok(Self {
            checkpoint: ReplicationCheckpoint::new(
                config.target_id.clone(),
                config.start_lsn.clone(),
            ),
            config,
            frames: frames.into_iter().collect(),
            acks: Vec::new(),
        })
    }

    pub fn acks(&self) -> &[ReplicationAck] {
        &self.acks
    }
}

impl LogicalReplicationClient for InMemoryReplicationClient {
    fn config(&self) -> &ReplicationStreamConfig {
        &self.config
    }

    fn checkpoint(&self) -> &ReplicationCheckpoint {
        &self.checkpoint
    }

    fn next_frame(&mut self) -> Result<Option<ReplicationFrame>, CdcSidecarError> {
        if let Some(frame) = self.frames.pop_front() {
            frame.validate()?;
            Ok(Some(frame))
        } else {
            Ok(None)
        }
    }

    fn ack(&mut self, ack: ReplicationAck) -> Result<(), CdcSidecarError> {
        self.checkpoint.apply_ack(&ack)?;
        self.acks.push(ack);
        Ok(())
    }
}

fn operation_from_pgoutput(operation: &str) -> Result<CdcOperation, CdcSidecarError> {
    match operation {
        "I" | "insert" => Ok(CdcOperation::Insert),
        "U" | "update" => Ok(CdcOperation::Update),
        "D" | "delete" => Ok(CdcOperation::Delete),
        "T" | "truncate" => Ok(CdcOperation::Truncate),
        other => Err(CdcSidecarError::UnsupportedOperation(other.to_string())),
    }
}

fn json_scalar_to_string(value: &Value) -> Result<Option<String>, CdcSidecarError> {
    Ok(match value {
        Value::Null => None,
        Value::Bool(value) => Some(value.to_string()),
        Value::Number(value) => Some(value.to_string()),
        Value::String(value) => Some(value.clone()),
        _ => {
            return Err(CdcSidecarError::InvalidPgOutput(
                "column values must be scalar".to_string(),
            ))
        }
    })
}

fn validate_required(field: &'static str, value: &str) -> Result<(), CdcSidecarError> {
    if value.trim().is_empty() {
        Err(CdcSidecarError::MissingRequiredField(field))
    } else {
        Ok(())
    }
}

fn validate_name(field: &'static str, value: &str) -> Result<(), CdcSidecarError> {
    validate_required(field, value)?;
    if value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-')
    {
        Ok(())
    } else {
        Err(CdcSidecarError::InvalidIdentifier(field))
    }
}

fn validate_lsn_text(value: &str) -> Result<(), CdcSidecarError> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canonical_wal2json_frame;

    #[test]
    fn wal2json_decoder_uses_canonical_frame_surface() {
        let logical = canonical_wal2json_frame();
        let frame = ReplicationFrame {
            plugin: WalOutputPlugin::Wal2Json,
            start_lsn: logical.start_lsn.clone(),
            end_lsn: logical.end_lsn.clone(),
            payload: logical.payload.into_bytes(),
        };
        let events = Wal2JsonDecoder.decode(&frame).expect("decode");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].tenant_id, "tenant-a");
        assert_eq!(events[0].lsn, "16/B374D900");
    }

    #[test]
    fn pgoutput_logical_decoder_maps_relation_tuple_view() {
        let frame = ReplicationFrame {
            plugin: WalOutputPlugin::PgOutput,
            start_lsn: "16/B374D848".to_string(),
            end_lsn: "16/B374D900".to_string(),
            payload: br#"{"messages":[{"op":"I","schema":"public","table":"orders","columns":[{"name":"id","value":1},{"name":"tenant_id","value":"tenant-a"},{"name":"status","value":"paid"}]}]}"#.to_vec(),
        };
        let events = PgOutputLogicalDecoder.decode(&frame).expect("decode");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].operation, CdcOperation::Insert);
        assert_eq!(events[0].table, "orders");
        assert_eq!(events[0].tenant_id, "tenant-a");
    }

    #[test]
    fn in_memory_client_advances_checkpoint_only_on_ack() {
        let config = ReplicationStreamConfig {
            target_id: "coordinator".to_string(),
            conninfo: "postgres://coordinator/citus".to_string(),
            slot_name: "ai_blaise_cdc".to_string(),
            publication_name: "ai_blaise_publication".to_string(),
            plugin: WalOutputPlugin::Wal2Json,
            start_lsn: "16/B374D848".to_string(),
        };
        let logical = canonical_wal2json_frame();
        let frame = ReplicationFrame {
            plugin: WalOutputPlugin::Wal2Json,
            start_lsn: logical.start_lsn,
            end_lsn: logical.end_lsn,
            payload: logical.payload.into_bytes(),
        };
        let mut client = InMemoryReplicationClient::new(config, [frame]).expect("client");
        assert_eq!(client.checkpoint().confirmed_flush_lsn, "16/B374D848");
        assert!(client.next_frame().expect("frame").is_some());
        client
            .ack(ReplicationAck {
                write_lsn: "16/B374D900".to_string(),
                flush_lsn: "16/B374D900".to_string(),
                apply_lsn: "16/B374D900".to_string(),
            })
            .expect("ack");
        assert_eq!(client.acks().len(), 1);
        assert_eq!(client.checkpoint().confirmed_flush_lsn, "16/B374D900");
    }
}
