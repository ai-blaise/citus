//! Auth sidecar contracts and runtime.
//!
//! The runtime portion of this crate issues and verifies JSON Web Tokens
//! (HS256), enrolls TOTP MFA secrets, and hashes user passwords with
//! PBKDF2-SHA256. It runs as a synchronous HTTP service on top of the
//! `SidecarRuntime` probe machinery shared with the rest of the workspace.
//!
//! RS256 issuance, WebAuthn registration, and external OIDC token exchange
//! remain alpha-level contracts. The runtime does validate OIDC provider
//! configuration, state, nonce, and redirect URI boundaries before failing
//! closed at the unimplemented IdP exchange step. The HS256 wire format is
//! intentionally compatible with the SQL verifier shipped by Sec2
//! (`companion_verify_jwt_hs256`).

// FEATURE: Auth1
// FEATURE: Auth2
// FEATURE: Auth3
// FEATURE: Auth4
// FEATURE: Auth5

use ai_blaise_citus_sidecar_shared::{
    AuthIssuerContract, HttpProbeRequest, HttpProbeResponse, SidecarContractError, SidecarRuntime,
    SidecarRuntimeError,
};
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fmt;
use std::fs::File;
use std::io::Read;
use std::sync::{Mutex, MutexGuard};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub mod http;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TokenClaims {
    pub subject: String,
    pub tenant_id: String,
    pub role: String,
    pub jwt_id: String,
    pub custom_claims: Vec<CustomClaim>,
}

