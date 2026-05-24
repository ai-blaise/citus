// FEATURE: API3
// FEATURE: API4
// FEATURE: API5

use ai_blaise_citus_sidecar_graphql::{
    canonical_graphql_execution_plan, canonical_graphql_runtime_report,
    serve_graphql_sidecar_http_forever,
};
use std::env;
use std::process;

fn main() {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print_usage();
        return;
    }

    if args == ["serve"] {
        run_http_server("0.0.0.0:8080");
        return;
    }

    if args == ["run-runtime-canonical"] {
        run_runtime_canonical();
        return;
    }

    if !args.is_empty() && args != ["run-canonical"] {
        eprintln!("graphql: unknown command");
        print_usage();
        process::exit(2);
    }

    let plan = canonical_graphql_execution_plan().unwrap_or_else(|error| {
        eprintln!("graphql: canonical plan failed: {error}");
        process::exit(1);
    });
    let schema = &plan.schema_bindings[0];
    let binding = &plan.distributed_bindings[0];

    println!(
        "endpoint\tnamespace\texposed_tables\ttype_name\ttable\tdistribution_column\troute_function\ttenant_claim\tintrospection"
    );
    println!(
        "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
        plan.endpoint_path,
        schema.graphql_namespace,
        schema.exposed_tables.join(","),
        binding.type_name,
        binding.table,
        binding.distribution_column,
        binding.route_function,
        plan.auth.tenant_claim,
        plan.auth.introspection_enabled,
    );
}

fn print_usage() {
    println!("usage: graphql [serve|run-canonical|run-runtime-canonical]");
    println!("serves the GraphQL sidecar HTTP front door or emits a deterministic TSV plan");
}

fn run_http_server(default_addr: &str) {
    if let Err(error) = serve_graphql_sidecar_http_forever(default_addr) {
        eprintln!("graphql: HTTP server failed: {error}");
        process::exit(1);
    }
}

fn run_runtime_canonical() {
    let report = canonical_graphql_runtime_report().unwrap_or_else(|error| {
        eprintln!("graphql: canonical runtime report failed: {error}");
        process::exit(1);
    });
    println!(
        "endpoint\tnamespace\tqueries_resolved\tsubscriptions_registered\tplans_persisted\ttenant_id\tdistributed_types\tsubscription_field"
    );
    println!(
        "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
        report.plan.endpoint_path,
        report.plan.schema_bindings[0].graphql_namespace,
        report.state.queries_resolved,
        report.state.subscriptions_registered,
        report.state.plans_persisted,
        report
            .response
            .tenant_id
            .unwrap_or_else(|| "none".to_string()),
        report.response.execution_plan.distributed_types.join(","),
        report.subscription.field,
    );
}
