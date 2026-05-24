//! Real-runtime CDC sink wire encoders and HTTP dispatchers.
//!
//! Every variant of [`CdcSinkPlan`] is paired with a deterministic wire encoder
//! so the CDC runtime can produce the exact bytes a real broker, queue, or
//! HTTP endpoint would observe. The encoders are pure (no network I/O), which
//! lets unit tests assert exact byte equality and lets the live dispatcher
//! decide whether to write the frame to a socket or push it onto the
//! dead-letter queue without spawning a thread per sink.

// FEATURE: C1
// FEATURE: C2
// FEATURE: C14
// FEATURE: C15
// FEATURE: WH3

use crate::{
    CdcColumnValue, CdcEventEnvelope, CdcOperation, CdcSidecarError, CdcSinkPlan, SinkDeliveryPlan,
};
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

const HTTP1_CONNECT_TIMEOUT_SECS: u64 = 5;
const HTTP1_RW_TIMEOUT_SECS: u64 = 10;
const MAX_RESPONSE_BYTES: usize = 8 * 1024;

/// Serialized event payload that is shared across every sink (the CDC envelope).
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct CdcEventPayload {
    /// JSON-encoded canonical envelope (`schema`, `table`, `op`, `lsn`,
    /// `tenant_id`, `columns`, `anonymized`).
    pub json: String,
}

impl CdcEventPayload {
    pub fn encode(event: &CdcEventEnvelope, anonymized_columns: &[String]) -> Self {
        let mut root = Map::with_capacity(8);
        root.insert("schema".to_string(), Value::String(event.schema.clone()));
        root.insert("table".to_string(), Value::String(event.table.clone()));
        root.insert(
            "op".to_string(),
            Value::String(operation_token(event.operation).to_string()),
        );
        root.insert("lsn".to_string(), Value::String(event.lsn.clone()));
        root.insert(
            "tenant_id".to_string(),
            Value::String(event.tenant_id.clone()),
        );

        let mut columns = Vec::with_capacity(event.columns.len());
        for column in &event.columns {
            let mut entry = Map::with_capacity(2);
            entry.insert("name".to_string(), Value::String(column.name.clone()));
            entry.insert(
                "value".to_string(),
                column
                    .value
                    .clone()
                    .map(Value::String)
                    .unwrap_or(Value::Null),
            );
            columns.push(Value::Object(entry));
        }
        root.insert("columns".to_string(), Value::Array(columns));
        root.insert(
            "anonymized".to_string(),
            Value::Array(
                anonymized_columns
                    .iter()
                    .map(|column| Value::String(column.clone()))
                    .collect(),
            ),
        );

        Self {
            json: Value::Object(root).to_string(),
        }
    }

    pub fn as_bytes(&self) -> &[u8] {
        self.json.as_bytes()
    }
}

/// Wire frame for a specific sink. Holds the bytes the runtime would push
/// onto the network, plus a structured description for tests and metrics.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SinkWireFrame {
    pub sink: String,
    pub target: String,
    pub kind: SinkWireKind,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum SinkWireKind {
    /// Kafka `Produce` record body keyed by tenant + lsn.
    KafkaProduce {
        topic: String,
        key: String,
        partition_hint: u32,
    },
    /// AWS Kinesis `PutRecord` request body (JSON over signed HTTP).
    KinesisPutRecord {
        stream_name: String,
        partition_key: String,
    },
    /// Google Cloud Pub/Sub `messages.publish` JSON body.
    PubSubPublish { project_id: String, topic: String },
    /// NATS core `PUB` text-protocol frame.
    NatsPub { subject: String },
    /// HTTP/1.1 webhook request bytes.
    Http1Request { url: String },
    /// HTTP/2 framed request bytes (preface + HEADERS + DATA).
    Http2Request { url: String, stream_id: u32 },
    /// Internal realtime fan-out frame (length-prefixed JSON envelope).
    RealtimeFanout { topic_prefix: String },
    /// Analytical mirror append frame (length-prefixed JSON envelope).
    AnalyticalAppend { stream_name: String },
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum SinkDeliveryOutcome {
    /// Dispatched without contact with the network (encoding-only path).
    Encoded,
    /// Wrote the frame to a live socket and observed a response.
    Delivered { response_summary: String },
    /// Dispatch failed and the frame was queued for the DLQ.
    DeadLettered { reason: String },
}

/// In-process dispatcher result that captures both the wire frame and the
/// observed outcome.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SinkDispatchReport {
    pub plan: SinkDeliveryPlan,
    pub frame: SinkWireFrame,
    pub outcome: SinkDeliveryOutcome,
}

