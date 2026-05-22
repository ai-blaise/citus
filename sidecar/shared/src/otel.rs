//! W3C trace-context propagation primitives shared across pool, companion,
//! sidecars, and operator HTTP/gRPC surfaces.
//!
//! The module is intentionally dependency-free. It models the W3C
//! `traceparent` header from <https://www.w3.org/TR/trace-context/> and
//! exposes a `TraceContext` extract/inject trait that any carrier in
//! `ai-blaise/citus` can implement without pulling `http`, `tonic`,
//! `opentracing`, or `opentelemetry` into the workspace dependency graph.
//!
//! Three concrete carriers are provided in-crate:
//!
//! * `HeaderMap` — a deterministic case-insensitive header map backed by a
//!   `BTreeMap`, suitable for HTTP request/response carriers (axum, reqwest)
//!   when those surfaces are wired up. Sidecar HTTP handlers wrap their real
//!   carrier in `HeaderMap::from_iter` and call `TraceContext::extract` to
//!   recover the inbound traceparent, then `inject` on the outbound carrier.
//! * `MetadataMap` — the same map shape with lower-cased ASCII keys for gRPC
//!   metadata pairs. Tonic interceptors translate `tonic::metadata::MetadataMap`
//!   into this carrier before extract/inject.
//! * `SetLocalBuilder` — a PostgreSQL `SET LOCAL trace.parent = '<value>'`
//!   statement builder used by the pool proxy to inject the traceparent into
//!   the database session before forwarding query traffic.
//!
//! The extraction path also understands the libpq `application_name` field
//! produced by `ai-blaise` clients, which embeds the traceparent as
//! `application=<original>;traceparent=<value>`. The pool extracts that
//! token in startup-message handling and forwards the traceparent into the
//! session via `SetLocalBuilder`, so that companion-extension pgrx code can
//! read it back with `current_setting('trace.parent', true)` and emit a
//! child span anchored to the same trace.

// FEATURE: O14

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

/// Maximum number of bytes allowed in a single traceparent header. The W3C
/// spec fixes the length at 55 ASCII characters for version `00`.
pub const TRACEPARENT_MAX_LEN: usize = 55;

/// The canonical lower-cased traceparent header name.
pub const TRACEPARENT_HEADER: &str = "traceparent";

/// The canonical lower-cased tracestate header name. The carrier preserves an
/// opaque tracestate string but the propagation logic only validates the
/// traceparent portion.
pub const TRACESTATE_HEADER: &str = "tracestate";

/// The PostgreSQL GUC under which the pool injects the inbound traceparent.
/// Companion pgrx code reads this back via
/// `current_setting('trace.parent', true)`.
pub const PG_TRACE_PARENT_GUC: &str = "trace.parent";

/// Application-name extension key under which `ai-blaise` clients embed the
/// traceparent. The full application name is parsed as
/// `application=<name>;traceparent=<value>;tracestate=<value>`.
pub const APP_NAME_TRACEPARENT_KEY: &str = "traceparent";

/// Application-name extension key under which clients embed the tracestate.
pub const APP_NAME_TRACESTATE_KEY: &str = "tracestate";

/// Application-name extension key under which clients embed the original
/// application name. When absent, the entire application_name string is
/// treated as the original.
pub const APP_NAME_APPLICATION_KEY: &str = "application";

/// Parsed W3C traceparent value. Fields follow
/// <https://www.w3.org/TR/trace-context/#traceparent-header>.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TraceParent {
    version: [u8; 1],
    trace_id: [u8; 16],
    parent_id: [u8; 8],
    flags: [u8; 1],
}

