use ai_blaise_citus_watch::canonical_watch_dashboard;
use std::env;
use std::process;

fn main() {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print_usage();
        return;
    }

    if !args.is_empty() && args != ["run-canonical"] {
        eprintln!("citus-watch: unknown command");
        print_usage();
        process::exit(2);
    }

    let plan = canonical_watch_dashboard();
    let queries = plan.queries().unwrap_or_else(|error| {
        eprintln!("citus-watch: canonical dashboard failed: {error}");
        process::exit(1);
    });

    println!("data_sources\tpanels\tqueries\trefresh_interval_seconds");
    println!(
        "{}\t{}\t{}\t{}",
        plan.data_sources.len(),
        plan.panels.len(),
        queries.len(),
        plan.refresh_interval_seconds,
    );
}

fn print_usage() {
    println!("usage: citus-watch [run-canonical]");
    println!("runs the deterministic canonical watch dashboard report and emits TSV");
}
