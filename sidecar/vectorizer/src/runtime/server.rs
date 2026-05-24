//! Axum HTTP server for the vectorizer sidecar.
//!
//! Routes:
//! - `GET /healthz` – liveness probe (always 200 unless we cannot bind state).
//! - `GET /readyz` – readiness probe; 503 while draining.
//! - `GET /drain` – drain state snapshot.
//! - `POST /drain` – mark the sidecar as draining.
//! - `GET /metrics` – Prometheus text exposition.
//! - `POST /vectorize` – manual single-row embed for tests/operators.
//! - `GET /queue/status` – per-tenant queue depth + remaining budget.

// FEATURE: A2
// FEATURE: A5
// FEATURE: A6

use crate::runtime::budget::BudgetError;
use crate::runtime::worker::{RuntimeError, VectorizerRuntime};
use ai_blaise_citus_sidecar_shared::{ComponentState, HealthReport};
use axum::extract::{Query, State};
use axum::http::{HeaderValue, StatusCode};
use axum::response::Response;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::SystemTime;
use tokio::net::TcpListener;
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct AppState {
    pub runtime: Arc<VectorizerRuntime>,
    pub started_at: SystemTime,
    pub state: Arc<Mutex<ServerState>>,
}

#[derive(Debug, Clone)]
pub struct ServerState {
    pub accepting_new_work: bool,
    pub in_flight_work: u64,
}

impl ServerState {
    fn ready() -> Self {
        Self {
            accepting_new_work: true,
            in_flight_work: 0,
        }
    }
}

impl AppState {
    pub fn new(runtime: Arc<VectorizerRuntime>) -> Self {
        Self {
            runtime,
            started_at: SystemTime::now(),
            state: Arc::new(Mutex::new(ServerState::ready())),
        }
    }
}

pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/healthz", get(healthz))
        .route("/readyz", get(readyz))
        .route("/drain", get(drain_status).post(begin_drain))
        .route("/metrics", get(metrics))
        .route("/vectorize", post(vectorize))
        .route("/queue/status", get(queue_status))
        .with_state(state)
}

/// Bind to `listen_addr` and serve until the runtime's shutdown handle fires.
pub async fn serve_http(state: AppState, listen_addr: &str) -> Result<(), RuntimeError> {
    let addr: SocketAddr = listen_addr
        .parse()
        .map_err(|error: std::net::AddrParseError| RuntimeError::Server(error.to_string()))?;
    let listener = TcpListener::bind(addr)
        .await
        .map_err(|error| RuntimeError::Server(error.to_string()))?;
    let router = build_router(state.clone());
    let shutdown = state.runtime.shutdown_handle();
    axum::serve(listener, router.into_make_service())
        .with_graceful_shutdown(async move {
            shutdown.notified().await;
        })
        .await
        .map_err(|error| RuntimeError::Server(error.to_string()))
}

async fn healthz(State(state): State<AppState>) -> Response {
    let report = make_health_report(&state).await;
    let body = serde_json::to_string(&HealthBody::from_report(&report, &state).await).unwrap();
    response_json(StatusCode::OK, body)
}

async fn readyz(State(state): State<AppState>) -> Response {
    let server_state = state.state.lock().await.clone();
    let report = make_health_report(&state).await;
    let status = if server_state.accepting_new_work {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };
    let body = serde_json::to_string(&HealthBody::from_report(&report, &state).await).unwrap();
    response_json(status, body)
}

async fn drain_status(State(state): State<AppState>) -> Response {
    let snapshot = state.state.lock().await.clone();
    let body = serde_json::to_string(&DrainBody::from(&snapshot)).unwrap();
    response_json(StatusCode::OK, body)
}

async fn begin_drain(State(state): State<AppState>) -> Response {
    let snapshot = {
        let mut guard = state.state.lock().await;
        guard.accepting_new_work = false;
        guard.clone()
    };
    state.runtime.trigger_shutdown();
    let body = serde_json::to_string(&DrainBody::from(&snapshot)).unwrap();
    response_json(StatusCode::ACCEPTED, body)
}

