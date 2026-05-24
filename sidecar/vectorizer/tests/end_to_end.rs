//! End-to-end test for the asynchronous vectorizer runtime using only the
//! in-memory stores. The mock provider returns deterministic embeddings and
//! the same code path that the Postgres-backed runtime uses is exercised.

use ai_blaise_citus_sidecar_vectorizer::runtime::budget::InMemoryBudgetStore;
use ai_blaise_citus_sidecar_vectorizer::runtime::queue::{InMemoryQueueStore, QueueStore};
use ai_blaise_citus_sidecar_vectorizer::runtime::usage_log::InMemoryUsageLog;
use ai_blaise_citus_sidecar_vectorizer::runtime::{
    AppState, MockProvider, ProviderRegistry, RuntimeConfig, StaticCostTable, VectorizerRuntime,
};
use std::sync::Arc;
use std::time::Duration;

#[tokio::test(flavor = "multi_thread")]
async fn end_to_end_embeds_inserted_rows_with_mock_provider() {
    let queue = Arc::new(InMemoryQueueStore::new());
    let budgets = Arc::new(InMemoryBudgetStore::new());
    budgets.seed("tenant-a", 100_000).await;
    let usage_log = Arc::new(InMemoryUsageLog::new());

    let mut registry = ProviderRegistry::new();
    registry.insert(Arc::new(MockProvider::new("mock", 4, 5)));
    let providers = Arc::new(registry);
    let cost = Arc::new(StaticCostTable::new(5).with("mock", 5));

    let config = RuntimeConfig {
        database_url: "postgres://test".into(),
        queue_table: "ai.vectorizer_queue".into(),
        budget_table: "ai.tenant_budget".into(),
        usage_log_table: "ai.usage_log".into(),
        listen_addr: "127.0.0.1:0".into(),
        batch_size: 8,
        poll_interval: Duration::from_millis(20),
        visibility_timeout: Duration::from_secs(30),
        retry_initial_backoff: Duration::from_millis(1),
        provider_max_attempts: 3,
        mock_dimensions: 4,
        provider_mode: "mock".into(),
    };

    let runtime = Arc::new(VectorizerRuntime::new(
        config,
        queue.clone(),
        budgets.clone(),
        usage_log.clone(),
        providers,
        cost,
        "worker-1",
    ));

    // Enqueue 16 rows across two tenants — only tenant-a has a budget, so
    // tenant-b should be marked failed.
    for index in 0..12 {
        queue
            .enqueue(
                "tenant-a",
                "mock",
                "embed-v1",
                "public.documents",
                &format!("doc-{index}"),
                &format!("document number {index} with some text"),
            )
            .await;
    }
    for index in 0..4 {
        queue
            .enqueue(
                "tenant-b",
                "mock",
                "embed-v1",
                "public.documents",
                &format!("doc-{index}"),
                &format!("document {index}"),
            )
            .await;
    }

    let runtime_for_loop = runtime.clone();
    let worker = tokio::spawn(async move { runtime_for_loop.run_until_shutdown().await });

    // Wait for the loop to drain the queue (with a hard timeout).
    let queue_for_wait = queue.clone();
    tokio::time::timeout(Duration::from_secs(10), async move {
        loop {
            tokio::time::sleep(Duration::from_millis(50)).await;
            if queue_for_wait.pending_count(None).await.unwrap() == 0 {
                break;
            }
        }
    })
    .await
    .expect("runtime drained queue within 10s");

    runtime.trigger_shutdown();
    worker
        .await
        .expect("worker task joined")
        .expect("worker exited cleanly");

    // tenant-a rows should be succeeded, tenant-b rows should be failed.
    let snapshot = queue.snapshot().await;
    let succeeded: Vec<_> = snapshot
        .iter()
        .filter(|(_, status, _, _)| status == "Succeeded")
        .collect();
    let failed: Vec<_> = snapshot
        .iter()
        .filter(|(_, status, _, _)| status == "Failed")
        .collect();
    assert_eq!(succeeded.len(), 12, "tenant-a rows all succeed");
    assert_eq!(failed.len(), 4, "tenant-b rows all fail without a budget");

    for (_, _, embedding, _) in &succeeded {
        let embedding = embedding.as_ref().expect("succeeded row has embedding");
        assert_eq!(
            embedding.len(),
            4,
            "embedding dimension matches mock config"
        );
    }

    let usage_entries = usage_log.entries().await;
    assert_eq!(
        usage_entries.len(),
        12,
        "every succeeded row writes a usage log entry"
    );
    let total_tokens: u64 = usage_entries.iter().map(|entry| entry.tokens).sum();
    assert!(total_tokens > 0);
    let remaining_a = budgets.snapshot("tenant-a").await.unwrap();
    assert!(
        remaining_a < 100_000,
        "tenant-a budget was decremented (was 100000, now {remaining_a})"
    );

    let metrics = runtime.metrics_snapshot().await;
    assert!(metrics.batches_processed >= 1);
    assert_eq!(metrics.rows_embedded, 12);
    assert_eq!(metrics.rows_failed, 4);
}

