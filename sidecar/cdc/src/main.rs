// FEATURE: C1
// FEATURE: C2
// FEATURE: C3
// FEATURE: C9
// FEATURE: C14
// FEATURE: C15
// FEATURE: WH3

use ai_blaise_citus_sidecar_cdc::{
    canonical_cdc_event, canonical_cdc_plan, canonical_cdc_runtime_report, canonical_delivery_plan,
    canonical_wal2json_frame, encode_sink_frame, runtime::serve, CdcDispatchReport,
    CdcEventEnvelope, CdcEventPayload, CdcLiveRuntime, CdcOperation, CdcReplicationSource,
    CdcRuntimeConfig, CdcSidecarError, DdlStreamEvent, LogicalReplicationFrame,
    SinkDeliveryOutcome,
};
use ai_blaise_citus_sidecar_shared::listen_addr_from_env;
use serde_json::Value;
use std::env;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::process;
use std::time::Duration;

fn main() {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print_usage();
        return;
    }
    if args == ["serve"] {
        run_serve("0.0.0.0:8080");
        return;
    }

    if args == ["serve-runtime"] {
        serve_runtime();
        return;
    }

    if args == ["run-runtime-canonical"] {
        run_runtime_canonical();
        return;
    }

    if args == ["run-live-canonical"] {
        run_live_canonical();
        return;
    }

    if !args.is_empty() && args != ["run-canonical"] {
        eprintln!("cdc: unknown command");
        print_usage();
        process::exit(2);
    }

    let event = canonical_cdc_event();
    let delivery = canonical_delivery_plan().unwrap_or_else(|error| {
        eprintln!("cdc: canonical delivery failed: {error}");
        process::exit(1);
    });
    let anonymized_columns = delivery.anonymized_columns.join(",");

    println!("lsn\ttable\ttenant_id\toperation\tsink\ttarget\tmax_attempts\tdead_letter_queue\tanonymized_columns");
    for sink in &delivery.routed_sinks {
        println!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            delivery.event_lsn,
            delivery.table,
            event.tenant_id,
            operation_name(&delivery.operation),
            sink.sink,
            sink.target,
            sink.retry_policy.max_attempts,
            sink.retry_policy.dead_letter_queue,
            anonymized_columns,
        );
    }
}

fn run_runtime_canonical() {
    let report = canonical_cdc_runtime_report().unwrap_or_else(|error| {
        eprintln!("cdc: canonical runtime failed: {error}");
        process::exit(1);
    });
    let runtime_delivery = &report.batch.deliveries[0];
    let anonymized_columns = runtime_delivery.delivery.anonymized_columns.join(",");

    println!(
        "slot\tstart_lsn\tend_lsn\tevent_count\tdelivered_sinks\tack_flush_lsn\tlast_delivered_lsn\ttable\ttenant_id\toperation\tanonymized_columns"
    );
    println!(
        "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
        report.state.slot_name,
        report.batch.start_lsn,
        report.batch.end_lsn,
        report.batch.deliveries.len(),
        report.state.delivered_sink_writes,
        report.batch.ack.flush_lsn,
        report.state.last_delivered_lsn,
        runtime_delivery.delivery.table,
        runtime_delivery.event.tenant_id,
        operation_name(&runtime_delivery.delivery.operation),
        anonymized_columns,
    );
}

/// Drive the live runtime through the canonical wal2json frame and emit one
/// row per sink containing the wire-frame bytes count plus the dispatch
/// outcome. Smoke scripts assert against this TSV.
fn run_live_canonical() {
    let mut config = CdcRuntimeConfig::canonical();
    config.dispatch_live = false; // canonical run never touches the network
    let mut runtime = CdcLiveRuntime::new(config).unwrap_or_else(|error| {
        eprintln!("cdc: live runtime failed: {error}");
        process::exit(1);
    });
    let frame = canonical_wal2json_frame();
    let report = runtime.ingest_wal2json(&frame).unwrap_or_else(|error| {
        eprintln!("cdc: live ingest failed: {error}");
        process::exit(1);
    });

    println!("start_lsn\tend_lsn\tsink\ttarget\tbytes\toutcome\tanonymized");
    for event in &report.events {
        for (frame, outcome) in event.frames.iter().zip(event.outcomes.iter()) {
            let outcome_token = match outcome {
                SinkDeliveryOutcome::Encoded => "encoded".to_string(),
                SinkDeliveryOutcome::Delivered { response_summary } => {
                    format!("delivered:{response_summary}")
                }
                SinkDeliveryOutcome::DeadLettered { reason } => format!("dlq:{reason}"),
            };
            println!(
                "{}\t{}\t{}\t{}\t{}\t{}\t{}",
                report.start_lsn,
                report.end_lsn,
                frame.sink,
                frame.target,
                frame.bytes.len(),
                outcome_token,
                event.anonymized_columns.join(","),
            );
        }
    }
}

