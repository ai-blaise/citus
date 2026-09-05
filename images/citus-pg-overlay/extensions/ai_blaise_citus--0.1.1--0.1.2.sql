-- FEATURE: D9
-- Security floor: rollback requires a pre-upgrade backup/PITR, not PUBLIC grants.
SET LOCAL search_path = pg_catalog, pg_temp;

DO $privileges$
DECLARE
    routine record;
BEGIN
    IF EXISTS (
        SELECT FROM pg_catalog.pg_proc AS p
        JOIN pg_catalog.pg_depend AS d
          ON d.classid = 'pg_catalog.pg_proc'::pg_catalog.regclass
         AND d.objid = p.oid AND d.deptype = 'e'
        JOIN pg_catalog.pg_extension AS e
          ON d.refclassid = 'pg_catalog.pg_extension'::pg_catalog.regclass
         AND d.refobjid = e.oid
        CROSS JOIN LATERAL pg_catalog.aclexplode(p.proacl) AS acl
        WHERE e.extname = 'ai_blaise_citus' AND acl.grantee NOT IN (0, p.proowner)
    ) THEN
        RAISE EXCEPTION USING ERRCODE = '55000',
            MESSAGE = 'explicit routine grants require the privilege-preserving 0.1.2 upgrade',
            HINT = 'Run upgrades/ai_blaise_citus--0.1.2.sql in one administrative psql session; see the upgrade runbook.';
    END IF;
    FOR routine IN
        SELECT n.nspname, p.proname, pg_catalog.pg_get_function_identity_arguments(p.oid) AS arguments
        FROM pg_catalog.pg_proc AS p
        JOIN pg_catalog.pg_namespace AS n ON n.oid = p.pronamespace
        JOIN pg_catalog.pg_depend AS d
          ON d.classid = 'pg_catalog.pg_proc'::pg_catalog.regclass
         AND d.objid = p.oid AND d.objsubid = 0 AND d.deptype = 'e'
        JOIN pg_catalog.pg_extension AS e
          ON d.refclassid = 'pg_catalog.pg_extension'::pg_catalog.regclass
         AND d.refobjid = e.oid
        WHERE e.extname = 'ai_blaise_citus'
        ORDER BY n.nspname, p.proname, arguments
    LOOP
        EXECUTE pg_catalog.format('REVOKE ALL ON ROUTINE %I.%I(%s) FROM PUBLIC',
                                  routine.nspname, routine.proname, routine.arguments);
    END LOOP;
END
$privileges$;

