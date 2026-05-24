// FEATURE: Auth3

//! Live pool-side token introspection gate.
//!
//! The proxy remains PostgreSQL byte-transparent after admission, but this
//! module lets admission require a JWT/access token in the startup envelope and
//! verify it against the auth sidecar's `/auth/introspect` endpoint before any
//! backend socket is opened. The cache is deliberately opt-in via
//! `AI_BLAISE_POOL_AUTH_CACHE_TTL_MS`; the production default revalidates every
//! startup so revocation fails closed through the auth sidecar cache.

use crate::trace_tap::StartupTraceTap;
use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::Mutex;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const DEFAULT_AUTH_TIMEOUT: Duration = Duration::from_millis(750);
const DEFAULT_CACHE_TTL: Duration = Duration::ZERO;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PoolAuthConfig {
    pub introspection_url: String,
    pub timeout: Duration,
    pub cache_ttl: Duration,
    pub require_tenant_match: bool,
}

impl PoolAuthConfig {
    pub fn from_env() -> Result<Option<Self>, PoolAuthError> {
        let Some(introspection_url) = std::env::var("AI_BLAISE_POOL_AUTH_INTROSPECTION_URL")
            .ok()
            .filter(|value| !value.trim().is_empty())
        else {
            return Ok(None);
        };
        let timeout = parse_duration_ms_env(
            "AI_BLAISE_POOL_AUTH_TIMEOUT_MS",
            DEFAULT_AUTH_TIMEOUT,
            "positive integer milliseconds",
        )?;
        let cache_ttl = parse_duration_ms_env(
            "AI_BLAISE_POOL_AUTH_CACHE_TTL_MS",
            DEFAULT_CACHE_TTL,
            "non-negative integer milliseconds",
        )?;
        let require_tenant_match = std::env::var("AI_BLAISE_POOL_AUTH_REQUIRE_TENANT_MATCH")
            .map(|value| value != "0" && !value.eq_ignore_ascii_case("false"))
            .unwrap_or(true);
        let config = Self {
            introspection_url,
            timeout,
            cache_ttl,
            require_tenant_match,
        };
        config.validate()?;
        Ok(Some(config))
    }

