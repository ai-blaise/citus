//! Per-tenant token budget store.
//!
//! Budgets live in `ai.tenant_budget(tenant_id text PRIMARY KEY, remaining_tokens bigint, updated_at timestamptz)`.
//! The vectorizer must decrement budgets atomically with the queue dequeue so a
//! tenant that runs out of tokens cannot race additional jobs through.

// FEATURE: A4

use async_trait::async_trait;
use std::error::Error;
use std::fmt;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_postgres::Client;

#[async_trait]
pub trait BudgetStore: Send + Sync {
    /// Atomically subtract `tokens` from the tenant's remaining budget.
    /// Returns `Ok(remaining)` on success or `Err(BudgetError::Exceeded { remaining })`
    /// when the request would underflow the budget.
    async fn reserve_tokens(&self, tenant_id: &str, tokens: u64) -> Result<u64, BudgetError>;

    /// Re-credit tokens previously reserved (e.g. on provider failure).
    async fn refund_tokens(&self, tenant_id: &str, tokens: u64) -> Result<(), BudgetError>;

    /// Read the current remaining budget for observability/status endpoints.
    async fn remaining(&self, tenant_id: &str) -> Result<u64, BudgetError>;
}

/// Errors emitted by the budget store.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BudgetError {
    Storage(String),
    Exceeded { requested: u64, remaining: u64 },
    NotFound,
}

impl fmt::Display for BudgetError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Storage(detail) => write!(formatter, "budget storage: {detail}"),
            Self::Exceeded {
                requested,
                remaining,
            } => write!(
                formatter,
                "tenant budget exceeded: requested {requested}, remaining {remaining}"
            ),
            Self::NotFound => write!(formatter, "tenant budget not found"),
        }
    }
}

impl Error for BudgetError {}

/// In-memory budget store for tests and the mock smoke flow.
#[derive(Debug, Clone, Default)]
pub struct InMemoryBudgetStore {
    inner: Arc<Mutex<std::collections::HashMap<String, u64>>>,
}

impl InMemoryBudgetStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn seed(&self, tenant_id: &str, tokens: u64) {
        let mut guard = self.inner.lock().await;
        guard.insert(tenant_id.to_string(), tokens);
    }

    pub async fn snapshot(&self, tenant_id: &str) -> Option<u64> {
        let guard = self.inner.lock().await;
        guard.get(tenant_id).copied()
    }
}

#[async_trait]
impl BudgetStore for InMemoryBudgetStore {
    async fn reserve_tokens(&self, tenant_id: &str, tokens: u64) -> Result<u64, BudgetError> {
        let mut guard = self.inner.lock().await;
        let remaining = guard.get(tenant_id).copied().ok_or(BudgetError::NotFound)?;
        if tokens > remaining {
            return Err(BudgetError::Exceeded {
                requested: tokens,
                remaining,
            });
        }
        let updated = remaining - tokens;
        guard.insert(tenant_id.to_string(), updated);
        Ok(updated)
    }

    async fn refund_tokens(&self, tenant_id: &str, tokens: u64) -> Result<(), BudgetError> {
        let mut guard = self.inner.lock().await;
        let remaining = guard.get(tenant_id).copied().unwrap_or(0);
        guard.insert(tenant_id.to_string(), remaining + tokens);
        Ok(())
    }

    async fn remaining(&self, tenant_id: &str) -> Result<u64, BudgetError> {
        let guard = self.inner.lock().await;
        guard.get(tenant_id).copied().ok_or(BudgetError::NotFound)
    }
}

/// Postgres-backed budget store.
#[derive(Clone)]
pub struct PgBudgetStore {
    client: Arc<Client>,
    table: String,
}

impl PgBudgetStore {
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
    pub async fn ensure_schema(&self) -> Result<(), BudgetError> {
        let create_schema = "CREATE SCHEMA IF NOT EXISTS ai".to_string();
        let create_table = format!(
            "CREATE TABLE IF NOT EXISTS {} (\
                tenant_id text PRIMARY KEY,\
                remaining_tokens bigint NOT NULL CHECK (remaining_tokens >= 0),\
                updated_at timestamptz NOT NULL DEFAULT now()\
            )",
            self.table
        );
        self.client
            .batch_execute(&format!("{create_schema}; {create_table};"))
            .await
            .map_err(|error| BudgetError::Storage(format!("{error:?}")))
    }

