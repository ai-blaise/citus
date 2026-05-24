// FEATURE: O4

use ai_blaise_citus_sidecar_shared::{
    canonical_sidecar_log_schemas, run_probe_server, HttpMethod, HttpProbeRequest, SidecarRuntime,
};
use std::env;
use std::process;

fn main() {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print_usage();
        return;
    }

    match args.as_slice() {
        [command] if command == "serve" => {
            run_server("sidecar-shared", "0.0.0.0:8080");
        }
        [] => {
            emit_probe_canonical();
        }
        [command] if command == "probe-canonical" => {
            emit_probe_canonical();
        }
        [command] if command == "log-schema-canonical" => {
            emit_log_schema_canonical();
        }
        _ => {
            eprintln!("sidecar-shared: unknown command");
            print_usage();
            process::exit(2);
        }
    }
}

fn emit_probe_canonical() {
    let mut runtime = SidecarRuntime::ready("sidecar-shared").with_in_flight_work(2);
    println!("method\tpath\tstatus\tcontent_type\tbody");
    emit_probe(&mut runtime, HttpMethod::Get, "/healthz");
    emit_probe(&mut runtime, HttpMethod::Get, "/readyz");
    emit_probe(&mut runtime, HttpMethod::Get, "/metrics");
    emit_probe(&mut runtime, HttpMethod::Post, "/drain");
    emit_probe(&mut runtime, HttpMethod::Get, "/readyz");
}

fn emit_log_schema_canonical() {
    println!("sidecar\tcommon_fields\textension_fields\trequired_fields\ttotal_fields");
    for sidecar in canonical_sidecar_log_schemas() {
        if let Err(error) = sidecar.schema.validate() {
            eprintln!(
                "sidecar-shared: log schema {} failed: {error}",
                sidecar.sidecar
            );
            process::exit(1);
        }

        let required_fields = sidecar
            .schema
            .all_fields()
            .filter(|field| field.required)
            .count();
        let total_fields = sidecar.schema.all_fields().count();
        println!(
            "{}\t{}\t{}\t{}\t{}",
            sidecar.sidecar,
            sidecar.schema.common.len(),
            sidecar.schema.extensions.len(),
            required_fields,
            total_fields,
        );
    }
}

fn emit_probe(runtime: &mut SidecarRuntime, method: HttpMethod, path: &str) {
    let method_name = method_name(&method).to_string();
    let response = runtime.handle_http_request(&HttpProbeRequest::new(method, path));
    println!(
        "{}\t{}\t{}\t{}\t{}",
        method_name,
        path,
        response.status_code,
        response.content_type,
        escape_field(&response.body),
    );
}

fn method_name(method: &HttpMethod) -> &str {
    match method {
        HttpMethod::Get => "GET",
        HttpMethod::Post => "POST",
        HttpMethod::Other(method) => method.as_str(),
    }
}

fn print_usage() {
    println!("usage: sidecar-shared [serve|probe-canonical|log-schema-canonical]");
    println!("emits tab-separated canonical health, readiness, drain, and metrics probes");
    println!("log-schema-canonical emits tab-separated sidecar structured-log schema counts");
}

fn run_server(component: &str, default_addr: &str) {
    if let Err(error) = run_probe_server(component, default_addr) {
        eprintln!("{component}: probe server failed: {error}");
        process::exit(1);
    }
}

fn escape_field(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('\t', "\\t")
        .replace('\n', "\\n")
}