    pub fn validate(&self) -> Result<(), PoolAuthError> {
        ParsedHttpUrl::parse(&self.introspection_url)?;
        if self.timeout.is_zero() {
            return Err(PoolAuthError::InvalidEnv {
                name: "AI_BLAISE_POOL_AUTH_TIMEOUT_MS",
                value: "0".to_string(),
                expected: "positive integer milliseconds",
            });
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct PoolAuthGate {
    config: PoolAuthConfig,
    cache: Mutex<BTreeMap<String, CachedIntrospection>>,
}

impl PoolAuthGate {
    pub fn new(config: PoolAuthConfig) -> Result<Self, PoolAuthError> {
        config.validate()?;
        Ok(Self {
            config,
            cache: Mutex::new(BTreeMap::new()),
        })
    }

    pub fn config(&self) -> &PoolAuthConfig {
        &self.config
    }

    pub fn authorize_startup(
        &self,
        tap: &StartupTraceTap,
    ) -> Result<PoolAuthDecision, PoolAuthError> {
        let token = token_from_startup(tap).ok_or(PoolAuthError::MissingToken)?;
        let startup_tenant = tenant_id_from_startup(tap);
        let now = Instant::now();
        let token_key = cache_key(&token);

        if !self.config.cache_ttl.is_zero() {
            if let Some(claims) = self.cached_claims(&token_key, now)? {
                self.validate_tenant(&claims, startup_tenant.as_deref())?;
                return Ok(PoolAuthDecision {
                    cache_hit: true,
                    tenant_id: claims.tenant_id,
                    subject: claims.subject,
                    jwt_id: claims.jwt_id,
                });
            }
        }

        let response = introspect_token(&self.config, &token)?;
        if !response.active {
            self.evict(&token_key)?;
            return Err(PoolAuthError::Inactive(response.reason.unwrap_or_else(
                || "token introspection returned active=false".to_string(),
            )));
        }
        let claims = response.claims.ok_or(PoolAuthError::MalformedResponse(
            "active introspection response omitted claims".to_string(),
        ))?;
        self.validate_tenant(&claims, startup_tenant.as_deref())?;

        if !self.config.cache_ttl.is_zero() {
            self.cache_claims(token_key, claims.clone(), now)?;
        }

        Ok(PoolAuthDecision {
            cache_hit: false,
            tenant_id: claims.tenant_id,
            subject: claims.subject,
            jwt_id: claims.jwt_id,
        })
    }

    fn cached_claims(
        &self,
        token_key: &str,
        now: Instant,
    ) -> Result<Option<VerifiedPoolClaims>, PoolAuthError> {
        let mut cache = self
            .cache
            .lock()
            .map_err(|_| PoolAuthError::StatePoisoned("auth_cache"))?;
        let Some(entry) = cache.get(token_key).cloned() else {
            return Ok(None);
        };
        if now.duration_since(entry.inserted_at) >= self.config.cache_ttl
            || entry.claims.is_expired_at(current_unix_time())
        {
            cache.remove(token_key);
            return Ok(None);
        }
        Ok(Some(entry.claims))
    }

    fn cache_claims(
        &self,
        token_key: String,
        claims: VerifiedPoolClaims,
        now: Instant,
    ) -> Result<(), PoolAuthError> {
        if claims.is_expired_at(current_unix_time()) {
            return Ok(());
        }
        let mut cache = self
            .cache
            .lock()
            .map_err(|_| PoolAuthError::StatePoisoned("auth_cache"))?;
        cache.insert(
            token_key,
            CachedIntrospection {
                claims,
                inserted_at: now,
            },
        );
        Ok(())
    }

    fn evict(&self, token_key: &str) -> Result<(), PoolAuthError> {
        let mut cache = self
            .cache
            .lock()
            .map_err(|_| PoolAuthError::StatePoisoned("auth_cache"))?;
        cache.remove(token_key);
        Ok(())
    }

    fn validate_tenant(
        &self,
        claims: &VerifiedPoolClaims,
        startup_tenant: Option<&str>,
    ) -> Result<(), PoolAuthError> {
        if !self.config.require_tenant_match {
            return Ok(());
        }
        if let Some(startup_tenant) = startup_tenant {
            if startup_tenant != claims.tenant_id {
                return Err(PoolAuthError::TenantMismatch {
                    startup_tenant: startup_tenant.to_string(),
                    token_tenant: claims.tenant_id.clone(),
                });
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PoolAuthDecision {
    pub cache_hit: bool,
    pub tenant_id: String,
    pub subject: String,
    pub jwt_id: String,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct VerifiedPoolClaims {
    pub jwt_id: String,
    pub tenant_id: String,
    pub subject: String,
    pub role: String,
    pub expires_at: Option<u64>,
}

impl VerifiedPoolClaims {
    fn is_expired_at(&self, now: u64) -> bool {
        self.expires_at
            .map(|expires_at| expires_at <= now)
            .unwrap_or(false)
    }
}

#[derive(Debug, Clone)]
struct CachedIntrospection {
    claims: VerifiedPoolClaims,
    inserted_at: Instant,
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct IntrospectionResponse {
    active: bool,
    claims: Option<VerifiedPoolClaims>,
    reason: Option<String>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct ParsedHttpUrl {
    host: String,
    port: u16,
    path: String,
}

impl ParsedHttpUrl {
    fn parse(url: &str) -> Result<Self, PoolAuthError> {
        let rest = url
            .strip_prefix("http://")
            .ok_or_else(|| PoolAuthError::InvalidUrl(url.to_string()))?;
        let (authority, path) = rest.split_once('/').unwrap_or((rest, "auth/introspect"));
        if authority.is_empty() {
            return Err(PoolAuthError::InvalidUrl(url.to_string()));
        }
        let (host, port) = if let Some((host, port)) = authority.rsplit_once(':') {
            let port = port
                .parse::<u16>()
                .map_err(|_| PoolAuthError::InvalidUrl(url.to_string()))?;
            (host.to_string(), port)
        } else {
            (authority.to_string(), 80)
        };
        if host.is_empty() || port == 0 {
            return Err(PoolAuthError::InvalidUrl(url.to_string()));
        }
        let path = format!("/{}", path.trim_start_matches('/'));
        Ok(Self { host, port, path })
    }

    fn addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

fn introspect_token(
    config: &PoolAuthConfig,
    token: &str,
) -> Result<IntrospectionResponse, PoolAuthError> {
    let url = ParsedHttpUrl::parse(&config.introspection_url)?;
    let mut stream = TcpStream::connect(url.addr())?;
    stream.set_read_timeout(Some(config.timeout))?;
    stream.set_write_timeout(Some(config.timeout))?;
    let body = format!("{{\"token\":\"{}\"}}", escape_json(token));
    let request = format!(
        "POST {} HTTP/1.1\r\nHost: {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        url.path,
        url.host,
        body.len(),
        body
    );
    stream.write_all(request.as_bytes())?;
    stream.flush()?;
    let mut response = String::new();
    stream.read_to_string(&mut response)?;
    parse_introspection_http_response(&response)
}

fn parse_introspection_http_response(
    response: &str,
) -> Result<IntrospectionResponse, PoolAuthError> {
    let (head, body) = response.split_once("\r\n\r\n").ok_or_else(|| {
        PoolAuthError::MalformedResponse("missing HTTP header terminator".to_string())
    })?;
    let status = head
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|status| status.parse::<u16>().ok())
        .ok_or_else(|| PoolAuthError::MalformedResponse("missing HTTP status".to_string()))?;
    if status != 200 {
        return Err(PoolAuthError::HttpStatus(status));
    }
    let active = extract_json_bool(body, "active").ok_or_else(|| {
        PoolAuthError::MalformedResponse("introspection body omitted active".to_string())
    })?;
    if !active {
        return Ok(IntrospectionResponse {
            active: false,
            claims: None,
            reason: extract_json_string(body, "reason"),
        });
    }
    let jwt_id = extract_json_string(body, "jti").ok_or_else(|| {
        PoolAuthError::MalformedResponse("active introspection body omitted jti".to_string())
    })?;
    let tenant_id = extract_json_string(body, "tenant_id").ok_or_else(|| {
        PoolAuthError::MalformedResponse("active introspection body omitted tenant_id".to_string())
    })?;
    let subject = extract_json_string(body, "sub").ok_or_else(|| {
        PoolAuthError::MalformedResponse("active introspection body omitted sub".to_string())
    })?;
    let role = extract_json_string(body, "role").unwrap_or_default();
    let expires_at = extract_json_number(body, "exp");
    Ok(IntrospectionResponse {
        active: true,
        claims: Some(VerifiedPoolClaims {
            jwt_id,
            tenant_id,
            subject,
            role,
            expires_at,
        }),
        reason: None,
    })
}

pub fn token_from_startup(tap: &StartupTraceTap) -> Option<String> {
    for key in [
        "ai_blaise.jwt",
        "ai_blaise.token",
        "ai_blaise.access_token",
        "jwt",
        "token",
        "access_token",
    ] {
        if let Some(value) = tap.startup_parameter(key).and_then(non_empty_value) {
            return Some(value.to_string());
        }
    }

    if let Some(options) = tap.startup_parameter("options") {
        for key in [
            "ai_blaise.jwt",
            "ai_blaise.token",
            "ai_blaise.access_token",
            "jwt",
            "token",
            "access_token",
        ] {
            if let Some(value) = extract_options_assignment(options, key).and_then(non_empty_value)
            {
                return Some(value.to_string());
            }
        }
    }

    tap.startup_parameter("application_name")
        .and_then(|application_name| {
            extract_application_assignment(application_name, "jwt")
                .or_else(|| extract_application_assignment(application_name, "token"))
                .or_else(|| extract_application_assignment(application_name, "access_token"))
        })
        .and_then(non_empty_value)
        .map(str::to_string)
}

fn tenant_id_from_startup(tap: &StartupTraceTap) -> Option<String> {
    for key in ["ai_blaise.tenant_id", "tenant_id", "tenant"] {
        if let Some(value) = tap.startup_parameter(key).and_then(non_empty_value) {
            return Some(value.to_string());
        }
    }

    if let Some(options) = tap.startup_parameter("options") {
        for key in ["ai_blaise.tenant_id", "tenant_id", "tenant"] {
            if let Some(value) = extract_options_assignment(options, key).and_then(non_empty_value)
            {
                return Some(value.to_string());
            }
        }
    }

    tap.startup_parameter("application_name")
        .and_then(|application_name| {
            extract_application_assignment(application_name, "tenant_id")
                .or_else(|| extract_application_assignment(application_name, "tenant"))
        })
        .and_then(non_empty_value)
        .map(str::to_string)
}

fn extract_options_assignment<'a>(options: &'a str, key: &str) -> Option<&'a str> {
    let tokens = options.split_whitespace().collect::<Vec<_>>();
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
                    return Some(assignment_value);
                }
            }
        }
        index += 1;
    }
    None
}

fn extract_application_assignment<'a>(application_name: &'a str, key: &str) -> Option<&'a str> {
    for pair in application_name.split(';') {
        let Some((field, value)) = pair.trim().split_once('=') else {
            continue;
        };
        if field.trim() == key {
            return Some(value.trim());
        }
    }
    None
}

fn non_empty_value(value: &str) -> Option<&str> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

fn parse_duration_ms_env(
    name: &'static str,
    default: Duration,
    expected: &'static str,
) -> Result<Duration, PoolAuthError> {
    let Ok(raw) = std::env::var(name) else {
        return Ok(default);
    };
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(default);
    }
    let value = trimmed
        .parse::<u64>()
        .map_err(|_| PoolAuthError::InvalidEnv {
            name,
            value: raw.clone(),
            expected,
        })?;
    if expected.starts_with("positive") && value == 0 {
        return Err(PoolAuthError::InvalidEnv {
            name,
            value: raw,
            expected,
        });
    }
    Ok(Duration::from_millis(value))
}

fn extract_json_string(body: &str, field: &str) -> Option<String> {
    let needle = format!("\"{field}\":\"");
    let start = body.find(&needle)? + needle.len();
    let mut out = String::new();
    let mut escaped = false;
    for ch in body[start..].chars() {
        if escaped {
            out.push(match ch {
                'n' => '\n',
                'r' => '\r',
                't' => '\t',
                '\\' => '\\',
                '"' => '"',
                other => other,
            });
            escaped = false;
            continue;
        }
        match ch {
            '\\' => escaped = true,
            '"' => return Some(out),
            other => out.push(other),
        }
    }
    None
}

fn extract_json_bool(body: &str, field: &str) -> Option<bool> {
    let needle = format!("\"{field}\":");
    let start = body.find(&needle)? + needle.len();
    let rest = body[start..].trim_start();
    if rest.starts_with("true") {
        Some(true)
    } else if rest.starts_with("false") {
        Some(false)
    } else {
        None
    }
}

fn extract_json_number(body: &str, field: &str) -> Option<u64> {
    let needle = format!("\"{field}\":");
    let start = body.find(&needle)? + needle.len();
    let digits = body[start..]
        .trim_start()
        .chars()
        .take_while(|ch| ch.is_ascii_digit())
        .collect::<String>();
    digits.parse().ok()
}

fn escape_json(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

fn cache_key(token: &str) -> String {
    const FNV_OFFSET: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x100000001b3;
    let mut hash = FNV_OFFSET;
    for byte in token.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    format!("{hash:016x}")
}

fn current_unix_time() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_secs()
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum PoolAuthError {
    HttpStatus(u16),
    Inactive(String),
    InvalidEnv {
        name: &'static str,
        value: String,
        expected: &'static str,
    },
    InvalidUrl(String),
    Io(String),
    MalformedResponse(String),
    MissingToken,
    StatePoisoned(&'static str),
    TenantMismatch {
        startup_tenant: String,
        token_tenant: String,
    },
}

impl fmt::Display for PoolAuthError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::HttpStatus(status) => write!(formatter, "auth introspection returned HTTP {status}"),
            Self::Inactive(reason) => write!(formatter, "auth token is inactive: {reason}"),
            Self::InvalidEnv { name, value, expected } => {
                write!(formatter, "{name}={value:?} must be {expected}")
            }
            Self::InvalidUrl(url) => write!(formatter, "auth introspection URL must be http://host[:port]/path, got {url}"),
            Self::Io(error) => write!(formatter, "auth introspection I/O failed: {error}"),
            Self::MalformedResponse(error) => write!(formatter, "auth introspection response malformed: {error}"),
            Self::MissingToken => write!(formatter, "auth token is required in startup parameter ai_blaise.jwt, jwt, token, or application_name jwt=..."),
            Self::StatePoisoned(field) => write!(formatter, "pool auth state poisoned: {field}"),
            Self::TenantMismatch { startup_tenant, token_tenant } => write!(
                formatter,
                "startup tenant {startup_tenant} does not match introspected token tenant {token_tenant}",
            ),
        }
    }
}