impl TraceParent {
    /// Parse a traceparent value. The W3C spec only standardizes version
    /// `00`; later versions MUST round-trip the same 55-byte structure.
    pub fn parse(value: &str) -> Result<Self, TraceContextError> {
        if value.len() != TRACEPARENT_MAX_LEN {
            return Err(TraceContextError::InvalidTraceparentLength(value.len()));
        }
        if !value.is_ascii() {
            return Err(TraceContextError::InvalidTraceparentBytes);
        }

        let parts: Vec<&str> = value.split('-').collect();
        if parts.len() != 4 {
            return Err(TraceContextError::InvalidTraceparentShape);
        }
        let (version_text, trace_id_text, parent_id_text, flags_text) =
            (parts[0], parts[1], parts[2], parts[3]);
        if version_text.len() != 2
            || trace_id_text.len() != 32
            || parent_id_text.len() != 16
            || flags_text.len() != 2
        {
            return Err(TraceContextError::InvalidTraceparentShape);
        }

        let version = decode_hex_fixed::<1>(version_text)?;
        let trace_id = decode_hex_fixed::<16>(trace_id_text)?;
        let parent_id = decode_hex_fixed::<8>(parent_id_text)?;
        let flags = decode_hex_fixed::<1>(flags_text)?;

        if version[0] == 0xff {
            return Err(TraceContextError::ReservedVersion);
        }
        if trace_id.iter().all(|byte| *byte == 0) {
            return Err(TraceContextError::ZeroTraceId);
        }
        if parent_id.iter().all(|byte| *byte == 0) {
            return Err(TraceContextError::ZeroParentId);
        }

        Ok(Self {
            version,
            trace_id,
            parent_id,
            flags,
        })
    }

    /// Construct a traceparent from raw bytes. Used by the pool's PostgreSQL
    /// startup-message handler when the application_name carries a parsed
    /// traceparent it has already validated.
    pub fn from_parts(
        trace_id: [u8; 16],
        parent_id: [u8; 8],
        flags: u8,
    ) -> Result<Self, TraceContextError> {
        if trace_id.iter().all(|byte| *byte == 0) {
            return Err(TraceContextError::ZeroTraceId);
        }
        if parent_id.iter().all(|byte| *byte == 0) {
            return Err(TraceContextError::ZeroParentId);
        }
        Ok(Self {
            version: [0],
            trace_id,
            parent_id,
            flags: [flags],
        })
    }

    /// Hex-encoded trace id (32 chars).
    pub fn trace_id_hex(&self) -> String {
        encode_hex(&self.trace_id)
    }

    /// Hex-encoded parent (span) id (16 chars).
    pub fn parent_id_hex(&self) -> String {
        encode_hex(&self.parent_id)
    }

    /// Sampled flag from the trace-flags byte. Bit 0 is the sampled bit per
    /// W3C trace-context.
    pub fn sampled(&self) -> bool {
        self.flags[0] & 0x01 != 0
    }

    /// Re-serialize the traceparent into its canonical 55-byte form.
    pub fn to_header_value(&self) -> String {
        format!(
            "{}-{}-{}-{}",
            encode_hex(&self.version),
            encode_hex(&self.trace_id),
            encode_hex(&self.parent_id),
            encode_hex(&self.flags),
        )
    }
}

impl fmt::Display for TraceParent {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.to_header_value())
    }
}

/// Optional opaque tracestate value paired with a parent. The propagator
/// preserves it verbatim and never inspects its contents.
#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct TraceState(String);

impl TraceState {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// Extract from / inject into a carrier.
pub trait TraceContext {
    /// Extract a traceparent (and optional tracestate) from the carrier.
    fn extract(&self) -> Option<(TraceParent, TraceState)>;

    /// Inject a traceparent into the carrier, replacing any existing value.
    /// If `state` is non-empty it is also injected under the tracestate key.
    fn inject(&mut self, traceparent: &TraceParent, state: &TraceState);
}

/// Case-insensitive header map. Keys are stored lower-cased; lookups always
/// lower-case the input. Suitable for HTTP request carriers.
#[derive(Debug, Clone, Default, Eq, PartialEq)]
pub struct HeaderMap {
    entries: BTreeMap<String, String>,
}

impl HeaderMap {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, name: &str, value: impl Into<String>) {
        self.entries.insert(name.to_ascii_lowercase(), value.into());
    }

    pub fn get(&self, name: &str) -> Option<&str> {
        self.entries
            .get(&name.to_ascii_lowercase())
            .map(String::as_str)
    }

