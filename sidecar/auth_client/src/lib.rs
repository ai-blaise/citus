//! Bounded, mutually authenticated Auth3 token-introspection client.
//!
//! Production construction accepts HTTPS only, trusts only the explicitly
//! mounted CA bundle, and requires a PEM client identity. Errors deliberately
//! omit endpoints, credentials, bearer tokens, response bodies, and transport
//! internals.

// FEATURE: API3
// FEATURE: Auth3

use reqwest::blocking::{Body, Client, Response};
use reqwest::header::{CONTENT_LENGTH, CONTENT_TYPE};
use reqwest::{Certificate, Identity, StatusCode, Url};
use serde::Deserialize;
use std::error::Error;
use std::fmt;
use std::fs::File;
use std::io::{Read, Take};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use zeroize::Zeroizing;

pub const AUTH_INTROSPECTION_URL_ENV: &str = "AI_BLAISE_GRAPHQL_AUTH_INTROSPECTION_URL";
pub const AUTH_CA_CERT_PATH_ENV: &str = "AI_BLAISE_GRAPHQL_AUTH_CA_CERT_PATH";
pub const AUTH_CLIENT_IDENTITY_PATH_ENV: &str = "AI_BLAISE_GRAPHQL_AUTH_CLIENT_IDENTITY_PATH";
pub const AUTH_EXPECTED_ISSUER_ENV: &str = "AI_BLAISE_GRAPHQL_AUTH_EXPECTED_ISSUER";
pub const AUTH_EXPECTED_AUDIENCE_ENV: &str = "AI_BLAISE_GRAPHQL_AUTH_EXPECTED_AUDIENCE";
pub const AUTH_TIMEOUT_MS_ENV: &str = "AI_BLAISE_GRAPHQL_AUTH_TIMEOUT_MS";

const DEFAULT_TIMEOUT: Duration = Duration::from_millis(750);
const MAX_TIMEOUT: Duration = Duration::from_secs(5);
const MAX_CREDENTIAL_BYTES: u64 = 1_048_576;
const MAX_RESPONSE_BYTES: u64 = 16_384;
const MAX_TOKEN_BYTES: usize = 16_384;
const MAX_CLAIM_BYTES: usize = 1_024;
const CLOCK_LEEWAY_SECONDS: i64 = 30;

#[derive(Clone, Eq, PartialEq)]
pub struct AuthIntrospectionConfig {
    endpoint: String,
    ca_cert_path: PathBuf,
    client_identity_path: PathBuf,
    expected_issuer: String,
    expected_audience: String,
    timeout: Duration,
}

impl fmt::Debug for AuthIntrospectionConfig {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AuthIntrospectionConfig")
            .field("endpoint", &"<redacted>")
            .field("ca_cert_path", &"<configured>")
            .field("client_identity_path", &"<configured>")
            .field("expected_issuer", &"<redacted>")
            .field("expected_audience", &"<redacted>")
            .field("timeout", &self.timeout)
            .finish()
    }
}

impl AuthIntrospectionConfig {
    pub fn from_env() -> Result<Self, AuthIntrospectionError> {
        Self::from_lookup(|name| std::env::var(name).ok())
    }

    pub fn from_lookup<F>(lookup: F) -> Result<Self, AuthIntrospectionError>
    where
        F: Fn(&str) -> Option<String>,
    {
        let endpoint = required(&lookup, AUTH_INTROSPECTION_URL_ENV)?;
        let ca_cert_path = required(&lookup, AUTH_CA_CERT_PATH_ENV)?;
        let client_identity_path = required(&lookup, AUTH_CLIENT_IDENTITY_PATH_ENV)?;
        let expected_issuer = required(&lookup, AUTH_EXPECTED_ISSUER_ENV)?;
        let expected_audience = required(&lookup, AUTH_EXPECTED_AUDIENCE_ENV)?;
        let timeout =
            match lookup(AUTH_TIMEOUT_MS_ENV) {
                Some(raw) => Duration::from_millis(raw.parse::<u64>().map_err(|_| {
                    AuthIntrospectionError::InvalidConfiguration(AUTH_TIMEOUT_MS_ENV)
                })?),
                None => DEFAULT_TIMEOUT,
            };
        Self::new(
            endpoint,
            ca_cert_path,
            client_identity_path,
            expected_issuer,
            expected_audience,
            timeout,
        )
    }

