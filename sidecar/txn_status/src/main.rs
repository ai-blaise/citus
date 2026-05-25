// FEATURE: T5

use ai_blaise_citus_sidecar_hlc::HlcTimestamp;
use ai_blaise_citus_sidecar_shared::{listen_addr_from_env, HttpProbeResponse, SidecarRuntime};
use ai_blaise_citus_sidecar_txn_status::{
    canonical_txn_runtime_report, canonical_txn_status_report, finalize_decision_name,
    render_finalize_json, render_record_json, run_parallel_commit_microbench, txn_status_name,
    ParallelCommitMicrobench, ParallelCommitRecord, TxnFinalizeDecision, TxnIntent,
    TxnRaftReplication, TxnStatus, TxnStatusRuntime,
};
use serde::Deserialize;
use serde_json::json;
use std::env;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::process;
use std::sync::Mutex;

const MAX_HTTP_REQUEST_BYTES: usize = 65_536;

fn main() {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print_usage();
        return;
    }

    if args == ["serve"] {
        run_txn_status_server("0.0.0.0:8080");
        return;
    }

    if args == ["run-runtime-canonical"] {
        run_runtime_canonical();
        return;
    }

    if let Some(arg) = args.first() {
        if arg == "run-parallel-commit-microbench" {
            let shard_count = args
                .get(1)
                .and_then(|value| value.parse::<u32>().ok())
                .unwrap_or(3);
            run_microbench(shard_count);
            return;
        }
    }

    if !args.is_empty() && args != ["run-canonical"] {
        eprintln!("txn-status: unknown command");
        print_usage();
        process::exit(2);
    }

    let report = canonical_txn_status_report().unwrap_or_else(|error| {
        eprintln!("txn-status: canonical report failed: {error}");
        process::exit(1);
    });
    let first_intent = &report.record.intents[0];

    println!(
        "raft_group\tvoters\tclock_physical_ms\ttxn_id\tcoordinator\tstatus\tstaging_physical_ms\tobserved_physical_ms\tmax_staging_ms\tintent_count\tfirst_shard_id\tfirst_key_range\tfirst_replica_acks\tfirst_required_acks\tdecision"
    );
    println!(
        "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
        report.service.raft_group,
        report.service.voters.join(","),
        report.service.clock.physical_ms,
        report.record.txn_id,
        report.record.coordinator,
        status_name(&report.record.status),
        report.record.staging_at.physical_ms,
        report.observed_at.physical_ms,
        report.service.max_staging_ms,
        report.record.intents.len(),
        first_intent.shard_id,
        first_intent.key_range,
        first_intent.replica_ack_count,
        first_intent.required_acks,
        decision_name(&report.decision),
    );
}

fn run_runtime_canonical() {
    let report = canonical_txn_runtime_report().unwrap_or_else(|error| {
        eprintln!("txn-status: canonical runtime report failed: {error}");
        process::exit(1);
    });
    let staged = &report.staged_record;
    let finalized = &report.finalized_record;
    println!(
        "raft_group\tvoters\tmax_staging_ms\ttxn_id\tstaged_status\tstaged_raft_index\tfinalize_decision\tfinalized_status\tfinalized_raft_index\tintent_count"
    );
    println!(
        "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
        report.raft_group,
        report.voters.join(","),
        report.max_staging_ms,
        staged.txn_id,
        txn_status_name(staged.status),
        staged.raft_index,
        finalize_decision_name(report.finalize_decision),
        txn_status_name(finalized.status),
        finalized.raft_index,
        staged.intents.len(),
    );
}

fn run_microbench(shard_count: u32) {
    let micro = run_parallel_commit_microbench(shard_count);
    emit_microbench(&micro);
}

fn emit_microbench(micro: &ParallelCommitMicrobench) {
    println!("shard_count\ttwo_phase_commit_steps\tparallel_commit_steps\tspeedup");
    println!(
        "{}\t{}\t{}\t{:.3}",
        micro.shard_count,
        micro.two_phase_commit_steps,
        micro.parallel_commit_steps,
        micro.speedup(),
    );
}

fn print_usage() {
    println!(
        "usage: txn-status [serve|run-canonical|run-runtime-canonical|run-parallel-commit-microbench [SHARD_COUNT]]"
    );
    println!(
        "runs the deterministic canonical transaction-status sidecar plan, runtime, or parallel-commit microbenchmark and emits TSV"
    );
}

