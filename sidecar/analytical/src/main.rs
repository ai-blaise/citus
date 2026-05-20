// FEATURE: L1
// FEATURE: L2
// FEATURE: L3
// FEATURE: L4
// FEATURE: L5
// FEATURE: L6
// FEATURE: L8
// FEATURE: L12
// FEATURE: L13

use ai_blaise_citus_sidecar_analytical::{
    canonical_analytical_execution_plan, AnalyticalEngine, FederationTarget, LakehouseFormat,
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
        eprintln!("analytical: unknown command");
        print_usage();
        process::exit(2);
    }

    let plan = canonical_analytical_execution_plan().unwrap_or_else(|error| {
        eprintln!("analytical: canonical plan failed: {error}");
        process::exit(1);
    });
    let snapshot = plan.snapshot_commit.as_ref();
    let catalog = &plan.federated_catalogs[0];

    println!(
        "mirror\tengine\ttable\tformat\tobject_uri\tprojected_columns\tpredicates\tpushdown_plan\tlimit\tsnapshot_id\tfederated_catalog\tfederation_target\tmotherduck"
    );
    println!(
        "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
        plan.mirror.mirror_name,
        engine_name(&plan.engine),
        plan.lakehouse.table,
        format_name(&plan.lakehouse.format),
        plan.lakehouse.object_uri,
        plan.lakehouse.projected_columns.join(","),
        plan.lakehouse.predicates.join(","),
        plan.pushdown.plan_id,
        plan.pushdown
            .limit
            .map(|limit| limit.to_string())
            .unwrap_or_else(|| "none".to_string()),
        snapshot
            .map(|commit| commit.snapshot_id.as_str())
            .unwrap_or("none"),
        catalog.name,
        federation_target_name(&catalog.target),
        plan.motherduck
            .as_ref()
            .map(|connector| connector.database.as_str())
            .unwrap_or("none"),
    );
}

fn print_usage() {
    println!("usage: analytical [run-canonical]");
    println!("runs the deterministic canonical analytical sidecar plan and emits TSV");
}

fn engine_name(engine: &AnalyticalEngine) -> &'static str {
    match engine {
        AnalyticalEngine::PgLake => "pg_lake",
        AnalyticalEngine::DataFusion => "datafusion",
        AnalyticalEngine::DuckDb => "duckdb",
    }
}

fn format_name(format: &LakehouseFormat) -> &'static str {
    match format {
        LakehouseFormat::Iceberg => "iceberg",
        LakehouseFormat::Parquet => "parquet",
        LakehouseFormat::Delta => "delta",
    }
}

fn federation_target_name(target: &FederationTarget) -> &'static str {
    match target {
        FederationTarget::Snowflake => "snowflake",
        FederationTarget::Trino => "trino",
        FederationTarget::Spark => "spark",
        FederationTarget::Databricks => "databricks",
    }
}
