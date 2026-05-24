//! Top-level vectorizer worker that ties the queue, providers, budgets, and
//! usage log into a single poll loop.

// FEATURE: A2
// FEATURE: A3
// FEATURE: A4
// FEATURE: A5
// FEATURE: A6

use crate::runtime::budget::{BudgetError, BudgetStore};
use crate::runtime::contract::VectorizerRuntimeContract;
use crate::runtime::provider::{
    AsyncEmbeddingProvider, EmbeddingError, EmbeddingResponse, ProviderRegistry,
};
use crate::runtime::queue::{QueueError, QueueRow, QueueStore};
use crate::runtime::usage_log::{UsageLogEntry, UsageLogError, UsageLogStore};
use std::error::Error;
use std::fmt;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Mutex, Notify};

#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    pub database_url: String,
    pub queue_table: String,
    pub budget_table: String,
    pub usage_log_table: String,
    pub listen_addr: String,
    pub batch_size: u32,
    pub poll_interval: Duration,
    pub visibility_timeout: Duration,
    pub retry_initial_backoff: Duration,
    pub provider_max_attempts: u32,
    pub mock_dimensions: usize,
    pub provider_mode: String,
    pub dimension_contract: Option<VectorizerRuntimeContract>,
}

impl RuntimeConfig {
    pub fn validate(&self) -> Result<(), RuntimeError> {
        if self.database_url.trim().is_empty() {
            return Err(RuntimeError::InvalidEnv(
                "AI_BLAISE_VECTORIZER_DATABASE_URL",
                "must not be empty".to_string(),
            ));
        }
        if self.batch_size == 0 {
            return Err(RuntimeError::InvalidEnv(
                "AI_BLAISE_VECTORIZER_BATCH_SIZE",
                "must be greater than zero".to_string(),
            ));
        }
        if self.poll_interval.is_zero() {
            return Err(RuntimeError::InvalidEnv(
                "AI_BLAISE_VECTORIZER_POLL_INTERVAL_MS",
                "must be greater than zero".to_string(),
            ));
        }
        if self.visibility_timeout.is_zero() {
            return Err(RuntimeError::InvalidEnv(
                "AI_BLAISE_VECTORIZER_VISIBILITY_TIMEOUT_SECONDS",
                "must be greater than zero".to_string(),
            ));
        }
        if self.retry_initial_backoff.is_zero() {
            return Err(RuntimeError::InvalidEnv(
                "AI_BLAISE_VECTORIZER_RETRY_INITIAL_BACKOFF_MS",
                "must be greater than zero".to_string(),
            ));
        }
        if self.provider_max_attempts == 0 {
            return Err(RuntimeError::InvalidEnv(
                "AI_BLAISE_VECTORIZER_PROVIDER_MAX_ATTEMPTS",
                "must be greater than zero".to_string(),
            ));
        }
        if self.mock_dimensions == 0 {
            return Err(RuntimeError::InvalidEnv(
                "AI_BLAISE_VECTORIZER_MOCK_DIMENSIONS",
                "must be greater than zero".to_string(),
            ));
        }
        if let Some(contract) = &self.dimension_contract {
            contract.validate().map_err(|error| {
                RuntimeError::InvalidEnv("AI_BLAISE_VECTORIZER_CONTRACT_*", error.to_string())
            })?;
            if self.provider_mode == "mock" && contract.provider != "mock" {
                return Err(RuntimeError::InvalidEnv(
                    "AI_BLAISE_VECTORIZER_CONTRACT_PROVIDER",
                    "provider mode mock can only satisfy a mock vectorizer contract".to_string(),
                ));
            }
            if self.provider_mode == "mock"
                && contract.provider == "mock"
                && contract.dimensions != self.mock_dimensions
            {
                return Err(RuntimeError::InvalidEnv(
                    "AI_BLAISE_VECTORIZER_MOCK_DIMENSIONS",
                    format!(
                        "must equal AI_BLAISE_VECTORIZER_CONTRACT_DIMENSIONS ({})",
                        contract.dimensions
                    ),
                ));
            }
        }
        validate_qualified_table_name("AI_BLAISE_VECTORIZER_QUEUE_TABLE", &self.queue_table)?;
        validate_qualified_table_name("AI_BLAISE_VECTORIZER_BUDGET_TABLE", &self.budget_table)?;
        validate_qualified_table_name(
            "AI_BLAISE_VECTORIZER_USAGE_LOG_TABLE",
            &self.usage_log_table,
        )?;
        match self.provider_mode.as_str() {
            "mock" | "live" | "mixed" => Ok(()),
            other => Err(RuntimeError::InvalidEnv(
                "AI_BLAISE_VECTORIZER_PROVIDER_MODE",
                format!("expected mock, live, or mixed; got {other}"),
            )),
        }
    }
}

