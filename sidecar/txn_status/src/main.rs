// FEATURE: T5
// FEATURE: T5

use ai_blaise_citus_sidecar_hlc::HlcTimestamp;
use ai_blaise_citus_sidecar_shared::{listen_addr_from_env, HttpProbeResponse, SidecarRuntime};
use ai_blaise_citus_sidecar_txn_status::{
    canonical_txn_runtime_report, canonical_txn_status_report, finalize_decision_name,
    render_finalize_json, render_record_json, run_parallel_commit_microbench, txn_status_name,
    ParallelCommitMicrobench, ParallelCommitRecord, TxnFinalizeDecision, TxnIntent, TxnStatus,
    TxnStatusRuntime,
};
use std::env;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::process;
use std::sync::Mutex;

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
        vec![
            "worker-a".to_string(),
            "worker-b".to_string(),
            "worker-c".to_string(),
        ],
        env::var("AI_BLAISE_TXN_MAX_STAGING_MS")
            .ok()
            .and_then(|value| value.parse::<u64>().ok())
            .unwrap_or(5_000),
    ) {
        Ok(runtime) => Mutex::new(runtime),
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

fn handle_request(request: &[u8], runtime: &Mutex<TxnStatusRuntime>) -> HttpProbeResponse {
    let Ok(text) = std::str::from_utf8(request) else {
        return HttpProbeResponse::new(400, "application/json", "{\"error\":\"invalid utf-8\"}\n");
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
                    Err(error) => HttpProbeResponse::new(
                        409,
                        "application/json",
                        format!("{{\"error\":\"{error}\"}}\n"),
                    ),
                }
            }
            Err(error) => HttpProbeResponse::new(
                400,
                "application/json",
                format!("{{\"error\":\"{error}\"}}\n"),
            ),
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
                    Err(error) => HttpProbeResponse::new(
                        404,
                        "application/json",
                        format!("{{\"error\":\"{error}\"}}\n"),
                    ),
                }
            }
            Err(error) => HttpProbeResponse::new(
                400,
                "application/json",
                format!("{{\"error\":\"{error}\"}}\n"),
            ),
        },
        ("GET", path) if path.starts_with("/txn/") && path.ends_with("/status") => {
            let txn_id = &path["/txn/".len()..path.len() - "/status".len()];
            let guard = runtime.lock().expect("runtime mutex");
            match guard.status(txn_id) {
                Ok(record) => {
                    HttpProbeResponse::new(200, "application/json", render_record_json(record))
                }
                Err(error) => HttpProbeResponse::new(
                    404,
                    "application/json",
                    format!("{{\"error\":\"{error}\"}}\n"),
                ),
            }
        }
        (_, "/txn/staging") | (_, "/txn/finalize") => HttpProbeResponse::new(
            405,
            "application/json",
            "{\"error\":\"method not allowed\"}\n",
        ),
        _ => {
            let mut probe = SidecarRuntime::ready("txn-status");
            probe.handle_http_bytes(request).unwrap_or_else(|error| {
                HttpProbeResponse::new(
                    400,
                    "application/json",
                    format!("{{\"error\":\"{error}\"}}\n"),
                )
            })
        }
    }
}

fn parse_staging_body(body: &str) -> Result<ParallelCommitRecord, String> {
    // Minimal hand-rolled JSON parser sufficient for the staging shape:
    //   {"txn_id":"...","coordinator":"...","staging_physical_ms":N,
    //    "intents":[{"shard_id":N,"key_range":"...","required_acks":N,"replica_acks":N}]}
    let txn_id = extract_string_field(body, "txn_id").ok_or("missing txn_id")?;
    let coordinator = extract_string_field(body, "coordinator").ok_or("missing coordinator")?;
    let staging_physical_ms = extract_u64_field(body, "staging_physical_ms").unwrap_or(0);
    let intents_section = extract_array_field(body, "intents").ok_or("missing intents")?;
    let intents = parse_intent_objects(&intents_section)?;
    if intents.is_empty() {
        return Err("intents must not be empty".to_string());
    }
    Ok(ParallelCommitRecord {
        txn_id,
        coordinator,
        status: TxnStatus::Staging,
        staging_at: HlcTimestamp {
            physical_ms: staging_physical_ms,
            logical: 0,
        },
        intents,
    })
}

fn parse_finalize_body(body: &str) -> Result<(String, HlcTimestamp), String> {
    let txn_id = extract_string_field(body, "txn_id").ok_or("missing txn_id")?;
    let observed_physical_ms = extract_u64_field(body, "observed_physical_ms").unwrap_or(0);
    Ok((
        txn_id,
        HlcTimestamp {
            physical_ms: observed_physical_ms,
            logical: 0,
        },
    ))
}

fn parse_intent_objects(section: &str) -> Result<Vec<TxnIntent>, String> {
    let mut intents = Vec::new();
    let mut depth = 0;
    let mut start = None;
    for (index, ch) in section.char_indices() {
        match ch {
            '{' => {
                if depth == 0 {
                    start = Some(index);
                }
                depth += 1;
            }
            '}' => {
                depth -= 1;
                if depth == 0 {
                    let from = start.take().ok_or("malformed intents")?;
                    let slice = &section[from..=index];
                    intents.push(parse_intent_object(slice)?);
                }
            }
            _ => {}
        }
    }
    Ok(intents)
}

fn parse_intent_object(slice: &str) -> Result<TxnIntent, String> {
    let shard_id = extract_u64_field(slice, "shard_id").ok_or("missing shard_id")?;
    let key_range = extract_string_field(slice, "key_range").ok_or("missing key_range")?;
    let required_acks =
        extract_u64_field(slice, "required_acks").ok_or("missing required_acks")? as u32;
    let replica_acks = extract_u64_field(slice, "replica_acks").unwrap_or(0) as u32;
    Ok(TxnIntent {
        shard_id,
        key_range,
        replica_ack_count: replica_acks,
        required_acks,
    })
}

fn extract_string_field(input: &str, key: &str) -> Option<String> {
    let needle = format!("\"{key}\"");
    let start = input.find(&needle)?;
    let after = &input[start + needle.len()..];
    let after = after.trim_start();
    let after = after.strip_prefix(':')?.trim_start();
    let after = after.strip_prefix('"')?;
    let end = after.find('"')?;
    Some(after[..end].to_string())
}

fn extract_u64_field(input: &str, key: &str) -> Option<u64> {
    let needle = format!("\"{key}\"");
    let start = input.find(&needle)?;
    let after = &input[start + needle.len()..];
    let after = after.trim_start();
    let after = after.strip_prefix(':')?.trim_start();
    let end = after
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(after.len());
    after[..end].parse::<u64>().ok()
}

fn extract_array_field(input: &str, key: &str) -> Option<String> {
    let needle = format!("\"{key}\"");
    let start = input.find(&needle)?;
    let after = &input[start + needle.len()..];
    let after = after.trim_start();
    let after = after.strip_prefix(':')?.trim_start();
    let after = after.strip_prefix('[')?;
    let mut depth = 1;
    let mut end = None;
    for (index, ch) in after.char_indices() {
        match ch {
            '[' => depth += 1,
            ']' => {
                depth -= 1;
                if depth == 0 {
                    end = Some(index);
                    break;
                }
            }
            _ => {}
        }
    }
    Some(after[..end?].to_string())
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
        if request_complete(&request) || request.len() >= 65_536 {
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