#[tokio::test(flavor = "multi_thread")]
async fn end_to_end_rejects_oversized_jobs_with_budget_exceeded() {
    let queue = Arc::new(InMemoryQueueStore::new());
    let budgets = Arc::new(InMemoryBudgetStore::new());
    budgets.seed("tenant-z", 4).await;
    let usage_log = Arc::new(InMemoryUsageLog::new());

    let mut registry = ProviderRegistry::new();
    registry.insert(Arc::new(MockProvider::new("mock", 4, 5)));
    let providers = Arc::new(registry);
    let cost = Arc::new(StaticCostTable::new(5).with("mock", 5));

    let config = RuntimeConfig {
        database_url: "postgres://test".into(),
        queue_table: "ai.vectorizer_queue".into(),
        budget_table: "ai.tenant_budget".into(),
        usage_log_table: "ai.usage_log".into(),
        listen_addr: "127.0.0.1:0".into(),
        batch_size: 4,
        poll_interval: Duration::from_millis(20),
        visibility_timeout: Duration::from_secs(30),
        retry_initial_backoff: Duration::from_millis(1),
        provider_max_attempts: 3,
        mock_dimensions: 4,
        provider_mode: "mock".into(),
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

    queue
        .enqueue(
            "tenant-z",
            "mock",
            "embed-v1",
            "public.documents",
            "doc-1",
            "this string is intentionally longer than four tokens of budget",
        )
        .await;

    let processed = runtime.process_one_batch().await.expect("process batch");
    assert_eq!(processed, 1);

    let snapshot = queue.snapshot().await;
    assert_eq!(
        snapshot[0].1, "Failed",
        "row should fail when budget exceeded"
    );
    assert!(snapshot[0]
        .3
        .as_deref()
        .unwrap_or("")
        .contains("budget exceeded"));
    assert!(
        usage_log.entries().await.is_empty(),
        "no usage logged on rejection"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn app_state_clones_runtime_handle() {
    let queue = Arc::new(InMemoryQueueStore::new());
    let budgets = Arc::new(InMemoryBudgetStore::new());
    budgets.seed("tenant-a", 100).await;
    let usage_log = Arc::new(InMemoryUsageLog::new());

    let mut registry = ProviderRegistry::new();
    registry.insert(Arc::new(MockProvider::new("mock", 2, 1)));
    let providers = Arc::new(registry);
    let cost = Arc::new(StaticCostTable::new(1).with("mock", 1));

    let config = RuntimeConfig {
        database_url: "postgres://test".into(),
        queue_table: "ai.vectorizer_queue".into(),
        budget_table: "ai.tenant_budget".into(),
        usage_log_table: "ai.usage_log".into(),
        listen_addr: "127.0.0.1:0".into(),
        batch_size: 4,
        poll_interval: Duration::from_millis(10),
        visibility_timeout: Duration::from_secs(30),
        retry_initial_backoff: Duration::from_millis(1),
        provider_max_attempts: 3,
        mock_dimensions: 2,
        provider_mode: "mock".into(),
    };

    let runtime = Arc::new(VectorizerRuntime::new(
        config,
        queue.clone(),
        budgets.clone(),
        usage_log.clone(),
        providers,
        cost,
        "worker-1",
    ));

    let state = AppState::new(runtime.clone());
    let cloned = state.clone();
    assert!(Arc::ptr_eq(&state.runtime, &cloned.runtime));
}