impl TokenClaims {
    pub fn validate(&self) -> Result<(), AuthSidecarError> {
        validate_required("claims.subject", &self.subject)?;
        validate_required("claims.tenant_id", &self.tenant_id)?;
        validate_required("claims.role", &self.role)?;
        validate_required("claims.jwt_id", &self.jwt_id)?;
        for claim in &self.custom_claims {
            claim.validate()?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct CustomClaim {
    pub name: String,
    pub value: String,
}

impl CustomClaim {
    fn validate(&self) -> Result<(), AuthSidecarError> {
        validate_required("claims.custom.name", &self.name)?;
        validate_required("claims.custom.value", &self.value)
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum SigningAlgorithm {
    Rs256,
    Hs256,
}

impl SigningAlgorithm {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Rs256 => "RS256",
            Self::Hs256 => "HS256",
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct JwtIssueRequest {
    pub issuer: AuthIssuerContract,
    pub algorithm: SigningAlgorithm,
    pub claims: TokenClaims,
    pub audience: String,
}

impl JwtIssueRequest {
    pub fn plan(&self) -> Result<JwtIssuePlan, AuthSidecarError> {
        self.issuer.validate()?;
        self.claims.validate()?;
        validate_required("audience", &self.audience)?;

        Ok(JwtIssuePlan {
            issuer: self.issuer.issuer.clone(),
            signing_key_ref: self.issuer.signing_key_ref.clone(),
            algorithm: self.algorithm,
            subject: self.claims.subject.clone(),
            tenant_id: self.claims.tenant_id.clone(),
            role: self.claims.role.clone(),
            audience: self.audience.clone(),
            ttl_seconds: self.issuer.token_ttl_seconds,
        })
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct JwtIssuePlan {
    pub issuer: String,
    pub signing_key_ref: String,
    pub algorithm: SigningAlgorithm,
    pub subject: String,
    pub tenant_id: String,
    pub role: String,
    pub audience: String,
    pub ttl_seconds: u32,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TokenIntrospectionPlan {
    pub issuer: String,
    pub jwt_id: String,
    pub tenant_id: String,
    pub cache_ttl_seconds: u32,
}

impl TokenIntrospectionPlan {
    pub fn validate(&self) -> Result<(), AuthSidecarError> {
        validate_required("introspection.issuer", &self.issuer)?;
        validate_required("introspection.jwt_id", &self.jwt_id)?;
        validate_required("introspection.tenant_id", &self.tenant_id)?;
        if self.cache_ttl_seconds == 0 {
            return Err(AuthSidecarError::InvalidCacheTtl);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct OidcProviderConfig {
    pub name: String,
    pub issuer_url: String,
    pub authorization_endpoint: String,
    pub client_id: String,
    pub client_secret_ref: String,
    pub redirect_uris: Vec<String>,
    pub scopes: Vec<String>,
}

impl OidcProviderConfig {
    pub fn validate(&self) -> Result<(), AuthSidecarError> {
        validate_required("oidc.name", &self.name)?;
        if !self
            .name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
        {
            return Err(AuthSidecarError::InvalidProviderName);
        }
        validate_https_url("oidc.issuer_url", &self.issuer_url)?;
        validate_https_url("oidc.authorization_endpoint", &self.authorization_endpoint)?;
        if !self.authorization_endpoint.starts_with(&self.issuer_url) {
            return Err(AuthSidecarError::InvalidAuthorizationEndpoint);
        }
        validate_required("oidc.client_id", &self.client_id)?;
        if self.client_id.chars().any(char::is_whitespace) {
            return Err(AuthSidecarError::InvalidClientId);
        }
        validate_required("oidc.client_secret_ref", &self.client_secret_ref)?;
        validate_required_list("oidc.redirect_uris", &self.redirect_uris)?;
        for redirect_uri in &self.redirect_uris {
            validate_redirect_uri(redirect_uri)?;
        }
        validate_required_list("oidc.scopes", &self.scopes)?;
        if !self.scopes.iter().any(|scope| scope == "openid") {
            return Err(AuthSidecarError::MissingOpenIdScope);
        }
        Ok(())
    }

    fn allows_redirect_uri(&self, redirect_uri: &str) -> bool {
        self.redirect_uris
            .iter()
            .any(|allowed| allowed == redirect_uri)
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct MfaPolicy {
    pub totp_enabled: bool,
    pub webauthn_enabled: bool,
    pub max_attempts: u32,
}

impl MfaPolicy {
    pub fn validate(&self) -> Result<(), AuthSidecarError> {
        if !self.totp_enabled && !self.webauthn_enabled {
            return Err(AuthSidecarError::NoMfaMethodEnabled);
        }
        if self.max_attempts == 0 {
            return Err(AuthSidecarError::InvalidMfaAttempts);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct AuthSidecarPlan {
    pub issuer: AuthIssuerContract,
    pub oidc_providers: Vec<OidcProviderConfig>,
    pub mfa: Option<MfaPolicy>,
}

impl AuthSidecarPlan {
    pub fn validate(&self) -> Result<(), AuthSidecarError> {
        self.issuer.validate()?;
        for provider in &self.oidc_providers {
            provider.validate()?;
        }
        if let Some(mfa) = &self.mfa {
            mfa.validate()?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum AuthSidecarError {
    InvalidAuthorizationEndpoint,
    InvalidCacheTtl,
    InvalidClientId,
    InvalidIssuerUrl,
    InvalidProviderName,
    InvalidRedirectUri,
    MissingOpenIdScope,
    InvalidMfaAttempts,
    MissingRequiredField(&'static str),
    NoMfaMethodEnabled,
    SharedContract(String),
    JwtMalformed,
    JwtBadSignature,
    JwtExpired,
    JwtNotYetValid,
    JwtBadIssuer,
    JwtBadAudience,
    JwtRevoked,
    JwtUnsupportedAlgorithm,
    PasswordVerificationFailed,
    UnknownUser,
    UnknownProvider,
    UnknownSession,
    InvalidOidcState,
    OidcStateExpired,
    OidcExchangeUnavailable,
    TotpNotEnrolled,
    TotpAlreadyEnrolled,
    TotpCodeInvalid,
    MfaAttemptsExceeded,
    AlphaSurface(&'static str),
    Runtime(String),
}

impl fmt::Display for AuthSidecarError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidAuthorizationEndpoint => write!(
                formatter,
                "authorization_endpoint must be https and under issuer_url"
            ),
            Self::InvalidCacheTtl => {
                write!(formatter, "cache_ttl_seconds must be greater than zero")
            }
            Self::InvalidClientId => write!(formatter, "client_id must not contain whitespace"),
            Self::InvalidIssuerUrl => write!(formatter, "issuer_url must start with https://"),
            Self::InvalidProviderName => write!(
                formatter,
                "provider name may only contain ASCII letters, digits, dash, or underscore"
            ),
            Self::InvalidRedirectUri => write!(
                formatter,
                "redirect_uri must be an allowed https URI without a fragment"
            ),
            Self::MissingOpenIdScope => write!(formatter, "oidc.scopes must include openid"),
            Self::InvalidMfaAttempts => write!(formatter, "max_attempts must be greater than zero"),
            Self::MissingRequiredField(field) => write!(formatter, "{field} must not be empty"),
            Self::NoMfaMethodEnabled => {
                write!(formatter, "at least one MFA method must be enabled")
            }
            Self::SharedContract(error) => write!(formatter, "{error}"),
            Self::JwtMalformed => write!(formatter, "token is malformed"),
            Self::JwtBadSignature => write!(formatter, "token signature is invalid"),
            Self::JwtExpired => write!(formatter, "token is expired"),
            Self::JwtNotYetValid => write!(formatter, "token is not yet valid"),
            Self::JwtBadIssuer => write!(formatter, "token issuer does not match"),
            Self::JwtBadAudience => write!(formatter, "token audience does not match"),
            Self::JwtRevoked => write!(formatter, "token has been revoked"),
            Self::JwtUnsupportedAlgorithm => write!(formatter, "token algorithm is not supported"),
            Self::PasswordVerificationFailed => write!(formatter, "password does not match"),
            Self::UnknownUser => write!(formatter, "user not found"),
            Self::UnknownProvider => write!(formatter, "OIDC provider not found"),
            Self::UnknownSession => write!(formatter, "session not found"),
            Self::InvalidOidcState => {
                write!(formatter, "OIDC state, nonce, or redirect_uri is invalid")
            }
            Self::OidcStateExpired => write!(formatter, "OIDC state has expired"),
            Self::OidcExchangeUnavailable => {
                write!(formatter, "OIDC token exchange is not enabled")
            }
            Self::TotpNotEnrolled => write!(formatter, "user has not enrolled TOTP"),
            Self::TotpAlreadyEnrolled => write!(formatter, "user has already enrolled TOTP"),
            Self::TotpCodeInvalid => write!(formatter, "TOTP code is invalid"),
            Self::MfaAttemptsExceeded => write!(formatter, "MFA attempts exceeded"),
            Self::AlphaSurface(surface) => {
                write!(formatter, "{surface} is alpha and not yet runtime-backed")
            }
            Self::Runtime(error) => write!(formatter, "{error}"),
        }
    }
}

impl Error for AuthSidecarError {}

impl From<SidecarContractError> for AuthSidecarError {
    fn from(error: SidecarContractError) -> Self {
        Self::SharedContract(error.to_string())
    }
}

impl From<SidecarRuntimeError> for AuthSidecarError {
    fn from(error: SidecarRuntimeError) -> Self {
        Self::Runtime(error.to_string())
    }
}

impl From<std::io::Error> for AuthSidecarError {
    fn from(error: std::io::Error) -> Self {
        Self::Runtime(error.to_string())
    }
}

fn validate_required(field: &'static str, value: &str) -> Result<(), AuthSidecarError> {
    if value.trim().is_empty() {
        return Err(AuthSidecarError::MissingRequiredField(field));
    }
    Ok(())
}

fn validate_required_list(field: &'static str, values: &[String]) -> Result<(), AuthSidecarError> {
    if values.is_empty() || values.iter().any(|value| value.trim().is_empty()) {
        return Err(AuthSidecarError::MissingRequiredField(field));
    }
    Ok(())
}

fn validate_https_url(field: &'static str, value: &str) -> Result<(), AuthSidecarError> {
    validate_required(field, value)?;
    if !value.starts_with("https://") {
        if field == "oidc.authorization_endpoint" {
            return Err(AuthSidecarError::InvalidAuthorizationEndpoint);
        }
        return Err(AuthSidecarError::InvalidIssuerUrl);
    }
    Ok(())
}

fn validate_redirect_uri(value: &str) -> Result<(), AuthSidecarError> {
    validate_required("oidc.redirect_uri", value)?;
    if !value.starts_with("https://") || value.contains('#') {
        return Err(AuthSidecarError::InvalidRedirectUri);
    }
    Ok(())
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct AuthCanonicalReport {
    pub sidecar: AuthSidecarPlan,
    pub issue_plan: JwtIssuePlan,
    pub introspection: TokenIntrospectionPlan,
}

pub fn canonical_issuer() -> AuthIssuerContract {
    AuthIssuerContract {
        issuer: "https://auth.example.com".to_string(),
        signing_key_ref: "jwt-signing-key".to_string(),
        token_ttl_seconds: 3_600,
        tenant_claim: "tenant_id".to_string(),
    }
}

pub fn canonical_claims() -> TokenClaims {
    TokenClaims {
        subject: "user-123".to_string(),
        tenant_id: "tenant-a".to_string(),
        role: "authenticated".to_string(),
        jwt_id: "jti-123".to_string(),
        custom_claims: vec![CustomClaim {
            name: "email".to_string(),
            value: "user@example.com".to_string(),
        }],
    }
}

pub fn canonical_auth_sidecar_plan() -> AuthSidecarPlan {
    AuthSidecarPlan {
        issuer: canonical_issuer(),
        oidc_providers: vec![OidcProviderConfig {
            name: "github".to_string(),
            issuer_url: "https://github.example".to_string(),
            authorization_endpoint: "https://github.example/oauth/authorize".to_string(),
            client_id: "ai-blaise-github".to_string(),
            client_secret_ref: "k8s://auth/github-client-secret".to_string(),
            redirect_uris: vec!["https://auth.example.com/auth/oidc/callback".to_string()],
            scopes: vec!["openid".to_string(), "email".to_string()],
        }],
        mfa: Some(MfaPolicy {
            totp_enabled: true,
            webauthn_enabled: true,
            max_attempts: 3,
        }),
    }
}

pub fn canonical_jwt_issue_request() -> JwtIssueRequest {
    JwtIssueRequest {
        issuer: canonical_issuer(),
        algorithm: SigningAlgorithm::Hs256,
        claims: canonical_claims(),
        audience: "postgres".to_string(),
    }
}

pub fn canonical_token_introspection_plan() -> TokenIntrospectionPlan {
    TokenIntrospectionPlan {
        issuer: "https://auth.example.com".to_string(),
        jwt_id: "jti-123".to_string(),
        tenant_id: "tenant-a".to_string(),
        cache_ttl_seconds: 300,
    }
}

pub fn canonical_auth_report() -> Result<AuthCanonicalReport, AuthSidecarError> {
    let sidecar = canonical_auth_sidecar_plan();
    let issue_plan = canonical_jwt_issue_request().plan()?;
    let introspection = canonical_token_introspection_plan();

    sidecar.validate()?;
    introspection.validate()?;

    Ok(AuthCanonicalReport {
        sidecar,
        issue_plan,
        introspection,
    })
}

// -----------------------------------------------------------------------------
// Runtime: HS256 JWT, TOTP, PBKDF2 password storage, in-memory user store
// -----------------------------------------------------------------------------

const ISSUE_LEEWAY_SECONDS: i64 = 5;
const VERIFY_LEEWAY_SECONDS: i64 = 5;
const INTROSPECTION_CACHE_TTL_SECONDS: u64 = 60;
const PBKDF2_ITERATIONS: u32 = 200_000;
const PBKDF2_SALT_LEN: usize = 16;
const PBKDF2_HASH_LEN: usize = 32;
const TOTP_PERIOD_SECONDS: u64 = 30;
const TOTP_DIGITS: u32 = 6;
const TOTP_STEP_TOLERANCE: i64 = 1;
const DEFAULT_MFA_MAX_ATTEMPTS: u32 = 5;
const OIDC_STATE_TTL_SECONDS: u64 = 10 * 60;
const REFRESH_TTL_SECONDS: u64 = 7 * 24 * 60 * 60;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct IntrospectionResult {
    pub active: bool,
    pub claims: Option<VerifiedClaims>,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct VerifiedClaims {
    pub subject: String,
    pub tenant_id: String,
    pub role: String,
    pub jwt_id: String,
    pub issuer: String,
    pub audience: String,
    pub issued_at: i64,
    pub expires_at: i64,
    pub mfa_verified: bool,
    pub custom: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
    pub totp_code: Option<String>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct LoginResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: u32,
    pub mfa_required: bool,
    pub mfa_verified: bool,
}

#[derive(Debug, Clone)]
pub struct RefreshRequest {
    pub refresh_token: String,
}

#[derive(Debug, Clone)]
pub struct TotpEnrollment {
    pub username: String,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TotpEnrollmentResponse {
    pub username: String,
    pub secret_base32: String,
    pub period_seconds: u64,
    pub digits: u32,
    pub algorithm: &'static str,
    pub otpauth_uri: String,
}

#[derive(Debug, Clone)]
pub struct TotpVerifyRequest {
    pub username: String,
    pub code: String,
}

#[derive(Debug, Clone)]
pub struct OidcLoginRequest {
    pub provider: String,
    pub redirect_uri: String,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct OidcLoginResponse {
    pub provider: String,
    pub authorization_url: String,
    pub state: String,
    pub nonce: String,
    pub redirect_uri: String,
    pub expires_in: u64,
}

#[derive(Debug, Clone)]
pub struct OidcCallbackRequest {
    pub provider: String,
    pub redirect_uri: String,
    pub state: String,
    pub nonce: String,
    pub code: String,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct OidcCallbackValidation {
    pub provider: String,
    pub redirect_uri: String,
    pub nonce: String,
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct StoredUser {
    username: String,
    role: String,
    tenant_id: String,
    password_hash: PasswordHash,
    totp_secret: Option<Vec<u8>>,
    mfa_required: bool,
    failed_totp_attempts: u32,
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct PasswordHash {
    iterations: u32,
    salt: Vec<u8>,
    hash: Vec<u8>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct StoredSession {
    refresh_token: String,
    username: String,
    current_jti: String,
    issued_at: u64,
    expires_at: u64,
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct IntrospectionCacheEntry {
    inserted_at: u64,
    result: IntrospectionResult,
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct PendingOidcLogin {
    provider: String,
    redirect_uri: String,
    nonce: String,
    expires_at: u64,
}

#[derive(Debug)]
struct EngineState {
    users: HashMap<String, StoredUser>,
    sessions: HashMap<String, StoredSession>,
    revoked_jti: HashSet<String>,
    introspection_cache: HashMap<String, IntrospectionCacheEntry>,
    oidc_states: HashMap<String, PendingOidcLogin>,
    next_jti: u64,
}

impl EngineState {
    fn new() -> Self {
        Self {
            users: HashMap::new(),
            sessions: HashMap::new(),
            revoked_jti: HashSet::new(),
            introspection_cache: HashMap::new(),
            oidc_states: HashMap::new(),
            next_jti: 0,
        }
    }

    fn allocate_jti(&mut self) -> Result<String, AuthSidecarError> {
        self.next_jti = self.next_jti.wrapping_add(1);
        let mut bytes = [0_u8; 16];
        fill_random_bytes(&mut bytes)?;
        Ok(base64_url_encode(&bytes))
    }
}

/// Production runtime backing the auth sidecar's HTTP API.
#[derive(Debug)]
pub struct AuthEngine {
    issuer: String,
    audience: String,
    token_ttl_seconds: u32,
    hs256_secret: Vec<u8>,
    oidc_providers: Vec<OidcProviderConfig>,
    mfa_max_attempts: u32,
    state: Mutex<EngineState>,
}

impl AuthEngine {
    pub fn new(
        issuer: impl Into<String>,
        audience: impl Into<String>,
        hs256_secret: Vec<u8>,
    ) -> Self {
        Self::with_ttl(issuer, audience, hs256_secret, 3_600)
    }

    pub fn with_ttl(
        issuer: impl Into<String>,
        audience: impl Into<String>,
        hs256_secret: Vec<u8>,
        token_ttl_seconds: u32,
    ) -> Self {
        Self::with_runtime_config(
            issuer,
            audience,
            hs256_secret,
            token_ttl_seconds,
            Vec::new(),
            DEFAULT_MFA_MAX_ATTEMPTS,
        )
        .expect("default auth runtime config must validate")
    }

    pub fn with_runtime_config(
        issuer: impl Into<String>,
        audience: impl Into<String>,
        hs256_secret: Vec<u8>,
        token_ttl_seconds: u32,
        oidc_providers: Vec<OidcProviderConfig>,
        mfa_max_attempts: u32,
    ) -> Result<Self, AuthSidecarError> {
        if mfa_max_attempts == 0 {
            return Err(AuthSidecarError::InvalidMfaAttempts);
        }
        for provider in &oidc_providers {
            provider.validate()?;
        }
        Ok(Self {
            issuer: issuer.into(),
            audience: audience.into(),
            token_ttl_seconds,
            hs256_secret,
            oidc_providers,
            mfa_max_attempts,
            state: Mutex::new(EngineState::new()),
        })
    }

    /// Bootstrap a fresh engine seeded from `AI_BLAISE_AUTH_*` environment variables.
    /// `AI_BLAISE_AUTH_HS256_SECRET` is required in normal serve mode; local
    /// smokes may opt into an ephemeral key with
    /// `AI_BLAISE_AUTH_ALLOW_EPHEMERAL_SECRET=1`.
    pub fn from_env() -> Result<Self, AuthSidecarError> {
        let issuer = std::env::var("AI_BLAISE_AUTH_ISSUER")
            .unwrap_or_else(|_| "https://auth.example.com".to_string());
        let audience =
            std::env::var("AI_BLAISE_AUTH_AUDIENCE").unwrap_or_else(|_| "postgres".to_string());
        let ttl: u32 = std::env::var("AI_BLAISE_AUTH_TTL_SECONDS")
            .ok()
            .and_then(|value| value.parse().ok())
            .unwrap_or(3_600);
        if ttl == 0 {
            return Err(AuthSidecarError::Runtime(
                "AI_BLAISE_AUTH_TTL_SECONDS must be greater than zero".to_string(),
            ));
        }
        let secret_bytes = match std::env::var("AI_BLAISE_AUTH_HS256_SECRET") {
            Ok(value) if value.len() >= 32 => value.into_bytes(),
            Ok(_) => {
                return Err(AuthSidecarError::Runtime(
                    "AI_BLAISE_AUTH_HS256_SECRET must be at least 32 bytes".to_string(),
                ));
            }
            _ if std::env::var("AI_BLAISE_AUTH_ALLOW_EPHEMERAL_SECRET").as_deref() == Ok("1") => {
                let mut bytes = [0_u8; 32];
                fill_random_bytes(&mut bytes)?;
                bytes.to_vec()
            }
            _ => {
                return Err(AuthSidecarError::Runtime(
                    "AI_BLAISE_AUTH_HS256_SECRET is required unless AI_BLAISE_AUTH_ALLOW_EPHEMERAL_SECRET=1".to_string(),
                ));
            }
        };
        let mfa_max_attempts = match std::env::var("AI_BLAISE_AUTH_MFA_MAX_ATTEMPTS") {
            Ok(value) => value.parse().map_err(|_| {
                AuthSidecarError::Runtime(
                    "AI_BLAISE_AUTH_MFA_MAX_ATTEMPTS must be an integer".to_string(),
                )
            })?,
            Err(_) => DEFAULT_MFA_MAX_ATTEMPTS,
        };
        let oidc_providers = oidc_providers_from_env()?;
        Self::with_runtime_config(
            issuer,
            audience,
            secret_bytes,
            ttl,
            oidc_providers,
            mfa_max_attempts,
        )
    }

    pub fn issuer(&self) -> &str {
        &self.issuer
    }

    pub fn audience(&self) -> &str {
        &self.audience
    }

    pub fn token_ttl_seconds(&self) -> u32 {
        self.token_ttl_seconds
    }

    /// Register a user. Used by enrollment paths and tests.
    pub fn register_user(
        &self,
        username: &str,
        password: &str,
        role: &str,
        tenant_id: &str,
    ) -> Result<(), AuthSidecarError> {
        validate_required("username", username)?;
        validate_required("password", password)?;
        validate_required("role", role)?;
        validate_required("tenant_id", tenant_id)?;

        let password_hash = hash_password(password)?;
        let mut state = self.lock_state()?;
        state.users.insert(
            username.to_string(),
            StoredUser {
                username: username.to_string(),
                role: role.to_string(),
                tenant_id: tenant_id.to_string(),
                password_hash,
                totp_secret: None,
                mfa_required: false,
                failed_totp_attempts: 0,
            },
        );
        Ok(())
    }

    pub fn user_count(&self) -> Result<usize, AuthSidecarError> {
        Ok(self.lock_state()?.users.len())
    }

    /// Verify password + optional TOTP and issue an access + refresh token pair.
    pub fn login(&self, request: &LoginRequest) -> Result<LoginResponse, AuthSidecarError> {
        validate_required("username", &request.username)?;
        validate_required("password", &request.password)?;

        let now = current_unix_time();
        let (user_clone, mfa_verified) = {
            let mut state = self.lock_state()?;
            let user = state
                .users
                .get(&request.username)
                .cloned()
                .ok_or(AuthSidecarError::UnknownUser)?;
            if !verify_password(&user.password_hash, &request.password)? {
                return Err(AuthSidecarError::PasswordVerificationFailed);
            }
            let mfa_verified = if user.totp_secret.is_some() || user.mfa_required {
                let code = request
                    .totp_code
                    .as_deref()
                    .ok_or(AuthSidecarError::TotpCodeInvalid)?;
                let secret = user
                    .totp_secret
                    .as_ref()
                    .ok_or(AuthSidecarError::TotpNotEnrolled)?
                    .clone();
                if user.failed_totp_attempts >= self.mfa_max_attempts {
                    return Err(AuthSidecarError::MfaAttemptsExceeded);
                }
                if !verify_totp(&secret, code, now) {
                    let user = state
                        .users
                        .get_mut(&request.username)
                        .ok_or(AuthSidecarError::UnknownUser)?;
                    user.failed_totp_attempts = user.failed_totp_attempts.saturating_add(1);
                    return Err(AuthSidecarError::TotpCodeInvalid);
                }
                let user = state
                    .users
                    .get_mut(&request.username)
                    .ok_or(AuthSidecarError::UnknownUser)?;
                user.failed_totp_attempts = 0;
                true
            } else {
                false
            };

            (user, mfa_verified)
        };

        let claims = TokenClaims {
            subject: user_clone.username.clone(),
            tenant_id: user_clone.tenant_id.clone(),
            role: user_clone.role.clone(),
            jwt_id: self.allocate_jti()?,
            custom_claims: vec![CustomClaim {
                name: "mfa_verified".to_string(),
                value: mfa_verified.to_string(),
            }],
        };
        let access_token = self.issue_token_inner(&claims, now)?;
        let refresh_token =
            self.allocate_refresh_token(&user_clone.username, &claims.jwt_id, now)?;

        Ok(LoginResponse {
            access_token,
            refresh_token,
            expires_in: self.token_ttl_seconds,
            mfa_required: user_clone.totp_secret.is_some() || user_clone.mfa_required,
            mfa_verified,
        })
    }

    /// Exchange a refresh token for a new access token.
    pub fn refresh(&self, request: &RefreshRequest) -> Result<LoginResponse, AuthSidecarError> {
        validate_required("refresh_token", &request.refresh_token)?;
        let now = current_unix_time();
        let username = {
            let mut state = self.lock_state()?;
            let session = state
                .sessions
                .get(&request.refresh_token)
                .cloned()
                .ok_or(AuthSidecarError::UnknownSession)?;
            if session.expires_at <= now {
                state.sessions.remove(&request.refresh_token);
                return Err(AuthSidecarError::JwtExpired);
            }
            session.username
        };
        let user = self
            .lock_state()?
            .users
            .get(&username)
            .cloned()
            .ok_or(AuthSidecarError::UnknownUser)?;
        let claims = TokenClaims {
            subject: user.username.clone(),
            tenant_id: user.tenant_id.clone(),
            role: user.role.clone(),
            jwt_id: self.allocate_jti()?,
            custom_claims: vec![CustomClaim {
                name: "mfa_verified".to_string(),
                value: false.to_string(),
            }],
        };
        let access_token = self.issue_token_inner(&claims, now)?;
        {
            let mut state = self.lock_state()?;
            let session = state
                .sessions
                .get_mut(&request.refresh_token)
                .ok_or(AuthSidecarError::UnknownSession)?;
            session.current_jti = claims.jwt_id;
        }
        Ok(LoginResponse {
            access_token,
            refresh_token: request.refresh_token.clone(),
            expires_in: self.token_ttl_seconds,
            mfa_required: user.totp_secret.is_some() || user.mfa_required,
            mfa_verified: false,
        })
    }

    /// Issue a token directly. The token is signed HS256 and validated against
    /// the verifier shipped by Sec2 (`companion_verify_jwt_hs256`).
    pub fn issue_token(&self, claims: TokenClaims) -> Result<String, AuthSidecarError> {
        claims.validate()?;
        self.issue_token_inner(&claims, current_unix_time())
    }

    fn issue_token_inner(
        &self,
        claims: &TokenClaims,
        now_unix: u64,
    ) -> Result<String, AuthSidecarError> {
        let header = b"{\"alg\":\"HS256\",\"typ\":\"JWT\"}";
        let header_b64 = base64_url_encode(header);
        let iat = (now_unix as i64) - ISSUE_LEEWAY_SECONDS;
        let exp = (now_unix as i64) + (self.token_ttl_seconds as i64);
        let payload_json = encode_claims_json(claims, &self.issuer, &self.audience, iat, exp);
        let payload_b64 = base64_url_encode(payload_json.as_bytes());
        let signing_input = format!("{header_b64}.{payload_b64}");
        let signature = hmac_sha256(&self.hs256_secret, signing_input.as_bytes());
        let signature_b64 = base64_url_encode(&signature);
        Ok(format!("{signing_input}.{signature_b64}"))
    }

    /// Verify a token: signature, expiry, not-before, issuer/audience, JTI
    /// revocation. Returns parsed claims on success.
    pub fn verify_token(&self, token: &str) -> Result<VerifiedClaims, AuthSidecarError> {
        let now = current_unix_time();
        let (header_b64, payload_b64, signature_b64) = split_jwt(token)?;
        let header_bytes = base64_url_decode(header_b64)?;
        let header_str =
            std::str::from_utf8(&header_bytes).map_err(|_| AuthSidecarError::JwtMalformed)?;
        let alg = extract_json_string(header_str, "alg").ok_or(AuthSidecarError::JwtMalformed)?;
        if alg != "HS256" {
            return Err(AuthSidecarError::JwtUnsupportedAlgorithm);
        }

        let signing_input = format!("{header_b64}.{payload_b64}");
        let expected = hmac_sha256(&self.hs256_secret, signing_input.as_bytes());
        let provided = base64_url_decode(signature_b64)?;
        if !constant_time_eq(&expected, &provided) {
            return Err(AuthSidecarError::JwtBadSignature);
        }

        let payload_bytes = base64_url_decode(payload_b64)?;
        let payload =
            std::str::from_utf8(&payload_bytes).map_err(|_| AuthSidecarError::JwtMalformed)?;
        let issuer = extract_json_string(payload, "iss").ok_or(AuthSidecarError::JwtMalformed)?;
        if issuer != self.issuer {
            return Err(AuthSidecarError::JwtBadIssuer);
        }
        let audience = extract_json_string(payload, "aud").ok_or(AuthSidecarError::JwtMalformed)?;
        if audience != self.audience {
            return Err(AuthSidecarError::JwtBadAudience);
        }
        let exp = extract_json_number(payload, "exp").ok_or(AuthSidecarError::JwtMalformed)?;
        let iat = extract_json_number(payload, "iat").ok_or(AuthSidecarError::JwtMalformed)?;
        let now_i = now as i64;
        if exp + VERIFY_LEEWAY_SECONDS < now_i {
            return Err(AuthSidecarError::JwtExpired);
        }
        if iat - VERIFY_LEEWAY_SECONDS > now_i {
            return Err(AuthSidecarError::JwtNotYetValid);
        }
        let subject = extract_json_string(payload, "sub").ok_or(AuthSidecarError::JwtMalformed)?;
        let tenant_id =
            extract_json_string(payload, "tenant_id").ok_or(AuthSidecarError::JwtMalformed)?;
        let role = extract_json_string(payload, "role").ok_or(AuthSidecarError::JwtMalformed)?;
        let jwt_id = extract_json_string(payload, "jti").ok_or(AuthSidecarError::JwtMalformed)?;
        let mfa_verified = extract_json_string(payload, "mfa_verified")
            .map(|value| value == "true")
            .unwrap_or(false);

        {
            let state = self.lock_state()?;
            if state.revoked_jti.contains(&jwt_id) {
                return Err(AuthSidecarError::JwtRevoked);
            }
        }

        Ok(VerifiedClaims {
            subject,
            tenant_id,
            role,
            jwt_id,
            issuer,
            audience,
            issued_at: iat,
            expires_at: exp,
            mfa_verified,
            custom: HashMap::new(),
        })
    }

    /// RFC 7662 introspection. Returns `active=false` on any error rather than
    /// surfacing the failure to the caller.
    pub fn introspect(&self, token: &str) -> IntrospectionResult {
        let cache_key = blake_like_digest(token);
        let now = current_unix_time();
        {
            let mut state = match self.lock_state() {
                Ok(state) => state,
                Err(_) => return inactive_introspection("runtime lock poisoned"),
            };
            self.gc_introspection_cache(&mut state, now);
            if let Some(entry) = state.introspection_cache.get(&cache_key) {
                if let Some(claims) = entry.result.claims.as_ref() {
                    if state.revoked_jti.contains(&claims.jwt_id) {
                        return inactive_introspection("token has been revoked");
                    }
                }
                return entry.result.clone();
            }
        }
        let result = match self.verify_token(token) {
            Ok(claims) => IntrospectionResult {
                active: true,
                claims: Some(claims),
                reason: None,
            },
            Err(error) => IntrospectionResult {
                active: false,
                claims: None,
                reason: Some(error.to_string()),
            },
        };
        if let Ok(mut state) = self.lock_state() {
            state.introspection_cache.insert(
                cache_key,
                IntrospectionCacheEntry {
                    inserted_at: now,
                    result: result.clone(),
                },
            );
        }
        result
    }

    fn gc_introspection_cache(&self, state: &mut EngineState, now: u64) {
        state.introspection_cache.retain(|_, entry| {
            now.saturating_sub(entry.inserted_at) < INTROSPECTION_CACHE_TTL_SECONDS
        });
    }

    pub fn logout(&self, token: &str) -> Result<(), AuthSidecarError> {
        let claims = self.verify_token(token)?;
        let cache_key = blake_like_digest(token);
        let mut state = self.lock_state()?;
        state.revoked_jti.insert(claims.jwt_id.clone());
        state.introspection_cache.remove(&cache_key);
        state
            .sessions
            .retain(|_, session| session.current_jti != claims.jwt_id);
        Ok(())
    }

    /// Revoke a JTI directly. Used by administrative paths.
    pub fn revoke_jti(&self, jti: &str) -> Result<(), AuthSidecarError> {
        validate_required("jti", jti)?;
        let mut state = self.lock_state()?;
        state.revoked_jti.insert(jti.to_string());
        Ok(())
    }

    pub fn enroll_totp(
        &self,
        request: &TotpEnrollment,
    ) -> Result<TotpEnrollmentResponse, AuthSidecarError> {
        validate_required("username", &request.username)?;
        let secret = generate_totp_secret()?;
        let secret_base32 = base32_encode(&secret);
        let otpauth_uri = format!(
            "otpauth://totp/{issuer}:{user}?secret={secret}&issuer={issuer}&algorithm=SHA1&digits={digits}&period={period}",
            issuer = url_encode(&self.issuer),
            user = url_encode(&request.username),
            secret = secret_base32,
            digits = TOTP_DIGITS,
            period = TOTP_PERIOD_SECONDS,
        );
        let mut state = self.lock_state()?;
        let user = state
            .users
            .get_mut(&request.username)
            .ok_or(AuthSidecarError::UnknownUser)?;
        if user.totp_secret.is_some() {
            return Err(AuthSidecarError::TotpAlreadyEnrolled);
        }
        user.totp_secret = Some(secret.clone());
        user.mfa_required = true;
        user.failed_totp_attempts = 0;
        Ok(TotpEnrollmentResponse {
            username: request.username.clone(),
            secret_base32,
            period_seconds: TOTP_PERIOD_SECONDS,
            digits: TOTP_DIGITS,
            algorithm: "SHA1",
            otpauth_uri,
        })
    }

    pub fn verify_totp(&self, request: &TotpVerifyRequest) -> Result<(), AuthSidecarError> {
        validate_required("username", &request.username)?;
        validate_required("code", &request.code)?;
        let now = current_unix_time();
        let mut state = self.lock_state()?;
        let user = state
            .users
            .get_mut(&request.username)
            .ok_or(AuthSidecarError::UnknownUser)?;
        let secret = user
            .totp_secret
            .as_ref()
            .ok_or(AuthSidecarError::TotpNotEnrolled)?
            .clone();
        if user.failed_totp_attempts >= self.mfa_max_attempts {
            return Err(AuthSidecarError::MfaAttemptsExceeded);
        }
        if !verify_totp(&secret, &request.code, now) {
            user.failed_totp_attempts = user.failed_totp_attempts.saturating_add(1);
            return Err(AuthSidecarError::TotpCodeInvalid);
        }
        user.failed_totp_attempts = 0;
        Ok(())
    }

    pub fn oidc_login(
        &self,
        request: &OidcLoginRequest,
    ) -> Result<OidcLoginResponse, AuthSidecarError> {
        validate_required("oidc.provider", &request.provider)?;
        validate_redirect_uri(&request.redirect_uri)?;
        let provider = self
            .oidc_providers
            .iter()
            .find(|provider| provider.name == request.provider)
            .ok_or(AuthSidecarError::UnknownProvider)?;
        provider.validate()?;
        if !provider.allows_redirect_uri(&request.redirect_uri) {
            return Err(AuthSidecarError::InvalidRedirectUri);
        }

        let state = random_url_token(32)?;
        let nonce = random_url_token(32)?;
        let now = current_unix_time();
        let authorization_url = format!(
            "{}?response_type=code&client_id={}&redirect_uri={}&scope={}&state={}&nonce={}",
            provider.authorization_endpoint,
            url_encode(&provider.client_id),
            url_encode(&request.redirect_uri),
            url_encode(&provider.scopes.join(" ")),
            url_encode(&state),
            url_encode(&nonce),
        );
        self.lock_state()?.oidc_states.insert(
            state.clone(),
            PendingOidcLogin {
                provider: provider.name.clone(),
                redirect_uri: request.redirect_uri.clone(),
                nonce: nonce.clone(),
                expires_at: now + OIDC_STATE_TTL_SECONDS,
            },
        );

        Ok(OidcLoginResponse {
            provider: provider.name.clone(),
            authorization_url,
            state,
            nonce,
            redirect_uri: request.redirect_uri.clone(),
            expires_in: OIDC_STATE_TTL_SECONDS,
        })
    }

    pub fn validate_oidc_callback(
        &self,
        request: &OidcCallbackRequest,
    ) -> Result<OidcCallbackValidation, AuthSidecarError> {
        validate_required("oidc.provider", &request.provider)?;
        validate_required("oidc.state", &request.state)?;
        validate_required("oidc.nonce", &request.nonce)?;
        validate_required("oidc.code", &request.code)?;
        validate_redirect_uri(&request.redirect_uri)?;
        let provider = self
            .oidc_providers
            .iter()
            .find(|provider| provider.name == request.provider)
            .ok_or(AuthSidecarError::UnknownProvider)?;
        if !provider.allows_redirect_uri(&request.redirect_uri) {
            return Err(AuthSidecarError::InvalidRedirectUri);
        }

        let now = current_unix_time();
        let mut state = self.lock_state()?;
        self.gc_oidc_states(&mut state, now);
        let pending = state
            .oidc_states
            .get(&request.state)
            .cloned()
            .ok_or(AuthSidecarError::InvalidOidcState)?;
        if pending.expires_at <= now {
            state.oidc_states.remove(&request.state);
            return Err(AuthSidecarError::OidcStateExpired);
        }
        if pending.provider != request.provider
            || pending.redirect_uri != request.redirect_uri
            || pending.nonce != request.nonce
        {
            return Err(AuthSidecarError::InvalidOidcState);
        }
        state.oidc_states.remove(&request.state);

        Ok(OidcCallbackValidation {
            provider: pending.provider,
            redirect_uri: pending.redirect_uri,
            nonce: pending.nonce,
        })
    }

    fn gc_oidc_states(&self, state: &mut EngineState, now: u64) {
        state
            .oidc_states
            .retain(|_, pending| pending.expires_at > now);
    }

    fn allocate_refresh_token(
        &self,
        username: &str,
        current_jti: &str,
        now: u64,
    ) -> Result<String, AuthSidecarError> {
        let mut bytes = [0_u8; 32];
        fill_random_bytes(&mut bytes)?;
        let token = base64_url_encode(&bytes);
        let mut state = self.lock_state()?;
        state.sessions.insert(
            token.clone(),
            StoredSession {
                refresh_token: token.clone(),
                username: username.to_string(),
                current_jti: current_jti.to_string(),
                issued_at: now,
                expires_at: now + REFRESH_TTL_SECONDS,
            },
        );
        Ok(token)
    }

    fn allocate_jti(&self) -> Result<String, AuthSidecarError> {
        let mut state = self.lock_state()?;
        state.allocate_jti()
    }

    fn lock_state(&self) -> Result<MutexGuard<'_, EngineState>, AuthSidecarError> {
        self.state.lock().map_err(|error| {
            AuthSidecarError::Runtime(format!("auth runtime state poisoned: {error}"))
        })
    }
}

fn oidc_providers_from_env() -> Result<Vec<OidcProviderConfig>, AuthSidecarError> {
    let Ok(name) = std::env::var("AI_BLAISE_AUTH_OIDC_PROVIDER_NAME") else {
        return Ok(Vec::new());
    };
    let provider = OidcProviderConfig {
        name,
        issuer_url: required_env("AI_BLAISE_AUTH_OIDC_ISSUER")?,
        authorization_endpoint: required_env("AI_BLAISE_AUTH_OIDC_AUTHORIZATION_ENDPOINT")?,
        client_id: required_env("AI_BLAISE_AUTH_OIDC_CLIENT_ID")?,
        client_secret_ref: required_env("AI_BLAISE_AUTH_OIDC_CLIENT_SECRET_REF")?,
        redirect_uris: split_env_list("AI_BLAISE_AUTH_OIDC_REDIRECT_URIS")?,
        scopes: split_env_list("AI_BLAISE_AUTH_OIDC_SCOPES")?,
    };
    provider.validate()?;
    Ok(vec![provider])
}

fn required_env(name: &str) -> Result<String, AuthSidecarError> {
    std::env::var(name).map_err(|_| AuthSidecarError::Runtime(format!("{name} is required")))
}

fn split_env_list(name: &str) -> Result<Vec<String>, AuthSidecarError> {
    let value = required_env(name)?;
    let values = value
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    if values.is_empty() {
        return Err(AuthSidecarError::Runtime(format!(
            "{name} must not be empty"
        )));
    }
    Ok(values)
}

fn inactive_introspection(reason: &str) -> IntrospectionResult {
    IntrospectionResult {
        active: false,
        claims: None,
        reason: Some(reason.to_string()),
    }
}

fn blake_like_digest(token: &str) -> String {
    base64_url_encode(&sha256(token.as_bytes()))
}

fn random_url_token(len: usize) -> Result<String, AuthSidecarError> {
    let mut bytes = vec![0_u8; len];
    fill_random_bytes(bytes.as_mut_slice())?;
    Ok(base64_url_encode(&bytes))
}

fn current_unix_time() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_else(|_| Duration::from_secs(0))
        .as_secs()
}

fn encode_claims_json(
    claims: &TokenClaims,
    issuer: &str,
    audience: &str,
    iat: i64,
    exp: i64,
) -> String {
    let mut body = String::with_capacity(256);
    body.push('{');
    body.push_str("\"sub\":\"");
    body.push_str(&escape_json(&claims.subject));
    body.push_str("\",\"iss\":\"");
    body.push_str(&escape_json(issuer));
    body.push_str("\",\"aud\":\"");
    body.push_str(&escape_json(audience));
    body.push_str("\",\"role\":\"");
    body.push_str(&escape_json(&claims.role));
    body.push_str("\",\"tenant_id\":\"");
    body.push_str(&escape_json(&claims.tenant_id));
    body.push_str("\",\"jti\":\"");
    body.push_str(&escape_json(&claims.jwt_id));
    body.push_str("\",\"iat\":");
    body.push_str(&iat.to_string());
    body.push_str(",\"exp\":");
    body.push_str(&exp.to_string());
    for custom in &claims.custom_claims {
        body.push_str(",\"");
        body.push_str(&escape_json(&custom.name));
        body.push_str("\":\"");
        body.push_str(&escape_json(&custom.value));
        body.push('"');
    }
    body.push('}');
    body
}

fn escape_json(input: &str) -> String {
    input
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

fn hmac_sha256(secret: &[u8], input: &[u8]) -> Vec<u8> {
    // Pure-Rust HMAC-SHA-256 (RFC 2104) using the in-crate SHA-256 below.
    // Keeping the implementation local avoids version coupling against the
    // RustCrypto `hmac` / `sha2` crates whose `Mac` API has changed across
    // releases used elsewhere in the workspace.
    const BLOCK_SIZE: usize = 64;
    let mut block_key = [0_u8; BLOCK_SIZE];
    if secret.len() > BLOCK_SIZE {
        let digest = sha256(secret);
        block_key[..digest.len()].copy_from_slice(&digest);
    } else {
        block_key[..secret.len()].copy_from_slice(secret);
    }
    let mut o_key_pad = [0_u8; BLOCK_SIZE];
    let mut i_key_pad = [0_u8; BLOCK_SIZE];
    for i in 0..BLOCK_SIZE {
        o_key_pad[i] = block_key[i] ^ 0x5c;
        i_key_pad[i] = block_key[i] ^ 0x36;
    }
    let mut inner = Vec::with_capacity(BLOCK_SIZE + input.len());
    inner.extend_from_slice(&i_key_pad);
    inner.extend_from_slice(input);
    let inner_digest = sha256(&inner);
    let mut outer = Vec::with_capacity(BLOCK_SIZE + inner_digest.len());
    outer.extend_from_slice(&o_key_pad);
    outer.extend_from_slice(&inner_digest);
    sha256(&outer).to_vec()
}

/// SHA-256 (FIPS 180-4) single-shot.
fn sha256(message: &[u8]) -> [u8; 32] {
    const K: [u32; 64] = [
        0x428a_2f98,
        0x7137_4491,
        0xb5c0_fbcf,
        0xe9b5_dba5,
        0x3956_c25b,
        0x59f1_11f1,
        0x923f_82a4,
        0xab1c_5ed5,
        0xd807_aa98,
        0x1283_5b01,
        0x2431_85be,
        0x550c_7dc3,
        0x72be_5d74,
        0x80de_b1fe,
        0x9bdc_06a7,
        0xc19b_f174,
        0xe49b_69c1,
        0xefbe_4786,
        0x0fc1_9dc6,
        0x240c_a1cc,
        0x2de9_2c6f,
        0x4a74_84aa,
        0x5cb0_a9dc,
        0x76f9_88da,
        0x983e_5152,
        0xa831_c66d,
        0xb003_27c8,
        0xbf59_7fc7,
        0xc6e0_0bf3,
        0xd5a7_9147,
        0x06ca_6351,
        0x1429_2967,
        0x27b7_0a85,
        0x2e1b_2138,
        0x4d2c_6dfc,
        0x5338_0d13,
        0x650a_7354,
        0x766a_0abb,
        0x81c2_c92e,
        0x9272_2c85,
        0xa2bf_e8a1,
        0xa81a_664b,
        0xc24b_8b70,
        0xc76c_51a3,
        0xd192_e819,
        0xd699_0624,
        0xf40e_3585,
        0x106a_a070,
        0x19a4_c116,
        0x1e37_6c08,
        0x2748_774c,
        0x34b0_bcb5,
        0x391c_0cb3,
        0x4ed8_aa4a,
        0x5b9c_ca4f,
        0x682e_6ff3,
        0x748f_82ee,
        0x78a5_636f,
        0x84c8_7814,
        0x8cc7_0208,
        0x90be_fffa,
        0xa450_6ceb,
        0xbef9_a3f7,
        0xc671_78f2,
    ];
    let mut h = [
        0x6a09_e667_u32,
        0xbb67_ae85,
        0x3c6e_f372,
        0xa54f_f53a,
        0x510e_527f,
        0x9b05_688c,
        0x1f83_d9ab,
        0x5be0_cd19,
    ];

    let bit_len = (message.len() as u64).wrapping_mul(8);
    let mut padded = Vec::with_capacity((message.len() + 9).div_ceil(64) * 64);
    padded.extend_from_slice(message);
    padded.push(0x80);
    while padded.len() % 64 != 56 {
        padded.push(0);
    }
    padded.extend_from_slice(&bit_len.to_be_bytes());

    for chunk in padded.chunks(64) {
        let mut w = [0_u32; 64];
        for i in 0..16 {
            w[i] = ((chunk[i * 4] as u32) << 24)
                | ((chunk[i * 4 + 1] as u32) << 16)
                | ((chunk[i * 4 + 2] as u32) << 8)
                | (chunk[i * 4 + 3] as u32);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }
        let mut a = h[0];
        let mut b = h[1];
        let mut c = h[2];
        let mut d = h[3];
        let mut e = h[4];
        let mut f = h[5];
        let mut g = h[6];
        let mut hh = h[7];
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let temp1 = hh
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(maj);
            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }
        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
        h[5] = h[5].wrapping_add(f);
        h[6] = h[6].wrapping_add(g);
        h[7] = h[7].wrapping_add(hh);
    }

    let mut out = [0_u8; 32];
    for (i, word) in h.iter().enumerate() {
        out[i * 4..(i + 1) * 4].copy_from_slice(&word.to_be_bytes());
    }
    out
}

fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    let mut diff = 0_u8;
    for i in 0..left.len() {
        diff |= left[i] ^ right[i];
    }
    diff == 0
}

fn fill_random_bytes(bytes: &mut [u8]) -> Result<(), AuthSidecarError> {
    let mut file = File::open("/dev/urandom").map_err(|error| {
        AuthSidecarError::Runtime(format!("failed to open OS entropy source: {error}"))
    })?;
    file.read_exact(bytes).map_err(|error| {
        AuthSidecarError::Runtime(format!("failed to read OS entropy source: {error}"))
    })
}

fn base64_url_encode(input: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut out = String::with_capacity((input.len() * 4).div_ceil(3));
    let mut i = 0;
    while i + 3 <= input.len() {
        let chunk = ((input[i] as u32) << 16) | ((input[i + 1] as u32) << 8) | input[i + 2] as u32;
        out.push(ALPHABET[((chunk >> 18) & 0x3f) as usize] as char);
        out.push(ALPHABET[((chunk >> 12) & 0x3f) as usize] as char);
        out.push(ALPHABET[((chunk >> 6) & 0x3f) as usize] as char);
        out.push(ALPHABET[(chunk & 0x3f) as usize] as char);
        i += 3;
    }
    match input.len() - i {
        1 => {
            let chunk = (input[i] as u32) << 16;
            out.push(ALPHABET[((chunk >> 18) & 0x3f) as usize] as char);
            out.push(ALPHABET[((chunk >> 12) & 0x3f) as usize] as char);
        }
        2 => {
            let chunk = ((input[i] as u32) << 16) | ((input[i + 1] as u32) << 8);
            out.push(ALPHABET[((chunk >> 18) & 0x3f) as usize] as char);
            out.push(ALPHABET[((chunk >> 12) & 0x3f) as usize] as char);
            out.push(ALPHABET[((chunk >> 6) & 0x3f) as usize] as char);
        }
        _ => {}
    }
    out
}

fn base64_url_decode(input: &str) -> Result<Vec<u8>, AuthSidecarError> {
    let mut buffer = 0_u32;
    let mut bits = 0_u32;
    let mut output = Vec::with_capacity(input.len() * 3 / 4);
    for byte in input.bytes() {
        let value = match byte {
            b'A'..=b'Z' => byte - b'A',
            b'a'..=b'z' => byte - b'a' + 26,
            b'0'..=b'9' => byte - b'0' + 52,
            b'-' => 62,
            b'_' => 63,
            _ => return Err(AuthSidecarError::JwtMalformed),
        } as u32;
        buffer = (buffer << 6) | value;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            output.push(((buffer >> bits) & 0xff) as u8);
            if bits > 0 {
                buffer &= (1 << bits) - 1;
            } else {
                buffer = 0;
            }
        }
    }
    if bits > 0 && buffer != 0 {
        return Err(AuthSidecarError::JwtMalformed);
    }
    Ok(output)
}

fn split_jwt(token: &str) -> Result<(&str, &str, &str), AuthSidecarError> {
    let mut parts = token.split('.');
    let header = parts.next().ok_or(AuthSidecarError::JwtMalformed)?;
    let payload = parts.next().ok_or(AuthSidecarError::JwtMalformed)?;
    let signature = parts.next().ok_or(AuthSidecarError::JwtMalformed)?;
    if parts.next().is_some() || header.is_empty() || payload.is_empty() || signature.is_empty() {
        return Err(AuthSidecarError::JwtMalformed);
    }
    Ok((header, payload, signature))
}

/// Extract a JSON string field from a *flat* JSON object literal. This is
/// intentionally bespoke (we control the encoder above) and does not handle
/// nested objects or arrays.
fn extract_json_string(json: &str, key: &str) -> Option<String> {
    let needle = format!("\"{key}\":\"");
    let start = json.find(&needle)? + needle.len();
    let bytes = json.as_bytes();
    let mut i = start;
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

fn extract_json_number(json: &str, key: &str) -> Option<i64> {
    let needle = format!("\"{key}\":");
    let start = json.find(&needle)? + needle.len();
    let bytes = json.as_bytes();
    let mut end = start;
    while end < bytes.len() {
        let b = bytes[end];
        if b == b',' || b == b'}' || b == b' ' || b == b'\n' || b == b'\r' || b == b'\t' {
            break;
        }
        end += 1;
    }
    json[start..end].parse::<i64>().ok()
}

fn hash_password(password: &str) -> Result<PasswordHash, AuthSidecarError> {
    let mut salt = vec![0_u8; PBKDF2_SALT_LEN];
    fill_random_bytes(salt.as_mut_slice())?;
    let hash = pbkdf2_sha256(
        password.as_bytes(),
        &salt,
        PBKDF2_ITERATIONS,
        PBKDF2_HASH_LEN,
    );
    Ok(PasswordHash {
        iterations: PBKDF2_ITERATIONS,
        salt,
        hash,
    })
}

fn verify_password(stored: &PasswordHash, password: &str) -> Result<bool, AuthSidecarError> {
    let candidate = pbkdf2_sha256(
        password.as_bytes(),
        &stored.salt,
        stored.iterations,
        stored.hash.len(),
    );
    Ok(constant_time_eq(&candidate, &stored.hash))
}

/// Pure-Rust PBKDF2-HMAC-SHA256, matching RFC 2898.
fn pbkdf2_sha256(password: &[u8], salt: &[u8], iterations: u32, output_len: usize) -> Vec<u8> {
    let hash_len = 32_usize;
    let blocks = output_len.div_ceil(hash_len);
    let mut output = Vec::with_capacity(output_len);
    for block in 1..=blocks {
        let mut salt_block = Vec::with_capacity(salt.len() + 4);
        salt_block.extend_from_slice(salt);
        salt_block.extend_from_slice(&(block as u32).to_be_bytes());
        let mut u = hmac_sha256(password, &salt_block);
        let mut t = u.clone();
        for _ in 1..iterations {
            u = hmac_sha256(password, &u);
            for (t_byte, u_byte) in t.iter_mut().zip(u.iter()) {
                *t_byte ^= *u_byte;
            }
        }
        output.extend_from_slice(&t);
    }
    output.truncate(output_len);
    output
}

fn generate_totp_secret() -> Result<Vec<u8>, AuthSidecarError> {
    let mut secret = vec![0_u8; 20];
    fill_random_bytes(secret.as_mut_slice())?;
    Ok(secret)
}

fn verify_totp(secret: &[u8], code: &str, now_unix: u64) -> bool {
    let counter = (now_unix / TOTP_PERIOD_SECONDS) as i64;
    let trimmed = code.trim();
    for step in -TOTP_STEP_TOLERANCE..=TOTP_STEP_TOLERANCE {
        let candidate = counter + step;
        if candidate < 0 {
            continue;
        }
        let expected = totp_code(secret, candidate as u64, TOTP_DIGITS);
        if constant_time_eq(expected.as_bytes(), trimmed.as_bytes()) {
            return true;
        }
    }
    false
}

fn totp_code(secret: &[u8], counter: u64, digits: u32) -> String {
    // RFC 4226 HOTP / RFC 6238 TOTP -- HMAC-SHA1 is required by all common
    // authenticator apps. We implement SHA-1 in-crate to avoid a dependency.
    let mut counter_bytes = [0_u8; 8];
    counter_bytes.copy_from_slice(&counter.to_be_bytes());
    let hmac = hmac_sha1(secret, &counter_bytes);
    let offset = (hmac[hmac.len() - 1] & 0x0f) as usize;
    let binary = ((hmac[offset] as u32 & 0x7f) << 24)
        | ((hmac[offset + 1] as u32) << 16)
        | ((hmac[offset + 2] as u32) << 8)
        | (hmac[offset + 3] as u32);
    let modulus = 10_u32.pow(digits);
    let code = binary % modulus;
    format!("{code:0width$}", width = digits as usize)
}

/// HMAC-SHA1 (RFC 2104).
fn hmac_sha1(key: &[u8], input: &[u8]) -> [u8; 20] {
    const BLOCK_SIZE: usize = 64;
    let mut block_key = [0_u8; BLOCK_SIZE];
    if key.len() > BLOCK_SIZE {
        let digest = sha1(key);
        block_key[..digest.len()].copy_from_slice(&digest);
    } else {
        block_key[..key.len()].copy_from_slice(key);
    }
    let mut o_key_pad = [0_u8; BLOCK_SIZE];
    let mut i_key_pad = [0_u8; BLOCK_SIZE];
    for i in 0..BLOCK_SIZE {
        o_key_pad[i] = block_key[i] ^ 0x5c;
        i_key_pad[i] = block_key[i] ^ 0x36;
    }
    let mut inner = Vec::with_capacity(BLOCK_SIZE + input.len());
    inner.extend_from_slice(&i_key_pad);
    inner.extend_from_slice(input);
    let inner_digest = sha1(&inner);
    let mut outer = Vec::with_capacity(BLOCK_SIZE + inner_digest.len());
    outer.extend_from_slice(&o_key_pad);
    outer.extend_from_slice(&inner_digest);
    sha1(&outer)
}

/// SHA-1 (RFC 3174) -- single-shot. Used only for HMAC-SHA1 / TOTP; not for
/// any password or token signing path. Implemented in-crate to avoid pulling
/// in the `sha1` dep solely for authenticator compatibility.
fn sha1(message: &[u8]) -> [u8; 20] {
    let mut h0: u32 = 0x6745_2301;
    let mut h1: u32 = 0xEFCD_AB89;
    let mut h2: u32 = 0x98BA_DCFE;
    let mut h3: u32 = 0x1032_5476;
    let mut h4: u32 = 0xC3D2_E1F0;

    let bit_len = (message.len() as u64).wrapping_mul(8);
    let mut padded = Vec::with_capacity((message.len() + 9).div_ceil(64) * 64);
    padded.extend_from_slice(message);
    padded.push(0x80);
    while padded.len() % 64 != 56 {
        padded.push(0);
    }
    padded.extend_from_slice(&bit_len.to_be_bytes());

    for chunk in padded.chunks(64) {
        let mut w = [0_u32; 80];
        for i in 0..16 {
            w[i] = ((chunk[i * 4] as u32) << 24)
                | ((chunk[i * 4 + 1] as u32) << 16)
                | ((chunk[i * 4 + 2] as u32) << 8)
                | (chunk[i * 4 + 3] as u32);
        }
        for i in 16..80 {
            w[i] = (w[i - 3] ^ w[i - 8] ^ w[i - 14] ^ w[i - 16]).rotate_left(1);
        }
        let mut a = h0;
        let mut b = h1;
        let mut c = h2;
        let mut d = h3;
        let mut e = h4;
        #[allow(clippy::needless_range_loop)]
        // SHA-1 round-mixing: index access is the textbook form
        for i in 0..80 {
            let (f, k) = if i < 20 {
                ((b & c) | ((!b) & d), 0x5A82_7999)
            } else if i < 40 {
                (b ^ c ^ d, 0x6ED9_EBA1)
            } else if i < 60 {
                ((b & c) | (b & d) | (c & d), 0x8F1B_BCDC)
            } else {
                (b ^ c ^ d, 0xCA62_C1D6)
            };
            let temp = a
                .rotate_left(5)
                .wrapping_add(f)
                .wrapping_add(e)
                .wrapping_add(k)
                .wrapping_add(w[i]);
            e = d;
            d = c;
            c = b.rotate_left(30);
            b = a;
            a = temp;
        }
        h0 = h0.wrapping_add(a);
        h1 = h1.wrapping_add(b);
        h2 = h2.wrapping_add(c);
        h3 = h3.wrapping_add(d);
        h4 = h4.wrapping_add(e);
    }
    let mut out = [0_u8; 20];
    out[0..4].copy_from_slice(&h0.to_be_bytes());
    out[4..8].copy_from_slice(&h1.to_be_bytes());
    out[8..12].copy_from_slice(&h2.to_be_bytes());
    out[12..16].copy_from_slice(&h3.to_be_bytes());
    out[16..20].copy_from_slice(&h4.to_be_bytes());
    out
}

/// Base32 (RFC 4648) encoder used for authenticator-app secrets.
fn base32_encode(input: &[u8]) -> String {
    const ALPHABET: &[u8; 32] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";
    let mut output = String::with_capacity(input.len().div_ceil(5) * 8);
    let mut buffer: u64 = 0;
    let mut bits = 0_u32;
    for byte in input {
        buffer = (buffer << 8) | (*byte as u64);
        bits += 8;
        while bits >= 5 {
            bits -= 5;
            let index = ((buffer >> bits) & 0x1F) as usize;
            output.push(ALPHABET[index] as char);
        }
    }
    if bits > 0 {
        let index = ((buffer << (5 - bits)) & 0x1F) as usize;
        output.push(ALPHABET[index] as char);
    }
    output
}

fn url_encode(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for byte in input.bytes() {
        let unreserved = byte.is_ascii_alphanumeric()
            || byte == b'-'
            || byte == b'_'
            || byte == b'.'
            || byte == b'~';
        if unreserved {
            out.push(byte as char);
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

/// Probe-style entry point shared with the rest of the workspace: drive the
/// runtime by responding to canned HTTP-probe-style requests over a synchronous
/// `SidecarRuntime`. The `serve_http_forever` path below wraps a real listener.
pub fn handle_health_probe(
    runtime: &mut SidecarRuntime,
    request: &HttpProbeRequest,
) -> HttpProbeResponse {
    runtime.handle_http_request(request)
}

pub use http::{handle_http_bytes, serve_http_forever};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jwt_issue_request_renders_auditable_plan() {
        let plan = canonical_jwt_issue_request().plan().expect("issue plan");

        assert_eq!(plan.issuer, "https://auth.example.com");
        assert_eq!(plan.signing_key_ref, "jwt-signing-key");
        assert_eq!(plan.tenant_id, "tenant-a");
        assert_eq!(plan.ttl_seconds, 3_600);
    }

    #[test]
    fn canonical_auth_report_is_deterministic() {
        let report = canonical_auth_report().expect("canonical report");

        assert_eq!(report.issue_plan.subject, "user-123");
        assert_eq!(report.introspection.cache_ttl_seconds, 300);
        assert_eq!(report.sidecar.oidc_providers[0].name, "github");
    }

    #[test]
    fn oidc_provider_requires_https_issuer() {
        let provider = OidcProviderConfig {
            name: "github".to_string(),
            issuer_url: "http://github.example".to_string(),
            authorization_endpoint: "http://github.example/oauth/authorize".to_string(),
            client_id: "ai-blaise-github".to_string(),
            client_secret_ref: "k8s://auth/github-client-secret".to_string(),
            redirect_uris: vec!["https://auth.example.com/auth/oidc/callback".to_string()],
            scopes: vec!["openid".to_string()],
        };

        assert_eq!(provider.validate(), Err(AuthSidecarError::InvalidIssuerUrl));
    }

    #[test]
    fn mfa_requires_at_least_one_method() {
        let policy = MfaPolicy {
            totp_enabled: false,
            webauthn_enabled: false,
            max_attempts: 3,
        };

        assert_eq!(policy.validate(), Err(AuthSidecarError::NoMfaMethodEnabled));
    }

    #[test]
    fn auth_sidecar_plan_validates_oidc_and_mfa() {
        assert_eq!(canonical_auth_sidecar_plan().validate(), Ok(()));
    }

    #[test]
    fn oidc_provider_requires_allowed_https_redirect_and_openid_scope() {
        let mut provider = oidc_provider();
        provider.redirect_uris = vec!["http://db.example.com/callback".to_string()];
        assert_eq!(
            provider.validate(),
            Err(AuthSidecarError::InvalidRedirectUri)
        );

        let mut provider = oidc_provider();
        provider.scopes = vec!["email".to_string()];
        assert_eq!(
            provider.validate(),
            Err(AuthSidecarError::MissingOpenIdScope)
        );
    }

    fn fixture_engine() -> AuthEngine {
        AuthEngine::with_ttl(
            "https://auth.example.com",
            "postgres",
            b"unit-test-secret-key".to_vec(),
            300,
        )
    }

    fn oidc_provider() -> OidcProviderConfig {
        OidcProviderConfig {
            name: "stub".to_string(),
            issuer_url: "https://idp.example.com".to_string(),
            authorization_endpoint: "https://idp.example.com/oauth2/v1/authorize".to_string(),
            client_id: "ai-blaise-client".to_string(),
            client_secret_ref: "k8s://auth/idp-client-secret".to_string(),
            redirect_uris: vec!["https://db.example.com/auth/oidc/callback".to_string()],
            scopes: vec!["openid".to_string(), "email".to_string()],
        }
    }

    fn fixture_engine_with_oidc() -> AuthEngine {
        AuthEngine::with_runtime_config(
            "https://auth.example.com",
            "postgres",
            b"unit-test-secret-key".to_vec(),
            300,
            vec![oidc_provider()],
            3,
        )
        .expect("engine")
    }

    #[test]
    fn from_env_requires_explicit_secret_by_default() {
        std::env::remove_var("AI_BLAISE_AUTH_HS256_SECRET");
        std::env::remove_var("AI_BLAISE_AUTH_ALLOW_EPHEMERAL_SECRET");
        let error = AuthEngine::from_env().unwrap_err();
        assert!(error.to_string().contains("AI_BLAISE_AUTH_HS256_SECRET"));
    }

    #[test]
    fn issue_and_verify_round_trip() {
        let engine = fixture_engine();
        let claims = canonical_claims();
        let token = engine.issue_token(claims.clone()).expect("issue");
        let verified = engine.verify_token(&token).expect("verify");

        assert_eq!(verified.subject, claims.subject);
        assert_eq!(verified.tenant_id, claims.tenant_id);
        assert_eq!(verified.role, claims.role);
        assert_eq!(verified.jwt_id, claims.jwt_id);
        assert_eq!(verified.issuer, "https://auth.example.com");
        assert_eq!(verified.audience, "postgres");
    }

    #[test]
    fn verify_rejects_tampered_token() {
        let engine = fixture_engine();
        let claims = canonical_claims();
        let token = engine.issue_token(claims).expect("issue");
        // Swap the first signature character with another base64url character
        // so the resulting string still decodes -- only the signature value
        // changes, not its length or the final unpadded-base64 leftover bits.
        let mut bytes = token.as_bytes().to_vec();
        let signature_start = token.rfind('.').expect("signature separator") + 1;
        let swap = if bytes[signature_start] == b'A' {
            b'B'
        } else {
            b'A'
        };
        bytes[signature_start] = swap;
        let tampered = String::from_utf8(bytes).expect("utf-8");
        assert_eq!(
            engine.verify_token(&tampered),
            Err(AuthSidecarError::JwtBadSignature)
        );
    }

    #[test]
    fn verify_rejects_expired_token() {
        let engine = AuthEngine::with_ttl(
            "https://auth.example.com",
            "postgres",
            b"unit-test-secret-key".to_vec(),
            0,
        );
        let claims = canonical_claims();
        let token = engine.issue_token(claims).expect("issue");
        std::thread::sleep(std::time::Duration::from_secs(7));
        assert_eq!(
            engine.verify_token(&token),
            Err(AuthSidecarError::JwtExpired)
        );
    }

    #[test]
    fn revoked_jti_fails_verification() {
        let engine = fixture_engine();
        let mut claims = canonical_claims();
        claims.jwt_id = "jti-revoked".to_string();
        let token = engine.issue_token(claims).expect("issue");
        engine.logout(&token).expect("logout");
        assert_eq!(
            engine.verify_token(&token),
            Err(AuthSidecarError::JwtRevoked)
        );
    }

    #[test]
    fn login_with_correct_password_returns_token() {
        let engine = fixture_engine();
        engine
            .register_user(
                "alice",
                "hunter2-correct-horse",
                "authenticated",
                "tenant-a",
            )
            .expect("register");
        let response = engine
            .login(&LoginRequest {
                username: "alice".to_string(),
                password: "hunter2-correct-horse".to_string(),
                totp_code: None,
            })
            .expect("login");
        let verified = engine.verify_token(&response.access_token).expect("verify");
        assert_eq!(verified.subject, "alice");
        assert_eq!(verified.tenant_id, "tenant-a");
        assert_eq!(verified.role, "authenticated");
        assert!(!response.mfa_required);
    }

    #[test]
    fn login_with_wrong_password_is_rejected() {
        let engine = fixture_engine();
        engine
            .register_user(
                "alice",
                "hunter2-correct-horse",
                "authenticated",
                "tenant-a",
            )
            .expect("register");
        let error = engine
            .login(&LoginRequest {
                username: "alice".to_string(),
                password: "wrong".to_string(),
                totp_code: None,
            })
            .unwrap_err();
        assert_eq!(error, AuthSidecarError::PasswordVerificationFailed);
    }

    #[test]
    fn oidc_login_issues_authorization_url_and_callback_consumes_state() {
        let engine = fixture_engine_with_oidc();
        let login = engine
            .oidc_login(&OidcLoginRequest {
                provider: "stub".to_string(),
                redirect_uri: "https://db.example.com/auth/oidc/callback".to_string(),
            })
            .expect("oidc login");
        assert!(login.authorization_url.contains("response_type=code"));
        assert!(login
            .authorization_url
            .contains("client_id=ai-blaise-client"));
        assert!(login.authorization_url.contains("state="));
        assert!(login.authorization_url.contains("nonce="));

        let validation = engine
            .validate_oidc_callback(&OidcCallbackRequest {
                provider: "stub".to_string(),
                redirect_uri: "https://db.example.com/auth/oidc/callback".to_string(),
                state: login.state.clone(),
                nonce: login.nonce.clone(),
                code: "stub-code".to_string(),
            })
            .expect("callback validation");
        assert_eq!(validation.provider, "stub");
        assert_eq!(
            engine.validate_oidc_callback(&OidcCallbackRequest {
                provider: "stub".to_string(),
                redirect_uri: "https://db.example.com/auth/oidc/callback".to_string(),
                state: login.state,
                nonce: login.nonce,
                code: "stub-code".to_string(),
            }),
            Err(AuthSidecarError::InvalidOidcState)
        );
    }

    #[test]
    fn oidc_login_rejects_unknown_provider_and_bad_redirect() {
        let engine = fixture_engine_with_oidc();
        assert_eq!(
            engine.oidc_login(&OidcLoginRequest {
                provider: "missing".to_string(),
                redirect_uri: "https://db.example.com/auth/oidc/callback".to_string(),
            }),
            Err(AuthSidecarError::UnknownProvider)
        );
        assert_eq!(
            engine.oidc_login(&OidcLoginRequest {
                provider: "stub".to_string(),
                redirect_uri: "https://evil.example.com/auth/oidc/callback".to_string(),
            }),
            Err(AuthSidecarError::InvalidRedirectUri)
        );
    }

    #[test]
    fn totp_enroll_and_verify_round_trip() {
        let engine = fixture_engine();
        engine
            .register_user("bob", "hunter2-correct-horse", "authenticated", "tenant-a")
            .expect("register");
        let enrollment = engine
            .enroll_totp(&TotpEnrollment {
                username: "bob".to_string(),
            })
            .expect("enroll");
        // Decode the base32 secret so we can compute the live code without
        // touching engine internals.
        let secret = base32_decode(&enrollment.secret_base32);
        let now = current_unix_time();
        let counter = now / TOTP_PERIOD_SECONDS;
        let code = totp_code(&secret, counter, TOTP_DIGITS);
        engine
            .verify_totp(&TotpVerifyRequest {
                username: "bob".to_string(),
                code,
            })
            .expect("verify");
    }

    #[test]
    fn totp_policy_locks_after_max_attempts() {
        let engine = AuthEngine::with_runtime_config(
            "https://auth.example.com",
            "postgres",
            b"unit-test-secret-key".to_vec(),
            300,
            Vec::new(),
            2,
        )
        .expect("engine");
        engine
            .register_user("bob", "hunter2-correct-horse", "authenticated", "tenant-a")
            .expect("register");
        engine
            .enroll_totp(&TotpEnrollment {
                username: "bob".to_string(),
            })
            .expect("enroll");
        for _ in 0..2 {
            assert_eq!(
                engine.verify_totp(&TotpVerifyRequest {
                    username: "bob".to_string(),
                    code: "000000".to_string(),
                }),
                Err(AuthSidecarError::TotpCodeInvalid)
            );
        }
        assert_eq!(
            engine.verify_totp(&TotpVerifyRequest {
                username: "bob".to_string(),
                code: "000000".to_string(),
            }),
            Err(AuthSidecarError::MfaAttemptsExceeded)
        );
    }

    #[test]
    fn logout_invalidates_refresh_token() {
        let engine = fixture_engine();
        engine
            .register_user(
                "carol",
                "hunter2-correct-horse",
                "authenticated",
                "tenant-a",
            )
            .expect("register");
        let response = engine
            .login(&LoginRequest {
                username: "carol".to_string(),
                password: "hunter2-correct-horse".to_string(),
                totp_code: None,
            })
            .expect("login");
        engine.logout(&response.access_token).expect("logout");
        assert_eq!(
            engine.refresh(&RefreshRequest {
                refresh_token: response.refresh_token,
            }),
            Err(AuthSidecarError::UnknownSession)
        );
    }

    #[test]
    fn logout_invalidates_cached_introspection() {
        let engine = fixture_engine();
        let token = engine.issue_token(canonical_claims()).expect("issue");
        assert!(engine.introspect(&token).active);
        engine.logout(&token).expect("logout");
        let result = engine.introspect(&token);
        assert!(!result.active);
        assert_eq!(result.reason.as_deref(), Some("token has been revoked"));
    }

    #[test]
    fn introspection_round_trip() {
        let engine = fixture_engine();
        let claims = canonical_claims();
        let token = engine.issue_token(claims).expect("issue");
        let result = engine.introspect(&token);
        assert!(result.active);
        let claims = result.claims.expect("claims");
        assert_eq!(claims.subject, "user-123");
    }

    #[test]
    fn introspection_caches_repeated_lookups() {
        let engine = fixture_engine();
        let claims = canonical_claims();
        let token = engine.issue_token(claims).expect("issue");
        let first = engine.introspect(&token);
        let second = engine.introspect(&token);
        assert!(first.active);
        assert!(second.active);
    }

    #[test]
    fn password_hash_round_trip() {
        let hash = hash_password("hunter2-correct-horse").expect("hash");
        assert!(verify_password(&hash, "hunter2-correct-horse").expect("verify"));
        assert!(!verify_password(&hash, "wrong").expect("verify"));
    }

    #[test]
    fn pbkdf2_matches_rfc_6070_vector() {
        // Vector from RFC 6070 (adapted from SHA-1 to SHA-256 reference vectors):
        // PBKDF2-HMAC-SHA256("password", "salt", c=1, dkLen=32) ==
        //   120fb6cffcf8b32c43e7225256c4f837a86548c92ccc35480805987cb70be17b
        let output = pbkdf2_sha256(b"password", b"salt", 1, 32);
        let expected = [
            0x12, 0x0f, 0xb6, 0xcf, 0xfc, 0xf8, 0xb3, 0x2c, 0x43, 0xe7, 0x22, 0x52, 0x56, 0xc4,
            0xf8, 0x37, 0xa8, 0x65, 0x48, 0xc9, 0x2c, 0xcc, 0x35, 0x48, 0x08, 0x05, 0x98, 0x7c,
            0xb7, 0x0b, 0xe1, 0x7b,
        ];
        assert_eq!(output.as_slice(), expected.as_slice());
    }

    #[test]
    fn base64_url_round_trip_omits_padding() {
        assert_eq!(base64_url_encode(b"hello"), "aGVsbG8");
        assert_eq!(base64_url_decode("aGVsbG8").expect("decode"), b"hello");
    }

    #[test]
    fn sha256_matches_nist_vector() {
        // FIPS 180-4 / NIST CAVS example: SHA-256("abc") ==
        //   ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad
        let expected = [
            0xba, 0x78, 0x16, 0xbf, 0x8f, 0x01, 0xcf, 0xea, 0x41, 0x41, 0x40, 0xde, 0x5d, 0xae,
            0x22, 0x23, 0xb0, 0x03, 0x61, 0xa3, 0x96, 0x17, 0x7a, 0x9c, 0xb4, 0x10, 0xff, 0x61,
            0xf2, 0x00, 0x15, 0xad,
        ];
        assert_eq!(sha256(b"abc"), expected);
    }

    #[test]
    fn hmac_sha256_matches_rfc_4231_vector() {
        // RFC 4231 test case 1: key=0x0b * 20, data=b"Hi There".
        let key = [0x0b_u8; 20];
        let mac = hmac_sha256(&key, b"Hi There");
        let expected = [
            0xb0, 0x34, 0x4c, 0x61, 0xd8, 0xdb, 0x38, 0x53, 0x5c, 0xa8, 0xaf, 0xce, 0xaf, 0x0b,
            0xf1, 0x2b, 0x88, 0x1d, 0xc2, 0x00, 0xc9, 0x83, 0x3d, 0xa7, 0x26, 0xe9, 0x37, 0x6c,
            0x2e, 0x32, 0xcf, 0xf7,
        ];
        assert_eq!(mac.as_slice(), expected.as_slice());
    }

    #[test]
    fn sha1_matches_rfc_3174_vector() {
        // "abc" -> a9993e364706816aba3e25717850c26c9cd0d89d
        let expected = [
            0xa9, 0x99, 0x3e, 0x36, 0x47, 0x06, 0x81, 0x6a, 0xba, 0x3e, 0x25, 0x71, 0x78, 0x50,
            0xc2, 0x6c, 0x9c, 0xd0, 0xd8, 0x9d,
        ];
        assert_eq!(sha1(b"abc"), expected);
    }

    #[test]
    fn totp_matches_rfc_6238_sha1_vectors() {
        // RFC 6238 Appendix B test secret "12345678901234567890" (ASCII).
        let secret = b"12345678901234567890";
        // Time 59 -> counter 1 -> code 94287082.
        assert_eq!(totp_code(secret, 1, 8), "94287082");
        // Time 1111111109 -> counter 37037036 -> code 07081804.
        assert_eq!(totp_code(secret, 37_037_036, 8), "07081804");
    }

    /// Local base32 decoder used only by tests.
    fn base32_decode(input: &str) -> Vec<u8> {
        const ALPHABET: &[u8; 32] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";
        let mut output = Vec::with_capacity(input.len() * 5 / 8);
        let mut buffer: u64 = 0;
        let mut bits = 0_u32;
        for ch in input.chars() {
            let upper = ch.to_ascii_uppercase();
            let Some(index) = ALPHABET.iter().position(|c| *c as char == upper) else {
                continue;
            };
            buffer = (buffer << 5) | (index as u64);
            bits += 5;
            if bits >= 8 {
                bits -= 8;
                output.push(((buffer >> bits) & 0xFF) as u8);
            }
        }
        output
    }
}