fn run_txn_status_server(default_addr: &str) {
    let listen_addr = match listen_addr_from_env(default_addr) {
        Ok(addr) => addr,
        Err(error) => {
            eprintln!("txn-status: invalid listen address: {error}");
            process::exit(1);
        }
    };
    let listener = match TcpListener::bind(&listen_addr) {
        Ok(listener) => listener,
        Err(error) => {
            eprintln!("txn-status: bind failed: {error}");
            process::exit(1);
        }
    };
    eprintln!("ai-blaise txn-status HTTP server listening on {listen_addr}");

    let runtime = match TxnStatusRuntime::new(
        env::var("AI_BLAISE_TXN_RAFT_GROUP").unwrap_or_else(|_| "txn-status-orders".to_string()),
        parse_voters(),
        env::var("AI_BLAISE_TXN_MAX_STAGING_MS")
            .ok()
            .and_then(|value| value.parse::<u64>().ok())
            .unwrap_or(5_000),
    ) {
        Ok(runtime) => match raft_replication_from_env() {
            Ok(replication) => Mutex::new(runtime.with_raft_replication(replication)),
            Err(error) => {
                eprintln!("txn-status: raft replication config failed: {error}");
                process::exit(1);
            }
        },
        Err(error) => {
            eprintln!("txn-status: runtime init failed: {error}");
            process::exit(1);
        }
    };

    for stream in listener.incoming() {
        let mut stream = match stream {
            Ok(stream) => stream,
            Err(error) => {
                eprintln!("txn-status: accept failed: {error}");
                continue;
            }
        };
        let request = match read_http_request(&mut stream) {
            Ok(request) => request,
            Err(error) => {
                eprintln!("txn-status: read failed: {error}");
                continue;
            }
        };
        let response = handle_request(&request, &runtime);
        if let Err(error) = stream.write_all(response.to_http_string().as_bytes()) {
            eprintln!("txn-status: write failed: {error}");
        }
    }
}

