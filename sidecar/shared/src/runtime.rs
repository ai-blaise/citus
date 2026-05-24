// FEATURE: O4

use crate::{ComponentState, DrainState, HealthReport};
use std::error::Error;
use std::fmt;
use std::net::TcpListener;
use std::time::{Duration, SystemTime};

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SidecarRuntime {
    component: String,
    started_at: SystemTime,
    readiness_detail: Option<String>,
    drain: DrainState,
}

impl SidecarRuntime {
    pub fn ready(component: impl Into<String>) -> Self {
        Self {
            component: component.into(),
            started_at: SystemTime::now(),
            readiness_detail: None,
            drain: DrainState::active(0),
        }
    }

    pub fn not_ready(component: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            component: component.into(),
            started_at: SystemTime::now(),
            readiness_detail: Some(detail.into()),
            drain: DrainState::active(0),
        }
    }

    pub fn with_in_flight_work(mut self, in_flight_work: u64) -> Self {
        self.drain.in_flight_work = in_flight_work;
        self
    }

    pub fn component(&self) -> &str {
        &self.component
    }

    pub fn drain_state(&self) -> &DrainState {
        &self.drain
    }

    pub fn health_report(&self) -> HealthReport {
        if !self.drain.accepting_new_work {
            return HealthReport::draining(
                self.component.clone(),
                self.started_at,
                "drain requested",
            );
        }

        match &self.readiness_detail {
            Some(detail) => {
                HealthReport::not_ready(self.component.clone(), self.started_at, detail.clone())
            }
            None => HealthReport::ready(self.component.clone(), self.started_at),
        }
    }

    pub fn begin_drain(&mut self, in_flight_work: u64) {
        self.drain = DrainState::draining(in_flight_work);
    }

    pub fn finish_drain(&mut self) {
        self.drain = DrainState::draining(0);
    }

    pub fn handle_http_request(&mut self, request: &HttpProbeRequest) -> HttpProbeResponse {
        match (&request.method, request.path.as_str()) {
            (HttpMethod::Get, "/healthz") => self.health_response(200),
            (HttpMethod::Get, "/readyz") => {
                if self.is_ready_for_work() {
                    self.health_response(200)
                } else {
                    self.health_response(503)
                }
            }
            (HttpMethod::Get, "/drain") => self.drain_response(200),
            (HttpMethod::Post, "/drain") => {
                let in_flight_work = self.drain.in_flight_work;
                self.begin_drain(in_flight_work);
                self.drain_response(202)
            }
            (HttpMethod::Get, "/metrics") => {
                HttpProbeResponse::new(200, "text/plain; version=0.0.4", self.prometheus_metrics())
            }
            (HttpMethod::Other(_), _) => HttpProbeResponse::new(
                405,
                "application/json",
                "{\"error\":\"method not allowed\"}\n".to_string(),
            ),
            _ if is_known_path(&request.path) => HttpProbeResponse::new(
                405,
                "application/json",
                "{\"error\":\"method not allowed\"}\n".to_string(),
            ),
            _ => HttpProbeResponse::new(
                404,
                "application/json",
                "{\"error\":\"not found\"}\n".to_string(),
            ),
        }
    }

    pub fn handle_http_bytes(
        &mut self,
        request: &[u8],
    ) -> Result<HttpProbeResponse, SidecarRuntimeError> {
        let request = std::str::from_utf8(request)
            .map_err(|_| SidecarRuntimeError::MalformedRequest)?
            .parse::<HttpProbeRequest>()?;
        Ok(self.handle_http_request(&request))
    }

    fn is_ready_for_work(&self) -> bool {
        self.readiness_detail.is_none() && self.drain.accepting_new_work
    }

    fn health_response(&self, status_code: u16) -> HttpProbeResponse {
        let report = self.health_report();
        HttpProbeResponse::new(status_code, "application/json", self.health_json(&report))
    }

    fn drain_response(&self, status_code: u16) -> HttpProbeResponse {
        HttpProbeResponse::new(status_code, "application/json", self.drain_json())
    }

    fn health_json(&self, report: &HealthReport) -> String {
        format!(
            "{{\"component\":\"{}\",\"state\":\"{}\",\"ready\":{},\"accepting_new_work\":{},\"in_flight_work\":{},\"uptime_seconds\":{},\"detail\":{}}}\n",
            escape_json(&report.component),
            component_state(report.state),
            report.is_ready(),
            self.drain.accepting_new_work,
            self.drain.in_flight_work,
            report.uptime().unwrap_or(Duration::ZERO).as_secs(),
            json_optional(report.detail.as_deref()),
        )
    }

    fn drain_json(&self) -> String {
        format!(
            "{{\"component\":\"{}\",\"accepting_new_work\":{},\"in_flight_work\":{},\"drained\":{}}}\n",
            escape_json(&self.component),
            self.drain.accepting_new_work,
            self.drain.in_flight_work,
            self.drain.is_drained(),
        )
    }

    fn prometheus_metrics(&self) -> String {
        let ready = u8::from(self.is_ready_for_work());
        let accepting = u8::from(self.drain.accepting_new_work);
        format!(
            "# HELP ai_blaise_sidecar_ready Whether the sidecar is ready for new work.\n\
             # TYPE ai_blaise_sidecar_ready gauge\n\
             ai_blaise_sidecar_ready{{component=\"{}\"}} {}\n\
             # HELP ai_blaise_sidecar_accepting_new_work Whether the sidecar accepts new work.\n\
             # TYPE ai_blaise_sidecar_accepting_new_work gauge\n\
             ai_blaise_sidecar_accepting_new_work{{component=\"{}\"}} {}\n\
             # HELP ai_blaise_sidecar_in_flight_work In-flight work tracked by the sidecar runtime.\n\
             # TYPE ai_blaise_sidecar_in_flight_work gauge\n\
             ai_blaise_sidecar_in_flight_work{{component=\"{}\"}} {}\n",
            escape_prometheus_label(&self.component),
            ready,
            escape_prometheus_label(&self.component),
            accepting,
            escape_prometheus_label(&self.component),
            self.drain.in_flight_work,
        )
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct HttpProbeRequest {
    pub method: HttpMethod,
    pub path: String,
    pub headers: crate::otel::HeaderMap,
}

impl HttpProbeRequest {
    pub fn new(method: HttpMethod, path: impl Into<String>) -> Self {
        Self {
            method,
            path: path.into(),
            headers: crate::otel::HeaderMap::new(),
        }
    }

    /// Construct a probe request with a populated header map. Used by HTTP
    /// servers and tests that need to assert traceparent extraction.
    pub fn with_headers(
        method: HttpMethod,
        path: impl Into<String>,
        headers: crate::otel::HeaderMap,
    ) -> Self {
        Self {
            method,
            path: path.into(),
            headers,
        }
    }

    /// Extract a W3C traceparent (and optional tracestate) from the request
    /// headers if present.
    pub fn extract_trace_context(
        &self,
    ) -> Option<(crate::otel::TraceParent, crate::otel::TraceState)> {
        use crate::otel::TraceContext;
        self.headers.extract()
    }
}

impl std::str::FromStr for HttpProbeRequest {
    type Err = SidecarRuntimeError;

    fn from_str(request: &str) -> Result<Self, Self::Err> {
        let mut lines = request.lines();
        let request_line = lines.next().ok_or(SidecarRuntimeError::MalformedRequest)?;
        let mut parts = request_line.split_whitespace();
        let method = parts.next().ok_or(SidecarRuntimeError::MalformedRequest)?;
        let path = parts.next().ok_or(SidecarRuntimeError::MalformedRequest)?;

        if !path.starts_with('/') {
            return Err(SidecarRuntimeError::MalformedRequest);
        }

        let mut headers = crate::otel::HeaderMap::new();
        for header_line in lines {
            if header_line.is_empty() {
                break;
            }
            if let Some((name, value)) = header_line.split_once(':') {
                headers.insert(name.trim(), value.trim());
            }
        }

        Ok(Self::with_headers(HttpMethod::from(method), path, headers))
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum HttpMethod {
    Get,
    Post,
    Other(String),
}

impl From<&str> for HttpMethod {
    fn from(method: &str) -> Self {
        match method {
            "GET" => Self::Get,
            "POST" => Self::Post,
            other => Self::Other(other.to_string()),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct HttpProbeResponse {
    pub status_code: u16,
    pub reason: &'static str,
    pub content_type: String,
    pub body: String,
}

impl HttpProbeResponse {
    pub fn new(status_code: u16, content_type: impl Into<String>, body: impl Into<String>) -> Self {
        Self {
            status_code,
            reason: status_reason(status_code),
            content_type: content_type.into(),
            body: body.into(),
        }
    }

    pub fn to_http_string(&self) -> String {
        format!(
            "HTTP/1.1 {} {}\r\ncontent-type: {}\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
            self.status_code,
            self.reason,
            self.content_type,
            self.body.len(),
            self.body,
        )
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum SidecarRuntimeError {
    Io(String),
    MalformedRequest,
    InvalidListenAddress(String),
}

impl fmt::Display for SidecarRuntimeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "{error}"),
            Self::MalformedRequest => write!(formatter, "malformed HTTP probe request"),
            Self::InvalidListenAddress(address) => {
                write!(formatter, "invalid listen address: {address}")
            }
        }
    }
}

impl Error for SidecarRuntimeError {}

impl From<std::io::Error> for SidecarRuntimeError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error.to_string())
    }
}

pub fn listen_addr_from_env(default_addr: &str) -> Result<String, SidecarRuntimeError> {
    let address = std::env::var("AI_BLAISE_LISTEN_ADDR").unwrap_or_else(|_| default_addr.into());
    if address.trim().is_empty() {
        return Err(SidecarRuntimeError::InvalidListenAddress(address));
    }
    Ok(address)
}

pub fn serve_tcp_forever(
    listen_addr: &str,
    runtime: SidecarRuntime,
) -> Result<(), SidecarRuntimeError> {
    use std::io::{Read, Write};

    let listener = TcpListener::bind(listen_addr)?;
    let component = runtime.component().to_string();
    eprintln!("ai-blaise {component} probe server listening on {listen_addr}");

    for stream in listener.incoming() {
        let mut stream = stream?;
        let mut runtime = runtime.clone();
        let mut buffer = [0_u8; 8192];
        let read_len = stream.read(&mut buffer)?;
        let response = runtime
            .handle_http_bytes(&buffer[..read_len])
            .unwrap_or_else(|error| {
                HttpProbeResponse::new(
                    400,
                    "application/json",
                    format!("{{\"error\":\"{}\"}}\n", escape_json(&error.to_string())),
                )
            });
        stream.write_all(response.to_http_string().as_bytes())?;
    }

    Ok(())
}

pub fn run_probe_server(component: &str, default_addr: &str) -> Result<(), SidecarRuntimeError> {
    let listen_addr = listen_addr_from_env(default_addr)?;
    serve_tcp_forever(&listen_addr, SidecarRuntime::ready(component))
}

#[cfg(unix)]
pub fn serve_unix_once<P: AsRef<std::path::Path>>(
    socket_path: P,
    mut runtime: SidecarRuntime,
) -> Result<(), SidecarRuntimeError> {
    use std::io::{Read, Write};
    use std::os::unix::fs::FileTypeExt;
    use std::os::unix::net::UnixListener;

    let socket_path = socket_path.as_ref();
    if let Ok(metadata) = std::fs::metadata(socket_path) {
        if metadata.file_type().is_socket() {
            std::fs::remove_file(socket_path)?;
        }
    }

    let listener = UnixListener::bind(socket_path)?;
    let (mut stream, _) = listener.accept()?;
    let mut buffer = [0_u8; 4096];
    let read_len = stream.read(&mut buffer)?;
    let response = runtime
        .handle_http_bytes(&buffer[..read_len])
        .unwrap_or_else(|error| {
            HttpProbeResponse::new(
                400,
                "application/json",
                format!("{{\"error\":\"{}\"}}\n", escape_json(&error.to_string())),
            )
        });
    stream.write_all(response.to_http_string().as_bytes())?;
    std::fs::remove_file(socket_path)?;
    Ok(())
}

fn is_known_path(path: &str) -> bool {
    matches!(path, "/healthz" | "/readyz" | "/drain" | "/metrics")
}

fn component_state(state: ComponentState) -> &'static str {
    match state {
        ComponentState::Ready => "ready",
        ComponentState::NotReady => "not_ready",
        ComponentState::Draining => "draining",
    }
}