    pub async fn seed(&self, tenant_id: &str, tokens: u64) -> Result<(), BudgetError> {
        let upsert = format!(
            r#"INSERT INTO {table} (tenant_id, remaining_tokens) VALUES ($1, $2)
               ON CONFLICT (tenant_id) DO UPDATE
               SET remaining_tokens = EXCLUDED.remaining_tokens, updated_at = now()"#,
            table = self.table
        );
        self.client
            .execute(&upsert, &[&tenant_id, &(tokens as i64)])
            .await
            .map_err(|error| BudgetError::Storage(format!("{error:?}")))?;
        Ok(())
    }
}

#[async_trait]
impl BudgetStore for PgBudgetStore {
    async fn reserve_tokens(&self, tenant_id: &str, tokens: u64) -> Result<u64, BudgetError> {
        // Single-statement compare-and-decrement: returns the new value on
        // success and emits zero rows when the request would underflow.
        let update = format!(
            "UPDATE {table} SET remaining_tokens = remaining_tokens - $2, updated_at = now() \
                WHERE tenant_id = $1 AND remaining_tokens >= $2 \
                RETURNING remaining_tokens",
            table = self.table
        );
        let updated = self
            .client
            .query_opt(&update, &[&tenant_id, &(tokens as i64)])
            .await
            .map_err(|error| BudgetError::Storage(format!("{error:?}")))?;

        if let Some(row) = updated {
            let remaining: i64 = row.get(0);
            return Ok(remaining as u64);
        }

        // Determine whether the row exists at all so the caller can decide
        // between "tenant unknown" and "budget exceeded".
        let select = format!(
            "SELECT remaining_tokens FROM {table} WHERE tenant_id = $1",
            table = self.table
        );
        let row = self
            .client
            .query_opt(&select, &[&tenant_id])
            .await
            .map_err(|error| BudgetError::Storage(format!("{error:?}")))?;
        match row {
            Some(row) => {
                let remaining: i64 = row.get(0);
                Err(BudgetError::Exceeded {
                    requested: tokens,
                    remaining: remaining as u64,
                })
            }
            None => Err(BudgetError::NotFound),
        }
    }

    async fn refund_tokens(&self, tenant_id: &str, tokens: u64) -> Result<(), BudgetError> {
        let update = format!(
            "UPDATE {table} SET remaining_tokens = remaining_tokens + $2, updated_at = now() \
                WHERE tenant_id = $1",
            table = self.table
        );
        self.client
            .execute(&update, &[&tenant_id, &(tokens as i64)])
            .await
            .map_err(|error| BudgetError::Storage(format!("{error:?}")))?;
        Ok(())
    }

    async fn remaining(&self, tenant_id: &str) -> Result<u64, BudgetError> {
        let select = format!(
            "SELECT remaining_tokens FROM {table} WHERE tenant_id = $1",
            table = self.table
        );
        let row = self
            .client
            .query_opt(&select, &[&tenant_id])
            .await
            .map_err(|error| BudgetError::Storage(format!("{error:?}")))?;
        match row {
            Some(row) => Ok(row.get::<_, i64>(0) as u64),
            None => Err(BudgetError::NotFound),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn in_memory_budget_reserves_and_refunds() {
        let store = InMemoryBudgetStore::new();
        store.seed("tenant-a", 100).await;

        let remaining = store.reserve_tokens("tenant-a", 40).await.expect("reserve");
        assert_eq!(remaining, 60);

        store.refund_tokens("tenant-a", 10).await.expect("refund");
        assert_eq!(store.remaining("tenant-a").await.unwrap(), 70);
    }

    #[tokio::test]
    async fn in_memory_budget_rejects_overrun() {
        let store = InMemoryBudgetStore::new();
        store.seed("tenant-a", 10).await;
        let error = store
            .reserve_tokens("tenant-a", 25)
            .await
            .expect_err("overrun");
        assert_eq!(
            error,
            BudgetError::Exceeded {
                requested: 25,
                remaining: 10,
            }
        );
    }

    #[tokio::test]
    async fn in_memory_budget_reports_not_found() {
        let store = InMemoryBudgetStore::new();
        assert_eq!(
            store.reserve_tokens("tenant-z", 1).await,
            Err(BudgetError::NotFound)
        );
    }
}