#[derive(Debug, Clone)]
pub struct WorkerMetrics {
    pub batches_processed: u64,
    pub rows_embedded: u64,
    pub rows_failed: u64,
    pub last_error: Option<String>,
}

impl WorkerMetrics {
    fn new() -> Self {
        Self {
            batches_processed: 0,
            rows_embedded: 0,
            rows_failed: 0,
            last_error: None,
        }
    }
}

pub struct VectorizerRuntime {
    config: RuntimeConfig,
    queue: Arc<dyn QueueStore>,
    budgets: Arc<dyn BudgetStore>,
    usage_log: Arc<dyn UsageLogStore>,
    providers: Arc<ProviderRegistry>,
    metrics: Arc<Mutex<WorkerMetrics>>,
    cost_table: Arc<dyn ProviderCostTable>,
    shutdown: Arc<Notify>,
    worker_id: String,
}

impl VectorizerRuntime {
    pub fn new(
        config: RuntimeConfig,
        queue: Arc<dyn QueueStore>,
        budgets: Arc<dyn BudgetStore>,
        usage_log: Arc<dyn UsageLogStore>,
        providers: Arc<ProviderRegistry>,
        cost_table: Arc<dyn ProviderCostTable>,
        worker_id: impl Into<String>,
    ) -> Self {
        Self {
            config,
            queue,
            budgets,
            usage_log,
            providers,
            metrics: Arc::new(Mutex::new(WorkerMetrics::new())),
            cost_table,
            shutdown: Arc::new(Notify::new()),
            worker_id: worker_id.into(),
        }
    }

    pub fn queue(&self) -> Arc<dyn QueueStore> {
        self.queue.clone()
    }

    pub fn budgets(&self) -> Arc<dyn BudgetStore> {
        self.budgets.clone()
    }

    pub fn usage_log(&self) -> Arc<dyn UsageLogStore> {
        self.usage_log.clone()
    }

    pub fn providers(&self) -> Arc<ProviderRegistry> {
        self.providers.clone()
    }

    pub fn config(&self) -> &RuntimeConfig {
        &self.config
    }

    pub fn dimension_contract(&self) -> Option<&VectorizerRuntimeContract> {
        self.config.dimension_contract.as_ref()
    }

    pub fn shutdown_handle(&self) -> Arc<Notify> {
        self.shutdown.clone()
    }

    pub fn worker_id(&self) -> &str {
        &self.worker_id
    }

    pub async fn metrics_snapshot(&self) -> WorkerMetrics {
        self.metrics.lock().await.clone()
    }

    /// Process a single batch: lock rows, reserve tokens, call the provider,
    /// record usage, and mark rows succeeded/failed. Returns the number of
    /// rows the batch processed (successful + failed).
    pub async fn process_one_batch(&self) -> Result<usize, RuntimeError> {
        let rows = self
            .queue
            .lock_batch(
                &self.worker_id,
                self.config.batch_size,
                self.config.visibility_timeout,
            )
            .await
            .map_err(RuntimeError::Queue)?;
        if rows.is_empty() {
            return Ok(0);
        }

        let grouped = group_by_provider_model(rows);
        let mut total = 0usize;
        for group in grouped {
            total += self.process_provider_group(group).await?;
        }
        let mut metrics = self.metrics.lock().await;
        metrics.batches_processed += 1;
        Ok(total)
    }

