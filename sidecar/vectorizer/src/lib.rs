//! Vectorizer sidecar core.

// FEATURE: A2
// FEATURE: A3
// FEATURE: A4
// FEATURE: A5
// FEATURE: A6

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
pub struct ProviderRoute {
    pub provider: EmbeddingProvider,
    pub model: String,
    pub secret_ref: String,
}

impl ProviderRoute {
    pub fn validate(&self) -> Result<(), VectorizerError> {
        validate_required("model", &self.model)?;
        validate_required("secret_ref", &self.secret_ref)
    }
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

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct QueuePollPlan {
    pub queue_table: String,
    pub batch_size: u32,
    pub visibility_timeout_seconds: u32,
}

impl QueuePollPlan {
    pub fn validate(&self) -> Result<(), VectorizerError> {
        validate_required("queue_table", &self.queue_table)?;
        if self.batch_size == 0 {
            return Err(VectorizerError::InvalidBatchSize);
        }
        if self.visibility_timeout_seconds == 0 {
            return Err(VectorizerError::InvalidVisibilityTimeout);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct DistributedVectorizePlan {
    pub worker_name: String,
    pub shard_id: u64,
    pub queue_table: String,
    pub local_only: bool,
}

impl DistributedVectorizePlan {
    pub fn validate(&self) -> Result<(), VectorizerError> {
        validate_required("worker_name", &self.worker_name)?;
        validate_required("queue_table", &self.queue_table)?;
        if self.shard_id == 0 {
            return Err(VectorizerError::InvalidShardId);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ProviderEmbeddingRequest {
    pub tenant_id: String,
    pub source_table: String,
    pub source_pk: String,
    pub provider: EmbeddingProvider,
    pub model: String,
    pub input: String,
    pub reserved_tokens: u64,
}

impl ProviderEmbeddingRequest {
    fn from_job(job: &VectorizerJob, reservation: TokenReservation) -> Self {
        Self {
            tenant_id: job.tenant_id.clone(),
            source_table: job.source_table.clone(),
            source_pk: job.source_pk.clone(),
            provider: job.provider.clone(),
            model: job.model.clone(),
            input: job.source_text.clone(),
            reserved_tokens: reservation.reserved_tokens,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProviderEmbeddingResult {
    pub source_table: String,
    pub source_pk: String,
    pub embedding: Vec<f32>,
    pub usage: UsageLogRecord,
}

impl ProviderEmbeddingResult {
    pub fn validate(&self) -> Result<(), VectorizerError> {
        validate_required("source_table", &self.source_table)?;
        validate_required("source_pk", &self.source_pk)?;
        if self.embedding.is_empty() {
            return Err(VectorizerError::EmptyEmbedding);
        }
        self.usage.validate()
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct UsageLogRecord {
    pub tenant_id: String,
    pub provider: EmbeddingProvider,
    pub model: String,
    pub tokens: u64,
    pub cost_micros: u64,
}

impl UsageLogRecord {
    pub fn validate(&self) -> Result<(), VectorizerError> {
        validate_required("tenant_id", &self.tenant_id)?;
        validate_required("model", &self.model)?;
        if self.tokens == 0 {
            return Err(VectorizerError::InvalidTokenEstimate);
        }
        Ok(())
    }
}

pub trait EmbeddingProviderClient {
    fn embed(
        &self,
        request: &ProviderEmbeddingRequest,
    ) -> Result<ProviderEmbeddingResult, VectorizerError>;
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct DeterministicEmbeddingClient {
    pub dimensions: usize,
    pub cost_micros_per_token: u64,
}

impl EmbeddingProviderClient for DeterministicEmbeddingClient {
    fn embed(
        &self,
        request: &ProviderEmbeddingRequest,
    ) -> Result<ProviderEmbeddingResult, VectorizerError> {
        if self.dimensions == 0 {
            return Err(VectorizerError::EmptyEmbedding);
        }
        validate_required("input", &request.input)?;

        // Provider-stub embedding values do not need full u64 precision; the
        // canonical contract only checks ordering of the deterministic series.
        #[allow(clippy::cast_precision_loss)]
        let embedding = (0..self.dimensions)
            .map(|index| (request.reserved_tokens as f32 + index as f32) / 1000.0)
            .collect();

        Ok(ProviderEmbeddingResult {
            source_table: request.source_table.clone(),
            source_pk: request.source_pk.clone(),
            embedding,
            usage: UsageLogRecord {
                tenant_id: request.tenant_id.clone(),
                provider: request.provider.clone(),
                model: request.model.clone(),
                tokens: request.reserved_tokens,
                cost_micros: request.reserved_tokens * self.cost_micros_per_token,
            },
        })
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct VectorizerRunPlan {
    pub queue: QueuePollPlan,
    pub distributed: DistributedVectorizePlan,
    pub requests: Vec<ProviderEmbeddingRequest>,
}

impl VectorizerRunPlan {
    pub fn validate(&self) -> Result<(), VectorizerError> {
        self.queue.validate()?;
        self.distributed.validate()?;
        if self.requests.is_empty() {
            return Err(VectorizerError::EmptyBatch);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct VectorizerExecutionReport {
    pub plan: VectorizerRunPlan,
    pub results: Vec<ProviderEmbeddingResult>,
    pub usage: Vec<UsageLogRecord>,
    pub health: HealthReport,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct VectorizerWorker {
    pub queue: QueuePollPlan,
    pub distributed: DistributedVectorizePlan,
    pub routes: Vec<ProviderRoute>,
}

impl VectorizerWorker {
    pub fn plan_batch(
        &self,
        jobs: &[VectorizerJob],
        budgets: &mut [TenantTokenBudget],
    ) -> Result<VectorizerRunPlan, VectorizerError> {
        self.queue.validate()?;
        self.distributed.validate()?;
        if jobs.is_empty() {
            return Err(VectorizerError::EmptyBatch);
        }
        if jobs.len() > self.queue.batch_size as usize {
            // jobs.len() exceeded the queue batch size (a u32 ceiling), so a
            // u32 cap is correct for reporting; clamp to u32::MAX defensively.
            let requested = u32::try_from(jobs.len()).unwrap_or(u32::MAX);
            return Err(VectorizerError::BatchTooLarge {
                requested,
                max: self.queue.batch_size,
            });
        }

        let mut requests = Vec::with_capacity(jobs.len());
        for job in jobs {
            job.validate()?;
            self.validate_route(job)?;
            let budget = budgets
                .iter_mut()
                .find(|budget| budget.tenant_id == job.tenant_id)
                .ok_or(VectorizerError::BudgetNotFound)?;
            let reservation = budget.reserve(job)?;
            requests.push(ProviderEmbeddingRequest::from_job(job, reservation));
        }

        Ok(VectorizerRunPlan {
            queue: self.queue.clone(),
            distributed: self.distributed.clone(),
            requests,
        })
    }

    pub fn execute_batch<C: EmbeddingProviderClient>(
        &self,
        jobs: &[VectorizerJob],
        budgets: &mut [TenantTokenBudget],
        client: &C,
    ) -> Result<VectorizerExecutionReport, VectorizerError> {
        let plan = self.plan_batch(jobs, budgets)?;
        let mut results = Vec::with_capacity(plan.requests.len());
        let mut usage = Vec::with_capacity(plan.requests.len());

        for request in &plan.requests {
            let result = client.embed(request)?;
            result.validate()?;
            usage.push(result.usage.clone());
            results.push(result);
        }

        Ok(VectorizerExecutionReport {
            plan,
            results,
            usage,
            health: health_report(SystemTime::now(), 0),
        })
    }

    fn validate_route(&self, job: &VectorizerJob) -> Result<(), VectorizerError> {
        if self.routes.is_empty() {
            return Err(VectorizerError::MissingProviderRoute);
        }

        if self.routes.iter().any(|route| {
            route.provider == job.provider && route.model == job.model && route.validate().is_ok()
        }) {
            return Ok(());
        }

        Err(VectorizerError::MissingProviderRoute)
    }
}

pub fn canonical_worker() -> VectorizerWorker {
    VectorizerWorker {
        queue: QueuePollPlan {
            queue_table: "ai.vectorizer_queue".to_string(),
            batch_size: 8,
            visibility_timeout_seconds: 30,
        },
        distributed: DistributedVectorizePlan {
            worker_name: "worker-1".to_string(),
            shard_id: 10_240,
            queue_table: "ai.vectorizer_queue".to_string(),
            local_only: true,
        },
        routes: vec![ProviderRoute {
            provider: EmbeddingProvider::OpenAi,
            model: "text-embedding-3-large".to_string(),
            secret_ref: "openai-embeddings".to_string(),
        }],
    }
}

pub fn canonical_jobs() -> Vec<VectorizerJob> {
    vec![
        VectorizerJob {
            tenant_id: "tenant-a".to_string(),
            source_table: "public.documents".to_string(),
            source_pk: "doc-1".to_string(),
            source_text: "Citus shards tenant data across workers.".to_string(),
            provider: EmbeddingProvider::OpenAi,
            model: "text-embedding-3-large".to_string(),
            estimated_tokens: 64,
        },
        VectorizerJob {
            tenant_id: "tenant-a".to_string(),
            source_table: "public.documents".to_string(),
            source_pk: "doc-2".to_string(),
            source_text: "The vectorizer runs shard-local batches.".to_string(),
            provider: EmbeddingProvider::OpenAi,
            model: "text-embedding-3-large".to_string(),
            estimated_tokens: 48,
        },
    ]
}

pub fn canonical_budgets() -> Vec<TenantTokenBudget> {
    vec![TenantTokenBudget {
        tenant_id: "tenant-a".to_string(),
        remaining_tokens: 256,
    }]
}

pub fn canonical_execution_report() -> Result<VectorizerExecutionReport, VectorizerError> {
    let worker = canonical_worker();
    let jobs = canonical_jobs();
    let mut budgets = canonical_budgets();
    let client = DeterministicEmbeddingClient {
        dimensions: 3,
        cost_micros_per_token: 10,
    };

    worker.execute_batch(&jobs, &mut budgets, &client)
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
    BatchTooLarge { requested: u32, max: u32 },
    BudgetNotFound,
    BudgetExceeded { requested: u64, remaining: u64 },
    EmptyBatch,
    EmptyEmbedding,
    InvalidBatchSize,
    InvalidShardId,
    InvalidTokenEstimate,
    InvalidVisibilityTimeout,
    MissingProviderRoute,
    MissingRequiredField(&'static str),
    TenantMismatch,
}

impl fmt::Display for VectorizerError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BatchTooLarge { requested, max } => {
                write!(
                    formatter,
                    "batch too large: requested {requested}, max {max}"
                )
            }
            Self::BudgetNotFound => write!(formatter, "tenant token budget not found"),
            Self::BudgetExceeded {
                requested,
                remaining,
            } => write!(
                formatter,
                "token budget exceeded: requested {requested}, remaining {remaining}"
            ),
            Self::EmptyBatch => write!(formatter, "batch must contain at least one job"),
            Self::EmptyEmbedding => write!(formatter, "embedding must not be empty"),
            Self::InvalidBatchSize => write!(formatter, "batch_size must be greater than zero"),
            Self::InvalidShardId => write!(formatter, "shard_id must be greater than zero"),
            Self::InvalidTokenEstimate => write!(formatter, "estimated_tokens must be positive"),
            Self::InvalidVisibilityTimeout => {
                write!(
                    formatter,
                    "visibility_timeout_seconds must be greater than zero"
                )
            }
            Self::MissingProviderRoute => write!(formatter, "provider route not configured"),
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

    #[test]
    fn worker_plans_provider_requests_and_reserves_budgets() {
        let worker = valid_worker();
        let jobs = vec![valid_job(64), valid_job(32)];
        let mut budgets = vec![TenantTokenBudget::new("tenant-a", 128).expect("budget")];

        let plan = worker.plan_batch(&jobs, &mut budgets).expect("run plan");

        assert_eq!(plan.requests.len(), 2);
        assert_eq!(budgets[0].remaining_tokens, 32);
        assert_eq!(plan.requests[0].model, "text-embedding-3-large");
        assert_eq!(plan.validate(), Ok(()));
    }

    #[test]
    fn worker_rejects_missing_provider_route() {
        let mut worker = valid_worker();
        worker.routes.clear();
        let jobs = vec![valid_job(64)];
        let mut budgets = vec![TenantTokenBudget::new("tenant-a", 128).expect("budget")];

        assert_eq!(
            worker.plan_batch(&jobs, &mut budgets),
            Err(VectorizerError::MissingProviderRoute)
        );
    }

    #[test]
    fn worker_rejects_batches_larger_than_queue_limit() {
        let mut worker = valid_worker();
        worker.queue.batch_size = 1;
        let jobs = vec![valid_job(64), valid_job(32)];
        let mut budgets = vec![TenantTokenBudget::new("tenant-a", 128).expect("budget")];

        assert_eq!(
            worker.plan_batch(&jobs, &mut budgets),
            Err(VectorizerError::BatchTooLarge {
                requested: 2,
                max: 1,
            })
        );
    }

    #[test]
    fn deterministic_provider_returns_usage_record() {
        let worker = valid_worker();
        let jobs = vec![valid_job(64)];
        let mut budgets = vec![TenantTokenBudget::new("tenant-a", 128).expect("budget")];
        let plan = worker.plan_batch(&jobs, &mut budgets).expect("run plan");
        let client = DeterministicEmbeddingClient {
            dimensions: 3,
            cost_micros_per_token: 10,
        };

        let result = client.embed(&plan.requests[0]).expect("embedding result");

        assert_eq!(result.embedding.len(), 3);
        assert_eq!(result.usage.tokens, 64);
        assert_eq!(result.usage.cost_micros, 640);
        assert_eq!(result.validate(), Ok(()));
    }

    #[test]
    fn worker_executes_batch_and_returns_usage_report() {
        let worker = canonical_worker();
        let jobs = canonical_jobs();
        let mut budgets = canonical_budgets();
        let client = DeterministicEmbeddingClient {
            dimensions: 3,
            cost_micros_per_token: 10,
        };

        let report = worker
            .execute_batch(&jobs, &mut budgets, &client)
            .expect("execution report");

        assert_eq!(report.plan.requests.len(), 2);
        assert_eq!(report.results.len(), 2);
        assert_eq!(
            report.usage.iter().map(|record| record.tokens).sum::<u64>(),
            112
        );
        assert_eq!(
            report
                .usage
                .iter()
                .map(|record| record.cost_micros)
                .sum::<u64>(),
            1_120
        );
        assert!(report.health.is_ready());
        assert_eq!(budgets[0].remaining_tokens, 144);
    }

    #[test]
    fn canonical_execution_report_is_deterministic() {
        let report = canonical_execution_report().expect("canonical report");

        assert_eq!(report.results[0].embedding, vec![0.064, 0.065, 0.066]);
        assert_eq!(report.results[1].embedding, vec![0.048, 0.049, 0.05]);
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

    fn valid_worker() -> VectorizerWorker {
        VectorizerWorker {
            queue: QueuePollPlan {
                queue_table: "ai.vectorizer_queue".to_string(),
                batch_size: 8,
                visibility_timeout_seconds: 30,
            },
            distributed: DistributedVectorizePlan {
                worker_name: "worker-1".to_string(),
                shard_id: 10_240,
                queue_table: "ai.vectorizer_queue".to_string(),
                local_only: true,
            },
            routes: vec![ProviderRoute {
                provider: EmbeddingProvider::OpenAi,
                model: "text-embedding-3-large".to_string(),
                secret_ref: "openai-embeddings".to_string(),
            }],
        }
    }
}
