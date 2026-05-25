// FEATURE: A9
// FEATURE: A10
// FEATURE: A11
// FEATURE: B4
// FEATURE: D7
// FEATURE: D8
// FEATURE: D9
// FEATURE: D10
// FEATURE: D11
// FEATURE: O15
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
    canonical_advanced_planner_execution_report, canonical_advanced_planner_runtime_report,
    canonical_bulk_distsql_report, canonical_bulk_distsql_sql_plan, canonical_clone_node_report,
    canonical_clone_node_sql_plan, canonical_cohabit_detection_report,
    canonical_columnar_tiering_report, canonical_columnar_tiering_sql_plan,
    canonical_domain_contracts_report, canonical_extension_catalog_execution_report,
    canonical_fdw_credential_rotation_report, canonical_fdw_credential_rotation_sql_plan,
    canonical_operations_readiness_report, canonical_plan_runtime_report,
    canonical_regional_placement_report, canonical_regional_placement_sql_plan,
    canonical_regional_row_placement_report, canonical_regional_row_placement_sql_plan,
    canonical_release_hardening_report, canonical_schema_drift_report,
    canonical_schema_drift_sql_plan, canonical_shard_split_report, canonical_shard_split_sql_plan,
    canonical_shard_temperature_ranking_report, canonical_shard_temperature_sql_plan,
    canonical_timescale_advanced_report, canonical_timescale_advanced_sql_plan,
    canonical_transaction_state_report, canonical_transaction_state_sql_plan, render_all_views,
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
        [command] if command == "run-advanced-planner-runtime-canonical" => {
            run_advanced_planner_runtime_canonical();
        }
        [command] if command == "run-columnar-tiering-canonical" => {
            run_columnar_tiering_canonical();
        }
        [command] if command == "run-columnar-tiering-sql-canonical" => {
            run_columnar_tiering_sql_canonical();
        }
        [command] if command == "run-fdw-credential-rotation-canonical" => {
            run_fdw_credential_rotation_canonical();
        }
        [command] if command == "run-fdw-credential-rotation-sql-canonical" => {
            run_fdw_credential_rotation_sql_canonical();
        }
        [command] if command == "run-schema-drift-canonical" => {
            run_schema_drift_canonical();
        }
        [command] if command == "run-schema-drift-sql-canonical" => {
            run_schema_drift_sql_canonical();
        }
        [command] if command == "run-extension-catalog-canonical" => {
            run_extension_catalog_canonical();
        }
        [command] if command == "run-cohabit-detection-canonical" => {
            run_cohabit_detection_canonical();
        }
        [command] if command == "run-domain-contracts-canonical" => {
            run_domain_contracts_canonical();
        }
        [command] if command == "run-operations-canonical" => {
            run_operations_canonical();
        }
        [command] if command == "run-release-hardening-canonical" => {
            run_release_hardening_canonical();
        }
        [command] if command == "run-plan-runtime-canonical" => {
            run_plan_runtime_canonical();
        }
        [command] if command == "run-regional-placement-canonical" => {
            run_regional_placement_canonical();
        }
        [command] if command == "run-regional-placement-sql-canonical" => {
            run_regional_placement_sql_canonical();
        }
        [command] if command == "run-regional-row-placement-canonical" => {
            run_regional_row_placement_canonical();
        }
        [command] if command == "run-regional-row-placement-sql-canonical" => {
            run_regional_row_placement_sql_canonical();
        }
        [command] if command == "run-shard-temperature-ranking-canonical" => {
            run_shard_temperature_ranking_canonical();
        }
        [command] if command == "run-shard-temperature-ranking-sql-canonical" => {
            run_shard_temperature_ranking_sql_canonical();
        }
        [command] if command == "run-shard-split-canonical" => {
            run_shard_split_canonical();
        }
        [command] if command == "run-shard-split-sql-canonical" => {
            run_shard_split_sql_canonical();
        }
        [command] if command == "run-clone-node-canonical" => {
            run_clone_node_canonical();
        }
        [command] if command == "run-clone-node-setup-sql-canonical" => {
            run_clone_node_setup_sql_canonical();
        }
        [command] if command == "run-clone-node-promote-sql-canonical" => {
            run_clone_node_promote_sql_canonical();
        }
        [command] if command == "run-transaction-state-canonical" => {
            run_transaction_state_canonical();
        }
        [command] if command == "run-transaction-state-sql-canonical" => {
            run_transaction_state_sql_canonical();
        }
        [command] if command == "run-bulk-distsql-canonical" => {
            run_bulk_distsql_canonical();
        }
        [command] if command == "run-bulk-distsql-sql-canonical" => {
            run_bulk_distsql_sql_canonical();
        }
        [command] if command == "run-timescale-advanced-canonical" => {
            run_timescale_advanced_canonical();
        }
        [command] if command == "run-timescale-advanced-sql-canonical" => {
            run_timescale_advanced_sql_canonical();
        }
        [command] if command == "run-log-view-sql-canonical" => {
            run_log_view_sql_canonical();
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

fn run_advanced_planner_runtime_canonical() {
    let report = canonical_advanced_planner_runtime_report().unwrap_or_else(|error| {
        eprintln!("companion-contracts: advanced planner runtime report failed: {error}");
        process::exit(1);
    });

    println!(
        "scenarios	covered_features	contract_checks	fail_closed_checks	live_execution_claims	patch_smoke_boundaries	plan_only_boundaries	deterministic_boundaries	research_guard_boundaries"
    );
    println!(
        "{}	{}	{}	{}	{}	{}	{}	{}	{}",
        report.scenario_count,
        report.covered_features,
        report.contract_checks,
        report.fail_closed_checks,
        report.live_execution_claims,
        report.patch_smoke_boundaries,
        report.plan_only_boundaries,
        report.deterministic_boundaries,
        report.research_guard_boundaries,
    );
}

fn run_columnar_tiering_canonical() {
    let report = canonical_columnar_tiering_report().unwrap_or_else(|error| {
        eprintln!("companion-contracts: columnar tiering report failed: {error}");
        process::exit(1);
    });

    println!(
        "feature_ids\thot_table\tcolumnar_table\tdistribution_column\tshard_count\texpected_rows\texpected_total\tmin_worker_placements\tstatements\tjoins_citus_catalog\tchecks_columnar_access_method\tchecks_non_hypertable\tmutating_sql\tfail_closed_checks\tcost_model_selection_exercised\tautomatic_tier_movement_executed\tworkload_routing_exercised\tkubernetes_traffic_exercised"
    );
    println!(
        "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
        report.feature_ids.join(","),
        report.hot_table,
        report.columnar_table,
        report.distribution_column,
        report.shard_count,
        report.expected_rows,
        report.expected_total,
        report.min_worker_placements,
        report.statement_count,
        report.joins_citus_catalog,
        report.checks_columnar_access_method,
        report.checks_non_hypertable,
        report.mutating_sql,
        report.fail_closed_checks,
        report.cost_model_selection_exercised,
        report.automatic_tier_movement_executed,
        report.workload_routing_exercised,
        report.kubernetes_traffic_exercised,
    );
}

fn run_columnar_tiering_sql_canonical() {
    let sql_plan = canonical_columnar_tiering_sql_plan().unwrap_or_else(|error| {
        eprintln!("companion-contracts: columnar tiering SQL render failed: {error}");
        process::exit(1);
    });
    println!("{}", sql_plan.render_psql_script());
}

fn run_fdw_credential_rotation_canonical() {
    let report = canonical_fdw_credential_rotation_report().unwrap_or_else(|error| {
        eprintln!("companion-contracts: FDW credential rotation report failed: {error}");
        process::exit(1);
    });

    println!(
        "feature_id\tserver\tmapping_user\tvalidation_table\tstatements\tdisconnect_calls\tuses_secret_variable\tplan_secret_literals"
    );
    println!(
        "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
        report.feature_id,
        report.server_name,
        report.mapping_user,
        report.validation_table,
        report.statement_count,
        report.disconnect_calls,
        report.uses_secret_variable,
        report.plan_secret_literals,
    );
}

fn run_fdw_credential_rotation_sql_canonical() {
    let sql_plan = canonical_fdw_credential_rotation_sql_plan().unwrap_or_else(|error| {
        eprintln!("companion-contracts: FDW credential rotation SQL render failed: {error}");
        process::exit(1);
    });
    println!("{}", sql_plan.render_psql_script());
}

fn run_schema_drift_canonical() {
    let report = canonical_schema_drift_report().unwrap_or_else(|error| {
        eprintln!("companion-contracts: schema drift report failed: {error}");
        process::exit(1);
    });

    println!(
        "feature_id\texpected_columns\tstatements\tdrift_kinds\tinformation_schema_queries\ttemporary_tables"
    );
    println!(
        "{}\t{}\t{}\t{}\t{}\t{}",
        report.feature_id,
        report.expected_columns,
        report.statement_count,
        report.drift_kinds.join(","),
        report.information_schema_queries,
        report.temporary_tables,
    );
}

fn run_schema_drift_sql_canonical() {
    let sql_plan = canonical_schema_drift_sql_plan().unwrap_or_else(|error| {
        eprintln!("companion-contracts: schema drift SQL render failed: {error}");
        process::exit(1);
    });
    println!("{}", sql_plan.render_psql_script());
}

fn run_cohabit_detection_canonical() {
    let report = canonical_cohabit_detection_report();

    println!("extension	role	configured	preloaded	installed	ready	reason");
    for observation in &report.observations {
        println!(
            "{}	{}	{}	{}	{}	{}	{}",
            observation.name,
            observation.role.as_str(),
            observation.configured,
            observation.preloaded,
            observation.installed,
            observation.ready,
            observation.reason.as_deref().unwrap_or("ok"),
        );
    }
    println!(
        "summary	detected={}	ready={}	hard_failures={}	unsupported={}",
        report.detected,
        report.ready,
        report.hard_failures,
        report.unsupported_configured_extensions.len(),
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

fn run_release_hardening_canonical() {
    let report = canonical_release_hardening_report().unwrap_or_else(|error| {
        eprintln!("companion-contracts: release hardening report failed: {error}");
        process::exit(1);
    });

    println!(
        "feature_id\trequired_gates\trelease_record_fields\tproduction_release_block_required\towner_signoff_required\trollback_evidence_required\tproduction_gap_audit_required\trunbook_command_check_required"
    );
    println!(
        "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
        report.feature_id,
        report.required_gates,
        report.release_record_fields,
        report.production_release_block_required,
        report.owner_signoff_required,
        report.rollback_evidence_required,
        report.production_gap_audit_required,
        report.runbook_command_check_required,
    );
}

fn run_plan_runtime_canonical() {
    let report = canonical_plan_runtime_report().unwrap_or_else(|error| {
        eprintln!("companion-contracts: plan runtime report failed: {error}");
        process::exit(1);
    });

    println!(
        "records\tpromoted\tobservations\taudit_events\tidempotent_replays\tretry_attempts\tfailed_commands\tregression_violations\tsql_contract_commands"
    );
    println!(
        "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
        report.records,
        report.promoted,
        report.observations,
        report.audit_events,
        report.idempotent_replays,
        report.retry_attempts,
        report.failed_commands,
        report.regression_violations,
        report.sql_contract_commands,
    );
}

fn run_regional_placement_canonical() {
    let report = canonical_regional_placement_report().unwrap_or_else(|error| {
        eprintln!("companion-contracts: regional placement report failed: {error}");
        process::exit(1);
    });

    println!(
        "feature_ids\tlocality_table\tlocality_column\ttenant_column\tpk_prefix_columns\tregion_tablespaces\tstatement_count\tcatalog_tables\tread_only_sql\tfail_closed_checks\tautomatic_rebalance_executed\tshard_movement_executed"
    );
    println!(
        "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
        report.feature_ids.join(","),
        report.locality_table,
        report.locality_column,
        report.tenant_column,
        report.pk_prefix_columns,
        report.region_tablespace_count,
        report.statement_count,
        report.catalog_tables.join(","),
        report.read_only_sql,
        report.fail_closed_checks,
        report.automatic_rebalance_executed,
        report.shard_movement_executed,
    );
}

fn run_regional_placement_sql_canonical() {
    let sql_plan = canonical_regional_placement_sql_plan().unwrap_or_else(|error| {
        eprintln!("companion-contracts: regional placement SQL render failed: {error}");
        process::exit(1);
    });
    println!("{}", sql_plan.render_psql_script());
}

fn run_regional_row_placement_canonical() {
    let report = canonical_regional_row_placement_report().unwrap_or_else(|error| {
        eprintln!("companion-contracts: regional row placement report failed: {error}");
        process::exit(1);
    });

    println!(
        "feature_id\ttable_name\tlocality_column\tregional_keys\tstatements\tuses_isolate_tenant_to_new_shard\tuses_citus_move_shard_placement\trecords_row_preservation\trecords_worker_placement\trequires_worker_psql_variables\tfail_closed_checks\tautomatic_repartition_scheduler_exercised\tkubernetes_operator_reconciliation_exercised\tregional_traffic_router_exercised\tmulti_region_network_exercised\tregional_failover_exercised"
    );
    println!(
        "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
        report.feature_id,
        report.table_name,
        report.locality_column,
        report.regional_key_count,
        report.statement_count,
        report.uses_isolate_tenant_to_new_shard,
        report.uses_citus_move_shard_placement,
        report.records_row_preservation,
        report.records_worker_placement,
        report.requires_worker_psql_variables,
        report.fail_closed_checks,
        report.automatic_repartition_scheduler_exercised,
        report.kubernetes_operator_reconciliation_exercised,
        report.regional_traffic_router_exercised,
        report.multi_region_network_exercised,
        report.regional_failover_exercised,
    );
}

fn run_regional_row_placement_sql_canonical() {
    let sql_plan = canonical_regional_row_placement_sql_plan().unwrap_or_else(|error| {
        eprintln!("companion-contracts: regional row placement SQL render failed: {error}");
        process::exit(1);
    });
    println!("{}", sql_plan.render_psql_script());
}

fn run_shard_temperature_ranking_canonical() {
    let report = canonical_shard_temperature_ranking_report().unwrap_or_else(|error| {
        eprintln!("companion-contracts: shard temperature report failed: {error}");
        process::exit(1);
    });

    println!(
        "feature_id\tmetrics_table\tstatements\tjoins_citus_catalog\tranks_shards\ttarget_tiers\tfail_closed_checks\tautomatic_tier_movement\tcoldtier_moves_executed"
    );
    println!(
        "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
        report.feature_id,
        report.metrics_table,
        report.statement_count,
        report.joins_citus_catalog,
        report.ranks_shards,
        report.target_tiers.join(","),
        report.fail_closed_checks,
        report.automatic_tier_movement,
        report.coldtier_moves_executed,
    );
}

fn run_shard_temperature_ranking_sql_canonical() {
    let sql_plan = canonical_shard_temperature_sql_plan().unwrap_or_else(|error| {
        eprintln!("companion-contracts: shard temperature SQL render failed: {error}");
        process::exit(1);
    });
    println!("{}", sql_plan.render_psql_script());
}

fn run_shard_split_canonical() {
    let report = canonical_shard_split_report().unwrap_or_else(|error| {
        eprintln!("companion-contracts: shard split report failed: {error}");
        process::exit(1);
    });

    println!(
        "feature_id	table	tenant_id	initial_shard_count	statements	uses_isolate_tenant_to_new_shard	requires_logical_wal	records_shard_count_before_after	records_row_preservation	records_isolated_range	fail_closed_checks	policy_scheduler_exercised	threshold_telemetry_exercised	rollback_automation_exercised	multi_node_movement_exercised	kubernetes_traffic_exercised"
    );
    println!(
        "{}	{}	{}	{}	{}	{}	{}	{}	{}	{}	{}	{}	{}	{}	{}	{}",
        report.feature_id,
        report.table_name,
        report.tenant_id,
        report.initial_shard_count,
        report.statement_count,
        report.uses_isolate_tenant_to_new_shard,
        report.requires_logical_wal,
        report.records_shard_count_before_after,
        report.records_row_preservation,
        report.records_isolated_range,
        report.fail_closed_checks,
        report.policy_scheduler_exercised,
        report.threshold_telemetry_exercised,
        report.rollback_automation_exercised,
        report.multi_node_movement_exercised,
        report.kubernetes_traffic_exercised,
    );
}

fn run_shard_split_sql_canonical() {
    let sql_plan = canonical_shard_split_sql_plan().unwrap_or_else(|error| {
        eprintln!("companion-contracts: shard split SQL render failed: {error}");
        process::exit(1);
    });
    println!("{}", sql_plan.render_psql_script());
}

fn run_clone_node_canonical() {
    let report = canonical_clone_node_report().unwrap_or_else(|error| {
        eprintln!("companion-contracts: clone node report failed: {error}");
        process::exit(1);
    });

    println!(
        "feature_id	table	shard_count	expected_rows	expected_sum	catchup_timeout_seconds	setup_statements	promote_statements	uses_citus_add_clone_node	uses_citus_promote_clone_and_rebalance	records_data_preservation	records_clone_metadata	registers_primary_worker	fail_closed_checks	kubernetes_clone_orchestration_exercised	csi_snapshot_exercised	automatic_capacity_policy_exercised	production_traffic_cutover_exercised"
    );
    println!(
        "{}	{}	{}	{}	{}	{}	{}	{}	{}	{}	{}	{}	{}	{}	{}	{}	{}	{}",
        report.feature_id,
        report.table_name,
        report.shard_count,
        report.expected_rows,
        report.expected_sum,
        report.catchup_timeout_seconds,
        report.setup_statement_count,
        report.promote_statement_count,
        report.uses_citus_add_clone_node,
        report.uses_citus_promote_clone_and_rebalance,
        report.records_data_preservation,
        report.records_clone_metadata,
        report.registers_primary_worker,
        report.fail_closed_checks,
        report.kubernetes_clone_orchestration_exercised,
        report.csi_snapshot_exercised,
        report.automatic_capacity_policy_exercised,
        report.production_traffic_cutover_exercised,
    );
}

fn run_clone_node_setup_sql_canonical() {
    let sql_plan = canonical_clone_node_sql_plan().unwrap_or_else(|error| {
        eprintln!("companion-contracts: clone node setup SQL render failed: {error}");
        process::exit(1);
    });
    println!("{}", sql_plan.render_setup_psql_script());
}

fn run_clone_node_promote_sql_canonical() {
    let sql_plan = canonical_clone_node_sql_plan().unwrap_or_else(|error| {
        eprintln!("companion-contracts: clone node promote SQL render failed: {error}");
        process::exit(1);
    });
    println!("{}", sql_plan.render_promote_psql_script());
}

fn run_transaction_state_canonical() {
    let report = canonical_transaction_state_report().unwrap_or_else(|error| {
        eprintln!("companion-contracts: transaction state report failed: {error}");
        process::exit(1);
    });

    println!(
        "feature_ids\ttable\tstatements\tcursor_declared\tcursor_fetches\tsavepoint_declared\trollback_to_savepoint\tcitus_explain_required\tfetch_batch_rows\tfail_closed_checks\tcoordinator_failover_exercised\tmulti_worker_cleanup_exercised"
    );
    println!(
        "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
        report.feature_ids.join(","),
        report.table_name,
        report.statement_count,
        report.cursor_declared,
        report.cursor_fetches,
        report.savepoint_declared,
        report.rollback_to_savepoint,
        report.citus_explain_required,
        report.fetch_batch_rows,
        report.fail_closed_checks,
        report.coordinator_failover_exercised,
        report.multi_worker_cleanup_exercised,
    );
}

fn run_transaction_state_sql_canonical() {
    let sql_plan = canonical_transaction_state_sql_plan().unwrap_or_else(|error| {
        eprintln!("companion-contracts: transaction state SQL render failed: {error}");
        process::exit(1);
    });
    println!("{}", sql_plan.render_psql_script());
}

fn run_bulk_distsql_canonical() {
    let report = canonical_bulk_distsql_report().unwrap_or_else(|error| {
        eprintln!("companion-contracts: bulk/DistSQL report failed: {error}");
        process::exit(1);
    });

    println!(
        "feature_ids	table	statements	cursor_declared	bulk_fetch_budget_enforced	distsql_explain_required	max_batch_rows	worker_task_budget	fail_closed_checks	wire_protocol_implementation_exercised	backpressure_scheduler_exercised	physical_plan_rewrite_exercised	multi_worker_fanout_exercised"
    );
    println!(
        "{}	{}	{}	{}	{}	{}	{}	{}	{}	{}	{}	{}	{}",
        report.feature_ids.join(","),
        report.table_name,
        report.statement_count,
        report.cursor_declared,
        report.bulk_fetch_budget_enforced,
        report.distsql_explain_required,
        report.max_batch_rows,
        report.worker_task_budget,
        report.fail_closed_checks,
        report.wire_protocol_implementation_exercised,
        report.backpressure_scheduler_exercised,
        report.physical_plan_rewrite_exercised,
        report.multi_worker_fanout_exercised,
    );
}

fn run_bulk_distsql_sql_canonical() {
    let sql_plan = canonical_bulk_distsql_sql_plan().unwrap_or_else(|error| {
        eprintln!("companion-contracts: bulk/DistSQL SQL render failed: {error}");
        process::exit(1);
    });
    println!("{}", sql_plan.render_psql_script());
}

fn run_timescale_advanced_canonical() {
    let report = canonical_timescale_advanced_report().unwrap_or_else(|error| {
        eprintln!("companion-contracts: Timescale advanced report failed: {error}");
        process::exit(1);
    });

    println!(
        "feature_ids	base_table	source_cagg	target_cagg	bloom_table	statements	hierarchical_cagg_refresh_required	compression_segmentby_required	bloom_filter_materialized	bloom_bit_count	bloom_hash_count	fail_closed_checks	native_timescale_bloom_filter_claimed	planner_integration_exercised	multi_worker_fanout_exercised"
    );
    println!(
        "{}	{}	{}	{}	{}	{}	{}	{}	{}	{}	{}	{}	{}	{}	{}",
        report.feature_ids.join(","),
        report.base_table,
        report.source_cagg,
        report.target_cagg,
        report.bloom_table,
        report.statement_count,
        report.hierarchical_cagg_refresh_required,
        report.compression_segmentby_required,
        report.bloom_filter_materialized,
        report.bloom_bit_count,
        report.bloom_hash_count,
        report.fail_closed_checks,
        report.native_timescale_bloom_filter_claimed,
        report.planner_integration_exercised,
        report.multi_worker_fanout_exercised,
    );
}

fn run_timescale_advanced_sql_canonical() {
    let sql_plan = canonical_timescale_advanced_sql_plan().unwrap_or_else(|error| {
        eprintln!("companion-contracts: Timescale advanced SQL render failed: {error}");
        process::exit(1);
    });
    println!("{}", sql_plan.render_psql_script());
}

fn run_log_view_sql_canonical() {
    let sql = render_all_views().unwrap_or_else(|error| {
        eprintln!("companion-contracts: log-view SQL render failed: {error}");
        process::exit(1);
    });
    println!("{sql}");
}

fn print_usage() {
    println!(
        "usage: companion_contracts [run-advanced-planner-canonical|run-advanced-planner-runtime-canonical|run-columnar-tiering-canonical|run-columnar-tiering-sql-canonical|run-fdw-credential-rotation-canonical|run-fdw-credential-rotation-sql-canonical|run-schema-drift-canonical|run-schema-drift-sql-canonical|run-extension-catalog-canonical|run-cohabit-detection-canonical|run-domain-contracts-canonical|run-operations-canonical|run-release-hardening-canonical|run-plan-runtime-canonical|run-regional-placement-canonical|run-regional-placement-sql-canonical|run-regional-row-placement-canonical|run-regional-row-placement-sql-canonical|run-shard-temperature-ranking-canonical|run-shard-temperature-ranking-sql-canonical|run-shard-split-canonical|run-shard-split-sql-canonical|run-clone-node-canonical|run-clone-node-setup-sql-canonical|run-clone-node-promote-sql-canonical|run-transaction-state-canonical|run-transaction-state-sql-canonical|run-bulk-distsql-canonical|run-bulk-distsql-sql-canonical|run-timescale-advanced-canonical|run-timescale-advanced-sql-canonical|run-log-view-sql-canonical]"
    );
    println!("runs deterministic canonical companion contract execution reports, SQL, and TSV");
}
