// FEATURE: O4
// FEATURE: O14
// FEATURE: Sec13
// FEATURE: T3
// FEATURE: T7

use crate::trace_tap;
use std::error::Error;
use std::fmt;
use std::io::{self, Read, Write};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, Shutdown, TcpListener, TcpStream, ToSocketAddrs};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, SystemTime};

const DEFAULT_PROXY_LISTEN_ADDR: &str = "0.0.0.0:5432";
const DEFAULT_ADMIN_LISTEN_ADDR: &str = "0.0.0.0:8080";
const CONNECT_TIMEOUT: Duration = Duration::from_millis(750);

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PoolProxyConfig {
    pub listen_addr: String,
    pub admin_addr: String,
    pub upstream_addr: String,
    pub client_cidr_allowlist: ClientCidrAllowlist,
}

impl PoolProxyConfig {
    pub fn from_env() -> Result<Self, PoolProxyError> {
        let listen_addr = env_or_default("AI_BLAISE_POOL_LISTEN_ADDR", DEFAULT_PROXY_LISTEN_ADDR);
        let admin_addr = std::env::var("AI_BLAISE_POOL_ADMIN_ADDR")
            .or_else(|_| std::env::var("AI_BLAISE_LISTEN_ADDR"))
            .unwrap_or_else(|_| DEFAULT_ADMIN_LISTEN_ADDR.to_string());
        let upstream_addr = std::env::var("AI_BLAISE_POOL_UPSTREAM_ADDR")
            .map_err(|_| PoolProxyError::MissingEnv("AI_BLAISE_POOL_UPSTREAM_ADDR"))?;
        let client_cidr_allowlist = ClientCidrAllowlist::parse_csv(&env_or_default(
            "AI_BLAISE_POOL_CLIENT_CIDR_ALLOWLIST",
            "",
        ))?;

        let config = Self {
            listen_addr,
            admin_addr,
            upstream_addr,
            client_cidr_allowlist,
        };
        config.validate()?;
        Ok(config)
    }

