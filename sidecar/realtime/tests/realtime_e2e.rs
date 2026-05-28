//! End-to-end test for the realtime sidecar: spawn a WS listener, connect
//! a raw client, perform the phoenix-channel join, then push a CDC event
//! through the ingest TCP listener and assert the WS client receives the
//! corresponding `postgres_changes` frame.
//!
//! The test uses only `std` — the WS handshake, framing, and base64 are
//! all written in-tree in `sidecar/realtime/src/ws.rs`. No external WS
//! library is involved.

use ai_blaise_citus_sidecar_cdc::{canonical_cdc_event, CdcEventPayload};
use ai_blaise_citus_sidecar_realtime::{
    decode_frame, encode_text_frame, PhoenixFrame, RealtimeLiveConfig, RealtimeLiveRuntime,
};
use serde_json::json;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::thread;
use std::time::Duration;

#[test]
fn realtime_e2e_join_and_broadcast() {
    let config = RealtimeLiveConfig {
        ws_listen_addr: ephemeral_addr(),
        cdc_ingest_addr: ephemeral_addr(),
        ..Default::default()
    };
    let runtime = RealtimeLiveRuntime::new(config.clone());
    let _ws_thread = runtime.spawn_ws_listener().expect("ws bind");
    let _cdc_thread = runtime
        .spawn_cdc_ingest_listener()
        .expect("cdc ingest bind");

    // Connect WS client.
    let mut client = TcpStream::connect(&config.ws_listen_addr).expect("connect ws");
    client
        .set_read_timeout(Some(Duration::from_secs(10)))
        .expect("rt");
    client
        .set_write_timeout(Some(Duration::from_secs(10)))
        .expect("wt");

    // Send WS upgrade request.
    let request = "GET /realtime/v1/websocket?vsn=2.0.0 HTTP/1.1\r\n\
         Host: localhost\r\n\
         Upgrade: websocket\r\n\
         Connection: Upgrade\r\n\
         Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n\
         Sec-WebSocket-Version: 13\r\n\r\n"
        .to_string();
    client.write_all(request.as_bytes()).expect("write upgrade");

    // Read handshake response.
    let mut response = Vec::with_capacity(512);
    let mut chunk = [0_u8; 256];
    loop {
        let n = client.read(&mut chunk).expect("read handshake");
        response.extend_from_slice(&chunk[..n]);
        if response.windows(4).any(|w| w == b"\r\n\r\n") {
            break;
        }
        if response.len() > 4096 {
            panic!("handshake reply too large");
        }
    }
    let response_text = String::from_utf8_lossy(&response);
    assert!(
        response_text.starts_with("HTTP/1.1 101 "),
        "{response_text}"
    );
    assert!(response_text.contains("Sec-WebSocket-Accept: s3pPLMBiTxaQ9kYGzzhZRbK+xOo="));

    // Drain any leftover bytes after the handshake header (server may
    // append frames if it pushes before phx_join, but in our impl it
    // does not).
    let header_end = response
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .expect("header end")
        + 4;
    let _leftover = response.split_off(header_end);

    // Send phx_join (masked client frame).
    let topic = "realtime:public:orders".to_string();
    let join = PhoenixFrame {
        join_ref: Some("1".to_string()),
        message_ref: Some("1".to_string()),
        topic: topic.clone(),
        event: "phx_join".to_string(),
        payload: json!({
            "tenant_id": "tenant-a",
            "user_id": "user-a",
            "schema": "public",
            "table": "orders",
            "operation": "INSERT",
            "presence": true,
            "online_at": "2026-05-19T12:00:00Z",
        }),
    };
    write_masked(&mut client, &join.encode());

    // Server should reply with phx_reply{ok} + presence_diff.
    let mut received = Vec::new();
    let frames = drain_frames(&mut client, &mut received, 2);
    assert!(
        frames
            .iter()
            .any(|f| f.event == "phx_reply" && f.payload["status"] == "ok"),
        "no phx_reply: {frames:?}"
    );
    assert!(
        frames.iter().any(|f| f.event == "presence_diff"),
        "no presence_diff: {frames:?}"
    );

    // Send CDC event via the ingest TCP listener.
    let mut cdc_stream = TcpStream::connect(&config.cdc_ingest_addr).expect("ingest connect");
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
    let header = (envelope.len() as u32).to_be_bytes();
    cdc_stream.write_all(&header).expect("header");
    cdc_stream.write_all(envelope.as_bytes()).expect("body");
    drop(cdc_stream);

    // Expect a postgres_changes frame.
    let mut postgres_changes = None;
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    while std::time::Instant::now() < deadline {
        let new_frames = drain_frames(&mut client, &mut received, 1);
        for frame in new_frames {
            if frame.event == "postgres_changes" {
                postgres_changes = Some(frame);
                break;
            }
        }
        if postgres_changes.is_some() {
            break;
        }
        thread::sleep(Duration::from_millis(50));
    }
    let postgres_changes = postgres_changes.expect("expected postgres_changes frame");
    assert_eq!(postgres_changes.topic, topic);
    assert_eq!(postgres_changes.payload["schema"], "public");
    assert_eq!(postgres_changes.payload["table"], "orders");
    assert_eq!(postgres_changes.payload["type"], "INSERT");
}

fn ephemeral_addr() -> String {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
    let addr = listener.local_addr().expect("addr");
    drop(listener);
    addr.to_string()
}

fn write_masked(stream: &mut TcpStream, text: &str) {
    // FIN+text opcode, with mask bit set; uses an all-zero mask so the
    // unmasked payload equals the text bytes.
    let bytes = text.as_bytes();
    let mut frame = Vec::with_capacity(bytes.len() + 10);
    frame.push(0x81);
    if bytes.len() < 126 {
        frame.push(0x80 | bytes.len() as u8);
    } else {
        frame.push(0x80 | 126);
        frame.extend_from_slice(&(bytes.len() as u16).to_be_bytes());
    }
    // All-zero mask.
    frame.extend_from_slice(&[0_u8; 4]);
    frame.extend_from_slice(bytes);
    stream.write_all(&frame).expect("write masked");
    stream.flush().expect("flush");
}

fn drain_frames(stream: &mut TcpStream, buffer: &mut Vec<u8>, want: usize) -> Vec<PhoenixFrame> {
    let mut frames = Vec::new();
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    while frames.len() < want && std::time::Instant::now() < deadline {
        let mut chunk = [0_u8; 1024];
        match stream.read(&mut chunk) {
            Ok(0) => break,
            Ok(n) => buffer.extend_from_slice(&chunk[..n]),
            Err(_) => break,
        }
        while let Ok(Some((opcode, payload, consumed))) = decode_frame(buffer) {
            buffer.drain(..consumed);
            if opcode == 0x1 {
                let text = String::from_utf8_lossy(&payload).to_string();
                if let Ok(frame) = PhoenixFrame::decode(&text) {
                    frames.push(frame);
                }
            }
        }
    }
    frames
}

#[allow(dead_code)]
fn _force_link(_: &[u8]) {
    let _ = encode_text_frame("x");
}
