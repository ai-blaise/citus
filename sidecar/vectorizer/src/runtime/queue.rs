//! Shard-local vectorizer queue store backed by tokio-postgres.

// FEATURE: A2
// FEATURE: A6

use async_trait::async_trait;
use std::error::Error;
use std::fmt;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio_postgres::Client;

#[derive(Debug, Clone, PartialEq)]
pub struct QueueRow {
    pub id: i64,
    pub tenant_id: String,
    pub provider: String,
    pub model: String,
    pub source_table: String,
    pub source_pk: String,
    pub source_text: String,
}

#[async_trait]
pub trait QueueStore: Send + Sync {
    async fn lock_batch(
        &self,
        worker_id: &str,
        batch_size: u32,
        visibility_timeout: Duration,
    ) -> Result<Vec<QueueRow>, QueueError>;

    async fn mark_succeeded(&self, ids: &[i64], embeddings: &[Vec<f32>]) -> Result<(), QueueError>;

    async fn mark_failed(&self, id: i64, detail: &str) -> Result<(), QueueError>;

    async fn pending_count(&self, tenant_id: Option<&str>) -> Result<u64, QueueError>;

    async fn completed_count(&self, tenant_id: Option<&str>) -> Result<u64, QueueError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueueError {
    Storage(String),
    BatchLengthMismatch { rows: usize, embeddings: usize },
}

impl fmt::Display for QueueError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Storage(detail) => write!(formatter, "queue storage: {detail}"),
            Self::BatchLengthMismatch { rows, embeddings } => write!(
                formatter,
                "queue mark_succeeded expected {rows} embeddings, got {embeddings}"
            ),
        }
    }
}

impl Error for QueueError {}

/// In-memory queue used by the canonical tests and the smoke harness when no
/// database is available.
#[derive(Debug, Clone, Default)]
pub struct InMemoryQueueStore {
    inner: Arc<Mutex<InMemoryQueueInner>>,
}

#[derive(Debug, Default)]
struct InMemoryQueueInner {
    next_id: i64,
    rows: Vec<InMemoryRow>,
}

#[derive(Debug, Clone, PartialEq)]
struct InMemoryRow {
    id: i64,
    tenant_id: String,
    provider: String,
    model: String,
    source_table: String,
    source_pk: String,
    source_text: String,
    status: RowStatus,
    embedding: Option<Vec<f32>>,
    last_error: Option<String>,
    locked_by: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RowStatus {
    Pending,
    Locked,
    Succeeded,
    Failed,
}

impl InMemoryQueueStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn enqueue(
        &self,
        tenant_id: &str,
        provider: &str,
        model: &str,
        source_table: &str,
        source_pk: &str,
        source_text: &str,
    ) -> i64 {
        let mut guard = self.inner.lock().await;
        guard.next_id += 1;
        let id = guard.next_id;
        guard.rows.push(InMemoryRow {
            id,
            tenant_id: tenant_id.to_string(),
            provider: provider.to_string(),
            model: model.to_string(),
            source_table: source_table.to_string(),
            source_pk: source_pk.to_string(),
            source_text: source_text.to_string(),
            status: RowStatus::Pending,
            embedding: None,
            last_error: None,
            locked_by: None,
        });
        id
    }

    pub async fn snapshot(&self) -> Vec<(i64, String, Option<Vec<f32>>, Option<String>)> {
        let guard = self.inner.lock().await;
        guard
            .rows
            .iter()
            .map(|row| {
                (
                    row.id,
                    format!("{:?}", row.status),
                    row.embedding.clone(),
                    row.last_error.clone(),
                )
            })
            .collect()
    }
}

#[async_trait]
impl QueueStore for InMemoryQueueStore {
    async fn lock_batch(
        &self,
        worker_id: &str,
        batch_size: u32,
        _visibility_timeout: Duration,
    ) -> Result<Vec<QueueRow>, QueueError> {
        let mut guard = self.inner.lock().await;
        let mut out = Vec::new();
        for row in guard.rows.iter_mut() {
            if out.len() as u32 == batch_size {
                break;
            }
            if row.status == RowStatus::Pending {
                row.status = RowStatus::Locked;
                row.locked_by = Some(worker_id.to_string());
                out.push(QueueRow {
                    id: row.id,
                    tenant_id: row.tenant_id.clone(),
                    provider: row.provider.clone(),
                    model: row.model.clone(),
                    source_table: row.source_table.clone(),
                    source_pk: row.source_pk.clone(),
                    source_text: row.source_text.clone(),
                });
            }
        }
        Ok(out)
    }

    async fn mark_succeeded(&self, ids: &[i64], embeddings: &[Vec<f32>]) -> Result<(), QueueError> {
        if ids.len() != embeddings.len() {
            return Err(QueueError::BatchLengthMismatch {
                rows: ids.len(),
                embeddings: embeddings.len(),
            });
        }
        let mut guard = self.inner.lock().await;
        for (id, embedding) in ids.iter().zip(embeddings.iter()) {
            if let Some(row) = guard.rows.iter_mut().find(|row| row.id == *id) {
                row.status = RowStatus::Succeeded;
                row.embedding = Some(embedding.clone());
                row.last_error = None;
            }
        }
        Ok(())
    }

