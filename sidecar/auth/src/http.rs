//! Hand-rolled HTTP surface for the auth sidecar.
//!
//! This mirrors the convention used by `sidecar/mcp`: a synchronous
//! `TcpListener` accepting one request per connection, parsing just enough of
//! the HTTP request to route to a per-endpoint handler, and falling back to
//! the shared `SidecarRuntime` probe machinery for `/healthz`, `/readyz`,
//! `/drain`, and `/metrics`.
//!
//! The endpoints implemented here cover Auth1 (issuance), Auth2 (claims),
//! Auth3 (introspection), and Auth5 TOTP (MFA). Auth4 OIDC and Auth5 WebAuthn
//! endpoints exist on the router but return `501 Not Implemented` with a
//! clear `alpha contract` reason -- they stay alpha until the full runtime is implemented and live-gated.

use crate::{
    AuthEngine, AuthSidecarError, LoginRequest, OidcCallbackRequest, OidcLoginRequest,
    RefreshRequest, TotpEnrollment, TotpVerifyRequest, VerifiedClaims,
};
use ai_blaise_citus_sidecar_shared::{listen_addr_from_env, HttpProbeResponse, SidecarRuntime};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::time::Duration;

const READ_TIMEOUT: Duration = Duration::from_secs(5);
const MAX_REQUEST_BYTES: usize = 1 << 16;

pub fn serve_http_forever(default_addr: &str) -> Result<(), AuthSidecarError> {
    let engine = Arc::new(AuthEngine::from_env()?);
    let listen_addr = listen_addr_from_env(default_addr)?;
    let listener = TcpListener::bind(&listen_addr)?;
    eprintln!("ai-blaise auth-sidecar HTTP server listening on {listen_addr}");

    for stream in listener.incoming() {
        let stream = stream?;
        if let Err(error) = handle_connection(stream, Arc::clone(&engine)) {
            eprintln!("ai-blaise auth-sidecar request failed: {error}");
        }
    }

    Ok(())
}

fn handle_connection(
    mut stream: TcpStream,
    engine: Arc<AuthEngine>,
) -> Result<(), AuthSidecarError> {
    let request_bytes = read_http_request(&mut stream)?;
    let response = handle_http_bytes(&engine, &request_bytes);
    stream.write_all(response.to_http_string().as_bytes())?;
    Ok(())
}

fn read_http_request(stream: &mut TcpStream) -> Result<Vec<u8>, std::io::Error> {
    stream.set_read_timeout(Some(READ_TIMEOUT))?;
    let mut buffer = Vec::new();
    let mut chunk = [0_u8; 4096];
    loop {
        let read_len = stream.read(&mut chunk)?;
        if read_len == 0 {
            break;
        }
        buffer.extend_from_slice(&chunk[..read_len]);
        if request_complete(&buffer) || buffer.len() >= MAX_REQUEST_BYTES {
            break;
        }
    }
    Ok(buffer)
}

fn request_complete(buffer: &[u8]) -> bool {
    let Some(head_end) = find_subsequence(buffer, b"\r\n\r\n") else {
        return false;
    };
    let header_bytes = &buffer[..head_end];
    let headers_str = match std::str::from_utf8(header_bytes) {
        Ok(value) => value,
        Err(_) => return true,
    };
    let content_length = headers_str
        .lines()
        .filter_map(|line| {
            let (name, value) = line.split_once(':')?;
            if name.trim().eq_ignore_ascii_case("content-length") {
                value.trim().parse::<usize>().ok()
            } else {
                None
            }
        })
        .next()
        .unwrap_or(0);
    buffer.len() >= head_end + 4 + content_length
}

fn find_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

