// FEATURE: API1
// FEATURE: API2
// FEATURE: API5
// FEATURE: API6

use ai_blaise_citus_sidecar_postgrest::{canonical_postgrest_execution_plan, RestMethod};
use std::env;
use std::process;

fn main() {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print_usage();
        return;
    }

    if !args.is_empty() && args != ["run-canonical"] {
        eprintln!("postgrest: unknown command");
        print_usage();
        process::exit(2);
    }

    let plan = canonical_postgrest_execution_plan().unwrap_or_else(|error| {
        eprintln!("postgrest: canonical plan failed: {error}");
        process::exit(1);
    });
    let route = &plan.routes[0];
    let binding = route.distributed_view.as_ref().expect("canonical binding");

    println!(
        "schemas\troute\tmethods\tview\tdistribution_column\tshard_count\topenapi_path\ttenant_claim"
    );
    println!(
        "{}\t{}.{}\t{}\t{}\t{}\t{}\t{}\t{}",
        plan.schemas.join(","),
        route.schema,
        route.table,
        route
            .methods
            .iter()
            .map(method_name)
            .collect::<Vec<_>>()
            .join(","),
        binding.view_name,
        binding.distribution_column,
        binding.shard_count,
        plan.openapi.path,
        plan.auth.tenant_claim,
    );
}

fn print_usage() {
    println!("usage: postgrest [run-canonical]");
    println!("runs the deterministic canonical PostgREST sidecar plan and emits TSV");
}

fn method_name(method: &RestMethod) -> &'static str {
    match method {
        RestMethod::Get => "get",
        RestMethod::Post => "post",
        RestMethod::Patch => "patch",
        RestMethod::Delete => "delete",
    }
}