fn status_reason(status_code: u16) -> &'static str {
    match status_code {
        200 => "OK",
        101 => "Switching Protocols",
        202 => "Accepted",
        400 => "Bad Request",
        426 => "Upgrade Required",
        404 => "Not Found",
        405 => "Method Not Allowed",
        503 => "Service Unavailable",
        _ => "Unknown",
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

fn escape_prometheus_label(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn readyz_reports_ready_until_drain_starts() {
        let mut runtime = SidecarRuntime::ready("cdc").with_in_flight_work(3);

        let ready = runtime.handle_http_request(&HttpProbeRequest::new(HttpMethod::Get, "/readyz"));
        assert_eq!(ready.status_code, 200);
        assert!(ready.body.contains("\"state\":\"ready\""));

        let drain = runtime.handle_http_request(&HttpProbeRequest::new(HttpMethod::Post, "/drain"));
        assert_eq!(drain.status_code, 202);
        assert!(drain.body.contains("\"accepting_new_work\":false"));

        let not_ready =
            runtime.handle_http_request(&HttpProbeRequest::new(HttpMethod::Get, "/readyz"));
        assert_eq!(not_ready.status_code, 503);
        assert!(not_ready.body.contains("\"state\":\"draining\""));
    }

    #[test]
    fn metrics_expose_ready_and_in_flight_state() {
        let mut runtime = SidecarRuntime::ready("storage").with_in_flight_work(7);

        let metrics =
            runtime.handle_http_request(&HttpProbeRequest::new(HttpMethod::Get, "/metrics"));

        assert_eq!(metrics.status_code, 200);
        assert!(metrics.body.contains("ai_blaise_sidecar_ready"));
        assert!(metrics.body.contains("component=\"storage\""));
        assert!(metrics
            .body
            .contains("ai_blaise_sidecar_in_flight_work{component=\"storage\"} 7"));
    }

    #[test]
    fn malformed_request_is_rejected() {
        let mut runtime = SidecarRuntime::ready("auth");

        assert_eq!(
            runtime.handle_http_bytes(b"not-http").unwrap_err(),
            SidecarRuntimeError::MalformedRequest
        );
    }

    #[test]
    fn raw_http_probe_renders_http_response() {
        let mut runtime = SidecarRuntime::not_ready("vectorizer", "queue lag budget exceeded");

        let response = runtime
            .handle_http_bytes(b"GET /readyz HTTP/1.1\r\nHost: local\r\n\r\n")
            .unwrap();

        assert_eq!(response.status_code, 503);
        assert!(response
            .to_http_string()
            .starts_with("HTTP/1.1 503 Service Unavailable"));
        assert!(response.body.contains("queue lag budget exceeded"));
    }

    #[test]
    fn http_probe_request_parses_traceparent_header() {
        let raw = concat!(
            "GET /healthz HTTP/1.1\r\n",
            "Host: local\r\n",
            "traceparent: 00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01\r\n",
            "tracestate: vendor=opaque\r\n",
            "\r\n",
        );
        let request: HttpProbeRequest = raw.parse().unwrap();
        let (traceparent, state) = request.extract_trace_context().unwrap();
        assert_eq!(
            traceparent.to_header_value(),
            "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01",
        );
        assert_eq!(state.as_str(), "vendor=opaque");
    }

    #[test]
    fn listen_addr_defaults_and_rejects_empty_env() {
        std::env::remove_var("AI_BLAISE_LISTEN_ADDR");
        assert_eq!(
            listen_addr_from_env("127.0.0.1:8080").unwrap(),
            "127.0.0.1:8080"
        );

        std::env::set_var("AI_BLAISE_LISTEN_ADDR", "");
        assert_eq!(
            listen_addr_from_env("127.0.0.1:8080").unwrap_err(),
            SidecarRuntimeError::InvalidListenAddress(String::new())
        );
        std::env::remove_var("AI_BLAISE_LISTEN_ADDR");
    }

    #[cfg(unix)]
    #[test]
    fn unix_socket_one_shot_serves_healthz() {
        use std::io::{Read, Write};
        use std::os::unix::net::UnixStream;
        use std::thread;
        use std::time::Duration;

        let socket_path = std::path::PathBuf::from("/tmp").join(format!(
            "ai-blaise-sidecar-runtime-{}-{}.sock",
            std::process::id(),
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let server_path = socket_path.clone();
        let runtime = SidecarRuntime::ready("shared");
        let server = thread::spawn(move || serve_unix_once(&server_path, runtime).unwrap());

        for _ in 0..100 {
            if socket_path.exists() {
                break;
            }
            thread::sleep(Duration::from_millis(10));
        }

        let mut stream = UnixStream::connect(&socket_path).unwrap();
        stream
            .write_all(b"GET /healthz HTTP/1.1\r\nHost: local\r\n\r\n")
            .unwrap();
        let mut response = String::new();
        stream.read_to_string(&mut response).unwrap();
        server.join().unwrap();

        assert!(response.starts_with("HTTP/1.1 200 OK"));
        assert!(response.contains("\"component\":\"shared\""));
    }
}
