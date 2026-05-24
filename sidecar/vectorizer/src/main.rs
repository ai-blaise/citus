// FEATURE: A2
// FEATURE: A3
// FEATURE: A4
// FEATURE: A5
// FEATURE: A6

use ai_blaise_citus_sidecar_vectorizer::runtime::{
    self, AppState, BudgetStore, CohereProvider, OllamaProvider, OpenAiProvider, PgBudgetStore,
    PgQueueStore, PgUsageLogStore, ProviderRegistry, QueueStore, RuntimeConfig, StaticCostTable,
    UsageLogStore, VectorizerRuntime, VoyageProvider,
};
use ai_blaise_citus_sidecar_vectorizer::{canonical_execution_report, EmbeddingProvider};
use std::env;
use std::error::Error;
use std::process;
use std::sync::Arc;

fn main() {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print_usage();
        return;
    }

    if args == ["serve"] {
        run_serve();
        return;
    }

    if !args.is_empty() && args != ["run-canonical"] {
        eprintln!("vectorizer: unknown command");
        print_usage();
        process::exit(2);
    }

    let report = canonical_execution_report().unwrap_or_else(|error| {
        eprintln!("vectorizer: canonical execution failed: {error}");
        process::exit(1);
    });

    println!(
        "source_table\tsource_pk\ttenant_id\tprovider\tmodel\ttokens\tcost_micros\tdimensions"
    );
    for result in &report.results {
        println!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            result.source_table,
            result.source_pk,
            result.usage.tenant_id,
            provider_name(&result.usage.provider),
            result.usage.model,
            result.usage.tokens,
            result.usage.cost_micros,
            result.embedding.len(),
        );
    }
}

fn print_usage() {
    println!("usage: vectorizer [serve|run-canonical]");
    println!("  serve         start the async runtime: HTTP server + queue poll loop");
    println!("  run-canonical run the deterministic in-process model and emit TSV");
}

fn run_serve() {
    let runtime_handle = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(error) => {
            eprintln!("vectorizer: failed to start tokio runtime: {error}");
            process::exit(1);
        }
    };
    runtime_handle.block_on(async {
        if let Err(error) = serve_async().await {
            eprintln!("vectorizer: serve failed: {error}");
            process::exit(1);
        }
    });
}

async fn serve_async() -> Result<(), Box<dyn Error>> {
    init_tracing();
    let config = runtime::runtime_config_from_env()?;
    config.validate()?;
    tracing::info!(
        provider_mode = %config.provider_mode,
        listen_addr = %config.listen_addr,
        queue_table = %config.queue_table,
        "vectorizer starting"
    );

    let pg_client = connect_postgres(&config.database_url).await?;
    let pg_client = Arc::new(pg_client);

    let queue_store = Arc::new(PgQueueStore::new(pg_client.clone(), &config.queue_table));
    let budget_store = Arc::new(PgBudgetStore::new(pg_client.clone(), &config.budget_table));
    let usage_log_store = Arc::new(PgUsageLogStore::new(
        pg_client.clone(),
        &config.usage_log_table,
    ));

    queue_store
        .ensure_schema()
        .await
        .map_err(|error| format!("queue schema bootstrap: {error}"))?;
    budget_store
        .ensure_schema()
        .await
        .map_err(|error| format!("budget schema bootstrap: {error}"))?;
    usage_log_store
        .ensure_schema()
        .await
        .map_err(|error| format!("usage log schema bootstrap: {error}"))?;

    let queue: Arc<dyn QueueStore> = queue_store.clone();
    let budgets: Arc<dyn BudgetStore> = budget_store.clone();
    let usage_log: Arc<dyn UsageLogStore> = usage_log_store.clone();

    let providers = build_providers(&config)?;
    let cost_table = Arc::new(default_cost_table());
    let worker_id = env::var("AI_BLAISE_VECTORIZER_WORKER_ID")
        .unwrap_or_else(|_| format!("worker-{}", process::id()));
    let runtime_inner = Arc::new(VectorizerRuntime::new(
        config.clone(),
        queue,
        budgets,
        usage_log,
        Arc::new(providers),
        cost_table,
        worker_id,
    ));

    let state = AppState::new(runtime_inner.clone());
    let listen_addr = config.listen_addr.clone();
    let mut server_handle =
        tokio::spawn(async move { runtime::serve_http(state, &listen_addr).await });

    // Install Ctrl-C/SIGTERM as a drain trigger.
    let shutdown_runtime = runtime_inner.clone();
    let mut signal_task = tokio::spawn(async move {
        wait_for_shutdown_signal().await;
        tracing::info!("shutdown signal received; draining vectorizer");
        shutdown_runtime.trigger_shutdown();
    });

    let worker_runtime = runtime_inner.clone();
    let mut worker_task = tokio::spawn(async move { worker_runtime.run_until_shutdown().await });

    let mut server_done = false;
    let mut worker_done = false;
    let mut signal_done = false;
    tokio::select! {
        result = &mut server_handle => {
            server_done = true;
            runtime_inner.trigger_shutdown();
            result.map_err(|error| error.to_string())??;
        }
        result = &mut worker_task => {
            worker_done = true;
            runtime_inner.trigger_shutdown();
            result.map_err(|error| error.to_string())??;
        }
        result = &mut signal_task => {
            signal_done = true;
            result.map_err(|error| error.to_string())?;
        }
    }

    if !server_done {
        server_handle.await.map_err(|error| error.to_string())??;
    }
    if !worker_done {
        worker_task.await.map_err(|error| error.to_string())??;
    }
    if !signal_done {
        signal_task.abort();
    }
    Ok(())
}

