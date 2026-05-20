// FEATURE: API3
// FEATURE: API4
// FEATURE: API5

use ai_blaise_citus_sidecar_graphql::canonical_graphql_execution_plan;
use std::env;
use std::process;

fn main() {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print_usage();
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
    println!("usage: graphql [run-canonical]");
    println!("runs the deterministic canonical GraphQL sidecar plan and emits TSV");
}