/// Start the live TCP server. Listens on `CDC_LISTEN_ADDR`
/// (default 127.0.0.1:9300) and accepts:
/// - `POST /ingest`  body = wal2json frame JSON `{"start_lsn":..,"end_lsn":..,"payload":..}`
/// - `GET  /state`   -> JSON runtime state
/// - `GET  /dlq`     -> JSON list of dead-lettered records
/// - `GET  /streams` -> JSON list of configured sinks
/// - `POST /streams/:name/restart` -> 202 with state echo
fn serve_runtime() {
    let plan = canonical_cdc_plan();
    let mut config = CdcRuntimeConfig::canonical();
    config.dispatch_live = env::var("CDC_DISPATCH_LIVE")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);
    config.dlq_path = env::var("CDC_DLQ_PATH").ok();
    config.plan = plan;
    let runtime_inner = CdcLiveRuntime::new(config.clone()).unwrap_or_else(|error| {
        eprintln!("cdc: live runtime init failed: {error}");
        process::exit(1);
    });
    let runtime = std::sync::Arc::new(std::sync::Mutex::new(runtime_inner));

    let addr = env::var("CDC_LISTEN_ADDR").unwrap_or_else(|_| {
        listen_addr_from_env("127.0.0.1:9300").unwrap_or_else(|_| "127.0.0.1:9300".to_string())
    });
    let listener = TcpListener::bind(&addr).unwrap_or_else(|error| {
        eprintln!("cdc: bind {addr} failed: {error}");
        process::exit(1);
    });
    eprintln!("cdc: serve-runtime listening on {addr}");

    let realtime_bridge = env::var("CDC_REALTIME_BRIDGE_ADDR").ok();

    for stream in listener.incoming() {
        let mut stream = match stream {
            Ok(stream) => stream,
            Err(error) => {
                eprintln!("cdc: accept failed: {error}");
                continue;
            }
        };
        let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
        let _ = stream.set_write_timeout(Some(Duration::from_secs(5)));

        let mut buffer = Vec::with_capacity(4096);
        let mut chunk = [0_u8; 1024];
        loop {
            match stream.read(&mut chunk) {
                Ok(0) => break,
                Ok(n) => {
                    buffer.extend_from_slice(&chunk[..n]);
                    if buffer.windows(4).any(|window| window == b"\r\n\r\n") {
                        if let Some(content_length) = parse_content_length(&buffer) {
                            let header_end = find_header_end(&buffer).unwrap_or(buffer.len());
                            let body_have = buffer.len() - header_end;
                            if body_have >= content_length {
                                break;
                            }
                        } else {
                            break;
                        }
                    }
                }
                Err(_) => break,
            }
        }

        let response = match handle_request(&runtime, realtime_bridge.as_deref(), &buffer) {
            Ok(payload) => payload,
            Err(error) => format!(
                "HTTP/1.1 500 Internal Server Error\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                error.len(),
                error
            ),
        };
        let _ = stream.write_all(response.as_bytes());
        let _ = stream.flush();
    }
}