    async fn process_provider_group(&self, group: ProviderGroup) -> Result<usize, RuntimeError> {
        let total_rows = group.rows.len();
        if let Some(contract) = self.dimension_contract() {
            if let Err(error) = contract.assert_route(&group.provider, &group.model) {
                let detail = error.to_string();
                self.fail_rows(&group.rows, &detail).await;
                let mut metrics = self.metrics.lock().await;
                metrics.last_error = Some(detail);
                return Ok(total_rows);
            }
        }

        let provider = match self.providers.get(&group.provider) {
            Some(provider) => provider,
            None => {
                let detail = format!("provider {} not registered", group.provider);
                self.fail_rows(&group.rows, &detail).await;
                return Ok(total_rows);
            }
        };

        // Estimate tokens per row up-front so we can reserve budget before any
        // network call. Rows whose tenant has insufficient budget are failed
        // individually so other tenants in the same batch can still complete.
        let mut admitted_rows: Vec<QueueRow> = Vec::with_capacity(total_rows);
        let mut reservations: Vec<u64> = Vec::with_capacity(total_rows);
        for row in group.rows {
            let estimate = estimate_input_tokens(&row.source_text);
            match self.budgets.reserve_tokens(&row.tenant_id, estimate).await {
                Ok(_remaining) => {
                    admitted_rows.push(row);
                    reservations.push(estimate);
                }
                Err(BudgetError::Exceeded {
                    requested,
                    remaining,
                }) => {
                    let detail = format!(
                        "budget exceeded: tenant={} requested={} remaining={}",
                        row.tenant_id, requested, remaining
                    );
                    self.fail_rows(std::slice::from_ref(&row), &detail).await;
                }
                Err(BudgetError::NotFound) => {
                    let detail = format!("tenant budget not found: tenant={}", row.tenant_id);
                    self.fail_rows(std::slice::from_ref(&row), &detail).await;
                }
                Err(BudgetError::Storage(detail)) => {
                    // Refund every prior reservation in this batch and bail.
                    for (admitted, estimate) in admitted_rows.iter().zip(reservations.iter()) {
                        let _ = self
                            .budgets
                            .refund_tokens(&admitted.tenant_id, *estimate)
                            .await;
                    }
                    return Err(RuntimeError::Budget(BudgetError::Storage(detail)));
                }
            }
        }

        if admitted_rows.is_empty() {
            return Ok(total_rows);
        }

        let inputs: Vec<String> = admitted_rows
            .iter()
            .map(|row| row.source_text.clone())
            .collect();
        let admitted_group = ProviderGroup {
            provider: group.provider.clone(),
            model: group.model.clone(),
            rows: admitted_rows,
        };

        match self
            .embed_with_retry(provider.as_ref(), &admitted_group.model, &inputs)
            .await
        {
            Ok(response) => {
                self.handle_provider_success(
                    provider.name(),
                    admitted_group,
                    response,
                    reservations,
                )
                .await?;
            }
            Err(error) => {
                // Refund every reservation since the call did not bill them.
                for (row, estimate) in admitted_group.rows.iter().zip(reservations.iter()) {
                    let _ = self.budgets.refund_tokens(&row.tenant_id, *estimate).await;
                }
                let detail = format!("{error}");
                self.fail_rows(&admitted_group.rows, &detail).await;
                let mut metrics = self.metrics.lock().await;
                metrics.last_error = Some(detail);
            }
        }
        Ok(total_rows)
    }

    async fn handle_provider_success(
        &self,
        provider_name: &str,
        group: ProviderGroup,
        response: EmbeddingResponse,
        reservations: Vec<u64>,
    ) -> Result<(), RuntimeError> {
        if response.embeddings.len() != group.rows.len() {
            let detail = format!(
                "provider returned {} embeddings for {} rows",
                response.embeddings.len(),
                group.rows.len()
            );
            for (row, estimate) in group.rows.iter().zip(reservations.iter()) {
                let _ = self.budgets.refund_tokens(&row.tenant_id, *estimate).await;
            }
            self.fail_rows(&group.rows, &detail).await;
            let mut metrics = self.metrics.lock().await;
            metrics.last_error = Some(detail);
            return Ok(());
        }

        if let Some(contract) = self.dimension_contract() {
            if let Err(error) = contract.assert_embeddings(&response.embeddings) {
                let detail = error.to_string();
                for (row, estimate) in group.rows.iter().zip(reservations.iter()) {
                    let _ = self.budgets.refund_tokens(&row.tenant_id, *estimate).await;
                }
                self.fail_rows(&group.rows, &detail).await;
                let mut metrics = self.metrics.lock().await;
                metrics.last_error = Some(detail);
                return Ok(());
            }
        }

        let billed_tokens = allocate_tokens(&reservations, response.total_tokens);
        for ((row, reserved), billed) in group
            .rows
            .iter()
            .zip(reservations.iter())
            .zip(billed_tokens.iter())
        {
            if *billed > *reserved {
                self.budgets
                    .reserve_tokens(&row.tenant_id, *billed - *reserved)
                    .await
                    .map_err(RuntimeError::Budget)?;
            } else if *billed < *reserved {
                self.budgets
                    .refund_tokens(&row.tenant_id, *reserved - *billed)
                    .await
                    .map_err(RuntimeError::Budget)?;
            }
        }

        let cost_per_token = self.cost_micros_per_token(provider_name);
        let ids: Vec<i64> = group.rows.iter().map(|row| row.id).collect();
        self.queue
            .mark_succeeded(&ids, &response.embeddings)
            .await
            .map_err(RuntimeError::Queue)?;

        for (row, tokens) in group.rows.iter().zip(billed_tokens.iter()) {
            let entry = UsageLogEntry {
                tenant_id: row.tenant_id.clone(),
                provider: provider_name.to_string(),
                model: group.model.clone(),
                tokens: *tokens,
                cost_micros: tokens.saturating_mul(cost_per_token),
            };
            self.usage_log
                .record(&entry)
                .await
                .map_err(RuntimeError::UsageLog)?;
        }

        let mut metrics = self.metrics.lock().await;
        metrics.rows_embedded += group.rows.len() as u64;
        Ok(())
    }