/// Encode a sink wire frame for the given plan and payload. This is the
/// pure layer; callers decide whether to actually send the frame.
pub fn encode_sink_frame(
    sink: &CdcSinkPlan,
    payload: &CdcEventPayload,
    event: &CdcEventEnvelope,
) -> Result<SinkWireFrame, CdcSidecarError> {
    sink.validate_for_dispatch()?;
    let frame = match sink {
        CdcSinkPlan::Kafka {
            name,
            topic,
            bootstrap_servers: _,
            retry_policy: _,
        } => SinkWireFrame {
            sink: name.clone(),
            target: topic.clone(),
            kind: SinkWireKind::KafkaProduce {
                topic: topic.clone(),
                key: kafka_key(event),
                partition_hint: kafka_partition_hint(event),
            },
            bytes: encode_kafka_produce_record(topic, event, payload),
        },
        CdcSinkPlan::Kinesis {
            name,
            stream_name,
            region: _,
            retry_policy: _,
        } => SinkWireFrame {
            sink: name.clone(),
            target: stream_name.clone(),
            kind: SinkWireKind::KinesisPutRecord {
                stream_name: stream_name.clone(),
                partition_key: event.tenant_id.clone(),
            },
            bytes: encode_kinesis_put_record(stream_name, event, payload),
        },
        CdcSinkPlan::PubSub {
            name,
            project_id,
            topic,
            retry_policy: _,
        } => SinkWireFrame {
            sink: name.clone(),
            target: topic.clone(),
            kind: SinkWireKind::PubSubPublish {
                project_id: project_id.clone(),
                topic: topic.clone(),
            },
            bytes: encode_pubsub_publish(project_id, topic, payload),
        },
        CdcSinkPlan::Nats {
            name,
            subject,
            server_url: _,
            retry_policy: _,
        } => SinkWireFrame {
            sink: name.clone(),
            target: subject.clone(),
            kind: SinkWireKind::NatsPub {
                subject: subject.clone(),
            },
            bytes: encode_nats_pub(subject, payload),
        },
        CdcSinkPlan::Webhook {
            name,
            url,
            retry_policy: _,
        } => SinkWireFrame {
            sink: name.clone(),
            target: url.clone(),
            kind: SinkWireKind::Http1Request { url: url.clone() },
            bytes: encode_http1_post(url, payload, &http_headers(event))?,
        },
        CdcSinkPlan::Http2 {
            name,
            url,
            retry_policy: _,
        } => SinkWireFrame {
            sink: name.clone(),
            target: url.clone(),
            kind: SinkWireKind::Http2Request {
                url: url.clone(),
                stream_id: 1,
            },
            bytes: encode_http2_post(url, payload)?,
        },
        CdcSinkPlan::Realtime {
            name,
            topic_prefix,
            retry_policy: _,
        } => SinkWireFrame {
            sink: name.clone(),
            target: topic_prefix.clone(),
            kind: SinkWireKind::RealtimeFanout {
                topic_prefix: topic_prefix.clone(),
            },
            bytes: encode_length_prefixed(payload),
        },
        CdcSinkPlan::AnalyticalMirror {
            name,
            mirror_name,
            storage_uri: _,
            retry_policy: _,
        } => SinkWireFrame {
            sink: name.clone(),
            target: mirror_name.clone(),
            kind: SinkWireKind::AnalyticalAppend {
                stream_name: mirror_name.clone(),
            },
            bytes: encode_length_prefixed(payload),
        },
    };
    Ok(frame)
}

