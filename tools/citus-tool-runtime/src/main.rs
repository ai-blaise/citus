use ai_blaise_citus_tool_runtime::canonical_snapshot;
use std::env;
use std::process;

fn main() {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print_usage();
        return;
    }

    if !args.is_empty() && args != ["run-canonical"] {
        eprintln!("citus-tool-runtime: unknown command");
        print_usage();
        process::exit(2);
    }

    let snapshot = canonical_snapshot();
    if let Err(error) = snapshot.validate() {
        eprintln!("citus-tool-runtime: canonical snapshot failed: {error}");
        process::exit(1);
    }

    println!(
        "workers\ttables\tshards\ttenants\tvectorizers\tsearch_indexes\tbranches\tbackups\trealtime_streams\tpool"
    );
    println!(
        "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
        snapshot.workers.len(),
        snapshot.tables.len(),
        snapshot.shards.len(),
        snapshot.tenants.len(),
        snapshot.vectorizers.len(),
        snapshot.search_indexes.len(),
        snapshot.branches.len(),
        snapshot.backups.len(),
        snapshot.realtime_streams.len(),
        usize::from(snapshot.pool.is_some()),
    );
}

fn print_usage() {
    println!("usage: citus-tool-runtime [run-canonical]");
    println!("emits the deterministic canonical tools snapshot summary as TSV");
}