impl Error for PoolAuthError {}

impl From<std::io::Error> for PoolAuthError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ai_blaise_citus_sidecar_shared::ApplicationNameFields;

    fn startup(parameters: Vec<(&str, &str)>) -> StartupTraceTap {
        StartupTraceTap {
            fields: ApplicationNameFields::default(),
            buffered_bytes: Vec::new(),
            parameters: parameters
                .into_iter()
                .map(|(key, value)| (key.to_string(), value.to_string()))
                .collect(),
            special_envelope: false,
        }
    }

    #[test]
    fn extracts_tokens_from_application_name_options_or_parameter() {
        assert_eq!(
            token_from_startup(&startup(vec![("application_name", "app=psql;jwt=abc.def")]))
                .as_deref(),
            Some("abc.def")
        );
        assert_eq!(
            token_from_startup(&startup(vec![("options", "-c ai_blaise.jwt=abc.def")])).as_deref(),
            Some("abc.def")
        );
        assert_eq!(
            token_from_startup(&startup(vec![("ai_blaise.jwt", "abc.def")])).as_deref(),
            Some("abc.def")
        );
    }

    #[test]
    fn parses_auth_sidecar_introspection_response() {
        let response = "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\n\r\n{\"active\":true,\"sub\":\"alice\",\"tenant_id\":\"tenant-a\",\"role\":\"authenticated\",\"jti\":\"jti-1\",\"exp\":1893456000}\n";
        let parsed = parse_introspection_http_response(response).expect("parse response");
        let claims = parsed.claims.expect("claims");
        assert!(parsed.active);
        assert_eq!(claims.subject, "alice");
        assert_eq!(claims.tenant_id, "tenant-a");
        assert_eq!(claims.jwt_id, "jti-1");
        assert_eq!(claims.expires_at, Some(1_893_456_000));
    }

    #[test]
    fn inactive_response_carries_reason() {
        let response =
            "HTTP/1.1 200 OK\r\n\r\n{\"active\":false,\"reason\":\"token has been revoked\"}\n";
        let parsed = parse_introspection_http_response(response).expect("parse response");
        assert!(!parsed.active);
        assert_eq!(parsed.reason.as_deref(), Some("token has been revoked"));
    }

    #[test]
    fn rejects_https_urls_until_tls_client_is_wired() {
        assert!(matches!(
            ParsedHttpUrl::parse("https://auth.example.com/auth/introspect"),
            Err(PoolAuthError::InvalidUrl(_))
        ));
    }
}
