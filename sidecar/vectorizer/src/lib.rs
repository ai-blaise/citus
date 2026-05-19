//! Vectorizer sidecar core.

// FEATURE: A2
// FEATURE: A4

use ai_blaise_citus_sidecar_shared::{ComponentState, HealthReport};
use std::error::Error;
use std::fmt;
use std::time::SystemTime;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct VectorizerJob {
    pub tenant_id: String,
    pub source_table: String,
    pub source_pk: String,
    pub source_text: String,
    pub provider: EmbeddingProvider,
    pub model: String,
    pub estimated_tokens: u64,
}

impl VectorizerJob {
    pub fn validate(&self) -> Result<(), VectorizerError> {
        validate_required("tenant_id", &self.tenant_id)?;
        validate_required("source_table", &self.source_table)?;
        validate_required("source_pk", &self.source_pk)?;
        validate_required("source_text", &self.source_text)?;
        validate_required("model", &self.model)?;
        if self.estimated_tokens == 0 {
            return Err(VectorizerError::InvalidTokenEstimate);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum EmbeddingProvider {
    OpenAi,
    AzureOpenAi,
    Anthropic,
    Cohere,
    Voyage,
    Ollama,
    VertexAi,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TenantTokenBudget {
    pub tenant_id: String,
    pub remaining_tokens: u64,
}

impl TenantTokenBudget {
    pub fn new(
        tenant_id: impl Into<String>,
        remaining_tokens: u64,
    ) -> Result<Self, VectorizerError> {
        let budget = Self {
            tenant_id: tenant_id.into(),
            remaining_tokens,
        };
        validate_required("tenant_id", &budget.tenant_id)?;
        Ok(budget)
    }

    pub fn reserve(&mut self, job: &VectorizerJob) -> Result<TokenReservation, VectorizerError> {
        job.validate()?;
        if job.tenant_id != self.tenant_id {
            return Err(VectorizerError::TenantMismatch);
        }
        if job.estimated_tokens > self.remaining_tokens {
            return Err(VectorizerError::BudgetExceeded {
                requested: job.estimated_tokens,
                remaining: self.remaining_tokens,
            });
        }

        self.remaining_tokens -= job.estimated_tokens;
        Ok(TokenReservation {
            tenant_id: self.tenant_id.clone(),
            reserved_tokens: job.estimated_tokens,
        })
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TokenReservation {
    pub tenant_id: String,
    pub reserved_tokens: u64,
}

pub fn health_report(started_at: SystemTime, queue_depth: u64) -> HealthReport {
    if queue_depth == 0 {
        return HealthReport::ready("vectorizer", started_at);
    }

    HealthReport {
        component: "vectorizer".to_string(),
        state: ComponentState::Ready,
        started_at,
        checked_at: SystemTime::now(),
        detail: Some(format!("queue_depth={queue_depth}")),
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum VectorizerError {
    BudgetExceeded { requested: u64, remaining: u64 },
    InvalidTokenEstimate,
    MissingRequiredField(&'static str),
    TenantMismatch,
}

impl fmt::Display for VectorizerError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BudgetExceeded {
                requested,
                remaining,
            } => write!(
                formatter,
                "token budget exceeded: requested {requested}, remaining {remaining}"
            ),
            Self::InvalidTokenEstimate => write!(formatter, "estimated_tokens must be positive"),
            Self::MissingRequiredField(field) => write!(formatter, "{field} must not be empty"),
            Self::TenantMismatch => write!(formatter, "job tenant does not match budget tenant"),
        }
    }
}

impl Error for VectorizerError {}

fn validate_required(field: &'static str, value: &str) -> Result<(), VectorizerError> {
    if value.trim().is_empty() {
        return Err(VectorizerError::MissingRequiredField(field));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reserves_tokens_for_matching_tenant() {
        let job = valid_job(128);
        let mut budget = TenantTokenBudget::new("tenant-a", 256).expect("budget");

        let reservation = budget.reserve(&job).expect("reservation");

        assert_eq!(
            reservation,
            TokenReservation {
                tenant_id: "tenant-a".to_string(),
                reserved_tokens: 128,
            }
        );
        assert_eq!(budget.remaining_tokens, 128);
    }

    #[test]
    fn rejects_budget_overrun() {
        let job = valid_job(512);
        let mut budget = TenantTokenBudget::new("tenant-a", 128).expect("budget");

        assert_eq!(
            budget.reserve(&job),
            Err(VectorizerError::BudgetExceeded {
                requested: 512,
                remaining: 128,
            })
        );
    }

    #[test]
    fn rejects_tenant_mismatch() {
        let mut job = valid_job(64);
        job.tenant_id = "tenant-b".to_string();
        let mut budget = TenantTokenBudget::new("tenant-a", 128).expect("budget");

        assert_eq!(budget.reserve(&job), Err(VectorizerError::TenantMismatch));
    }

    #[test]
    fn health_report_includes_queue_depth_detail() {
        let report = health_report(SystemTime::now(), 3);

        assert!(report.is_ready());
        assert_eq!(report.detail.as_deref(), Some("queue_depth=3"));
    }

    fn valid_job(estimated_tokens: u64) -> VectorizerJob {
        VectorizerJob {
            tenant_id: "tenant-a".to_string(),
            source_table: "documents".to_string(),
            source_pk: "42".to_string(),
            source_text: "hello world".to_string(),
            provider: EmbeddingProvider::OpenAi,
            model: "text-embedding-3-large".to_string(),
            estimated_tokens,
        }
    }
}
