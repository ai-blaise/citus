use ai_blaise_citus_admin::canonical_admin_plan;
use std::env;
use std::process;

fn main() {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print_usage();
        return;
    }

    if !args.is_empty() && args != ["run-canonical"] {
        eprintln!("citus-admin: unknown command");
        print_usage();
        process::exit(2);
    }

    let plan = canonical_admin_plan();
    if let Err(errors) = plan.validate() {
        eprintln!("citus-admin: canonical plan failed: {}", errors.join("; "));
        process::exit(1);
    }

    let routes = plan.routes();
    println!("routes\tactions");
    println!("{}\t{}", routes.len(), plan.actions.len());
}

fn print_usage() {
    println!("usage: citus-admin [run-canonical]");
    println!("runs the deterministic canonical admin UI route/action report and emits TSV");
}
