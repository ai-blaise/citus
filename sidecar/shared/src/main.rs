// FEATURE: O4

use ai_blaise_citus_sidecar_shared::{
    run_probe_server, HttpMethod, HttpProbeRequest, SidecarRuntime,
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
        run_server("sidecar-shared", "0.0.0.0:8080");
        return;
    }

    if !args.is_empty() && args != ["probe-canonical"] {
        eprintln!("sidecar-shared: unknown command");
        print_usage();
        process::exit(2);
    }

    let mut runtime = SidecarRuntime::ready("sidecar-shared").with_in_flight_work(2);
    println!("method\tpath\tstatus\tcontent_type\tbody");
    emit_probe(&mut runtime, HttpMethod::Get, "/healthz");
    emit_probe(&mut runtime, HttpMethod::Get, "/readyz");
    emit_probe(&mut runtime, HttpMethod::Get, "/metrics");
    emit_probe(&mut runtime, HttpMethod::Post, "/drain");
    emit_probe(&mut runtime, HttpMethod::Get, "/readyz");
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
    println!("usage: sidecar-shared [serve|probe-canonical]");
    println!("emits tab-separated canonical health, readiness, drain, and metrics probes");
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
