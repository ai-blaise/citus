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
    canonical_analytical_execution_plan, canonical_analytical_runtime_report,
    canonical_duckdb_extension_catalog_report, materialize_test_decoding_mirror_to_local_artifact,
    AnalyticalEngine, FederationTarget, LakehouseFormat,
};
use ai_blaise_citus_sidecar_shared::run_probe_server;
use std::env;
use std::io::{self, Read};
use std::path::PathBuf;
use std::process;

fn main() {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print_usage();
        return;
    }
    if args == ["serve"] {
        run_server("analytical", "0.0.0.0:8080");
        return;
    }

    if args == ["run-runtime-canonical"] {
        run_runtime_canonical();
        return;
    }

    if args == ["run-logical-mirror-materialization-from-stdin"] {
        run_logical_mirror_materialization_from_stdin();
        return;
    }

    if args == ["run-duckdb-extension-catalog-canonical"] {
        run_duckdb_extension_catalog_canonical();
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

fn run_runtime_canonical() {
    let report = canonical_analytical_runtime_report().unwrap_or_else(|error| {
        eprintln!("analytical: canonical runtime failed: {error}");
        process::exit(1);
    });

    let snapshot_id = report
        .snapshot_commit
        .as_ref()
        .map(|snapshot| snapshot.snapshot_id.as_str())
        .unwrap_or("none");
    let federated_catalogs = report
        .federated_catalogs
        .iter()
        .map(|catalog| catalog.catalog.as_str())
        .collect::<Vec<_>>()
        .join(",");
    let federation_targets = report
        .federated_catalogs
        .iter()
        .map(|catalog| federation_target_name(&catalog.target))
        .collect::<Vec<_>>()
        .join(",");

    let datafusion = report.datafusion_execution.as_ref();
    println!(
        "mirror\tengine\ttable\tformat\tobject_uri\tpushdown_plan\tprojected_columns\tpredicates\tpushed_down\tlimit\testimated_rows\tsnapshot_id\tfederated_catalogs\tfederation_targets\tduckdb_extensions\tmotherduck\tmirrored_cdc_events\tlakehouse_reads\tpushed_down_plans\tsnapshot_commits\tfederated_catalog_publications\tduckdb_extension_loads\tmotherduck_sessions\tquery_engine_executions\tquery_engine_output_rows\tdatafusion_output_rows\tdatafusion_output_total\tprojection_pushdown_executed\tfilter_pushdown_executed\tlimit_pushdown_executed\tallowed_engines\tallowed_object_uri_schemes\tmax_pushdown_limit\texternal_io_enabled\texternal_io_attempted\tquery_engine_executed\tevidence_boundary"
    );
    let row = vec![
        report.read.mirror_name,
        engine_name(&report.read.engine).to_string(),
        report.read.table,
        format_name(&report.read.format).to_string(),
        report.read.object_uri,
        report.read.pushdown_plan_id,
        report.read.projected_columns.join(","),
        report.read.predicates.join(","),
        report.read.pushed_down.to_string(),
        report
            .read
            .limit
            .map(|limit| limit.to_string())
            .unwrap_or_else(|| "none".to_string()),
        report.read.estimated_rows.to_string(),
        snapshot_id.to_string(),
        federated_catalogs,
        federation_targets,
        report.duckdb_extensions.join(","),
        report
            .motherduck_database
            .as_deref()
            .unwrap_or("none")
            .to_string(),
        report.read.mirrored_cdc_events.to_string(),
        report.state.lakehouse_reads.to_string(),
        report.state.pushed_down_plans.to_string(),
        report.state.snapshot_commits.to_string(),
        report.state.federated_catalog_publications.to_string(),
        report.state.duckdb_extension_loads.to_string(),
        report.state.motherduck_sessions.to_string(),
        report.state.query_engine_executions.to_string(),
        report.state.query_engine_output_rows.to_string(),
        datafusion
            .map(|execution| execution.output_rows.to_string())
            .unwrap_or_else(|| "0".to_string()),
        datafusion
            .map(|execution| execution.output_total.to_string())
            .unwrap_or_else(|| "0".to_string()),
        datafusion
            .map(|execution| execution.projection_pushdown_executed.to_string())
            .unwrap_or_else(|| "false".to_string()),
        datafusion
            .map(|execution| execution.filter_pushdown_executed.to_string())
            .unwrap_or_else(|| "false".to_string()),
        datafusion
            .map(|execution| execution.limit_pushdown_executed.to_string())
            .unwrap_or_else(|| "false".to_string()),
        report
            .runtime_policy
            .allowed_engines
            .iter()
            .map(engine_name)
            .collect::<Vec<_>>()
            .join(","),
        report.runtime_policy.allowed_object_uri_schemes.join(","),
        report.runtime_policy.max_pushdown_limit.to_string(),
        report.runtime_policy.external_io_enabled.to_string(),
        report.external_io_attempted.to_string(),
        report.query_engine_executed.to_string(),
        report.evidence_boundary,
    ];
    println!("{}", row.join("\t"));
}

fn run_duckdb_extension_catalog_canonical() {
    let report = canonical_duckdb_extension_catalog_report().unwrap_or_else(|error| {
        eprintln!("analytical: DuckDB extension catalog report failed: {error}");
        process::exit(1);
    });

    println!(
        "feature_id\tallowed_extensions\tallowed_extension_count\tinstall_sql\tload_sql\texternal_io_attempted\tpg_duckdb_runtime_exercised\tmotherduck_session_exercised\tevidence_boundary"
    );
    let row = vec![
        report.feature_id.to_string(),
        report.allowed_extensions.join(","),
        report.allowed_extension_count.to_string(),
        report.install_sql.join(";"),
        report.load_sql.join(";"),
        report.external_io_attempted.to_string(),
        report.pg_duckdb_runtime_exercised.to_string(),
        report.motherduck_session_exercised.to_string(),
        report.evidence_boundary.to_string(),
    ];
    println!("{}", row.join("\t"));
}

fn run_logical_mirror_materialization_from_stdin() {
    let artifact_path = env::var("AI_BLAISE_ANALYTICAL_MIRROR_ARTIFACT")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp/ai-blaise-l8-mirror.tsv"));
    let mut decoded_changes = String::new();
    io::stdin()
        .read_to_string(&mut decoded_changes)
        .unwrap_or_else(|error| {
            eprintln!("analytical: failed to read decoded logical stream from stdin: {error}");
            process::exit(1);
        });

    let report =
        materialize_test_decoding_mirror_to_local_artifact(&decoded_changes, &artifact_path)
            .unwrap_or_else(|error| {
                eprintln!("analytical: logical mirror materialization failed: {error}");
                process::exit(1);
            });

    println!(
        "feature_id\tmirror\tsource_table\tsource_plugin\tdecoded_change_lines\tmaterialized_rows\tmaterialized_total\tartifact_path\tartifact_bytes\tdatafusion_query_executed\tdatafusion_output_rows\tdatafusion_output_total\tlocal_mirror_artifact_created\tobject_store_io_attempted\tlong_running_slot_tailing\tcheckpoint_persistence_exercised\tkubernetes_traffic_exercised"
    );
    let row = vec![
        report.feature_id.to_string(),
        report.mirror_name,
        report.source_table,
        report.source_plugin,
        report.decoded_change_lines.to_string(),
        report.materialized_rows.to_string(),
        report.materialized_total.to_string(),
        report.artifact_path,
        report.artifact_bytes.to_string(),
        report.datafusion_query_executed.to_string(),
        report.datafusion_output_rows.to_string(),
        report.datafusion_output_total.to_string(),
        report.local_mirror_artifact_created.to_string(),
        report.object_store_io_attempted.to_string(),
        report.long_running_slot_tailing.to_string(),
        report.checkpoint_persistence_exercised.to_string(),
        report.kubernetes_traffic_exercised.to_string(),
    ];
    println!("{}", row.join("\t"));
}

fn print_usage() {
    println!("usage: analytical [serve|run-canonical|run-runtime-canonical|run-logical-mirror-materialization-from-stdin|run-duckdb-extension-catalog-canonical]");
    println!("runs deterministic canonical analytical sidecar plan/runtime reports and emits TSV");
}

fn run_server(component: &str, default_addr: &str) {
    if let Err(error) = run_probe_server(component, default_addr) {
        eprintln!("{component}: probe server failed: {error}");
        process::exit(1);
    }
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