fn handle_request(
    runtime: &std::sync::Mutex<CdcLiveRuntime>,
    realtime_bridge: Option<&str>,
    buffer: &[u8],
) -> Result<String, String> {
    let request = std::str::from_utf8(buffer).map_err(|e| e.to_string())?;
    let mut lines = request.lines();
    let request_line = lines.next().ok_or("empty request")?;
    let mut parts = request_line.split_whitespace();
    let method = parts.next().ok_or("missing method")?;
    let path = parts.next().ok_or("missing path")?;

    match (method, path) {
        ("GET", "/healthz") => Ok(http_response(
            200,
            "application/json",
            "{\"status\":\"ok\",\"component\":\"cdc\"}",
        )),
        ("GET", "/readyz") => {
            let runtime = runtime.lock().map_err(|_| "lock")?;
            let state = runtime.runtime().state();
            let body = serde_json::json!({
                "ready": true,
                "slot": state.slot_name,
                "acked_flush_lsn": state.acked_flush_lsn,
            })
            .to_string();
            Ok(http_response(200, "application/json", &body))
        }
        ("GET", "/metrics") => {
            let runtime = runtime.lock().map_err(|_| "lock")?;
            let body = cdc_metrics_text(&runtime);
            Ok(http_response(200, "text/plain; version=0.0.4", &body))
        }
        ("GET", "/streams") => {
            let runtime = runtime.lock().map_err(|_| "lock")?;
            let body = streams_json(runtime.plan());
            Ok(http_response(200, "application/json", &body))
        }
        ("GET", "/state") => {
            let runtime = runtime.lock().map_err(|_| "lock")?;
            let state = runtime.runtime().state();
            let body = serde_json::json!({
                "slot": state.slot_name,
                "last_received_lsn": state.last_received_lsn,
                "last_delivered_lsn": state.last_delivered_lsn,
                "acked_flush_lsn": state.acked_flush_lsn,
                "delivered_events": state.delivered_events,
                "delivered_sink_writes": state.delivered_sink_writes,
                "dlq_pending": runtime.dlq().len(),
            })
            .to_string();
            Ok(http_response(200, "application/json", &body))
        }
        ("GET", "/dlq") => {
            let runtime = runtime.lock().map_err(|_| "lock")?;
            let records = runtime.dlq().records().map_err(|e| e.to_string())?;
            let body =
                serde_json::Value::Array(records.iter().map(|record| record.as_json()).collect())
                    .to_string();
            Ok(http_response(200, "application/json", &body))
        }
        ("POST", path) if path.starts_with("/streams/") && path.ends_with("/restart") => {
            let runtime = runtime.lock().map_err(|_| "lock")?;
            let state = runtime.runtime().state();
            let body = serde_json::json!({
                "restarted": true,
                "slot": state.slot_name,
            })
            .to_string();
            Ok(http_response(202, "application/json", &body))
        }
        ("POST", "/ingest") => {
            let body = request_body(buffer)?;
            let frame: serde_json::Value =
                serde_json::from_slice(body).map_err(|e| e.to_string())?;
            let frame = LogicalReplicationFrame {
                start_lsn: frame
                    .get("start_lsn")
                    .and_then(Value::as_str)
                    .ok_or("missing start_lsn")?
                    .to_string(),
                end_lsn: frame
                    .get("end_lsn")
                    .and_then(Value::as_str)
                    .ok_or("missing end_lsn")?
                    .to_string(),
                payload: frame
                    .get("payload")
                    .and_then(Value::as_str)
                    .ok_or("missing payload")?
                    .to_string(),
            };
            let report = {
                let mut runtime = runtime.lock().map_err(|_| "lock")?;
                runtime
                    .ingest_wal2json(&frame)
                    .map_err(|e: CdcSidecarError| e.to_string())?
            };
            if let Some(addr) = realtime_bridge {
                bridge_realtime(addr, &report);
            }
            Ok(http_response(
                200,
                "application/json",
                &dispatch_report_json(&report),
            ))
        }
        _ => Ok(http_response(
            404,
            "application/json",
            "{\"error\":\"not found\"}",
        )),
    }
}

fn http_response(status: u16, content_type: &str, body: &str) -> String {
    let reason = match status {
        200 => "OK",
        202 => "Accepted",
        404 => "Not Found",
        _ => "Error",
    };
    format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body,
    )
}

fn parse_content_length(buffer: &[u8]) -> Option<usize> {
    let text = std::str::from_utf8(buffer).ok()?;
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("Content-Length:") {
            return rest.trim().parse().ok();
        }
        if let Some(rest) = line.strip_prefix("content-length:") {
            return rest.trim().parse().ok();
        }
    }
    None
}

fn find_header_end(buffer: &[u8]) -> Option<usize> {
    buffer
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .map(|pos| pos + 4)
}