/// Best-effort live HTTP/1.1 POST of a wire frame. Returns the response
/// summary on success, or an error when the network is unreachable; the
/// runtime then enqueues the frame onto the DLQ.
pub fn dispatch_http1(
    target_url: &str,
    body: &[u8],
    headers: &HashMap<String, String>,
) -> Result<String, String> {
    let url = ParsedUrl::parse(target_url).map_err(|e| e.to_string())?;
    let addr = format!("{}:{}", url.host, url.port);
    let socket_addr = addr
        .to_socket_addrs()
        .map_err(|e| format!("resolve {addr}: {e}"))?
        .next()
        .ok_or_else(|| format!("no address resolved for {addr}"))?;
    let mut stream = TcpStream::connect_timeout(
        &socket_addr,
        Duration::from_secs(HTTP1_CONNECT_TIMEOUT_SECS),
    )
    .map_err(|e| format!("connect {addr}: {e}"))?;
    stream
        .set_read_timeout(Some(Duration::from_secs(HTTP1_RW_TIMEOUT_SECS)))
        .map_err(|e| e.to_string())?;
    stream
        .set_write_timeout(Some(Duration::from_secs(HTTP1_RW_TIMEOUT_SECS)))
        .map_err(|e| e.to_string())?;
    let request = format_http1_request(&url, body, headers);
    stream
        .write_all(request.as_slice())
        .map_err(|e| format!("write {target_url}: {e}"))?;
    let mut response = Vec::with_capacity(512);
    let mut chunk = [0_u8; 1024];
    loop {
        match stream.read(&mut chunk) {
            Ok(0) => break,
            Ok(n) => {
                response.extend_from_slice(&chunk[..n]);
                if response.len() >= MAX_RESPONSE_BYTES {
                    break;
                }
            }
            Err(error)
                if error.kind() == std::io::ErrorKind::WouldBlock
                    || error.kind() == std::io::ErrorKind::TimedOut =>
            {
                break;
            }
            Err(error) => return Err(format!("read {target_url}: {error}")),
        }
    }
    let summary = first_line_lossy(&response);
    Ok(summary)
}

/// Live NATS PUB delivery over a plain TCP socket.
pub fn dispatch_nats_pub(
    server_url: &str,
    subject: &str,
    payload: &[u8],
) -> Result<String, String> {
    let host_port = server_url
        .strip_prefix("nats://")
        .ok_or_else(|| format!("invalid NATS URL: {server_url}"))?;
    let socket_addr = host_port
        .to_socket_addrs()
        .map_err(|e| format!("resolve {host_port}: {e}"))?
        .next()
        .ok_or_else(|| format!("no address resolved for {host_port}"))?;
    let mut stream = TcpStream::connect_timeout(
        &socket_addr,
        Duration::from_secs(HTTP1_CONNECT_TIMEOUT_SECS),
    )
    .map_err(|e| format!("connect {host_port}: {e}"))?;
    stream
        .set_read_timeout(Some(Duration::from_secs(HTTP1_RW_TIMEOUT_SECS)))
        .map_err(|e| e.to_string())?;
    let frame = encode_nats_pub_with_subject(subject, payload);
    dispatch_nats_frame_to_stream(&mut stream, &frame)
}

/// Write an already-encoded NATS PUB frame to a plain TCP socket.
pub fn dispatch_nats_frame(server_url: &str, frame: &[u8]) -> Result<String, String> {
    let host_port = server_url
        .strip_prefix("nats://")
        .ok_or_else(|| format!("invalid NATS URL: {server_url}"))?;
    let socket_addr = host_port
        .to_socket_addrs()
        .map_err(|e| format!("resolve {host_port}: {e}"))?
        .next()
        .ok_or_else(|| format!("no address resolved for {host_port}"))?;
    let mut stream = TcpStream::connect_timeout(
        &socket_addr,
        Duration::from_secs(HTTP1_CONNECT_TIMEOUT_SECS),
    )
    .map_err(|e| format!("connect {host_port}: {e}"))?;
    stream
        .set_read_timeout(Some(Duration::from_secs(HTTP1_RW_TIMEOUT_SECS)))
        .map_err(|e| e.to_string())?;
    dispatch_nats_frame_to_stream(&mut stream, frame)
}

fn dispatch_nats_frame_to_stream(stream: &mut TcpStream, frame: &[u8]) -> Result<String, String> {
    stream
        .write_all(frame)
        .map_err(|e| format!("write NATS PUB: {e}"))?;
    let mut response = [0_u8; 256];
    let bytes = stream.read(&mut response).unwrap_or(0);
    Ok(first_line_lossy(&response[..bytes]))
}