/// Dispatch a complete HTTP request. Exposed for in-process testing.
pub fn handle_http_bytes(engine: &AuthEngine, request: &[u8]) -> HttpProbeResponse {
    let parsed = match parse_http_request(request) {
        Ok(parsed) => parsed,
        Err(error) => return bad_request(&error.to_string()),
    };

    let route = (parsed.method.as_str(), parsed.path.as_str());
    match route {
        ("POST", "/auth/login") => endpoint_login(engine, &parsed),
        ("POST", "/auth/refresh") => endpoint_refresh(engine, &parsed),
        ("POST", "/auth/verify") => endpoint_verify(engine, &parsed),
        ("POST", "/auth/introspect") => endpoint_introspect(engine, &parsed),
        ("POST", "/auth/logout") => endpoint_logout(engine, &parsed),
        ("POST", "/auth/mfa/totp/enroll") => endpoint_totp_enroll(engine, &parsed),
        ("POST", "/auth/mfa/totp/verify") => endpoint_totp_verify(engine, &parsed),
        ("POST", "/auth/mfa/webauthn/register") => alpha_response("/auth/mfa/webauthn/register"),
        ("POST", "/auth/mfa/webauthn/finish") => alpha_response("/auth/mfa/webauthn/finish"),
        ("GET", "/auth/oidc/login") => endpoint_oidc_login(engine, &parsed),
        ("GET", "/auth/oidc/callback") => endpoint_oidc_callback(engine, &parsed),
        ("POST", "/auth/users") => endpoint_register_user(engine, &parsed),
        _ => fall_back_to_probe(&parsed, request),
    }
}

#[derive(Debug, Clone)]
struct HttpRequest {
    method: String,
    path: String,
    query: String,
    body: String,
}

fn parse_http_request(bytes: &[u8]) -> Result<HttpRequest, AuthSidecarError> {
    let text = std::str::from_utf8(bytes).map_err(|_| AuthSidecarError::JwtMalformed)?;
    let mut iter = text.splitn(2, "\r\n\r\n");
    let head = iter.next().ok_or(AuthSidecarError::JwtMalformed)?;
    let body = iter.next().unwrap_or("");
    let mut lines = head.lines();
    let request_line = lines.next().ok_or(AuthSidecarError::JwtMalformed)?;
    let mut parts = request_line.split_whitespace();
    let method = parts.next().ok_or(AuthSidecarError::JwtMalformed)?;
    let target = parts.next().ok_or(AuthSidecarError::JwtMalformed)?;
    let (path, query) = target.split_once('?').unwrap_or((target, ""));

    Ok(HttpRequest {
        method: method.to_string(),
        path: path.to_string(),
        query: query.to_string(),
        body: body.to_string(),
    })
}

fn endpoint_login(engine: &AuthEngine, request: &HttpRequest) -> HttpProbeResponse {
    let json = request.body.trim();
    let username = match parse_json_string(json, "username") {
        Some(value) => value,
        None => return bad_request("username field is required"),
    };
    let password = match parse_json_string(json, "password") {
        Some(value) => value,
        None => return bad_request("password field is required"),
    };
    let totp_code = parse_json_string(json, "totp_code");

    match engine.login(&LoginRequest {
        username,
        password,
        totp_code,
    }) {
        Ok(response) => {
            let body = format!(
                "{{\"access_token\":\"{access}\",\"refresh_token\":\"{refresh}\",\"expires_in\":{ttl},\"mfa_required\":{mfa_required},\"mfa_verified\":{mfa_verified}}}\n",
                access = escape_json(&response.access_token),
                refresh = escape_json(&response.refresh_token),
                ttl = response.expires_in,
                mfa_required = response.mfa_required,
                mfa_verified = response.mfa_verified,
            );
            HttpProbeResponse::new(200, "application/json", body)
        }
        Err(AuthSidecarError::UnknownUser) | Err(AuthSidecarError::PasswordVerificationFailed) => {
            unauthorized("invalid credentials")
        }
        Err(AuthSidecarError::TotpCodeInvalid) => unauthorized("invalid totp code"),
        Err(AuthSidecarError::TotpNotEnrolled) => unauthorized("totp not enrolled"),
        Err(AuthSidecarError::MfaAttemptsExceeded) => unauthorized("mfa attempts exceeded"),
        Err(error) => bad_request(&error.to_string()),
    }
}

