//! Live realtime runtime: WS listener -> phoenix channels -> hub -> mailboxes,
//! with a side TCP/Unix-domain-socket listener accepting CDC ingest frames from sidecar/cdc.
//!
//! The runtime is split into a [`RealtimeLiveRuntime`] type (the
//! state-holder) and a set of free functions that drive the TCP loops.
//! Both ends - WS and CDC ingest - can be exercised in isolation by tests
//! that bind ephemeral sockets.

// FEATURE: RT1
// FEATURE: RT2
// FEATURE: RT3
// FEATURE: RT4
// FEATURE: RT5

use crate::hub::{RealtimeHub, SubscriptionFilter};
use crate::phoenix::PhoenixFrame;
use crate::ws::{UpgradeRequest, WsConnection, WsError};
use ai_blaise_citus_sidecar_cdc::{CdcColumnValue, CdcEventEnvelope, CdcOperation};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

pub const MAX_CDC_INGEST_FRAME_BYTES: usize = 1 << 20;

/// Configuration for the live runtime.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct RealtimeLiveConfig {
    pub ws_listen_addr: String,
    /// TCP address (`host:port`) or Unix-domain-socket address (`unix:/path.sock`).
    pub cdc_ingest_addr: String,
    pub realm_apikey: Option<String>,
}

impl Default for RealtimeLiveConfig {
    fn default() -> Self {
        Self {
            ws_listen_addr: "127.0.0.1:4000".to_string(),
            cdc_ingest_addr: "127.0.0.1:4001".to_string(),
            realm_apikey: None,
        }
    }
}

/// Top-level realtime live runtime. Owns the shared hub.
#[derive(Debug)]
pub struct RealtimeLiveRuntime {
    pub hub: Arc<RealtimeHub>,
    pub config: RealtimeLiveConfig,
}

impl RealtimeLiveRuntime {
    pub fn new(config: RealtimeLiveConfig) -> Self {
        Self {
            hub: Arc::new(RealtimeHub::new()),
            config,
        }
    }

    /// Spawn the WS listener thread. The thread loops forever; tests start
    /// the runtime, send a single WS connection, then drop the listener
    /// by closing the parent thread.
    pub fn spawn_ws_listener(&self) -> std::io::Result<thread::JoinHandle<()>> {
        let listener = TcpListener::bind(&self.config.ws_listen_addr)?;
        let hub = self.hub.clone();
        let apikey = self.config.realm_apikey.clone();
        Ok(thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(stream) = stream else {
                    continue;
                };
                let _ = stream.set_read_timeout(Some(Duration::from_secs(60)));
                let _ = stream.set_write_timeout(Some(Duration::from_secs(60)));
                let hub = hub.clone();
                let apikey = apikey.clone();
                thread::spawn(move || {
                    let _ = handle_ws_connection(stream, hub, apikey);
                });
            }
        }))
    }

    pub fn spawn_cdc_ingest_listener(&self) -> std::io::Result<thread::JoinHandle<()>> {
        if let Some(path) = unix_addr_path(&self.config.cdc_ingest_addr) {
            return self.spawn_cdc_ingest_unix_listener(path.to_string());
        }
        let listener = TcpListener::bind(&self.config.cdc_ingest_addr)?;
        let hub = self.hub.clone();
        Ok(thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(stream) = stream else {
                    continue;
                };
                let hub = hub.clone();
                thread::spawn(move || {
                    let _ = handle_cdc_ingest(stream, hub);
                });
            }
        }))
    }

    #[cfg(unix)]
    fn spawn_cdc_ingest_unix_listener(
        &self,
        path: String,
    ) -> std::io::Result<thread::JoinHandle<()>> {
        let _ = std::fs::remove_file(&path);
        let listener = std::os::unix::net::UnixListener::bind(&path)?;
        let hub = self.hub.clone();
        Ok(thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(stream) = stream else {
                    continue;
                };
                let hub = hub.clone();
                thread::spawn(move || {
                    let _ = stream.set_read_timeout(Some(Duration::from_secs(60)));
                    let _ = handle_cdc_ingest_stream(stream, hub);
                });
            }
            let _ = std::fs::remove_file(&path);
        }))
    }

    #[cfg(not(unix))]
    fn spawn_cdc_ingest_unix_listener(
        &self,
        _path: String,
    ) -> std::io::Result<thread::JoinHandle<()>> {
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "Unix-domain sockets are unsupported on this platform",
        ))
    }
}

