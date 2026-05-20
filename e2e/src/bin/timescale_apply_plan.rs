// FEATURE: TS7

use ai_blaise_citus_e2e::TimescaleOnCitusAcceptance;
use std::process;

fn main() {
    let plan = TimescaleOnCitusAcceptance::canonical_metrics()
        .plan()
        .unwrap_or_else(|error| {
            eprintln!("failed to build canonical Timescale-on-Citus plan: {error}");
            process::exit(1);
        });

    println!("{}", plan.reconcile.apply_sql_script());
}
