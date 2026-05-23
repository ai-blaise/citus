use ai_blaise_citus_companion::canonical_companion_runtime_depth_a_report;
use std::env;
use std::process;

fn main() {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print_usage();
        return;
    }

    match args.as_slice() {
        [] => run_canonical(),
        [command] if command == "run-canonical" => run_canonical(),
        _ => {
            eprintln!("companion-runtime-depth-a: unknown command");
            print_usage();
            process::exit(2);
        }
    }
}

fn run_canonical() {
    let report = canonical_companion_runtime_depth_a_report().unwrap_or_else(|error| {
        eprintln!("companion-runtime-depth-a: canonical report failed: {error}");
        process::exit(1);
    });

    println!(
        "features\tfeature_ids\tmigration_phases\tmigration_sql_batches\tmigration_commands\tqueue_commands\tqueue_leased_messages\tqueue_dead_lettered_messages\tconflict_classes\tconflict_resolutions\tconflict_rejections\tfail_closed_guards"
    );
    println!(
        "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
        report.feature_ids.len(),
        report.feature_ids.join(","),
        report.migration_phases,
        report.migration_sql_batches,
        report.migration_commands,
        report.queue_commands,
        report.queue_leased_messages,
        report.queue_dead_lettered_messages,
        report.conflict_classes,
        report.conflict_resolutions,
        report.conflict_rejections,
        report.fail_closed_guards,
    );
}

fn print_usage() {
    println!("usage: companion_runtime_depth_a [run-canonical]");
    println!("runs deterministic companion migration, queue, and replication-conflict runtime reports as TSV");
}
