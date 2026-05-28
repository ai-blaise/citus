// FEATURE: S5
// FEATURE: S5

use ai_blaise_citus_sidecar_raft::{
    canonical_raft_report, canonical_raft_runtime_report, run_durable_log_snapshot_round_trip,
    AppendEntries, AppendResponse, FailoverDecision, LogIndex, NodeId, RaftDurableLogReport,
    RaftEntry, RaftMessage, RaftNode, RaftRole, RaftRoundTripReport, Term, VoteRequest,
    VoteResponse,
};
use ai_blaise_citus_sidecar_shared::{listen_addr_from_env, HttpProbeResponse, SidecarRuntime};
use std::collections::{BTreeMap, VecDeque};
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
        run_raft_server("0.0.0.0:8080");
        return;
    }

    if args == ["run-runtime-canonical"] {
        run_runtime_canonical();
        return;
    }

    if args == ["run-durable-canonical"] {
        run_durable_canonical();
        return;
    }

    if !args.is_empty() && args != ["run-canonical"] {
        eprintln!("raft: unknown command");
        print_usage();
        process::exit(2);
    }

    let report = canonical_raft_report().unwrap_or_else(|error| {
        eprintln!("raft: canonical report failed: {error}");
        process::exit(1);
    });
    let intent = &report.plan.placement_intents[0];
    let (decision, decision_node, decision_pod) = decision_fields(&report.decision);

    println!(
        "shard_group\tterm\tleader\tquorum_size\tlive_nodes\tlease_holder\tlease_expires_physical_ms\tobserved_physical_ms\tintent_shard_id\tintent_target_node\tintent_generation\tdecision\tdecision_node\tdecision_pod"
    );
    println!(
        "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
        report.plan.shard_group,
        report.plan.term,
        report.plan.leader.as_deref().unwrap_or("none"),
        report.plan.quorum_size(),
        report.live_nodes.join(","),
        report.plan.lease.holder,
        report.plan.lease.expires_at.physical_ms,
        report.observed_at.physical_ms,
        intent.shard_id,
        intent.target_node,
        intent.placement_generation,
        decision,
        decision_node,
        decision_pod,
    );
}

fn run_durable_canonical() {
    let dir = env::var("AI_BLAISE_RAFT_DURABLE_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| {
            std::env::temp_dir().join(format!("ai-blaise-raft-durable-{}", std::process::id()))
        });
    let _ = std::fs::remove_dir_all(&dir);
    let report = run_durable_log_snapshot_round_trip(&dir).unwrap_or_else(|error| {
        eprintln!("raft: durable canonical report failed: {error}");
        process::exit(1);
    });
    emit_durable_report(&report);
}

fn emit_durable_report(report: &RaftDurableLogReport) {
    println!(
        "appended_entries\treplayed_entries\tsnapshot_index\tsnapshot_term\tlog_path\tsnapshot_path"
    );
    println!(
        "{}\t{}\t{}\t{}\t{}\t{}",
        report.appended_entries,
        report.replayed_entries,
        report.snapshot_index,
        report.snapshot_term,
        report.log_path,
        report.snapshot_path,
    );
}

fn run_runtime_canonical() {
    let report = canonical_raft_runtime_report().unwrap_or_else(|error| {
        eprintln!("raft: canonical runtime report failed: {error}");
        process::exit(1);
    });
    emit_runtime_report(&report);
}

fn emit_runtime_report(report: &RaftRoundTripReport) {
    println!(
        "elected_leader\tterm\tcommitted_index\tcommitted_payload\tcommit_indices\tlast_log_indices"
    );
    let payload = std::str::from_utf8(&report.committed_payload).unwrap_or("<binary>");
    println!(
        "{}\t{}\t{}\t{}\t{}\t{}",
        report.elected_leader,
        report.term,
        report.committed_index,
        payload,
        format_node_indices(&report.commit_indices),
        format_node_indices(&report.last_log_indices),
    );
}

fn format_node_indices(map: &std::collections::BTreeMap<String, u64>) -> String {
    map.iter()
        .map(|(id, value)| format!("{id}={value}"))
        .collect::<Vec<_>>()
        .join(",")
}

fn print_usage() {
    println!("usage: raft [serve|run-canonical|run-runtime-canonical|run-durable-canonical]");
    println!(
        "runs the deterministic canonical Raft sidecar plan, 3-node runtime, durable proof, or HTTP transport server"
    );
}