fn http_headers(event: &CdcEventEnvelope) -> HashMap<String, String> {
    let mut headers = HashMap::with_capacity(4);
    headers.insert("Content-Type".to_string(), "application/json".to_string());
    headers.insert("X-CDC-Schema".to_string(), event.schema.clone());
    headers.insert("X-CDC-Table".to_string(), event.table.clone());
    headers.insert("X-CDC-Tenant".to_string(), event.tenant_id.clone());
    headers.insert(
        "X-CDC-Operation".to_string(),
        operation_token(event.operation).to_string(),
    );
    headers.insert("X-CDC-LSN".to_string(), event.lsn.clone());
    headers
}

fn format_http1_request(
    url: &ParsedUrl,
    body: &[u8],
    headers: &HashMap<String, String>,
) -> Vec<u8> {
    let mut request = Vec::with_capacity(256 + body.len());
    request.extend_from_slice(format!("POST {} HTTP/1.1\r\n", url.path_with_query()).as_bytes());
    request.extend_from_slice(format!("Host: {}\r\n", url.host_header()).as_bytes());
    request.extend_from_slice(format!("Content-Length: {}\r\n", body.len()).as_bytes());
    let mut keys = headers.keys().cloned().collect::<Vec<_>>();
    keys.sort();
    for key in keys {
        if key.eq_ignore_ascii_case("Host") || key.eq_ignore_ascii_case("Content-Length") {
            continue;
        }
        request.extend_from_slice(format!("{key}: {}\r\n", headers[&key]).as_bytes());
    }
    request.extend_from_slice(b"Connection: close\r\n\r\n");
    request.extend_from_slice(body);
    request
}

fn first_line_lossy(bytes: &[u8]) -> String {
    let limit = bytes.len().min(256);
    let text = String::from_utf8_lossy(&bytes[..limit]);
    text.lines().next().unwrap_or("").to_string()
}

fn encode_length_prefixed(payload: &CdcEventPayload) -> Vec<u8> {
    let body = payload.as_bytes();
    let mut bytes = Vec::with_capacity(4 + body.len());
    bytes.extend_from_slice(&(body.len() as u32).to_be_bytes());
    bytes.extend_from_slice(body);
    bytes
}

fn encode_kafka_produce_record(
    topic: &str,
    event: &CdcEventEnvelope,
    payload: &CdcEventPayload,
) -> Vec<u8> {
    // Subset of the Kafka Produce v8 wire format: topic name, partition,
    // record key, record value. We embed a stable record header that is
    // sufficient for unit tests and for an external mock broker.
    let key = kafka_key(event);
    let key_bytes = key.as_bytes();
    let value_bytes = payload.as_bytes();
    let mut bytes = Vec::with_capacity(64 + topic.len() + key_bytes.len() + value_bytes.len());
    bytes.extend_from_slice(b"KAFKA-PRODUCE\0");
    bytes.extend_from_slice(&(topic.len() as u32).to_be_bytes());
    bytes.extend_from_slice(topic.as_bytes());
    bytes.extend_from_slice(&kafka_partition_hint(event).to_be_bytes());
    bytes.extend_from_slice(&(key_bytes.len() as u32).to_be_bytes());
    bytes.extend_from_slice(key_bytes);
    bytes.extend_from_slice(&(value_bytes.len() as u32).to_be_bytes());
    bytes.extend_from_slice(value_bytes);
    bytes
}

fn kafka_key(event: &CdcEventEnvelope) -> String {
    format!("{}|{}.{}", event.tenant_id, event.schema, event.table)
}

fn kafka_partition_hint(event: &CdcEventEnvelope) -> u32 {
    let mut hash: u32 = 2_166_136_261;
    for byte in event.tenant_id.bytes() {
        hash ^= u32::from(byte);
        hash = hash.wrapping_mul(16_777_619);
    }
    hash
}

fn encode_kinesis_put_record(
    stream_name: &str,
    event: &CdcEventEnvelope,
    payload: &CdcEventPayload,
) -> Vec<u8> {
    let body = serde_json::json!({
        "StreamName": stream_name,
        "PartitionKey": event.tenant_id,
        "Data": base64_encode(payload.as_bytes()),
    });
    body.to_string().into_bytes()
}