    async fn mark_failed(&self, id: i64, detail: &str) -> Result<(), QueueError> {
        let mut guard = self.inner.lock().await;
        if let Some(row) = guard.rows.iter_mut().find(|row| row.id == id) {
            row.status = RowStatus::Failed;
            row.last_error = Some(detail.to_string());
        }
        Ok(())
    }

    async fn pending_count(&self, tenant_id: Option<&str>) -> Result<u64, QueueError> {
        let guard = self.inner.lock().await;
        Ok(guard
            .rows
            .iter()
            .filter(|row| row.status == RowStatus::Pending)
            .filter(|row| tenant_id.is_none_or(|tenant| row.tenant_id == tenant))
            .count() as u64)
    }

    async fn completed_count(&self, tenant_id: Option<&str>) -> Result<u64, QueueError> {
        let guard = self.inner.lock().await;
        Ok(guard
            .rows
            .iter()
            .filter(|row| row.status == RowStatus::Succeeded)
            .filter(|row| tenant_id.is_none_or(|tenant| row.tenant_id == tenant))
            .count() as u64)
    }
}

/// Postgres-backed queue store.
#[derive(Clone)]
pub struct PgQueueStore {
    client: Arc<Client>,
    table: String,
}

impl PgQueueStore {
    pub fn new(client: Arc<Client>, table: impl Into<String>) -> Self {
        Self {
            client,
            table: table.into(),
        }
    }

    pub fn table(&self) -> &str {
        &self.table
    }

    pub fn client(&self) -> Arc<Client> {
        self.client.clone()
    }

    /// Schema bootstrap used by the smoke script and tests.
    pub async fn ensure_schema(&self) -> Result<(), QueueError> {
        let create_schema = "CREATE SCHEMA IF NOT EXISTS ai".to_string();
        let create_table = format!(
            "CREATE TABLE IF NOT EXISTS {table} (\
                id bigserial PRIMARY KEY,\
                tenant_id text NOT NULL,\
                provider text NOT NULL,\
                model text NOT NULL,\
                source_table text NOT NULL,\
                source_pk text NOT NULL,\
                source_text text NOT NULL,\
                status text NOT NULL DEFAULT 'pending',\
                embedding double precision[],\
                attempts integer NOT NULL DEFAULT 0,\
                last_error text,\
                locked_at timestamptz,\
                locked_by text,\
                enqueued_at timestamptz NOT NULL DEFAULT now()\
            )",
            table = self.table
        );
        self.client
            .batch_execute(&format!("{create_schema}; {create_table};"))
            .await
            .map_err(|error| QueueError::Storage(format!("{error:?}")))
    }

    pub async fn enqueue(
        &self,
        tenant_id: &str,
        provider: &str,
        model: &str,
        source_table: &str,
        source_pk: &str,
        source_text: &str,
    ) -> Result<i64, QueueError> {
        let insert = format!(
            "INSERT INTO {table} (tenant_id, provider, model, source_table, source_pk, source_text) \
                VALUES ($1, $2, $3, $4, $5, $6) RETURNING id",
            table = self.table
        );
        let row = self
            .client
            .query_one(
                &insert,
                &[
                    &tenant_id,
                    &provider,
                    &model,
                    &source_table,
                    &source_pk,
                    &source_text,
                ],
            )
            .await
            .map_err(|error| QueueError::Storage(format!("{error:?}")))?;
        Ok(row.get::<_, i64>(0))
    }
}