    pub fn remove(&mut self, name: &str) -> Option<String> {
        self.entries.remove(&name.to_ascii_lowercase())
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.entries
            .iter()
            .map(|(name, value)| (name.as_str(), value.as_str()))
    }
}

impl FromIterator<(String, String)> for HeaderMap {
    fn from_iter<I: IntoIterator<Item = (String, String)>>(iter: I) -> Self {
        let mut map = Self::new();
        for (name, value) in iter {
            map.insert(&name, value);
        }
        map
    }
}

impl TraceContext for HeaderMap {
    fn extract(&self) -> Option<(TraceParent, TraceState)> {
        let raw = self.get(TRACEPARENT_HEADER)?;
        let traceparent = TraceParent::parse(raw).ok()?;
        let state = self
            .get(TRACESTATE_HEADER)
            .map(TraceState::new)
            .unwrap_or_default();
        Some((traceparent, state))
    }

    fn inject(&mut self, traceparent: &TraceParent, state: &TraceState) {
        self.insert(TRACEPARENT_HEADER, traceparent.to_header_value());
        if state.is_empty() {
            self.remove(TRACESTATE_HEADER);
        } else {
            self.insert(TRACESTATE_HEADER, state.as_str().to_string());
        }
    }
}

/// gRPC metadata carrier. gRPC metadata keys are ASCII-lower-cased by the
/// HTTP/2 layer; this map mirrors that contract.
#[derive(Debug, Clone, Default, Eq, PartialEq)]
pub struct MetadataMap {
    entries: BTreeMap<String, String>,
}

impl MetadataMap {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, name: &str, value: impl Into<String>) {
        self.entries.insert(name.to_ascii_lowercase(), value.into());
    }

    pub fn get(&self, name: &str) -> Option<&str> {
        self.entries
            .get(&name.to_ascii_lowercase())
            .map(String::as_str)
    }

    pub fn remove(&mut self, name: &str) -> Option<String> {
        self.entries.remove(&name.to_ascii_lowercase())
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.entries
            .iter()
            .map(|(name, value)| (name.as_str(), value.as_str()))
    }
}

impl TraceContext for MetadataMap {
    fn extract(&self) -> Option<(TraceParent, TraceState)> {
        let raw = self.get(TRACEPARENT_HEADER)?;
        let traceparent = TraceParent::parse(raw).ok()?;
        let state = self
            .get(TRACESTATE_HEADER)
            .map(TraceState::new)
            .unwrap_or_default();
        Some((traceparent, state))
    }

    fn inject(&mut self, traceparent: &TraceParent, state: &TraceState) {
        self.insert(TRACEPARENT_HEADER, traceparent.to_header_value());
        if state.is_empty() {
            self.remove(TRACESTATE_HEADER);
        } else {
            self.insert(TRACESTATE_HEADER, state.as_str().to_string());
        }
    }
}

/// PostgreSQL `SET LOCAL` builder used to inject the traceparent into a
/// session GUC visible to companion pgrx code via `current_setting`.
#[derive(Debug, Clone, Default, Eq, PartialEq)]
pub struct SetLocalBuilder {
    traceparent: Option<String>,
    tracestate: Option<String>,
}

impl SetLocalBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Render the `SET LOCAL` statement sequence that injects the traceparent
    /// into the session. Returns `None` when no traceparent has been
    /// injected. The output is safe to send as a single simple-query frame
    /// because all literals are single-quoted with embedded quotes escaped
    /// via PostgreSQL's doubled-quote convention.
    pub fn render(&self) -> Option<String> {
        let traceparent = self.traceparent.as_ref()?;
        let mut statement = format!(
            "SET LOCAL trace.parent = '{}'",
            escape_sql_literal(traceparent)
        );
        if let Some(state) = &self.tracestate {
            statement.push_str(&format!(
                "; SET LOCAL trace.state = '{}'",
                escape_sql_literal(state)
            ));
        }
        Some(statement)
    }

    pub fn traceparent(&self) -> Option<&str> {
        self.traceparent.as_deref()
    }

    pub fn tracestate(&self) -> Option<&str> {
        self.tracestate.as_deref()
    }
}