fn endpoint_refresh(engine: &AuthEngine, request: &HttpRequest) -> HttpProbeResponse {
    let json = request.body.trim();
    let refresh_token = match parse_json_string(json, "refresh_token") {
        Some(value) => value,
        None => return bad_request("refresh_token field is required"),
    };
    match engine.refresh(&RefreshRequest { refresh_token }) {
        Ok(response) => {
            let body = format!(
                "{{\"access_token\":\"{access}\",\"refresh_token\":\"{refresh}\",\"expires_in\":{ttl},\"mfa_required\":{mfa_required},\"mfa_verified\":{mfa_verified}}}\n",
                access = escape_json(&response.access_token),
                refresh = escape_json(&response.refresh_token),
                ttl = response.expires_in,
                mfa_required = response.mfa_required,
                mfa_verified = response.mfa_verified,
            );
            HttpProbeResponse::new(200, "application/json", body)
        }
        Err(AuthSidecarError::UnknownSession) | Err(AuthSidecarError::JwtExpired) => {
            unauthorized("refresh token invalid")
        }
        Err(error) => bad_request(&error.to_string()),
    }
}

fn endpoint_verify(engine: &AuthEngine, request: &HttpRequest) -> HttpProbeResponse {
    let token = match parse_json_string(request.body.trim(), "token") {
        Some(value) => value,
        None => return bad_request("token field is required"),
    };
    match engine.verify_token(&token) {
        Ok(claims) => HttpProbeResponse::new(200, "application/json", render_claims(&claims)),
        Err(AuthSidecarError::JwtMalformed)
        | Err(AuthSidecarError::JwtExpired)
        | Err(AuthSidecarError::JwtBadSignature)
        | Err(AuthSidecarError::JwtRevoked)
        | Err(AuthSidecarError::JwtNotYetValid)
        | Err(AuthSidecarError::JwtBadIssuer)
        | Err(AuthSidecarError::JwtBadAudience)
        | Err(AuthSidecarError::JwtUnsupportedAlgorithm) => unauthorized("token rejected"),
        Err(error) => bad_request(&error.to_string()),
    }
}

fn endpoint_introspect(engine: &AuthEngine, request: &HttpRequest) -> HttpProbeResponse {
    let token = match parse_json_string(request.body.trim(), "token") {
        Some(value) => value,
        None => return bad_request("token field is required"),
    };
    let result = engine.introspect(&token);
    let mut body = String::with_capacity(256);
    body.push_str(&format!("{{\"active\":{}", result.active));
    if let Some(claims) = result.claims.as_ref() {
        body.push_str(",\"sub\":\"");
        body.push_str(&escape_json(&claims.subject));
        body.push_str("\",\"tenant_id\":\"");
        body.push_str(&escape_json(&claims.tenant_id));
        body.push_str("\",\"role\":\"");
        body.push_str(&escape_json(&claims.role));
        body.push_str("\",\"jti\":\"");
        body.push_str(&escape_json(&claims.jwt_id));
        body.push_str("\",\"iss\":\"");
        body.push_str(&escape_json(&claims.issuer));
        body.push_str("\",\"aud\":\"");
        body.push_str(&escape_json(&claims.audience));
        body.push_str(&format!(
            "\",\"iat\":{},\"exp\":{},\"mfa_verified\":{}",
            claims.issued_at, claims.expires_at, claims.mfa_verified
        ));
    }
    if let Some(reason) = result.reason.as_ref() {
        body.push_str(",\"reason\":\"");
        body.push_str(&escape_json(reason));
        body.push('"');
    }
    body.push_str("}\n");
    HttpProbeResponse::new(200, "application/json", body)
}

fn endpoint_logout(engine: &AuthEngine, request: &HttpRequest) -> HttpProbeResponse {
    let token = match parse_json_string(request.body.trim(), "token") {
        Some(value) => value,
        None => return bad_request("token field is required"),
    };
    match engine.logout(&token) {
        Ok(()) => HttpProbeResponse::new(200, "application/json", "{\"revoked\":true}\n"),
        Err(AuthSidecarError::JwtMalformed)
        | Err(AuthSidecarError::JwtExpired)
        | Err(AuthSidecarError::JwtBadSignature)
        | Err(AuthSidecarError::JwtRevoked)
        | Err(AuthSidecarError::JwtNotYetValid)
        | Err(AuthSidecarError::JwtBadIssuer)
        | Err(AuthSidecarError::JwtBadAudience)
        | Err(AuthSidecarError::JwtUnsupportedAlgorithm) => unauthorized("token rejected"),
        Err(error) => bad_request(&error.to_string()),
    }
}