/// Single-connection WS loop. Handles handshake, phx_join, postgres_changes
/// dispatch (via the hub mailbox), heartbeats, and phx_leave.
pub fn handle_ws_connection(
    mut stream: TcpStream,
    hub: Arc<RealtimeHub>,
    apikey: Option<String>,
) -> Result<(), WsError> {
    // Read the HTTP upgrade request.
    let mut handshake_buffer = Vec::with_capacity(2048);
    let mut chunk = [0_u8; 1024];
    loop {
        let n = stream.read(&mut chunk).map_err(|_| WsError::Closed)?;
        if n == 0 {
            return Err(WsError::Closed);
        }
        handshake_buffer.extend_from_slice(&chunk[..n]);
        if handshake_buffer.windows(4).any(|w| w == b"\r\n\r\n") {
            break;
        }
        if handshake_buffer.len() > 16 * 1024 {
            return Err(WsError::InvalidHandshake);
        }
    }
    let request = UpgradeRequest::parse(&handshake_buffer)?;
    if request.path == "/healthz" {
        write_http_probe(
            &mut stream,
            200,
            "application/json",
            "{\"status\":\"ok\",\"component\":\"realtime\"}",
        )
        .map_err(|_| WsError::Closed)?;
        return Ok(());
    }
    if request.path == "/readyz" {
        write_http_probe(&mut stream, 200, "application/json", "{\"ready\":true}")
            .map_err(|_| WsError::Closed)?;
        return Ok(());
    }
    if request.path == "/metrics" {
        let metrics = hub.metrics();
        let body = format!(
            "# HELP ai_blaise_realtime_broadcasts Total CDC broadcasts.\n\
# TYPE ai_blaise_realtime_broadcasts counter\n\
ai_blaise_realtime_broadcasts {}\n\
# HELP ai_blaise_realtime_delivered Total delivered realtime messages.\n\
# TYPE ai_blaise_realtime_delivered counter\n\
ai_blaise_realtime_delivered {}\n\
# HELP ai_blaise_realtime_filtered Total filtered subscriptions.\n\
# TYPE ai_blaise_realtime_filtered counter\n\
ai_blaise_realtime_filtered {}\n",
            metrics.broadcasts, metrics.delivered, metrics.filtered,
        );
        write_http_probe(&mut stream, 200, "text/plain; version=0.0.4", &body)
            .map_err(|_| WsError::Closed)?;
        return Ok(());
    }
    if let Some(required) = &apikey {
        if request.query.get("apikey") != Some(required) {
            let response = b"HTTP/1.1 401 Unauthorized\r\nContent-Length: 0\r\n\r\n";
            let _ = stream.write_all(response);
            return Err(WsError::InvalidHandshake);
        }
    }
    if request.path != "/realtime/v1/websocket" {
        let response = b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n";
        let _ = stream.write_all(response);
        return Err(WsError::InvalidHandshake);
    }
    let response = request.handshake_response()?;
    stream.write_all(&response).map_err(|_| WsError::Closed)?;

    // Move to WS frames. Short read timeout so the loop can flush
    // outbound mailboxes between blocking pulls.
    let _ = stream.set_read_timeout(Some(Duration::from_millis(100)));
    let mut conn = WsConnection::new(stream);
    let mut joined: HashMap<String, JoinedChannel> = HashMap::new();

    loop {
        // Drain mailboxes for already-joined channels.
        for channel in joined.values() {
            for frame in channel.mailbox.drain() {
                if conn.write_text(&frame).is_err() {
                    return Ok(());
                }
            }
        }

        // Read next frame.
        let n = conn.pull().map_err(|_| WsError::Closed)?;
        if n == 0 {
            // Avoid busy looping when there is no traffic.
            thread::sleep(Duration::from_millis(50));
            continue;
        }
        while let Some(text) = conn.next_text_frame()? {
            let Ok(frame) = PhoenixFrame::decode(&text) else {
                continue;
            };
            match frame.event.as_str() {
                "phx_join" => handle_phx_join(&mut conn, &hub, &frame, &mut joined)?,
                "phx_leave" => handle_phx_leave(&mut conn, &hub, &frame, &mut joined)?,
                "heartbeat" => {
                    let reply = frame.reply_ok(json!({}));
                    if conn.write_text(&reply.encode()).is_err() {
                        return Ok(());
                    }
                }
                _ => {}
            }
        }
    }
}

