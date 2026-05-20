use ai_blaise_citus_tui::{canonical_tui_session, TuiSessionPlan};
use std::env;
use std::process;

fn main() {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print_usage();
        return;
    }

    if !args.is_empty() && args != ["run-canonical"] {
        eprintln!("citus-tui: unknown command");
        print_usage();
        process::exit(2);
    }

    let plan = canonical_tui_session();
    if let Err(errors) = plan.validate() {
        eprintln!("citus-tui: canonical session failed: {}", errors.join("; "));
        process::exit(1);
    }

    println!("panels\tactions\tsafe_mode\trequired_panels");
    println!(
        "{}\t{}\t{}\t{}",
        plan.panels.len(),
        plan.actions.len(),
        plan.safe_mode,
        TuiSessionPlan::required_panels().len(),
    );
}

fn print_usage() {
    println!("usage: citus-tui [run-canonical]");
    println!("runs the deterministic canonical TUI session report and emits TSV");
}
