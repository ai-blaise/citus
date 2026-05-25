use ai_blaise_citusctl::{
    canonical_citusctl_report, canonical_dev_lifecycle_report, parse_request,
    render_dev_lifecycle_cli_report_from_args, render_k8s_manifest_cli_report_from_args,
    wal_replay_debug_plan_from_args,
};
use std::env;
use std::process;

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    if args == ["run-dev-lifecycle-canonical"] {
        let report = canonical_dev_lifecycle_report().unwrap_or_else(|error| {
            eprintln!("citusctl: dev lifecycle canonical report failed: {error}");
            process::exit(2);
        });
        println!(
            "state_dir	plan_up_steps	apply_up_changed	idempotent_up_changed	apply_down_changed	idempotent_down_changed	final_state_present	cleanup_guard	evidence_boundary"
        );
        println!(
            "{}	{}	{}	{}	{}	{}	{}	{}	{}",
            report.state_dir,
            report.plan_up_steps,
            report.apply_up_changed,
            report.idempotent_up_changed,
            report.apply_down_changed,
            report.idempotent_down_changed,
            report.final_state_present,
            report.cleanup_guard,
            report.evidence_boundary,
        );
        return;
    }

    if args == ["run-canonical"] {
        let report = canonical_citusctl_report().unwrap_or_else(|error| {
            eprintln!("citusctl: canonical report failed: {error}");
            process::exit(2);
        });
        println!("plans\tcatalog\tdestructive\tpreflight\texecute\tsteps");
        println!(
            "{}\t{}\t{}\t{}\t{}\t{}",
            report.plans.len(),
            report.catalog_count,
            report.destructive_count(),
            report.preflight_count(),
            report.execute_count(),
            report.total_steps(),
        );
        return;
    }

    match render_k8s_manifest_cli_report_from_args(&args) {
        Ok(Some(output)) => {
            println!("{output}");
            return;
        }
        Ok(None) => {}
        Err(error) => {
            eprintln!("citusctl: {error}");
            process::exit(2);
        }
    }

    match render_dev_lifecycle_cli_report_from_args(&args) {
        Ok(Some(output)) => {
            println!("{output}");
            return;
        }
        Ok(None) => {}
        Err(error) => {
            eprintln!("citusctl: {error}");
            process::exit(2);
        }
    }

    let is_wal_replay_debug = args.iter().any(|arg| arg == "--fixture")
        || (args.iter().any(|arg| arg == "wal-replay") && args.iter().any(|arg| arg == "--json"));
    if is_wal_replay_debug {
        match wal_replay_debug_plan_from_args(&args) {
            Ok(plan) => println!("{}", plan.to_json()),
            Err(error) => {
                eprintln!("citusctl: {error}");
                process::exit(2);
            }
        }
        return;
    }

    match parse_request(&args).and_then(|request| request.plan()) {
        Ok(plan) => {
            println!(
                "citusctl {} destructive={} requires_plan_id={} steps={}",
                plan.command_name,
                plan.destructive,
                plan.requires_plan_id,
                plan.steps.len()
            );
        }
        Err(error) => {
            eprintln!("citusctl: {error}");
            process::exit(2);
        }
    }
}