async fn metrics(State(state): State<AppState>) -> Response {
    let metrics_snapshot = state.runtime.metrics_snapshot().await;
    let server_state = state.state.lock().await.clone();
    let body = format!(
        "# HELP ai_blaise_sidecar_ready Whether the sidecar is ready for new work.\n\
         # TYPE ai_blaise_sidecar_ready gauge\n\
         ai_blaise_sidecar_ready{{component=\"vectorizer\"}} {ready}\n\
         # HELP ai_blaise_sidecar_accepting_new_work Whether the sidecar accepts new work.\n\
         # TYPE ai_blaise_sidecar_accepting_new_work gauge\n\
         ai_blaise_sidecar_accepting_new_work{{component=\"vectorizer\"}} {accepting}\n\
         # HELP ai_blaise_sidecar_in_flight_work In-flight work tracked by the sidecar runtime.\n\
         # TYPE ai_blaise_sidecar_in_flight_work gauge\n\
         ai_blaise_sidecar_in_flight_work{{component=\"vectorizer\"}} {in_flight}\n\
         # HELP ai_blaise_vectorizer_batches_processed_total Number of batches processed.\n\
         # TYPE ai_blaise_vectorizer_batches_processed_total counter\n\
         ai_blaise_vectorizer_batches_processed_total {batches}\n\
         # HELP ai_blaise_vectorizer_rows_embedded_total Number of queue rows embedded.\n\
         # TYPE ai_blaise_vectorizer_rows_embedded_total counter\n\
         ai_blaise_vectorizer_rows_embedded_total {rows_embedded}\n\
         # HELP ai_blaise_vectorizer_rows_failed_total Number of queue rows that failed.\n\
         # TYPE ai_blaise_vectorizer_rows_failed_total counter\n\
         ai_blaise_vectorizer_rows_failed_total {rows_failed}\n",
        ready = u8::from(server_state.accepting_new_work),
        accepting = u8::from(server_state.accepting_new_work),
        in_flight = server_state.in_flight_work,
        batches = metrics_snapshot.batches_processed,
        rows_embedded = metrics_snapshot.rows_embedded,
        rows_failed = metrics_snapshot.rows_failed,
    );

    let mut response = Response::new(body.into());
    *response.status_mut() = StatusCode::OK;
    response.headers_mut().insert(
        "content-type",
        HeaderValue::from_static("text/plain; version=0.0.4"),
    );
    response
}

#[derive(Debug, Deserialize)]
pub struct VectorizeRequest {
    pub tenant_id: String,
    pub provider: String,
    pub model: String,
    pub source_table: String,
    pub source_pk: String,
    pub source_text: String,
}

impl VectorizeRequest {
    fn validate(&self) -> Result<(), &'static str> {
        validate_required("tenant_id", &self.tenant_id)?;
        validate_required("provider", &self.provider)?;
        validate_required("model", &self.model)?;
        validate_required("source_table", &self.source_table)?;
        validate_required("source_pk", &self.source_pk)?;
        validate_required("source_text", &self.source_text)?;
        if !is_safe_provider_name(&self.provider) {
            return Err("provider must contain only ASCII letters, digits, '_', or '-'");
        }
        if !is_qualified_table_name(&self.source_table) {
            return Err("source_table must be schema.table with unquoted SQL identifiers");
        }
        Ok(())
    }
}

#[derive(Debug, Serialize)]
pub struct VectorizeResponse {
    pub tenant_id: String,
    pub provider: String,
    pub model: String,
    pub source_pk: String,
    pub embedding: Vec<f32>,
    pub tokens: u64,
    pub cost_micros: u64,
}