async fn wait_for_shutdown_signal() {
    use tokio::signal::unix::{signal, SignalKind};
    let mut sigterm = signal(SignalKind::terminate()).expect("install SIGTERM handler");
    let mut sigint = signal(SignalKind::interrupt()).expect("install SIGINT handler");
    tokio::select! {
        _ = sigterm.recv() => {}
        _ = sigint.recv() => {}
    }
}

fn init_tracing() {
    use tracing_subscriber::filter::EnvFilter;
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .try_init();
}

async fn connect_postgres(database_url: &str) -> Result<tokio_postgres::Client, Box<dyn Error>> {
    let (client, connection) = tokio_postgres::connect(database_url, tokio_postgres::NoTls).await?;
    tokio::spawn(async move {
        if let Err(error) = connection.await {
            tracing::error!(error = %error, "postgres connection closed");
        }
    });
    Ok(client)
}

fn build_providers(config: &RuntimeConfig) -> Result<ProviderRegistry, Box<dyn Error>> {
    let mut registry = ProviderRegistry::new();
    match config.provider_mode.as_str() {
        "mock" => {
            registry.insert(Arc::new(
                ai_blaise_citus_sidecar_vectorizer::runtime::MockProvider::new(
                    "mock",
                    config.mock_dimensions,
                    1,
                ),
            ));
        }
        "live" => {
            register_live_providers(&mut registry)?;
        }
        "mixed" => {
            registry.insert(Arc::new(
                ai_blaise_citus_sidecar_vectorizer::runtime::MockProvider::new(
                    "mock",
                    config.mock_dimensions,
                    1,
                ),
            ));
            register_live_providers(&mut registry)?;
        }
        other => {
            return Err(format!("unknown provider mode: {other}").into());
        }
    }
    if registry.names().is_empty() {
        return Err(
            "provider mode live requires at least one configured provider credential or base URL"
                .into(),
        );
    }
    Ok(registry)
}

