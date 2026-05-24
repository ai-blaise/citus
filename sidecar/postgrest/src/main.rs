// FEATURE: API1
// FEATURE: API2
// FEATURE: API5
// FEATURE: API6

use ai_blaise_citus_sidecar_postgrest::{
    canonical_postgrest_execution_plan, canonical_postgrest_runtime_report,
    postgrest_runtime_dependency_report_from_env, serve_postgrest_sidecar_http_forever, RestMethod,
    SupervisorState,
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

    if args == ["check-runtime-dependencies"] {
        run_dependency_check();
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
    println!(
        "usage: postgrest [serve|run-canonical|run-runtime-canonical|check-runtime-dependencies]"
    );
    println!("serves the PostgREST sidecar HTTP front door or emits a deterministic TSV plan");
}

fn run_http_server(default_addr: &str) {
    if let Err(error) = serve_postgrest_sidecar_http_forever(default_addr) {
        eprintln!("postgrest: HTTP server failed: {error}");
        process::exit(1);
    }
}

fn run_runtime_canonical() {
    let report = canonical_postgrest_runtime_report().unwrap_or_else(|error| {
        eprintln!("postgrest: canonical runtime report failed: {error}");
        process::exit(1);
    });
    println!(
        "binary\tconfig_path\tlaunches\trestarts\tstate\tconfig_bytes\topenapi_bytes\tschemas\troute"
    );
    println!(
        "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}.{}",
        report.launch.binary_path,
        report.launch.config_path,
        report.state.launches,
        report.state.restarts,
        supervisor_state_name(&report.state.state),
        report.state.config_bytes,
        report.state.openapi_bytes,
        report.plan.schemas.join(","),
        report.plan.routes[0].schema,
        report.plan.routes[0].table,
    );
}

fn method_name(method: &RestMethod) -> &'static str {
    match method {
        RestMethod::Get => "get",
        RestMethod::Post => "post",
        RestMethod::Patch => "patch",
        RestMethod::Delete => "delete",
    }
}

fn supervisor_state_name(state: &SupervisorState) -> &'static str {
    match state {
        SupervisorState::Pending => "pending",
        SupervisorState::Launched => "launched",
        SupervisorState::CrashedAndRestarted => "crashed_and_restarted",
        SupervisorState::Drained => "drained",
    }
}

fn run_dependency_check() {
    let report = postgrest_runtime_dependency_report_from_env().unwrap_or_else(|error| {
        eprintln!("postgrest: runtime dependency check failed: {error}");
        process::exit(1);
    });
    println!("binary	config_path	db_uri_env	jwt_secret_env	schemas	route_count");
    println!(
        "{}	{}	{}	{}	{}	{}",
        report.binary_path,
        report.config_path,
        report.db_uri_env,
        report.jwt_secret_env,
        report.schemas.join(","),
        report.route_count,
    );
}