-- These tables are empty at installation. Preserve all operator and tenant state.
SELECT pg_catalog.pg_extension_config_dump('companion_internal.txn_status_records', '');
SELECT pg_catalog.pg_extension_config_dump('companion_internal.timescale_bridge_state', '');
SELECT pg_catalog.pg_extension_config_dump('companion_internal.shard_placement_generations', '');
SELECT pg_catalog.pg_extension_config_dump('companion_internal.plan_freezes', '');
SELECT pg_catalog.pg_extension_config_dump('companion_internal.plan_promotion_policies', '');
SELECT pg_catalog.pg_extension_config_dump('companion_internal.plan_regression_policies', '');
SELECT pg_catalog.pg_extension_config_dump('companion_internal.plan_regression_samples', '');
SELECT pg_catalog.pg_extension_config_dump('companion_internal.migration_runs', '');
SELECT pg_catalog.pg_extension_config_dump('companion_internal.migration_operations', '');
SELECT pg_catalog.pg_extension_config_dump('companion_internal.migration_invariant_checks', '');
SELECT pg_catalog.pg_extension_config_dump('companion_internal.index_advisor_candidates', '');
SELECT pg_catalog.pg_extension_config_dump('companion_internal.webhook_registrations', '');
SELECT pg_catalog.pg_extension_config_dump('companion_internal.webhook_triggers', '');
SELECT pg_catalog.pg_extension_config_dump('companion_internal.webhook_events', '');
SELECT pg_catalog.pg_extension_config_dump('companion_internal.search_worker_indexes', '');
SELECT pg_catalog.pg_extension_config_dump('companion_internal.search_documents', '');
SELECT pg_catalog.pg_extension_config_dump('companion_internal.search_rerank_requests', '');
SELECT pg_catalog.pg_extension_config_dump('companion_internal.graph_colocations', '');
SELECT pg_catalog.pg_extension_config_dump('companion_internal.graphql_distributed_graphs', '');
SELECT pg_catalog.pg_extension_config_dump('companion_internal.json_schemas', '');
SELECT pg_catalog.pg_extension_config_dump('companion_internal.jsonschema_triggers', '');
SELECT pg_catalog.pg_extension_config_dump('companion_internal.geo_distributions', '');
SELECT pg_catalog.pg_extension_config_dump('companion_internal.geo_pruning_policies', '');
SELECT pg_catalog.pg_extension_config_dump('companion_internal.vectorizer_definitions', '');
SELECT pg_catalog.pg_extension_config_dump('companion_internal.vectorizer_usage_log', '');
SELECT pg_catalog.pg_extension_config_dump('companion_internal.ai_provider_bindings', '');
SELECT pg_catalog.pg_extension_config_dump('companion_internal.semantic_catalog_objects', '');
SELECT pg_catalog.pg_extension_config_dump('companion_internal.db_doctor_rules', '');
SELECT pg_catalog.pg_extension_config_dump('companion_internal.db_doctor_violations', '');
SELECT pg_catalog.pg_extension_config_dump('companion_internal.toolkit_aggregate_plans', '');
SELECT pg_catalog.pg_extension_config_dump('companion_internal.schema_jobs', '');
SELECT pg_catalog.pg_extension_config_dump('companion_internal.schema_job_operations', '');
SELECT pg_catalog.pg_extension_config_dump('companion_internal.tenant_moves', '');
SELECT pg_catalog.pg_extension_config_dump('companion_internal.tenant_quotas', '');
SELECT pg_catalog.pg_extension_config_dump('companion_internal.tenant_archives', '');
SELECT pg_catalog.pg_extension_config_dump('companion_internal.tenant_region_affinities', '');
SELECT pg_catalog.pg_extension_config_dump('companion_internal.extension_catalog_contracts', '');
SELECT pg_catalog.pg_extension_config_dump('storage.file_attachment_refs', '');
SELECT pg_catalog.pg_extension_config_dump('companion_internal.ledger_entries', '');
SELECT pg_catalog.pg_extension_config_dump('companion_internal.ledger_seals', '');
SELECT pg_catalog.pg_extension_config_dump('companion_internal.schema_job_phase_log', '');
SELECT pg_catalog.pg_extension_config_dump('companion_internal.worker_schema_lease', '');
SELECT pg_catalog.pg_extension_config_dump('companion_internal.cluster_alarms', '');
SELECT pg_catalog.pg_extension_config_dump('companion_internal.extension_upgrade_events', '');