impl TraceContext for SetLocalBuilder {
    fn extract(&self) -> Option<(TraceParent, TraceState)> {
        let raw = self.traceparent.as_ref()?;
        let traceparent = TraceParent::parse(raw).ok()?;
        let state = self
            .tracestate
            .as_ref()
            .map(|value| TraceState::new(value.clone()))
            .unwrap_or_default();
        Some((traceparent, state))
    }

    fn inject(&mut self, traceparent: &TraceParent, state: &TraceState) {
        self.traceparent = Some(traceparent.to_header_value());
        self.tracestate = if state.is_empty() {
            None
        } else {
            Some(state.as_str().to_string())
        };
    }
}

/// Parse the libpq `application_name` startup parameter for an embedded
/// traceparent.
///
/// The wire format is a semicolon-separated list of `key=value` pairs:
///
/// ```text
/// application=<name>;traceparent=<00-...>;tracestate=<...>
/// ```
///
/// When `application_name` contains no `=` sign the entire value is treated
/// as the original application name and no traceparent is returned. The
/// original-application portion is preserved separately so the pool can
/// forward it to PostgreSQL via a rewritten startup parameter.
pub fn parse_application_name(value: &str) -> ApplicationNameFields {
    let mut fields = ApplicationNameFields::default();
    if !value.contains('=') {
        fields.application = Some(value.to_string());
        return fields;
    }
    for pair in value.split(';') {
        let trimmed = pair.trim();
        if trimmed.is_empty() {
            continue;
        }
        let Some((key, raw_value)) = trimmed.split_once('=') else {
            // Tokens without `=` after the first pair are preserved as part
            // of the application name to round-trip names that legitimately
            // contain semicolons.
            if fields.application.is_none() {
                fields.application = Some(trimmed.to_string());
            }
            continue;
        };
        let key = key.trim();
        let raw_value = raw_value.trim();
        match key {
            APP_NAME_APPLICATION_KEY => {
                fields.application = Some(raw_value.to_string());
            }
            APP_NAME_TRACEPARENT_KEY => {
                if let Ok(traceparent) = TraceParent::parse(raw_value) {
                    fields.traceparent = Some(traceparent);
                }
            }
            APP_NAME_TRACESTATE_KEY => {
                fields.tracestate = Some(TraceState::new(raw_value));
            }
            _ => {}
        }
    }
    fields
}

/// Result of parsing an `application_name` startup parameter.
#[derive(Debug, Clone, Default, Eq, PartialEq)]
pub struct ApplicationNameFields {
    pub application: Option<String>,
    pub traceparent: Option<TraceParent>,
    pub tracestate: Option<TraceState>,
}

impl ApplicationNameFields {
    /// Build a `SetLocalBuilder` for the parsed traceparent, if any.
    pub fn to_set_local_builder(&self) -> SetLocalBuilder {
        let mut builder = SetLocalBuilder::new();
        if let Some(traceparent) = &self.traceparent {
            let state = self.tracestate.clone().unwrap_or_default();
            builder.inject(traceparent, &state);
        }
        builder
    }

    /// Reduce the parsed fields back to the original application name, with
    /// the traceparent/tracestate stripped. The pool rewrites the inbound
    /// startup message to use this value so PostgreSQL log lines do not
    /// embed the traceparent.
    pub fn original_application_name(&self) -> &str {
        self.application.as_deref().unwrap_or("")
    }
}

/// Errors produced by traceparent parsing.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum TraceContextError {
    InvalidTraceparentLength(usize),
    InvalidTraceparentBytes,
    InvalidTraceparentShape,
    InvalidHexDigit(char),
    ReservedVersion,
    ZeroTraceId,
    ZeroParentId,
}