    async fn fail_rows(&self, rows: &[QueueRow], detail: &str) {
        for row in rows {
            let _ = self.queue.mark_failed(row.id, detail).await;
        }
        let mut metrics = self.metrics.lock().await;
        metrics.rows_failed += rows.len() as u64;
    }

    /// Run the poll loop until [`shutdown_handle`] is notified.
    pub async fn run_until_shutdown(&self) -> Result<(), RuntimeError> {
        let shutdown = self.shutdown.clone();
        loop {
            tokio::select! {
                _ = shutdown.notified() => return Ok(()),
                _ = tokio::time::sleep(self.config.poll_interval) => {}
            }
            match self.process_one_batch().await {
                Ok(0) => continue,
                Ok(_) => continue,
                Err(error) => {
                    let mut metrics = self.metrics.lock().await;
                    metrics.last_error = Some(error.to_string());
                    drop(metrics);
                    tracing::warn!(error = %error, "vectorizer batch failed; continuing");
                }
            }
        }
    }

    pub fn cost_micros_per_token(&self, provider: &str) -> u64 {
        self.cost_table.cost_micros_per_token(provider)
    }

    pub async fn embed_with_retry(
        &self,
        provider: &dyn AsyncEmbeddingProvider,
        model: &str,
        inputs: &[String],
    ) -> Result<EmbeddingResponse, EmbeddingError> {
        let mut attempt = 1u32;
        loop {
            match provider.embed(model, inputs).await {
                Ok(response) => return Ok(response),
                Err(error)
                    if error.is_retryable() && attempt < self.config.provider_max_attempts =>
                {
                    let multiplier = 1u32 << (attempt - 1).min(10);
                    let delay = self.config.retry_initial_backoff.saturating_mul(multiplier);
                    tracing::warn!(
                        provider = provider.name(),
                        attempt,
                        max_attempts = self.config.provider_max_attempts,
                        error = %error,
                        "embedding provider retryable failure"
                    );
                    tokio::time::sleep(delay).await;
                    attempt += 1;
                }
                Err(error) => return Err(error),
            }
        }
    }

    pub fn trigger_shutdown(&self) {
        self.shutdown.notify_waiters();
    }
}

/// Static per-provider cost-per-token table.
pub trait ProviderCostTable: Send + Sync {
    fn cost_micros_per_token(&self, provider: &str) -> u64;
}

#[derive(Debug, Clone)]
pub struct StaticCostTable {
    entries: std::collections::HashMap<String, u64>,
    default_cost: u64,
}

impl StaticCostTable {
    pub fn new(default_cost: u64) -> Self {
        Self {
            entries: std::collections::HashMap::new(),
            default_cost,
        }
    }

    pub fn with(mut self, provider: impl Into<String>, cost_micros_per_token: u64) -> Self {
        self.entries.insert(provider.into(), cost_micros_per_token);
        self
    }
}

impl ProviderCostTable for StaticCostTable {
    fn cost_micros_per_token(&self, provider: &str) -> u64 {
        self.entries
            .get(provider)
            .copied()
            .unwrap_or(self.default_cost)
    }
}

#[derive(Debug)]
struct ProviderGroup {
    provider: String,
    model: String,
    rows: Vec<QueueRow>,
}

fn group_by_provider_model(rows: Vec<QueueRow>) -> Vec<ProviderGroup> {
    let mut groups: Vec<ProviderGroup> = Vec::new();
    for row in rows {
        if let Some(group) = groups
            .iter_mut()
            .find(|group| group.provider == row.provider && group.model == row.model)
        {
            group.rows.push(row);
        } else {
            groups.push(ProviderGroup {
                provider: row.provider.clone(),
                model: row.model.clone(),
                rows: vec![row],
            });
        }
    }
    groups
}

fn estimate_input_tokens(input: &str) -> u64 {
    (input.len().div_ceil(4)) as u64
}

fn allocate_tokens(estimates: &[u64], total_tokens: u64) -> Vec<u64> {
    if estimates.is_empty() {
        return Vec::new();
    }
    if total_tokens == 0 {
        return estimates.iter().map(|_| 1).collect();
    }

    let estimated_total: u64 = estimates.iter().sum();
    if estimated_total == 0 {
        let base = total_tokens / estimates.len() as u64;
        let mut remainder = total_tokens % estimates.len() as u64;
        return estimates
            .iter()
            .map(|_| {
                let extra = u64::from(remainder > 0);
                remainder = remainder.saturating_sub(extra);
                (base + extra).max(1)
            })
            .collect();
    }

    let mut allocated = Vec::with_capacity(estimates.len());
    let mut assigned = 0u64;
    for estimate in estimates {
        let value = ((*estimate as u128 * total_tokens as u128) / estimated_total as u128) as u64;
        let value = value.max(1);
        assigned = assigned.saturating_add(value);
        allocated.push(value);
    }
    while assigned > total_tokens && allocated.len() > 1 {
        if let Some(value) = allocated.iter_mut().rev().find(|value| **value > 1) {
            *value -= 1;
            assigned -= 1;
        } else {
            break;
        }
    }
    while assigned < total_tokens {
        if let Some(value) = allocated.first_mut() {
            *value += 1;
            assigned += 1;
        }
    }
    allocated
}

