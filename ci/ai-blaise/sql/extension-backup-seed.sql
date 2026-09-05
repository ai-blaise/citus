-- FEATURE: D9
-- One non-default row in every extension table, including each FK dependency.
INSERT INTO companion_internal.txn_status_records (txn_id, coordinator, status, staging_physical_ms, intents)
VALUES ('backup-txn', 'coordinator', 'committed', 1000, '[]');
INSERT INTO companion_internal.timescale_bridge_state (feature_id, object_name, parameters)
VALUES ('backup', 'events', '{"interval":"1 day"}');
INSERT INTO companion_internal.shard_placement_generations (shard_id, generation, worker_name)
VALUES (9001, 7, 'worker-a');
INSERT INTO companion_internal.plan_freezes (query_hash, plan_xml, hint_set_name)
VALUES ('backup-query', '<plan/>', 'backup-hint');
INSERT INTO companion_internal.plan_promotion_policies (query_hash, min_executions, stable_days)
VALUES ('backup-query', 20, 3);
INSERT INTO companion_internal.plan_regression_policies (query_hash, max_latency_regression_percent, max_cost_regression_percent)
VALUES ('backup-query', 5, 8);
INSERT INTO companion_internal.plan_regression_samples (query_hash, baseline_p95_ms, candidate_p95_ms, baseline_cost, candidate_cost, violates_policy)
VALUES ('backup-query', 100, 102, 90, 91, false);
INSERT INTO companion_internal.migration_runs (migration_name, table_name, lock_timeout_ms, backfill_batch_size, status)
VALUES ('backup-migration', 'events', 100, 1000, 'completed');
INSERT INTO companion_internal.migration_operations (migration_name, operation_type, column_name, rendered_sql)
VALUES ('backup-migration', 'add_column', 'payload', 'SELECT 1');
INSERT INTO companion_internal.migration_invariant_checks (migration_name, check_name, check_sql, last_result)
VALUES ('backup-migration', 'backup-check', 'SELECT true', '{"ok":true}');
INSERT INTO companion_internal.index_advisor_candidates (workload_window, table_name, index_name, columns, index_method, estimated_cost_before, estimated_cost_after, qual_count)
VALUES ('1 hour', 'events', 'backup_index', ARRAY['id'], 'btree', 100, 50, 20);
INSERT INTO companion_internal.webhook_registrations (webhook_name, table_name, url, headers, max_retries)
VALUES ('backup-hook', 'events', 'https://example.invalid/hook', '{"X-Test":"backup-fixture"}', 3);
INSERT INTO companion_internal.webhook_triggers (webhook_name, table_name, events, queue_name, trigger_name, trigger_sql)
VALUES ('backup-hook', 'events', ARRAY['INSERT'], 'backup-queue', 'backup_trigger', 'SELECT 1');
INSERT INTO companion_internal.webhook_events (webhook_name, queue_name, table_name, operation, row_data)
VALUES ('backup-hook', 'backup-queue', 'events', 'INSERT', '{"id":37}');
INSERT INTO companion_internal.search_worker_indexes (index_name, table_name, distribution_column, text_columns)
VALUES ('backup_search', 'events', 'tenant_id', ARRAY['body']);
INSERT INTO companion_internal.search_documents (table_name, document_key, text_body, vector_score)
VALUES ('events', 'backup-document', 'Preserve this document', 0.9);
INSERT INTO companion_internal.search_rerank_requests (input_view, provider, model)
VALUES ('backup_view', 'test', 'test-model');
INSERT INTO companion_internal.graph_colocations (vertex_table, edge_table, vertex_key, colocation_group)
VALUES ('vertices', 'edges', 'id', 'backup-group');
INSERT INTO companion_internal.graphql_distributed_graphs (graph_name, vertex_table, edge_table)
VALUES ('backup-graph', 'vertices', 'edges');
INSERT INTO companion_internal.json_schemas (schema_name, schema_document)
VALUES ('backup-schema', '{"type":"object"}');
INSERT INTO companion_internal.jsonschema_triggers (table_name, json_column, schema_name, timing, trigger_name, trigger_sql)
VALUES ('events', 'payload', 'backup-schema', 'BEFORE', 'backup_json_trigger', 'SELECT 1');
INSERT INTO companion_internal.geo_distributions (table_name, geometry_column, distribution_column, precision)
VALUES ('places', 'point', 'tenant_id', 8);
INSERT INTO companion_internal.geo_pruning_policies (table_name, geometry_column, precision)
VALUES ('places', 'point', 8);
INSERT INTO companion_internal.vectorizer_definitions (vectorizer_name, source_table, source_pk, source_column, chunk_max_tokens, chunk_overlap_tokens, provider, model, secret_ref, destination_table, destination_column, dimensions, schedule_interval, max_concurrency, queue_table, create_sql)
VALUES ('backup-vectorizer', 'events', 'id', 'body', 512, 32, 'test', 'test-model', 'secret://backup-reference', 'vectors', 'embedding', 128, '1 hour', 2, 'backup_vectors', 'SELECT 1');
INSERT INTO companion_internal.vectorizer_usage_log (vectorizer_name, tenant_id, tokens)
VALUES ('backup-vectorizer', 'backup-tenant', 300);
INSERT INTO companion_internal.ai_provider_bindings (binding_name, tenant_id, provider, model, secret_ref, max_tokens_per_request)
VALUES ('backup-provider', 'backup-tenant', 'openai', 'test-model', 'secret://backup-reference', 1000);
INSERT INTO companion_internal.semantic_catalog_objects (tenant_id, object_name, relation_name, allowed_columns, description)
VALUES ('backup-tenant', 'backup-object', 'events', ARRAY['id'], 'Recovery fixture');
INSERT INTO companion_internal.db_doctor_rules (rule_id, severity)
VALUES ('backup-rule', 'warning');
INSERT INTO companion_internal.db_doctor_violations (rule_id, severity, object_name, message)
VALUES ('backup-rule', 'warning', 'events', 'Recovery fixture');
INSERT INTO companion_internal.toolkit_aggregate_plans (feature_id, aggregate_kind, source_table, worker_view, coordinator_view, distribution_column, value_column, worker_sql, coordinator_sql)
VALUES ('backup', 'sum', 'events', 'backup_worker', 'backup_coordinator', 'tenant_id', 'value', 'SELECT 1', 'SELECT 1');
INSERT INTO companion_internal.schema_jobs (job_name, table_name, state, lease_seconds, lease_expires_at)
VALUES ('backup-job', 'events', 'paused', 30, '2000-01-01 00:00:00+00');
INSERT INTO companion_internal.schema_job_operations (job_name, operation_type, rendered_sql)
VALUES ('backup-job', 'add_column', 'SELECT 1');
INSERT INTO companion_internal.tenant_moves (tenant_name, source_worker, target_worker, status)
VALUES ('backup-tenant', 'worker-a', 'worker-b', 'queued');
INSERT INTO companion_internal.tenant_quotas (tenant_name, max_connections, max_qps)
VALUES ('backup-tenant', 8, 100);
INSERT INTO companion_internal.tenant_archives (tenant_name, destination_uri, retention_days, status)
VALUES ('backup-tenant', 's3://backup-fixture/archive', 30, 'queued');
INSERT INTO companion_internal.tenant_region_affinities (tenant_name, region_affinity)
VALUES ('backup-tenant', 'test-region');
INSERT INTO companion_internal.extension_catalog_contracts (extension_name, tier, feature_ids, policy)
VALUES ('backup-extension', 'optional', ARRAY['backup'], 'Recovery fixture');
INSERT INTO storage.file_attachment_refs (tenant_id, owner_id, attachment)
VALUES ('backup-tenant', 'backup-owner', storage.file_attachment('backup-bucket', 'objects/test', 'text/plain', 37, repeat('a', 64), '{"test":true}'));
INSERT INTO companion_internal.ledger_entries (transfer_id, debit_account, credit_account, amount_cents, currency, previous_hash, entry_hash)
VALUES ('backup-transfer', 'a', 'b', 37, 'USD', 'previous', 'backup-entry');
INSERT INTO companion_internal.ledger_seals (transfer_id, hmac_algorithm, seal)
VALUES ('backup-transfer', 'HMAC-SHA256', 'backup-seal');
INSERT INTO companion_internal.schema_job_phase_log (job_name, from_state, to_state, started_at, completed_at, gate)
VALUES ('backup-job', 'backfill', 'paused', '2000-01-01 00:00:00+00', '2000-01-01 00:00:01+00', 'wait_forever');
INSERT INTO companion_internal.worker_schema_lease (worker_id, job_name, schema_version_id, phase, expires_at)
VALUES ('worker-a', 'backup-job', 'backup-version', 'paused', '2000-01-01 00:00:00+00');
INSERT INTO companion_internal.cluster_alarms (alarm_kind, severity, detail)
VALUES ('backup-alarm', 'warning', '{"test":true}');
INSERT INTO companion_internal.extension_upgrade_events (release_id, previous_version, target_version, action)
VALUES ('backup-release', '0.1.0', '0.1.1', 'upgrade');

DO $$
DECLARE
    sequence_oid oid;
BEGIN
    FOR sequence_oid IN
        SELECT c.oid FROM pg_class c
        JOIN pg_depend d ON d.classid = 'pg_class'::regclass AND d.objid = c.oid
        JOIN pg_extension e ON d.refclassid = 'pg_extension'::regclass AND d.refobjid = e.oid
        WHERE e.extname = 'ai_blaise_citus' AND d.deptype = 'e' AND c.relkind = 'S'
    LOOP
        PERFORM setval(sequence_oid::regclass, 987654, true);
    END LOOP;
END
$$;