async fn vectorize(
    State(state): State<AppState>,
    Json(request): Json<VectorizeRequest>,
) -> Response {
    if !state.state.lock().await.accepting_new_work {
        return json_error(StatusCode::SERVICE_UNAVAILABLE, "vectorizer is draining");
    }
    if let Err(error) = request.validate() {
        return json_error(StatusCode::BAD_REQUEST, error);
    }

    // Reserve tokens and call the provider directly so test harnesses can
    // bypass the queue path without bootstrapping the schema.
    let provider = match state.runtime.providers().get(&request.provider) {
        Some(provider) => provider,
        None => {
            return json_error(
                StatusCode::BAD_REQUEST,
                format!("unknown provider: {}", request.provider),
            );
        }
    };

    let estimate = (request.source_text.len().div_ceil(4)) as u64;
    if let Err(error) = state
        .runtime
        .budgets()
        .reserve_tokens(&request.tenant_id, estimate)
        .await
    {
        let (status, message) = budget_status(&error);
        return json_error(status, message);
    }

    let response = match state
        .runtime
        .embed_with_retry(
            provider.as_ref(),
            &request.model,
            std::slice::from_ref(&request.source_text),
        )
        .await
    {
        Ok(response) => response,
        Err(error) => {
            let _ = state
                .runtime
                .budgets()
                .refund_tokens(&request.tenant_id, estimate)
                .await;
            return json_error(StatusCode::BAD_GATEWAY, error.to_string());
        }
    };

    let Some(embedding) = response.embeddings.first().cloned() else {
        let _ = state
            .runtime
            .budgets()
            .refund_tokens(&request.tenant_id, estimate)
            .await;
        return json_error(
            StatusCode::BAD_GATEWAY,
            "embedding provider returned no embedding".to_string(),
        );
    };

    let billed_tokens = response.total_tokens.max(estimate).max(1);
    if billed_tokens > estimate {
        if let Err(error) = state
            .runtime
            .budgets()
            .reserve_tokens(&request.tenant_id, billed_tokens - estimate)
            .await
        {
            let _ = state
                .runtime
                .budgets()
                .refund_tokens(&request.tenant_id, estimate)
                .await;
            let (status, message) = budget_status(&error);
            return json_error(status, message);
        }
    } else if billed_tokens < estimate {
        let _ = state
            .runtime
            .budgets()
            .refund_tokens(&request.tenant_id, estimate - billed_tokens)
            .await;
    }

    let cost_per_token = state.runtime.cost_micros_per_token(provider.name());
    let entry = crate::runtime::usage_log::UsageLogEntry {
        tenant_id: request.tenant_id.clone(),
        provider: provider.name().to_string(),
        model: request.model.clone(),
        tokens: billed_tokens,
        cost_micros: billed_tokens.saturating_mul(cost_per_token),
    };
    if let Err(error) = state.runtime.usage_log().record(&entry).await {
        let _ = state
            .runtime
            .budgets()
            .refund_tokens(&request.tenant_id, billed_tokens)
            .await;
        return json_error(StatusCode::INTERNAL_SERVER_ERROR, error.to_string());
    }
    let body = serde_json::to_string(&VectorizeResponse {
        tenant_id: request.tenant_id,
        provider: provider.name().to_string(),
        model: request.model,
        source_pk: request.source_pk,
        embedding,
        tokens: entry.tokens,
        cost_micros: entry.cost_micros,
    })
    .unwrap();
    response_json(StatusCode::OK, body)
}

