//! Asynchronous vectorizer runtime: real provider clients, queue polling,
//! tenant token budgets, usage logging, and the HTTP serve loop.
//!
//! The deterministic in-process model in `crate` remains the canonical CI
//! evidence, but this module is what the sidecar binary runs in production.

// FEATURE: A2
// FEATURE: A3
// FEATURE: A4
// FEATURE: A5
// FEATURE: A6

pub mod budget;
pub mod provider;
pub mod queue;
pub mod server;
pub mod usage_log;
pub mod worker;

pub use budget::{BudgetError, BudgetStore, PgBudgetStore};
pub use provider::{
    AsyncEmbeddingProvider, CohereProvider, EmbeddingError, MockProvider, OllamaProvider,
    OpenAiProvider, ProviderRegistry, VoyageProvider,
};
pub use queue::{PgQueueStore, QueueError, QueueRow, QueueStore};
pub use server::{serve_http, AppState};
pub use usage_log::{PgUsageLogStore, UsageLogError, UsageLogStore};
pub use worker::{
    ProviderCostTable, RuntimeConfig, RuntimeError, StaticCostTable, VectorizerRuntime,
    WorkerMetrics,
};

use std::env;
use std::time::Duration;

/// Read runtime configuration from environment variables that the Helm chart
/// and CI smoke script populate.
pub fn runtime_config_from_env() -> Result<RuntimeConfig, RuntimeError> {
    let database_url = env::var("AI_BLAISE_VECTORIZER_DATABASE_URL")
        .map_err(|_| RuntimeError::MissingEnv("AI_BLAISE_VECTORIZER_DATABASE_URL"))?;
    let queue_table = env::var("AI_BLAISE_VECTORIZER_QUEUE_TABLE")
        .unwrap_or_else(|_| "ai.vectorizer_queue".to_string());
    let budget_table = env::var("AI_BLAISE_VECTORIZER_BUDGET_TABLE")
        .unwrap_or_else(|_| "ai.tenant_budget".to_string());
    let usage_log_table = env::var("AI_BLAISE_VECTORIZER_USAGE_LOG_TABLE")
        .unwrap_or_else(|_| "ai.usage_log".to_string());
    let listen_addr =
        env::var("AI_BLAISE_LISTEN_ADDR").unwrap_or_else(|_| "0.0.0.0:8080".to_string());

    let batch_size = parse_env_u32("AI_BLAISE_VECTORIZER_BATCH_SIZE", 16)?;
    let poll_interval_ms = parse_env_u64("AI_BLAISE_VECTORIZER_POLL_INTERVAL_MS", 500)?;
    let visibility_timeout_seconds =
        parse_env_u32("AI_BLAISE_VECTORIZER_VISIBILITY_TIMEOUT_SECONDS", 30)?;
    let retry_initial_backoff_ms =
        parse_env_u64("AI_BLAISE_VECTORIZER_RETRY_INITIAL_BACKOFF_MS", 100)?;
    let provider_max_attempts = parse_env_u32("AI_BLAISE_VECTORIZER_PROVIDER_MAX_ATTEMPTS", 3)?;
    let mock_dimensions = parse_env_usize("AI_BLAISE_VECTORIZER_MOCK_DIMENSIONS", 16)?;
    let provider_mode =
        env::var("AI_BLAISE_VECTORIZER_PROVIDER_MODE").unwrap_or_else(|_| "mock".to_string());

    Ok(RuntimeConfig {
        database_url,
        queue_table,
        budget_table,
        usage_log_table,
        listen_addr,
        batch_size,
        poll_interval: Duration::from_millis(poll_interval_ms),
        visibility_timeout: Duration::from_secs(visibility_timeout_seconds as u64),
        retry_initial_backoff: Duration::from_millis(retry_initial_backoff_ms),
        provider_max_attempts,
        mock_dimensions,
        provider_mode,
    })
}

fn parse_env_u32(name: &'static str, default: u32) -> Result<u32, RuntimeError> {
    parse_env(name, default, str::parse::<u32>)
}

fn parse_env_u64(name: &'static str, default: u64) -> Result<u64, RuntimeError> {
    parse_env(name, default, str::parse::<u64>)
}

fn parse_env_usize(name: &'static str, default: usize) -> Result<usize, RuntimeError> {
    parse_env(name, default, str::parse::<usize>)
}

fn parse_env<T, F, E>(name: &'static str, default: T, parser: F) -> Result<T, RuntimeError>
where
    F: Fn(&str) -> Result<T, E>,
    E: std::fmt::Display,
{
    match env::var(name) {
        Ok(value) => parser(&value)
            .map_err(|error| RuntimeError::InvalidEnv(name, format!("{error}: {value}"))),
        Err(env::VarError::NotPresent) => Ok(default),
        Err(error) => Err(RuntimeError::InvalidEnv(name, error.to_string())),
    }
}
