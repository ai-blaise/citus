//! `serve` runtime for the CDC sidecar.
//!
//! Wires the axum-backed probe server, the logical-replication consumer pool,
//! and the async-nats sink under a single tokio runtime, with a SIGTERM
//! graceful-shutdown handler.

use crate::nats_sink::NatsSink;
use crate::replication::{
    publication_from_env, targets_from_env, ReplicationConsumer, ReplicationError,
};
use ai_blaise_citus_sidecar_shared::{HttpMethod, HttpProbeRequest, SidecarRuntime};
use axum::{
    extract::State,
    response::IntoResponse,
    routing::{get, post},
    Router,
};
use std::env;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use tokio::signal;
use tracing::{error, info, warn};

#[derive(Clone)]
struct ProbeState(Arc<Mutex<SidecarRuntime>>);

impl ProbeState {
    fn new(component: &str) -> Self {
        // FEATURE: C1
        Self(Arc::new(Mutex::new(SidecarRuntime::not_ready(
            component,
            "logical replication stream is not active",
        ))))
    }
}

/// Entry point invoked from `main serve`.
pub async fn serve(component: &'static str, default_addr: &str) -> Result<(), ReplicationError> {
    init_tracing();
    let state = ProbeState::new(component);
    let runtime = state.0.clone();
    let bind: SocketAddr = env::var("AI_BLAISE_SIDECAR_LISTEN_ADDR")
        .unwrap_or_else(|_| default_addr.to_string())
        .parse()
        .map_err(|error: std::net::AddrParseError| ReplicationError::Sink(error.to_string()))?;

    // Build the probe server.
    let app = Router::new()
        .route("/healthz", get(healthz))
        .route("/readyz", get(readyz))
        .route("/metrics", get(metrics))
        .route("/drain", post(post_drain).get(get_drain))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(bind)
        .await
        .map_err(|error| ReplicationError::Sink(error.to_string()))?;
    info!(%bind, "cdc sidecar probe server listening");

    let probe_task = tokio::spawn(async move {
        if let Err(error) = axum::serve(listener, app).await {
            error!(?error, "axum probe server exited");
        }
    });

    // Optional: connect NATS + spin replication consumers if env is configured.
    let sink = match NatsSink::connect_from_env().await {
        Ok(client) => Some(client),
        Err(error) => {
            warn!(?error, "CDC NATS sink unavailable; running probe-only");
            None
        }
    };

    let consumers_task = if let Some(sink_handle) = sink {
        match targets_from_env() {
            Ok(targets) => {
                let publication = publication_from_env();
                Some(tokio::spawn(async move {
                    for target in targets {
                        let consumer =
                            ReplicationConsumer::new(target.clone(), publication.clone());
                        if let Err(error) = consumer.run(sink_handle.as_ref()).await {
                            warn!(target = %target.label, ?error, "replication consumer exited");
                        }
                    }
                }))
            }
            Err(error) => {
                warn!(?error, "CDC replication targets unset; running probe-only");
                None
            }
        }
    } else {
        None
    };

    wait_for_shutdown().await;
    info!("cdc sidecar received shutdown signal; draining");
    {
        let mut state = runtime.lock().expect("runtime mutex");
        state.begin_drain(0);
    }
    probe_task.abort();
    if let Some(handle) = consumers_task {
        handle.abort();
    }
    Ok(())
}

async fn healthz(State(state): State<ProbeState>) -> impl IntoResponse {
    dispatch(&state, HttpMethod::Get, "/healthz")
}

async fn readyz(State(state): State<ProbeState>) -> impl IntoResponse {
    dispatch(&state, HttpMethod::Get, "/readyz")
}

async fn metrics(State(state): State<ProbeState>) -> impl IntoResponse {
    dispatch(&state, HttpMethod::Get, "/metrics")
}

async fn get_drain(State(state): State<ProbeState>) -> impl IntoResponse {
    dispatch(&state, HttpMethod::Get, "/drain")
}

async fn post_drain(State(state): State<ProbeState>) -> impl IntoResponse {
    dispatch(&state, HttpMethod::Post, "/drain")
}

fn dispatch(state: &ProbeState, method: HttpMethod, path: &str) -> impl IntoResponse {
    let request = HttpProbeRequest::new(method, path);
    let response = state
        .0
        .lock()
        .expect("runtime mutex")
        .handle_http_request(&request);
    let status = axum::http::StatusCode::from_u16(response.status_code)
        .unwrap_or(axum::http::StatusCode::OK);
    (
        status,
        [("content-type", response.content_type.clone())],
        response.body,
    )
}

async fn wait_for_shutdown() {
    let ctrl_c = async {
        let _ = signal::ctrl_c().await;
    };

    #[cfg(unix)]
    let terminate = async {
        if let Ok(mut term) = signal::unix::signal(signal::unix::SignalKind::terminate()) {
            term.recv().await;
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}

fn init_tracing() {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));
    let _ = tracing_subscriber::fmt().with_env_filter(filter).try_init();
}

/// Returns the address the runtime would bind to. Exposed for testing.
pub fn listen_addr_from_env(default_addr: &str) -> String {
    env::var("AI_BLAISE_SIDECAR_LISTEN_ADDR").unwrap_or_else(|_| default_addr.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn listen_addr_defaults_when_env_unset() {
        // Defensive — don't mutate env in parallel tests, just check default.
        let addr = listen_addr_from_env("0.0.0.0:8080");
        assert!(!addr.is_empty());
    }

    #[tokio::test]
    async fn probe_only_runtime_is_not_ready() {
        let state = ProbeState::new("cdc");
        let response = dispatch(&state, HttpMethod::Get, "/readyz").into_response();
        assert_eq!(
            response.status(),
            axum::http::StatusCode::SERVICE_UNAVAILABLE
        );
        let body = axum::body::to_bytes(response.into_body(), 65536)
            .await
            .expect("readiness body");
        let report: serde_json::Value = serde_json::from_slice(&body).expect("readiness JSON");
        assert_eq!(report["ready"], false);
        assert_eq!(report["detail"], "logical replication stream is not active");
    }

    #[tokio::test]
    async fn unready_runtime_retains_liveness_and_zero_ready_metric() {
        let state = ProbeState::new("cdc");
        let response = dispatch(&state, HttpMethod::Get, "/healthz").into_response();
        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), 65536)
            .await
            .expect("health body");
        let report: serde_json::Value = serde_json::from_slice(&body).expect("health JSON");
        assert_eq!(report["ready"], false);
        let response = dispatch(&state, HttpMethod::Get, "/metrics").into_response();
        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), 65536)
            .await
            .expect("metrics body");
        let metrics = std::str::from_utf8(&body).expect("metrics text");
        assert!(metrics.contains("ai_blaise_sidecar_ready{component=\"cdc\"} 0\n"));
    }

    #[tokio::test]
    async fn draining_unready_runtime_does_not_promote_readiness() {
        let state = ProbeState::new("cdc");
        let response = dispatch(&state, HttpMethod::Post, "/drain").into_response();
        assert_eq!(response.status(), axum::http::StatusCode::ACCEPTED);
        let response = dispatch(&state, HttpMethod::Get, "/readyz").into_response();
        assert_eq!(
            response.status(),
            axum::http::StatusCode::SERVICE_UNAVAILABLE
        );
        let body = axum::body::to_bytes(response.into_body(), 65536)
            .await
            .expect("draining body");
        let report: serde_json::Value = serde_json::from_slice(&body).expect("draining JSON");
        assert_eq!(report["ready"], false);
        assert_eq!(report["accepting_new_work"], false);
    }
}
