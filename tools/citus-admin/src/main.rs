use ai_blaise_citus_admin::{
    canonical_admin_plan, AdminRuntime, AdminRuntimeAction, AdminRuntimeError,
};
use ai_blaise_citus_tool_runtime::{parse_snapshot_tsv, ToolSnapshot};
use std::env;
use std::fs;
use std::process;

fn main() {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print_usage();
        return;
    }

    let result = match args.first().map(String::as_str) {
        None | Some("run-canonical") => run_canonical(),
        Some("render") => render_snapshot(&args[1..]),
        Some("action") => execute_action(&args[1..]),
        Some(_) => {
            eprintln!("citus-admin: unknown command");
            print_usage();
            process::exit(2);
        }
    };

    if let Err(error) = result {
        eprintln!("citus-admin: {error}");
        process::exit(1);
    }
}

fn run_canonical() -> Result<(), String> {
    let plan = canonical_admin_plan();
    if let Err(errors) = plan.validate() {
        return Err(format!("canonical plan failed: {}", errors.join("; ")));
    }

    let routes = plan.routes();
    println!("routes\tactions");
    println!("{}\t{}", routes.len(), plan.actions.len());
    Ok(())
}

fn render_snapshot(args: &[String]) -> Result<(), String> {
    let snapshot = load_snapshot(&required_value(args, "--snapshot")?)?;
    let route = required_value(args, "--route")?;
    let runtime = AdminRuntime::new(snapshot).map_err(to_string)?;
    println!("{}", runtime.render_route(&route).map_err(to_string)?);
    Ok(())
}

fn execute_action(args: &[String]) -> Result<(), String> {
    let snapshot = load_snapshot(&required_value(args, "--snapshot")?)?;
    let kind = required_value(args, "--kind")?;
    let confirmation = optional_value(args, "--confirm").unwrap_or_default();
    let action = match kind.as_str() {
        "rebalance-shard" => AdminRuntimeAction::RebalanceShard {
            shard_id: parse_u64(&required_value(args, "--shard-id")?, "--shard-id")?,
            confirmation,
        },
        "move-tenant" => AdminRuntimeAction::MoveTenant {
            tenant: required_value(args, "--tenant")?,
            target_worker: required_value(args, "--target-worker")?,
            confirmation,
        },
        "suspend-branch" => AdminRuntimeAction::SuspendBranch {
            branch: required_value(args, "--branch")?,
            confirmation,
        },
        "replay-realtime-stream" => AdminRuntimeAction::ReplayRealtimeStream {
            tenant: required_value(args, "--tenant")?,
            lsn: required_value(args, "--lsn")?,
        },
        _ => return Err(format!("unknown action kind {kind}")),
    };

    let runtime = AdminRuntime::new(snapshot).map_err(to_string)?;
    let receipt = runtime.execute_action(action).map_err(to_string)?;
    println!("action\tstatus\tdetail");
    println!("{}\t{}\t{}", receipt.action, receipt.status, receipt.detail);
    Ok(())
}

fn load_snapshot(path: &str) -> Result<ToolSnapshot, String> {
    let input = fs::read_to_string(path).map_err(|error| format!("read {path}: {error}"))?;
    parse_snapshot_tsv(&input).map_err(|error| format!("parse {path}: {error}"))
}

fn required_value(args: &[String], flag: &str) -> Result<String, String> {
    optional_value(args, flag).ok_or_else(|| format!("missing required {flag}"))
}

fn optional_value(args: &[String], flag: &str) -> Option<String> {
    args.windows(2)
        .find(|window| window[0] == flag)
        .map(|window| window[1].clone())
}

fn parse_u64(value: &str, flag: &str) -> Result<u64, String> {
    value
        .parse::<u64>()
        .map_err(|_| format!("{flag} must be an unsigned integer"))
}

fn to_string(error: AdminRuntimeError) -> String {
    error.to_string()
}

fn print_usage() {
    println!("usage: citus-admin [run-canonical]");
    println!("       citus-admin render --snapshot <snapshot.tsv> --route <route>");
    println!("       citus-admin action --snapshot <snapshot.tsv> --kind <kind> [options]");
    println!("runs the canonical TSV report or snapshot-backed admin UI runtime");
}