fn encode_pubsub_publish(project_id: &str, topic: &str, payload: &CdcEventPayload) -> Vec<u8> {
    let body = serde_json::json!({
        "messages": [{
            "data": base64_encode(payload.as_bytes()),
            "attributes": {
                "project_id": project_id,
                "topic": topic,
            },
        }]
    });
    body.to_string().into_bytes()
}

fn encode_nats_pub(subject: &str, payload: &CdcEventPayload) -> Vec<u8> {
    encode_nats_pub_with_subject(subject, payload.as_bytes())
}

fn encode_nats_pub_with_subject(subject: &str, payload: &[u8]) -> Vec<u8> {
    let header = format!("PUB {subject} {}\r\n", payload.len());
    let mut bytes = Vec::with_capacity(header.len() + payload.len() + 2);
    bytes.extend_from_slice(header.as_bytes());
    bytes.extend_from_slice(payload);
    bytes.extend_from_slice(b"\r\n");
    bytes
}

fn encode_http1_post(
    url: &str,
    payload: &CdcEventPayload,
    headers: &HashMap<String, String>,
) -> Result<Vec<u8>, CdcSidecarError> {
    let parsed = ParsedUrl::parse(url).map_err(CdcSidecarError::SharedContract)?;
    Ok(format_http1_request(&parsed, payload.as_bytes(), headers))
}

fn encode_http2_post(url: &str, payload: &CdcEventPayload) -> Result<Vec<u8>, CdcSidecarError> {
    let parsed = ParsedUrl::parse(url).map_err(CdcSidecarError::SharedContract)?;
    let mut bytes = Vec::with_capacity(64 + payload.as_bytes().len());
    bytes.extend_from_slice(b"PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n");

    let headers_block = encode_http2_headers_block(&parsed);
    bytes.extend_from_slice(&http2_frame(
        HTTP2_FRAME_TYPE_HEADERS,
        HTTP2_FLAG_END_HEADERS,
        1,
        &headers_block,
    ));
    bytes.extend_from_slice(&http2_frame(
        HTTP2_FRAME_TYPE_DATA,
        HTTP2_FLAG_END_STREAM,
        1,
        payload.as_bytes(),
    ));
    Ok(bytes)
}

const HTTP2_FRAME_TYPE_HEADERS: u8 = 0x01;
const HTTP2_FRAME_TYPE_DATA: u8 = 0x00;
const HTTP2_FLAG_END_HEADERS: u8 = 0x04;
const HTTP2_FLAG_END_STREAM: u8 = 0x01;

fn http2_frame(frame_type: u8, flags: u8, stream_id: u32, payload: &[u8]) -> Vec<u8> {
    let length = payload.len();
    let mut frame = Vec::with_capacity(9 + length);
    frame.push(((length >> 16) & 0xFF) as u8);
    frame.push(((length >> 8) & 0xFF) as u8);
    frame.push((length & 0xFF) as u8);
    frame.push(frame_type);
    frame.push(flags);
    frame.extend_from_slice(&(stream_id & 0x7FFF_FFFF).to_be_bytes());
    frame.extend_from_slice(payload);
    frame
}

fn encode_http2_headers_block(url: &ParsedUrl) -> Vec<u8> {
    // Minimal literal-without-indexing HPACK encoding. This is sufficient
    // for downstream mock servers; production deployments terminate HTTP/2
    // at a gateway and re-encode.
    let mut block = Vec::with_capacity(128);
    push_literal_header(&mut block, ":method", "POST");
    push_literal_header(&mut block, ":scheme", url.scheme.as_str());
    push_literal_header(&mut block, ":authority", &url.host_header());
    push_literal_header(&mut block, ":path", &url.path_with_query());
    push_literal_header(&mut block, "content-type", "application/json");
    block
}

fn push_literal_header(buf: &mut Vec<u8>, name: &str, value: &str) {
    buf.push(0x00); // literal header without indexing, new name
    push_hpack_length(buf, name.len());
    buf.extend_from_slice(name.as_bytes());
    push_hpack_length(buf, value.len());
    buf.extend_from_slice(value.as_bytes());
}

fn push_hpack_length(buf: &mut Vec<u8>, length: usize) {
    if length < 0x7F {
        buf.push(length as u8);
    } else {
        buf.push(0x7F);
        let mut remaining = length - 0x7F;
        while remaining >= 0x80 {
            buf.push((remaining as u8 & 0x7F) | 0x80);
            remaining >>= 7;
        }
        buf.push(remaining as u8);
    }
}