    pub fn new(
        endpoint: impl Into<String>,
        ca_cert_path: impl Into<PathBuf>,
        client_identity_path: impl Into<PathBuf>,
        expected_issuer: impl Into<String>,
        expected_audience: impl Into<String>,
        timeout: Duration,
    ) -> Result<Self, AuthIntrospectionError> {
        let config = Self {
            endpoint: endpoint.into(),
            ca_cert_path: ca_cert_path.into(),
            client_identity_path: client_identity_path.into(),
            expected_issuer: expected_issuer.into(),
            expected_audience: expected_audience.into(),
            timeout,
        };
        config.validate()?;
        Ok(config)
    }

    fn validate(&self) -> Result<(), AuthIntrospectionError> {
        validate_endpoint(&self.endpoint)?;
        validate_path(&self.ca_cert_path, AUTH_CA_CERT_PATH_ENV)?;
        validate_path(&self.client_identity_path, AUTH_CLIENT_IDENTITY_PATH_ENV)?;
        validate_claim(AUTH_EXPECTED_ISSUER_ENV, &self.expected_issuer)?;
        validate_claim(AUTH_EXPECTED_AUDIENCE_ENV, &self.expected_audience)?;
        if self.timeout.is_zero() || self.timeout > MAX_TIMEOUT {
            return Err(AuthIntrospectionError::InvalidConfiguration(
                AUTH_TIMEOUT_MS_ENV,
            ));
        }
        Ok(())
    }
}

pub struct AuthIntrospectionClient {
    client: Client,
    endpoint: Url,
    expected_issuer: String,
    expected_audience: String,
}

struct SensitiveRequestBody {
    bytes: Zeroizing<Vec<u8>>,
    offset: usize,
}

impl SensitiveRequestBody {
    fn new(bytes: Vec<u8>) -> Self {
        Self {
            bytes: Zeroizing::new(bytes),
            offset: 0,
        }
    }

    fn len(&self) -> u64 {
        self.bytes.len() as u64
    }
}

impl Read for SensitiveRequestBody {
    fn read(&mut self, output: &mut [u8]) -> std::io::Result<usize> {
        let remaining = &self.bytes[self.offset..];
        let length = remaining.len().min(output.len());
        output[..length].copy_from_slice(&remaining[..length]);
        self.offset += length;
        Ok(length)
    }
}

impl AuthIntrospectionClient {
    pub fn from_env() -> Result<Self, AuthIntrospectionError> {
        Self::new(AuthIntrospectionConfig::from_env()?)
    }

    pub fn new(config: AuthIntrospectionConfig) -> Result<Self, AuthIntrospectionError> {
        config.validate()?;
        let ca_pem = read_bounded(&config.ca_cert_path, AUTH_CA_CERT_PATH_ENV)?;
        let identity_pem =
            read_bounded(&config.client_identity_path, AUTH_CLIENT_IDENTITY_PATH_ENV)?;
        let certificates = Certificate::from_pem_bundle(ca_pem.as_slice())
            .map_err(|_| AuthIntrospectionError::CredentialInvalid(AUTH_CA_CERT_PATH_ENV))?;
        if certificates.is_empty() {
            return Err(AuthIntrospectionError::CredentialInvalid(
                AUTH_CA_CERT_PATH_ENV,
            ));
        }
        let identity = Identity::from_pem(identity_pem.as_slice()).map_err(|_| {
            AuthIntrospectionError::CredentialInvalid(AUTH_CLIENT_IDENTITY_PATH_ENV)
        })?;
        let mut builder = Client::builder()
            .use_rustls_tls()
            .https_only(true)
            .tls_built_in_root_certs(false)
            .min_tls_version(reqwest::tls::Version::TLS_1_2)
            .identity(identity)
            .connect_timeout(config.timeout)
            .timeout(config.timeout)
            .redirect(reqwest::redirect::Policy::none())
            .no_proxy()
            .http1_only()
            .pool_max_idle_per_host(2);
        for certificate in certificates {
            builder = builder.add_root_certificate(certificate);
        }
        let client = builder
            .build()
            .map_err(|_| AuthIntrospectionError::ClientBuild)?;
        let endpoint = Url::parse(&config.endpoint).map_err(|_| {
            AuthIntrospectionError::InvalidConfiguration(AUTH_INTROSPECTION_URL_ENV)
        })?;
        Ok(Self {
            client,
            endpoint,
            expected_issuer: config.expected_issuer,
            expected_audience: config.expected_audience,
        })
    }

