// FEATURE: A2
// FEATURE: A5
// FEATURE: A6

use ai_blaise_citus_sidecar_shared::run_probe_server;
use ai_blaise_citus_sidecar_vectorizer::{canonical_execution_report, EmbeddingProvider};
use std::env;
use std::process;

fn main() {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print_usage();
        return;
    }

    if args == ["serve"] {
        run_server("vectorizer", "0.0.0.0:8080");
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
    println!("runs the deterministic canonical vectorizer batch and emits usage TSV");
}

fn run_server(component: &str, default_addr: &str) {
    if let Err(error) = run_probe_server(component, default_addr) {
        eprintln!("{component}: probe server failed: {error}");
        process::exit(1);
    }
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
