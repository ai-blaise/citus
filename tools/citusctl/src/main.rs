use ai_blaise_citusctl::{
    canonical_citusctl_report, canonical_dev_lifecycle_report, parse_request,
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