fn request_body(buffer: &[u8]) -> Result<&[u8], String> {
    let header_end = find_header_end(buffer).ok_or("malformed request")?;
    Ok(&buffer[header_end..])
}

fn streams_json(plan: &ai_blaise_citus_sidecar_cdc::CdcSidecarPlan) -> String {
    let entries: Vec<serde_json::Value> = plan
        .sinks
        .iter()
        .map(|sink| {
            let (kind, target) = match sink {
                ai_blaise_citus_sidecar_cdc::CdcSinkPlan::Webhook { url, .. } => {
                    ("webhook", url.as_str())
                }
                ai_blaise_citus_sidecar_cdc::CdcSinkPlan::Realtime { topic_prefix, .. } => {
                    ("realtime", topic_prefix.as_str())
                }
                ai_blaise_citus_sidecar_cdc::CdcSinkPlan::AnalyticalMirror {
                    mirror_name, ..
                } => ("analytical-mirror", mirror_name.as_str()),
                ai_blaise_citus_sidecar_cdc::CdcSinkPlan::Kafka { topic, .. } => {
                    ("kafka", topic.as_str())
                }
                ai_blaise_citus_sidecar_cdc::CdcSinkPlan::Nats { subject, .. } => {
                    ("nats", subject.as_str())
                }
                ai_blaise_citus_sidecar_cdc::CdcSinkPlan::PubSub { topic, .. } => {
                    ("pubsub", topic.as_str())
                }
                ai_blaise_citus_sidecar_cdc::CdcSinkPlan::Kinesis { stream_name, .. } => {
                    ("kinesis", stream_name.as_str())
                }
                ai_blaise_citus_sidecar_cdc::CdcSinkPlan::Http2 { url, .. } => {
                    ("http2", url.as_str())
                }
            };
            serde_json::json!({"kind": kind, "target": target})
        })
        .collect();
    serde_json::Value::Array(entries).to_string()
}

fn cdc_metrics_text(runtime: &CdcLiveRuntime) -> String {
    let state = runtime.runtime().state();
    format!(
        "# HELP ai_blaise_cdc_delivered_events Total CDC events delivered.\n\
# TYPE ai_blaise_cdc_delivered_events counter\n\
ai_blaise_cdc_delivered_events {}\n\
# HELP ai_blaise_cdc_delivered_sink_writes Total sink writes encoded or delivered.\n\
# TYPE ai_blaise_cdc_delivered_sink_writes counter\n\
ai_blaise_cdc_delivered_sink_writes {}\n\
# HELP ai_blaise_cdc_dlq_pending Dead-letter queue entries pending.\n\
# TYPE ai_blaise_cdc_dlq_pending gauge\n\
ai_blaise_cdc_dlq_pending {}\n",
        state.delivered_events,
        state.delivered_sink_writes,
        runtime.dlq().len(),
    )
}

fn dispatch_report_json(report: &CdcDispatchReport) -> String {
    let events: Vec<serde_json::Value> = report
        .events
        .iter()
        .map(|event| {
            let frames: Vec<serde_json::Value> = event
                .frames
                .iter()
                .zip(event.outcomes.iter())
                .map(|(frame, outcome)| {
                    let outcome_token = match outcome {
                        SinkDeliveryOutcome::Encoded => "encoded".to_string(),
                        SinkDeliveryOutcome::Delivered { response_summary } => {
                            format!("delivered:{response_summary}")
                        }
                        SinkDeliveryOutcome::DeadLettered { reason } => format!("dlq:{reason}"),
                    };
                    serde_json::json!({
                        "sink": frame.sink,
                        "target": frame.target,
                        "bytes": frame.bytes.len(),
                        "outcome": outcome_token,
                    })
                })
                .collect();
            serde_json::json!({
                "tenant_id": event.event.tenant_id,
                "lsn": event.event.lsn,
                "table": format!("{}.{}", event.event.schema, event.event.table),
                "operation": operation_name(&event.event.operation),
                "anonymized_columns": event.anonymized_columns,
                "ddl_event": event.ddl_event.as_ref().map(ddl_event_json),
                "frames": frames,
            })
        })
        .collect();
    let ddl_events: Vec<serde_json::Value> = report.ddl_events.iter().map(ddl_event_json).collect();
    serde_json::json!({
        "start_lsn": report.start_lsn,
        "end_lsn": report.end_lsn,
        "ddl_events_total": report.ddl_events.len(),
        "ddl_events": ddl_events,
        "dlq_total": report.dlq_total,
        "bytes_total": report.bytes_total,
        "events": events,
    })
    .to_string()
}