-- A table's dump registration does not preserve its serial sequence state.
SELECT pg_catalog.pg_extension_config_dump(pg_catalog.pg_get_serial_sequence('companion_internal.txn_status_records', 'raft_index')::pg_catalog.regclass, '');
SELECT pg_catalog.pg_extension_config_dump(pg_catalog.pg_get_serial_sequence('companion_internal.timescale_bridge_state', 'bridge_id')::pg_catalog.regclass, '');
SELECT pg_catalog.pg_extension_config_dump(pg_catalog.pg_get_serial_sequence('companion_internal.plan_regression_samples', 'sample_id')::pg_catalog.regclass, '');
SELECT pg_catalog.pg_extension_config_dump(pg_catalog.pg_get_serial_sequence('companion_internal.migration_operations', 'operation_id')::pg_catalog.regclass, '');
SELECT pg_catalog.pg_extension_config_dump(pg_catalog.pg_get_serial_sequence('companion_internal.index_advisor_candidates', 'candidate_id')::pg_catalog.regclass, '');
SELECT pg_catalog.pg_extension_config_dump(pg_catalog.pg_get_serial_sequence('companion_internal.webhook_triggers', 'trigger_id')::pg_catalog.regclass, '');
SELECT pg_catalog.pg_extension_config_dump(pg_catalog.pg_get_serial_sequence('companion_internal.webhook_events', 'event_id')::pg_catalog.regclass, '');
SELECT pg_catalog.pg_extension_config_dump(pg_catalog.pg_get_serial_sequence('companion_internal.search_documents', 'document_id')::pg_catalog.regclass, '');
SELECT pg_catalog.pg_extension_config_dump(pg_catalog.pg_get_serial_sequence('companion_internal.search_rerank_requests', 'request_id')::pg_catalog.regclass, '');
SELECT pg_catalog.pg_extension_config_dump(pg_catalog.pg_get_serial_sequence('companion_internal.graph_colocations', 'colocation_id')::pg_catalog.regclass, '');
SELECT pg_catalog.pg_extension_config_dump(pg_catalog.pg_get_serial_sequence('companion_internal.jsonschema_triggers', 'trigger_id')::pg_catalog.regclass, '');
SELECT pg_catalog.pg_extension_config_dump(pg_catalog.pg_get_serial_sequence('companion_internal.geo_pruning_policies', 'policy_id')::pg_catalog.regclass, '');
SELECT pg_catalog.pg_extension_config_dump(pg_catalog.pg_get_serial_sequence('companion_internal.vectorizer_usage_log', 'usage_id')::pg_catalog.regclass, '');
SELECT pg_catalog.pg_extension_config_dump(pg_catalog.pg_get_serial_sequence('companion_internal.db_doctor_violations', 'violation_id')::pg_catalog.regclass, '');
SELECT pg_catalog.pg_extension_config_dump(pg_catalog.pg_get_serial_sequence('companion_internal.toolkit_aggregate_plans', 'plan_id')::pg_catalog.regclass, '');
SELECT pg_catalog.pg_extension_config_dump(pg_catalog.pg_get_serial_sequence('companion_internal.schema_job_operations', 'operation_id')::pg_catalog.regclass, '');
SELECT pg_catalog.pg_extension_config_dump(pg_catalog.pg_get_serial_sequence('companion_internal.tenant_moves', 'move_id')::pg_catalog.regclass, '');
SELECT pg_catalog.pg_extension_config_dump(pg_catalog.pg_get_serial_sequence('companion_internal.tenant_archives', 'archive_id')::pg_catalog.regclass, '');
SELECT pg_catalog.pg_extension_config_dump(pg_catalog.pg_get_serial_sequence('storage.file_attachment_refs', 'ref_id')::pg_catalog.regclass, '');
SELECT pg_catalog.pg_extension_config_dump(pg_catalog.pg_get_serial_sequence('companion_internal.ledger_entries', 'ledger_sequence')::pg_catalog.regclass, '');
SELECT pg_catalog.pg_extension_config_dump(pg_catalog.pg_get_serial_sequence('companion_internal.ledger_seals', 'seal_sequence')::pg_catalog.regclass, '');
SELECT pg_catalog.pg_extension_config_dump(pg_catalog.pg_get_serial_sequence('companion_internal.schema_job_phase_log', 'log_id')::pg_catalog.regclass, '');
SELECT pg_catalog.pg_extension_config_dump(pg_catalog.pg_get_serial_sequence('companion_internal.cluster_alarms', 'alarm_id')::pg_catalog.regclass, '');
SELECT pg_catalog.pg_extension_config_dump(pg_catalog.pg_get_serial_sequence('companion_internal.extension_upgrade_events', 'event_id')::pg_catalog.regclass, '');