fn endpoint_totp_enroll(engine: &AuthEngine, request: &HttpRequest) -> HttpProbeResponse {
    let username = match parse_json_string(request.body.trim(), "username") {
        Some(value) => value,
        None => return bad_request("username field is required"),
    };
    match engine.enroll_totp(&TotpEnrollment { username }) {
        Ok(response) => {
            let body = format!(
                "{{\"username\":\"{user}\",\"secret_base32\":\"{secret}\",\"period_seconds\":{period},\"digits\":{digits},\"algorithm\":\"{algo}\",\"otpauth_uri\":\"{uri}\"}}\n",
                user = escape_json(&response.username),
                secret = escape_json(&response.secret_base32),
                period = response.period_seconds,
                digits = response.digits,
                algo = response.algorithm,
                uri = escape_json(&response.otpauth_uri),
            );
            HttpProbeResponse::new(200, "application/json", body)
        }
        Err(AuthSidecarError::UnknownUser) => not_found("user not found"),
        Err(AuthSidecarError::TotpAlreadyEnrolled) => HttpProbeResponse::new(
            409,
            "application/json",
            "{\"error\":\"already enrolled\"}\n",
        ),
        Err(error) => bad_request(&error.to_string()),
    }
}

fn endpoint_totp_verify(engine: &AuthEngine, request: &HttpRequest) -> HttpProbeResponse {
    let username = match parse_json_string(request.body.trim(), "username") {
        Some(value) => value,
        None => return bad_request("username field is required"),
    };
    let code = match parse_json_string(request.body.trim(), "code") {
        Some(value) => value,
        None => return bad_request("code field is required"),
    };
    match engine.verify_totp(&TotpVerifyRequest { username, code }) {
        Ok(()) => HttpProbeResponse::new(200, "application/json", "{\"verified\":true}\n"),
        Err(AuthSidecarError::TotpCodeInvalid) | Err(AuthSidecarError::TotpNotEnrolled) => {
            unauthorized("totp rejected")
        }
        Err(AuthSidecarError::MfaAttemptsExceeded) => unauthorized("mfa attempts exceeded"),
        Err(AuthSidecarError::UnknownUser) => not_found("user not found"),
        Err(error) => bad_request(&error.to_string()),
    }
}

fn endpoint_oidc_login(engine: &AuthEngine, request: &HttpRequest) -> HttpProbeResponse {
    let provider = match query_param(&request.query, "provider") {
        Some(value) => value,
        None => return bad_request("provider query field is required"),
    };
    let redirect_uri = match query_param(&request.query, "redirect_uri") {
        Some(value) => value,
        None => return bad_request("redirect_uri query field is required"),
    };
    match engine.oidc_login(&OidcLoginRequest {
        provider,
        redirect_uri,
    }) {
        Ok(response) => {
            let body = format!(
                r#"{{"provider":"{provider}","authorization_url":"{url}","state":"{state}","nonce":"{nonce}","redirect_uri":"{redirect_uri}","expires_in":{expires}}}
"#,
                provider = escape_json(&response.provider),
                url = escape_json(&response.authorization_url),
                state = escape_json(&response.state),
                nonce = escape_json(&response.nonce),
                redirect_uri = escape_json(&response.redirect_uri),
                expires = response.expires_in,
            );
            HttpProbeResponse::new(200, "application/json", body)
        }
        Err(AuthSidecarError::UnknownProvider) => not_found("oidc provider not found"),
        Err(AuthSidecarError::InvalidRedirectUri) => bad_request("redirect_uri is not allowed"),
        Err(error) => bad_request(&error.to_string()),
    }
}