fn parse_voters() -> Vec<String> {
    env::var("AI_BLAISE_TXN_RAFT_VOTERS")
        .unwrap_or_else(|_| "worker-a,worker-b,worker-c".to_string())
        .split(',')
        .map(str::trim)
        .filter(|voter| !voter.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn raft_replication_from_env() -> Result<TxnRaftReplication, String> {
    match env::var("AI_BLAISE_TXN_RAFT_LEADER_ADDR") {
        Ok(leader_addr) => {
            TxnRaftReplication::http_leader(leader_addr).map_err(|error| error.to_string())
        }
        Err(env::VarError::NotPresent) => Ok(TxnRaftReplication::InProcess),
        Err(error) => Err(format!("AI_BLAISE_TXN_RAFT_LEADER_ADDR: {error}")),
    }
}

fn handle_request(request: &[u8], runtime: &Mutex<TxnStatusRuntime>) -> HttpProbeResponse {
    if request.len() >= MAX_HTTP_REQUEST_BYTES && !request_complete(request) {
        return error_response(400, "request exceeds maximum HTTP request size");
    }
    let Ok(text) = std::str::from_utf8(request) else {
        return error_response(400, "invalid utf-8");
    };
    let (head, body) = match text.split_once("\r\n\r\n") {
        Some((head, body)) => (head, body),
        None => match text.split_once("\n\n") {
            Some((head, body)) => (head, body),
            None => (text, ""),
        },
    };
    let request_line = head.lines().next().unwrap_or("");
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or("");
    let path = parts.next().unwrap_or("");

    match (method, path) {
        ("POST", "/txn/staging") => match parse_staging_body(body) {
            Ok(record) => {
                let mut guard = runtime.lock().expect("runtime mutex");
                match guard.stage(record) {
                    Ok(staged) => {
                        HttpProbeResponse::new(201, "application/json", render_record_json(&staged))
                    }
                    Err(error) => error_response(409, &error.to_string()),
                }
            }
            Err(error) => error_response(400, &error),
        },
        ("POST", "/txn/finalize") => match parse_finalize_body(body) {
            Ok((txn_id, observed_at)) => {
                let mut guard = runtime.lock().expect("runtime mutex");
                match guard.finalize(&txn_id, observed_at) {
                    Ok((record, decision)) => HttpProbeResponse::new(
                        200,
                        "application/json",
                        render_finalize_json(&record, decision),
                    ),
                    Err(error) => error_response(404, &error.to_string()),
                }
            }
            Err(error) => error_response(400, &error),
        },
        ("POST", "/txn/ack") => match parse_ack_body(body) {
            Ok((txn_id, shard_id, replica_acks)) => {
                let mut guard = runtime.lock().expect("runtime mutex");
                match guard.record_replica_ack(&txn_id, shard_id, replica_acks) {
                    Ok(record) => {
                        HttpProbeResponse::new(200, "application/json", render_record_json(&record))
                    }
                    Err(error) => error_response(404, &error.to_string()),
                }
            }
            Err(error) => error_response(400, &error),
        },
        ("GET", path) if path.starts_with("/txn/") && path.ends_with("/status") => {
            let txn_id = &path["/txn/".len()..path.len() - "/status".len()];
            let guard = runtime.lock().expect("runtime mutex");
            match guard.status(txn_id) {
                Ok(record) => {
                    HttpProbeResponse::new(200, "application/json", render_record_json(record))
                }
                Err(error) => error_response(404, &error.to_string()),
            }
        }
        (_, "/txn/staging") | (_, "/txn/finalize") | (_, "/txn/ack") => HttpProbeResponse::new(
            405,
            "application/json",
            "{\"error\":\"method not allowed\"}\n",
        ),
        _ => {
            let mut probe = SidecarRuntime::ready("txn-status");
            probe
                .handle_http_bytes(request)
                .unwrap_or_else(|error| error_response(400, &error.to_string()))
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct StagingRequest {
    txn_id: String,
    coordinator: String,
    staging_physical_ms: u64,
    intents: Vec<IntentRequest>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct IntentRequest {
    shard_id: u64,
    key_range: String,
    required_acks: u32,
    #[serde(default)]
    replica_acks: u32,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct FinalizeRequest {
    txn_id: String,
    observed_physical_ms: u64,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct AckRequest {
    txn_id: String,
    shard_id: u64,
    replica_acks: u32,
}

fn parse_staging_body(body: &str) -> Result<ParallelCommitRecord, String> {
    let request: StagingRequest =
        serde_json::from_str(body).map_err(|error| format!("invalid staging JSON: {error}"))?;
    let record = ParallelCommitRecord {
        txn_id: request.txn_id,
        coordinator: request.coordinator,
        status: TxnStatus::Staging,
        staging_at: HlcTimestamp {
            physical_ms: request.staging_physical_ms,
            logical: 0,
        },
        intents: request
            .intents
            .into_iter()
            .map(|intent| TxnIntent {
                shard_id: intent.shard_id,
                key_range: intent.key_range,
                replica_ack_count: intent.replica_acks,
                required_acks: intent.required_acks,
            })
            .collect(),
    };
    record.validate().map_err(|error| error.to_string())?;
    Ok(record)
}

fn parse_finalize_body(body: &str) -> Result<(String, HlcTimestamp), String> {
    let request: FinalizeRequest =
        serde_json::from_str(body).map_err(|error| format!("invalid finalize JSON: {error}"))?;
    Ok((
        request.txn_id,
        HlcTimestamp {
            physical_ms: request.observed_physical_ms,
            logical: 0,
        },
    ))
}

fn parse_ack_body(body: &str) -> Result<(String, u64, u32), String> {
    let request: AckRequest =
        serde_json::from_str(body).map_err(|error| format!("invalid ack JSON: {error}"))?;
    if request.txn_id.trim().is_empty() {
        return Err("txn_id must not be empty".to_string());
    }
    if request.shard_id == 0 {
        return Err("shard_id must be greater than zero".to_string());
    }
    Ok((request.txn_id, request.shard_id, request.replica_acks))
}

fn error_response(status_code: u16, message: &str) -> HttpProbeResponse {
    HttpProbeResponse::new(
        status_code,
        "application/json",
        format!("{}\n", json!({ "error": message })),
    )
}

fn read_http_request(stream: &mut std::net::TcpStream) -> Result<Vec<u8>, std::io::Error> {
    stream.set_read_timeout(Some(std::time::Duration::from_secs(5)))?;
    let mut request = Vec::new();
    let mut chunk = [0_u8; 4096];

    loop {
        let read_len = stream.read(&mut chunk)?;
        if read_len == 0 {
            break;
        }
        request.extend_from_slice(&chunk[..read_len]);
        if request_complete(&request) || request.len() >= MAX_HTTP_REQUEST_BYTES {
            break;
        }
    }
    Ok(request)
}

fn request_complete(request: &[u8]) -> bool {
    let Some(head_end) =
        window_find(request, b"\r\n\r\n").or_else(|| window_find(request, b"\n\n"))
    else {
        return false;
    };
    let header_block = std::str::from_utf8(&request[..head_end]).unwrap_or("");
    let content_length = header_block
        .lines()
        .filter_map(|line| line.split_once(':'))
        .find(|(name, _)| name.eq_ignore_ascii_case("content-length"))
        .and_then(|(_, value)| value.trim().parse::<usize>().ok())
        .unwrap_or(0);
    let separator_len = if window_find(request, b"\r\n\r\n").is_some() {
        4
    } else {
        2
    };
    request.len() >= head_end + separator_len + content_length
}

fn window_find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

fn status_name(status: &TxnStatus) -> &'static str {
    match status {
        TxnStatus::Pending => "pending",
        TxnStatus::Staging => "staging",
        TxnStatus::Committed => "committed",
        TxnStatus::Aborted => "aborted",
    }
}

fn decision_name(decision: &TxnFinalizeDecision) -> &'static str {
    match decision {
        TxnFinalizeDecision::Commit => "commit",
        TxnFinalizeDecision::WaitForReplicationEvidence => "wait_for_replication_evidence",
        TxnFinalizeDecision::AbortStaleStagingRecord => "abort_stale_staging_record",
        TxnFinalizeDecision::FallbackToTwoPhaseCommit => "fallback_to_two_phase_commit",
        TxnFinalizeDecision::AlreadyCommitted => "already_committed",
        TxnFinalizeDecision::AlreadyAborted => "already_aborted",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn runtime() -> Mutex<TxnStatusRuntime> {
        Mutex::new(
            TxnStatusRuntime::new(
                "txn-status-orders",
                vec![
                    "worker-a".to_string(),
                    "worker-b".to_string(),
                    "worker-c".to_string(),
                ],
                5_000,
            )
            .expect("runtime"),
        )
    }

    fn request(method: &str, path: &str, body: &str) -> Vec<u8> {
        format!(
            "{method} {path} HTTP/1.1\r\ncontent-type: application/json\r\ncontent-length: {}\r\n\r\n{body}",
            body.len()
        )
        .into_bytes()
    }

    #[test]
    fn http_lifecycle_waits_then_commits_after_ack() {
        let runtime = runtime();
        let stage_body = r#"{"txn_id":"txn-http-1","coordinator":"worker-a","staging_physical_ms":1700000000,"intents":[{"shard_id":10,"key_range":"[a,m)","required_acks":2,"replica_acks":0}]}"#;
        let staged = handle_request(&request("POST", "/txn/staging", stage_body), &runtime);
        assert_eq!(staged.status_code, 201);
        assert!(staged.body.contains("\"status\":\"staging\""));

        let finalize_body = r#"{"txn_id":"txn-http-1","observed_physical_ms":1700000010}"#;
        let waiting = handle_request(&request("POST", "/txn/finalize", finalize_body), &runtime);
        assert_eq!(waiting.status_code, 200);
        assert!(waiting
            .body
            .contains("\"decision\":\"wait_for_replication_evidence\""));

        let ack_body = r#"{"txn_id":"txn-http-1","shard_id":10,"replica_acks":2}"#;
        let acked = handle_request(&request("POST", "/txn/ack", ack_body), &runtime);
        assert_eq!(acked.status_code, 200);
        assert!(acked.body.contains("\"replica_acks\":2"));

        let committed = handle_request(&request("POST", "/txn/finalize", finalize_body), &runtime);
        assert_eq!(committed.status_code, 200);
        assert!(committed.body.contains("\"decision\":\"commit\""));
        assert!(committed.body.contains("\"status\":\"committed\""));
    }

    #[test]
    fn malformed_json_and_unknown_fields_fail_closed() {
        let runtime = runtime();
        let bad_json = handle_request(&request("POST", "/txn/staging", "{"), &runtime);
        assert_eq!(bad_json.status_code, 400);

        let extra_field = r#"{"txn_id":"txn-http-1","coordinator":"worker-a","staging_physical_ms":1700000000,"unexpected":true,"intents":[]}"#;
        let rejected = handle_request(&request("POST", "/txn/staging", extra_field), &runtime);
        assert_eq!(rejected.status_code, 400);
    }

    #[test]
    fn invalid_utf8_is_rejected() {
        let runtime = runtime();
        let response = handle_request(&[0xff, 0xfe, 0xfd], &runtime);
        assert_eq!(response.status_code, 400);
    }
}
