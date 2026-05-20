// FEATURE: RT1
// FEATURE: RT2
// FEATURE: RT3
// FEATURE: RT4

use ai_blaise_citus_sidecar_cdc::CdcOperation;
use ai_blaise_citus_sidecar_realtime::{
    canonical_broadcast_plan, canonical_realtime_event, canonical_realtime_runtime_report,
};
use std::env;
use std::process;

fn main() {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print_usage();
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

fn print_usage() {
    println!("usage: realtime [run-canonical|run-runtime-canonical]");
    println!("runs deterministic canonical realtime broadcast/runtime reports and emits TSV");
}

fn operation_name(operation: &CdcOperation) -> &'static str {
    match operation {
        CdcOperation::Insert => "insert",
        CdcOperation::Update => "update",
        CdcOperation::Delete => "delete",
        CdcOperation::Truncate => "truncate",
    }
}
