// FEATURE: RT1
// FEATURE: RT2
// FEATURE: RT3
// FEATURE: RT4
// FEATURE: RT5

use ai_blaise_citus_sidecar_cdc::CdcOperation;
use ai_blaise_citus_sidecar_realtime::{
    canonical_broadcast_plan, canonical_realtime_event, canonical_realtime_runtime_report,
    RealtimeLiveConfig, RealtimeLiveRuntime,
};
use ai_blaise_citus_sidecar_shared::run_probe_server;
use std::env;
use std::process;

fn main() {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print_usage();
        return;
    }

    if args == ["serve"] {
        run_server("realtime", "0.0.0.0:8080");
        return;
    }

    if args == ["serve-runtime"] {
        serve_runtime();
        return;
    }

    if args == ["run-runtime-canonical"] {
        run_runtime_canonical();
        return;
    }

    if !args.is_empty() && args != ["run-canonical"] {
        eprintln!("realtime: unknown command");
        print_usage();
        process::exit(2);
    }

    let event = canonical_realtime_event();
    let broadcast = canonical_broadcast_plan().unwrap_or_else(|error| {
        eprintln!("realtime: canonical broadcast failed: {error}");
        process::exit(1);
    });
    let presence_users = broadcast
        .presence_snapshot
        .as_ref()
        .map(|presence| presence.online_users.join(","))
        .unwrap_or_default();

    println!("lsn\ttopic\ttenant_id\toperation\trecipients\tpresence_users");
    println!(
        "{}\t{}\t{}\t{}\t{}\t{}",
        event.lsn,
        broadcast.topic,
        broadcast.tenant_id,
        operation_name(&broadcast.operation),
        broadcast.recipients.join(","),
        presence_users,
    );
}

fn run_runtime_canonical() {
    let report = canonical_realtime_runtime_report().unwrap_or_else(|error| {
        eprintln!("realtime: canonical runtime failed: {error}");
        process::exit(1);
    });
    let delivery = &report.broadcast.deliveries[0];

    println!(
        "topic\ttenant_id\toperation\tconnection_id\tuser_id\tframe_bytes\tactive_connections\tfiltered_connections\tbroadcasts\tdelivered_messages\tpresence_snapshots\tpresence_users"
    );
    println!(
        "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
        report.broadcast.topic,
        report.broadcast.tenant_id,
        operation_name(&report.broadcast.operation),
        delivery.connection_id,
        delivery.user_id,
        delivery.frame_bytes,
        report.state.active_connections,
        report.broadcast.filtered_connections,
        report.state.broadcasts,
        report.state.delivered_messages,
        report.state.presence_snapshots,
        report.broadcast.presence_users.join(","),
    );
}

fn serve_runtime() {
    let mut config = RealtimeLiveConfig::default();
    if let Ok(addr) = env::var("REALTIME_WS_LISTEN_ADDR") {
        config.ws_listen_addr = addr;
    }
    if let Ok(addr) = env::var("REALTIME_CDC_INGEST_ADDR") {
        config.cdc_ingest_addr = addr;
    }
    if let Ok(key) = env::var("REALTIME_APIKEY") {
        config.realm_apikey = Some(key);
    }
    let runtime = RealtimeLiveRuntime::new(config.clone());
    let ws_handle = runtime.spawn_ws_listener().unwrap_or_else(|error| {
        eprintln!(
            "realtime: ws bind {} failed: {error}",
            config.ws_listen_addr
        );
        process::exit(1);
    });
    let cdc_handle = runtime.spawn_cdc_ingest_listener().unwrap_or_else(|error| {
        eprintln!(
            "realtime: cdc ingest bind {} failed: {error}",
            config.cdc_ingest_addr
        );
        process::exit(1);
    });
    eprintln!(
        "realtime: serve-runtime ws={} cdc_ingest={}",
        config.ws_listen_addr, config.cdc_ingest_addr
    );
    let _ = ws_handle.join();
    let _ = cdc_handle.join();
}

fn print_usage() {
    println!("usage: realtime [serve|serve-runtime|run-canonical|run-runtime-canonical]");
    println!("runs the realtime sidecar: a deterministic canonical broadcaster and a live");
    println!("WS + CDC ingest runtime that fans CDC events out to phoenix-channel clients.");
}

fn run_server(component: &str, default_addr: &str) {
    if let Err(error) = run_probe_server(component, default_addr) {
        eprintln!("{component}: probe server failed: {error}");
        process::exit(1);
    }
}

fn operation_name(operation: &CdcOperation) -> &'static str {
    match operation {
        CdcOperation::Insert => "insert",
        CdcOperation::Update => "update",
        CdcOperation::Delete => "delete",
        CdcOperation::Truncate => "truncate",
    }
}