fn endpoint_oidc_callback(engine: &AuthEngine, request: &HttpRequest) -> HttpProbeResponse {
    let provider = match query_param(&request.query, "provider") {
        Some(value) => value,
        None => return bad_request("provider query field is required"),
    };
    let redirect_uri = match query_param(&request.query, "redirect_uri") {
        Some(value) => value,
        None => return bad_request("redirect_uri query field is required"),
    };
    let state = match query_param(&request.query, "state") {
        Some(value) => value,
        None => return bad_request("state query field is required"),
    };
    let nonce = match query_param(&request.query, "nonce") {
        Some(value) => value,
        None => return bad_request("nonce query field is required"),
    };
    let code = match query_param(&request.query, "code") {
        Some(value) => value,
        None => return bad_request("code query field is required"),
    };
    match engine.validate_oidc_callback(&OidcCallbackRequest {
        provider,
        redirect_uri,
        state,
        nonce,
        code,
    }) {
        Ok(validation) => {
            let body = format!(
                r#"{{"error":"idp_exchange_unavailable","provider":"{provider}","redirect_uri":"{redirect_uri}","detail":"OIDC state, nonce, and redirect_uri validated; token exchange is not enabled"}}
"#,
                provider = escape_json(&validation.provider),
                redirect_uri = escape_json(&validation.redirect_uri),
            );
            HttpProbeResponse::new(501, "application/json", body)
        }
        Err(AuthSidecarError::UnknownProvider) => not_found("oidc provider not found"),
        Err(AuthSidecarError::InvalidRedirectUri) => bad_request("redirect_uri is not allowed"),
        Err(AuthSidecarError::InvalidOidcState) | Err(AuthSidecarError::OidcStateExpired) => {
            unauthorized("oidc callback rejected")
        }
        Err(error) => bad_request(&error.to_string()),
    }
}

fn endpoint_register_user(engine: &AuthEngine, request: &HttpRequest) -> HttpProbeResponse {
    let body = request.body.trim();
    let username = match parse_json_string(body, "username") {
        Some(value) => value,
        None => return bad_request("username field is required"),
    };
    let password = match parse_json_string(body, "password") {
        Some(value) => value,
        None => return bad_request("password field is required"),
    };
    let role = parse_json_string(body, "role").unwrap_or_else(|| "authenticated".to_string());
    let tenant_id = match parse_json_string(body, "tenant_id") {
        Some(value) => value,
        None => return bad_request("tenant_id field is required"),
    };
    match engine.register_user(&username, &password, &role, &tenant_id) {
        Ok(()) => HttpProbeResponse::new(201, "application/json", "{\"registered\":true}\n"),
        Err(error) => bad_request(&error.to_string()),
    }
}

fn alpha_response(surface: &'static str) -> HttpProbeResponse {
    let body = format!(
        "{{\"error\":\"alpha\",\"surface\":\"{}\",\"detail\":\"alpha boundary, runtime is not enabled\"}}\n",
        surface
    );
    HttpProbeResponse::new(501, "application/json", body)
}

fn fall_back_to_probe(parsed: &HttpRequest, original: &[u8]) -> HttpProbeResponse {
    let mut runtime = SidecarRuntime::ready("auth-sidecar");
    match runtime.handle_http_bytes(original) {
        Ok(response) => response,
        Err(error) => bad_request(&format!(
            "{error} (method {} path {})",
            parsed.method, parsed.path
        )),
    }
}

fn render_claims(claims: &VerifiedClaims) -> String {
    format!(
        "{{\"sub\":\"{sub}\",\"tenant_id\":\"{tenant}\",\"role\":\"{role}\",\"jti\":\"{jti}\",\"iss\":\"{iss}\",\"aud\":\"{aud}\",\"iat\":{iat},\"exp\":{exp},\"mfa_verified\":{mfa}}}\n",
        sub = escape_json(&claims.subject),
        tenant = escape_json(&claims.tenant_id),
        role = escape_json(&claims.role),
        jti = escape_json(&claims.jwt_id),
        iss = escape_json(&claims.issuer),
        aud = escape_json(&claims.audience),
        iat = claims.issued_at,
        exp = claims.expires_at,
        mfa = claims.mfa_verified,
    )
}

fn bad_request(detail: &str) -> HttpProbeResponse {
    let body = format!(
        "{{\"error\":\"bad_request\",\"detail\":\"{}\"}}\n",
        escape_json(detail)
    );
    HttpProbeResponse::new(400, "application/json", body)
}

fn unauthorized(detail: &str) -> HttpProbeResponse {
    let body = format!(
        "{{\"error\":\"unauthorized\",\"detail\":\"{}\"}}\n",
        escape_json(detail)
    );
    HttpProbeResponse::new(401, "application/json", body)
}

fn not_found(detail: &str) -> HttpProbeResponse {
    let body = format!(
        "{{\"error\":\"not_found\",\"detail\":\"{}\"}}\n",
        escape_json(detail)
    );
    HttpProbeResponse::new(404, "application/json", body)
}