fn register_live_providers(registry: &mut ProviderRegistry) -> Result<(), Box<dyn Error>> {
    use ai_blaise_citus_sidecar_vectorizer::runtime::provider::HttpProviderConfig;

    if let Ok(api_key) = env::var("OPENAI_API_KEY") {
        let base_url = env::var("OPENAI_BASE_URL")
            .unwrap_or_else(|_| OpenAiProvider::DEFAULT_BASE_URL.to_string());
        let config = HttpProviderConfig::new(base_url).with_api_key(api_key);
        let cost = parse_cost("OPENAI_COST_MICROS_PER_TOKEN", 13)?;
        registry.insert(Arc::new(OpenAiProvider::new("openai", config, cost)?));
    }
    if let Ok(api_key) = env::var("AZURE_OPENAI_API_KEY") {
        let base_url = env::var("AZURE_OPENAI_BASE_URL")
            .map_err(|_| "AZURE_OPENAI_BASE_URL is required when AZURE_OPENAI_API_KEY is set")?;
        let config = HttpProviderConfig::new(base_url).with_api_key(api_key);
        let cost = parse_cost("AZURE_OPENAI_COST_MICROS_PER_TOKEN", 13)?;
        registry.insert(Arc::new(OpenAiProvider::new("azure_openai", config, cost)?));
    }
    if let Ok(api_key) = env::var("VOYAGE_API_KEY") {
        let base_url = env::var("VOYAGE_BASE_URL")
            .unwrap_or_else(|_| VoyageProvider::DEFAULT_BASE_URL.to_string());
        let config = HttpProviderConfig::new(base_url).with_api_key(api_key);
        let cost = parse_cost("VOYAGE_COST_MICROS_PER_TOKEN", 6)?;
        registry.insert(Arc::new(VoyageProvider::new("voyage", config, cost)?));
    }
    if let Ok(api_key) = env::var("COHERE_API_KEY") {
        let base_url = env::var("COHERE_BASE_URL")
            .unwrap_or_else(|_| CohereProvider::DEFAULT_BASE_URL.to_string());
        let config = HttpProviderConfig::new(base_url).with_api_key(api_key);
        let cost = parse_cost("COHERE_COST_MICROS_PER_TOKEN", 10)?;
        registry.insert(Arc::new(CohereProvider::new("cohere", config, cost)?));
    }
    if let Ok(base_url) = env::var("OLLAMA_BASE_URL") {
        let config = HttpProviderConfig::new(base_url);
        let cost = parse_cost("OLLAMA_COST_MICROS_PER_TOKEN", 0)?;
        registry.insert(Arc::new(OllamaProvider::new("ollama", config, cost)?));
    } else if env::var("ENABLE_OLLAMA").is_ok() {
        let config = HttpProviderConfig::new(OllamaProvider::DEFAULT_BASE_URL.to_string());
        let cost = parse_cost("OLLAMA_COST_MICROS_PER_TOKEN", 0)?;
        registry.insert(Arc::new(OllamaProvider::new("ollama", config, cost)?));
    }
    if let Ok(base_url) = env::var("VLLM_BASE_URL") {
        let api_key = env::var("VLLM_API_KEY").ok();
        let mut http =
            ai_blaise_citus_sidecar_vectorizer::runtime::provider::HttpProviderConfig::new(
                base_url,
            );
        if let Some(key) = api_key {
            http = http.with_api_key(key);
        }
        let cost = parse_cost("VLLM_COST_MICROS_PER_TOKEN", 1)?;
        registry.insert(Arc::new(OpenAiProvider::new("vllm", http, cost)?));
    }
    Ok(())
}

fn parse_cost(name: &str, default: u64) -> Result<u64, Box<dyn Error>> {
    match env::var(name) {
        Ok(value) => value
            .parse::<u64>()
            .map_err(|error| format!("invalid {name}: {error}").into()),
        Err(_) => Ok(default),
    }
}

fn default_cost_table() -> StaticCostTable {
    StaticCostTable::new(1)
        .with("openai", 13)
        .with("azure_openai", 13)
        .with("voyage", 6)
        .with("cohere", 10)
        .with("ollama", 0)
        .with("vllm", 1)
        .with("mock", 1)
}

fn provider_name(provider: &EmbeddingProvider) -> &'static str {
    match provider {
        EmbeddingProvider::OpenAi => "openai",
        EmbeddingProvider::AzureOpenAi => "azure_openai",
        EmbeddingProvider::Anthropic => "anthropic",
        EmbeddingProvider::Cohere => "cohere",
        EmbeddingProvider::Voyage => "voyage",
        EmbeddingProvider::Ollama => "ollama",
        EmbeddingProvider::VertexAi => "vertex_ai",
    }
}
