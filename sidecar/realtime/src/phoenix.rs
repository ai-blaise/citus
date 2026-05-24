//! Phoenix Channels protocol (v2) frame parser + encoder.
//!
//! Phoenix sends messages as JSON arrays:
//! `[join_ref, ref, topic, event, payload]`.
//! The encoder + decoder here are pure functions over `serde_json::Value`,
//! making them trivial to unit-test and reuse from the WS handler.

// FEATURE: RT1
// FEATURE: RT2
// FEATURE: RT3
// FEATURE: RT4
// FEATURE: RT5

use serde_json::{json, Value};

/// One parsed Phoenix Channels frame.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PhoenixFrame {
    pub join_ref: Option<String>,
    pub message_ref: Option<String>,
    pub topic: String,
    pub event: String,
    pub payload: Value,
}

impl PhoenixFrame {
    /// Encode the frame as a Phoenix-v2 JSON array string.
    pub fn encode(&self) -> String {
        json!([
            self.join_ref,
            self.message_ref,
            self.topic,
            self.event,
            self.payload,
        ])
        .to_string()
    }

    /// Parse a Phoenix-v2 JSON array string.
    pub fn decode(text: &str) -> Result<Self, PhoenixDecodeError> {
        let value: Value = serde_json::from_str(text)
            .map_err(|e| PhoenixDecodeError::InvalidJson(e.to_string()))?;
        let array = value
            .as_array()
            .ok_or(PhoenixDecodeError::NotAnArray)?
            .clone();
        if array.len() != 5 {
            return Err(PhoenixDecodeError::WrongArity(array.len()));
        }
        Ok(Self {
            join_ref: optional_string(&array[0]),
            message_ref: optional_string(&array[1]),
            topic: required_string(&array[2], "topic")?,
            event: required_string(&array[3], "event")?,
            payload: array[4].clone(),
        })
    }

    pub fn reply_ok(&self, response: Value) -> Self {
        Self {
            join_ref: self.join_ref.clone(),
            message_ref: self.message_ref.clone(),
            topic: self.topic.clone(),
            event: "phx_reply".to_string(),
            payload: json!({"status": "ok", "response": response}),
        }
    }

    pub fn reply_error(&self, reason: &str) -> Self {
        Self {
            join_ref: self.join_ref.clone(),
            message_ref: self.message_ref.clone(),
            topic: self.topic.clone(),
            event: "phx_reply".to_string(),
            payload: json!({"status": "error", "response": {"reason": reason}}),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum PhoenixDecodeError {
    InvalidJson(String),
    NotAnArray,
    WrongArity(usize),
    MissingField(&'static str),
}

impl std::fmt::Display for PhoenixDecodeError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidJson(error) => write!(formatter, "phoenix frame is not JSON: {error}"),
            Self::NotAnArray => write!(formatter, "phoenix frame must be a JSON array"),
            Self::WrongArity(len) => {
                write!(formatter, "phoenix frame must have 5 elements, got {len}")
            }
            Self::MissingField(field) => write!(formatter, "phoenix frame missing {field}"),
        }
    }
}

impl std::error::Error for PhoenixDecodeError {}

fn optional_string(value: &Value) -> Option<String> {
    if value.is_null() {
        None
    } else {
        value.as_str().map(ToString::to_string)
    }
}

fn required_string(value: &Value, field: &'static str) -> Result<String, PhoenixDecodeError> {
    value
        .as_str()
        .map(ToString::to_string)
        .ok_or(PhoenixDecodeError::MissingField(field))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_then_decode_roundtrips() {
        let frame = PhoenixFrame {
            join_ref: Some("1".to_string()),
            message_ref: Some("2".to_string()),
            topic: "realtime:public:orders".to_string(),
            event: "phx_join".to_string(),
            payload: json!({"config": {"postgres_changes": [{"event": "INSERT", "schema": "public", "table": "orders"}]}}),
        };
        let encoded = frame.encode();
        let parsed = PhoenixFrame::decode(&encoded).expect("decode");
        assert_eq!(parsed, frame);
    }

    #[test]
    fn decode_rejects_wrong_arity() {
        let err = PhoenixFrame::decode("[1, 2, 3]").unwrap_err();
        assert_eq!(err, PhoenixDecodeError::WrongArity(3));
    }

    #[test]
    fn decode_handles_null_refs() {
        let frame = PhoenixFrame::decode(r#"[null, null, "realtime:public", "heartbeat", {}]"#)
            .expect("decode heartbeat");
        assert!(frame.join_ref.is_none());
        assert!(frame.message_ref.is_none());
        assert_eq!(frame.topic, "realtime:public");
        assert_eq!(frame.event, "heartbeat");
    }

    #[test]
    fn reply_ok_uses_phx_reply_event() {
        let frame = PhoenixFrame {
            join_ref: Some("1".to_string()),
            message_ref: Some("2".to_string()),
            topic: "t".to_string(),
            event: "phx_join".to_string(),
            payload: Value::Null,
        };
        let reply = frame.reply_ok(json!({"ack": true}));
        assert_eq!(reply.event, "phx_reply");
        assert_eq!(reply.payload["status"], "ok");
        assert_eq!(reply.payload["response"]["ack"], true);
    }
}
