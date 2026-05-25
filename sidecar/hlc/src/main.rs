// FEATURE: S9
// FEATURE: MR6
// FEATURE: Edge1

use ai_blaise_citus_sidecar_hlc::{
    canonical_hlc_report, canonical_hlc_runtime_report, render_closed_ts_json, EdgeReadDecision,
    FollowerReadDecision, HlcClock, HlcRuntime, HlcRuntimeError, HlcRuntimeReport, HlcTimestamp,
    PeerClockExchange,
};
use ai_blaise_citus_sidecar_shared::{listen_addr_from_env, HttpProbeResponse, SidecarRuntime};
use std::collections::BTreeMap;
use std::env;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::process;
use std::sync::Mutex;
use std::time::Duration;

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
    let server = match HlcHttpServer::from_env() {
        Ok(server) => server,
        Err(error) => {
            eprintln!("hlc: runtime init failed: {error}");
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
        let response = server.handle_http_bytes(&request);
        if let Err(error) = stream.write_all(response.to_http_string().as_bytes()) {
            eprintln!("hlc: write failed: {error}");
        }
    }
}

struct HlcHttpServer {
    runtime: Mutex<HlcRuntime>,
    probe: Mutex<SidecarRuntime>,
}

impl HlcHttpServer {
    fn from_env() -> Result<Self, String> {
        let node_id = env::var("AI_BLAISE_HLC_NODE_ID").unwrap_or_else(|_| "worker-a".to_string());
        let shard_group =
            env::var("AI_BLAISE_HLC_SHARD_GROUP").unwrap_or_else(|_| "orders-sg".to_string());
        let max_offset_ms = parse_env_u64("AI_BLAISE_HLC_MAX_OFFSET_MS", 500)?;
        let max_staleness_ms = parse_env_u64("AI_BLAISE_HLC_MAX_STALENESS_MS", 5_000)?;
        let initial_physical_ms =
            parse_env_u64("AI_BLAISE_HLC_INITIAL_PHYSICAL_MS", 1_700_000_000)?;
        let peers = parse_csv(
            &env::var("AI_BLAISE_HLC_PEERS").unwrap_or_else(|_| "worker-b,worker-c".to_string()),
        );
        let replica_count_default = peers.len().saturating_add(1) as u32;
        let replica_count =
            parse_env_u32("AI_BLAISE_HLC_REPLICA_COUNT", replica_count_default.max(1))?;
        let edge_replicas = parse_edge_replicas(
            &env::var("AI_BLAISE_HLC_EDGE_REPLICAS").unwrap_or_else(|_| {
                "iad-edge=worker-a-replica,sfo-edge=worker-c-replica".to_string()
            }),
        )?;
        let clock = HlcClock {
            node_id,
            timestamp: HlcTimestamp::new(initial_physical_ms, 0)
                .map_err(|error| error.to_string())?,
            max_offset_ms,
        };
        let runtime = HlcRuntime::new_with_edge_replicas(
            shard_group,
            clock,
            peers,
            replica_count,
            max_staleness_ms,
            edge_replicas,
        )
        .map_err(|error| error.to_string())?;
        Ok(Self {
            runtime: Mutex::new(runtime),
            probe: Mutex::new(SidecarRuntime::ready("hlc")),
        })
    }

    fn handle_http_bytes(&self, request: &[u8]) -> HttpProbeResponse {
        let parsed = parse_http_request(request);
        let Ok((method, path, body)) = parsed else {
            return error_response(400, "malformed HTTP request");
        };
        let (route, query) = split_route_query(path);

        match (method, route) {
            ("GET", "/closed_ts") => self.closed_ts_response(),
            ("POST", "/clock/tick") => self.tick_response(body),
            ("POST", "/clock/observe") => self.observe_response(body),
            ("GET", "/follower_read") => self.follower_read_response(query),
            ("GET", "/edge_read") => self.edge_read_response(query),
            (
                _,
                "/closed_ts" | "/clock/tick" | "/clock/observe" | "/follower_read" | "/edge_read",
            ) => error_response(405, "method not allowed"),
            _ => {
                let mut probe = self.probe.lock().expect("probe mutex");
                probe.handle_http_bytes(request).unwrap_or_else(|error| {
                    error_response(400, &format!("probe request failed: {error}"))
                })
            }
        }
    }