    pub fn validate(&self) -> Result<(), PoolProxyError> {
        validate_addr("listen_addr", &self.listen_addr)?;
        validate_addr("admin_addr", &self.admin_addr)?;
        validate_addr("upstream_addr", &self.upstream_addr)?;
        if self.listen_addr == self.admin_addr {
            return Err(PoolProxyError::AddressCollision {
                left: "listen_addr",
                right: "admin_addr",
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ClientCidrAllowlist {
    entries: Vec<CidrBlock>,
}

impl ClientCidrAllowlist {
    pub fn parse_csv(value: &str) -> Result<Self, PoolProxyError> {
        let entries = value
            .split(',')
            .map(str::trim)
            .filter(|entry| !entry.is_empty())
            .map(CidrBlock::parse)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self { entries })
    }

    pub fn allows(&self, ip: IpAddr) -> bool {
        self.entries.is_empty() || self.entries.iter().any(|entry| entry.contains(ip))
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn as_csv(&self) -> String {
        self.entries
            .iter()
            .map(|entry| entry.source.as_str())
            .collect::<Vec<_>>()
            .join(",")
    }
}

impl Default for ClientCidrAllowlist {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct CidrBlock {
    network: IpAddr,
    prefix: u8,
    source: String,
}

impl CidrBlock {
    fn parse(value: &str) -> Result<Self, PoolProxyError> {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err(PoolProxyError::InvalidCidr(value.to_string()));
        }

        let (ip_text, prefix) = if let Some((ip_text, prefix_text)) = trimmed.split_once('/') {
            let ip = ip_text
                .parse::<IpAddr>()
                .map_err(|_| PoolProxyError::InvalidCidr(trimmed.to_string()))?;
            let prefix = prefix_text
                .parse::<u8>()
                .map_err(|_| PoolProxyError::InvalidCidr(trimmed.to_string()))?;
            (ip, prefix)
        } else {
            let ip = trimmed
                .parse::<IpAddr>()
                .map_err(|_| PoolProxyError::InvalidCidr(trimmed.to_string()))?;
            let prefix = match ip {
                IpAddr::V4(_) => 32,
                IpAddr::V6(_) => 128,
            };
            (ip, prefix)
        };

        let max_prefix = match ip_text {
            IpAddr::V4(_) => 32,
            IpAddr::V6(_) => 128,
        };
        if prefix > max_prefix {
            return Err(PoolProxyError::InvalidCidr(trimmed.to_string()));
        }

        Ok(Self {
            network: ip_text,
            prefix,
            source: trimmed.to_string(),
        })
    }

    fn contains(&self, candidate: IpAddr) -> bool {
        match (self.network, candidate) {
            (IpAddr::V4(network), IpAddr::V4(candidate)) => {
                cidr_contains_v4(network, candidate, self.prefix)
            }
            (IpAddr::V6(network), IpAddr::V6(candidate)) => {
                cidr_contains_v6(network, candidate, self.prefix)
            }
            _ => false,
        }
    }
}

fn cidr_contains_v4(network: Ipv4Addr, candidate: Ipv4Addr, prefix: u8) -> bool {
    let network = u32::from(network);
    let candidate = u32::from(candidate);
    let mask = if prefix == 0 {
        0
    } else {
        u32::MAX << (32 - prefix)
    };
    (network & mask) == (candidate & mask)
}

fn cidr_contains_v6(network: Ipv6Addr, candidate: Ipv6Addr, prefix: u8) -> bool {
    let network = u128::from(network);
    let candidate = u128::from(candidate);
    let mask = if prefix == 0 {
        0
    } else {
        u128::MAX << (128 - prefix)
    };
    (network & mask) == (candidate & mask)
}

#[derive(Debug)]
pub struct PoolProxyState {
    started_at: SystemTime,
    active_connections: AtomicU64,
    accepted_connections: AtomicU64,
    completed_connections: AtomicU64,
    rejected_connections: AtomicU64,
    upstream_connect_errors: AtomicU64,
    io_errors: AtomicU64,
    client_to_upstream_bytes: AtomicU64,
    upstream_to_client_bytes: AtomicU64,
    traceparent_tapped: AtomicU64,
    traceparent_absent: AtomicU64,
}

impl PoolProxyState {
    pub fn new() -> Self {
        Self {
            started_at: SystemTime::now(),
            active_connections: AtomicU64::new(0),
            accepted_connections: AtomicU64::new(0),
            completed_connections: AtomicU64::new(0),
            rejected_connections: AtomicU64::new(0),
            upstream_connect_errors: AtomicU64::new(0),
            io_errors: AtomicU64::new(0),
            client_to_upstream_bytes: AtomicU64::new(0),
            upstream_to_client_bytes: AtomicU64::new(0),
            traceparent_tapped: AtomicU64::new(0),
            traceparent_absent: AtomicU64::new(0),
        }
    }

    pub fn active_connections(&self) -> u64 {
        self.active_connections.load(Ordering::Relaxed)
    }

    fn accepted(&self) {
        self.active_connections.fetch_add(1, Ordering::Relaxed);
        self.accepted_connections.fetch_add(1, Ordering::Relaxed);
    }

    fn completed(&self) {
        self.active_connections.fetch_sub(1, Ordering::Relaxed);
        self.completed_connections.fetch_add(1, Ordering::Relaxed);
    }

    fn rejected(&self) {
        self.rejected_connections.fetch_add(1, Ordering::Relaxed);
    }

    fn connect_error(&self) {
        self.upstream_connect_errors.fetch_add(1, Ordering::Relaxed);
    }

    fn io_error(&self) {
        self.io_errors.fetch_add(1, Ordering::Relaxed);
    }

    fn add_client_bytes(&self, bytes: u64) {
        self.client_to_upstream_bytes
            .fetch_add(bytes, Ordering::Relaxed);
    }

    fn add_upstream_bytes(&self, bytes: u64) {
        self.upstream_to_client_bytes
            .fetch_add(bytes, Ordering::Relaxed);
    }

    fn traceparent_tapped(&self) {
        self.traceparent_tapped.fetch_add(1, Ordering::Relaxed);
    }

    fn traceparent_absent(&self) {
        self.traceparent_absent.fetch_add(1, Ordering::Relaxed);
    }

    pub fn traceparent_tapped_count(&self) -> u64 {
        self.traceparent_tapped.load(Ordering::Relaxed)
    }

    pub fn traceparent_absent_count(&self) -> u64 {
        self.traceparent_absent.load(Ordering::Relaxed)
    }

    fn uptime_seconds(&self) -> u64 {
        self.started_at
            .elapsed()
            .unwrap_or(Duration::ZERO)
            .as_secs()
    }
}

impl Default for PoolProxyState {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct AdminRequest {
    pub method: String,
    pub path: String,
}

impl AdminRequest {
    pub fn parse(bytes: &[u8]) -> Result<Self, PoolProxyError> {
        let request = std::str::from_utf8(bytes).map_err(|_| PoolProxyError::MalformedHttp)?;
        let request_line = request
            .lines()
            .next()
            .ok_or(PoolProxyError::MalformedHttp)?;
        let mut parts = request_line.split_whitespace();
        let method = parts.next().ok_or(PoolProxyError::MalformedHttp)?;
        let path = parts.next().ok_or(PoolProxyError::MalformedHttp)?;
        if !path.starts_with('/') {
            return Err(PoolProxyError::MalformedHttp);
        }
        Ok(Self {
            method: method.to_string(),
            path: path.to_string(),
        })
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct AdminResponse {
    pub status_code: u16,
    pub content_type: &'static str,
    pub body: String,
}

impl AdminResponse {
    fn to_http_string(&self) -> String {
        format!(
            "HTTP/1.1 {} {}\r\ncontent-type: {}\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
            self.status_code,
            status_reason(self.status_code),
            self.content_type,
            self.body.len(),
            self.body,
        )
    }
}

pub fn run_pool_service_from_env() -> Result<(), PoolProxyError> {
    run_pool_service(PoolProxyConfig::from_env()?)
}

pub fn run_pool_service(config: PoolProxyConfig) -> Result<(), PoolProxyError> {
    config.validate()?;
    let proxy_listener = TcpListener::bind(&config.listen_addr)?;
    let admin_listener = TcpListener::bind(&config.admin_addr)?;
    let state = Arc::new(PoolProxyState::new());

    let admin_state = Arc::clone(&state);
    let admin_upstream = config.upstream_addr.clone();
    thread::spawn(move || {
        if let Err(error) = serve_admin(admin_listener, admin_state, admin_upstream) {
            eprintln!("pool admin server stopped: {error}");
        }
    });

    eprintln!(
        "ai-blaise pool proxy listening on {} and forwarding PostgreSQL traffic to {} with client CIDR allowlist {}",
        config.listen_addr,
        config.upstream_addr,
        if config.client_cidr_allowlist.is_empty() {
            "<allow-all>".to_string()
        } else {
            config.client_cidr_allowlist.as_csv()
        }
    );
    for client in proxy_listener.incoming() {
        let client = client?;
        let upstream_addr = config.upstream_addr.clone();
        let client_cidr_allowlist = config.client_cidr_allowlist.clone();
        let state = Arc::clone(&state);
        thread::spawn(move || {
            if let Err(error) =
                handle_proxy_connection(client, &upstream_addr, &client_cidr_allowlist, &state)
            {
                eprintln!("pool connection failed: {error}");
            }
        });
    }

    Ok(())
}

pub fn handle_proxy_connection(
    client: TcpStream,
    upstream_addr: &str,
    client_cidr_allowlist: &ClientCidrAllowlist,
    state: &PoolProxyState,
) -> Result<(), PoolProxyError> {
    let client_ip = client.peer_addr()?.ip();
    if !client_cidr_allowlist.allows(client_ip) {
        state.rejected();
        return Err(PoolProxyError::ClientRejected {
            client_ip: client_ip.to_string(),
            allowlist: client_cidr_allowlist.as_csv(),
        });
    }

    state.accepted();
    let result = proxy_connection(client, upstream_addr, state);
    state.completed();
    result
}

fn proxy_connection(
    client: TcpStream,
    upstream_addr: &str,
    state: &PoolProxyState,
) -> Result<(), PoolProxyError> {
    let upstream = connect_upstream(upstream_addr).inspect_err(|_error| {
        state.connect_error();
    })?;

    client.set_nodelay(true)?;
    upstream.set_nodelay(true)?;

    // Peek the PostgreSQL startup envelope so the trace_tap can extract a W3C
    // traceparent embedded in the application_name parameter without
    // modifying the byte stream. The buffered bytes are replayed to upstream
    // ahead of the io::copy fan-out below so the proxy stays transparent to
    // PostgreSQL.
    let mut startup_reader = client.try_clone()?;
    let prefix_bytes = match trace_tap::tap_startup_message(&mut startup_reader) {
        Ok(tap) => {
            if tap.traceparent().is_some() {
                state.traceparent_tapped();
            } else {
                state.traceparent_absent();
            }
            eprintln!("ai-blaise pool {}", trace_tap::render_tap_log(&tap));
            tap.buffered_bytes
        }
        Err(error) => {
            state.io_error();
            return Err(PoolProxyError::from(error));
        }
    };

    thread::scope(|scope| {
        let mut client_reader = client.try_clone()?;
        let mut upstream_writer = upstream.try_clone()?;
        let prefix_bytes = prefix_bytes;
        let upload = scope.spawn(move || -> io::Result<u64> {
            upstream_writer.write_all(&prefix_bytes)?;
            let prefix_len = prefix_bytes.len() as u64;
            let copied = copy_and_shutdown(&mut client_reader, &mut upstream_writer)?;
            Ok(prefix_len + copied)
        });

        let mut upstream_reader = upstream;
        let mut client_writer = client;
        let downstream_result = copy_and_shutdown(&mut upstream_reader, &mut client_writer);
        let upload_result = upload.join().map_err(|_| PoolProxyError::WorkerPanicked)?;

        let mut first_error = None;

        match upload_result {
            Ok(bytes) => state.add_client_bytes(bytes),
            Err(error) => {
                state.io_error();
                first_error = Some(PoolProxyError::from(error));
            }
        }
        match downstream_result {
            Ok(bytes) => {
                state.add_upstream_bytes(bytes);
            }
            Err(error) => {
                state.io_error();
                if first_error.is_none() {
                    first_error = Some(PoolProxyError::from(error));
                }
            }
        }

        if let Some(error) = first_error {
            Err(error)
        } else {
            Ok(())
        }
    })
}

fn copy_and_shutdown(reader: &mut TcpStream, writer: &mut TcpStream) -> io::Result<u64> {
    let bytes = io::copy(reader, writer)?;
    let _ = writer.shutdown(Shutdown::Write);
    Ok(bytes)
}

fn serve_admin(
    listener: TcpListener,
    state: Arc<PoolProxyState>,
    upstream_addr: String,
) -> Result<(), PoolProxyError> {
    eprintln!(
        "ai-blaise pool admin server listening on {}",
        listener.local_addr()?
    );
    for stream in listener.incoming() {
        let mut stream = stream?;
        let mut buffer = [0_u8; 8192];
        let read_len = stream.read(&mut buffer)?;
        let response = AdminRequest::parse(&buffer[..read_len])
            .map(|request| handle_admin_request(&request, &state, &upstream_addr))
            .unwrap_or_else(|error| AdminResponse {
                status_code: 400,
                content_type: "application/json",
                body: format!("{{\"error\":\"{}\"}}\n", escape_json(&error.to_string())),
            });
        stream.write_all(response.to_http_string().as_bytes())?;
    }
    Ok(())
}

pub fn handle_admin_request(
    request: &AdminRequest,
    state: &PoolProxyState,
    upstream_addr: &str,
) -> AdminResponse {
    if request.method != "GET" {
        return AdminResponse {
            status_code: 405,
            content_type: "application/json",
            body: "{\"error\":\"method not allowed\"}\n".to_string(),
        };
    }

    match request.path.as_str() {
        "/healthz" => AdminResponse {
            status_code: 200,
            content_type: "application/json",
            body: health_json(
                state,
                upstream_addr,
                true,
                connect_upstream(upstream_addr).is_ok(),
            ),
        },
        "/readyz" => {
            let upstream_ready = connect_upstream(upstream_addr).is_ok();
            AdminResponse {
                status_code: if upstream_ready { 200 } else { 503 },
                content_type: "application/json",
                body: health_json(state, upstream_addr, upstream_ready, upstream_ready),
            }
        }
        "/metrics" => AdminResponse {
            status_code: 200,
            content_type: "text/plain; version=0.0.4",
            body: metrics_text(state, upstream_addr),
        },
        _ => AdminResponse {
            status_code: 404,
            content_type: "application/json",
            body: "{\"error\":\"not found\"}\n".to_string(),
        },
    }
}

fn connect_upstream(upstream_addr: &str) -> Result<TcpStream, PoolProxyError> {
    let socket_addr = upstream_addr
        .to_socket_addrs()?
        .next()
        .ok_or_else(|| PoolProxyError::InvalidAddress(upstream_addr.to_string()))?;
    TcpStream::connect_timeout(&socket_addr, CONNECT_TIMEOUT).map_err(PoolProxyError::from)
}

fn env_or_default(name: &str, default_value: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| default_value.to_string())
}

fn validate_addr(field: &'static str, address: &str) -> Result<(), PoolProxyError> {
    if address.trim().is_empty() {
        return Err(PoolProxyError::MissingAddress(field));
    }
    if !address.contains(':') {
        return Err(PoolProxyError::InvalidAddress(address.to_string()));
    }
    Ok(())
}

fn health_json(
    state: &PoolProxyState,
    upstream_addr: &str,
    ready: bool,
    upstream_ready: bool,
) -> String {
    format!(
        "{{\"component\":\"pool\",\"ready\":{},\"upstream_ready\":{},\"upstream_addr\":\"{}\",\"active_connections\":{},\"accepted_connections\":{},\"completed_connections\":{},\"rejected_connections\":{},\"upstream_connect_errors\":{},\"io_errors\":{},\"uptime_seconds\":{}}}\n",
        ready,
        upstream_ready,
        escape_json(upstream_addr),
        state.active_connections.load(Ordering::Relaxed),
        state.accepted_connections.load(Ordering::Relaxed),
        state.completed_connections.load(Ordering::Relaxed),
        state.rejected_connections.load(Ordering::Relaxed),
        state.upstream_connect_errors.load(Ordering::Relaxed),
        state.io_errors.load(Ordering::Relaxed),
        state.uptime_seconds(),
    )
}

fn metrics_text(state: &PoolProxyState, upstream_addr: &str) -> String {
    let upstream_ready = u8::from(connect_upstream(upstream_addr).is_ok());
    format!(
        "# HELP ai_blaise_citus_pool_upstream_ready Whether the configured PostgreSQL upstream accepts TCP connections.\n\
         # TYPE ai_blaise_citus_pool_upstream_ready gauge\n\
         ai_blaise_citus_pool_upstream_ready{{upstream=\"{}\"}} {}\n\
         # HELP ai_blaise_citus_pool_active_connections Active proxied PostgreSQL connections.\n\
         # TYPE ai_blaise_citus_pool_active_connections gauge\n\
         ai_blaise_citus_pool_active_connections {}\n\
         # HELP ai_blaise_citus_pool_requests_total Accepted PostgreSQL client connections.\n\
         # TYPE ai_blaise_citus_pool_requests_total counter\n\
         ai_blaise_citus_pool_requests_total {}\n\
         # HELP ai_blaise_citus_pool_rejected_connections_total PostgreSQL client connections rejected by CIDR allowlist.\n\
         # TYPE ai_blaise_citus_pool_rejected_connections_total counter\n\
         ai_blaise_citus_pool_rejected_connections_total {}\n\
         # HELP ai_blaise_citus_pool_errors_total Upstream connect and proxy I/O errors.\n\
         # TYPE ai_blaise_citus_pool_errors_total counter\n\
         ai_blaise_citus_pool_errors_total {}\n\
         # HELP ai_blaise_citus_pool_client_to_upstream_bytes_total Bytes copied from clients to upstream PostgreSQL.\n\
         # TYPE ai_blaise_citus_pool_client_to_upstream_bytes_total counter\n\
         ai_blaise_citus_pool_client_to_upstream_bytes_total {}\n\
         # HELP ai_blaise_citus_pool_upstream_to_client_bytes_total Bytes copied from upstream PostgreSQL to clients.\n\
         # TYPE ai_blaise_citus_pool_upstream_to_client_bytes_total counter\n\
         ai_blaise_citus_pool_upstream_to_client_bytes_total {}\n\
         # HELP ai_blaise_citus_pool_traceparent_tapped_total Client connections whose startup envelope carried a W3C traceparent.\n\
         # TYPE ai_blaise_citus_pool_traceparent_tapped_total counter\n\
         ai_blaise_citus_pool_traceparent_tapped_total {}\n\
         # HELP ai_blaise_citus_pool_traceparent_absent_total Client connections whose startup envelope did not carry a W3C traceparent.\n\
         # TYPE ai_blaise_citus_pool_traceparent_absent_total counter\n\
         ai_blaise_citus_pool_traceparent_absent_total {}\n",
        escape_prometheus_label(upstream_addr),
        upstream_ready,
        state.active_connections.load(Ordering::Relaxed),
        state.accepted_connections.load(Ordering::Relaxed),
        state.rejected_connections.load(Ordering::Relaxed),
        state.upstream_connect_errors.load(Ordering::Relaxed)
            + state.io_errors.load(Ordering::Relaxed),
        state.client_to_upstream_bytes.load(Ordering::Relaxed),
        state.upstream_to_client_bytes.load(Ordering::Relaxed),
        state.traceparent_tapped.load(Ordering::Relaxed),
        state.traceparent_absent.load(Ordering::Relaxed),
    )
}

fn status_reason(status_code: u16) -> &'static str {
    match status_code {
        200 => "OK",
        400 => "Bad Request",
        404 => "Not Found",
        405 => "Method Not Allowed",
        503 => "Service Unavailable",
        _ => "Unknown",
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

fn escape_prometheus_label(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

#[derive(Debug, Eq, PartialEq)]
pub enum PoolProxyError {
    AddressCollision {
        left: &'static str,
        right: &'static str,
    },
    ClientRejected {
        client_ip: String,
        allowlist: String,
    },
    InvalidAddress(String),
    InvalidCidr(String),
    Io(String),
    MalformedHttp,
    MissingAddress(&'static str),
    MissingEnv(&'static str),
    WorkerPanicked,
}

impl fmt::Display for PoolProxyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AddressCollision { left, right } => {
                write!(formatter, "{left} must differ from {right}")
            }
            Self::ClientRejected {
                client_ip,
                allowlist,
            } => {
                write!(
                    formatter,
                    "client {client_ip} is outside pool CIDR allowlist {allowlist}"
                )
            }
            Self::InvalidAddress(address) => write!(formatter, "invalid address: {address}"),
            Self::InvalidCidr(cidr) => write!(formatter, "invalid CIDR allowlist entry: {cidr}"),
            Self::Io(error) => write!(formatter, "{error}"),
            Self::MalformedHttp => write!(formatter, "malformed HTTP request"),
            Self::MissingAddress(field) => write!(formatter, "{field} must not be empty"),
            Self::MissingEnv(name) => write!(formatter, "{name} is required"),
            Self::WorkerPanicked => write!(formatter, "proxy worker panicked"),
        }
    }
}

impl Error for PoolProxyError {}

impl From<io::Error> for PoolProxyError {
    fn from(error: io::Error) -> Self {
        Self::Io(error.to_string())
    }
}

impl From<std::net::AddrParseError> for PoolProxyError {
    fn from(error: std::net::AddrParseError) -> Self {
        Self::InvalidAddress(error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};

    #[test]
    fn config_requires_upstream_and_separate_ports() {
        let config = PoolProxyConfig {
            listen_addr: "127.0.0.1:15432".to_string(),
            admin_addr: "127.0.0.1:15432".to_string(),
            upstream_addr: "127.0.0.1:25432".to_string(),
            client_cidr_allowlist: ClientCidrAllowlist::default(),
        };

        assert_eq!(
            config.validate().unwrap_err(),
            PoolProxyError::AddressCollision {
                left: "listen_addr",
                right: "admin_addr",
            }
        );
    }

    #[test]
    fn admin_readyz_reflects_upstream_connectivity() {
        let upstream = TcpListener::bind("127.0.0.1:0").unwrap();
        let upstream_addr = upstream.local_addr().unwrap().to_string();
        let upstream_thread = thread::spawn(move || {
            let _ = upstream.accept();
        });
        let state = PoolProxyState::new();
        let request = AdminRequest {
            method: "GET".to_string(),
            path: "/readyz".to_string(),
        };

        let response = handle_admin_request(&request, &state, &upstream_addr);

        assert_eq!(response.status_code, 200);
        assert!(response.body.contains("\"upstream_ready\":true"));
        upstream_thread.join().unwrap();
    }

    #[test]
    fn admin_healthz_does_not_claim_unreachable_upstream_ready() {
        let state = PoolProxyState::new();
        let request = AdminRequest {
            method: "GET".to_string(),
            path: "/healthz".to_string(),
        };

        let response = handle_admin_request(&request, &state, "127.0.0.1:1");

        assert_eq!(response.status_code, 200);
        assert!(response.body.contains("\"ready\":true"));
        assert!(response.body.contains("\"upstream_ready\":false"));
    }

    #[test]
    fn admin_metrics_expose_pool_counters() {
        let state = PoolProxyState::new();
        state.accepted();
        state.connect_error();
        state.completed();
        let request = AdminRequest {
            method: "GET".to_string(),
            path: "/metrics".to_string(),
        };

        let response = handle_admin_request(&request, &state, "127.0.0.1:1");

        assert_eq!(response.status_code, 200);
        assert!(response
            .body
            .contains("ai_blaise_citus_pool_requests_total 1"));
        assert!(response
            .body
            .contains("ai_blaise_citus_pool_rejected_connections_total 0"));
        assert!(response
            .body
            .contains("ai_blaise_citus_pool_errors_total 1"));
    }

    #[test]
    fn cidr_allowlist_accepts_matching_clients_and_rejects_others() {
        let allowlist = ClientCidrAllowlist::parse_csv("127.0.0.0/8,10.244.0.0/16").unwrap();

        assert!(allowlist.allows("127.0.0.1".parse().unwrap()));
        assert!(allowlist.allows("10.244.2.15".parse().unwrap()));
        assert!(!allowlist.allows("192.0.2.10".parse().unwrap()));
        assert_eq!(allowlist.as_csv(), "127.0.0.0/8,10.244.0.0/16");
    }

    #[test]
    fn cidr_allowlist_rejects_invalid_prefixes() {
        assert_eq!(
            ClientCidrAllowlist::parse_csv("127.0.0.1/33").unwrap_err(),
            PoolProxyError::InvalidCidr("127.0.0.1/33".to_string())
        );
        assert_eq!(
            ClientCidrAllowlist::parse_csv("not-a-cidr").unwrap_err(),
            PoolProxyError::InvalidCidr("not-a-cidr".to_string())
        );
    }

    fn build_startup_packet(application_name: &str) -> Vec<u8> {
        let mut body: Vec<u8> = Vec::new();
        body.extend_from_slice(&196608_u32.to_be_bytes());
        body.extend_from_slice(b"user");
        body.push(0);
        body.extend_from_slice(b"postgres");
        body.push(0);
        body.extend_from_slice(b"database");
        body.push(0);
        body.extend_from_slice(b"postgres");
        body.push(0);
        body.extend_from_slice(b"application_name");
        body.push(0);
        body.extend_from_slice(application_name.as_bytes());
        body.push(0);
        body.push(0);
        let length = (body.len() + 4) as u32;
        let mut packet = Vec::with_capacity(body.len() + 4);
        packet.extend_from_slice(&length.to_be_bytes());
        packet.extend_from_slice(&body);
        packet
    }

    #[test]
    fn proxy_forwards_bidirectional_tcp_bytes() {
        let startup_packet = build_startup_packet("psql");
        let expected_upstream = startup_packet.clone();
        let upstream = TcpListener::bind("127.0.0.1:0").unwrap();
        let upstream_addr = upstream.local_addr().unwrap().to_string();
        let upstream_thread = thread::spawn(move || {
            let (mut stream, _) = upstream.accept().unwrap();
            let mut received = vec![0_u8; expected_upstream.len()];
            stream.read_exact(&mut received).unwrap();
            assert_eq!(received, expected_upstream);
            stream.write_all(b"AuthenticationOK").unwrap();
        });

        let proxy_listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let proxy_addr = proxy_listener.local_addr().unwrap();
        let proxy_thread = thread::spawn(move || {
            let (client, _) = proxy_listener.accept().unwrap();
            let state = PoolProxyState::new();
            let allowlist = ClientCidrAllowlist::parse_csv("127.0.0.0/8").unwrap();
            handle_proxy_connection(client, &upstream_addr, &allowlist, &state).unwrap();
            assert_eq!(state.accepted_connections.load(Ordering::Relaxed), 1);
            assert_eq!(state.completed_connections.load(Ordering::Relaxed), 1);
            assert_eq!(state.traceparent_absent_count(), 1);
            assert_eq!(state.traceparent_tapped_count(), 0);
        });

        let mut client = TcpStream::connect(proxy_addr).unwrap();
        client.write_all(&startup_packet).unwrap();
        client.shutdown(Shutdown::Write).unwrap();
        let mut response = vec![0_u8; b"AuthenticationOK".len()];
        client.read_exact(&mut response).unwrap();

        assert_eq!(&response, b"AuthenticationOK");
        upstream_thread.join().unwrap();
        proxy_thread.join().unwrap();
    }

    #[test]
    fn proxy_records_traceparent_when_application_name_embeds_one() {
        const TRACEPARENT: &str = "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01";
        let application_name =
            format!("application=ai_blaise_pipeline_smoke;traceparent={TRACEPARENT}");
        let startup_packet = build_startup_packet(&application_name);
        let expected_upstream = startup_packet.clone();
        let upstream = TcpListener::bind("127.0.0.1:0").unwrap();
        let upstream_addr = upstream.local_addr().unwrap().to_string();
        let upstream_thread = thread::spawn(move || {
            let (mut stream, _) = upstream.accept().unwrap();
            let mut received = vec![0_u8; expected_upstream.len()];
            stream.read_exact(&mut received).unwrap();
            assert_eq!(received, expected_upstream);
            stream.write_all(b"AuthenticationOK").unwrap();
        });

        let proxy_listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let proxy_addr = proxy_listener.local_addr().unwrap();
        let proxy_thread = thread::spawn(move || {
            let (client, _) = proxy_listener.accept().unwrap();
            let state = PoolProxyState::new();
            let allowlist = ClientCidrAllowlist::parse_csv("127.0.0.0/8").unwrap();
            handle_proxy_connection(client, &upstream_addr, &allowlist, &state).unwrap();
            assert_eq!(state.traceparent_tapped_count(), 1);
            assert_eq!(state.traceparent_absent_count(), 0);
        });

        let mut client = TcpStream::connect(proxy_addr).unwrap();
        client.write_all(&startup_packet).unwrap();
        client.shutdown(Shutdown::Write).unwrap();
        let mut response = vec![0_u8; b"AuthenticationOK".len()];
        client.read_exact(&mut response).unwrap();

        assert_eq!(&response, b"AuthenticationOK");
        upstream_thread.join().unwrap();
        proxy_thread.join().unwrap();
    }

    #[test]
    fn proxy_rejects_clients_outside_cidr_allowlist_before_upstream_connect() {
        let proxy_listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let proxy_addr = proxy_listener.local_addr().unwrap();
        let proxy_thread = thread::spawn(move || {
            let (client, _) = proxy_listener.accept().unwrap();
            let state = PoolProxyState::new();
            let allowlist = ClientCidrAllowlist::parse_csv("192.0.2.0/24").unwrap();
            let error =
                handle_proxy_connection(client, "127.0.0.1:1", &allowlist, &state).unwrap_err();

            assert!(matches!(error, PoolProxyError::ClientRejected { .. }));
            assert_eq!(state.rejected_connections.load(Ordering::Relaxed), 1);
            assert_eq!(state.accepted_connections.load(Ordering::Relaxed), 0);
            assert_eq!(state.upstream_connect_errors.load(Ordering::Relaxed), 0);
        });

        let _client = TcpStream::connect(proxy_addr).unwrap();
        proxy_thread.join().unwrap();
    }
}