    pub fn introspect(&self, token: &str) -> Result<VerifiedIdentity, AuthIntrospectionError> {
        validate_bearer_token(token)?;
        let request_body =
            SensitiveRequestBody::new(format!("{{\"token\":\"{token}\"}}").into_bytes());
        let request_body_length = request_body.len();
        let response = self
            .client
            .post(self.endpoint.clone())
            .header(CONTENT_TYPE, "application/json")
            .body(Body::sized(request_body, request_body_length))
            .send()
            .map_err(|_| AuthIntrospectionError::RequestFailed)?;
        parse_http_response(
            response,
            &self.expected_issuer,
            &self.expected_audience,
            unix_time(),
        )
    }

    #[cfg(test)]
    fn new_loopback_for_test(
        endpoint: &str,
        expected_issuer: impl Into<String>,
        expected_audience: impl Into<String>,
        timeout: Duration,
    ) -> Result<Self, AuthIntrospectionError> {
        let parsed = Url::parse(endpoint).map_err(|_| {
            AuthIntrospectionError::InvalidConfiguration(AUTH_INTROSPECTION_URL_ENV)
        })?;
        let host = parsed
            .host_str()
            .ok_or(AuthIntrospectionError::InvalidConfiguration(
                AUTH_INTROSPECTION_URL_ENV,
            ))?;
        if parsed.scheme() != "http"
            || parsed.path() != "/auth/introspect"
            || parsed.query().is_some()
            || parsed.fragment().is_some()
            || !parsed.username().is_empty()
            || parsed.password().is_some()
            || !matches!(host, "127.0.0.1" | "::1")
            || timeout.is_zero()
            || timeout > MAX_TIMEOUT
        {
            return Err(AuthIntrospectionError::InvalidConfiguration(
                AUTH_INTROSPECTION_URL_ENV,
            ));
        }
        let expected_issuer = expected_issuer.into();
        let expected_audience = expected_audience.into();
        validate_claim(AUTH_EXPECTED_ISSUER_ENV, &expected_issuer)?;
        validate_claim(AUTH_EXPECTED_AUDIENCE_ENV, &expected_audience)?;
        let client = Client::builder()
            .connect_timeout(timeout)
            .timeout(timeout)
            .redirect(reqwest::redirect::Policy::none())
            .no_proxy()
            .http1_only()
            .build()
            .map_err(|_| AuthIntrospectionError::ClientBuild)?;
        Ok(Self {
            client,
            endpoint: parsed,
            expected_issuer,
            expected_audience,
        })
    }
}

#[derive(Clone, Eq, PartialEq)]
pub struct VerifiedIdentity {
    subject: String,
    tenant_id: String,
    role: String,
    jwt_id: String,
    issuer: String,
    audience: String,
    issued_at: i64,
    expires_at: i64,
    mfa_verified: bool,
}

impl fmt::Debug for VerifiedIdentity {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("VerifiedIdentity")
            .field("subject", &"<redacted>")
            .field("tenant_id", &"<redacted>")
            .field("role", &"<redacted>")
            .field("jwt_id", &"<redacted>")
            .field("issuer", &"<redacted>")
            .field("audience", &"<redacted>")
            .field("issued_at", &self.issued_at)
            .field("expires_at", &self.expires_at)
            .field("mfa_verified", &self.mfa_verified)
            .finish()
    }
}

impl VerifiedIdentity {
    pub fn subject(&self) -> &str {
        &self.subject
    }

    pub fn tenant_id(&self) -> &str {
        &self.tenant_id
    }

    pub fn role(&self) -> &str {
        &self.role
    }