#[derive(Debug)]
struct JoinedChannel {
    connection_id: String,
    #[allow(dead_code)]
    user_id: String,
    #[allow(dead_code)]
    tenant_id: String,
    subscription_id: u64,
    mailbox: Arc<crate::hub::Mailbox>,
}

#[derive(Debug)]
struct JoinRequest {
    tenant_id: String,
    user_id: String,
    schema: String,
    table: String,
    op: Option<CdcOperation>,
    equals: HashMap<String, String>,
    online_at: Option<String>,
}

impl JoinRequest {
    fn parse(frame: &PhoenixFrame) -> Result<Self, &'static str> {
        let payload = frame
            .payload
            .as_object()
            .ok_or("join payload must be an object")?;
        let tenant_id = required_token(payload, "tenant_id")?.to_string();
        let user_id = required_token(payload, "user_id")?.to_string();
        let schema = required_identifier(payload, "schema")?.to_string();
        let table = required_identifier(payload, "table")?.to_string();
        let expected_topic = format!("realtime:{schema}:{table}");
        if frame.topic != expected_topic {
            return Err("topic must match realtime:<schema>:<table>");
        }
        let op = match payload.get("operation") {
            Some(Value::String(raw)) => Some(parse_operation(raw)?),
            Some(_) => return Err("operation must be a string"),
            None => None,
        };
        let mut equals = HashMap::new();
        if let Some(filters) = payload.get("filters") {
            let filters = filters.as_object().ok_or("filters must be an object")?;
            for (key, value) in filters {
                if !is_identifier(key) {
                    return Err("filter column must be an identifier");
                }
                let Some(text) = value.as_str() else {
                    return Err("filter value must be a string");
                };
                if text.len() > 512 || text.chars().any(char::is_control) {
                    return Err("filter value is invalid");
                }
                equals.insert(key.clone(), text.to_string());
            }
        }
        let presence = payload
            .get("presence")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let online_at = if presence {
            let online_at = payload
                .get("online_at")
                .and_then(Value::as_str)
                .ok_or("online_at is required when presence is enabled")?;
            if !is_utc_timestamp(online_at) {
                return Err("online_at must be an RFC3339 UTC timestamp");
            }
            Some(online_at.to_string())
        } else {
            None
        };
        Ok(Self {
            tenant_id,
            user_id,
            schema,
            table,
            op,
            equals,
            online_at,
        })
    }
}

fn required_token<'a>(
    payload: &'a serde_json::Map<String, Value>,
    field: &'static str,
) -> Result<&'a str, &'static str> {
    let value = payload
        .get(field)
        .and_then(Value::as_str)
        .ok_or("required join field missing")?;
    if value.is_empty()
        || value.len() > 128
        || !value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
    {
        return Err("join token field is invalid");
    }
    Ok(value)
}

fn required_identifier<'a>(
    payload: &'a serde_json::Map<String, Value>,
    field: &'static str,
) -> Result<&'a str, &'static str> {
    let value = payload
        .get(field)
        .and_then(Value::as_str)
        .ok_or("required join field missing")?;
    if !is_identifier(value) {
        return Err("join identifier field is invalid");
    }
    Ok(value)
}

fn is_identifier(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first.is_ascii_alphabetic() || first == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn is_utc_timestamp(value: &str) -> bool {
    value.len() >= 20 && value.contains('T') && value.ends_with('Z')
}

fn parse_operation(raw: &str) -> Result<CdcOperation, &'static str> {
    match raw.to_ascii_uppercase().as_str() {
        "INSERT" => Ok(CdcOperation::Insert),
        "UPDATE" => Ok(CdcOperation::Update),
        "DELETE" => Ok(CdcOperation::Delete),
        "TRUNCATE" => Ok(CdcOperation::Truncate),
        _ => Err("operation is unsupported"),
    }
}

