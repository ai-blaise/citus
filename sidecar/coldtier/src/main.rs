// FEATURE: R1
// FEATURE: R5
// FEATURE: R9
// FEATURE: Search8

use ai_blaise_citus_sidecar_coldtier::{
    canonical_cold_tier_plan, canonical_move_plans, ColdTierFormat, StorageTier,
};
use std::env;
use std::process;

fn main() {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print_usage();
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

fn print_usage() {
    println!("usage: coldtier [run-canonical]");
    println!("runs the deterministic canonical cold-tier move plan and emits TSV");
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
