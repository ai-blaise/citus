use ai_blaise_citusctl::{
    canonical_citusctl_report, parse_request, wal_replay_debug_plan_from_args,
};
use std::env;
use std::process;

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
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

    if args.iter().any(|arg| arg == "--json" || arg == "--fixture") {
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
