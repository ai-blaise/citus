// FEATURE: RT1
// FEATURE: RT2
// FEATURE: RT3
// FEATURE: RT4

use ai_blaise_citus_sidecar_cdc::CdcOperation;
use ai_blaise_citus_sidecar_realtime::{canonical_broadcast_plan, canonical_realtime_event};
use std::env;
use std::process;

fn main() {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print_usage();
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

fn print_usage() {
    println!("usage: realtime [run-canonical]");
    println!("runs the deterministic canonical realtime broadcast plan and emits TSV");
}

fn operation_name(operation: &CdcOperation) -> &'static str {
    match operation {
        CdcOperation::Insert => "insert",
        CdcOperation::Update => "update",
        CdcOperation::Delete => "delete",
        CdcOperation::Truncate => "truncate",
    }
}