fn run_raft_server(default_addr: &str) {
    let listen_addr = match listen_addr_from_env(default_addr) {
        Ok(addr) => addr,
        Err(error) => {
            eprintln!("raft: invalid listen address: {error}");
            process::exit(1);
        }
    };
    let server = match RaftHttpServer::from_env() {
        Ok(server) => server,
        Err(error) => {
            eprintln!("raft: runtime init failed: {error}");
            process::exit(1);
        }
    };
    let listener = match TcpListener::bind(&listen_addr) {
        Ok(listener) => listener,
        Err(error) => {
            eprintln!("raft: bind failed: {error}");
            process::exit(1);
        }
    };
    eprintln!(
        "ai-blaise raft HTTP server node={} listening on {listen_addr}",
        server.node_id
    );

    for stream in listener.incoming() {
        let mut stream = match stream {
            Ok(stream) => stream,
            Err(error) => {
                eprintln!("raft: accept failed: {error}");
                continue;
            }
        };
        let request = match read_http_request(&mut stream) {
            Ok(request) => request,
            Err(error) => {
                eprintln!("raft: read failed: {error}");
                continue;
            }
        };
        let response = server.handle_http_bytes(&request);
        if let Err(error) = stream.write_all(response.to_http_string().as_bytes()) {
            eprintln!("raft: write failed: {error}");
        }
    }
}

struct RaftHttpServer {
    node_id: NodeId,
    node: Mutex<RaftNode>,
    peers: BTreeMap<NodeId, String>,
    probe: Mutex<SidecarRuntime>,
}

impl RaftHttpServer {
    fn from_env() -> Result<Self, String> {
        let node_id = env::var("AI_BLAISE_RAFT_NODE_ID").unwrap_or_else(|_| "worker-a".to_string());
        let members = parse_members(
            &env::var("AI_BLAISE_RAFT_MEMBERS")
                .unwrap_or_else(|_| "worker-a,worker-b,worker-c".to_string()),
        )?;
        let peers = parse_peers(&env::var("AI_BLAISE_RAFT_PEERS").unwrap_or_default())?;
        let node = RaftNode::new(node_id.clone(), members).map_err(|error| error.to_string())?;
        Ok(Self {
            node_id,
            node: Mutex::new(node),
            peers,
            probe: Mutex::new(SidecarRuntime::ready("raft")),
        })
    }

    fn handle_http_bytes(&self, request: &[u8]) -> HttpProbeResponse {
        let parsed = parse_http_request(request);
        let Ok((method, path, body)) = parsed else {
            return error_response(400, "malformed HTTP request");
        };

        match (method, path) {
            ("POST", "/raft/campaign") => self.handle_campaign(),
            ("POST", "/raft/propose") => self.handle_propose(body),
            ("POST", "/raft/message") => self.handle_message(body),
            ("GET", "/raft/status") => self.status_response(200, None),
            (_, "/raft/campaign" | "/raft/propose" | "/raft/message" | "/raft/status") => {
                error_response(405, "method not allowed")
            }
            _ => {
                let mut probe = self.probe.lock().expect("probe mutex");
                probe.handle_http_bytes(request).unwrap_or_else(|error| {
                    error_response(400, &format!("probe request failed: {error}"))
                })
            }
        }
    }

    fn handle_campaign(&self) -> HttpProbeResponse {
        let messages = {
            let mut node = self.node.lock().expect("raft mutex");
            node.become_candidate()
        };
        if let Err(error) = self.route_messages(self.node_id.clone(), messages) {
            return error_response(502, &error);
        }
        let role = self.node.lock().expect("raft mutex").role();
        if role != RaftRole::Leader {
            return error_response(409, "campaign did not reach quorum");
        }
        self.status_response(200, Some("campaigned"))
    }

    fn handle_propose(&self, body: &str) -> HttpProbeResponse {
        let payload = body.trim();
        if payload.is_empty() {
            return error_response(400, "proposal payload is required");
        }
        let messages = {
            let mut node = self.node.lock().expect("raft mutex");
            if let Err(error) = node.propose(payload.as_bytes().to_vec()) {
                return error_response(409, &error.to_string());
            }
            node.build_append_entries()
        };
        if let Err(error) = self.route_messages(self.node_id.clone(), messages) {
            return error_response(502, &error);
        }
        let heartbeat_messages = {
            let node = self.node.lock().expect("raft mutex");
            node.tick_heartbeat()
        };
        if let Err(error) = self.route_messages(self.node_id.clone(), heartbeat_messages) {
            return error_response(502, &error);
        }
        self.status_response(201, Some("proposed"))
    }