#[async_trait]
impl QueueStore for PgQueueStore {
    async fn lock_batch(
        &self,
        worker_id: &str,
        batch_size: u32,
        visibility_timeout: Duration,
    ) -> Result<Vec<QueueRow>, QueueError> {
        let visibility_seconds = visibility_timeout.as_secs_f64().max(1.0);
        let sql = format!(
            r#"WITH locked AS (
                SELECT id FROM {table}
                 WHERE status = 'pending'
                    OR (status = 'in_flight' AND locked_at < now() - make_interval(secs => $3))
                 ORDER BY id
                 LIMIT $1
                 FOR UPDATE SKIP LOCKED
             )
             UPDATE {table} AS q
                SET status = 'in_flight',
                    attempts = q.attempts + 1,
                    locked_at = now(),
                    locked_by = $2
              FROM locked
              WHERE q.id = locked.id
              RETURNING q.id, q.tenant_id, q.provider, q.model, q.source_table, q.source_pk, q.source_text"#,
            table = self.table
        );
        let rows = self
            .client
            .query(
                &sql,
                &[&(batch_size as i64), &worker_id, &visibility_seconds],
            )
            .await
            .map_err(|error| QueueError::Storage(format!("{error:?}")))?;
        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            out.push(QueueRow {
                id: row.get(0),
                tenant_id: row.get(1),
                provider: row.get(2),
                model: row.get(3),
                source_table: row.get(4),
                source_pk: row.get(5),
                source_text: row.get(6),
            });
        }
        Ok(out)
    }

    async fn mark_succeeded(&self, ids: &[i64], embeddings: &[Vec<f32>]) -> Result<(), QueueError> {
        if ids.len() != embeddings.len() {
            return Err(QueueError::BatchLengthMismatch {
                rows: ids.len(),
                embeddings: embeddings.len(),
            });
        }
        let update = format!(
            "UPDATE {table} SET status = 'succeeded', embedding = $2, last_error = NULL, locked_at = NULL, locked_by = NULL WHERE id = $1",
            table = self.table
        );
        for (id, embedding) in ids.iter().zip(embeddings.iter()) {
            let as_f64: Vec<f64> = embedding
                .iter()
                .map(|component| *component as f64)
                .collect();
            self.client
                .execute(&update, &[id, &as_f64])
                .await
                .map_err(|error| QueueError::Storage(format!("{error:?}")))?;
        }
        Ok(())
    }

    async fn mark_failed(&self, id: i64, detail: &str) -> Result<(), QueueError> {
        let update = format!(
            "UPDATE {table} SET status = 'failed', last_error = $2, locked_at = NULL, locked_by = NULL WHERE id = $1",
            table = self.table
        );
        self.client
            .execute(&update, &[&id, &detail])
            .await
            .map_err(|error| QueueError::Storage(format!("{error:?}")))?;
        Ok(())
    }

    async fn pending_count(&self, tenant_id: Option<&str>) -> Result<u64, QueueError> {
        let row = match tenant_id {
            Some(tenant) => {
                let sql = format!(
                    "SELECT count(*)::bigint FROM {table} WHERE status = 'pending' AND tenant_id = $1",
                    table = self.table
                );
                self.client
                    .query_one(&sql, &[&tenant])
                    .await
                    .map_err(|error| QueueError::Storage(format!("{error:?}")))?
            }
            None => {
                let sql = format!(
                    "SELECT count(*)::bigint FROM {table} WHERE status = 'pending'",
                    table = self.table
                );
                self.client
                    .query_one(&sql, &[])
                    .await
                    .map_err(|error| QueueError::Storage(format!("{error:?}")))?
            }
        };
        let total: i64 = row.get(0);
        Ok(total as u64)
    }

    async fn completed_count(&self, tenant_id: Option<&str>) -> Result<u64, QueueError> {
        let row = match tenant_id {
            Some(tenant) => {
                let sql = format!(
                    "SELECT count(*)::bigint FROM {table} WHERE status = 'succeeded' AND tenant_id = $1",
                    table = self.table
                );
                self.client
                    .query_one(&sql, &[&tenant])
                    .await
                    .map_err(|error| QueueError::Storage(format!("{error:?}")))?
            }
            None => {
                let sql = format!(
                    "SELECT count(*)::bigint FROM {table} WHERE status = 'succeeded'",
                    table = self.table
                );
                self.client
                    .query_one(&sql, &[])
                    .await
                    .map_err(|error| QueueError::Storage(format!("{error:?}")))?
            }
        };
        let total: i64 = row.get(0);
        Ok(total as u64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn in_memory_queue_locks_and_completes() {
        let queue = InMemoryQueueStore::new();
        queue
            .enqueue(
                "tenant-a",
                "mock",
                "embed-v1",
                "public.documents",
                "doc-1",
                "hello",
            )
            .await;
        queue
            .enqueue(
                "tenant-a",
                "mock",
                "embed-v1",
                "public.documents",
                "doc-2",
                "world",
            )
            .await;

        assert_eq!(queue.pending_count(None).await.unwrap(), 2);
        let batch = queue
            .lock_batch("worker-1", 8, Duration::from_secs(30))
            .await
            .unwrap();
        assert_eq!(batch.len(), 2);
        assert_eq!(queue.pending_count(None).await.unwrap(), 0);

        let ids: Vec<i64> = batch.iter().map(|row| row.id).collect();
        let embeddings = vec![vec![0.1f32, 0.2], vec![0.3, 0.4]];
        queue.mark_succeeded(&ids, &embeddings).await.unwrap();
        assert_eq!(queue.completed_count(None).await.unwrap(), 2);
    }

    #[tokio::test]
    async fn in_memory_queue_marks_failed_rows() {
        let queue = InMemoryQueueStore::new();
        let id = queue
            .enqueue("tenant-a", "mock", "embed-v1", "t", "1", "x")
            .await;
        let _ = queue
            .lock_batch("worker-1", 8, Duration::from_secs(30))
            .await
            .unwrap();
        queue.mark_failed(id, "boom").await.unwrap();
        assert_eq!(queue.completed_count(None).await.unwrap(), 0);
        let snapshot = queue.snapshot().await;
        assert_eq!(snapshot[0].1, "Failed");
        assert_eq!(snapshot[0].3.as_deref(), Some("boom"));
    }

    #[tokio::test]
    async fn mark_succeeded_validates_lengths() {
        let queue = InMemoryQueueStore::new();
        let err = queue
            .mark_succeeded(&[1, 2], &[vec![0.0]])
            .await
            .expect_err("mismatch");
        assert_eq!(
            err,
            QueueError::BatchLengthMismatch {
                rows: 2,
                embeddings: 1,
            }
        );
    }
}
