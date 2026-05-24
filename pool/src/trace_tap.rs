//! PostgreSQL startup-message tap that extracts a W3C traceparent embedded
//! in the libpq `application_name` startup parameter.
//!
//! The pool proxy is otherwise a transparent byte-copy after admission. The
//! trace-tap reads the first startup-message envelope from the client, parses
//! the `application_name` value for a traceparent using the canonical wire
//! format documented on `ai_blaise_citus_sidecar_shared::parse_application_name`,
//! and returns both the parsed result and the buffered bytes. When pool auth is
//! enabled, callers replay `sanitized_startup_bytes()` so pool-only auth
//! parameters are consumed by the pool and not leaked to PostgreSQL.
//!
//! The companion pgrx extension recovers the traceparent on the server side
//! by reading `current_setting('application_name')` and re-parsing it,
//! producing a child OpenTelemetry span anchored to the same trace. This
//! preserves PostgreSQL wire compatibility while threading the W3C
//! traceparent end-to-end through pool → PostgreSQL → companion → sidecars.

// FEATURE: O14

use std::io::{self, Read};
use std::time::Duration;

use ai_blaise_citus_sidecar_shared::{
    parse_application_name, ApplicationNameFields, TraceParent, TraceState,
};

/// Soft cap on the bytes we will buffer from the client before declaring the
/// startup message malformed. The PostgreSQL wire spec caps a startup packet
/// at 30,000 bytes; libpq enforces 8,192 bytes for the parameter list. We
/// accept up to the spec maximum so non-libpq clients still work.
const STARTUP_MESSAGE_MAX_BYTES: usize = 30_000;

/// Result of peeking the startup message. Callers normally replay
/// `buffered_bytes`; auth-enabled pool paths replay `sanitized_startup_bytes()`
/// so pool-only credential parameters do not reach PostgreSQL.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct StartupTraceTap {
    pub fields: ApplicationNameFields,
    pub buffered_bytes: Vec<u8>,
    pub parameters: Vec<(String, String)>,
    /// `true` when the client opened the connection with an `SSLRequest`,
    /// `GSSENCRequest`, or `CancelRequest` envelope. These envelopes are
    /// not startup packets so no traceparent can be extracted from them.
    pub special_envelope: bool,
}

impl StartupTraceTap {
    pub fn traceparent(&self) -> Option<&TraceParent> {
        self.fields.traceparent.as_ref()
    }

    pub fn tracestate(&self) -> Option<&TraceState> {
        self.fields.tracestate.as_ref()
    }

    pub fn application_name(&self) -> Option<&str> {
        self.fields.application.as_deref()
    }

    pub fn startup_parameter(&self, name: &str) -> Option<&str> {
        self.parameters
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }

    /// Return startup bytes with pool-only auth parameters removed.
    ///
    /// Auth and tenant guard parameters are consumed by the pool before it
    /// opens an upstream socket. Replaying them to PostgreSQL would either leak
    /// bearer material into backend logs or fail startup because the server does
    /// not know those custom parameters. Non-startup special envelopes are left
    /// unchanged.
    pub fn sanitized_startup_bytes(&self) -> Vec<u8> {
        if self.special_envelope || self.parameters.is_empty() || self.buffered_bytes.len() < 8 {
            return self.buffered_bytes.clone();
        }

        let protocol = &self.buffered_bytes[4..8];
        let mut body = Vec::new();
        body.extend_from_slice(protocol);
        for (key, value) in &self.parameters {
            if is_pool_only_startup_parameter(key) {
                continue;
            }
            body.extend_from_slice(key.as_bytes());
            body.push(0);
            body.extend_from_slice(value.as_bytes());
            body.push(0);
        }
        body.push(0);

        let length = (body.len() + 4) as u32;
        let mut packet = Vec::with_capacity(body.len() + 4);
        packet.extend_from_slice(&length.to_be_bytes());
        packet.extend_from_slice(&body);
        packet
    }
}

