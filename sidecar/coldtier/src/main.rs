// FEATURE: R1
// FEATURE: R5
// FEATURE: R9
// FEATURE: Search8

use ai_blaise_citus_sidecar_coldtier::{
    canonical_cold_tier_plan, canonical_cold_tier_runtime_report, canonical_move_plans,
    ColdTierFormat, StorageTier,
};
use ai_blaise_citus_sidecar_shared::run_probe_server;
use std::env;
use std::process;

fn main() {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print_usage();
        return;
    }

    if args == ["serve"] {
        run_server("coldtier", "0.0.0.0:8080");
        return;
    }

    if args == ["run-runtime-canonical"] {
        run_runtime_canonical();
        return;
    }

    if !args.is_empty() && args != ["run-canonical"] {
        eprintln!("coldtier: unknown command");
        print_usage();
        process::exit(2);
    }

    let plan = canonical_cold_tier_plan();
    let moves = canonical_move_plans().unwrap_or_else(|error| {
        eprintln!("coldtier: canonical move planning failed: {error}");
        process::exit(1);
    });
    let search_columns = plan
        .search
        .as_ref()
        .map(|search| search.indexed_columns.join(","))
        .unwrap_or_default();

    println!("shard_id\ttable\tfrom\tto\tformat\tobject_uri\tlayers\tsearch_columns");
    for move_plan in &moves {
        let shard = plan
            .shards
            .iter()
            .find(|shard| shard.shard_id == move_plan.shard_id)
            .expect("canonical shard");
        println!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            move_plan.shard_id,
            move_plan.table,
            tier_name(&move_plan.from),
            tier_name(&move_plan.to),
            format_name(&shard.format),
            move_plan.object_uri,
            shard.layers.len(),
            search_columns,
        );
    }
}

fn run_runtime_canonical() {
    let report = canonical_cold_tier_runtime_report().unwrap_or_else(|error| {
        eprintln!("coldtier: canonical runtime failed: {error}");
        process::exit(1);
    });
    let search_indexes = report
        .search
        .as_ref()
        .map(|search| search.index_uris.join(","))
        .unwrap_or_default();
    let search_columns = report
        .search
        .as_ref()
        .map(|search| search.indexed_columns.join(","))
        .unwrap_or_default();

    println!(
        "shard_id\ttable\tfrom\tto\tformat\tobject_uri\tlayers\tbytes_moved\timage_layers\tdelta_layers\tsearch_indexes\tsearch_columns\tsearch_indexes_materialized\tplanner_routes_refreshed\tcold_tier_reads\tmoved_shards\tmaterialized_layer_files\tobject_bytes_written"
    );
    for move_execution in &report.moves {
        println!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            move_execution.shard_id,
            move_execution.table,
            tier_name(&move_execution.from),
            tier_name(&move_execution.to),
            format_name(&move_execution.format),
            move_execution.object_uri,
            move_execution.layer_count,
            move_execution.bytes_moved,
            move_execution.image_layers,
            move_execution.delta_layers,
            search_indexes,
            search_columns,
            report.state.search_indexes_materialized,
            report.state.planner_routes_refreshed,
            report.state.cold_tier_reads,
            report.state.moved_shards,
            report.state.materialized_layer_files,
            report.state.object_bytes_written,
        );
    }
}

fn print_usage() {
    println!("usage: coldtier [serve|run-canonical|run-runtime-canonical]");
    println!("runs deterministic canonical cold-tier plan/runtime reports and emits TSV");
}

fn run_server(component: &str, default_addr: &str) {
    if let Err(error) = run_probe_server(component, default_addr) {
        eprintln!("{component}: probe server failed: {error}");
        process::exit(1);
    }
}

fn tier_name(tier: &StorageTier) -> &'static str {
    match tier {
        StorageTier::Hot => "hot",
        StorageTier::Warm => "warm",
        StorageTier::Cold => "cold",
    }
}

fn format_name(format: &ColdTierFormat) -> &'static str {
    match format {
        ColdTierFormat::Iceberg => "iceberg",
        ColdTierFormat::Parquet => "parquet",
    }
}