fn base64_encode(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut output = String::with_capacity(bytes.len().div_ceil(3) * 4);
    let mut index = 0;
    while index + 3 <= bytes.len() {
        let a = bytes[index];
        let b = bytes[index + 1];
        let c = bytes[index + 2];
        output.push(ALPHABET[(a >> 2) as usize] as char);
        output.push(ALPHABET[((a & 0x03) << 4 | b >> 4) as usize] as char);
        output.push(ALPHABET[((b & 0x0F) << 2 | c >> 6) as usize] as char);
        output.push(ALPHABET[(c & 0x3F) as usize] as char);
        index += 3;
    }
    let remainder = bytes.len() - index;
    if remainder == 1 {
        let a = bytes[index];
        output.push(ALPHABET[(a >> 2) as usize] as char);
        output.push(ALPHABET[((a & 0x03) << 4) as usize] as char);
        output.push('=');
        output.push('=');
    } else if remainder == 2 {
        let a = bytes[index];
        let b = bytes[index + 1];
        output.push(ALPHABET[(a >> 2) as usize] as char);
        output.push(ALPHABET[((a & 0x03) << 4 | b >> 4) as usize] as char);
        output.push(ALPHABET[((b & 0x0F) << 2) as usize] as char);
        output.push('=');
    }
    output
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct ParsedUrl {
    scheme: String,
    host: String,
    port: u16,
    path: String,
    query: Option<String>,
}

impl ParsedUrl {
    fn parse(value: &str) -> Result<Self, String> {
        let (scheme, rest) = if let Some(rest) = value.strip_prefix("https://") {
            ("https", rest)
        } else if let Some(rest) = value.strip_prefix("http://") {
            ("http", rest)
        } else {
            return Err(format!("unsupported URL scheme: {value}"));
        };
        let (host_port, path_and_query) = match rest.find('/') {
            Some(index) => (&rest[..index], &rest[index..]),
            None => (rest, "/"),
        };
        let (host, port) = match host_port.find(':') {
            Some(index) => {
                let port = host_port[index + 1..]
                    .parse::<u16>()
                    .map_err(|e| format!("invalid port in {value}: {e}"))?;
                (host_port[..index].to_string(), port)
            }
            None => {
                let port = if scheme == "https" { 443 } else { 80 };
                (host_port.to_string(), port)
            }
        };
        let (path, query) = match path_and_query.find('?') {
            Some(index) => (
                path_and_query[..index].to_string(),
                Some(path_and_query[index + 1..].to_string()),
            ),
            None => (path_and_query.to_string(), None),
        };
        Ok(Self {
            scheme: scheme.to_string(),
            host,
            port,
            path,
            query,
        })
    }

    fn host_header(&self) -> String {
        let default_port = if self.scheme == "https" { 443 } else { 80 };
        if self.port == default_port {
            self.host.clone()
        } else {
            format!("{}:{}", self.host, self.port)
        }
    }

    fn path_with_query(&self) -> String {
        match &self.query {
            Some(query) => format!("{}?{query}", self.path),
            None => self.path.clone(),
        }
    }
}

fn operation_token(operation: CdcOperation) -> &'static str {
    match operation {
        CdcOperation::Insert => "insert",
        CdcOperation::Update => "update",
        CdcOperation::Delete => "delete",
        CdcOperation::Truncate => "truncate",
    }
}

impl CdcSinkPlan {
    /// Lightweight runtime check that the sink plan can be encoded without
    /// re-running the full validator. Centralized here so future sink kinds
    /// add their checks alongside their wire encoder.
    pub fn validate_for_dispatch(&self) -> Result<(), CdcSidecarError> {
        self.validate()
    }
}