/// Read the first startup-message envelope from `reader` without modifying
/// the bytes, parse the embedded application_name for a traceparent, and
/// return both the buffered bytes and the parsed fields.
///
/// The returned bytes MUST be replayed to upstream PostgreSQL in order, in
/// addition to any subsequent bytes the client sends.
pub fn tap_startup_message<R: Read>(reader: &mut R) -> io::Result<StartupTraceTap> {
    let mut header = [0_u8; 8];
    read_exact_or_eof(reader, &mut header)?;
    let length = u32::from_be_bytes(header[0..4].try_into().unwrap()) as usize;
    let code = u32::from_be_bytes(header[4..8].try_into().unwrap());

    if !(8..=STARTUP_MESSAGE_MAX_BYTES).contains(&length) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("PostgreSQL startup envelope reported {length} byte length"),
        ));
    }

    let mut buffered_bytes = Vec::with_capacity(length);
    buffered_bytes.extend_from_slice(&header);

    let remaining = length - 8;
    if remaining > 0 {
        let mut body = vec![0_u8; remaining];
        read_exact_or_eof(reader, &mut body)?;
        buffered_bytes.extend_from_slice(&body);
    }

    // SSLRequest (80877103), GSSENCRequest (80877104), CancelRequest (80877102)
    // are special envelopes; they do not carry an application_name. The proxy
    // still replays them verbatim.
    if matches!(code, 80877102..=80877104) {
        return Ok(StartupTraceTap {
            fields: ApplicationNameFields::default(),
            buffered_bytes,
            parameters: Vec::new(),
            special_envelope: true,
        });
    }

    // The protocol version is the upper 16 bits major, lower 16 bits minor.
    // We accept anything from 2.0 onwards; the PostgreSQL community has used
    // 3.0 for two decades but the field is informational here.
    let major = (code >> 16) as u16;
    if !(2..=4).contains(&major) {
        // Not a recognized startup envelope; return an empty parse but keep
        // the bytes so they can be forwarded.
        return Ok(StartupTraceTap {
            fields: ApplicationNameFields::default(),
            buffered_bytes,
            parameters: Vec::new(),
            special_envelope: true,
        });
    }

    let body = &buffered_bytes[8..];
    let params = parse_libpq_startup_parameters(body);
    let fields = derive_fields_from_parameters(&params);
    Ok(StartupTraceTap {
        fields,
        buffered_bytes,
        parameters: params,
        special_envelope: false,
    })
}

fn derive_fields_from_parameters(params: &[(String, String)]) -> ApplicationNameFields {
    // We accept the traceparent in three places, in priority order:
    //
    //   1. A dedicated libpq startup parameter named "traceparent". Custom
    //      parameters require PG18+ but the pool happily passes them through.
    //   2. The libpq "options" parameter, when it contains
    //      `-c trace.parent=<value>`. PostgreSQL sets the GUC immediately
    //      and the pool records the same value here. This survives PG's
    //      63-byte identifier limit on application_name.
    //   3. The "application_name" parameter using the
    //      key=value;key=value wire format documented on
    //      ai_blaise_citus_sidecar_shared::parse_application_name. This is
    //      the legacy form for clients that cannot set custom GUCs.
    let mut fields = ApplicationNameFields::default();

    if let Some((_, value)) = params.iter().find(|(key, _)| key == "application_name") {
        fields = parse_application_name(value);
    }

    if let Some((_, options)) = params.iter().find(|(key, _)| key == "options") {
        if let Some(traceparent) = extract_options_traceparent(options) {
            if let Ok(parsed) = TraceParent::parse(&traceparent) {
                fields.traceparent = Some(parsed);
            }
        }
        if let Some(state) = extract_options_tracestate(options) {
            fields.tracestate = Some(TraceState::new(state));
        }
    }

    if let Some((_, raw)) = params.iter().find(|(key, _)| key == "traceparent") {
        if let Ok(parsed) = TraceParent::parse(raw) {
            fields.traceparent = Some(parsed);
        }
    }
    if let Some((_, raw)) = params.iter().find(|(key, _)| key == "tracestate") {
        fields.tracestate = Some(TraceState::new(raw));
    }

    fields
}

fn is_pool_only_startup_parameter(key: &str) -> bool {
    matches!(
        key.to_ascii_lowercase().as_str(),
        "ai_blaise.jwt"
            | "ai_blaise.token"
            | "ai_blaise.access_token"
            | "ai_blaise.tenant_id"
            | "jwt"
            | "token"
            | "access_token"
            | "tenant_id"
            | "tenant"
    )
}