fn query_param(query: &str, key: &str) -> Option<String> {
    query.split('&').find_map(|pair| {
        let (name, value) = pair.split_once('=')?;
        if name == key {
            url_decode(value)
        } else {
            None
        }
    })
}

fn url_decode(input: &str) -> Option<String> {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'+' => out.push(b' '),
            b'%' if i + 2 < bytes.len() => {
                let high = hex_value(bytes[i + 1])?;
                let low = hex_value(bytes[i + 2])?;
                out.push((high << 4) | low);
                i += 2;
            }
            byte => out.push(byte),
        }
        i += 1;
    }
    String::from_utf8(out).ok()
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn escape_json(input: &str) -> String {
    input
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

/// Minimal flat JSON string extractor. We control both encoder and decoder and
/// only handle one level of escaping.
fn parse_json_string(body: &str, key: &str) -> Option<String> {
    let needle = format!("\"{key}\":");
    let start = body.find(&needle)? + needle.len();
    let bytes = body.as_bytes();
    let mut i = start;
    while i < bytes.len() && (bytes[i] == b' ' || bytes[i] == b'\t' || bytes[i] == b'\n') {
        i += 1;
    }
    if i >= bytes.len() || bytes[i] != b'"' {
        return None;
    }
    i += 1;
    let mut out = String::new();
    while i < bytes.len() {
        let b = bytes[i];
        if b == b'\\' && i + 1 < bytes.len() {
            let escape = bytes[i + 1];
            match escape {
                b'"' => out.push('"'),
                b'\\' => out.push('\\'),
                b'n' => out.push('\n'),
                b'r' => out.push('\r'),
                b't' => out.push('\t'),
                _ => out.push(escape as char),
            }
            i += 2;
            continue;
        }
        if b == b'"' {
            return Some(out);
        }
        out.push(b as char);
        i += 1;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{canonical_claims, AuthEngine, OidcProviderConfig};

    fn engine() -> AuthEngine {
        AuthEngine::with_ttl(
            "https://auth.example.com",
            "postgres",
            b"http-test-secret".to_vec(),
            300,
        )
    }

    fn oidc_engine() -> AuthEngine {
        AuthEngine::with_runtime_config(
            "https://auth.example.com",
            "postgres",
            b"http-test-secret".to_vec(),
            300,
            vec![OidcProviderConfig {
                name: "stub".to_string(),
                issuer_url: "https://idp.example.com".to_string(),
                authorization_endpoint: "https://idp.example.com/oauth2/v1/authorize".to_string(),
                client_id: "ai-blaise-client".to_string(),
                client_secret_ref: "k8s://auth/idp-client-secret".to_string(),
                redirect_uris: vec!["https://db.example.com/auth/oidc/callback".to_string()],
                scopes: vec!["openid".to_string(), "email".to_string()],
            }],
            3,
        )
        .expect("engine")
    }

    fn get_request(target: &str) -> Vec<u8> {
        format!("GET {target} HTTP/1.1\r\nHost: localhost\r\n\r\n").into_bytes()
    }

    fn post_request(path: &str, body: &str) -> Vec<u8> {
        format!(
            "POST {path} HTTP/1.1\r\nHost: localhost\r\ncontent-type: application/json\r\ncontent-length: {len}\r\n\r\n{body}",
            len = body.len()
        )
        .into_bytes()
    }

    #[test]
    fn verify_endpoint_round_trip() {
        let engine = engine();
        let token = engine.issue_token(canonical_claims()).expect("issue");
        let body = format!("{{\"token\":\"{}\"}}", token);
        let response = handle_http_bytes(&engine, &post_request("/auth/verify", &body));
        assert_eq!(response.status_code, 200);
        assert!(response.body.contains("\"sub\":\"user-123\""));
        assert!(response.body.contains("\"tenant_id\":\"tenant-a\""));
    }

    #[test]
    fn verify_endpoint_rejects_garbage() {
        let engine = engine();
        let response = handle_http_bytes(
            &engine,
            &post_request("/auth/verify", "{\"token\":\"not.a.jwt\"}"),
        );
        assert_eq!(response.status_code, 401);
    }

    #[test]
    fn login_endpoint_requires_known_user() {
        let engine = engine();
        let body = "{\"username\":\"ghost\",\"password\":\"hunter2\"}";
        let response = handle_http_bytes(&engine, &post_request("/auth/login", body));
        assert_eq!(response.status_code, 401);
    }

    #[test]
    fn login_endpoint_issues_token() {
        let engine = engine();
        engine
            .register_user(
                "alice",
                "hunter2-correct-horse",
                "authenticated",
                "tenant-a",
            )
            .expect("register");
        let body = "{\"username\":\"alice\",\"password\":\"hunter2-correct-horse\"}";
        let response = handle_http_bytes(&engine, &post_request("/auth/login", body));
        assert_eq!(response.status_code, 200);
        assert!(response.body.contains("\"access_token\""));
        assert!(response.body.contains("\"refresh_token\""));
    }

    #[test]
    fn introspect_inactive_token_returns_active_false() {
        let engine = engine();
        let response = handle_http_bytes(
            &engine,
            &post_request("/auth/introspect", "{\"token\":\"not.a.jwt\"}"),
        );
        assert_eq!(response.status_code, 200);
        assert!(response.body.contains("\"active\":false"));
    }

    #[test]
    fn oidc_routes_validate_login_and_fail_closed_callback() {
        let engine = oidc_engine();
        let login = handle_http_bytes(
            &engine,
            &get_request("/auth/oidc/login?provider=stub&redirect_uri=https%3A%2F%2Fdb.example.com%2Fauth%2Foidc%2Fcallback"),
        );
        assert_eq!(login.status_code, 200);
        assert!(login.body.contains("authorization_url"));
        assert!(login.body.contains("client_id=ai-blaise-client"));
        let state = parse_json_string(&login.body, "state").expect("state");
        let nonce = parse_json_string(&login.body, "nonce").expect("nonce");

        let bad_callback = handle_http_bytes(
            &engine,
            &get_request(&format!("/auth/oidc/callback?provider=stub&redirect_uri=https%3A%2F%2Fdb.example.com%2Fauth%2Foidc%2Fcallback&state={state}&nonce=wrong&code=stub-code")),
        );
        assert_eq!(bad_callback.status_code, 401);

        let callback = handle_http_bytes(
            &engine,
            &get_request(&format!("/auth/oidc/callback?provider=stub&redirect_uri=https%3A%2F%2Fdb.example.com%2Fauth%2Foidc%2Fcallback&state={state}&nonce={nonce}&code=stub-code")),
        );
        assert_eq!(callback.status_code, 501);
        assert!(callback.body.contains("idp_exchange_unavailable"));

        let replay = handle_http_bytes(
            &engine,
            &get_request(&format!("/auth/oidc/callback?provider=stub&redirect_uri=https%3A%2F%2Fdb.example.com%2Fauth%2Foidc%2Fcallback&state={state}&nonce={nonce}&code=stub-code")),
        );
        assert_eq!(replay.status_code, 401);
    }

    #[test]
    fn oidc_login_rejects_bad_provider_or_redirect() {
        let engine = oidc_engine();
        let missing = handle_http_bytes(
            &engine,
            &get_request("/auth/oidc/login?provider=missing&redirect_uri=https%3A%2F%2Fdb.example.com%2Fauth%2Foidc%2Fcallback"),
        );
        assert_eq!(missing.status_code, 404);
        let redirect = handle_http_bytes(
            &engine,
            &get_request("/auth/oidc/login?provider=stub&redirect_uri=https%3A%2F%2Fevil.example.com%2Fcallback"),
        );
        assert_eq!(redirect.status_code, 400);
    }

    #[test]
    fn webauthn_endpoint_reports_alpha() {
        let engine = engine();
        let response =
            handle_http_bytes(&engine, &post_request("/auth/mfa/webauthn/register", "{}"));
        assert_eq!(response.status_code, 501);
        assert!(response.body.contains("alpha"));
    }

    #[test]
    fn healthz_falls_through_to_probe() {
        let engine = engine();
        let request = b"GET /healthz HTTP/1.1\r\nHost: localhost\r\n\r\n";
        let response = handle_http_bytes(&engine, request);
        assert_eq!(response.status_code, 200);
        assert!(response.body.contains("\"component\":\"auth-sidecar\""));
    }
}