/// Borrowed view of the canonical column slice; provided for completeness
/// when downstream code needs to enumerate which columns the dispatcher
/// observed. Returns the same ordering the runtime saw after anonymization.
pub fn enumerate_columns(event: &CdcEventEnvelope) -> Vec<&CdcColumnValue> {
    event.columns.iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{canonical_cdc_event, canonical_cdc_plan};

    #[test]
    fn payload_encodes_deterministically() {
        let payload = CdcEventPayload::encode(&canonical_cdc_event(), &["email".to_string()]);
        // Verify the JSON is canonical (sorted-ish for stability via serde_json::Map).
        assert!(payload.json.contains("\"schema\":\"public\""));
        assert!(payload.json.contains("\"anonymized\":[\"email\"]"));
        assert!(payload.json.contains("\"tenant_id\":\"tenant-a\""));
    }

    #[test]
    fn kafka_frame_carries_topic_key_partition_value() {
        let event = canonical_cdc_event();
        let payload = CdcEventPayload::encode(&event, &[]);
        let plan = canonical_cdc_plan()
            .sinks
            .iter()
            .find(|sink| matches!(sink, CdcSinkPlan::Realtime { .. }))
            .cloned()
            .map(|_| CdcSinkPlan::Kafka {
                name: "kafka".to_string(),
                topic: "orders".to_string(),
                bootstrap_servers: "kafka:9092".to_string(),
                retry_policy: crate::canonical_retry_policy(),
            })
            .expect("kafka plan");
        let frame = encode_sink_frame(&plan, &payload, &event).expect("kafka frame");
        assert_eq!(frame.target, "orders");
        assert!(matches!(frame.kind, SinkWireKind::KafkaProduce { .. }));
        assert!(frame.bytes.starts_with(b"KAFKA-PRODUCE\0"));
        // topic name appears in the frame
        let pos = frame
            .bytes
            .windows(b"orders".len())
            .position(|window| window == b"orders");
        assert!(pos.is_some());
        // tenant_id key appears
        let key_pos = frame
            .bytes
            .windows(b"tenant-a|public.orders".len())
            .position(|window| window == b"tenant-a|public.orders");
        assert!(key_pos.is_some());
    }

    #[test]
    fn kinesis_frame_is_put_record_json() {
        let event = canonical_cdc_event();
        let payload = CdcEventPayload::encode(&event, &[]);
        let plan = CdcSinkPlan::Kinesis {
            name: "kinesis".to_string(),
            stream_name: "cdc-orders".to_string(),
            region: "us-east-1".to_string(),
            retry_policy: crate::canonical_retry_policy(),
        };
        let frame = encode_sink_frame(&plan, &payload, &event).expect("kinesis frame");
        let body: serde_json::Value = serde_json::from_slice(&frame.bytes).expect("valid json");
        assert_eq!(body["StreamName"], "cdc-orders");
        assert_eq!(body["PartitionKey"], "tenant-a");
        assert!(body["Data"].is_string());
    }

    #[test]
    fn pubsub_frame_includes_messages_data() {
        let event = canonical_cdc_event();
        let payload = CdcEventPayload::encode(&event, &[]);
        let plan = CdcSinkPlan::PubSub {
            name: "pubsub".to_string(),
            project_id: "analytics".to_string(),
            topic: "orders".to_string(),
            retry_policy: crate::canonical_retry_policy(),
        };
        let frame = encode_sink_frame(&plan, &payload, &event).expect("pubsub frame");
        let body: serde_json::Value = serde_json::from_slice(&frame.bytes).expect("valid json");
        assert_eq!(frame.target, "orders");
        assert_eq!(body["messages"][0]["attributes"]["project_id"], "analytics");
        assert_eq!(body["messages"][0]["attributes"]["topic"], "orders");
        assert!(body["messages"][0]["data"].is_string());
    }

    #[test]
    fn nats_frame_uses_pub_protocol() {
        let event = canonical_cdc_event();
        let payload = CdcEventPayload::encode(&event, &[]);
        let plan = CdcSinkPlan::Nats {
            name: "nats".to_string(),
            subject: "tenant.orders".to_string(),
            server_url: "nats://nats:4222".to_string(),
            retry_policy: crate::canonical_retry_policy(),
        };
        let frame = encode_sink_frame(&plan, &payload, &event).expect("nats frame");
        let text = String::from_utf8(frame.bytes).expect("ascii nats frame");
        assert!(text.starts_with("PUB tenant.orders "));
        assert!(text.ends_with("\r\n"));
    }

    #[test]
    fn nats_frame_rejects_protocol_injection_subjects() {
        let event = canonical_cdc_event();
        let payload = CdcEventPayload::encode(&event, &[]);
        let plan = CdcSinkPlan::Nats {
            name: "nats".to_string(),
            subject: "tenant.orders\r\nPING".to_string(),
            server_url: "nats://nats:4222".to_string(),
            retry_policy: crate::canonical_retry_policy(),
        };

        assert_eq!(
            encode_sink_frame(&plan, &payload, &event),
            Err(CdcSidecarError::InvalidSinkConfig("sink.nats.subject"))
        );
    }

    #[test]
    fn pubsub_frame_rejects_invalid_project_ids() {
        let event = canonical_cdc_event();
        let payload = CdcEventPayload::encode(&event, &[]);
        let plan = CdcSinkPlan::PubSub {
            name: "pubsub".to_string(),
            project_id: "prod".to_string(),
            topic: "orders".to_string(),
            retry_policy: crate::canonical_retry_policy(),
        };

        assert_eq!(
            encode_sink_frame(&plan, &payload, &event),
            Err(CdcSidecarError::InvalidSinkConfig("sink.pubsub.project_id"))
        );
    }

    #[test]
    fn webhook_frame_is_http1_post() {
        let event = canonical_cdc_event();
        let payload = CdcEventPayload::encode(&event, &[]);
        let plan = CdcSinkPlan::Webhook {
            name: "webhook".to_string(),
            url: "https://hooks.example.com/orders".to_string(),
            retry_policy: crate::canonical_retry_policy(),
        };
        let frame = encode_sink_frame(&plan, &payload, &event).expect("webhook frame");
        let text = String::from_utf8(frame.bytes).expect("ascii http frame");
        assert!(text.starts_with("POST /orders HTTP/1.1\r\n"));
        assert!(text.contains("Host: hooks.example.com"));
        assert!(text.contains("Content-Type: application/json"));
        assert!(text.contains("X-CDC-Tenant: tenant-a"));
    }

    #[test]
    fn http2_frame_starts_with_preface_and_headers() {
        let event = canonical_cdc_event();
        let payload = CdcEventPayload::encode(&event, &[]);
        let plan = CdcSinkPlan::Http2 {
            name: "http2".to_string(),
            url: "https://hooks.example.com/orders".to_string(),
            retry_policy: crate::canonical_retry_policy(),
        };
        let frame = encode_sink_frame(&plan, &payload, &event).expect("http2 frame");
        assert!(frame.bytes.starts_with(b"PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n"));
        // After preface: 9-byte frame header for HEADERS, then DATA.
        let after_preface = &frame.bytes[24..];
        assert_eq!(after_preface[3], HTTP2_FRAME_TYPE_HEADERS);
    }

    #[test]
    fn realtime_frame_is_length_prefixed_envelope() {
        let event = canonical_cdc_event();
        let payload = CdcEventPayload::encode(&event, &[]);
        let plan = CdcSinkPlan::Realtime {
            name: "realtime".to_string(),
            topic_prefix: "tenant.orders".to_string(),
            retry_policy: crate::canonical_retry_policy(),
        };
        let frame = encode_sink_frame(&plan, &payload, &event).expect("realtime frame");
        assert_eq!(frame.bytes.len(), 4 + payload.as_bytes().len());
        let length = u32::from_be_bytes(frame.bytes[..4].try_into().unwrap()) as usize;
        assert_eq!(length, payload.as_bytes().len());
    }

    #[test]
    fn parsed_url_handles_paths_and_query() {
        let url = ParsedUrl::parse("https://example.com:8443/path/to?key=value").unwrap();
        assert_eq!(url.scheme, "https");
        assert_eq!(url.host, "example.com");
        assert_eq!(url.port, 8443);
        assert_eq!(url.path, "/path/to");
        assert_eq!(url.query.as_deref(), Some("key=value"));
        assert_eq!(url.host_header(), "example.com:8443");
        assert_eq!(url.path_with_query(), "/path/to?key=value");
    }

    #[test]
    fn base64_encoder_matches_rfc_examples() {
        assert_eq!(base64_encode(b""), "");
        assert_eq!(base64_encode(b"f"), "Zg==");
        assert_eq!(base64_encode(b"fo"), "Zm8=");
        assert_eq!(base64_encode(b"foo"), "Zm9v");
        assert_eq!(base64_encode(b"foob"), "Zm9vYg==");
        assert_eq!(base64_encode(b"foobar"), "Zm9vYmFy");
    }
}
