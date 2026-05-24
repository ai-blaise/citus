use ai_blaise_citus_tool_runtime::{parse_snapshot_tsv, ToolSnapshot};
use ai_blaise_citus_tui::{
    canonical_tui_session, TuiAction, TuiPanel, TuiRuntime, TuiRuntimeError, TuiSessionPlan,
};
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
        Some("render-frame") => render_frame(&args[1..]),
        Some("action") => preview_action(&args[1..]),
        Some(_) => {
            eprintln!("citus-tui: unknown command");
            print_usage();
            process::exit(2);
        }
    };

    if let Err(error) = result {
        eprintln!("citus-tui: {error}");
        process::exit(1);
    }
}

fn run_canonical() -> Result<(), String> {
    let plan = canonical_tui_session();
    if let Err(errors) = plan.validate() {
        return Err(format!("canonical session failed: {}", errors.join("; ")));
    }

    println!("panels\tactions\tsafe_mode\trequired_panels");
    println!(
        "{}\t{}\t{}\t{}",
        plan.panels.len(),
        plan.actions.len(),
        plan.safe_mode,
        TuiSessionPlan::required_panels().len(),
    );
    Ok(())
}

fn render_frame(args: &[String]) -> Result<(), String> {
    let snapshot = load_snapshot(&required_value(args, "--snapshot")?)?;
    let panel_name = required_value(args, "--panel")?;
    let panel =
        TuiPanel::from_name(&panel_name).ok_or_else(|| format!("unknown panel {panel_name}"))?;
    let runtime = TuiRuntime::new(snapshot).map_err(to_string)?;
    println!("{}", runtime.render_panel(panel).map_err(to_string)?);
    Ok(())
}

fn preview_action(args: &[String]) -> Result<(), String> {
    let snapshot = load_snapshot(&required_value(args, "--snapshot")?)?;
    let kind = required_value(args, "--kind")?;
    let action = match kind.as_str() {
        "explain-query" => TuiAction::ExplainQuery {
            sql: required_value(args, "--sql")?,
        },
        "rebalance-dry-run" => TuiAction::RebalanceDryRun {
            shard_id: parse_u64(&required_value(args, "--shard-id")?, "--shard-id")?,
        },
        "rebalance-apply" => TuiAction::RebalanceApply {
            shard_id: parse_u64(&required_value(args, "--shard-id")?, "--shard-id")?,
        },
        "tenant-move" => TuiAction::TenantMove {
            tenant: required_value(args, "--tenant")?,
            target_worker: required_value(args, "--target-worker")?,
        },
        "branch-promote" => TuiAction::BranchPromote {
            branch: required_value(args, "--branch")?,
        },
        _ => return Err(format!("unknown action kind {kind}")),
    };
    let unsafe_allow_mutation = has_flag(args, "--unsafe-allow-mutation");
    let confirmation = optional_value(args, "--confirm");
    let runtime = TuiRuntime::new(snapshot).map_err(to_string)?;
    let receipt = runtime
        .preview_action(action, unsafe_allow_mutation, confirmation.as_deref())
        .map_err(to_string)?;
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

fn has_flag(args: &[String], flag: &str) -> bool {
    args.iter().any(|arg| arg == flag)
}

fn parse_u64(value: &str, flag: &str) -> Result<u64, String> {
    value
        .parse::<u64>()
        .map_err(|_| format!("{flag} must be an unsigned integer"))
}

fn to_string(error: TuiRuntimeError) -> String {
    error.to_string()
}

fn print_usage() {
    println!("usage: citus-tui [run-canonical]");
    println!("       citus-tui render-frame --snapshot <snapshot.tsv> --panel <panel>");
    println!("       citus-tui action --snapshot <snapshot.tsv> --kind <kind> [options]");
    println!("runs the canonical TSV report or snapshot-backed TUI frame runtime");
}