    fn handle_message(&self, body: &str) -> HttpProbeResponse {
        let (from, message) = match parse_inbound_message(body) {
            Ok(message) => message,
            Err(error) => return error_response(400, &error),
        };
        let replies = {
            let mut node = self.node.lock().expect("raft mutex");
            node.step(&from, message)
        };
        HttpProbeResponse::new(200, "text/plain", serialize_routed_messages(&replies))
    }

    fn route_messages(
        &self,
        initial_from: NodeId,
        messages: Vec<(NodeId, RaftMessage)>,
    ) -> Result<(), String> {
        let mut queue = VecDeque::new();
        for (to, message) in messages {
            queue.push_back((initial_from.clone(), to, message));
        }

        while let Some((from, to, message)) = queue.pop_front() {
            if to == self.node_id {
                let replies = {
                    let mut node = self.node.lock().expect("raft mutex");
                    node.step(&from, message)
                };
                for (reply_to, reply) in replies {
                    queue.push_back((self.node_id.clone(), reply_to, reply));
                }
                continue;
            }

            let replies = self.send_message(&to, &from, &message)?;
            for (reply_to, reply) in replies {
                queue.push_back((to.clone(), reply_to, reply));
            }
        }
        Ok(())
    }

    fn send_message(
        &self,
        to: &str,
        from: &str,
        message: &RaftMessage,
    ) -> Result<Vec<(NodeId, RaftMessage)>, String> {
        let addr = self
            .peers
            .get(to)
            .ok_or_else(|| format!("missing peer address for {to}"))?;
        let body = format!("from\t{from}\n{}\n", serialize_message(message));
        let response = http_post(addr, "/raft/message", &body)?;
        let (status, response_body) = split_http_response(&response)?;
        if status != 200 {
            return Err(format!("peer {to} returned HTTP {status}: {response_body}"));
        }
        parse_routed_messages(response_body)
    }

    fn status_response(&self, status_code: u16, event: Option<&str>) -> HttpProbeResponse {
        let node = self.node.lock().expect("raft mutex");
        let committed_payload = node
            .committed_entry(node.commit_index())
            .map(|entry| String::from_utf8_lossy(&entry.payload).into_owned());
        let body = format!(
            "{{\"node_id\":\"{}\",\"event\":{},\"role\":\"{}\",\"term\":{},\"leader_id\":{},\"commit_index\":{},\"last_log_index\":{},\"committed_payload\":{}}}\n",
            escape_json(&self.node_id),
            json_optional(event),
            role_name(node.role()),
            node.current_term(),
            json_optional(node.leader_id()),
            node.commit_index(),
            node.last_log_index(),
            json_optional(committed_payload.as_deref()),
        );
        HttpProbeResponse::new(status_code, "application/json", body)
    }
}

fn parse_members(value: &str) -> Result<Vec<NodeId>, String> {
    let members = value
        .split(',')
        .map(str::trim)
        .filter(|member| !member.is_empty())
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    if members.is_empty() {
        return Err("AI_BLAISE_RAFT_MEMBERS must contain at least one node".to_string());
    }
    Ok(members)
}

