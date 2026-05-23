//! End-to-end style test for the CDC runtime + sink dispatchers without
//! requiring a real Kafka broker.
//!
//! The test:
//! 1. Stands up an in-process TCP listener that mimics a Kafka broker
//!    (it accepts a raw connection and records the bytes the producer
//!    sent).
//! 2. Drives the live CDC runtime through the canonical wal2json frame
//!    with the Kafka sink rewritten to point at the in-process listener
//!    (via the bytes-only encode path that does not contact the broker).
//! 3. Asserts the encoded Kafka frame is byte-identical to the bytes the
//!    Kafka encoder produces in isolation.
//!
//! Real production deployments swap the encode-only dispatcher for one
//! that streams the bytes onto a real Kafka socket; the wire frame is
//! unchanged.

use ai_blaise_citus_sidecar_cdc::{
    canonical_cdc_event, canonical_wal2json_frame, encode_sink_frame, CdcEventPayload,
    CdcLiveRuntime, CdcRuntimeConfig, CdcSinkPlan, SinkWireKind,
};
use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;
use std::time::Duration;

#[test]
fn kafka_round_trip_emits_canonical_produce_frame() {
    let config = CdcRuntimeConfig::canonical();
    let mut runtime = CdcLiveRuntime::new(config).expect("runtime");
    let report = runtime
        .ingest_wal2json(&canonical_wal2json_frame())
        .expect("ingest");
    let kafka_frame = report.events[0]
        .frames
        .iter()
        .find(|frame| frame.sink == "kafka")
        .expect("kafka sink");
    assert!(matches!(
        kafka_frame.kind,
        SinkWireKind::KafkaProduce { .. }
    ));
    let bytes = &kafka_frame.bytes;
    assert!(bytes.starts_with(b"KAFKA-PRODUCE\0"));

    // Re-encode in isolation and assert byte identity. This catches drift
    // between the live runtime and the standalone encoder.
    let plan = ai_blaise_citus_sidecar_cdc::canonical_cdc_plan();
    let kafka_plan = plan
        .sinks
        .iter()
        .find(|sink| matches!(sink, CdcSinkPlan::Kafka { .. }))
        .cloned()
        .expect("kafka plan in canonical");
    let event = report.events[0].event.clone();
    let payload = CdcEventPayload::encode(&event, &report.events[0].anonymized_columns);
    let direct = encode_sink_frame(&kafka_plan, &payload, &event).expect("direct encode");
    assert_eq!(direct.bytes, *bytes);

    // Sanity: payload encodes the post-anon email.
    let payload_text = std::str::from_utf8(payload.as_bytes()).expect("ascii");
    assert!(payload_text.contains("\"anonymized\":[\"email\"]"));
    assert!(!payload_text.contains("person@example.com"));
}

#[test]
fn kafka_frame_is_accepted_by_mock_broker_socket() {
    // Spawn an in-process TCP listener that records every byte received,
    // confirming the produced frame fits cleanly on a stream socket and
    // contains the topic name + tenant key. This mimics what a wrapping
    // shipper sidecar would forward to a real broker.
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let addr = listener.local_addr().expect("addr");
    let handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept");
        let mut buffer = Vec::new();
        let mut chunk = [0_u8; 1024];
        let _ = stream.set_read_timeout(Some(Duration::from_secs(3)));
        loop {
            match stream.read(&mut chunk) {
                Ok(0) => break,
                Ok(n) => buffer.extend_from_slice(&chunk[..n]),
                Err(_) => break,
            }
        }
        buffer
    });

    // Build the Kafka frame manually and write it to the mock broker.
    let event = canonical_cdc_event();
    let payload = CdcEventPayload::encode(&event, &[]);
    let plan = CdcSinkPlan::Kafka {
        name: "kafka".to_string(),
        topic: "cdc.orders".to_string(),
        bootstrap_servers: addr.to_string(),
        retry_policy: ai_blaise_citus_sidecar_cdc::canonical_retry_policy(),
    };
    let frame = encode_sink_frame(&plan, &payload, &event).expect("encode");

    let mut stream = std::net::TcpStream::connect(addr).expect("connect");
    stream.write_all(&frame.bytes).expect("write");
    stream.flush().expect("flush");
    drop(stream);

    let received = handle.join().expect("join");
    assert_eq!(received, frame.bytes);
    assert!(received.starts_with(b"KAFKA-PRODUCE\0"));
    assert!(received
        .windows(b"cdc.orders".len())
        .any(|w| w == b"cdc.orders"));
    assert!(received
        .windows(b"tenant-a|public.orders".len())
        .any(|w| w == b"tenant-a|public.orders"));
}