    fn closed_ts_response(&self) -> HttpProbeResponse {
        let runtime = self.runtime.lock().expect("hlc mutex");
        let report = runtime_report(&runtime);
        HttpProbeResponse::new(200, "application/json", render_closed_ts_json(&report))
    }

    fn tick_response(&self, body: &str) -> HttpProbeResponse {
        let params = parse_params(body);
        let physical_ms = match required_u64(&params, "physical_ms") {
            Ok(value) => value,
            Err(error) => return error_response(400, &error),
        };
        let mut runtime = self.runtime.lock().expect("hlc mutex");
        if let Err(error) = runtime.tick(physical_ms) {
            return error_response(409, &error.to_string());
        }
        HttpProbeResponse::new(
            200,
            "application/json",
            render_runtime_state_json("tick", &runtime),
        )
    }

    fn observe_response(&self, body: &str) -> HttpProbeResponse {
        let params = parse_params(body);
        let from = match required_param(&params, "from") {
            Ok(value) => value.to_string(),
            Err(error) => return error_response(400, &error),
        };
        let physical_ms = match required_u64(&params, "physical_ms") {
            Ok(value) => value,
            Err(error) => return error_response(400, &error),
        };
        let logical = match optional_u32(&params, "logical", 0) {
            Ok(value) => value,
            Err(error) => return error_response(400, &error),
        };
        let local_physical_ms = match required_u64(&params, "local_physical_ms") {
            Ok(value) => value,
            Err(error) => return error_response(400, &error),
        };
        let exchange = PeerClockExchange {
            from,
            timestamp: HlcTimestamp {
                physical_ms,
                logical,
            },
        };
        let mut runtime = self.runtime.lock().expect("hlc mutex");
        if let Err(error) = runtime.observe_peer(exchange, local_physical_ms) {
            return error_response(409, &error.to_string());
        }
        HttpProbeResponse::new(
            200,
            "application/json",
            render_runtime_state_json("observe", &runtime),
        )
    }

    fn follower_read_response(&self, query: Option<&str>) -> HttpProbeResponse {
        let params = parse_params(query.unwrap_or(""));
        let replica = match required_param(&params, "replica") {
            Ok(value) => value.to_string(),
            Err(error) => return error_response(400, &error),
        };
        let physical_ms = match required_u64(&params, "as_of_physical_ms") {
            Ok(value) => value,
            Err(error) => return error_response(400, &error),
        };
        let logical = match optional_u32(&params, "as_of_logical", 0) {
            Ok(value) => value,
            Err(error) => return error_response(400, &error),
        };
        let runtime = self.runtime.lock().expect("hlc mutex");
        match runtime.follower_read_decision(
            replica,
            HlcTimestamp {
                physical_ms,
                logical,
            },
        ) {
            Ok(decision @ FollowerReadDecision::ServeFromFollower { .. }) => {
                HttpProbeResponse::new(200, "application/json", render_decision_json(&decision))
            }
            Ok(decision @ FollowerReadDecision::RejectNotClosed { .. }) => {
                HttpProbeResponse::new(409, "application/json", render_decision_json(&decision))
            }
            Err(error) => error_response(400, &error.to_string()),
        }
    }

    fn edge_read_response(&self, query: Option<&str>) -> HttpProbeResponse {
        let params = parse_params(query.unwrap_or(""));
        let edge_region = match required_param(&params, "edge_region") {
            Ok(value) => value.to_string(),
            Err(error) => return error_response(400, &error),
        };
        let replica = match required_param(&params, "replica") {
            Ok(value) => value.to_string(),
            Err(error) => return error_response(400, &error),
        };
        let physical_ms = match required_u64(&params, "as_of_physical_ms") {
            Ok(value) => value,
            Err(error) => return error_response(400, &error),
        };
        let logical = match optional_u32(&params, "as_of_logical", 0) {
            Ok(value) => value,
            Err(error) => return error_response(400, &error),
        };
        let runtime = self.runtime.lock().expect("hlc mutex");
        match runtime.edge_read_decision(
            edge_region,
            replica,
            HlcTimestamp {
                physical_ms,
                logical,
            },
        ) {
            Ok(decision @ EdgeReadDecision::ServeFromEdge { .. }) => HttpProbeResponse::new(
                200,
                "application/json",
                render_edge_decision_json(&decision),
            ),
            Ok(decision) => HttpProbeResponse::new(
                409,
                "application/json",
                render_edge_decision_json(&decision),
            ),
            Err(HlcRuntimeError::UnknownEdgeRegion(error)) => {
                error_response(409, &format!("unknown edge region: {error}"))
            }
            Err(error) => error_response(400, &error.to_string()),
        }
    }
}

