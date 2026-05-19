//! Auth sidecar contracts.

// FEATURE: Auth1
// FEATURE: Auth2
// FEATURE: Auth4
// FEATURE: Auth5

use ai_blaise_citus_sidecar_shared::{AuthIssuerContract, SidecarContractError};
use std::error::Error;
use std::fmt;

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
    pub client_id_secret_ref: String,
    pub client_secret_ref: String,
    pub scopes: Vec<String>,
}

impl OidcProviderConfig {
    pub fn validate(&self) -> Result<(), AuthSidecarError> {
        validate_required("oidc.name", &self.name)?;
        validate_required("oidc.issuer_url", &self.issuer_url)?;
        if !self.issuer_url.starts_with("https://") {
            return Err(AuthSidecarError::InvalidIssuerUrl);
        }
        validate_required("oidc.client_id_secret_ref", &self.client_id_secret_ref)?;
        validate_required("oidc.client_secret_ref", &self.client_secret_ref)?;
        validate_required_list("oidc.scopes", &self.scopes)
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
    InvalidCacheTtl,
    InvalidIssuerUrl,
    InvalidMfaAttempts,
    MissingRequiredField(&'static str),
    NoMfaMethodEnabled,
    SharedContract(String),
}

impl fmt::Display for AuthSidecarError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidCacheTtl => {
                write!(formatter, "cache_ttl_seconds must be greater than zero")
            }
            Self::InvalidIssuerUrl => write!(formatter, "issuer_url must start with https://"),
            Self::InvalidMfaAttempts => write!(formatter, "max_attempts must be greater than zero"),
            Self::MissingRequiredField(field) => write!(formatter, "{field} must not be empty"),
            Self::NoMfaMethodEnabled => {
                write!(formatter, "at least one MFA method must be enabled")
            }
            Self::SharedContract(error) => write!(formatter, "{error}"),
        }
    }
}

impl Error for AuthSidecarError {}

impl From<SidecarContractError> for AuthSidecarError {
    fn from(error: SidecarContractError) -> Self {
        Self::SharedContract(error.to_string())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jwt_issue_request_renders_auditable_plan() {
        let request = JwtIssueRequest {
            issuer: valid_issuer(),
            algorithm: SigningAlgorithm::Rs256,
            claims: valid_claims(),
            audience: "postgres".to_string(),
        };

        let plan = request.plan().expect("issue plan");

        assert_eq!(plan.issuer, "https://auth.example.com");
        assert_eq!(plan.signing_key_ref, "jwt-signing-key");
        assert_eq!(plan.tenant_id, "tenant-a");
        assert_eq!(plan.ttl_seconds, 3_600);
    }

    #[test]
    fn oidc_provider_requires_https_issuer() {
        let provider = OidcProviderConfig {
            name: "github".to_string(),
            issuer_url: "http://github.example".to_string(),
            client_id_secret_ref: "github-client-id".to_string(),
            client_secret_ref: "github-client-secret".to_string(),
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
        let plan = AuthSidecarPlan {
            issuer: valid_issuer(),
            oidc_providers: vec![OidcProviderConfig {
                name: "github".to_string(),
                issuer_url: "https://github.example".to_string(),
                client_id_secret_ref: "github-client-id".to_string(),
                client_secret_ref: "github-client-secret".to_string(),
                scopes: vec!["openid".to_string(), "email".to_string()],
            }],
            mfa: Some(MfaPolicy {
                totp_enabled: true,
                webauthn_enabled: true,
                max_attempts: 3,
            }),
        };

        assert_eq!(plan.validate(), Ok(()));
    }

    fn valid_issuer() -> AuthIssuerContract {
        AuthIssuerContract {
            issuer: "https://auth.example.com".to_string(),
            signing_key_ref: "jwt-signing-key".to_string(),
            token_ttl_seconds: 3_600,
            tenant_claim: "tenant_id".to_string(),
        }
    }

    fn valid_claims() -> TokenClaims {
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
}
