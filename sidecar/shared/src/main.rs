// FEATURE: O4
// FEATURE: SC7

use ai_blaise_citus_sidecar_shared::{
    canonical_sidecar_log_records, canonical_sidecar_log_schemas, run_probe_server,
    validate_sidecar_log_json, EndpointRegistry, HttpMethod, HttpProbeRequest, RetargetConfig,
    SidecarRuntime,
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
        [command] if command == "serve" => run_server("sidecar-shared", "0.0.0.0:8080"),
        [] => emit_probe_canonical(),
        [command] if command == "probe-canonical" => emit_probe_canonical(),
        [command] if command == "ha-canonical" => emit_ha_canonical(),
        [command] if command == "log-schema-canonical" => emit_log_schema_canonical(),
        [command] if command == "log-schema-records-canonical" => {
            emit_log_schema_records_canonical()
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

fn emit_log_schema_records_canonical() {
    println!("sidecar	validated	json");
    for record in canonical_sidecar_log_records() {
        let rendered = record.to_json_line();
        if let Err(error) = validate_sidecar_log_json(&record.component, &rendered) {
            eprintln!(
                "sidecar-shared: log record {} failed: {error}",
                record.component
            );
            process::exit(1);
        }
        println!("{}	true	{}", record.component, escape_field(&rendered));
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
    println!("usage: sidecar-shared [serve|probe-canonical|ha-canonical|log-schema-canonical|log-schema-records-canonical]");
    println!("emits tab-separated canonical probes, sidecar HA retarget decisions, and log schema counts");
}

fn emit_ha_canonical() {
    let config = RetargetConfig::parse(
        "id=primary,target=http://10.0.0.10:8080,priority=1,weight=10,failover_after=1;\
         id=standby,target=http://10.0.1.10:8080,priority=2,weight=10,failover_after=1",
    )
    .expect("canonical retarget config");
    let mut registry = EndpointRegistry::new(config);

    println!("phase\tgeneration\tselected\treason");
    emit_decision("initial", &registry);
    registry
        .record_failure("primary", "connection refused")
        .expect("record primary failure");
    emit_decision("primary_failed", &registry);
    registry
        .begin_drain("standby", 1)
        .expect("begin standby drain");
    emit_decision("standby_draining", &registry);
    let reloaded = registry.reload(
        RetargetConfig::parse(
            "id=primary,target=http://10.0.0.10:8080,priority=1,weight=10,failover_after=1;\
             id=standby,target=http://10.0.1.10:8080,priority=2,weight=10,failover_after=1;\
             id=canary,target=http://10.0.2.10:8080,priority=3,weight=1,failover_after=1",
        )
        .expect("canonical reload config"),
    );
    println!(
        "reload\t{}\t{}\t{}",
        reloaded.previous_generation, reloaded.generation, reloaded.endpoint_count
    );
    emit_decision("after_reload", &registry);
}

fn emit_decision(phase: &str, registry: &EndpointRegistry) {
    let decision = registry.select();
    println!(
        "{}\t{}\t{}\t{}",
        phase,
        decision.generation,
        decision.selected_id().unwrap_or("none"),
        decision.reason,
    );
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