fn handle_phx_join(
    conn: &mut WsConnection,
    hub: &RealtimeHub,
    frame: &PhoenixFrame,
    joined: &mut HashMap<String, JoinedChannel>,
) -> Result<(), WsError> {
    let request = match JoinRequest::parse(frame) {
        Ok(request) => request,
        Err(reason) => {
            let reply = frame.reply_error(reason);
            if conn.write_text(&reply.encode()).is_err() {
                return Err(WsError::Closed);
            }
            return Ok(());
        }
    };
    let topic = frame.topic.clone();
    let tenant_id = request.tenant_id;
    let user_id = request.user_id;
    let filter = SubscriptionFilter {
        schema: request.schema,
        table: request.table,
        op: request.op,
        equals: request.equals,
    };
    let connection_id = format!("{}-{}", tenant_id, user_id);
    let (subscription, mailbox) = hub.subscribe(
        connection_id.clone(),
        user_id.clone(),
        tenant_id.clone(),
        topic.clone(),
        filter,
    );
    joined.insert(
        topic.clone(),
        JoinedChannel {
            connection_id: connection_id.clone(),
            user_id: user_id.clone(),
            tenant_id: tenant_id.clone(),
            subscription_id: subscription.id,
            mailbox: mailbox.clone(),
        },
    );
    let reply = frame.reply_ok(json!({"subscription_id": subscription.id}));
    if conn.write_text(&reply.encode()).is_err() {
        return Err(WsError::Closed);
    }
    if let Some(online_at) = request.online_at {
        let diff = hub.presence_join(
            &topic,
            user_id.clone(),
            connection_id.clone(),
            json!({}),
            online_at,
        );
        let presence_frame = PhoenixFrame {
            join_ref: frame.join_ref.clone(),
            message_ref: None,
            topic,
            event: "presence_diff".to_string(),
            payload: diff,
        };
        if conn.write_text(&presence_frame.encode()).is_err() {
            return Err(WsError::Closed);
        }
    }
    Ok(())
}

fn handle_phx_leave(
    conn: &mut WsConnection,
    hub: &RealtimeHub,
    frame: &PhoenixFrame,
    joined: &mut HashMap<String, JoinedChannel>,
) -> Result<(), WsError> {
    if let Some(channel) = joined.remove(&frame.topic) {
        hub.unsubscribe(channel.subscription_id);
        let _ = hub.presence_leave(&frame.topic, &channel.connection_id);
    }
    let reply = frame.reply_ok(json!({}));
    if conn.write_text(&reply.encode()).is_err() {
        return Err(WsError::Closed);
    }
    Ok(())
}

/// CDC ingest loop. Reads length-prefixed JSON frames from the CDC
/// sidecar and broadcasts each carried event through the hub.
pub fn handle_cdc_ingest(stream: TcpStream, hub: Arc<RealtimeHub>) -> std::io::Result<()> {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(60)));
    handle_cdc_ingest_stream(stream, hub)
}

pub fn handle_cdc_ingest_stream<R: Read>(
    mut stream: R,
    hub: Arc<RealtimeHub>,
) -> std::io::Result<()> {
    let mut header = [0_u8; 4];
    loop {
        if let Err(_) = stream.read_exact(&mut header) {
            return Ok(());
        }
        let length = u32::from_be_bytes(header) as usize;
        if length > MAX_CDC_INGEST_FRAME_BYTES {
            return Ok(());
        }
        let mut body = vec![0_u8; length];
        stream.read_exact(&mut body)?;
        let envelope: Value = match serde_json::from_slice(&body) {
            Ok(value) => value,
            Err(_) => continue,
        };
        let Some(event) = parse_cdc_envelope(&envelope) else {
            continue;
        };
        hub.broadcast(&event);
    }
}

fn unix_addr_path(addr: &str) -> Option<&str> {
    addr.strip_prefix("unix://")
        .or_else(|| addr.strip_prefix("unix:"))
        .filter(|path| !path.is_empty())
}

fn write_http_probe(
    stream: &mut TcpStream,
    status: u16,
    content_type: &str,
    body: &str,
) -> std::io::Result<()> {
    let reason = match status {
        200 => "OK",
        401 => "Unauthorized",
        404 => "Not Found",
        _ => "Error",
    };
    let response = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(), body,
    );
    stream.write_all(response.as_bytes())
}

