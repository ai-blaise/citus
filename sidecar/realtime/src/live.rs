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

fn handle_phx_join(
    conn: &mut WsConnection,
    hub: &RealtimeHub,
    frame: &PhoenixFrame,
    joined: &mut HashMap<String, JoinedChannel>,
) -> Result<(), WsError> {
    let topic = frame.topic.clone();
    let payload = &frame.payload;
    let tenant_id = payload
        .get("tenant_id")
        .and_then(Value::as_str)
        .unwrap_or("tenant-a")
        .to_string();
    let user_id = payload
        .get("user_id")
        .and_then(Value::as_str)
        .unwrap_or("anonymous")
        .to_string();
    let schema = payload
        .get("schema")
        .and_then(Value::as_str)
        .unwrap_or("public")
        .to_string();
    let table = payload
        .get("table")
        .and_then(Value::as_str)
        .unwrap_or("orders")
        .to_string();
    let op = payload
        .get("operation")
        .and_then(Value::as_str)
        .and_then(|raw| match raw.to_ascii_uppercase().as_str() {
            "INSERT" => Some(CdcOperation::Insert),
            "UPDATE" => Some(CdcOperation::Update),
            "DELETE" => Some(CdcOperation::Delete),
            "TRUNCATE" => Some(CdcOperation::Truncate),
            _ => None,
        });
    let mut equals = HashMap::new();
    if let Some(filters) = payload.get("filters").and_then(Value::as_object) {
        for (key, value) in filters {
            if let Some(text) = value.as_str() {
                equals.insert(key.clone(), text.to_string());
            }
        }
    }
    let filter = SubscriptionFilter {
        schema,
        table,
        op,
        equals,
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
    if payload
        .get("presence")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        let diff = hub.presence_join(
            &topic,
            user_id.clone(),
            connection_id.clone(),
            json!({}),
            payload
                .get("online_at")
                .and_then(Value::as_str)
                .unwrap_or("2026-01-01T00:00:00Z")
                .to_string(),
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
        if length > 1 << 20 {
            // Reject frames larger than 1 MiB; that protects against
            // malicious senders. Real production deployments swap this
            // for an enforced rate limiter at the operator level.
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