fn parse_peers(value: &str) -> Result<BTreeMap<NodeId, String>, String> {
    let mut peers = BTreeMap::new();
    for item in value
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
    {
        let (node, addr) = item
            .split_once('=')
            .ok_or_else(|| format!("invalid peer entry {item:?}; expected node=host:port"))?;
        if node.trim().is_empty() || addr.trim().is_empty() {
            return Err(format!(
                "invalid peer entry {item:?}; node and address are required"
            ));
        }
        peers.insert(node.trim().to_string(), addr.trim().to_string());
    }
    Ok(peers)
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

fn http_post(addr: &str, path: &str, body: &str) -> Result<String, String> {
    let mut stream =
        TcpStream::connect(addr).map_err(|error| format!("connect {addr}: {error}"))?;
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .map_err(|error| format!("set read timeout: {error}"))?;
    stream
        .set_write_timeout(Some(Duration::from_secs(5)))
        .map_err(|error| format!("set write timeout: {error}"))?;
    let request = format!(
        "POST {path} HTTP/1.1\r\nhost: {addr}\r\ncontent-type: text/plain\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
        body.len()
    );
    stream
        .write_all(request.as_bytes())
        .map_err(|error| format!("write {addr}: {error}"))?;
    let mut response = String::new();
    stream
        .read_to_string(&mut response)
        .map_err(|error| format!("read {addr}: {error}"))?;
    Ok(response)
}

fn split_http_response(response: &str) -> Result<(u16, &str), String> {
    let status_line = response
        .lines()
        .next()
        .ok_or_else(|| "missing HTTP response status".to_string())?;
    let status = status_line
        .split_whitespace()
        .nth(1)
        .ok_or_else(|| format!("malformed HTTP response status: {status_line}"))?
        .parse::<u16>()
        .map_err(|_| format!("invalid HTTP status in response: {status_line}"))?;
    let body = response
        .split_once("\r\n\r\n")
        .or_else(|| response.split_once("\n\n"))
        .map(|(_, body)| body)
        .unwrap_or("");
    Ok((status, body))
}

fn parse_inbound_message(body: &str) -> Result<(NodeId, RaftMessage), String> {
    let mut lines = body.lines().filter(|line| !line.trim().is_empty());
    let from_line = lines
        .next()
        .ok_or_else(|| "missing from line".to_string())?;
    let Some(("from", from)) = from_line.split_once('\t') else {
        return Err("first message line must be from<TAB>node".to_string());
    };
    if from.trim().is_empty() {
        return Err("from node is required".to_string());
    }
    let message_line = lines
        .next()
        .ok_or_else(|| "missing raft message line".to_string())?;
    Ok((from.trim().to_string(), parse_message(message_line)?))
}

fn serialize_routed_messages(messages: &[(NodeId, RaftMessage)]) -> String {
    messages
        .iter()
        .map(|(to, message)| format!("to\t{to}\t{}", serialize_message(message)))
        .collect::<Vec<_>>()
        .join("\n")
        + "\n"
}

fn parse_routed_messages(body: &str) -> Result<Vec<(NodeId, RaftMessage)>, String> {
    let mut messages = Vec::new();
    for line in body.lines().filter(|line| !line.trim().is_empty()) {
        let mut parts = line.splitn(3, '\t');
        match (parts.next(), parts.next(), parts.next()) {
            (Some("to"), Some(to), Some(message)) if !to.trim().is_empty() => {
                messages.push((to.trim().to_string(), parse_message(message)?));
            }
            _ => return Err(format!("malformed routed raft message line: {line}")),
        }
    }
    Ok(messages)
}

fn serialize_message(message: &RaftMessage) -> String {
    match message {
        RaftMessage::VoteRequest(request) => format!(
            "vote_request\t{}\t{}\t{}\t{}",
            request.term, request.candidate_id, request.last_log_index, request.last_log_term
        ),
        RaftMessage::VoteResponse(response) => format!(
            "vote_response\t{}\t{}\t{}",
            response.term, response.from, response.granted
        ),
        RaftMessage::AppendEntries(append) => format!(
            "append_entries\t{}\t{}\t{}\t{}\t{}\t{}",
            append.term,
            append.leader_id,
            append.prev_log_index,
            append.prev_log_term,
            append.leader_commit,
            serialize_entries(&append.entries)
        ),
        RaftMessage::AppendResponse(response) => format!(
            "append_response\t{}\t{}\t{}\t{}",
            response.term, response.from, response.success, response.match_index
        ),
    }
}

fn parse_message(line: &str) -> Result<RaftMessage, String> {
    let parts = line.split('\t').collect::<Vec<_>>();
    match parts.as_slice() {
        ["vote_request", term, candidate_id, last_log_index, last_log_term] => {
            Ok(RaftMessage::VoteRequest(VoteRequest {
                term: parse_term(term, "term")?,
                candidate_id: parse_node_id(candidate_id, "candidate_id")?,
                last_log_index: parse_log_index(last_log_index, "last_log_index")?,
                last_log_term: parse_term(last_log_term, "last_log_term")?,
            }))
        }
        ["vote_response", term, from, granted] => Ok(RaftMessage::VoteResponse(VoteResponse {
            term: parse_term(term, "term")?,
            from: parse_node_id(from, "from")?,
            granted: parse_bool(granted, "granted")?,
        })),
        ["append_entries", term, leader_id, prev_log_index, prev_log_term, leader_commit, entries] => {
            Ok(RaftMessage::AppendEntries(AppendEntries {
                term: parse_term(term, "term")?,
                leader_id: parse_node_id(leader_id, "leader_id")?,
                prev_log_index: parse_log_index(prev_log_index, "prev_log_index")?,
                prev_log_term: parse_term(prev_log_term, "prev_log_term")?,
                leader_commit: parse_log_index(leader_commit, "leader_commit")?,
                entries: parse_entries(entries)?,
            }))
        }
        ["append_response", term, from, success, match_index] => {
            Ok(RaftMessage::AppendResponse(AppendResponse {
                term: parse_term(term, "term")?,
                from: parse_node_id(from, "from")?,
                success: parse_bool(success, "success")?,
                match_index: parse_log_index(match_index, "match_index")?,
            }))
        }
        _ => Err(format!("unknown or malformed raft message: {line}")),
    }
}

fn serialize_entries(entries: &[RaftEntry]) -> String {
    entries
        .iter()
        .map(|entry| {
            format!(
                "{}:{}:{}",
                entry.index,
                entry.term,
                encode_hex(&entry.payload)
            )
        })
        .collect::<Vec<_>>()
        .join(";")
}

fn parse_entries(value: &str) -> Result<Vec<RaftEntry>, String> {
    if value.is_empty() {
        return Ok(Vec::new());
    }
    value
        .split(';')
        .map(|entry| {
            let parts = entry.split(':').collect::<Vec<_>>();
            match parts.as_slice() {
                [index, term, payload] => Ok(RaftEntry {
                    index: parse_log_index(index, "entry.index")?,
                    term: parse_term(term, "entry.term")?,
                    payload: decode_hex(payload)?,
                }),
                _ => Err(format!("malformed raft entry: {entry}")),
            }
        })
        .collect()
}

fn parse_node_id(value: &str, field: &str) -> Result<NodeId, String> {
    let value = value.trim();
    if value.is_empty() {
        return Err(format!("{field} is required"));
    }
    Ok(value.to_string())
}

fn parse_term(value: &str, field: &str) -> Result<Term, String> {
    value
        .parse::<Term>()
        .map_err(|_| format!("{field} must be an unsigned integer"))
}

fn parse_log_index(value: &str, field: &str) -> Result<LogIndex, String> {
    value
        .parse::<LogIndex>()
        .map_err(|_| format!("{field} must be an unsigned integer"))
}

fn parse_bool(value: &str, field: &str) -> Result<bool, String> {
    match value {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(format!("{field} must be true or false")),
    }
}

fn encode_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        encoded.push(HEX[(byte >> 4) as usize] as char);
        encoded.push(HEX[(byte & 0x0f) as usize] as char);
    }
    encoded
}