fn runtime_report(runtime: &HlcRuntime) -> HlcRuntimeReport {
    HlcRuntimeReport {
        shard_group: runtime.shard_group().to_string(),
        local_clock: runtime.clock().timestamp,
        closed_at: runtime.closed_timestamp(),
        max_offset_ms: runtime.clock().max_offset_ms,
        max_staleness_ms: runtime.max_staleness_ms(),
        replica_count: runtime.replica_count(),
        peers: runtime.peers().clone(),
    }
}

fn render_runtime_state_json(event: &str, runtime: &HlcRuntime) -> String {
    let report = runtime_report(runtime);
    format!(
        "{{\"event\":\"{}\",\"node_id\":\"{}\",\"shard_group\":\"{}\",\"local_clock\":{},\"closed_at\":{},\"max_offset_ms\":{},\"max_staleness_ms\":{},\"replica_count\":{},\"peer_count\":{}}}\n",
        escape_json(event),
        escape_json(&runtime.clock().node_id),
        escape_json(&report.shard_group),
        render_timestamp_json(report.local_clock),
        render_timestamp_json(report.closed_at),
        report.max_offset_ms,
        report.max_staleness_ms,
        report.replica_count,
        report.peers.len(),
    )
}

fn render_decision_json(decision: &FollowerReadDecision) -> String {
    match decision {
        FollowerReadDecision::ServeFromFollower {
            replica,
            as_of,
            closed_at,
        } => format!(
            "{{\"decision\":\"serve_from_follower\",\"serve_from_follower\":true,\"replica\":\"{}\",\"as_of\":{},\"closed_at\":{}}}\n",
            escape_json(replica),
            render_timestamp_json(*as_of),
            render_timestamp_json(*closed_at),
        ),
        FollowerReadDecision::RejectNotClosed {
            replica,
            as_of,
            closed_at,
        } => format!(
            "{{\"decision\":\"reject_not_closed\",\"serve_from_follower\":false,\"replica\":\"{}\",\"as_of\":{},\"closed_at\":{}}}\n",
            escape_json(replica),
            render_timestamp_json(*as_of),
            render_timestamp_json(*closed_at),
        ),
    }
}

fn render_edge_decision_json(decision: &EdgeReadDecision) -> String {
    match decision {
        EdgeReadDecision::ServeFromEdge {
            edge_region,
            replica,
            as_of,
            closed_at,
            max_staleness_ms,
            observed_staleness_ms,
        } => format!(
            "{{\"decision\":\"serve_from_edge\",\"serve_from_edge\":true,\"edge_region\":\"{}\",\"replica\":\"{}\",\"as_of\":{},\"closed_at\":{},\"max_staleness_ms\":{},\"observed_staleness_ms\":{}}}\n",
            escape_json(edge_region),
            escape_json(replica),
            render_timestamp_json(*as_of),
            render_timestamp_json(*closed_at),
            max_staleness_ms,
            observed_staleness_ms,
        ),
        EdgeReadDecision::RejectReplicaMismatch {
            edge_region,
            requested_replica,
            expected_replica,
        } => format!(
            "{{\"decision\":\"reject_replica_mismatch\",\"serve_from_edge\":false,\"edge_region\":\"{}\",\"requested_replica\":\"{}\",\"expected_replica\":\"{}\"}}\n",
            escape_json(edge_region),
            escape_json(requested_replica),
            escape_json(expected_replica),
        ),
        EdgeReadDecision::RejectNotClosed {
            edge_region,
            replica,
            as_of,
            closed_at,
        } => format!(
            "{{\"decision\":\"reject_not_closed\",\"serve_from_edge\":false,\"edge_region\":\"{}\",\"replica\":\"{}\",\"as_of\":{},\"closed_at\":{}}}\n",
            escape_json(edge_region),
            escape_json(replica),
            render_timestamp_json(*as_of),
            render_timestamp_json(*closed_at),
        ),
        EdgeReadDecision::RejectTooStale {
            edge_region,
            replica,
            as_of,
            closed_at,
            max_staleness_ms,
            observed_staleness_ms,
        } => format!(
            "{{\"decision\":\"reject_too_stale\",\"serve_from_edge\":false,\"edge_region\":\"{}\",\"replica\":\"{}\",\"as_of\":{},\"closed_at\":{},\"max_staleness_ms\":{},\"observed_staleness_ms\":{}}}\n",
            escape_json(edge_region),
            escape_json(replica),
            render_timestamp_json(*as_of),
            render_timestamp_json(*closed_at),
            max_staleness_ms,
            observed_staleness_ms,
        ),
    }
}