fn parse_cdc_envelope(value: &Value) -> Option<CdcEventEnvelope> {
    if value.get("type")? != "cdc_event" {
        return None;
    }
    let tenant_id = value.get("tenant_id")?.as_str()?.to_string();
    let schema = value.get("schema")?.as_str()?.to_string();
    let table = value.get("table")?.as_str()?.to_string();
    let lsn = value.get("lsn")?.as_str()?.to_string();
    let operation = match value.get("operation")?.as_str()? {
        "insert" => CdcOperation::Insert,
        "update" => CdcOperation::Update,
        "delete" => CdcOperation::Delete,
        "truncate" => CdcOperation::Truncate,
        _ => return None,
    };
    let payload_str = value.get("payload")?.as_str()?;
    let payload: Value = serde_json::from_str(payload_str).ok()?;
    let mut columns = Vec::new();
    if let Some(array) = payload.get("columns").and_then(Value::as_array) {
        for entry in array {
            let Some(name) = entry.get("name").and_then(Value::as_str) else {
                continue;
            };
            let value = entry.get("value").and_then(|v| {
                if v.is_null() {
                    None
                } else {
                    v.as_str().map(ToString::to_string)
                }
            });
            columns.push(CdcColumnValue {
                name: name.to_string(),
                value,
            });
        }
    }
    Some(CdcEventEnvelope {
        lsn,
        schema,
        table,
        tenant_id,
        operation,
        columns,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use ai_blaise_citus_sidecar_cdc::{canonical_cdc_event, CdcEventPayload};

    #[test]
    fn parse_cdc_envelope_decodes_round_trip_payload() {
        let event = canonical_cdc_event();
        let payload = CdcEventPayload::encode(&event, &[]);
        let envelope = json!({
            "type": "cdc_event",
            "tenant_id": event.tenant_id,
            "schema": event.schema,
            "table": event.table,
            "lsn": event.lsn,
            "operation": "insert",
            "payload": payload.json,
        });
        let parsed = parse_cdc_envelope(&envelope).expect("parse");
        assert_eq!(parsed.tenant_id, "tenant-a");
        assert_eq!(parsed.schema, "public");
        assert_eq!(parsed.table, "orders");
        assert_eq!(parsed.columns.len(), event.columns.len());
    }

    #[test]
    fn join_request_rejects_missing_tenant_and_does_not_subscribe() {
        let frame = PhoenixFrame {
            join_ref: Some("1".to_string()),
            message_ref: Some("1".to_string()),
            topic: "realtime:public:orders".to_string(),
            event: "phx_join".to_string(),
            payload: json!({
                "user_id": "user-a",
                "schema": "public",
                "table": "orders",
                "operation": "INSERT",
            }),
        };
        assert!(JoinRequest::parse(&frame).is_err());
    }

    #[test]
    fn join_request_rejects_invalid_filter_column() {
        let frame = PhoenixFrame {
            join_ref: Some("1".to_string()),
            message_ref: Some("1".to_string()),
            topic: "realtime:public:orders".to_string(),
            event: "phx_join".to_string(),
            payload: json!({
                "tenant_id": "tenant-a",
                "user_id": "user-a",
                "schema": "public",
                "table": "orders",
                "operation": "INSERT",
                "filters": {"status;drop": "paid"},
            }),
        };
        assert!(JoinRequest::parse(&frame).is_err());
    }

    #[test]
    fn handle_cdc_ingest_rejects_oversized_frame_before_broadcast() {
        let hub = Arc::new(RealtimeHub::new());
        let mut frame = Vec::new();
        frame.extend_from_slice(&((MAX_CDC_INGEST_FRAME_BYTES as u32) + 1).to_be_bytes());
        let _ = handle_cdc_ingest_stream(frame.as_slice(), hub.clone());
        assert_eq!(hub.metrics().broadcasts, 0);
    }

    #[test]
    fn handle_cdc_ingest_broadcasts_to_subscriber() {
        let hub = Arc::new(RealtimeHub::new());
        let (_sub, mailbox) = hub.subscribe(
            "conn".to_string(),
            "user".to_string(),
            "tenant-a".to_string(),
            "realtime:public:orders".to_string(),
            SubscriptionFilter {
                schema: "public".to_string(),
                table: "orders".to_string(),
                op: None,
                equals: HashMap::new(),
            },
        );
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let addr = listener.local_addr().expect("addr");
        let hub_for_thread = hub.clone();
        let handle = thread::spawn(move || {
            let (stream, _) = listener.accept().expect("accept");
            let _ = handle_cdc_ingest(stream, hub_for_thread);
        });

        let event = canonical_cdc_event();
        let payload = CdcEventPayload::encode(&event, &[]);
        let envelope = json!({
            "type": "cdc_event",
            "tenant_id": event.tenant_id,
            "schema": event.schema,
            "table": event.table,
            "lsn": event.lsn,
            "operation": "insert",
            "payload": payload.json,
        })
        .to_string();
        let mut stream = TcpStream::connect(addr).expect("connect");
        let header = (envelope.len() as u32).to_be_bytes();
        stream.write_all(&header).expect("header");
        stream.write_all(envelope.as_bytes()).expect("body");
        drop(stream);

        // Wait for the worker to process the frame.
        let _ = handle.join();
        assert!(!mailbox.is_empty());
    }
}
