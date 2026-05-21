// FEATURE: A9
// FEATURE: A10
// FEATURE: A11
// FEATURE: B4
// FEATURE: D7
// FEATURE: D8
// FEATURE: D9
// FEATURE: D10
// FEATURE: D11
// FEATURE: Edge1
// FEATURE: Edge2
// FEATURE: F3
// FEATURE: F4
// FEATURE: L7
// FEATURE: L10
// FEATURE: M4
// FEATURE: MR3
// FEATURE: MR6
// FEATURE: MR9
// FEATURE: R3
// FEATURE: R8
// FEATURE: R12
// FEATURE: RT5
// FEATURE: S1
// FEATURE: S3
// FEATURE: S7
// FEATURE: S8
// FEATURE: S12
// FEATURE: Sec7
// FEATURE: Sec8
// FEATURE: Sec9
// FEATURE: Sec13
// FEATURE: Sto2
// FEATURE: T4
// FEATURE: T6
// FEATURE: T7
// FEATURE: T10
// FEATURE: T11
// FEATURE: T13
// FEATURE: T14
// FEATURE: TS10
// FEATURE: TS11

use ai_blaise_citus_companion::{
    canonical_advanced_planner_execution_report, canonical_domain_contracts_report,
    canonical_extension_catalog_execution_report, canonical_operations_readiness_report,
};
use std::env;
use std::process;

fn main() {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print_usage();
        return;
    }

    match args.as_slice() {
        [] => run_advanced_planner_canonical(),
        [command] if command == "run-advanced-planner-canonical" => {
            run_advanced_planner_canonical()
        }
        [command] if command == "run-extension-catalog-canonical" => {
            run_extension_catalog_canonical();
        }
        [command] if command == "run-domain-contracts-canonical" => {
            run_domain_contracts_canonical();
        }
        [command] if command == "run-operations-canonical" => {
            run_operations_canonical();
        }
        _ => {
            eprintln!("companion-contracts: unknown command");
            print_usage();
            process::exit(2);
        }
    }
}

fn run_advanced_planner_canonical() {
    let report = canonical_advanced_planner_execution_report().unwrap_or_else(|error| {
        eprintln!("companion-contracts: advanced planner report failed: {error}");
        process::exit(1);
    });

    println!(
        "surfaces\tlookup_surfaces\tlookup_min_partitions\tmax_batch_rows\tdistsql_worker_tasks\ttransaction_state_surfaces\ttransaction_shard_budget\tpolicy_surfaces\tpolicy_required_inputs\tstorage_domains\tresearch_guards"
    );
    println!(
        "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
        report.surface_count,
        report.lookup_surfaces,
        report.lookup_min_partitions,
        report.max_batch_rows,
        report.distributed_sql_worker_tasks,
        report.transaction_state_surfaces,
        report.transaction_shard_budget,
        report.policy_surfaces,
        report.policy_required_inputs,
        report.storage_domains,
        report.research_guards,
    );
}

fn run_domain_contracts_canonical() {
    let report = canonical_domain_contracts_report().unwrap_or_else(|error| {
        eprintln!("companion-contracts: domain contracts report failed: {error}");
        process::exit(1);
    });

    println!("features\tfeature_ids\tsql_plans\tvalidations\tcommands");
    println!(
        "{}\t{}\t{}\t{}\t{}",
        report.feature_ids.len(),
        report.feature_ids.join(","),
        report.sql_plan_count,
        report.validation_count,
        report.command_count,
    );
}

fn run_extension_catalog_canonical() {
    let report = canonical_extension_catalog_execution_report().unwrap_or_else(|error| {
        eprintln!("companion-contracts: extension catalog report failed: {error}");
        process::exit(1);
    });

    println!(
        "contracts\tcovered_feature_ids\tfeature_edges\trequired\toptional\tintegration_targets\tpreloaded"
    );
    println!(
        "{}\t{}\t{}\t{}\t{}\t{}\t{}",
        report.contract_count,
        report.covered_feature_ids,
        report.feature_edges,
        report.required,
        report.optional,
        report.integration_targets,
        report.preloaded,
    );
}

fn run_operations_canonical() {
    let report = canonical_operations_readiness_report().unwrap_or_else(|error| {
        eprintln!("companion-contracts: operations readiness report failed: {error}");
        process::exit(1);
    });

    println!(
        "checks\thelm_renders\tscript_contracts\trunbooks\truntime_toggles\tsecurity_controls\tcompatibility_checks"
    );
    println!(
        "{}\t{}\t{}\t{}\t{}\t{}\t{}",
        report.check_count,
        report.helm_renders,
        report.script_contracts,
        report.runbooks,
        report.runtime_toggles,
        report.security_controls,
        report.compatibility_checks,
    );
}

fn print_usage() {
    println!(
        "usage: companion_contracts [run-advanced-planner-canonical|run-extension-catalog-canonical|run-domain-contracts-canonical|run-operations-canonical]"
    );
    println!("runs deterministic canonical companion contract execution reports and emits TSV");
}