fn ddl_event_json(event: &DdlStreamEvent) -> serde_json::Value {
    serde_json::json!({
        "lsn": event.lsn,
        "ddl_stream_table": event.ddl_stream_table,
        "command_tag": event.command_tag,
        "object_schema": event.object_schema,
        "object_identity": event.object_identity,
        "ddl": event.ddl,
        "occurred_at": event.occurred_at,
    })
}

fn bridge_realtime(addr: &str, report: &CdcDispatchReport) {
    if let Some(path) = unix_addr_path(addr) {
        bridge_realtime_unix(path, report);
        return;
    }
    let socket_addr: SocketAddr = match addr.parse() {
        Ok(addr) => addr,
        Err(error) => {
            eprintln!("cdc: realtime bridge address {addr} invalid: {error}");
            return;
        }
    };
    let Ok(mut stream) = TcpStream::connect_timeout(&socket_addr, Duration::from_secs(2)) else {
        eprintln!("cdc: realtime bridge {addr} unreachable; events not relayed");
        return;
    };
    write_realtime_report(&mut stream, report);
}

#[cfg(unix)]
fn bridge_realtime_unix(path: &str, report: &CdcDispatchReport) {
    match std::os::unix::net::UnixStream::connect(path) {
        Ok(mut stream) => write_realtime_report(&mut stream, report),
        Err(error) => eprintln!("cdc: realtime UDS bridge {path} unreachable: {error}"),
    }
}

#[cfg(not(unix))]
fn bridge_realtime_unix(path: &str, _report: &CdcDispatchReport) {
    eprintln!("cdc: realtime UDS bridge {path} is unsupported on this platform");
}

fn unix_addr_path(addr: &str) -> Option<&str> {
    addr.strip_prefix("unix://")
        .or_else(|| addr.strip_prefix("unix:"))
        .filter(|path| !path.is_empty())
}

fn write_realtime_report<W: Write>(stream: &mut W, report: &CdcDispatchReport) {
    for event in &report.events {
        let payload = CdcEventPayload::encode(&event.event, &event.anonymized_columns);
        let frame = serde_json::json!({
            "type": "cdc_event",
            "tenant_id": event.event.tenant_id,
            "schema": event.event.schema,
            "table": event.event.table,
            "lsn": event.event.lsn,
            "operation": operation_name(&event.event.operation),
            "payload": payload.json,
        })
        .to_string();
        let header = (frame.len() as u32).to_be_bytes();
        let _ = stream.write_all(&header);
        let _ = stream.write_all(frame.as_bytes());
    }
}

fn print_usage() {
    println!(
        "usage: cdc [serve|serve-runtime|run-canonical|run-runtime-canonical|run-live-canonical]"
    );
    println!("runs the CDC sidecar: a deterministic canonical emitter and a live runtime that");
    println!("ingests wal2json frames, applies PII anonymization, encodes per-sink wire frames,");
    println!("and tracks DLQ + ack state.");
}

fn run_serve(default_addr: &'static str) {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap_or_else(|error| {
            eprintln!("cdc: tokio runtime failed: {error}");
            process::exit(1);
        });
    if let Err(error) = runtime.block_on(serve("cdc", default_addr)) {
        eprintln!("cdc: serve failed: {error}");
        process::exit(1);
    }
}

fn operation_name(operation: &CdcOperation) -> &'static str {
    match operation {
        CdcOperation::Insert => "insert",
        CdcOperation::Update => "update",
        CdcOperation::Delete => "delete",
        CdcOperation::Truncate => "truncate",
    }
}

// Suppress unused-import lints for code paths only reached at runtime when
// alternative subcommands are invoked.
#[allow(dead_code)]
fn _force_link(event: &CdcEventEnvelope) {
    let plan = canonical_cdc_plan();
    let payload = CdcEventPayload::encode(event, &[]);
    let _ = encode_sink_frame(&plan.sinks[0], &payload, event);
    let _ = CdcReplicationSource::Wal2Json;
}
