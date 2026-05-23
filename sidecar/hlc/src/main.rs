// FEATURE: S9
// FEATURE: S9
// FEATURE: MR6

use ai_blaise_citus_sidecar_hlc::{
    canonical_hlc_report, canonical_hlc_runtime_report, render_closed_ts_json, HlcRuntimeReport,
};
use ai_blaise_citus_sidecar_shared::{listen_addr_from_env, HttpProbeResponse, SidecarRuntime};
use std::env;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::process;

fn main() {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print_usage();
        return;
    }

    if args == ["serve"] {
        run_hlc_server("0.0.0.0:8080");
        return;
    }

    if args == ["run-runtime-canonical"] {
        run_runtime_canonical();
        return;
    }

    if !args.is_empty() && args != ["run-canonical"] {
        eprintln!("hlc: unknown command");
        print_usage();
        process::exit(2);
    }

    let report = canonical_hlc_report().unwrap_or_else(|error| {
        eprintln!("hlc: canonical report failed: {error}");
        process::exit(1);
    });
    let closed = &report.follower_read.closed_timestamp;

    println!(
        "node_id\ttick_physical_ms\ttick_logical\tobserved_physical_ms\tobserved_logical\tfollower_replica\tas_of_physical_ms\tclosed_shard_group\tclosed_physical_ms\tclosed_logical\tmax_staleness_ms\treplica_count"
    );
    println!(
        "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
        report.ticked_clock.node_id,
        report.ticked_clock.timestamp.physical_ms,
        report.ticked_clock.timestamp.logical,
        report.observed_clock.timestamp.physical_ms,
        report.observed_clock.timestamp.logical,
        report.follower_read.replica,
        report.follower_read.as_of.physical_ms,
        closed.shard_group,
        closed.closed_at.physical_ms,
        closed.closed_at.logical,
        closed.max_staleness_ms,
        closed.replica_count,
    );
}

fn run_runtime_canonical() {
    let report = canonical_hlc_runtime_report().unwrap_or_else(|error| {
        eprintln!("hlc: canonical runtime report failed: {error}");
        process::exit(1);
    });
    emit_runtime_report(&report);
}

fn emit_runtime_report(report: &HlcRuntimeReport) {
    println!(
        "shard_group\tlocal_physical_ms\tlocal_logical\tclosed_physical_ms\tclosed_logical\tmax_offset_ms\tmax_staleness_ms\treplica_count\tpeers"
    );
    let peers = report
        .peers
        .iter()
        .map(|(id, ts)| format!("{id}={}.{}", ts.physical_ms, ts.logical))
        .collect::<Vec<_>>()
        .join(",");
    println!(
        "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
        report.shard_group,
        report.local_clock.physical_ms,
        report.local_clock.logical,
        report.closed_at.physical_ms,
        report.closed_at.logical,
        report.max_offset_ms,
        report.max_staleness_ms,
        report.replica_count,
        peers,
    );
}

fn print_usage() {
    println!("usage: hlc [serve|run-canonical|run-runtime-canonical]");
    println!(
        "runs the deterministic canonical HLC sidecar plan or the multi-node clock-exchange runtime and emits TSV"
    );
}

fn run_hlc_server(default_addr: &str) {
    let listen_addr = match listen_addr_from_env(default_addr) {
        Ok(addr) => addr,
        Err(error) => {
            eprintln!("hlc: invalid listen address: {error}");
            process::exit(1);
        }
    };
    let listener = match TcpListener::bind(&listen_addr) {
        Ok(listener) => listener,
        Err(error) => {
            eprintln!("hlc: bind failed: {error}");
            process::exit(1);
        }
    };
    eprintln!("ai-blaise hlc HTTP server listening on {listen_addr}");

    for stream in listener.incoming() {
        let mut stream = match stream {
            Ok(stream) => stream,
            Err(error) => {
                eprintln!("hlc: accept failed: {error}");
                continue;
            }
        };
        let request = match read_http_request(&mut stream) {
            Ok(request) => request,
            Err(error) => {
                eprintln!("hlc: read failed: {error}");
                continue;
            }
        };
        let response = handle_request(&request);
        if let Err(error) = stream.write_all(response.to_http_string().as_bytes()) {
            eprintln!("hlc: write failed: {error}");
        }
    }
}

fn handle_request(request: &[u8]) -> HttpProbeResponse {
    let Ok(text) = std::str::from_utf8(request) else {
        return HttpProbeResponse::new(400, "application/json", "{\"error\":\"invalid utf-8\"}\n");
    };
    let request_line = text.lines().next().unwrap_or("");
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or("");
    let path = parts.next().unwrap_or("");

    match (method, path) {
        ("GET", "/closed_ts") => match canonical_hlc_runtime_report() {
            Ok(report) => {
                HttpProbeResponse::new(200, "application/json", render_closed_ts_json(&report))
            }
            Err(error) => HttpProbeResponse::new(
                500,
                "application/json",
                format!("{{\"error\":\"{error}\"}}\n"),
            ),
        },
        (_, "/closed_ts") => HttpProbeResponse::new(
            405,
            "application/json",
            "{\"error\":\"method not allowed\"}\n",
        ),
        _ => {
            let mut runtime = SidecarRuntime::ready("hlc");
            runtime.handle_http_bytes(request).unwrap_or_else(|error| {
                HttpProbeResponse::new(
                    400,
                    "application/json",
                    format!("{{\"error\":\"{error}\"}}\n"),
                )
            })
        }
    }
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
        if request.windows(4).any(|window| window == b"\r\n\r\n") || request.len() >= 65_536 {
            break;
        }
    }
    Ok(request)
}
