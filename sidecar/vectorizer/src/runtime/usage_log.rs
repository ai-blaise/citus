//! Tenant-cost accounting writer for `ai.usage_log`.
//!
//! Schema:
//!
//! ```sql
//! CREATE TABLE IF NOT EXISTS ai.usage_log (
//!     tenant_id text NOT NULL,
//!     provider text NOT NULL,
//!     model text NOT NULL,
//!     tokens bigint NOT NULL CHECK (tokens > 0),
//!     cost_micros bigint NOT NULL,
//!     recorded_at timestamptz NOT NULL DEFAULT now()
//! );
//! ```
//!
//! TimescaleDB turns this into a hypertable in production; the smoke
//! script and tests work against a plain table.

// FEATURE: A5

use async_trait::async_trait;
use std::error::Error;
use std::fmt;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_postgres::Client;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UsageLogEntry {
    pub tenant_id: String,
    pub provider: String,
    pub model: String,
    pub tokens: u64,
    pub cost_micros: u64,
}

impl UsageLogEntry {
    pub fn validate(&self) -> Result<(), UsageLogError> {
        if self.tenant_id.trim().is_empty() {
            return Err(UsageLogError::InvalidField("tenant_id"));
        }
        if self.provider.trim().is_empty() {
            return Err(UsageLogError::InvalidField("provider"));
        }
        if self.model.trim().is_empty() {
            return Err(UsageLogError::InvalidField("model"));
        }
        if self.tokens == 0 {
            return Err(UsageLogError::InvalidField("tokens"));
        }
        Ok(())
    }
}

#[async_trait]
pub trait UsageLogStore: Send + Sync {
    async fn record(&self, entry: &UsageLogEntry) -> Result<(), UsageLogError>;
    async fn total_tokens_for_tenant(&self, tenant_id: &str) -> Result<u64, UsageLogError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UsageLogError {
    Storage(String),
    InvalidField(&'static str),
}

impl fmt::Display for UsageLogError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Storage(detail) => write!(formatter, "usage log storage: {detail}"),
            Self::InvalidField(field) => {
                write!(formatter, "usage log entry has invalid field: {field}")
            }
        }
    }
}

impl Error for UsageLogError {}

#[derive(Debug, Clone, Default)]
pub struct InMemoryUsageLog {
    entries: Arc<Mutex<Vec<UsageLogEntry>>>,
}

impl InMemoryUsageLog {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn entries(&self) -> Vec<UsageLogEntry> {
        let guard = self.entries.lock().await;
        guard.clone()
    }
}

#[async_trait]
impl UsageLogStore for InMemoryUsageLog {
    async fn record(&self, entry: &UsageLogEntry) -> Result<(), UsageLogError> {
        entry.validate()?;
        let mut guard = self.entries.lock().await;
        guard.push(entry.clone());
        Ok(())
    }

    async fn total_tokens_for_tenant(&self, tenant_id: &str) -> Result<u64, UsageLogError> {
        let guard = self.entries.lock().await;
        Ok(guard
            .iter()
            .filter(|entry| entry.tenant_id == tenant_id)
            .map(|entry| entry.tokens)
            .sum())
    }
}

/// Postgres-backed usage log writer.
#[derive(Clone)]
pub struct PgUsageLogStore {
    client: Arc<Client>,
    table: String,
}

impl PgUsageLogStore {
    pub fn new(client: Arc<Client>, table: impl Into<String>) -> Self {
        Self {
            client,
            table: table.into(),
        }
    }

    pub fn table(&self) -> &str {
        &self.table
    }

    /// Schema bootstrap used by the smoke script and tests.
    pub async fn ensure_schema(&self) -> Result<(), UsageLogError> {
        let create_schema = "CREATE SCHEMA IF NOT EXISTS ai".to_string();
        let create_table = format!(
            "CREATE TABLE IF NOT EXISTS {} (\
                tenant_id text NOT NULL,\
                provider text NOT NULL,\
                model text NOT NULL,\
                tokens bigint NOT NULL CHECK (tokens > 0),\
                cost_micros bigint NOT NULL,\
                recorded_at timestamptz NOT NULL DEFAULT now()\
            )",
            self.table
        );
        self.client
            .batch_execute(&format!("{create_schema}; {create_table};"))
            .await
            .map_err(|error| UsageLogError::Storage(format!("{error:?}")))
    }
}

#[async_trait]
impl UsageLogStore for PgUsageLogStore {
    async fn record(&self, entry: &UsageLogEntry) -> Result<(), UsageLogError> {
        entry.validate()?;
        let insert = format!(
            "INSERT INTO {table} (tenant_id, provider, model, tokens, cost_micros) \
                VALUES ($1, $2, $3, $4, $5)",
            table = self.table
        );
        self.client
            .execute(
                &insert,
                &[
                    &entry.tenant_id,
                    &entry.provider,
                    &entry.model,
                    &(entry.tokens as i64),
                    &(entry.cost_micros as i64),
                ],
            )
            .await
            .map_err(|error| UsageLogError::Storage(format!("{error:?}")))?;
        Ok(())
    }

    async fn total_tokens_for_tenant(&self, tenant_id: &str) -> Result<u64, UsageLogError> {
        let select = format!(
            "SELECT COALESCE(SUM(tokens), 0)::bigint FROM {table} WHERE tenant_id = $1",
            table = self.table
        );
        let row = self
            .client
            .query_one(&select, &[&tenant_id])
            .await
            .map_err(|error| UsageLogError::Storage(format!("{error:?}")))?;
        let total: i64 = row.get(0);
        Ok(total as u64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn in_memory_usage_log_records_and_totals() {
        let store = InMemoryUsageLog::new();
        store
            .record(&UsageLogEntry {
                tenant_id: "tenant-a".into(),
                provider: "openai".into(),
                model: "text-embedding-3-large".into(),
                tokens: 42,
                cost_micros: 420,
            })
            .await
            .expect("record");
        store
            .record(&UsageLogEntry {
                tenant_id: "tenant-a".into(),
                provider: "openai".into(),
                model: "text-embedding-3-large".into(),
                tokens: 8,
                cost_micros: 80,
            })
            .await
            .expect("record");

        assert_eq!(store.total_tokens_for_tenant("tenant-a").await.unwrap(), 50);
        assert_eq!(store.entries().await.len(), 2);
    }

    #[tokio::test]
    async fn in_memory_usage_log_rejects_invalid_fields() {
        let store = InMemoryUsageLog::new();
        let entry = UsageLogEntry {
            tenant_id: "".into(),
            provider: "openai".into(),
            model: "text-embedding-3-large".into(),
            tokens: 5,
            cost_micros: 10,
        };
        assert_eq!(
            store.record(&entry).await,
            Err(UsageLogError::InvalidField("tenant_id"))
        );
    }
}