fn parse_libpq_startup_parameters(body: &[u8]) -> Vec<(String, String)> {
    let mut params: Vec<(String, String)> = Vec::new();
    let mut iter = body.split(|byte| *byte == 0);
    loop {
        let key = match iter.next() {
            Some(slice) if !slice.is_empty() => slice,
            _ => return params,
        };
        let value = match iter.next() {
            Some(slice) => slice,
            None => return params,
        };
        let (Ok(key), Ok(value)) = (std::str::from_utf8(key), std::str::from_utf8(value)) else {
            continue;
        };
        params.push((key.to_string(), value.to_string()));
    }
}

fn extract_options_traceparent(options: &str) -> Option<String> {
    extract_options_assignment(options, "trace.parent")
}

fn extract_options_tracestate(options: &str) -> Option<String> {
    extract_options_assignment(options, "trace.state")
}

fn extract_options_assignment(options: &str, key: &str) -> Option<String> {
    // libpq's "options" parameter carries shell-style tokens, e.g.
    // "-c trace.parent=00-...-01 -c trace.state=vendor=ai-blaise". We accept
    // both "-c key=value" and "-ckey=value". Tokens that do not match are
    // ignored.
    let tokens: Vec<&str> = options.split_whitespace().collect();
    let mut index = 0;
    while index < tokens.len() {
        let token = tokens[index];
        if let Some(remainder) = token.strip_prefix("-c") {
            let assignment = if remainder.is_empty() {
                index += 1;
                if index >= tokens.len() {
                    break;
                }
                tokens[index]
            } else {
                remainder
            };
            if let Some((assignment_key, assignment_value)) = assignment.split_once('=') {
                if assignment_key == key {
                    return Some(assignment_value.to_string());
                }
            }
        }
        index += 1;
    }
    None
}

fn read_exact_or_eof<R: Read>(reader: &mut R, buffer: &mut [u8]) -> io::Result<()> {
    let mut filled = 0;
    while filled < buffer.len() {
        match reader.read(&mut buffer[filled..]) {
            Ok(0) => {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "client closed connection before sending complete startup envelope",
                ));
            }
            Ok(read) => filled += read,
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(error),
        }
    }
    Ok(())
}

/// Convenience helper exposed for tests and callers that want to log the
/// traceparent extraction outcome with a consistent format.
pub fn render_tap_log(tap: &StartupTraceTap) -> String {
    if tap.special_envelope {
        return "trace_tap=special_envelope".to_string();
    }
    match (tap.traceparent(), tap.tracestate(), tap.application_name()) {
        (Some(traceparent), Some(state), Some(name)) => format!(
            "trace_tap=present application_name={name} traceparent={traceparent} tracestate={}",
            state.as_str(),
        ),
        (Some(traceparent), None, Some(name)) => {
            format!("trace_tap=present application_name={name} traceparent={traceparent}")
        }
        (Some(traceparent), _, None) => {
            format!("trace_tap=present application_name=<none> traceparent={traceparent}")
        }
        (None, _, Some(name)) => {
            format!("trace_tap=absent application_name={name}")
        }
        (None, _, None) => "trace_tap=absent application_name=<none>".to_string(),
    }
}