impl fmt::Display for TraceContextError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidTraceparentLength(length) => {
                write!(
                    formatter,
                    "traceparent must be exactly {TRACEPARENT_MAX_LEN} bytes, got {length}"
                )
            }
            Self::InvalidTraceparentBytes => {
                write!(formatter, "traceparent must be 7-bit ASCII")
            }
            Self::InvalidTraceparentShape => {
                write!(
                    formatter,
                    "traceparent must contain four '-'-separated fields"
                )
            }
            Self::InvalidHexDigit(character) => {
                write!(formatter, "invalid hex digit '{character}' in traceparent")
            }
            Self::ReservedVersion => {
                write!(formatter, "traceparent version 0xff is reserved")
            }
            Self::ZeroTraceId => {
                write!(formatter, "traceparent trace-id must not be all zeros")
            }
            Self::ZeroParentId => {
                write!(formatter, "traceparent parent-id must not be all zeros")
            }
        }
    }
}

impl Error for TraceContextError {}

fn decode_hex_fixed<const N: usize>(text: &str) -> Result<[u8; N], TraceContextError> {
    if text.len() != N * 2 {
        return Err(TraceContextError::InvalidTraceparentShape);
    }
    let mut bytes = [0_u8; N];
    let chars: Vec<char> = text.chars().collect();
    for (index, byte) in bytes.iter_mut().enumerate() {
        let high = hex_digit(chars[index * 2])?;
        let low = hex_digit(chars[index * 2 + 1])?;
        *byte = (high << 4) | low;
    }
    Ok(bytes)
}

fn hex_digit(character: char) -> Result<u8, TraceContextError> {
    match character {
        '0'..='9' => Ok((character as u8) - b'0'),
        'a'..='f' => Ok((character as u8) - b'a' + 10),
        'A'..='F' => Ok((character as u8) - b'A' + 10),
        _ => Err(TraceContextError::InvalidHexDigit(character)),
    }
}

fn encode_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}