fn render_timestamp_json(timestamp: HlcTimestamp) -> String {
    format!(
        "{{\"physical_ms\":{},\"logical\":{}}}",
        timestamp.physical_ms, timestamp.logical
    )
}

fn read_http_request(stream: &mut TcpStream) -> Result<Vec<u8>, std::io::Error> {
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;
    let mut request = Vec::new();
    let mut chunk = [0_u8; 4096];

    loop {
        let read_len = stream.read(&mut chunk)?;
        if read_len == 0 {
            break;
        }
        request.extend_from_slice(&chunk[..read_len]);
        if let Some(header_len) = header_len(&request) {
            if let Some(content_length) = content_length(&request) {
                if request.len() >= header_len + content_length {
                    break;
                }
            } else {
                break;
            }
        }
        if request.len() >= 65_536 {
            break;
        }
    }
    Ok(request)
}

fn parse_http_request(request: &[u8]) -> Result<(&str, &str, &str), String> {
    let text = std::str::from_utf8(request).map_err(|_| "request must be utf-8".to_string())?;
    let (head, body) = text
        .split_once("\r\n\r\n")
        .or_else(|| text.split_once("\n\n"))
        .unwrap_or((text, ""));
    let request_line = head
        .lines()
        .next()
        .ok_or_else(|| "missing request line".to_string())?;
    let mut parts = request_line.split_whitespace();
    let method = parts.next().ok_or_else(|| "missing method".to_string())?;
    let path = parts.next().ok_or_else(|| "missing path".to_string())?;
    Ok((method, path, body))
}

fn split_route_query(path: &str) -> (&str, Option<&str>) {
    match path.split_once('?') {
        Some((route, query)) => (route, Some(query)),
        None => (path, None),
    }
}

fn parse_params(input: &str) -> BTreeMap<String, String> {
    input
        .split('&')
        .filter(|part| !part.trim().is_empty())
        .filter_map(|part| {
            let (key, value) = part.split_once('=')?;
            Some((key.trim().to_string(), value.trim().to_string()))
        })
        .collect()
}

fn required_param<'a>(params: &'a BTreeMap<String, String>, key: &str) -> Result<&'a str, String> {
    params
        .get(key)
        .map(String::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("{key} is required"))
}

fn required_u64(params: &BTreeMap<String, String>, key: &str) -> Result<u64, String> {
    required_param(params, key)?
        .parse::<u64>()
        .map_err(|_| format!("{key} must be an unsigned integer"))
}

fn optional_u32(params: &BTreeMap<String, String>, key: &str, default: u32) -> Result<u32, String> {
    match params.get(key) {
        Some(value) if !value.is_empty() => value
            .parse::<u32>()
            .map_err(|_| format!("{key} must be an unsigned integer")),
        _ => Ok(default),
    }
}