fn decode_hex(value: &str) -> Result<Vec<u8>, String> {
    if !value.len().is_multiple_of(2) {
        return Err("hex payload has odd length".to_string());
    }
    let mut decoded = Vec::with_capacity(value.len() / 2);
    let bytes = value.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        let high = decode_hex_nibble(bytes[index])?;
        let low = decode_hex_nibble(bytes[index + 1])?;
        decoded.push((high << 4) | low);
        index += 2;
    }
    Ok(decoded)
}

fn decode_hex_nibble(byte: u8) -> Result<u8, String> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        b'A'..=b'F' => Ok(byte - b'A' + 10),
        _ => Err("hex payload contains non-hex byte".to_string()),
    }
}

fn role_name(role: RaftRole) -> &'static str {
    match role {
        RaftRole::Follower => "follower",
        RaftRole::Candidate => "candidate",
        RaftRole::Leader => "leader",
    }
}

fn json_optional(value: Option<&str>) -> String {
    match value {
        Some(value) => format!("\"{}\"", escape_json(value)),
        None => "null".to_string(),
    }
}

fn escape_json(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

fn error_response(status_code: u16, detail: &str) -> HttpProbeResponse {
    HttpProbeResponse::new(
        status_code,
        "application/json",
        format!("{{\"error\":\"{}\"}}\n", escape_json(detail)),
    )
}

fn decision_fields(decision: &FailoverDecision) -> (&'static str, &str, &str) {
    match decision {
        FailoverDecision::KeepLeader { node_id } => ("keep_leader", node_id.as_str(), "none"),
        FailoverDecision::Promote { node_id, cnpg_pod } => {
            ("promote", node_id.as_str(), cnpg_pod.as_str())
        }
        FailoverDecision::WaitForQuorum => ("wait_for_quorum", "none", "none"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn append_entries_wire_round_trips() {
        let message = RaftMessage::AppendEntries(AppendEntries {
            term: 7,
            leader_id: "worker-a".to_string(),
            prev_log_index: 11,
            prev_log_term: 6,
            leader_commit: 12,
            entries: vec![RaftEntry {
                term: 7,
                index: 12,
                payload: b"placement-intent".to_vec(),
            }],
        });

        assert_eq!(
            parse_message(&serialize_message(&message)).unwrap(),
            message
        );
    }

    #[test]
    fn routed_vote_response_wire_round_trips() {
        let body = serialize_routed_messages(&[(
            "worker-a".to_string(),
            RaftMessage::VoteResponse(VoteResponse {
                term: 3,
                from: "worker-b".to_string(),
                granted: true,
            }),
        )]);

        let messages = parse_routed_messages(&body).unwrap();
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].0, "worker-a");
    }
}