fn validate_required(field: &'static str, value: &str) -> Result<(), &'static str> {
    if value.trim().is_empty() {
        Err(match field {
            "tenant_id" => "tenant_id must not be empty",
            "provider" => "provider must not be empty",
            "model" => "model must not be empty",
            "source_table" => "source_table must not be empty",
            "source_pk" => "source_pk must not be empty",
            "source_text" => "source_text must not be empty",
            _ => "field must not be empty",
        })
    } else {
        Ok(())
    }
}

fn is_safe_provider_name(value: &str) -> bool {
    value
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
}

fn is_qualified_table_name(value: &str) -> bool {
    let parts: Vec<&str> = value.split('.').collect();
    parts.len() == 2 && parts.iter().all(|part| is_sql_identifier(part))
}

fn is_sql_identifier(value: &str) -> bool {
    let mut chars = value.chars();
    match chars.next() {
        Some(first) if first == '_' || first.is_ascii_alphabetic() => {}
        _ => return false,
    }
    chars.all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
}

fn budget_status(error: &BudgetError) -> (StatusCode, String) {
    match error {
        BudgetError::Exceeded {
            requested,
            remaining,
        } => (
            StatusCode::TOO_MANY_REQUESTS,
            format!("tenant budget exceeded: requested {requested} remaining {remaining}",),
        ),
        BudgetError::NotFound => (
            StatusCode::PAYMENT_REQUIRED,
            "tenant budget not provisioned".to_string(),
        ),
        BudgetError::Storage(detail) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("budget storage error: {detail}"),
        ),
    }
}

#[derive(Debug, Deserialize)]
pub struct QueueStatusQuery {
    pub tenant: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct QueueStatusResponse {
    pub tenant: Option<String>,
    pub pending: u64,
    pub completed: u64,
    pub remaining_tokens: Option<u64>,
}

async fn queue_status(
    State(state): State<AppState>,
    Query(query): Query<QueueStatusQuery>,
) -> Response {
    let queue = state.runtime.queue();
    let pending = queue
        .pending_count(query.tenant.as_deref())
        .await
        .unwrap_or(0);
    let completed = queue
        .completed_count(query.tenant.as_deref())
        .await
        .unwrap_or(0);
    let remaining_tokens = match &query.tenant {
        Some(tenant) => state.runtime.budgets().remaining(tenant).await.ok(),
        None => None,
    };

    let response = QueueStatusResponse {
        tenant: query.tenant,
        pending,
        completed,
        remaining_tokens,
    };
    response_json(StatusCode::OK, serde_json::to_string(&response).unwrap())
}

async fn make_health_report(state: &AppState) -> HealthReport {
    let snapshot = state.state.lock().await.clone();
    if !snapshot.accepting_new_work {
        return HealthReport::draining("vectorizer", state.started_at, "drain requested");
    }
    HealthReport::ready("vectorizer", state.started_at)
}

#[derive(Debug, Serialize)]
struct HealthBody {
    component: String,
    state: String,
    ready: bool,
    accepting_new_work: bool,
    in_flight_work: u64,
    uptime_seconds: u64,
    detail: Option<String>,
    pending_queue_rows: u64,
}

impl HealthBody {
    async fn from_report(report: &HealthReport, state: &AppState) -> Self {
        let snapshot = state.state.lock().await.clone();
        let pending = state.runtime.queue().pending_count(None).await.unwrap_or(0);
        Self {
            component: report.component.clone(),
            state: state_name(report.state).to_string(),
            ready: report.is_ready(),
            accepting_new_work: snapshot.accepting_new_work,
            in_flight_work: snapshot.in_flight_work,
            uptime_seconds: report.uptime().map(|d| d.as_secs()).unwrap_or(0),
            detail: report.detail.clone(),
            pending_queue_rows: pending,
        }
    }
}

#[derive(Debug, Serialize)]
struct DrainBody {
    component: String,
    accepting_new_work: bool,
    in_flight_work: u64,
    drained: bool,
}

impl From<&ServerState> for DrainBody {
    fn from(state: &ServerState) -> Self {
        let drained = !state.accepting_new_work && state.in_flight_work == 0;
        Self {
            component: "vectorizer".to_string(),
            accepting_new_work: state.accepting_new_work,
            in_flight_work: state.in_flight_work,
            drained,
        }
    }
}

fn state_name(state: ComponentState) -> &'static str {
    match state {
        ComponentState::Ready => "ready",
        ComponentState::NotReady => "not_ready",
        ComponentState::Draining => "draining",
    }
}

