// FEATURE: Auth2
// FEATURE: Sec1
// FEATURE: Sec2

use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SessionClaims {
    pub uid: String,
    pub role: String,
    pub tenant_id: String,
    pub jwt_id: Option<String>,
}

impl SessionClaims {
    pub fn validate(&self) -> Result<(), AuthError> {
        validate_required("uid", &self.uid)?;
        validate_required("role", &self.role)?;
        validate_required("tenant_id", &self.tenant_id)?;
        validate_optional("jwt_id", &self.jwt_id)
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct JwtVerificationPlan {
    pub issuer: String,
    pub audience: String,
    pub jwks_secret_ref: String,
}

impl JwtVerificationPlan {
    pub fn validate(&self) -> Result<(), AuthError> {
        validate_required("issuer", &self.issuer)?;
        validate_required("audience", &self.audience)?;
        validate_required("jwks_secret_ref", &self.jwks_secret_ref)
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TenantRlsPolicyPlan {
    pub table: String,
    pub tenant_column: String,
    pub claims: SessionClaims,
}

impl TenantRlsPolicyPlan {
    pub fn validate(&self) -> Result<(), AuthError> {
        validate_required("table", &self.table)?;
        validate_required("tenant_column", &self.tenant_column)?;
        self.claims.validate()
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum AuthError {
    MissingRequiredField(&'static str),
}

impl fmt::Display for AuthError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingRequiredField(field) => {
                write!(formatter, "{field} must not be empty")
            }
        }
    }
}

impl Error for AuthError {}

fn validate_required(field: &'static str, value: &str) -> Result<(), AuthError> {
    if value.trim().is_empty() {
        return Err(AuthError::MissingRequiredField(field));
    }
    Ok(())
}

fn validate_optional(field: &'static str, value: &Option<String>) -> Result<(), AuthError> {
    if matches!(value, Some(value) if value.trim().is_empty()) {
        return Err(AuthError::MissingRequiredField(field));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_tenant_rls_plan_passes() {
        let plan = TenantRlsPolicyPlan {
            table: "tenant_a.orders".to_string(),
            tenant_column: "tenant_id".to_string(),
            claims: SessionClaims {
                uid: "user-123".to_string(),
                role: "authenticated".to_string(),
                tenant_id: "tenant-a".to_string(),
                jwt_id: Some("jti-123".to_string()),
            },
        };

        assert_eq!(plan.validate(), Ok(()));
    }

    #[test]
    fn session_claims_require_tenant_id() {
        let claims = SessionClaims {
            uid: "user-123".to_string(),
            role: "authenticated".to_string(),
            tenant_id: String::new(),
            jwt_id: None,
        };

        assert_eq!(
            claims.validate(),
            Err(AuthError::MissingRequiredField("tenant_id"))
        );
    }

    #[test]
    fn jwt_verification_requires_secret_ref() {
        let plan = JwtVerificationPlan {
            issuer: "https://auth.example.com".to_string(),
            audience: "citus".to_string(),
            jwks_secret_ref: " ".to_string(),
        };

        assert_eq!(
            plan.validate(),
            Err(AuthError::MissingRequiredField("jwks_secret_ref"))
        );
    }
}
