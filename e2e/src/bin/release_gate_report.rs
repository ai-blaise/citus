use ai_blaise_citus_e2e::{V2ReleaseGateAcceptance, V2ReleaseGateReport};
use std::process;

fn main() {
    let report = V2ReleaseGateAcceptance::canonical()
        .report()
        .unwrap_or_else(|error| {
            eprintln!("failed to build canonical V2 release-gate report: {error}");
            process::exit(1);
        });

    println!("{}", V2ReleaseGateReport::tsv_header());
    println!("{}", report.to_tsv_row());
}