fn validate_qualified_table_name(name: &'static str, value: &str) -> Result<(), RuntimeError> {
    let parts: Vec<&str> = value.split('.').collect();
    if parts.len() != 2 || parts.iter().any(|part| !is_sql_identifier(part)) {
        return Err(RuntimeError::InvalidEnv(
            name,
            "expected schema.table with unquoted SQL identifiers".to_string(),
        ));
    }
    Ok(())
}

fn is_sql_identifier(value: &str) -> bool {
    let mut chars = value.chars();
    match chars.next() {
        Some(first) if first == '_' || first.is_ascii_alphabetic() => {}
        _ => return false,
    }
    chars.all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
}

/// Errors emitted by the runtime layer.
#[derive(Debug, Clone)]
pub enum RuntimeError {
    Budget(BudgetError),
    UsageLog(UsageLogError),
    Queue(QueueError),
    Embedding(EmbeddingError),
    MissingEnv(&'static str),
    InvalidEnv(&'static str, String),
    Storage(String),
    Server(String),
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Budget(error) => write!(formatter, "{error}"),
            Self::UsageLog(error) => write!(formatter, "{error}"),
            Self::Queue(error) => write!(formatter, "{error}"),
            Self::Embedding(error) => write!(formatter, "{error}"),
            Self::MissingEnv(name) => write!(formatter, "missing required env var: {name}"),
            Self::InvalidEnv(name, detail) => {
                write!(formatter, "invalid env var {name}: {detail}")
            }
            Self::Storage(detail) => write!(formatter, "vectorizer storage error: {detail}"),
            Self::Server(detail) => write!(formatter, "vectorizer server error: {detail}"),
        }
    }
}

impl Error for RuntimeError {}

impl From<BudgetError> for RuntimeError {
    fn from(error: BudgetError) -> Self {
        Self::Budget(error)
    }
}

impl From<UsageLogError> for RuntimeError {
    fn from(error: UsageLogError) -> Self {
        Self::UsageLog(error)
    }
}

impl From<QueueError> for RuntimeError {
    fn from(error: QueueError) -> Self {
        Self::Queue(error)
    }
}