fn escape_sql_literal(value: &str) -> String {
    value.replace('\'', "''")
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_TRACEPARENT: &str = "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01";

    #[test]
    fn traceparent_round_trips_canonical_form() {
        let traceparent = TraceParent::parse(SAMPLE_TRACEPARENT).unwrap();
        assert_eq!(traceparent.to_header_value(), SAMPLE_TRACEPARENT);
        assert_eq!(
            traceparent.trace_id_hex(),
            "4bf92f3577b34da6a3ce929d0e0e4736"
        );
        assert_eq!(traceparent.parent_id_hex(), "00f067aa0ba902b7");
        assert!(traceparent.sampled());
    }

    #[test]
    fn traceparent_rejects_wrong_length() {
        let too_short = "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7";
        assert_eq!(
            TraceParent::parse(too_short).unwrap_err(),
            TraceContextError::InvalidTraceparentLength(too_short.len())
        );
    }

    #[test]
    fn traceparent_rejects_zero_ids() {
        let zero_trace = "00-00000000000000000000000000000000-00f067aa0ba902b7-01";
        assert_eq!(
            TraceParent::parse(zero_trace).unwrap_err(),
            TraceContextError::ZeroTraceId
        );

        let zero_parent = "00-4bf92f3577b34da6a3ce929d0e0e4736-0000000000000000-01";
        assert_eq!(
            TraceParent::parse(zero_parent).unwrap_err(),
            TraceContextError::ZeroParentId
        );
    }

    #[test]
    fn traceparent_rejects_reserved_version() {
        let reserved = "ff-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01";
        assert_eq!(
            TraceParent::parse(reserved).unwrap_err(),
            TraceContextError::ReservedVersion
        );
    }

    #[test]
    fn traceparent_rejects_invalid_hex() {
        let bad = "00-4bf92f3577b34da6a3ce929d0e0e473g-00f067aa0ba902b7-01";
        assert_eq!(
            TraceParent::parse(bad).unwrap_err(),
            TraceContextError::InvalidHexDigit('g')
        );
    }

    #[test]
    fn header_map_extracts_traceparent_case_insensitive() {
        let mut map = HeaderMap::new();
        map.insert("TraceParent", SAMPLE_TRACEPARENT);
        map.insert("TraceState", "vendor=opaque");
        let (traceparent, state) = map.extract().unwrap();
        assert_eq!(traceparent.to_header_value(), SAMPLE_TRACEPARENT);
        assert_eq!(state.as_str(), "vendor=opaque");
    }

    #[test]
    fn header_map_inject_overwrites_existing_state() {
        let mut map = HeaderMap::new();
        map.insert("traceparent", "ignored");
        map.insert("tracestate", "stale=1");
        let parent = TraceParent::parse(SAMPLE_TRACEPARENT).unwrap();
        map.inject(&parent, &TraceState::new("fresh=2"));
        assert_eq!(map.get("traceparent"), Some(SAMPLE_TRACEPARENT));
        assert_eq!(map.get("tracestate"), Some("fresh=2"));

        let mut empty_state = HeaderMap::new();
        empty_state.insert("tracestate", "stale=1");
        empty_state.inject(&parent, &TraceState::default());
        assert_eq!(empty_state.get("tracestate"), None);
    }

    #[test]
    fn metadata_map_mirrors_header_map_behavior() {
        let mut metadata = MetadataMap::new();
        let parent = TraceParent::parse(SAMPLE_TRACEPARENT).unwrap();
        metadata.inject(&parent, &TraceState::new("vendor=opaque"));
        let (recovered, state) = metadata.extract().unwrap();
        assert_eq!(recovered, parent);
        assert_eq!(state.as_str(), "vendor=opaque");
    }

    #[test]
    fn metadata_map_rejects_corrupted_traceparent() {
        let mut metadata = MetadataMap::new();
        metadata.insert(TRACEPARENT_HEADER, "not-a-traceparent");
        assert!(metadata.extract().is_none());
    }

    #[test]
    fn set_local_builder_renders_pg_statement() {
        let mut builder = SetLocalBuilder::new();
        let parent = TraceParent::parse(SAMPLE_TRACEPARENT).unwrap();
        builder.inject(&parent, &TraceState::new("vendor=o'paque"));
        let rendered = builder.render().unwrap();
        assert!(rendered.starts_with(&format!("SET LOCAL trace.parent = '{SAMPLE_TRACEPARENT}'")));
        // PostgreSQL escapes single quotes by doubling them.
        assert!(rendered.contains("SET LOCAL trace.state = 'vendor=o''paque'"));
    }

    #[test]
    fn set_local_builder_skips_when_no_traceparent() {
        let builder = SetLocalBuilder::new();
        assert!(builder.render().is_none());
    }

    #[test]
    fn parse_application_name_returns_traceparent_and_strips_name() {
        let raw = format!(
            "application=ai_blaise_pipeline_smoke;traceparent={SAMPLE_TRACEPARENT};tracestate=vendor=opaque"
        );
        let fields = parse_application_name(&raw);
        assert_eq!(
            fields.application.as_deref(),
            Some("ai_blaise_pipeline_smoke")
        );
        assert!(fields.traceparent.is_some());
        assert_eq!(
            fields.tracestate.as_ref().map(TraceState::as_str),
            Some("vendor=opaque")
        );

        let builder = fields.to_set_local_builder();
        let rendered = builder.render().unwrap();
        assert!(rendered.contains("SET LOCAL trace.parent"));
    }

    #[test]
    fn parse_application_name_handles_legacy_plain_name() {
        let fields = parse_application_name("plain_app_no_traceparent");
        assert_eq!(
            fields.application.as_deref(),
            Some("plain_app_no_traceparent")
        );
        assert!(fields.traceparent.is_none());
        assert!(fields.to_set_local_builder().render().is_none());
    }

    #[test]
    fn parse_application_name_rejects_corrupted_traceparent_token() {
        let fields = parse_application_name(
            "application=svc;traceparent=00-bad-token-here;tracestate=keep=1",
        );
        assert_eq!(fields.application.as_deref(), Some("svc"));
        assert!(fields.traceparent.is_none());
        assert_eq!(
            fields.tracestate.as_ref().map(TraceState::as_str),
            Some("keep=1")
        );
    }
}
