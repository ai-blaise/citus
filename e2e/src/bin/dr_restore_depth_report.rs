// FEATURE: B1
// FEATURE: B3
// FEATURE: B4
// FEATURE: B6
// FEATURE: MR9

use ai_blaise_citus_e2e::{DrRestoreDepthAcceptance, DrRestoreDepthReport};
use std::process;

fn main() {
    let report = DrRestoreDepthAcceptance::canonical()
        .report()
        .unwrap_or_else(|error| {
            eprintln!("failed to build canonical DR restore-depth report: {error}");
            process::exit(1);
        });

    println!("{}", DrRestoreDepthReport::tsv_header());
    println!("{}", report.to_tsv_row());
}