/// Minimum trace-tap timeout. Used by callers that wrap the tap in a
/// `TcpStream::set_read_timeout` to bound how long they wait for the client
/// to finish the startup envelope.
pub const STARTUP_TAP_MIN_TIMEOUT: Duration = Duration::from_millis(500);

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn make_startup_packet_with_options(application_name: &str, options: Option<&str>) -> Vec<u8> {
        let mut body = Vec::new();
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
        if let Some(options) = options {
            body.extend_from_slice(b"options");
            body.push(0);
            body.extend_from_slice(options.as_bytes());
            body.push(0);
        }
        body.push(0);
        let length = (body.len() + 4) as u32;
        let mut packet = Vec::with_capacity(body.len() + 4);
        packet.extend_from_slice(&length.to_be_bytes());
        packet.extend_from_slice(&body);
        packet
    }

    fn make_startup_packet(application_name: &str) -> Vec<u8> {
        make_startup_packet_with_options(application_name, None)
    }

    const TRACEPARENT: &str = "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01";

    #[test]
    fn tap_extracts_traceparent_from_application_name() {
        let application_name = format!(
            "application=ai_blaise_pipeline;traceparent={TRACEPARENT};tracestate=vendor=opaque"
        );
        let packet = make_startup_packet(&application_name);
        let mut cursor = Cursor::new(packet.clone());

        let tap = tap_startup_message(&mut cursor).unwrap();
        assert_eq!(tap.buffered_bytes, packet);
        assert!(!tap.special_envelope);
        assert_eq!(
            tap.traceparent()
                .map(TraceParent::to_header_value)
                .as_deref(),
            Some(TRACEPARENT),
        );
        assert_eq!(tap.application_name(), Some("ai_blaise_pipeline"));
        assert_eq!(
            tap.tracestate()
                .map(|state| state.as_str().to_string())
                .as_deref(),
            Some("vendor=opaque"),
        );
    }

    #[test]
    fn tap_returns_no_traceparent_for_plain_application_name() {
        let packet = make_startup_packet("psql");
        let mut cursor = Cursor::new(packet.clone());
        let tap = tap_startup_message(&mut cursor).unwrap();
        assert_eq!(tap.buffered_bytes, packet);
        assert!(tap.traceparent().is_none());
        assert_eq!(tap.application_name(), Some("psql"));
    }

    #[test]
    fn tap_passes_through_ssl_request_envelope_without_traceparent() {
        let mut packet = Vec::new();
        packet.extend_from_slice(&8_u32.to_be_bytes());
        packet.extend_from_slice(&80877103_u32.to_be_bytes());
        let mut cursor = Cursor::new(packet.clone());

        let tap = tap_startup_message(&mut cursor).unwrap();
        assert_eq!(tap.buffered_bytes, packet);
        assert!(tap.special_envelope);
        assert!(tap.traceparent().is_none());
    }

    #[test]
    fn tap_rejects_invalid_startup_length() {
        let mut packet = Vec::new();
        packet.extend_from_slice(&3_u32.to_be_bytes());
        packet.extend_from_slice(&196608_u32.to_be_bytes());
        let mut cursor = Cursor::new(packet);

        let error = tap_startup_message(&mut cursor).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    }

    #[test]
    fn sanitized_startup_bytes_strip_pool_auth_parameters() {
        let mut packet = Vec::new();
        let mut body = Vec::new();
        body.extend_from_slice(&196608_u32.to_be_bytes());
        for (key, value) in [
            ("user", "postgres"),
            ("database", "postgres"),
            ("application_name", "pool_auth_smoke"),
            ("ai_blaise.jwt", "secret-token"),
            ("ai_blaise.tenant_id", "tenant-a"),
        ] {
            body.extend_from_slice(key.as_bytes());
            body.push(0);
            body.extend_from_slice(value.as_bytes());
            body.push(0);
        }
        body.push(0);
        packet.extend_from_slice(&((body.len() + 4) as u32).to_be_bytes());
        packet.extend_from_slice(&body);

        let mut cursor = Cursor::new(packet);
        let tap = tap_startup_message(&mut cursor).unwrap();
        let sanitized = tap.sanitized_startup_bytes();
        assert!(!String::from_utf8_lossy(&sanitized).contains("secret-token"));
        assert!(!String::from_utf8_lossy(&sanitized).contains("tenant-a"));
        assert!(String::from_utf8_lossy(&sanitized).contains("pool_auth_smoke"));
    }

    #[test]
    fn render_tap_log_present_includes_traceparent_token() {
        let application_name = format!("application=svc;traceparent={TRACEPARENT}");
        let packet = make_startup_packet(&application_name);
        let mut cursor = Cursor::new(packet);
        let tap = tap_startup_message(&mut cursor).unwrap();
        let rendered = render_tap_log(&tap);
        assert!(rendered.starts_with("trace_tap=present"));
        assert!(rendered.contains(TRACEPARENT));
    }

    #[test]
    fn tap_extracts_traceparent_from_libpq_options() {
        const TRACEPARENT: &str = "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01";
        let options = format!("-c trace.parent={TRACEPARENT} -c trace.state=vendor=ai-blaise");
        let packet = make_startup_packet_with_options("ai_blaise_otel_smoke", Some(&options));
        let mut cursor = Cursor::new(packet.clone());
        let tap = tap_startup_message(&mut cursor).unwrap();
        assert_eq!(tap.buffered_bytes, packet);
        assert!(!tap.special_envelope);
        assert_eq!(
            tap.traceparent()
                .map(TraceParent::to_header_value)
                .as_deref(),
            Some(TRACEPARENT),
        );
        assert_eq!(
            tap.tracestate()
                .map(|state| state.as_str().to_string())
                .as_deref(),
            Some("vendor=ai-blaise"),
        );
    }

    #[test]
    fn tap_ignores_options_without_trace_parent_assignment() {
        let options = "-c statement_timeout=5000 -c application_name=svc";
        let packet = make_startup_packet_with_options("svc", Some(options));
        let mut cursor = Cursor::new(packet);
        let tap = tap_startup_message(&mut cursor).unwrap();
        assert!(tap.traceparent().is_none());
        assert!(tap.tracestate().is_none());
    }

    #[test]
    fn render_tap_log_absent_when_no_traceparent_in_application_name() {
        let packet = make_startup_packet("psql");
        let mut cursor = Cursor::new(packet);
        let tap = tap_startup_message(&mut cursor).unwrap();
        let rendered = render_tap_log(&tap);
        assert_eq!(rendered, "trace_tap=absent application_name=psql");
    }

    #[test]
    fn tap_prefers_dedicated_traceparent_over_options_and_application_name() {
        let app_traceparent = TRACEPARENT;
        let options_traceparent = "00-aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa-bbbbbbbbbbbbbbbb-01";
        let dedicated_traceparent = "00-cccccccccccccccccccccccccccccccc-dddddddddddddddd-01";
        let application_name =
            format!("application=svc;traceparent={app_traceparent};tracestate=app=1");
        let options = format!("-c trace.parent={options_traceparent} -c trace.state=options=1");

        let mut body = Vec::new();
        body.extend_from_slice(&196608_u32.to_be_bytes());
        for (key, value) in [
            ("user", "postgres"),
            ("database", "postgres"),
            ("application_name", application_name.as_str()),
            ("options", options.as_str()),
            ("traceparent", dedicated_traceparent),
            ("tracestate", "dedicated=1"),
        ] {
            body.extend_from_slice(key.as_bytes());
            body.push(0);
            body.extend_from_slice(value.as_bytes());
            body.push(0);
        }
        body.push(0);
        let mut packet = Vec::new();
        packet.extend_from_slice(&((body.len() + 4) as u32).to_be_bytes());
        packet.extend_from_slice(&body);

        let mut cursor = Cursor::new(packet);
        let tap = tap_startup_message(&mut cursor).unwrap();
        assert_eq!(
            tap.traceparent()
                .map(TraceParent::to_header_value)
                .as_deref(),
            Some(dedicated_traceparent),
        );
        assert_eq!(
            tap.tracestate()
                .map(|state| state.as_str().to_string())
                .as_deref(),
            Some("dedicated=1"),
        );
        assert_eq!(tap.application_name(), Some("svc"));
    }

    #[test]
    fn tap_rejects_corrupt_dedicated_traceparent_but_preserves_startup() {
        let mut body = Vec::new();
        body.extend_from_slice(&196608_u32.to_be_bytes());
        for (key, value) in [
            ("user", "postgres"),
            ("database", "postgres"),
            ("application_name", "svc"),
            ("traceparent", "not-a-traceparent"),
        ] {
            body.extend_from_slice(key.as_bytes());
            body.push(0);
            body.extend_from_slice(value.as_bytes());
            body.push(0);
        }
        body.push(0);
        let mut packet = Vec::new();
        packet.extend_from_slice(&((body.len() + 4) as u32).to_be_bytes());
        packet.extend_from_slice(&body);

        let mut cursor = Cursor::new(packet.clone());
        let tap = tap_startup_message(&mut cursor).unwrap();
        assert_eq!(tap.buffered_bytes, packet);
        assert!(tap.traceparent().is_none());
        assert_eq!(tap.application_name(), Some("svc"));
    }
}