fn parse_csv(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn parse_edge_replicas(value: &str) -> Result<BTreeMap<String, String>, String> {
    let mut replicas = BTreeMap::new();
    for item in value
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
    {
        let (edge_region, replica) = item
            .split_once('=')
            .ok_or_else(|| format!("invalid edge replica mapping: {item}"))?;
        let edge_region = edge_region.trim();
        let replica = replica.trim();
        if edge_region.is_empty() || replica.is_empty() {
            return Err(format!("invalid edge replica mapping: {item}"));
        }
        replicas.insert(edge_region.to_string(), replica.to_string());
    }
    Ok(replicas)
}

fn parse_env_u64(key: &str, default: u64) -> Result<u64, String> {
    match env::var(key) {
        Ok(value) => value
            .parse::<u64>()
            .map_err(|_| format!("{key} must be an unsigned integer")),
        Err(_) => Ok(default),
    }
}

fn parse_env_u32(key: &str, default: u32) -> Result<u32, String> {
    match env::var(key) {
        Ok(value) => value
            .parse::<u32>()
            .map_err(|_| format!("{key} must be an unsigned integer")),
        Err(_) => Ok(default),
    }
}

fn content_length(request: &[u8]) -> Option<usize> {
    let text = std::str::from_utf8(request).ok()?;
    for line in text.lines() {
        if line.is_empty() {
            break;
        }
        if let Some((name, value)) = line.split_once(':') {
            if name.eq_ignore_ascii_case("content-length") {
                return value.trim().parse::<usize>().ok();
            }
        }
    }
    None
}

fn header_len(request: &[u8]) -> Option<usize> {
    if let Some(index) = request.windows(4).position(|window| window == b"\r\n\r\n") {
        return Some(index + 4);
    }
    request
        .windows(2)
        .position(|window| window == b"\n\n")
        .map(|index| index + 2)
}

fn error_response(status_code: u16, detail: &str) -> HttpProbeResponse {
    HttpProbeResponse::new(
        status_code,
        "application/json",
        format!("{{\"error\":\"{}\"}}\n", escape_json(detail)),
    )
}

fn escape_json(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn params_parse_query_shape() {
        let params = parse_params("replica=worker-a-replica&as_of_physical_ms=170&as_of_logical=2");
        assert_eq!(
            required_param(&params, "replica").unwrap(),
            "worker-a-replica"
        );
        assert_eq!(required_u64(&params, "as_of_physical_ms").unwrap(), 170);
        assert_eq!(optional_u32(&params, "as_of_logical", 0).unwrap(), 2);
    }

    #[test]
    fn decision_json_marks_rejects_fail_closed() {
        let decision = FollowerReadDecision::RejectNotClosed {
            replica: "worker-b-replica".to_string(),
            as_of: HlcTimestamp {
                physical_ms: 101,
                logical: 0,
            },
            closed_at: HlcTimestamp {
                physical_ms: 100,
                logical: 0,
            },
        };
        let json = render_decision_json(&decision);
        assert!(json.contains("\"decision\":\"reject_not_closed\""));
        assert!(json.contains("\"serve_from_follower\":false"));
    }

    #[test]
    fn edge_replicas_parse_csv_mapping() {
        let mappings = parse_edge_replicas("iad-edge=worker-a-replica,sfo-edge=worker-c-replica")
            .expect("mappings");

        assert_eq!(
            mappings.get("iad-edge"),
            Some(&"worker-a-replica".to_string())
        );
        assert_eq!(
            mappings.get("sfo-edge"),
            Some(&"worker-c-replica".to_string())
        );
    }

    #[test]
    fn edge_decision_json_marks_fail_closed_rejects() {
        let decision = EdgeReadDecision::RejectTooStale {
            edge_region: "iad-edge".to_string(),
            replica: "worker-a-replica".to_string(),
            as_of: HlcTimestamp {
                physical_ms: 90,
                logical: 0,
            },
            closed_at: HlcTimestamp {
                physical_ms: 100,
                logical: 0,
            },
            max_staleness_ms: 5,
            observed_staleness_ms: 10,
        };
        let json = render_edge_decision_json(&decision);
        assert!(json.contains("\"decision\":\"reject_too_stale\""));
        assert!(json.contains("\"serve_from_edge\":false"));
    }
}