#[derive(Debug, Serialize)]
struct ErrorBody {
    error: String,
}

fn json_error(status: StatusCode, error: impl Into<String>) -> Response {
    response_json(
        status,
        serde_json::to_string(&ErrorBody {
            error: error.into(),
        })
        .unwrap(),
    )
}

fn response_json(status: StatusCode, body: String) -> Response {
    let mut response = Response::new(body.into());
    *response.status_mut() = status;
    response
        .headers_mut()
        .insert("content-type", HeaderValue::from_static("application/json"));
    response
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::budget::InMemoryBudgetStore;
    use crate::runtime::provider::{MockProvider, ProviderRegistry};
    use crate::runtime::queue::InMemoryQueueStore;
    use crate::runtime::usage_log::{
        InMemoryUsageLog, UsageLogEntry, UsageLogError, UsageLogStore,
    };
    use crate::runtime::worker::{RuntimeConfig, StaticCostTable, VectorizerRuntime};
    use std::time::Duration;

    #[derive(Debug)]
    struct FailingUsageLog;

    #[async_trait::async_trait]
    impl UsageLogStore for FailingUsageLog {
        async fn record(&self, _entry: &UsageLogEntry) -> Result<(), UsageLogError> {
            Err(UsageLogError::Storage(
                "forced usage log failure".to_string(),
            ))
        }

        async fn total_tokens_for_tenant(&self, _tenant_id: &str) -> Result<u64, UsageLogError> {
            Ok(0)
        }
    }

    fn build_state() -> (AppState, Arc<InMemoryBudgetStore>) {
        let queue = Arc::new(InMemoryQueueStore::new());
        let budgets = Arc::new(InMemoryBudgetStore::new());
        let usage_log = Arc::new(InMemoryUsageLog::new());
        let mut registry = ProviderRegistry::new();
        registry.insert(Arc::new(MockProvider::new("mock", 4, 7)));
        let providers = Arc::new(registry);
        let cost = Arc::new(StaticCostTable::new(7).with("mock", 7));
        let config = RuntimeConfig {
            database_url: "postgres://test".into(),
            queue_table: "ai.vectorizer_queue".into(),
            budget_table: "ai.tenant_budget".into(),
            usage_log_table: "ai.usage_log".into(),
            listen_addr: "127.0.0.1:0".into(),
            batch_size: 4,
            poll_interval: Duration::from_millis(5),
            visibility_timeout: Duration::from_secs(30),
            retry_initial_backoff: Duration::from_millis(1),
            provider_max_attempts: 3,
            mock_dimensions: 4,
            provider_mode: "mock".into(),
        };
        let runtime = Arc::new(VectorizerRuntime::new(
            config,
            queue,
            budgets.clone(),
            usage_log,
            providers,
            cost,
            "worker-1",
        ));
        (AppState::new(runtime), budgets)
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn drain_endpoint_transitions_state() {
        let (state, _budgets) = build_state();

        let snapshot_before = state.state.lock().await.clone();
        assert!(snapshot_before.accepting_new_work);

        let response = begin_drain(State(state.clone())).await;
        assert_eq!(response.status(), StatusCode::ACCEPTED);

        let snapshot_after = state.state.lock().await.clone();
        assert!(!snapshot_after.accepting_new_work);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn readyz_returns_503_while_draining() {
        let (state, _budgets) = build_state();
        state.state.lock().await.accepting_new_work = false;
        let response = readyz(State(state)).await;
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn vectorize_endpoint_rejects_invalid_requests_before_budget() {
        let (state, budgets) = build_state();
        budgets.seed("tenant-a", 100).await;

        let response = vectorize(
            State(state.clone()),
            Json(VectorizeRequest {
                tenant_id: "tenant-a".into(),
                provider: "mock".into(),
                model: "embed-v1".into(),
                source_table: "public.documents".into(),
                source_pk: "manual-1".into(),
                source_text: " ".into(),
            }),
        )
        .await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert_eq!(budgets.snapshot("tenant-a").await.unwrap(), 100);

        let response = vectorize(
            State(state.clone()),
            Json(VectorizeRequest {
                tenant_id: "tenant-a".into(),
                provider: "mock;drop".into(),
                model: "embed-v1".into(),
                source_table: "public.documents".into(),
                source_pk: "manual-1".into(),
                source_text: "hello".into(),
            }),
        )
        .await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert_eq!(budgets.snapshot("tenant-a").await.unwrap(), 100);

        let response = vectorize(
            State(state),
            Json(VectorizeRequest {
                tenant_id: "tenant-a".into(),
                provider: "mock".into(),
                model: "embed-v1".into(),
                source_table: "documents".into(),
                source_pk: "manual-1".into(),
                source_text: "hello".into(),
            }),
        )
        .await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert_eq!(budgets.snapshot("tenant-a").await.unwrap(), 100);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn vectorize_endpoint_refunds_budget_when_usage_log_fails() {
        let queue = Arc::new(InMemoryQueueStore::new());
        let budgets = Arc::new(InMemoryBudgetStore::new());
        budgets.seed("tenant-a", 100).await;
        let mut registry = ProviderRegistry::new();
        registry.insert(Arc::new(MockProvider::new("mock", 4, 7)));
        let providers = Arc::new(registry);
        let cost = Arc::new(StaticCostTable::new(7).with("mock", 7));
        let config = RuntimeConfig {
            database_url: "postgres://test".into(),
            queue_table: "ai.vectorizer_queue".into(),
            budget_table: "ai.tenant_budget".into(),
            usage_log_table: "ai.usage_log".into(),
            listen_addr: "127.0.0.1:0".into(),
            batch_size: 4,
            poll_interval: Duration::from_millis(5),
            visibility_timeout: Duration::from_secs(30),
            retry_initial_backoff: Duration::from_millis(1),
            provider_max_attempts: 3,
            mock_dimensions: 4,
            provider_mode: "mock".into(),
        };
        let runtime = Arc::new(VectorizerRuntime::new(
            config,
            queue,
            budgets.clone(),
            Arc::new(FailingUsageLog),
            providers,
            cost,
            "worker-1",
        ));
        let state = AppState::new(runtime);

        let response = vectorize(
            State(state),
            Json(VectorizeRequest {
                tenant_id: "tenant-a".into(),
                provider: "mock".into(),
                model: "embed-v1".into(),
                source_table: "public.documents".into(),
                source_pk: "manual-1".into(),
                source_text: "manual smoke embedding".into(),
            }),
        )
        .await;
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(budgets.snapshot("tenant-a").await.unwrap(), 100);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn vectorize_endpoint_returns_429_on_budget_exceeded() {
        let (state, budgets) = build_state();
        // Do not seed any budget – tenant will be Not Found, mapped to 402.
        let response = vectorize(
            State(state.clone()),
            Json(VectorizeRequest {
                tenant_id: "tenant-z".into(),
                provider: "mock".into(),
                model: "model".into(),
                source_table: "public.tbl".into(),
                source_pk: "1".into(),
                source_text: "hello".into(),
            }),
        )
        .await;
        assert_eq!(response.status(), StatusCode::PAYMENT_REQUIRED);

        // Now seed an under-resourced budget.
        budgets.seed("tenant-a", 1).await;
        let response = vectorize(
            State(state.clone()),
            Json(VectorizeRequest {
                tenant_id: "tenant-a".into(),
                provider: "mock".into(),
                model: "model".into(),
                source_table: "public.tbl".into(),
                source_pk: "1".into(),
                source_text:
                    "this string is intentionally long enough to overflow the seeded budget".into(),
            }),
        )
        .await;
        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
    }
}