impl From<EmbeddingError> for RuntimeError {
    fn from(error: EmbeddingError) -> Self {
        Self::Embedding(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::budget::InMemoryBudgetStore;
    use crate::runtime::provider::MockProvider;
    use crate::runtime::queue::InMemoryQueueStore;
    use crate::runtime::usage_log::InMemoryUsageLog;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn runtime_with_mock(
        dimensions: usize,
        cost_micros_per_token: u64,
        budget: u64,
    ) -> (
        VectorizerRuntime,
        Arc<InMemoryQueueStore>,
        Arc<InMemoryBudgetStore>,
        Arc<InMemoryUsageLog>,
    ) {
        let queue = Arc::new(InMemoryQueueStore::new());
        let budgets = Arc::new(InMemoryBudgetStore::new());
        let usage_log = Arc::new(InMemoryUsageLog::new());
        let mut registry = ProviderRegistry::new();
        registry.insert(Arc::new(MockProvider::new(
            "mock",
            dimensions,
            cost_micros_per_token,
        )));
        let providers = Arc::new(registry);
        let cost = Arc::new(
            StaticCostTable::new(cost_micros_per_token).with("mock", cost_micros_per_token),
        );
        let config = RuntimeConfig {
            database_url: "postgres://localhost/test".to_string(),
            queue_table: "ai.vectorizer_queue".to_string(),
            budget_table: "ai.tenant_budget".to_string(),
            usage_log_table: "ai.usage_log".to_string(),
            listen_addr: "127.0.0.1:0".to_string(),
            batch_size: 8,
            poll_interval: Duration::from_millis(5),
            visibility_timeout: Duration::from_secs(30),
            retry_initial_backoff: Duration::from_millis(1),
            provider_max_attempts: 3,
            mock_dimensions: dimensions,
            provider_mode: "mock".to_string(),
            dimension_contract: None,
        };
        let runtime = VectorizerRuntime::new(
            config,
            queue.clone(),
            budgets.clone(),
            usage_log.clone(),
            providers,
            cost,
            "worker-1",
        );

        // budgets must be seeded before reservation.
        let budgets_seeded = budgets.clone();
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current()
                .block_on(async { budgets_seeded.seed("tenant-a", budget).await })
        });

        (runtime, queue, budgets, usage_log)
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn process_one_batch_embeds_rows_and_records_usage() {
        let (runtime, queue, budgets, usage_log) = runtime_with_mock(4, 5, 10_000);
        queue
            .enqueue(
                "tenant-a",
                "mock",
                "embed-v1",
                "public.documents",
                "doc-1",
                "alpha beta",
            )
            .await;
        queue
            .enqueue(
                "tenant-a",
                "mock",
                "embed-v1",
                "public.documents",
                "doc-2",
                "gamma delta",
            )
            .await;

        let processed = runtime.process_one_batch().await.expect("process");
        assert_eq!(processed, 2);

        assert_eq!(queue.completed_count(None).await.unwrap(), 2);
        let usage = usage_log.entries().await;
        assert_eq!(usage.len(), 2);
        assert!(usage.iter().all(|entry| entry.tokens > 0));
        let remaining = budgets.snapshot("tenant-a").await.unwrap();
        assert!(remaining < 10_000);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn process_one_batch_marks_rows_failed_on_budget_exceeded() {
        let (runtime, queue, _budgets, usage_log) = runtime_with_mock(4, 5, 1);
        queue
            .enqueue(
                "tenant-a",
                "mock",
                "embed-v1",
                "public.documents",
                "doc-1",
                "this row needs more than one token of budget",
            )
            .await;

        let processed = runtime.process_one_batch().await.expect("process");
        assert_eq!(processed, 1);

        assert_eq!(queue.completed_count(None).await.unwrap(), 0);
        let snapshot = queue.snapshot().await;
        assert_eq!(snapshot[0].1, "Failed");
        assert!(snapshot[0]
            .3
            .as_deref()
            .unwrap_or("")
            .contains("budget exceeded"));
        assert!(usage_log.entries().await.is_empty());
    }

    #[derive(Debug)]
    struct FlakyProvider {
        attempts: Arc<AtomicUsize>,
    }

    #[async_trait::async_trait]
    impl AsyncEmbeddingProvider for FlakyProvider {
        fn name(&self) -> &str {
            "flaky"
        }

        async fn embed(
            &self,
            _model: &str,
            inputs: &[String],
        ) -> Result<EmbeddingResponse, EmbeddingError> {
            let attempt = self.attempts.fetch_add(1, Ordering::SeqCst);
            if attempt < 2 {
                return Err(EmbeddingError::Transport(
                    "temporary network failure".into(),
                ));
            }
            Ok(EmbeddingResponse {
                embeddings: inputs.iter().map(|_| vec![0.1, 0.2]).collect(),
                prompt_tokens: inputs.len() as u64,
                total_tokens: inputs.len() as u64,
            })
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn process_one_batch_retries_retryable_provider_errors() {
        let queue = Arc::new(InMemoryQueueStore::new());
        let budgets = Arc::new(InMemoryBudgetStore::new());
        budgets.seed("tenant-a", 1_000).await;
        let usage_log = Arc::new(InMemoryUsageLog::new());
        let attempts = Arc::new(AtomicUsize::new(0));
        let mut registry = ProviderRegistry::new();
        registry.insert(Arc::new(FlakyProvider {
            attempts: attempts.clone(),
        }));
        let runtime = VectorizerRuntime::new(
            RuntimeConfig {
                database_url: "x".into(),
                queue_table: "ai.vectorizer_queue".into(),
                budget_table: "ai.tenant_budget".into(),
                usage_log_table: "ai.usage_log".into(),
                listen_addr: "127.0.0.1:0".into(),
                batch_size: 4,
                poll_interval: Duration::from_millis(1),
                visibility_timeout: Duration::from_secs(30),
                retry_initial_backoff: Duration::from_millis(1),
                provider_max_attempts: 3,
                mock_dimensions: 4,
                provider_mode: "mock".into(),
                dimension_contract: None,
            },
            queue.clone(),
            budgets,
            usage_log.clone(),
            Arc::new(registry),
            Arc::new(StaticCostTable::new(1).with("flaky", 1)),
            "worker-1",
        );
        queue
            .enqueue("tenant-a", "flaky", "embed-v1", "public.docs", "1", "hello")
            .await;

        let processed = runtime.process_one_batch().await.expect("process");
        assert_eq!(processed, 1);
        assert_eq!(attempts.load(Ordering::SeqCst), 3);
        assert_eq!(queue.completed_count(None).await.unwrap(), 1);
        assert_eq!(usage_log.entries().await.len(), 1);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn process_one_batch_handles_missing_provider() {
        let queue = Arc::new(InMemoryQueueStore::new());
        let budgets = Arc::new(InMemoryBudgetStore::new());
        budgets.seed("tenant-a", 1_000).await;
        let usage_log = Arc::new(InMemoryUsageLog::new());
        let providers = Arc::new(ProviderRegistry::new());
        let cost = Arc::new(StaticCostTable::new(1));
        let runtime = VectorizerRuntime::new(
            RuntimeConfig {
                database_url: "x".into(),
                queue_table: "ai.vectorizer_queue".into(),
                budget_table: "ai.tenant_budget".into(),
                usage_log_table: "ai.usage_log".into(),
                listen_addr: "127.0.0.1:0".into(),
                batch_size: 4,
                poll_interval: Duration::from_millis(1),
                visibility_timeout: Duration::from_secs(30),
                retry_initial_backoff: Duration::from_millis(1),
                provider_max_attempts: 3,
                mock_dimensions: 4,
                provider_mode: "mock".into(),
                dimension_contract: None,
            },
            queue.clone(),
            budgets,
            usage_log,
            providers,
            cost,
            "worker-1",
        );

        queue
            .enqueue(
                "tenant-a",
                "missing",
                "ignored",
                "public.docs",
                "doc-1",
                "text",
            )
            .await;

        let processed = runtime.process_one_batch().await.expect("process");
        assert_eq!(processed, 1);
        let snapshot = queue.snapshot().await;
        assert_eq!(snapshot[0].1, "Failed");
        assert!(snapshot[0]
            .3
            .as_deref()
            .unwrap_or("")
            .contains("not registered"));
    }

    #[test]
    fn group_by_provider_model_groups_correctly() {
        let rows = vec![
            QueueRow {
                id: 1,
                tenant_id: "t".into(),
                provider: "openai".into(),
                model: "m1".into(),
                source_table: "tbl".into(),
                source_pk: "1".into(),
                source_text: "x".into(),
            },
            QueueRow {
                id: 2,
                tenant_id: "t".into(),
                provider: "openai".into(),
                model: "m1".into(),
                source_table: "tbl".into(),
                source_pk: "2".into(),
                source_text: "y".into(),
            },
            QueueRow {
                id: 3,
                tenant_id: "t".into(),
                provider: "voyage".into(),
                model: "m2".into(),
                source_table: "tbl".into(),
                source_pk: "3".into(),
                source_text: "z".into(),
            },
        ];

        let groups = group_by_provider_model(rows);
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].rows.len(), 2);
        assert_eq!(groups[0].provider, "openai");
        assert_eq!(groups[1].rows.len(), 1);
        assert_eq!(groups[1].provider, "voyage");
    }

    #[test]
    fn allocates_authoritative_provider_tokens_across_rows() {
        assert_eq!(allocate_tokens(&[2, 6], 16), vec![4, 12]);
        assert_eq!(allocate_tokens(&[0, 0], 3), vec![2, 1]);
        assert_eq!(allocate_tokens(&[10, 10], 1), vec![1, 1]);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn process_one_batch_fails_route_mismatch_before_budget() {
        let queue = Arc::new(InMemoryQueueStore::new());
        let budgets = Arc::new(InMemoryBudgetStore::new());
        budgets.seed("tenant-a", 1_000).await;
        let usage_log = Arc::new(InMemoryUsageLog::new());
        let mut registry = ProviderRegistry::new();
        registry.insert(Arc::new(MockProvider::new("mock", 8, 1)));
        let runtime = VectorizerRuntime::new(
            RuntimeConfig {
                database_url: "x".into(),
                queue_table: "ai.vectorizer_queue".into(),
                budget_table: "ai.tenant_budget".into(),
                usage_log_table: "ai.usage_log".into(),
                listen_addr: "127.0.0.1:0".into(),
                batch_size: 4,
                poll_interval: Duration::from_millis(1),
                visibility_timeout: Duration::from_secs(30),
                retry_initial_backoff: Duration::from_millis(1),
                provider_max_attempts: 3,
                mock_dimensions: 8,
                provider_mode: "mock".into(),
                dimension_contract: Some(VectorizerRuntimeContract::new("mock", "embed-v1", 8)),
            },
            queue.clone(),
            budgets.clone(),
            usage_log.clone(),
            Arc::new(registry),
            Arc::new(StaticCostTable::new(1).with("mock", 1)),
            "worker-1",
        );
        queue
            .enqueue(
                "tenant-a",
                "mock",
                "embed-v2",
                "public.docs",
                "doc-1",
                "hello",
            )
            .await;

        let processed = runtime.process_one_batch().await.expect("process");

        assert_eq!(processed, 1);
        assert_eq!(budgets.snapshot("tenant-a").await.unwrap(), 1_000);
        assert!(usage_log.entries().await.is_empty());
        let snapshot = queue.snapshot().await;
        assert_eq!(snapshot[0].1, "Failed");
        assert!(snapshot[0]
            .3
            .as_deref()
            .unwrap_or("")
            .contains("vectorizer contract mismatch"));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn process_one_batch_fails_embedding_dimension_mismatch_and_refunds() {
        let queue = Arc::new(InMemoryQueueStore::new());
        let budgets = Arc::new(InMemoryBudgetStore::new());
        budgets.seed("tenant-a", 1_000).await;
        let usage_log = Arc::new(InMemoryUsageLog::new());
        let mut registry = ProviderRegistry::new();
        registry.insert(Arc::new(MockProvider::new("mock", 7, 1)));
        let runtime = VectorizerRuntime::new(
            RuntimeConfig {
                database_url: "x".into(),
                queue_table: "ai.vectorizer_queue".into(),
                budget_table: "ai.tenant_budget".into(),
                usage_log_table: "ai.usage_log".into(),
                listen_addr: "127.0.0.1:0".into(),
                batch_size: 4,
                poll_interval: Duration::from_millis(1),
                visibility_timeout: Duration::from_secs(30),
                retry_initial_backoff: Duration::from_millis(1),
                provider_max_attempts: 3,
                mock_dimensions: 7,
                provider_mode: "mixed".into(),
                dimension_contract: Some(VectorizerRuntimeContract::new("mock", "embed-v1", 8)),
            },
            queue.clone(),
            budgets.clone(),
            usage_log.clone(),
            Arc::new(registry),
            Arc::new(StaticCostTable::new(1).with("mock", 1)),
            "worker-1",
        );
        queue
            .enqueue(
                "tenant-a",
                "mock",
                "embed-v1",
                "public.docs",
                "doc-1",
                "hello world",
            )
            .await;

        let processed = runtime.process_one_batch().await.expect("process");

        assert_eq!(processed, 1);
        assert_eq!(budgets.snapshot("tenant-a").await.unwrap(), 1_000);
        assert!(usage_log.entries().await.is_empty());
        let snapshot = queue.snapshot().await;
        assert_eq!(snapshot[0].1, "Failed");
        assert!(snapshot[0]
            .3
            .as_deref()
            .unwrap_or("")
            .contains("embedding dimension mismatch"));
    }

    #[test]
    fn validates_runtime_table_identifiers() {
        assert!(validate_qualified_table_name("T", "ai.usage_log").is_ok());
        assert!(validate_qualified_table_name("T", "ai.usage_log;drop").is_err());
        assert!(validate_qualified_table_name("T", "usage_log").is_err());
    }

    #[test]
    fn runtime_config_rejects_zero_duration_knobs() {
        let mut config = RuntimeConfig {
            database_url: "postgres://localhost/test".to_string(),
            queue_table: "ai.vectorizer_queue".to_string(),
            budget_table: "ai.tenant_budget".to_string(),
            usage_log_table: "ai.usage_log".to_string(),
            listen_addr: "127.0.0.1:0".to_string(),
            batch_size: 8,
            poll_interval: Duration::from_millis(0),
            visibility_timeout: Duration::from_secs(30),
            retry_initial_backoff: Duration::from_millis(1),
            provider_max_attempts: 3,
            mock_dimensions: 4,
            provider_mode: "mock".to_string(),
            dimension_contract: None,
        };
        assert!(matches!(
            config.validate(),
            Err(RuntimeError::InvalidEnv(
                "AI_BLAISE_VECTORIZER_POLL_INTERVAL_MS",
                _
            ))
        ));

        config.poll_interval = Duration::from_millis(1);
        config.visibility_timeout = Duration::from_secs(0);
        assert!(matches!(
            config.validate(),
            Err(RuntimeError::InvalidEnv(
                "AI_BLAISE_VECTORIZER_VISIBILITY_TIMEOUT_SECONDS",
                _
            ))
        ));

        config.visibility_timeout = Duration::from_secs(1);
        config.dimension_contract = Some(VectorizerRuntimeContract::new("mock", "embed-v1", 8));
        assert!(matches!(
            config.validate(),
            Err(RuntimeError::InvalidEnv(
                "AI_BLAISE_VECTORIZER_MOCK_DIMENSIONS",
                _
            ))
        ));
        config.dimension_contract = None;
        config.retry_initial_backoff = Duration::from_millis(0);
        assert!(matches!(
            config.validate(),
            Err(RuntimeError::InvalidEnv(
                "AI_BLAISE_VECTORIZER_RETRY_INITIAL_BACKOFF_MS",
                _
            ))
        ));
    }

    #[test]
    fn static_cost_table_returns_default_for_unknown_provider() {
        let table = StaticCostTable::new(7).with("openai", 13);
        assert_eq!(table.cost_micros_per_token("openai"), 13);
        assert_eq!(table.cost_micros_per_token("missing"), 7);
    }
}