    pub fn jwt_id(&self) -> &str {
        &self.jwt_id
    }

    pub fn expires_at(&self) -> i64 {
        self.expires_at
    }

    pub fn claims_json(&self) -> String {
        serde_json::json!({
            "sub": self.subject,
            "tenant_id": self.tenant_id,
            "role": self.role,
            "jti": self.jwt_id,
            "iss": self.issuer,
            "aud": self.audience,
            "iat": self.issued_at,
            "exp": self.expires_at,
            "mfa_verified": self.mfa_verified,
        })
        .to_string()
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum AuthIntrospectionError {
    MissingConfiguration(&'static str),
    InvalidConfiguration(&'static str),
    CredentialRead(&'static str),
    CredentialTooLarge(&'static str),
    CredentialInvalid(&'static str),
    ClientBuild,
    InvalidToken,
    RequestFailed,
    ResponseTooLarge,
    UnexpectedStatus(u16),
    InvalidResponse(&'static str),
    Inactive,
}

impl fmt::Display for AuthIntrospectionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingConfiguration(name) => {
                write!(
                    formatter,
                    "missing auth introspection configuration: {name}"
                )
            }
            Self::InvalidConfiguration(name) => {
                write!(
                    formatter,
                    "invalid auth introspection configuration: {name}"
                )
            }
            Self::CredentialRead(name) => {
                write!(
                    formatter,
                    "unable to read auth introspection credential: {name}"
                )
            }
            Self::CredentialTooLarge(name) => {
                write!(
                    formatter,
                    "auth introspection credential exceeds size limit: {name}"
                )
            }
            Self::CredentialInvalid(name) => {
                write!(formatter, "invalid auth introspection credential: {name}")
            }
            Self::ClientBuild => {
                write!(formatter, "unable to initialize auth introspection client")
            }
            Self::InvalidToken => write!(formatter, "invalid bearer token"),
            Self::RequestFailed => write!(formatter, "auth introspection request failed"),
            Self::ResponseTooLarge => {
                write!(formatter, "auth introspection response exceeds size limit")
            }
            Self::UnexpectedStatus(status) => {
                write!(formatter, "auth introspection returned HTTP {status}")
            }
            Self::InvalidResponse(field) => {
                write!(formatter, "invalid auth introspection response: {field}")
            }
            Self::Inactive => write!(formatter, "bearer token is inactive"),
        }
    }
}

impl Error for AuthIntrospectionError {}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WireIntrospectionResponse {
    active: bool,
    sub: Option<String>,
    tenant_id: Option<String>,
    role: Option<String>,
    jti: Option<String>,
    iss: Option<String>,
    aud: Option<String>,
    iat: Option<i64>,
    exp: Option<i64>,
    mfa_verified: Option<bool>,
    reason: Option<String>,
}

fn required<F>(lookup: &F, name: &'static str) -> Result<String, AuthIntrospectionError>
where
    F: Fn(&str) -> Option<String>,
{
    lookup(name)
        .filter(|value| !value.trim().is_empty())
        .ok_or(AuthIntrospectionError::MissingConfiguration(name))
}

fn validate_endpoint(raw: &str) -> Result<(), AuthIntrospectionError> {
    if raw.len() > 2_048 || raw.chars().any(char::is_control) {
        return Err(AuthIntrospectionError::InvalidConfiguration(
            AUTH_INTROSPECTION_URL_ENV,
        ));
    }
    let url = Url::parse(raw)
        .map_err(|_| AuthIntrospectionError::InvalidConfiguration(AUTH_INTROSPECTION_URL_ENV))?;
    if url.scheme() != "https"
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.path() != "/auth/introspect"
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(AuthIntrospectionError::InvalidConfiguration(
            AUTH_INTROSPECTION_URL_ENV,
        ));
    }
    Ok(())
}

fn validate_path(path: &Path, name: &'static str) -> Result<(), AuthIntrospectionError> {
    let raw = path
        .to_str()
        .ok_or(AuthIntrospectionError::InvalidConfiguration(name))?;
    if raw.trim().is_empty() || raw.len() > 4_096 || raw.chars().any(char::is_control) {
        return Err(AuthIntrospectionError::InvalidConfiguration(name));
    }
    Ok(())
}

fn validate_claim(name: &'static str, value: &str) -> Result<(), AuthIntrospectionError> {
    if value.trim().is_empty()
        || value.len() > MAX_CLAIM_BYTES
        || value.chars().any(char::is_control)
    {
        return Err(AuthIntrospectionError::InvalidConfiguration(name));
    }
    Ok(())
}

pub fn validate_bearer_token(token: &str) -> Result<(), AuthIntrospectionError> {
    if token.is_empty()
        || token.len() > MAX_TOKEN_BYTES
        || !token.bytes().all(|byte| {
            byte.is_ascii_alphanumeric()
                || matches!(byte, b'-' | b'.' | b'_' | b'~' | b'+' | b'/' | b'=')
        })
    {
        return Err(AuthIntrospectionError::InvalidToken);
    }
    Ok(())
}

fn read_bounded(
    path: &Path,
    name: &'static str,
) -> Result<Zeroizing<Vec<u8>>, AuthIntrospectionError> {
    let file = File::open(path).map_err(|_| AuthIntrospectionError::CredentialRead(name))?;
    let mut reader: Take<File> = file.take(MAX_CREDENTIAL_BYTES + 1);
    let mut bytes = Zeroizing::new(Vec::new());
    reader
        .read_to_end(&mut bytes)
        .map_err(|_| AuthIntrospectionError::CredentialRead(name))?;
    if bytes.len() as u64 > MAX_CREDENTIAL_BYTES {
        return Err(AuthIntrospectionError::CredentialTooLarge(name));
    }
    if bytes.is_empty() {
        return Err(AuthIntrospectionError::CredentialInvalid(name));
    }
    Ok(bytes)
}

fn parse_http_response(
    response: Response,
    expected_issuer: &str,
    expected_audience: &str,
    now: i64,
) -> Result<VerifiedIdentity, AuthIntrospectionError> {
    if response.status() != StatusCode::OK {
        return Err(AuthIntrospectionError::UnexpectedStatus(
            response.status().as_u16(),
        ));
    }
    let content_type = response
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(';').next())
        .map(str::trim);
    if content_type != Some("application/json") {
        return Err(AuthIntrospectionError::InvalidResponse("content-type"));
    }
    if response
        .headers()
        .get(CONTENT_LENGTH)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<u64>().ok())
        .is_some_and(|length| length > MAX_RESPONSE_BYTES)
    {
        return Err(AuthIntrospectionError::ResponseTooLarge);
    }
    let mut bytes = Zeroizing::new(Vec::new());
    response
        .take(MAX_RESPONSE_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| AuthIntrospectionError::RequestFailed)?;
    if bytes.len() as u64 > MAX_RESPONSE_BYTES {
        return Err(AuthIntrospectionError::ResponseTooLarge);
    }
    parse_response_bytes(&bytes, expected_issuer, expected_audience, now)
}

fn parse_response_bytes(
    bytes: &[u8],
    expected_issuer: &str,
    expected_audience: &str,
    now: i64,
) -> Result<VerifiedIdentity, AuthIntrospectionError> {
    let wire: WireIntrospectionResponse = serde_json::from_slice(bytes)
        .map_err(|_| AuthIntrospectionError::InvalidResponse("json"))?;
    if !wire.active {
        let _ = wire.reason;
        return Err(AuthIntrospectionError::Inactive);
    }
    let subject = required_response_claim(wire.sub, "sub")?;
    let tenant_id = required_response_claim(wire.tenant_id, "tenant_id")?;
    let role = required_response_claim(wire.role, "role")?;
    let jwt_id = required_response_claim(wire.jti, "jti")?;
    let issuer = required_response_claim(wire.iss, "iss")?;
    let audience = required_response_claim(wire.aud, "aud")?;
    if issuer != expected_issuer {
        return Err(AuthIntrospectionError::InvalidResponse("iss"));
    }
    if audience != expected_audience {
        return Err(AuthIntrospectionError::InvalidResponse("aud"));
    }
    let issued_at = wire
        .iat
        .ok_or(AuthIntrospectionError::InvalidResponse("iat"))?;
    let expires_at = wire
        .exp
        .ok_or(AuthIntrospectionError::InvalidResponse("exp"))?;
    if issued_at > now.saturating_add(CLOCK_LEEWAY_SECONDS) {
        return Err(AuthIntrospectionError::InvalidResponse("iat"));
    }
    if expires_at.saturating_add(CLOCK_LEEWAY_SECONDS) < now || expires_at <= issued_at {
        return Err(AuthIntrospectionError::Inactive);
    }
    Ok(VerifiedIdentity {
        subject,
        tenant_id,
        role,
        jwt_id,
        issuer,
        audience,
        issued_at,
        expires_at,
        mfa_verified: wire.mfa_verified.unwrap_or(false),
    })
}

fn required_response_claim(
    value: Option<String>,
    field: &'static str,
) -> Result<String, AuthIntrospectionError> {
    let value = value.ok_or(AuthIntrospectionError::InvalidResponse(field))?;
    if value.trim().is_empty()
        || value.len() > MAX_CLAIM_BYTES
        || value.chars().any(char::is_control)
    {
        return Err(AuthIntrospectionError::InvalidResponse(field));
    }
    Ok(value)
}

fn unix_time() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_secs() as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    fn response(active: bool) -> Vec<u8> {
        let now = unix_time();
        serde_json::json!({
            "active": active,
            "sub": "user-1",
            "tenant_id": "tenant-a",
            "role": "web_user",
            "jti": "token-1",
            "iss": "https://auth.example.com",
            "aud": "postgres",
            "iat": now - 1,
            "exp": now + 300,
            "mfa_verified": true,
        })
        .to_string()
        .into_bytes()
    }

    #[test]
    fn production_config_requires_exact_https_introspection_endpoint() {
        for endpoint in [
            "http://auth/auth/introspect",
            "https://user@auth/auth/introspect",
            "https://auth/other",
            "https://auth/auth/introspect?token=x",
        ] {
            assert_eq!(
                AuthIntrospectionConfig::new(
                    endpoint,
                    "/run/secrets/ca.pem",
                    "/run/secrets/client.pem",
                    "https://auth.example.com",
                    "postgres",
                    DEFAULT_TIMEOUT,
                ),
                Err(AuthIntrospectionError::InvalidConfiguration(
                    AUTH_INTROSPECTION_URL_ENV
                ))
            );
        }
    }

    #[test]
    fn production_config_requires_every_authority_binding() {
        let error = AuthIntrospectionConfig::from_lookup(|name| match name {
            AUTH_INTROSPECTION_URL_ENV => Some("https://auth/auth/introspect".to_string()),
            _ => None,
        })
        .expect_err("missing CA path");
        assert_eq!(
            error,
            AuthIntrospectionError::MissingConfiguration(AUTH_CA_CERT_PATH_ENV)
        );
    }

    #[test]
    fn response_parser_accepts_only_active_bound_authority_claims() {
        let now = unix_time();
        let identity =
            parse_response_bytes(&response(true), "https://auth.example.com", "postgres", now)
                .expect("identity");
        assert_eq!(identity.tenant_id(), "tenant-a");
        assert_eq!(identity.subject(), "user-1");
        assert_eq!(identity.role(), "web_user");
        assert_eq!(identity.jwt_id(), "token-1");
        assert!(identity.expires_at() > now);
        assert!(identity
            .claims_json()
            .contains("\"tenant_id\":\"tenant-a\""));

        assert_eq!(
            parse_response_bytes(
                &response(false),
                "https://auth.example.com",
                "postgres",
                now
            ),
            Err(AuthIntrospectionError::Inactive)
        );
        assert_eq!(
            parse_response_bytes(
                &response(true),
                "https://different.example.com",
                "postgres",
                now
            ),
            Err(AuthIntrospectionError::InvalidResponse("iss"))
        );
    }

    #[test]
    fn response_parser_rejects_missing_duplicate_unknown_and_expired_claims() {
        let now = unix_time();
        let missing = br#"{"active":true}"#;
        assert_eq!(
            parse_response_bytes(missing, "issuer", "audience", now),
            Err(AuthIntrospectionError::InvalidResponse("sub"))
        );
        let duplicate = br#"{"active":false,"active":true}"#;
        assert_eq!(
            parse_response_bytes(duplicate, "issuer", "audience", now),
            Err(AuthIntrospectionError::InvalidResponse("json"))
        );
        let unknown = br#"{"active":false,"unexpected":true}"#;
        assert_eq!(
            parse_response_bytes(unknown, "issuer", "audience", now),
            Err(AuthIntrospectionError::InvalidResponse("json"))
        );
        let mut expired: serde_json::Value =
            serde_json::from_slice(&response(true)).expect("fixture json");
        expired["iat"] = serde_json::json!(now - 600);
        expired["exp"] = serde_json::json!(now - 300);
        assert_eq!(
            parse_response_bytes(
                expired.to_string().as_bytes(),
                "https://auth.example.com",
                "postgres",
                now,
            ),
            Err(AuthIntrospectionError::Inactive)
        );
    }

    #[test]
    fn bearer_token_validation_is_bounded_and_header_safe() {
        assert_eq!(validate_bearer_token("a.b.c"), Ok(()));
        for token in ["", "a b", "a\nb", "a\"b", "a\\b"] {
            assert_eq!(
                validate_bearer_token(token),
                Err(AuthIntrospectionError::InvalidToken)
            );
        }
        let oversized = "a".repeat(MAX_TOKEN_BYTES + 1);
        assert_eq!(
            validate_bearer_token(&oversized),
            Err(AuthIntrospectionError::InvalidToken)
        );
    }

    #[test]
    fn identity_debug_output_is_redacted() {
        let identity = parse_response_bytes(
            &response(true),
            "https://auth.example.com",
            "postgres",
            unix_time(),
        )
        .expect("identity");
        let rendered = format!("{identity:?}");
        for sensitive in ["user-1", "tenant-a", "web_user", "token-1"] {
            assert!(!rendered.contains(sensitive));
        }
        assert!(rendered.contains("<redacted>"));
    }

    #[test]
    fn explicit_loopback_test_transport_posts_one_bounded_token() {
        use std::io::{Read as _, Write as _};
        use std::net::TcpListener;
        use std::thread;

        let listener = TcpListener::bind("127.0.0.1:0").expect("bind loopback");
        let address = listener.local_addr().expect("local address");
        let response_body = response(true);
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept request");
            stream
                .set_read_timeout(Some(Duration::from_secs(2)))
                .expect("read timeout");
            let mut request = Vec::new();
            let mut buffer = [0_u8; 1_024];
            loop {
                let length = stream.read(&mut buffer).expect("read request");
                if length == 0 {
                    break;
                }
                request.extend_from_slice(&buffer[..length]);
                let Some(head_end) = request.windows(4).position(|bytes| bytes == b"\r\n\r\n")
                else {
                    continue;
                };
                let body_offset = head_end + 4;
                let head = std::str::from_utf8(&request[..head_end]).expect("request head");
                let content_length = head
                    .lines()
                    .filter_map(|line| line.split_once(':'))
                    .find(|(name, _)| name.eq_ignore_ascii_case("content-length"))
                    .and_then(|(_, value)| value.trim().parse::<usize>().ok())
                    .expect("content length");
                if request.len() >= body_offset + content_length {
                    break;
                }
            }
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                response_body.len()
            );
            stream
                .write_all(response.as_bytes())
                .expect("response head");
            stream.write_all(&response_body).expect("response body");
            request
        });

        let client = AuthIntrospectionClient::new_loopback_for_test(
            &format!("http://{address}/auth/introspect"),
            "https://auth.example.com",
            "postgres",
            Duration::from_secs(2),
        )
        .expect("test client");
        let identity = client
            .introspect("header.token.value")
            .expect("introspection");
        assert_eq!(identity.tenant_id(), "tenant-a");

        let request = server.join().expect("server result");
        let request = std::str::from_utf8(&request).expect("request utf8");
        assert!(request.starts_with("POST /auth/introspect HTTP/1.1\r\n"));
        assert!(request
            .to_ascii_lowercase()
            .contains("content-type: application/json"));
        assert!(request.ends_with("{\"token\":\"header.token.value\"}"));
    }
}
