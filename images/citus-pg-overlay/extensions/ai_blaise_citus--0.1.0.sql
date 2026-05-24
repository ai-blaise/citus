-- FEATURE: TS18

CREATE SCHEMA IF NOT EXISTS companion_internal;
CREATE SCHEMA IF NOT EXISTS companion;


CREATE TABLE IF NOT EXISTS companion_internal.txn_status_records (
    txn_id text PRIMARY KEY,
    coordinator text NOT NULL,
    status text NOT NULL CHECK (status IN ('staging', 'committed', 'aborted')),
    staging_physical_ms bigint NOT NULL CHECK (staging_physical_ms > 0),
    observed_physical_ms bigint,
    intents jsonb NOT NULL CHECK (jsonb_typeof(intents) = 'array'),
    raft_index bigserial UNIQUE,
    updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS txn_status_records_status_updated_idx
    ON companion_internal.txn_status_records(status, updated_at);

CREATE TABLE IF NOT EXISTS companion_internal.timescale_bridge_state (
    bridge_id bigserial PRIMARY KEY,
    feature_id text NOT NULL,
    object_name text NOT NULL,
    parameters jsonb NOT NULL DEFAULT '{}'::jsonb,
    applied_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS timescale_bridge_state_feature_object_idx
ON companion_internal.timescale_bridge_state(feature_id, object_name);

CREATE TABLE IF NOT EXISTS companion_internal.shard_placement_generations (
    shard_id bigint PRIMARY KEY CHECK (shard_id > 0),
    generation bigint NOT NULL CHECK (generation > 0),
    worker_name text,
    updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS companion_internal.plan_freezes (
    query_hash text PRIMARY KEY,
    plan_xml text NOT NULL,
    hint_set_name text NOT NULL,
    frozen_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS companion_internal.plan_promotion_policies (
    query_hash text PRIMARY KEY
        REFERENCES companion_internal.plan_freezes(query_hash) ON DELETE CASCADE,
    min_executions integer NOT NULL CHECK (min_executions > 0),
    stable_days integer NOT NULL CHECK (stable_days > 0),
    updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS companion_internal.plan_regression_policies (
    query_hash text PRIMARY KEY
        REFERENCES companion_internal.plan_freezes(query_hash) ON DELETE CASCADE,
    max_latency_regression_percent integer NOT NULL CHECK (max_latency_regression_percent > 0),
    max_cost_regression_percent integer NOT NULL CHECK (max_cost_regression_percent > 0),
    updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS companion_internal.plan_regression_samples (
    sample_id bigserial PRIMARY KEY,
    query_hash text NOT NULL
        REFERENCES companion_internal.plan_freezes(query_hash) ON DELETE CASCADE,
    baseline_p95_ms bigint NOT NULL CHECK (baseline_p95_ms > 0),
    candidate_p95_ms bigint NOT NULL CHECK (candidate_p95_ms >= 0),
    baseline_cost bigint NOT NULL CHECK (baseline_cost > 0),
    candidate_cost bigint NOT NULL CHECK (candidate_cost >= 0),
    violates_policy boolean NOT NULL,
    recorded_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS companion_internal.migration_runs (
    migration_name text PRIMARY KEY,
    table_name text NOT NULL,
    lock_timeout_ms integer NOT NULL CHECK (lock_timeout_ms > 0),
    backfill_batch_size integer NOT NULL CHECK (backfill_batch_size > 0),
    status text NOT NULL CHECK (status IN ('running', 'completed')),
    started_at timestamptz NOT NULL DEFAULT now(),
    completed_at timestamptz
);

CREATE TABLE IF NOT EXISTS companion_internal.migration_operations (
    operation_id bigserial PRIMARY KEY,
    migration_name text NOT NULL
        REFERENCES companion_internal.migration_runs(migration_name) ON DELETE CASCADE,
    operation_type text NOT NULL,
    column_name text NOT NULL,
    sql_type text,
    default_expression text,
    new_column_name text,
    from_type text,
    to_type text,
    cast_expression text,
    rendered_sql text NOT NULL,
    recorded_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS companion_internal.migration_invariant_checks (
    migration_name text NOT NULL
        REFERENCES companion_internal.migration_runs(migration_name) ON DELETE CASCADE,
    check_name text NOT NULL,
    check_sql text NOT NULL,
    last_result jsonb,
    passed_at timestamptz,
    recorded_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (migration_name, check_name),
    CHECK (btrim(check_name) <> ''),
    CHECK (btrim(check_sql) <> '')
);

CREATE INDEX IF NOT EXISTS migration_invariant_checks_unpassed_idx
    ON companion_internal.migration_invariant_checks(migration_name)
    WHERE passed_at IS NULL;

CREATE TABLE IF NOT EXISTS companion_internal.index_advisor_candidates (
    candidate_id bigserial PRIMARY KEY,
    workload_window text NOT NULL,
    table_name text NOT NULL,
    index_name name NOT NULL,
    columns text[] NOT NULL CHECK (cardinality(columns) > 0),
    index_method text NOT NULL CHECK (
        index_method IN ('btree', 'gin', 'gist', 'brin', 'rum', 'hnsw')
    ),
    estimated_cost_before numeric NOT NULL CHECK (estimated_cost_before > 0),
    estimated_cost_after numeric NOT NULL CHECK (estimated_cost_after >= 0),
    qual_count bigint NOT NULL CHECK (qual_count > 0),
    created_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT index_advisor_candidates_improves
        CHECK (estimated_cost_after < estimated_cost_before)
);

CREATE TABLE IF NOT EXISTS companion_internal.webhook_registrations (
    webhook_name text PRIMARY KEY,
    table_name text NOT NULL,
    url text NOT NULL,
    headers jsonb NOT NULL DEFAULT '{}'::jsonb,
    max_retries integer NOT NULL CHECK (max_retries > 0),
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS companion_internal.webhook_triggers (
    trigger_id bigserial PRIMARY KEY,
    webhook_name text NOT NULL
        REFERENCES companion_internal.webhook_registrations(webhook_name)
        ON DELETE CASCADE,
    table_name text NOT NULL,
    events text[] NOT NULL CHECK (cardinality(events) > 0),
    queue_name text NOT NULL,
    trigger_name name NOT NULL UNIQUE,
    trigger_sql text NOT NULL,
    installed_at timestamptz NOT NULL DEFAULT now(),
    UNIQUE (webhook_name, table_name)
);

CREATE TABLE IF NOT EXISTS companion_internal.webhook_events (
    event_id bigserial PRIMARY KEY,
    webhook_name text NOT NULL
        REFERENCES companion_internal.webhook_registrations(webhook_name)
        ON DELETE CASCADE,
    queue_name text NOT NULL,
    table_name text NOT NULL,
    operation text NOT NULL CHECK (operation IN ('INSERT', 'UPDATE', 'DELETE')),
    row_data jsonb NOT NULL,
    queued_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS companion_internal.search_worker_indexes (
    index_name name PRIMARY KEY,
    table_name text NOT NULL,
    distribution_column name NOT NULL,
    text_columns text[] NOT NULL CHECK (cardinality(text_columns) > 0),
    vector_columns text[] NOT NULL DEFAULT ARRAY[]::text[],
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS companion_internal.search_documents (
    document_id bigserial PRIMARY KEY,
    table_name text NOT NULL,
    document_key text NOT NULL,
    text_body text NOT NULL,
    vector_score numeric NOT NULL DEFAULT 0,
    updated_at timestamptz NOT NULL DEFAULT now(),
    UNIQUE (table_name, document_key)
);

CREATE TABLE IF NOT EXISTS companion_internal.search_rerank_requests (
    request_id bigserial PRIMARY KEY,
    input_view text NOT NULL,
    provider text NOT NULL,
    model text NOT NULL,
    requested_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS companion_internal.graph_colocations (
    colocation_id bigserial PRIMARY KEY,
    vertex_table text NOT NULL,
    edge_table text NOT NULL,
    vertex_key name NOT NULL,
    colocation_group text NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    UNIQUE (vertex_table, edge_table, vertex_key, colocation_group)
);

CREATE TABLE IF NOT EXISTS companion_internal.graphql_distributed_graphs (
    graph_name text PRIMARY KEY,
    vertex_table text NOT NULL,
    edge_table text NOT NULL,
    registered_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS companion_internal.json_schemas (
    schema_name text PRIMARY KEY,
    schema_document jsonb NOT NULL,
    registered_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS companion_internal.jsonschema_triggers (
    trigger_id bigserial PRIMARY KEY,
    table_name text NOT NULL,
    json_column name NOT NULL,
    schema_name text NOT NULL
        REFERENCES companion_internal.json_schemas(schema_name)
        ON DELETE CASCADE,
    timing text NOT NULL,
    trigger_name name NOT NULL UNIQUE,
    trigger_sql text NOT NULL,
    installed_at timestamptz NOT NULL DEFAULT now(),
    UNIQUE (table_name, json_column, schema_name)
);

CREATE TABLE IF NOT EXISTS companion_internal.geo_distributions (
    table_name text PRIMARY KEY,
    geometry_column name NOT NULL,
    distribution_column name NOT NULL,
    precision integer NOT NULL CHECK (precision BETWEEN 1 AND 12),
    updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS companion_internal.geo_pruning_policies (
    policy_id bigserial PRIMARY KEY,
    table_name text NOT NULL,
    geometry_column name NOT NULL,
    precision integer NOT NULL CHECK (precision BETWEEN 1 AND 12),
    updated_at timestamptz NOT NULL DEFAULT now(),
    UNIQUE (table_name, geometry_column)
);

CREATE TABLE IF NOT EXISTS companion_internal.vectorizer_definitions (
    vectorizer_name text PRIMARY KEY,
    source_table text NOT NULL,
    source_pk name NOT NULL,
    source_column name NOT NULL,
    chunk_max_tokens integer NOT NULL CHECK (chunk_max_tokens > 0),
    chunk_overlap_tokens integer NOT NULL CHECK (chunk_overlap_tokens >= 0),
    provider text NOT NULL,
    model text NOT NULL,
    secret_ref text NOT NULL,
    destination_table text NOT NULL,
    destination_column name NOT NULL,
    dimensions integer NOT NULL CHECK (dimensions > 0),
    schedule_interval text NOT NULL,
    max_concurrency integer NOT NULL CHECK (max_concurrency > 0),
    tenant_budget_tokens bigint CHECK (tenant_budget_tokens IS NULL OR tenant_budget_tokens > 0),
    queue_table name NOT NULL,
    create_sql text NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    CHECK (chunk_overlap_tokens < chunk_max_tokens)
);

CREATE TABLE IF NOT EXISTS companion_internal.vectorizer_usage_log (
    usage_id bigserial PRIMARY KEY,
    vectorizer_name text NOT NULL
        REFERENCES companion_internal.vectorizer_definitions(vectorizer_name)
        ON DELETE CASCADE,
    tenant_id text NOT NULL,
    tokens bigint NOT NULL CHECK (tokens > 0),
    recorded_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS companion_internal.ai_provider_bindings (
    binding_name text PRIMARY KEY,
    tenant_id text NOT NULL CHECK (btrim(tenant_id) <> ''),
    provider text NOT NULL CHECK (
        provider IN ('openai', 'azure_openai', 'anthropic', 'cohere', 'voyage', 'ollama', 'vertex_ai')
    ),
    model text NOT NULL CHECK (btrim(model) <> ''),
    secret_ref text NOT NULL CHECK (
        secret_ref ~ '^(secret|external-secret)://[A-Za-z0-9._/@:-]+$'
    ),
    max_tokens_per_request integer NOT NULL CHECK (max_tokens_per_request BETWEEN 1 AND 200000),
    enabled boolean NOT NULL DEFAULT true,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS companion_internal.semantic_catalog_objects (
    tenant_id text NOT NULL CHECK (btrim(tenant_id) <> ''),
    object_name text NOT NULL CHECK (btrim(object_name) <> ''),
    relation_name text NOT NULL CHECK (btrim(relation_name) <> ''),
    allowed_columns text[] NOT NULL CHECK (cardinality(allowed_columns) > 0),
    description text NOT NULL CHECK (btrim(description) <> ''),
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (tenant_id, object_name)
);

CREATE TABLE IF NOT EXISTS companion_internal.db_doctor_rules (
    rule_id text PRIMARY KEY,
    severity text NOT NULL CHECK (severity IN ('error', 'warning', 'note')),
    enabled boolean NOT NULL DEFAULT true,
    updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS companion_internal.db_doctor_violations (
    violation_id bigserial PRIMARY KEY,
    rule_id text NOT NULL,
    severity text NOT NULL CHECK (severity IN ('error', 'warning', 'note')),
    object_name text NOT NULL,
    message text NOT NULL,
    detected_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS companion_internal.toolkit_aggregate_plans (
    plan_id bigserial PRIMARY KEY,
    feature_id text NOT NULL,
    aggregate_kind text NOT NULL,
    source_table text NOT NULL,
    worker_view name NOT NULL,
    coordinator_view name NOT NULL,
    distribution_column name NOT NULL,
    value_column name NOT NULL,
    time_column name,
    bucket_width text,
    worker_sql text NOT NULL,
    coordinator_sql text NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    UNIQUE (source_table, worker_view, coordinator_view, aggregate_kind)
);

CREATE TABLE IF NOT EXISTS companion_internal.schema_jobs (
    job_name text PRIMARY KEY,
    table_name text NOT NULL,
    state text NOT NULL CHECK (
        state IN ('delete_only', 'write_only', 'backfill', 'public', 'paused', 'canceled')
    ),
    lease_seconds integer NOT NULL CHECK (lease_seconds > 0),
    lease_expires_at timestamptz NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS companion_internal.schema_job_operations (
    operation_id bigserial PRIMARY KEY,
    job_name text NOT NULL
        REFERENCES companion_internal.schema_jobs(job_name)
        ON DELETE CASCADE,
    operation_type text NOT NULL CHECK (
        operation_type IN ('add_column', 'backfill', 'swap_column', 'drop_column')
    ),
    column_name text,
    sql_type text,
    statement text,
    new_column_name text,
    rendered_sql text NOT NULL,
    recorded_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS companion_internal.tenant_moves (
    move_id bigserial PRIMARY KEY,
    tenant_name text NOT NULL,
    source_worker text NOT NULL,
    target_worker text NOT NULL,
    region_affinity text,
    status text NOT NULL CHECK (status IN ('queued', 'completed', 'canceled')),
    created_at timestamptz NOT NULL DEFAULT now(),
    CHECK (source_worker <> target_worker)
);

CREATE TABLE IF NOT EXISTS companion_internal.tenant_quotas (
    tenant_name text PRIMARY KEY,
    max_connections integer NOT NULL CHECK (max_connections > 0),
    max_qps integer NOT NULL CHECK (max_qps > 0),
    updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS companion_internal.tenant_archives (
    archive_id bigserial PRIMARY KEY,
    tenant_name text NOT NULL,
    destination_uri text NOT NULL,
    retention_days integer NOT NULL CHECK (retention_days > 0),
    status text NOT NULL CHECK (status IN ('queued', 'completed', 'canceled')),
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS companion_internal.tenant_region_affinities (
    tenant_name text PRIMARY KEY,
    region_affinity text NOT NULL,
    updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS companion_internal.extension_catalog_contracts (
    extension_name text PRIMARY KEY,
    tier text NOT NULL CHECK (tier IN ('required', 'optional', 'integration-target', 'hard-block')),
    feature_ids text[] NOT NULL CHECK (cardinality(feature_ids) > 0),
    requires_preload boolean NOT NULL DEFAULT false,
    policy text NOT NULL,
    registered_at timestamptz NOT NULL DEFAULT now()
);

CREATE FUNCTION companion_feature_status()
RETURNS TABLE(feature_id text, feature_name text, status text)
LANGUAGE sql
STABLE
PARALLEL SAFE
AS $$
    VALUES
        ('TS1', 'distributed hypertable bridge', 'sql-runtime'),
        ('TS2', 'distributed compression policy', 'sql-runtime'),
        ('TS3', 'distributed continuous aggregates', 'sql-runtime'),
        ('TS4', 'distributed retention policy', 'sql-runtime'),
        ('TS5', 'time-range shard pruner', 'sql-runtime'),
        ('TS8', 'LSP hypertable invariants', 'sql-plan'),
        ('TS9', 'doctor rules for cohabitation', 'sql-runtime'),
        ('TS12', 'distributed reorder policy', 'sql-runtime'),
        ('TS18', 'executable Timescale bridge state', 'sql-runtime'),
        ('TS13', 'distributed time_bucket_gapfill', 'sql-runtime'),
        ('TS14', 'distributed metric toolkit aggregates', 'sql-runtime'),
        ('TS15', 'distributed approximate toolkit aggregates', 'sql-runtime'),
        ('TS16', 'distributed downsampler toolkit aggregates', 'sql-runtime'),
        ('TS17', 'distributed state toolkit aggregates', 'sql-runtime'),
        ('A1', 'pgai-compatible vectorizer DSL', 'sql-runtime'),
        ('A10', 'streaming chat completion SQL contract', 'sql-intent-fail-closed'),
        ('A11', 'semantic catalog text-to-SQL SQL contract', 'sql-intent-fail-closed'),
        ('Search2', 'distributed BM25 search index', 'sql-runtime'),
        ('Search3', 'hybrid BM25 and vector ranking', 'sql-runtime'),
        ('Search9', 'reranker UDF plan', 'sql-runtime'),
        ('G2', 'distributed graph bridge', 'sql-runtime'),
        ('G3', 'graph colocation policy', 'sql-runtime'),
        ('API4', 'GraphQL distributed graph metadata', 'sql-runtime'),
        ('JS2', 'distributed JSON Schema validation', 'sql-runtime'),
        ('M13', 'JSON Schema validation triggers', 'sql-runtime'),
        ('Geo2', 'geo-aware distribution', 'sql-runtime'),
        ('Geo3', 'geo shard pruning', 'sql-runtime'),
        ('T8', 'toolkit two-step aggregate pushdown', 'sql-runtime'),
        ('L9', 'worker partial aggregate pushdown', 'sql-runtime'),
        ('M7', 'pre-flight cohabit-extension check', 'sql-runtime'),
        ('PM3', 'plan freeze companion module', 'sql-runtime'),
        ('PM4', 'plan regression detection', 'sql-runtime'),
        ('IA3', 'companion index advisor', 'sql-runtime'),
        ('Sec5', 'immutable ledger', 'sql-runtime'),
        ('Sec6', 'ledger HMAC tamper evidence', 'sql-runtime'),
        ('M1', 'pgroll-style expand-contract migrations', 'sql-runtime'),
        ('M11', 'online column-type migration', 'sql-runtime'),
        ('WH2', 'companion webhook helpers', 'sql-runtime'),
        ('O1', 'query percentile views', 'sql-runtime'),
        ('O2', 'local activity stats view', 'sql-runtime'),
        ('O3', 'replication lag view', 'sql-runtime'),
        ('R4', 'idle transaction detector', 'sql-runtime'),
        ('Auth2', 'tenant-aware claims', 'sql-runtime'),
        ('Sec1', 'RLS helpers', 'sql-runtime'),
        ('Sec2', 'JWT verification UDF', 'sql-runtime'),
        ('S6', 'placement generation helpers', 'sql-runtime'),
        ('S13', 'range routing helpers', 'sql-runtime'),
        ('C10', 'online schema job state machine', 'sql-runtime'),
        ('M2', 'gh-ost-style online DDL', 'sql-runtime'),
        ('S14', 'tenant migration online', 'sql-runtime'),
        ('TO3', 'tenant migration online', 'sql-runtime'),
        ('TO4', 'tenant archive', 'sql-runtime'),
        ('TO5', 'tenant region affinity', 'sql-runtime'),
        ('D4', 'citus-lsp metadata views', 'sql-plan'),
        ('M5', 'LSP migration quick-fix metadata', 'sql-plan'),
        ('D7', 'Helm one-line install', 'ops-contract'),
        ('D8', 'infrastructure deploy wrapper', 'ops-contract'),
        ('D9', 'canary upgrade runbook', 'ops-contract'),
        ('D10', 'release hardening runbook', 'ops-contract'),
        ('D11', 'MCP developer workflow', 'ops-contract'),
        ('MR9', 'region survival runbook', 'ops-contract'),
        ('RT5', 'Phoenix-channel-compatible realtime client', 'ops-contract'),
        ('S7', 'cross-region replication via pgactive', 'ops-contract'),
        ('A9', 'secret binding via External Secrets', 'ops-contract'),
        ('Sec7', 'External Secrets integration', 'ops-contract'),
        ('Sec8', 'TLS everywhere', 'ops-contract'),
        ('Sec9', 'SBOM and cosign attestation', 'ops-contract'),
        ('Sec13', 'CIDR access control', 'ops-contract'),
        ('T6', 'PG18 io_uring default', 'ops-contract'),
        ('T7', 'pipelined client protocol in pool', 'ops-contract'),
        ('A7', 'pgvector cohabitation', 'extension-catalog-runtime'),
        ('A12', 'vchord alternate vector index', 'extension-catalog-runtime'),
        ('C11', 'DDL replication via pgl_ddl_deploy', 'extension-catalog-runtime'),
        ('C12', 'replication-slot failover', 'extension-catalog-runtime'),
        ('C13', 'subscription failover', 'extension-catalog-runtime'),
        ('EF6', 'in-database JavaScript and Rust UDF substrate', 'extension-catalog-runtime'),
        ('F2', 'foreign data wrapper bundle', 'extension-catalog-runtime'),
        ('F5', 'outbound HTTP extensions', 'extension-catalog-runtime'),
        ('G1', 'Apache AGE bundled', 'extension-catalog-runtime'),
        ('Geo1', 'PostGIS bundled', 'extension-catalog-runtime'),
        ('IA1', 'HypoPG bundled', 'extension-catalog-runtime'),
        ('IA2', 'pg_qualstats bundled', 'extension-catalog-runtime'),
        ('JS1', 'pg_jsonschema bundled', 'extension-catalog-runtime'),
        ('L11', 'pg_parquet bundled', 'extension-catalog-runtime'),
        ('M6', 'DDL replication', 'extension-catalog-runtime'),
        ('M10', 'track settings drift', 'extension-catalog-runtime'),
        ('M12', 'UUIDv7 primary keys', 'extension-catalog-runtime'),
        ('MR7', 'cross-region active-active references', 'extension-catalog-runtime'),
        ('O7', 'wait-event sampling', 'extension-catalog-runtime'),
        ('O8', 'OS metrics via SQL', 'extension-catalog-runtime'),
        ('O9', 'kernel stats via SQL', 'extension-catalog-runtime'),
        ('O11', 'pg_stat_monitor alternative', 'extension-catalog-runtime'),
        ('O12', 'pg_show_plans plan-inspection contract', 'extension-catalog-runtime'),
        ('PM1', 'pg_hint_plan bundled', 'extension-catalog-runtime'),
        ('PM2', 'sr_plan bundled', 'extension-catalog-runtime'),
        ('R6', 'bloat-free queue substrate', 'extension-catalog-runtime'),
        ('R11', 'pg_warm bundled', 'extension-catalog-runtime'),
        ('Search1', 'pg_search bundled', 'extension-catalog-runtime'),
        ('Search4', 'RUM index bundled', 'extension-catalog-runtime'),
        ('Search5', 'pg_trgm bundled', 'extension-catalog-runtime'),
        ('Search6', 'citext bundled', 'extension-catalog-runtime'),
        ('Sec3', 'pgaudit and file audit', 'extension-catalog-runtime'),
        ('Sec4', 'pgsodium crypto', 'extension-catalog-runtime'),
        ('Sec10', 'pg_safeupdate guard', 'extension-catalog-runtime'),
        ('Sec11', 'CDC anonymization extension', 'extension-catalog-runtime'),
        ('Sec14', 'pgcrypto bundled', 'extension-catalog-runtime'),
        ('Sec15', 'encryption-at-rest with CMK', 'extension-catalog-runtime'),
        ('WF1', 'pg_walinspect forensic workflow', 'extension-catalog-runtime')
$$;

-- FEATURE: A7
-- FEATURE: A12
-- FEATURE: C11
-- FEATURE: C12
-- FEATURE: C13
-- FEATURE: EF6
-- FEATURE: F2
-- FEATURE: F5
-- FEATURE: G1
-- FEATURE: Geo1
-- FEATURE: IA1
-- FEATURE: IA2
-- FEATURE: JS1
-- FEATURE: L11
-- FEATURE: M6
-- FEATURE: M10
-- FEATURE: M12
-- FEATURE: MR7
-- FEATURE: O7
-- FEATURE: O8
-- FEATURE: O9
-- FEATURE: O11
-- FEATURE: O12
-- FEATURE: PM1
-- FEATURE: PM2
-- FEATURE: R6
-- FEATURE: R11
-- FEATURE: Search1
-- FEATURE: Search4
-- FEATURE: Search5
-- FEATURE: Search6
-- FEATURE: Sec3
-- FEATURE: Sec4
-- FEATURE: Sec10
-- FEATURE: Sec11
-- FEATURE: Sec14
-- FEATURE: Sec15
-- FEATURE: WF1
CREATE VIEW companion_extension_catalog AS
SELECT
    extension_name,
    tier,
    feature_ids,
    requires_preload,
    policy,
    registered_at
FROM companion_internal.extension_catalog_contracts;

CREATE VIEW companion_extension_feature_coverage AS
SELECT
    extension_name,
    tier,
    unnest(feature_ids) AS feature_id,
    requires_preload,
    policy
FROM companion_internal.extension_catalog_contracts;

CREATE FUNCTION companion_internal.register_extension_contract(
    p_extension_name text,
    p_tier text,
    p_feature_ids text[],
    p_requires_preload boolean DEFAULT false,
    p_policy text DEFAULT ''
)
RETURNS text
LANGUAGE plpgsql
AS $$
DECLARE
    normalized_name text;
    normalized_tier text;
    normalized_feature_ids text[];
BEGIN
    normalized_name := lower(btrim(p_extension_name));
    normalized_tier := lower(btrim(p_tier));

    IF normalized_name IS NULL OR normalized_name = '' THEN
        RAISE EXCEPTION 'extension_name must not be empty';
    END IF;
    IF normalized_tier NOT IN ('required', 'optional', 'integration-target', 'hard-block') THEN
        RAISE EXCEPTION 'unsupported extension tier: %', p_tier;
    END IF;
    SELECT array_agg(DISTINCT btrim(feature_id) ORDER BY btrim(feature_id))
    INTO normalized_feature_ids
    FROM unnest(p_feature_ids) AS feature_id
    WHERE feature_id IS NOT NULL AND btrim(feature_id) <> '';
    IF normalized_feature_ids IS NULL OR cardinality(normalized_feature_ids) = 0 THEN
        RAISE EXCEPTION 'feature_ids must not be empty';
    END IF;
    IF normalized_tier = 'hard-block' AND p_requires_preload THEN
        RAISE EXCEPTION 'hard-blocked extensions cannot require preload';
    END IF;

    INSERT INTO companion_internal.extension_catalog_contracts(
        extension_name,
        tier,
        feature_ids,
        requires_preload,
        policy
    )
    VALUES (
        normalized_name,
        normalized_tier,
        normalized_feature_ids,
        COALESCE(p_requires_preload, false),
        COALESCE(NULLIF(btrim(p_policy), ''), 'no policy recorded')
    )
    ON CONFLICT (extension_name) DO UPDATE
    SET tier = EXCLUDED.tier,
        feature_ids = EXCLUDED.feature_ids,
        requires_preload = EXCLUDED.requires_preload,
        policy = EXCLUDED.policy,
        registered_at = now();

    RETURN normalized_name;
END;
$$;

CREATE FUNCTION companion_internal.seed_extension_catalog()
RETURNS integer
LANGUAGE plpgsql
AS $$
DECLARE
    seeded integer;
BEGIN
    WITH seed(extension_name, tier, feature_ids, requires_preload, policy) AS (
        VALUES
            ('pgvector', 'required', ARRAY['A7'], false, 'Vector similarity extension'),
            ('vchord', 'optional', ARRAY['A12'], false, 'Alternate vector index'),
            ('pgl_ddl_deploy', 'optional', ARRAY['C11','M6'], false, 'DDL replication across regions'),
            ('pg_failover_slots', 'required', ARRAY['C12'], true, 'Logical replication slot failover'),
            ('pg_subscription_pg_failover', 'optional', ARRAY['C13'], false, 'Logical subscription failover state'),
            ('plrust', 'required', ARRAY['EF6'], true, 'Rust UDF substrate'),
            ('plv8', 'required', ARRAY['EF6'], true, 'In-database JavaScript UDF substrate'),
            ('oracle_fdw', 'optional', ARRAY['F2'], false, 'Oracle migration and federation FDW'),
            ('mysql_fdw', 'optional', ARRAY['F2'], false, 'MySQL migration and federation FDW'),
            ('mongo_fdw', 'optional', ARRAY['F2'], false, 'Mongo migration and federation FDW'),
            ('tds_fdw', 'optional', ARRAY['F2'], false, 'SQL Server migration and federation FDW'),
            ('pgsql-http', 'optional', ARRAY['F5'], false, 'Outbound HTTP from SQL'),
            ('pg_net', 'optional', ARRAY['F5'], false, 'Async outbound HTTP from SQL'),
            ('age', 'required', ARRAY['G1'], true, 'Apache AGE graph query substrate'),
            ('postgis', 'required', ARRAY['Geo1'], false, 'Geospatial functions for distributed geo features'),
            ('hypopg', 'optional', ARRAY['IA1'], false, 'What-if indexing for advisor'),
            ('pg_qualstats', 'optional', ARRAY['IA2'], false, 'Predicate stats for advisor'),
            ('pg_jsonschema', 'required', ARRAY['JS1'], false, 'JSON Schema validation substrate'),
            ('pg_parquet', 'optional', ARRAY['L11'], false, 'Parquet read/write helper'),
            ('pg_track_settings', 'optional', ARRAY['M10'], false, 'Configuration drift tracking'),
            ('pg_uuidv7', 'required', ARRAY['M12'], false, 'Monotonic UUID helper'),
            ('pgactive', 'optional', ARRAY['MR7'], true, 'Cross-region active-active for reference tables'),
            ('pg_wait_sampling', 'optional', ARRAY['O7'], true, 'Wait-event sampling'),
            ('pgsentinel', 'optional', ARRAY['O7'], true, 'ASH-style wait diagnostics'),
            ('pgnodemx', 'required', ARRAY['O8'], false, 'OS and cgroup metrics through SQL'),
            ('pg_stat_kcache', 'optional', ARRAY['O9'], true, 'Kernel CPU and IO per statement'),
            ('pg_stat_monitor', 'optional', ARRAY['O11'], true, 'Alternative statement histogram view'),
            ('pg_show_plans', 'optional', ARRAY['O12'], true, 'Live plan inspection'),
            ('pg_hint_plan', 'optional', ARRAY['PM1'], true, 'Hint-driven plan management'),
            ('sr_plan', 'optional', ARRAY['PM2'], true, 'Saved-plan backend'),
            ('pgmq', 'optional', ARRAY['R6'], false, 'Alternative queue substrate'),
            ('pgque', 'optional', ARRAY['R6'], false, 'Bloat-free queue substrate'),
            ('pg_warm', 'required', ARRAY['R11'], true, 'Replica cold-start cache warming'),
            ('pg_search', 'required', ARRAY['Search1'], true, 'BM25 and hybrid search substrate'),
            ('rum', 'required', ARRAY['Search4'], false, 'Alternate full-text index'),
            ('pg_trgm', 'required', ARRAY['Search5'], false, 'Trigram search support'),
            ('citext', 'required', ARRAY['Search6'], false, 'Case-insensitive text type'),
            ('pgaudit', 'required', ARRAY['Sec3'], true, 'SQL audit baseline'),
            ('pgauditlogtofile', 'required', ARRAY['Sec3'], true, 'File-backed audit log sink'),
            ('pgsodium', 'required', ARRAY['Sec4','Sec15'], true, 'Libsodium crypto and encryption helpers'),
            ('pg_safeupdate', 'optional', ARRAY['Sec10'], true, 'Guard accidental full-table writes'),
            ('anon', 'optional', ARRAY['Sec11'], false, 'CDC anonymization substrate'),
            ('pgcrypto', 'required', ARRAY['Sec14'], false, 'Core crypto primitives'),
            ('pg_walinspect', 'optional', ARRAY['WF1'], false, 'WAL inspection from SQL'),
            ('omnigres', 'integration-target', ARRAY['F5'], false, 'Reference-only HTTP/API stack')
    ), upserted AS (
        INSERT INTO companion_internal.extension_catalog_contracts(
            extension_name,
            tier,
            feature_ids,
            requires_preload,
            policy
        )
        SELECT extension_name, tier, feature_ids, requires_preload, policy
        FROM seed
        ON CONFLICT (extension_name) DO UPDATE
        SET tier = EXCLUDED.tier,
            feature_ids = EXCLUDED.feature_ids,
            requires_preload = EXCLUDED.requires_preload,
            policy = EXCLUDED.policy,
            registered_at = now()
        RETURNING 1
    )
    SELECT count(*) INTO seeded FROM upserted;

    RETURN seeded;
END;
$$;

CREATE FUNCTION companion_extension_required(p_feature_id text)
RETURNS TABLE(extension_name text, tier text, requires_preload boolean)
LANGUAGE plpgsql
STABLE
AS $$
BEGIN
    IF p_feature_id IS NULL OR btrim(p_feature_id) = '' THEN
        RAISE EXCEPTION 'feature_id must not be empty';
    END IF;

    RETURN QUERY
    SELECT c.extension_name, c.tier, c.requires_preload
    FROM companion_internal.extension_catalog_contracts AS c
    WHERE btrim(p_feature_id) = ANY(c.feature_ids)
      AND c.tier <> 'hard-block'
    ORDER BY c.tier, c.extension_name;
END;
$$;

CREATE FUNCTION companion_required_preload_libraries()
RETURNS text[]
LANGUAGE sql
STABLE
AS $$
    SELECT COALESCE(array_agg(extension_name ORDER BY extension_name), ARRAY[]::text[])
    FROM companion_internal.extension_catalog_contracts
    WHERE requires_preload AND tier <> 'hard-block'
$$;

CREATE FUNCTION companion_extension_conflicts(p_extension_name text)
RETURNS boolean
LANGUAGE plpgsql
STABLE
AS $$
BEGIN
    IF p_extension_name IS NULL OR btrim(p_extension_name) = '' THEN
        RAISE EXCEPTION 'extension_name must not be empty';
    END IF;

    RETURN EXISTS (
        SELECT 1
        FROM companion_internal.extension_catalog_contracts
        WHERE extension_name = lower(btrim(p_extension_name))
          AND tier = 'hard-block'
    );
END;
$$;

CREATE FUNCTION companion_internal.assert_extension_allowed(p_extension_name text)
RETURNS void
LANGUAGE plpgsql
AS $$
DECLARE
    contract_tier text;
BEGIN
    IF p_extension_name IS NULL OR btrim(p_extension_name) = '' THEN
        RAISE EXCEPTION 'extension_name must not be empty';
    END IF;

    SELECT tier INTO contract_tier
    FROM companion_internal.extension_catalog_contracts
    WHERE extension_name = lower(btrim(p_extension_name));

    IF contract_tier IS NULL THEN
        RAISE EXCEPTION 'extension is not registered: %', p_extension_name;
    END IF;
    IF contract_tier = 'hard-block' THEN
        RAISE EXCEPTION 'extension is hard-blocked: %', p_extension_name;
    END IF;
END;
$$;

-- FEATURE: Search2
-- FEATURE: Search3
-- FEATURE: Search9
CREATE VIEW companion_search_worker_indexes AS
SELECT
    index_name,
    table_name,
    distribution_column,
    text_columns,
    vector_columns,
    created_at
FROM companion_internal.search_worker_indexes;

CREATE VIEW companion_search_documents AS
SELECT
    document_id,
    table_name,
    document_key,
    text_body,
    vector_score,
    updated_at
FROM companion_internal.search_documents;

CREATE VIEW companion_search_rerank_requests AS
SELECT
    request_id,
    input_view,
    provider,
    model,
    requested_at
FROM companion_internal.search_rerank_requests;

CREATE FUNCTION companion_internal.table_has_column(
    p_table_name text,
    p_column_name name
)
RETURNS boolean
LANGUAGE sql
STABLE
PARALLEL SAFE
AS $$
    SELECT EXISTS (
        SELECT 1
        FROM pg_attribute
        WHERE attrelid = p_table_name::regclass
          AND attname = p_column_name
          AND attnum > 0
          AND NOT attisdropped
    )
$$;

-- FEATURE: A1
CREATE VIEW companion_vectorizer_definitions AS
SELECT
    vectorizer_name,
    source_table,
    source_pk,
    source_column,
    chunk_max_tokens,
    chunk_overlap_tokens,
    provider,
    model,
    secret_ref,
    destination_table,
    destination_column,
    dimensions,
    schedule_interval,
    max_concurrency,
    tenant_budget_tokens,
    queue_table,
    create_sql,
    created_at
FROM companion_internal.vectorizer_definitions;

CREATE VIEW companion_vectorizer_usage_log AS
SELECT
    usage_id,
    vectorizer_name,
    tenant_id,
    tokens,
    recorded_at
FROM companion_internal.vectorizer_usage_log;

CREATE FUNCTION companion_internal.register_vectorizer(
    p_vectorizer_name text,
    p_source_table text,
    p_source_pk text,
    p_source_column text,
    p_chunk_max_tokens integer,
    p_chunk_overlap_tokens integer,
    p_provider text,
    p_model text,
    p_secret_ref text,
    p_destination_table text,
    p_destination_column text,
    p_dimensions integer,
    p_schedule_interval text,
    p_max_concurrency integer,
    p_tenant_budget_tokens bigint DEFAULT NULL
)
RETURNS text
LANGUAGE plpgsql
VOLATILE
AS $$
DECLARE
    source_regclass regclass;
    normalized_provider text;
    queue_table name;
    rendered_sql text;
BEGIN
    IF p_vectorizer_name IS NULL OR btrim(p_vectorizer_name) = '' THEN
        RAISE EXCEPTION 'vectorizer_name must not be empty';
    END IF;
    IF p_source_table IS NULL OR btrim(p_source_table) = '' THEN
        RAISE EXCEPTION 'source_table must not be empty';
    END IF;
    source_regclass := p_source_table::regclass;
    IF p_source_pk IS NULL OR btrim(p_source_pk) = '' THEN
        RAISE EXCEPTION 'source_pk must not be empty';
    END IF;
    IF NOT companion_internal.table_has_column(source_regclass::text, btrim(p_source_pk)::name) THEN
        RAISE EXCEPTION 'source_pk column does not exist on source table';
    END IF;
    IF p_source_column IS NULL OR btrim(p_source_column) = '' THEN
        RAISE EXCEPTION 'source_column must not be empty';
    END IF;
    IF NOT companion_internal.table_has_column(source_regclass::text, btrim(p_source_column)::name) THEN
        RAISE EXCEPTION 'source_column does not exist on source table';
    END IF;
    IF p_chunk_max_tokens IS NULL OR p_chunk_max_tokens <= 0 THEN
        RAISE EXCEPTION 'chunk_max_tokens must be greater than zero';
    END IF;
    IF p_chunk_overlap_tokens IS NULL OR p_chunk_overlap_tokens < 0 THEN
        RAISE EXCEPTION 'chunk_overlap_tokens must be zero or greater';
    END IF;
    IF p_chunk_overlap_tokens >= p_chunk_max_tokens THEN
        RAISE EXCEPTION 'chunk_overlap_tokens must be less than chunk_max_tokens';
    END IF;
    normalized_provider := lower(btrim(p_provider));
    IF normalized_provider NOT IN (
        'openai',
        'azure_openai',
        'anthropic',
        'cohere',
        'voyage',
        'ollama',
        'vertex_ai'
    ) THEN
        RAISE EXCEPTION 'unsupported vectorizer provider: %', p_provider;
    END IF;
    IF p_model IS NULL OR btrim(p_model) = '' THEN
        RAISE EXCEPTION 'model must not be empty';
    END IF;
    IF p_secret_ref IS NULL OR btrim(p_secret_ref) = '' THEN
        RAISE EXCEPTION 'secret_ref must not be empty';
    END IF;
    IF p_destination_table IS NULL OR btrim(p_destination_table) = '' THEN
        RAISE EXCEPTION 'destination_table must not be empty';
    END IF;
    IF p_destination_column IS NULL OR btrim(p_destination_column) = '' THEN
        RAISE EXCEPTION 'destination_column must not be empty';
    END IF;
    IF p_dimensions IS NULL OR p_dimensions <= 0 THEN
        RAISE EXCEPTION 'dimensions must be greater than zero';
    END IF;
    IF p_schedule_interval IS NULL OR btrim(p_schedule_interval) = '' THEN
        RAISE EXCEPTION 'schedule_interval must not be empty';
    END IF;
    IF p_max_concurrency IS NULL OR p_max_concurrency <= 0 THEN
        RAISE EXCEPTION 'max_concurrency must be greater than zero';
    END IF;
    IF p_tenant_budget_tokens IS NOT NULL AND p_tenant_budget_tokens <= 0 THEN
        RAISE EXCEPTION 'tenant_budget_tokens must be greater than zero';
    END IF;

    queue_table := ('vectorizer_queue_' || substr(md5(btrim(p_vectorizer_name)), 1, 16))::name;
    rendered_sql := format(
        'SELECT ai.create_vectorizer(%L, loading => ai.loading_table(%L, %L, %L), chunking => ai.chunking_recursive_text(%s, %s), embedding => ai.embedding_provider(%L, %L, %L), destination => ai.destination_table(%L, %L, %s), scheduling => ai.scheduling_interval(%L));',
        btrim(p_vectorizer_name),
        source_regclass::text,
        btrim(p_source_pk),
        btrim(p_source_column),
        p_chunk_max_tokens,
        p_chunk_overlap_tokens,
        normalized_provider,
        btrim(p_model),
        btrim(p_secret_ref),
        btrim(p_destination_table),
        btrim(p_destination_column),
        p_dimensions,
        btrim(p_schedule_interval)
    );

    EXECUTE format(
        'CREATE TABLE IF NOT EXISTS companion_internal.%I (tenant_id text NOT NULL, source_pk text NOT NULL, source_text text NOT NULL, enqueued_at timestamptz NOT NULL DEFAULT now())',
        queue_table
    );

    INSERT INTO companion_internal.vectorizer_definitions(
        vectorizer_name,
        source_table,
        source_pk,
        source_column,
        chunk_max_tokens,
        chunk_overlap_tokens,
        provider,
        model,
        secret_ref,
        destination_table,
        destination_column,
        dimensions,
        schedule_interval,
        max_concurrency,
        tenant_budget_tokens,
        queue_table,
        create_sql
    )
    VALUES (
        btrim(p_vectorizer_name),
        source_regclass::text,
        btrim(p_source_pk)::name,
        btrim(p_source_column)::name,
        p_chunk_max_tokens,
        p_chunk_overlap_tokens,
        normalized_provider,
        btrim(p_model),
        btrim(p_secret_ref),
        btrim(p_destination_table),
        btrim(p_destination_column)::name,
        p_dimensions,
        btrim(p_schedule_interval),
        p_max_concurrency,
        p_tenant_budget_tokens,
        queue_table,
        rendered_sql
    )
    ON CONFLICT (vectorizer_name) DO UPDATE
    SET source_table = EXCLUDED.source_table,
        source_pk = EXCLUDED.source_pk,
        source_column = EXCLUDED.source_column,
        chunk_max_tokens = EXCLUDED.chunk_max_tokens,
        chunk_overlap_tokens = EXCLUDED.chunk_overlap_tokens,
        provider = EXCLUDED.provider,
        model = EXCLUDED.model,
        secret_ref = EXCLUDED.secret_ref,
        destination_table = EXCLUDED.destination_table,
        destination_column = EXCLUDED.destination_column,
        dimensions = EXCLUDED.dimensions,
        schedule_interval = EXCLUDED.schedule_interval,
        max_concurrency = EXCLUDED.max_concurrency,
        tenant_budget_tokens = EXCLUDED.tenant_budget_tokens,
        queue_table = EXCLUDED.queue_table,
        create_sql = EXCLUDED.create_sql,
        created_at = now();

    RETURN rendered_sql;
END;
$$;

CREATE FUNCTION companion_internal.vectorizer_enqueue(
    p_vectorizer_name text,
    p_tenant_id text,
    p_source_pk text,
    p_source_text text
)
RETURNS void
LANGUAGE plpgsql
VOLATILE
AS $$
DECLARE
    queue_table name;
BEGIN
    IF p_vectorizer_name IS NULL OR btrim(p_vectorizer_name) = '' THEN
        RAISE EXCEPTION 'vectorizer_name must not be empty';
    END IF;
    SELECT companion_internal.vectorizer_definitions.queue_table
    INTO queue_table
    FROM companion_internal.vectorizer_definitions
    WHERE vectorizer_name = btrim(p_vectorizer_name);
    IF NOT FOUND THEN
        RAISE EXCEPTION 'vectorizer is not registered: %', p_vectorizer_name;
    END IF;
    IF p_tenant_id IS NULL OR btrim(p_tenant_id) = '' THEN
        RAISE EXCEPTION 'tenant_id must not be empty';
    END IF;
    IF p_source_pk IS NULL OR btrim(p_source_pk) = '' THEN
        RAISE EXCEPTION 'source_pk must not be empty';
    END IF;
    IF p_source_text IS NULL OR btrim(p_source_text) = '' THEN
        RAISE EXCEPTION 'source_text must not be empty';
    END IF;

    EXECUTE format(
        'INSERT INTO companion_internal.%I(tenant_id, source_pk, source_text) VALUES (%L, %L, %L)',
        queue_table,
        btrim(p_tenant_id),
        btrim(p_source_pk),
        p_source_text
    );
END;
$$;

CREATE FUNCTION companion_internal.vectorizer_record_usage(
    p_vectorizer_name text,
    p_tenant_id text,
    p_tokens bigint
)
RETURNS bigint
LANGUAGE plpgsql
VOLATILE
AS $$
DECLARE
    usage_id bigint;
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM companion_internal.vectorizer_definitions
        WHERE vectorizer_name = btrim(p_vectorizer_name)
    ) THEN
        RAISE EXCEPTION 'vectorizer is not registered: %', p_vectorizer_name;
    END IF;
    IF p_tenant_id IS NULL OR btrim(p_tenant_id) = '' THEN
        RAISE EXCEPTION 'tenant_id must not be empty';
    END IF;
    IF p_tokens IS NULL OR p_tokens <= 0 THEN
        RAISE EXCEPTION 'tokens must be greater than zero';
    END IF;

    INSERT INTO companion_internal.vectorizer_usage_log(
        vectorizer_name,
        tenant_id,
        tokens
    )
    VALUES (
        btrim(p_vectorizer_name),
        btrim(p_tenant_id),
        p_tokens
    )
    RETURNING companion_internal.vectorizer_usage_log.usage_id
    INTO usage_id;

    RETURN usage_id;
END;
$$;

-- FEATURE: A9
-- FEATURE: A10
-- FEATURE: A11
CREATE VIEW companion_ai_provider_bindings AS
SELECT
    binding_name,
    tenant_id,
    provider,
    model,
    max_tokens_per_request,
    enabled,
    (secret_ref IS NOT NULL AND btrim(secret_ref) <> '') AS has_secret_ref,
    substr(md5(secret_ref), 1, 16) AS secret_ref_fingerprint,
    created_at,
    updated_at
FROM companion_internal.ai_provider_bindings;

CREATE VIEW companion_semantic_catalog_objects AS
SELECT
    tenant_id,
    object_name,
    relation_name,
    allowed_columns,
    description,
    created_at,
    updated_at
FROM companion_internal.semantic_catalog_objects;

CREATE FUNCTION companion_internal.ai_identifier_is_safe(p_value text)
RETURNS boolean
LANGUAGE sql
IMMUTABLE
PARALLEL SAFE
AS $$
    SELECT p_value IS NOT NULL
       AND p_value ~ '^[A-Za-z_][A-Za-z0-9_]*$'
$$;

CREATE FUNCTION companion_internal.register_ai_provider_binding(
    p_binding_name text,
    p_tenant_id text,
    p_provider text,
    p_model text,
    p_secret_ref text,
    p_max_tokens_per_request integer,
    p_enabled boolean DEFAULT true
)
RETURNS text
LANGUAGE plpgsql
VOLATILE
AS $$
DECLARE
    normalized_provider text;
BEGIN
    IF p_binding_name IS NULL OR btrim(p_binding_name) = '' THEN
        RAISE EXCEPTION 'binding_name must not be empty';
    END IF;
    IF p_tenant_id IS NULL OR btrim(p_tenant_id) = '' THEN
        RAISE EXCEPTION 'tenant_id must not be empty';
    END IF;
    normalized_provider := lower(btrim(p_provider));
    IF normalized_provider NOT IN ('openai', 'azure_openai', 'anthropic', 'cohere', 'voyage', 'ollama', 'vertex_ai') THEN
        RAISE EXCEPTION 'unsupported AI provider: %', p_provider;
    END IF;
    IF p_model IS NULL OR btrim(p_model) = '' THEN
        RAISE EXCEPTION 'model must not be empty';
    END IF;
    IF p_secret_ref IS NULL OR btrim(p_secret_ref) = '' THEN
        RAISE EXCEPTION 'secret_ref must not be empty';
    END IF;
    IF btrim(p_secret_ref) !~ '^(secret|external-secret)://[A-Za-z0-9._/@:-]+$' THEN
        RAISE EXCEPTION 'secret_ref must use secret:// or external-secret:// URI form';
    END IF;
    IF p_max_tokens_per_request IS NULL OR p_max_tokens_per_request <= 0 OR p_max_tokens_per_request > 200000 THEN
        RAISE EXCEPTION 'max_tokens_per_request must be between 1 and 200000';
    END IF;

    INSERT INTO companion_internal.ai_provider_bindings(
        binding_name,
        tenant_id,
        provider,
        model,
        secret_ref,
        max_tokens_per_request,
        enabled
    )
    VALUES (
        btrim(p_binding_name),
        btrim(p_tenant_id),
        normalized_provider,
        btrim(p_model),
        btrim(p_secret_ref),
        p_max_tokens_per_request,
        COALESCE(p_enabled, true)
    )
    ON CONFLICT (binding_name) DO UPDATE
    SET tenant_id = EXCLUDED.tenant_id,
        provider = EXCLUDED.provider,
        model = EXCLUDED.model,
        secret_ref = EXCLUDED.secret_ref,
        max_tokens_per_request = EXCLUDED.max_tokens_per_request,
        enabled = EXCLUDED.enabled,
        updated_at = now();

    RETURN btrim(p_binding_name);
END;
$$;

CREATE FUNCTION companion_internal.ai_provider_binding_for_tenant(
    p_tenant_id text,
    p_binding_name text
)
RETURNS companion_internal.ai_provider_bindings
LANGUAGE plpgsql
STABLE
AS $$
DECLARE
    binding companion_internal.ai_provider_bindings%ROWTYPE;
BEGIN
    IF p_tenant_id IS NULL OR btrim(p_tenant_id) = '' THEN
        RAISE EXCEPTION 'tenant_id must not be empty';
    END IF;
    IF p_binding_name IS NULL OR btrim(p_binding_name) = '' THEN
        RAISE EXCEPTION 'binding_name must not be empty';
    END IF;
    SELECT *
    INTO binding
    FROM companion_internal.ai_provider_bindings
    WHERE binding_name = btrim(p_binding_name);
    IF NOT FOUND THEN
        RAISE EXCEPTION 'AI provider binding is not registered: %', p_binding_name;
    END IF;
    IF binding.tenant_id <> btrim(p_tenant_id) THEN
        RAISE EXCEPTION 'AI provider binding tenant mismatch';
    END IF;
    IF NOT binding.enabled THEN
        RAISE EXCEPTION 'AI provider binding is disabled: %', p_binding_name;
    END IF;
    RETURN binding;
END;
$$;

CREATE FUNCTION companion_internal.validate_ai_messages(p_messages jsonb)
RETURNS void
LANGUAGE plpgsql
STABLE
AS $$
DECLARE
    message jsonb;
    role text;
    content text;
BEGIN
    IF p_messages IS NULL OR jsonb_typeof(p_messages) <> 'array' OR jsonb_array_length(p_messages) = 0 THEN
        RAISE EXCEPTION 'messages must be a non-empty JSON array';
    END IF;
    IF jsonb_array_length(p_messages) > 64 THEN
        RAISE EXCEPTION 'messages must contain at most 64 entries';
    END IF;
    FOR message IN SELECT value FROM jsonb_array_elements(p_messages) AS value LOOP
        IF jsonb_typeof(message) <> 'object' THEN
            RAISE EXCEPTION 'each message must be a JSON object';
        END IF;
        role := message ->> 'role';
        content := message ->> 'content';
        IF role NOT IN ('system', 'user', 'assistant') THEN
            RAISE EXCEPTION 'unsupported chat message role: %', COALESCE(role, '<null>');
        END IF;
        IF content IS NULL OR btrim(content) = '' THEN
            RAISE EXCEPTION 'chat message content must not be empty';
        END IF;
        IF length(content) > 32768 THEN
            RAISE EXCEPTION 'chat message content exceeds 32768 characters';
        END IF;
    END LOOP;
END;
$$;

CREATE FUNCTION companion_ai_chat_stream(
    p_tenant_id text,
    p_binding_name text,
    p_messages jsonb,
    p_max_output_tokens integer DEFAULT 1024,
    p_temperature numeric DEFAULT 0,
    p_allow_provider_execution boolean DEFAULT false
)
RETURNS TABLE(chunk_index integer, event text, payload jsonb)
LANGUAGE plpgsql
STABLE
AS $$
DECLARE
    binding companion_internal.ai_provider_bindings%ROWTYPE;
    intent jsonb;
BEGIN
    binding := companion_internal.ai_provider_binding_for_tenant(p_tenant_id, p_binding_name);
    PERFORM companion_internal.validate_ai_messages(p_messages);
    IF p_max_output_tokens IS NULL OR p_max_output_tokens <= 0 THEN
        RAISE EXCEPTION 'max_output_tokens must be greater than zero';
    END IF;
    IF p_max_output_tokens > binding.max_tokens_per_request THEN
        RAISE EXCEPTION 'max_output_tokens exceeds binding limit';
    END IF;
    IF p_temperature IS NULL OR p_temperature < 0 OR p_temperature > 2 THEN
        RAISE EXCEPTION 'temperature must be between 0 and 2';
    END IF;
    IF p_allow_provider_execution THEN
        RAISE EXCEPTION 'AI provider runtime is unavailable; this SQL surface emits request intent only';
    END IF;

    intent := jsonb_build_object(
        'feature_id', 'A10',
        'evidence_boundary', 'sql-intent-fail-closed-only',
        'provider_runtime_available', false,
        'provider_execution_requested', false,
        'tenant_id', btrim(p_tenant_id),
        'binding_name', binding.binding_name,
        'provider', binding.provider,
        'model', binding.model,
        'secret_bound', true,
        'messages_count', jsonb_array_length(p_messages),
        'max_output_tokens', p_max_output_tokens,
        'temperature', p_temperature
    );

    RETURN QUERY SELECT 0, 'request_intent', intent;
END;
$$;

CREATE FUNCTION companion_internal.register_semantic_catalog_object(
    p_tenant_id text,
    p_object_name text,
    p_relation_name text,
    p_allowed_columns text[],
    p_description text
)
RETURNS text
LANGUAGE plpgsql
VOLATILE
AS $$
DECLARE
    relation_regclass regclass;
    normalized_columns text[];
    column_name text;
BEGIN
    IF p_tenant_id IS NULL OR btrim(p_tenant_id) = '' THEN
        RAISE EXCEPTION 'tenant_id must not be empty';
    END IF;
    IF p_object_name IS NULL OR NOT companion_internal.ai_identifier_is_safe(btrim(p_object_name)) THEN
        RAISE EXCEPTION 'object_name must be a SQL identifier';
    END IF;
    IF p_relation_name IS NULL OR btrim(p_relation_name) = '' THEN
        RAISE EXCEPTION 'relation_name must not be empty';
    END IF;
    relation_regclass := p_relation_name::regclass;
    IF p_allowed_columns IS NULL OR cardinality(p_allowed_columns) = 0 THEN
        RAISE EXCEPTION 'allowed_columns must contain at least one column';
    END IF;
    normalized_columns := ARRAY(
        SELECT DISTINCT btrim(value)
        FROM unnest(p_allowed_columns) AS value
        WHERE btrim(value) <> ''
        ORDER BY btrim(value)
    );
    IF cardinality(normalized_columns) = 0 THEN
        RAISE EXCEPTION 'allowed_columns must contain at least one column';
    END IF;
    FOREACH column_name IN ARRAY normalized_columns LOOP
        IF NOT companion_internal.ai_identifier_is_safe(column_name) THEN
            RAISE EXCEPTION 'allowed column must be a SQL identifier: %', column_name;
        END IF;
        IF NOT companion_internal.table_has_column(relation_regclass::text, column_name::name) THEN
            RAISE EXCEPTION 'allowed column does not exist on relation: %', column_name;
        END IF;
    END LOOP;
    IF p_description IS NULL OR btrim(p_description) = '' THEN
        RAISE EXCEPTION 'description must not be empty';
    END IF;
    IF length(p_description) > 4096 THEN
        RAISE EXCEPTION 'description exceeds 4096 characters';
    END IF;

    INSERT INTO companion_internal.semantic_catalog_objects(
        tenant_id,
        object_name,
        relation_name,
        allowed_columns,
        description
    )
    VALUES (
        btrim(p_tenant_id),
        btrim(p_object_name),
        relation_regclass::text,
        normalized_columns,
        btrim(p_description)
    )
    ON CONFLICT (tenant_id, object_name) DO UPDATE
    SET relation_name = EXCLUDED.relation_name,
        allowed_columns = EXCLUDED.allowed_columns,
        description = EXCLUDED.description,
        updated_at = now();

    RETURN btrim(p_object_name);
END;
$$;

CREATE FUNCTION companion_semantic_text_to_sql_intent(
    p_tenant_id text,
    p_question text,
    p_catalog_objects text[],
    p_binding_name text DEFAULT NULL,
    p_allow_query_execution boolean DEFAULT false
)
RETURNS jsonb
LANGUAGE plpgsql
STABLE
AS $$
DECLARE
    object_count integer;
    object_record record;
    selected_columns text;
    template_sql text;
    binding companion_internal.ai_provider_bindings%ROWTYPE;
BEGIN
    IF p_tenant_id IS NULL OR btrim(p_tenant_id) = '' THEN
        RAISE EXCEPTION 'tenant_id must not be empty';
    END IF;
    IF p_question IS NULL OR btrim(p_question) = '' THEN
        RAISE EXCEPTION 'question must not be empty';
    END IF;
    IF length(p_question) > 4000 THEN
        RAISE EXCEPTION 'question exceeds 4000 characters';
    END IF;
    IF p_question ~ '[;]' OR p_question ~* '\m(drop|alter|truncate|delete|insert|update|copy|grant|revoke)\M' THEN
        RAISE EXCEPTION 'question contains unsupported SQL-control text';
    END IF;
    IF p_catalog_objects IS NULL OR cardinality(p_catalog_objects) = 0 THEN
        RAISE EXCEPTION 'catalog_objects must contain at least one object';
    END IF;
    IF p_allow_query_execution THEN
        RAISE EXCEPTION 'text-to-SQL execution is unavailable; this SQL surface emits request intent only';
    END IF;
    IF p_binding_name IS NOT NULL AND btrim(p_binding_name) <> '' THEN
        binding := companion_internal.ai_provider_binding_for_tenant(p_tenant_id, p_binding_name);
    END IF;

    SELECT count(*)
    INTO object_count
    FROM unnest(p_catalog_objects) AS requested(object_name)
    JOIN companion_internal.semantic_catalog_objects AS catalog
      ON catalog.tenant_id = btrim(p_tenant_id)
     AND catalog.object_name = btrim(requested.object_name);
    IF object_count <> cardinality(p_catalog_objects) THEN
        RAISE EXCEPTION 'all catalog_objects must be registered for tenant';
    END IF;
    IF object_count <> 1 THEN
        RAISE EXCEPTION 'exactly one catalog object is supported by this deterministic intent boundary';
    END IF;

    SELECT catalog.*
    INTO object_record
    FROM unnest(p_catalog_objects) AS requested(object_name)
    JOIN companion_internal.semantic_catalog_objects AS catalog
      ON catalog.tenant_id = btrim(p_tenant_id)
     AND catalog.object_name = btrim(requested.object_name)
    LIMIT 1;

    SELECT string_agg(format('%I', column_name), ', ' ORDER BY column_name)
    INTO selected_columns
    FROM unnest(object_record.allowed_columns) AS column_name;
    template_sql := format(
        'SELECT %s FROM %s WHERE tenant_id = $1 LIMIT 100',
        selected_columns,
        object_record.relation_name::regclass
    );

    RETURN jsonb_build_object(
        'feature_id', 'A11',
        'evidence_boundary', 'sql-intent-fail-closed-only',
        'provider_runtime_available', false,
        'query_execution_requested', false,
        'tenant_id', btrim(p_tenant_id),
        'question', btrim(p_question),
        'catalog_objects', to_jsonb(p_catalog_objects),
        'relation_name', object_record.relation_name,
        'allowed_columns', to_jsonb(object_record.allowed_columns),
        'sql_template', template_sql,
        'binding_name', NULLIF(btrim(COALESCE(p_binding_name, '')), ''),
        'provider', CASE WHEN binding.binding_name IS NULL THEN NULL ELSE binding.provider END,
        'model', CASE WHEN binding.binding_name IS NULL THEN NULL ELSE binding.model END,
        'execution_allowed', false
    );
END;
$$;

-- FEATURE: TS9
-- FEATURE: M7
CREATE VIEW companion_db_doctor_rules AS
SELECT
    rule_id,
    severity,
    enabled,
    updated_at
FROM companion_internal.db_doctor_rules;

CREATE VIEW companion_db_doctor_violations AS
SELECT
    violation_id,
    rule_id,
    severity,
    object_name,
    message,
    detected_at
FROM companion_internal.db_doctor_violations;

CREATE FUNCTION companion_internal.assert_shared_preload_libraries(
    p_loaded_libraries text[],
    p_required_libraries text[]
)
RETURNS void
LANGUAGE plpgsql
STABLE
AS $$
DECLARE
    required_library text;
BEGIN
    IF p_loaded_libraries IS NULL OR cardinality(p_loaded_libraries) = 0 THEN
        RAISE EXCEPTION 'shared_preload_libraries must not be empty';
    END IF;
    IF NOT EXISTS (
        SELECT 1
        FROM unnest(p_loaded_libraries) AS loaded(library_name)
        WHERE lower(btrim(library_name)) = 'citus'
    ) THEN
        RAISE EXCEPTION 'citus must be preloaded';
    END IF;
    IF p_required_libraries IS NULL OR cardinality(p_required_libraries) = 0 THEN
        RAISE EXCEPTION 'required_extensions must not be empty';
    END IF;

    FOREACH required_library IN ARRAY p_required_libraries LOOP
        IF required_library IS NULL OR btrim(required_library) = '' THEN
            RAISE EXCEPTION 'required_extensions must not contain empty values';
        END IF;
        IF NOT EXISTS (
            SELECT 1
            FROM unnest(p_loaded_libraries) AS loaded(library_name)
            WHERE lower(btrim(library_name)) = lower(btrim(required_library))
        ) THEN
            RAISE EXCEPTION 'required cohabiting extension is not preloaded';
        END IF;
    END LOOP;
END;
$$;

CREATE FUNCTION companion_internal.assert_citus_cohabit_extension_order(
    p_loaded_libraries text[]
)
RETURNS void
LANGUAGE plpgsql
STABLE
AS $$
DECLARE
    citus_position integer;
    library_count integer;
BEGIN
    IF p_loaded_libraries IS NULL OR cardinality(p_loaded_libraries) = 0 THEN
        RAISE EXCEPTION 'shared_preload_libraries must not be empty';
    END IF;
    SELECT array_position(
        ARRAY(SELECT lower(btrim(library_name)) FROM unnest(p_loaded_libraries) AS loaded(library_name)),
        'citus'
    )
    INTO citus_position;
    library_count := cardinality(p_loaded_libraries);
    IF citus_position IS NULL THEN
        RAISE EXCEPTION 'citus must be preloaded';
    END IF;
    IF citus_position <> library_count THEN
        RAISE EXCEPTION 'citus must be loaded after trusted cohabiting extensions';
    END IF;
END;
$$;

CREATE FUNCTION companion_internal.assert_citus_cohabit_extension_order()
RETURNS void
LANGUAGE plpgsql
STABLE
AS $$
BEGIN
    PERFORM companion_internal.assert_citus_cohabit_extension_order(
        string_to_array(current_setting('shared_preload_libraries', true), ',')
    );
END;
$$;

CREATE FUNCTION companion_internal.get_violations(
    p_schemas text[],
    p_rules text[]
)
RETURNS jsonb
LANGUAGE plpgsql
VOLATILE
AS $$
DECLARE
    schema_name text;
    rule_name text;
    result jsonb;
BEGIN
    IF p_schemas IS NULL OR cardinality(p_schemas) = 0 THEN
        RAISE EXCEPTION 'schemas must not be empty';
    END IF;
    IF p_rules IS NULL OR cardinality(p_rules) = 0 THEN
        RAISE EXCEPTION 'rules must not be empty';
    END IF;

    FOREACH rule_name IN ARRAY p_rules LOOP
        rule_name := lower(btrim(rule_name));
        IF rule_name NOT IN (
            'cohabit_extensions',
            'non_colocated_join',
            'missing_distribution_column',
            'hypertable_bridge',
            'chunk_interval_out_of_band'
        ) THEN
            RAISE EXCEPTION 'unsupported doctor rule: %', rule_name;
        END IF;
        INSERT INTO companion_internal.db_doctor_rules(rule_id, severity)
        VALUES (rule_name, CASE WHEN rule_name = 'cohabit_extensions' THEN 'error' ELSE 'warning' END)
        ON CONFLICT (rule_id) DO UPDATE
        SET severity = EXCLUDED.severity,
            enabled = true,
            updated_at = now();
    END LOOP;

    FOREACH schema_name IN ARRAY p_schemas LOOP
        schema_name := btrim(schema_name);
        IF schema_name = '' THEN
            RAISE EXCEPTION 'schemas must not contain empty values';
        END IF;
        IF NOT EXISTS (
            SELECT 1
            FROM pg_namespace
            WHERE nspname = schema_name
        ) THEN
            INSERT INTO companion_internal.db_doctor_violations(
                rule_id,
                severity,
                object_name,
                message
            )
            VALUES (
                'missing_schema',
                'error',
                schema_name,
                'schema does not exist'
            );
        END IF;
    END LOOP;

    SELECT COALESCE(
        jsonb_agg(
            jsonb_build_object(
                'rule_id', rule_id,
                'severity', severity,
                'object_name', object_name,
                'message', message
            )
            ORDER BY violation_id
        ),
        '[]'::jsonb
    )
    INTO result
    FROM companion_internal.db_doctor_violations
    WHERE detected_at >= statement_timestamp() - interval '5 seconds';

    RETURN result;
END;
$$;

-- FEATURE: T8
-- FEATURE: L9
-- FEATURE: TS13
-- FEATURE: TS14
-- FEATURE: TS15
-- FEATURE: TS16
-- FEATURE: TS17
CREATE VIEW companion_toolkit_aggregate_plans AS
SELECT
    plan_id,
    feature_id,
    aggregate_kind,
    source_table,
    worker_view,
    coordinator_view,
    distribution_column,
    value_column,
    time_column,
    bucket_width,
    worker_sql,
    coordinator_sql,
    created_at
FROM companion_internal.toolkit_aggregate_plans;

CREATE FUNCTION companion_internal.toolkit_feature_id(p_aggregate_kind text)
RETURNS text
LANGUAGE plpgsql
IMMUTABLE
PARALLEL SAFE
AS $$
DECLARE
    normalized_kind text := lower(btrim(p_aggregate_kind));
BEGIN
    IF normalized_kind = 'time_bucket_gapfill' THEN
        RETURN 'TS13';
    ELSIF normalized_kind IN ('counter_agg', 'gauge_agg', 'heartbeat_agg') THEN
        RETURN 'TS14';
    ELSIF normalized_kind IN ('percentile_agg', 'freq_agg') THEN
        RETURN 'TS15';
    ELSIF normalized_kind IN ('asap_smooth', 'lttb') THEN
        RETURN 'TS16';
    ELSIF normalized_kind IN ('candlestick_agg', 'state_agg', 'range_agg') THEN
        RETURN 'TS17';
    ELSIF normalized_kind IN ('hyperloglog', 'tdigest', 'time_weight') THEN
        RETURN 'T8';
    END IF;

    RAISE EXCEPTION 'unsupported toolkit aggregate: %', p_aggregate_kind;
END;
$$;

CREATE FUNCTION companion_internal.register_toolkit_aggregate_plan(
    p_source_table text,
    p_worker_view name,
    p_coordinator_view name,
    p_distribution_column text,
    p_value_column text,
    p_aggregate_kind text,
    p_time_column text DEFAULT NULL,
    p_bucket_width text DEFAULT NULL
)
RETURNS text
LANGUAGE plpgsql
VOLATILE
AS $$
DECLARE
    source_regclass regclass;
    normalized_kind text;
    feature_id text;
    partial_expression text;
    finalize_expression text;
    worker_sql text;
    coordinator_sql text;
BEGIN
    IF p_source_table IS NULL OR btrim(p_source_table) = '' THEN
        RAISE EXCEPTION 'source_table must not be empty';
    END IF;
    source_regclass := p_source_table::regclass;
    IF p_worker_view IS NULL OR btrim(p_worker_view::text) = '' THEN
        RAISE EXCEPTION 'worker_view must not be empty';
    END IF;
    IF p_coordinator_view IS NULL OR btrim(p_coordinator_view::text) = '' THEN
        RAISE EXCEPTION 'coordinator_view must not be empty';
    END IF;
    IF p_distribution_column IS NULL OR btrim(p_distribution_column) = '' THEN
        RAISE EXCEPTION 'distribution_column must not be empty';
    END IF;
    IF NOT companion_internal.table_has_column(source_regclass::text, btrim(p_distribution_column)::name) THEN
        RAISE EXCEPTION 'distribution_column does not exist on source table';
    END IF;
    IF p_value_column IS NULL OR btrim(p_value_column) = '' THEN
        RAISE EXCEPTION 'value_column must not be empty';
    END IF;
    IF NOT companion_internal.table_has_column(source_regclass::text, btrim(p_value_column)::name) THEN
        RAISE EXCEPTION 'value_column does not exist on source table';
    END IF;
    normalized_kind := lower(btrim(p_aggregate_kind));
    feature_id := companion_internal.toolkit_feature_id(normalized_kind);

    IF normalized_kind IN ('time_bucket_gapfill', 'asap_smooth', 'lttb', 'candlestick_agg', 'time_weight') THEN
        IF p_time_column IS NULL OR btrim(p_time_column) = '' THEN
            RAISE EXCEPTION 'time_column must not be empty for aggregate %', normalized_kind;
        END IF;
        IF NOT companion_internal.table_has_column(source_regclass::text, btrim(p_time_column)::name) THEN
            RAISE EXCEPTION 'time_column does not exist on source table';
        END IF;
    END IF;
    IF normalized_kind = 'time_bucket_gapfill' AND (p_bucket_width IS NULL OR btrim(p_bucket_width) = '') THEN
        RAISE EXCEPTION 'bucket_width must not be empty for time_bucket_gapfill';
    END IF;

    partial_expression := CASE normalized_kind
        WHEN 'time_bucket_gapfill' THEN format(
            'time_bucket_gapfill(%L, %I) WITHIN GROUP (ORDER BY %I)',
            btrim(p_bucket_width),
            btrim(p_time_column),
            btrim(p_time_column)
        )
        WHEN 'counter_agg' THEN format('counter_agg(%I)', btrim(p_value_column))
        WHEN 'gauge_agg' THEN format('gauge_agg(%I)', btrim(p_value_column))
        WHEN 'heartbeat_agg' THEN format('heartbeat_agg(%I)', btrim(p_value_column))
        WHEN 'percentile_agg' THEN format('percentile_agg(%I)', btrim(p_value_column))
        WHEN 'freq_agg' THEN format('freq_agg(%I)', btrim(p_value_column))
        WHEN 'hyperloglog' THEN format('hyperloglog(%I)', btrim(p_value_column))
        WHEN 'tdigest' THEN format('tdigest(%I)', btrim(p_value_column))
        WHEN 'asap_smooth' THEN format('asap_smooth(%I, %I)', btrim(p_time_column), btrim(p_value_column))
        WHEN 'lttb' THEN format('lttb(%I, %I)', btrim(p_time_column), btrim(p_value_column))
        WHEN 'candlestick_agg' THEN format('candlestick_agg(%I, %I)', btrim(p_time_column), btrim(p_value_column))
        WHEN 'state_agg' THEN format('state_agg(%I)', btrim(p_value_column))
        WHEN 'range_agg' THEN format('range_agg(%I)', btrim(p_value_column))
        WHEN 'time_weight' THEN format('time_weight(%I, %I)', btrim(p_time_column), btrim(p_value_column))
    END;

    finalize_expression := CASE normalized_kind
        WHEN 'time_bucket_gapfill' THEN 'locf(interpolate(partial_state))'
        WHEN 'heartbeat_agg' THEN 'heartbeat_agg_rollup(partial_state)'
        WHEN 'percentile_agg' THEN 'approx_percentile(0.99, rollup(partial_state))'
        WHEN 'freq_agg' THEN 'topn(10, rollup(partial_state))'
        WHEN 'hyperloglog' THEN 'distinct_count(rollup(partial_state))'
        WHEN 'tdigest' THEN 'approx_percentile(0.99, rollup(partial_state))'
        WHEN 'asap_smooth' THEN 'asap_smooth_final(rollup(partial_state))'
        WHEN 'lttb' THEN 'lttb_final(rollup(partial_state))'
        WHEN 'time_weight' THEN 'average(rollup(partial_state))'
        ELSE 'rollup(partial_state)'
    END;

    worker_sql := format(
        'CREATE OR REPLACE VIEW %I AS SELECT %I AS distribution_key, %s AS partial_state FROM %s GROUP BY 1;',
        p_worker_view,
        btrim(p_distribution_column),
        partial_expression,
        source_regclass
    );
    coordinator_sql := format(
        'CREATE OR REPLACE VIEW %I AS SELECT distribution_key, %s AS aggregate_value FROM %I GROUP BY 1;',
        p_coordinator_view,
        finalize_expression,
        p_worker_view
    );

    INSERT INTO companion_internal.toolkit_aggregate_plans(
        feature_id,
        aggregate_kind,
        source_table,
        worker_view,
        coordinator_view,
        distribution_column,
        value_column,
        time_column,
        bucket_width,
        worker_sql,
        coordinator_sql
    )
    VALUES (
        feature_id,
        normalized_kind,
        source_regclass::text,
        p_worker_view,
        p_coordinator_view,
        btrim(p_distribution_column)::name,
        btrim(p_value_column)::name,
        NULLIF(btrim(COALESCE(p_time_column, '')), '')::name,
        NULLIF(btrim(COALESCE(p_bucket_width, '')), ''),
        worker_sql,
        coordinator_sql
    )
    ON CONFLICT (source_table, worker_view, coordinator_view, aggregate_kind) DO UPDATE
    SET feature_id = EXCLUDED.feature_id,
        distribution_column = EXCLUDED.distribution_column,
        value_column = EXCLUDED.value_column,
        time_column = EXCLUDED.time_column,
        bucket_width = EXCLUDED.bucket_width,
        worker_sql = EXCLUDED.worker_sql,
        coordinator_sql = EXCLUDED.coordinator_sql,
        created_at = now();

    RETURN worker_sql || E'\n' || coordinator_sql;
END;
$$;

-- FEATURE: C10
-- FEATURE: M2
CREATE VIEW companion_schema_jobs AS
SELECT
    job_name,
    table_name,
    state,
    lease_seconds,
    lease_expires_at,
    created_at,
    updated_at
FROM companion_internal.schema_jobs;

CREATE VIEW companion_schema_job_operations AS
SELECT
    operation_id,
    job_name,
    operation_type,
    column_name,
    sql_type,
    statement,
    new_column_name,
    rendered_sql,
    recorded_at
FROM companion_internal.schema_job_operations;

CREATE FUNCTION companion_internal.schema_job_start(
    p_job_name text,
    p_table_name text,
    p_lease_seconds integer
)
RETURNS void
LANGUAGE plpgsql
VOLATILE
AS $$
DECLARE
    existing_job record;
    table_regclass regclass;
BEGIN
    IF p_job_name IS NULL OR btrim(p_job_name) = '' THEN
        RAISE EXCEPTION 'job_name must not be empty';
    END IF;
    IF p_table_name IS NULL OR btrim(p_table_name) = '' THEN
        RAISE EXCEPTION 'table_name must not be empty';
    END IF;
    table_regclass := p_table_name::regclass;
    IF p_lease_seconds IS NULL OR p_lease_seconds <= 0 THEN
        RAISE EXCEPTION 'lease_seconds must be greater than zero';
    END IF;

    SELECT *
    INTO existing_job
    FROM companion_internal.schema_jobs
    WHERE job_name = btrim(p_job_name)
    FOR UPDATE;

    IF FOUND THEN
        IF existing_job.table_name <> table_regclass::text THEN
            RAISE EXCEPTION 'schema job re-entry conflicts with existing table: %', p_job_name;
        END IF;
        IF existing_job.state <> 'delete_only' THEN
            RAISE EXCEPTION 'schema job cannot restart from state %', existing_job.state;
        END IF;

        UPDATE companion_internal.schema_jobs
        SET lease_seconds = p_lease_seconds,
            lease_expires_at = now() + make_interval(secs => p_lease_seconds),
            updated_at = now()
        WHERE job_name = btrim(p_job_name)
          AND state = 'delete_only';

        RETURN;
    END IF;

    INSERT INTO companion_internal.schema_jobs(
        job_name,
        table_name,
        state,
        lease_seconds,
        lease_expires_at
    )
    VALUES (
        btrim(p_job_name),
        table_regclass::text,
        'delete_only',
        p_lease_seconds,
        now() + make_interval(secs => p_lease_seconds)
    );
END;
$$;

CREATE FUNCTION companion_internal.schema_job_add_operation(
    p_job_name text,
    p_operation_type text,
    p_column_name text DEFAULT NULL,
    p_sql_type text DEFAULT NULL,
    p_statement text DEFAULT NULL,
    p_new_column_name text DEFAULT NULL
)
RETURNS text
LANGUAGE plpgsql
VOLATILE
AS $$
DECLARE
    job_table text;
    job_state text;
    normalized_operation text;
    existing_sql text;
    rendered_sql text;
BEGIN
    IF p_job_name IS NULL OR btrim(p_job_name) = '' THEN
        RAISE EXCEPTION 'job_name must not be empty';
    END IF;
    SELECT table_name, state
    INTO job_table, job_state
    FROM companion_internal.schema_jobs
    WHERE job_name = btrim(p_job_name)
      AND state <> 'canceled';
    IF NOT FOUND THEN
        RAISE EXCEPTION 'schema job is not registered: %', p_job_name;
    END IF;
    IF job_state <> 'delete_only' THEN
        RAISE EXCEPTION 'schema job operations cannot be changed after delete_only: %', p_job_name;
    END IF;
    normalized_operation := lower(btrim(p_operation_type));
    IF normalized_operation NOT IN ('add_column', 'backfill', 'swap_column', 'drop_column') THEN
        RAISE EXCEPTION 'unsupported schema job operation: %', p_operation_type;
    END IF;

    IF normalized_operation = 'add_column' THEN
        IF p_column_name IS NULL OR btrim(p_column_name) = '' THEN
            RAISE EXCEPTION 'column_name must not be empty';
        END IF;
        IF p_sql_type IS NULL OR btrim(p_sql_type) = '' THEN
            RAISE EXCEPTION 'sql_type must not be empty';
        END IF;
        rendered_sql := format(
            'ALTER TABLE %s ADD COLUMN IF NOT EXISTS %I %s;',
            job_table,
            btrim(p_column_name),
            btrim(p_sql_type)
        );
    ELSIF normalized_operation = 'backfill' THEN
        IF p_statement IS NULL OR btrim(p_statement) = '' THEN
            RAISE EXCEPTION 'statement must not be empty';
        END IF;
        rendered_sql := btrim(p_statement);
    ELSIF normalized_operation = 'swap_column' THEN
        IF p_column_name IS NULL OR btrim(p_column_name) = '' THEN
            RAISE EXCEPTION 'column_name must not be empty';
        END IF;
        IF p_new_column_name IS NULL OR btrim(p_new_column_name) = '' THEN
            RAISE EXCEPTION 'new_column_name must not be empty';
        END IF;
        rendered_sql := format(
            'ALTER TABLE %s RENAME COLUMN %I TO %I;',
            job_table,
            btrim(p_column_name),
            btrim(p_new_column_name)
        );
    ELSE
        IF p_column_name IS NULL OR btrim(p_column_name) = '' THEN
            RAISE EXCEPTION 'column_name must not be empty';
        END IF;
        rendered_sql := format(
            'ALTER TABLE %s DROP COLUMN IF EXISTS %I;',
            job_table,
            btrim(p_column_name)
        );
    END IF;

    SELECT op.rendered_sql
    INTO existing_sql
    FROM companion_internal.schema_job_operations AS op
    WHERE op.job_name = btrim(p_job_name)
      AND op.operation_type = normalized_operation
      AND op.column_name IS NOT DISTINCT FROM NULLIF(btrim(COALESCE(p_column_name, '')), '')
      AND op.sql_type IS NOT DISTINCT FROM NULLIF(btrim(COALESCE(p_sql_type, '')), '')
      AND op.statement IS NOT DISTINCT FROM NULLIF(btrim(COALESCE(p_statement, '')), '')
      AND op.new_column_name IS NOT DISTINCT FROM NULLIF(btrim(COALESCE(p_new_column_name, '')), '')
    ORDER BY op.operation_id
    LIMIT 1;

    IF FOUND THEN
        RETURN existing_sql;
    END IF;

    IF EXISTS (
        SELECT 1
        FROM companion_internal.schema_job_operations AS op
        WHERE op.job_name = btrim(p_job_name)
          AND op.operation_type = normalized_operation
          AND op.column_name IS NOT DISTINCT FROM NULLIF(btrim(COALESCE(p_column_name, '')), '')
    ) THEN
        RAISE EXCEPTION 'schema job operation re-entry conflicts with existing operation: %.%', p_job_name, p_column_name;
    END IF;

    INSERT INTO companion_internal.schema_job_operations(
        job_name,
        operation_type,
        column_name,
        sql_type,
        statement,
        new_column_name,
        rendered_sql
    )
    VALUES (
        btrim(p_job_name),
        normalized_operation,
        NULLIF(btrim(COALESCE(p_column_name, '')), ''),
        NULLIF(btrim(COALESCE(p_sql_type, '')), ''),
        NULLIF(btrim(COALESCE(p_statement, '')), ''),
        NULLIF(btrim(COALESCE(p_new_column_name, '')), ''),
        rendered_sql
    );

    RETURN rendered_sql;
END;
$$;

CREATE FUNCTION companion_internal.schema_job_advance(
    p_job_name text,
    p_next_state text
)
RETURNS text
LANGUAGE plpgsql
VOLATILE
AS $$
DECLARE
    current_state text;
    normalized_next text;
BEGIN
    IF p_job_name IS NULL OR btrim(p_job_name) = '' THEN
        RAISE EXCEPTION 'job_name must not be empty';
    END IF;
    normalized_next := lower(btrim(p_next_state));
    IF normalized_next NOT IN ('delete_only', 'write_only', 'backfill', 'public', 'paused', 'canceled') THEN
        RAISE EXCEPTION 'unsupported schema job state: %', p_next_state;
    END IF;

    SELECT state
    INTO current_state
    FROM companion_internal.schema_jobs
    WHERE job_name = btrim(p_job_name)
    FOR UPDATE;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'schema job is not registered: %', p_job_name;
    END IF;

    IF current_state = normalized_next THEN
        RETURN normalized_next;
    END IF;

    IF NOT (
        (current_state = 'delete_only' AND normalized_next = 'write_only')
        OR (current_state = 'write_only' AND normalized_next = 'backfill')
        OR (current_state = 'backfill' AND normalized_next = 'public')
        OR normalized_next IN ('paused', 'canceled')
    ) THEN
        RAISE EXCEPTION 'invalid schema job transition: % -> %', current_state, normalized_next;
    END IF;

    UPDATE companion_internal.schema_jobs
    SET state = normalized_next,
        updated_at = now()
    WHERE job_name = btrim(p_job_name);

    RETURN normalized_next;
END;
$$;

CREATE FUNCTION companion_internal.schema_job_render_plan(p_job_name text)
RETURNS text
LANGUAGE plpgsql
STABLE
AS $$
DECLARE
    rendered_plan text;
BEGIN
    SELECT string_agg(rendered_sql, E'\n' ORDER BY operation_id)
    INTO rendered_plan
    FROM companion_internal.schema_job_operations
    WHERE job_name = btrim(p_job_name);

    IF rendered_plan IS NULL THEN
        RAISE EXCEPTION 'schema job has no operations: %', p_job_name;
    END IF;

    RETURN rendered_plan;
END;
$$;

-- FEATURE: S14
-- FEATURE: TO3
-- FEATURE: TO4
-- FEATURE: TO5
CREATE VIEW companion_tenant_moves AS
SELECT
    move_id,
    tenant_name,
    source_worker,
    target_worker,
    region_affinity,
    status,
    created_at
FROM companion_internal.tenant_moves;

CREATE VIEW companion_tenant_quotas AS
SELECT
    tenant_name,
    max_connections,
    max_qps,
    updated_at
FROM companion_internal.tenant_quotas;

CREATE VIEW companion_tenant_archives AS
SELECT
    archive_id,
    tenant_name,
    destination_uri,
    retention_days,
    status,
    created_at
FROM companion_internal.tenant_archives;

CREATE VIEW companion_tenant_region_affinities AS
SELECT
    tenant_name,
    region_affinity,
    updated_at
FROM companion_internal.tenant_region_affinities;

CREATE FUNCTION companion_internal.plan_tenant_move(
    p_tenant_name text,
    p_source_worker text,
    p_target_worker text,
    p_region_affinity text DEFAULT NULL
)
RETURNS bigint
LANGUAGE plpgsql
VOLATILE
AS $$
DECLARE
    move_id bigint;
BEGIN
    IF p_tenant_name IS NULL OR btrim(p_tenant_name) = '' THEN
        RAISE EXCEPTION 'tenant_name must not be empty';
    END IF;
    IF p_source_worker IS NULL OR btrim(p_source_worker) = '' THEN
        RAISE EXCEPTION 'source_worker must not be empty';
    END IF;
    IF p_target_worker IS NULL OR btrim(p_target_worker) = '' THEN
        RAISE EXCEPTION 'target_worker must not be empty';
    END IF;
    IF btrim(p_source_worker) = btrim(p_target_worker) THEN
        RAISE EXCEPTION 'source_worker and target_worker must differ';
    END IF;
    IF p_region_affinity IS NOT NULL AND btrim(p_region_affinity) = '' THEN
        RAISE EXCEPTION 'region_affinity must not be empty';
    END IF;

    INSERT INTO companion_internal.tenant_moves(
        tenant_name,
        source_worker,
        target_worker,
        region_affinity,
        status
    )
    VALUES (
        btrim(p_tenant_name),
        btrim(p_source_worker),
        btrim(p_target_worker),
        NULLIF(btrim(COALESCE(p_region_affinity, '')), ''),
        'queued'
    )
    RETURNING companion_internal.tenant_moves.move_id
    INTO move_id;

    RETURN move_id;
END;
$$;

CREATE FUNCTION companion_internal.set_tenant_quota(
    p_tenant_name text,
    p_max_connections integer,
    p_max_qps integer
)
RETURNS void
LANGUAGE plpgsql
VOLATILE
AS $$
BEGIN
    IF p_tenant_name IS NULL OR btrim(p_tenant_name) = '' THEN
        RAISE EXCEPTION 'tenant_name must not be empty';
    END IF;
    IF p_max_connections IS NULL OR p_max_connections <= 0 THEN
        RAISE EXCEPTION 'max_connections must be greater than zero';
    END IF;
    IF p_max_qps IS NULL OR p_max_qps <= 0 THEN
        RAISE EXCEPTION 'max_qps must be greater than zero';
    END IF;

    INSERT INTO companion_internal.tenant_quotas(
        tenant_name,
        max_connections,
        max_qps
    )
    VALUES (
        btrim(p_tenant_name),
        p_max_connections,
        p_max_qps
    )
    ON CONFLICT (tenant_name) DO UPDATE
    SET max_connections = EXCLUDED.max_connections,
        max_qps = EXCLUDED.max_qps,
        updated_at = now();
END;
$$;

CREATE FUNCTION companion_internal.plan_tenant_archive(
    p_tenant_name text,
    p_destination_uri text,
    p_retention_days integer
)
RETURNS bigint
LANGUAGE plpgsql
VOLATILE
AS $$
DECLARE
    archive_id bigint;
BEGIN
    IF p_tenant_name IS NULL OR btrim(p_tenant_name) = '' THEN
        RAISE EXCEPTION 'tenant_name must not be empty';
    END IF;
    IF p_destination_uri IS NULL OR btrim(p_destination_uri) = '' THEN
        RAISE EXCEPTION 'destination_uri must not be empty';
    END IF;
    IF p_retention_days IS NULL OR p_retention_days <= 0 THEN
        RAISE EXCEPTION 'retention_days must be greater than zero';
    END IF;

    INSERT INTO companion_internal.tenant_archives(
        tenant_name,
        destination_uri,
        retention_days,
        status
    )
    VALUES (
        btrim(p_tenant_name),
        btrim(p_destination_uri),
        p_retention_days,
        'queued'
    )
    RETURNING companion_internal.tenant_archives.archive_id
    INTO archive_id;

    RETURN archive_id;
END;
$$;

CREATE FUNCTION companion_internal.set_tenant_region_affinity(
    p_tenant_name text,
    p_region_affinity text
)
RETURNS void
LANGUAGE plpgsql
VOLATILE
AS $$
BEGIN
    IF p_tenant_name IS NULL OR btrim(p_tenant_name) = '' THEN
        RAISE EXCEPTION 'tenant_name must not be empty';
    END IF;
    IF p_region_affinity IS NULL OR btrim(p_region_affinity) = '' THEN
        RAISE EXCEPTION 'region_affinity must not be empty';
    END IF;

    INSERT INTO companion_internal.tenant_region_affinities(
        tenant_name,
        region_affinity
    )
    VALUES (
        btrim(p_tenant_name),
        btrim(p_region_affinity)
    )
    ON CONFLICT (tenant_name) DO UPDATE
    SET region_affinity = EXCLUDED.region_affinity,
        updated_at = now();
END;
$$;

CREATE FUNCTION companion_internal.ensure_search_workers(
    p_table_name text,
    p_distribution_column text
)
RETURNS void
LANGUAGE plpgsql
VOLATILE
AS $$
DECLARE
    table_regclass regclass;
BEGIN
    IF p_table_name IS NULL OR btrim(p_table_name) = '' THEN
        RAISE EXCEPTION 'table_name must not be empty';
    END IF;
    table_regclass := p_table_name::regclass;
    IF p_distribution_column IS NULL OR btrim(p_distribution_column) = '' THEN
        RAISE EXCEPTION 'distribution_column must not be empty';
    END IF;
    IF NOT companion_internal.table_has_column(table_regclass::text, btrim(p_distribution_column)::name) THEN
        RAISE EXCEPTION 'distribution column does not exist on table';
    END IF;
END;
$$;

CREATE FUNCTION companion_internal.register_search_index(
    p_table_name text,
    p_index_name name,
    p_text_columns text[],
    p_distribution_column text,
    p_vector_columns text[] DEFAULT ARRAY[]::text[]
)
RETURNS text
LANGUAGE plpgsql
VOLATILE
AS $$
DECLARE
    table_regclass regclass;
    normalized_text_columns text[];
    normalized_vector_columns text[];
    rendered_sql text;
BEGIN
    PERFORM companion_internal.ensure_search_workers(p_table_name, p_distribution_column);
    table_regclass := p_table_name::regclass;
    IF p_index_name IS NULL OR btrim(p_index_name::text) = '' THEN
        RAISE EXCEPTION 'index_name must not be empty';
    END IF;
    IF p_text_columns IS NULL OR cardinality(p_text_columns) = 0 THEN
        RAISE EXCEPTION 'text_columns must contain at least one column';
    END IF;
    normalized_text_columns := ARRAY(
        SELECT btrim(column_name)
        FROM unnest(p_text_columns) AS column_name
    );
    IF EXISTS (
        SELECT 1
        FROM unnest(normalized_text_columns) AS column_name
        WHERE column_name = ''
           OR NOT companion_internal.table_has_column(table_regclass::text, column_name::name)
    ) THEN
        RAISE EXCEPTION 'all text_columns must exist on table';
    END IF;
    normalized_vector_columns := COALESCE(
        ARRAY(
            SELECT btrim(column_name)
            FROM unnest(COALESCE(p_vector_columns, ARRAY[]::text[])) AS column_name
            WHERE btrim(column_name) <> ''
        ),
        ARRAY[]::text[]
    );

    rendered_sql := format(
        'CREATE INDEX IF NOT EXISTS %I ON %s USING gin (to_tsvector(''simple'', %s));',
        p_index_name,
        table_regclass,
        (
            SELECT string_agg(format('coalesce(%I::text, '''')', column_name), ' || '' '' || ')
            FROM unnest(normalized_text_columns) AS column_name
        )
    );

    INSERT INTO companion_internal.search_worker_indexes(
        index_name,
        table_name,
        distribution_column,
        text_columns,
        vector_columns
    )
    VALUES (
        p_index_name,
        table_regclass::text,
        btrim(p_distribution_column)::name,
        normalized_text_columns,
        normalized_vector_columns
    )
    ON CONFLICT (index_name) DO UPDATE
    SET table_name = EXCLUDED.table_name,
        distribution_column = EXCLUDED.distribution_column,
        text_columns = EXCLUDED.text_columns,
        vector_columns = EXCLUDED.vector_columns,
        created_at = now();

    RETURN rendered_sql;
END;
$$;

CREATE FUNCTION companion_internal.search_document_upsert(
    p_table_name text,
    p_document_key text,
    p_text_body text,
    p_vector_score numeric DEFAULT 0
)
RETURNS bigint
LANGUAGE plpgsql
VOLATILE
AS $$
DECLARE
    document_id bigint;
BEGIN
    IF p_table_name IS NULL OR btrim(p_table_name) = '' THEN
        RAISE EXCEPTION 'table_name must not be empty';
    END IF;
    IF p_document_key IS NULL OR btrim(p_document_key) = '' THEN
        RAISE EXCEPTION 'document_key must not be empty';
    END IF;
    IF p_text_body IS NULL OR btrim(p_text_body) = '' THEN
        RAISE EXCEPTION 'text_body must not be empty';
    END IF;
    IF p_vector_score IS NULL THEN
        RAISE EXCEPTION 'vector_score must not be null';
    END IF;

    INSERT INTO companion_internal.search_documents(
        table_name,
        document_key,
        text_body,
        vector_score
    )
    VALUES (
        btrim(p_table_name::regclass::text),
        btrim(p_document_key),
        p_text_body,
        p_vector_score
    )
    ON CONFLICT (table_name, document_key) DO UPDATE
    SET text_body = EXCLUDED.text_body,
        vector_score = EXCLUDED.vector_score,
        updated_at = now()
    RETURNING companion_internal.search_documents.document_id
    INTO document_id;

    RETURN document_id;
END;
$$;

CREATE FUNCTION companion_internal.hybrid_rank(
    p_table_name text,
    p_text_query text,
    p_vector_column text,
    p_vector_parameter text
)
RETURNS TABLE(
    document_key text,
    bm25_score numeric,
    vector_score numeric
)
LANGUAGE plpgsql
STABLE
AS $$
DECLARE
    table_regclass regclass;
BEGIN
    IF p_table_name IS NULL OR btrim(p_table_name) = '' THEN
        RAISE EXCEPTION 'table_name must not be empty';
    END IF;
    table_regclass := p_table_name::regclass;
    IF p_text_query IS NULL OR btrim(p_text_query) = '' THEN
        RAISE EXCEPTION 'text_query must not be empty';
    END IF;
    IF p_vector_column IS NULL OR btrim(p_vector_column) = '' THEN
        RAISE EXCEPTION 'vector_column must not be empty';
    END IF;
    IF NOT companion_internal.table_has_column(table_regclass::text, btrim(p_vector_column)::name) THEN
        RAISE EXCEPTION 'vector_column does not exist on table';
    END IF;
    IF p_vector_parameter IS NULL OR btrim(p_vector_parameter) = '' THEN
        RAISE EXCEPTION 'vector_parameter must not be empty';
    END IF;

    RETURN QUERY
    SELECT
        documents.document_key,
        ts_rank(
            to_tsvector('simple', documents.text_body),
            plainto_tsquery('simple', p_text_query)
        )::numeric AS bm25_score,
        documents.vector_score
    FROM companion_internal.search_documents AS documents
    WHERE documents.table_name = table_regclass::text
      AND to_tsvector('simple', documents.text_body)
          @@ plainto_tsquery('simple', p_text_query)
    ORDER BY bm25_score DESC, documents.vector_score DESC, documents.document_id;
END;
$$;

CREATE FUNCTION companion_internal.rerank_search(
    p_input_view text,
    p_provider text,
    p_model text
)
RETURNS TABLE(
    input_view text,
    provider text,
    model text,
    rerank_sql text
)
LANGUAGE plpgsql
VOLATILE
AS $$
BEGIN
    IF p_input_view IS NULL OR btrim(p_input_view) = '' THEN
        RAISE EXCEPTION 'input_view must not be empty';
    END IF;
    IF to_regclass(p_input_view) IS NULL THEN
        RAISE EXCEPTION 'input_view must reference an existing relation';
    END IF;
    IF p_provider IS NULL OR btrim(p_provider) = '' THEN
        RAISE EXCEPTION 'provider must not be empty';
    END IF;
    IF p_model IS NULL OR btrim(p_model) = '' THEN
        RAISE EXCEPTION 'model must not be empty';
    END IF;

    INSERT INTO companion_internal.search_rerank_requests(
        input_view,
        provider,
        model
    )
    VALUES (
        btrim(p_input_view::regclass::text),
        btrim(p_provider),
        btrim(p_model)
    );

    RETURN QUERY SELECT
        btrim(p_input_view::regclass::text),
        btrim(p_provider),
        btrim(p_model),
        format('SELECT * FROM %s', btrim(p_input_view::regclass::text));
END;
$$;

-- FEATURE: G2
-- FEATURE: G3
-- FEATURE: API4
CREATE VIEW companion_graph_colocations AS
SELECT
    colocation_id,
    vertex_table,
    edge_table,
    vertex_key,
    colocation_group,
    created_at
FROM companion_internal.graph_colocations;

CREATE VIEW companion_graphql_distributed_graphs AS
SELECT
    graph_name,
    vertex_table,
    edge_table,
    registered_at
FROM companion_internal.graphql_distributed_graphs;

CREATE FUNCTION companion_internal.ensure_graph_colocation(
    p_vertex_table text,
    p_edge_table text,
    p_vertex_key text,
    p_colocation_group text
)
RETURNS void
LANGUAGE plpgsql
VOLATILE
AS $$
DECLARE
    vertex_regclass regclass;
    edge_regclass regclass;
BEGIN
    IF p_vertex_table IS NULL OR btrim(p_vertex_table) = '' THEN
        RAISE EXCEPTION 'vertex_table must not be empty';
    END IF;
    IF p_edge_table IS NULL OR btrim(p_edge_table) = '' THEN
        RAISE EXCEPTION 'edge_table must not be empty';
    END IF;
    vertex_regclass := p_vertex_table::regclass;
    edge_regclass := p_edge_table::regclass;
    IF p_vertex_key IS NULL OR btrim(p_vertex_key) = '' THEN
        RAISE EXCEPTION 'vertex_key must not be empty';
    END IF;
    IF NOT companion_internal.table_has_column(vertex_regclass::text, btrim(p_vertex_key)::name) THEN
        RAISE EXCEPTION 'vertex_key column does not exist on vertex table';
    END IF;
    IF p_colocation_group IS NULL OR btrim(p_colocation_group) = '' THEN
        RAISE EXCEPTION 'colocation_group must not be empty';
    END IF;

    INSERT INTO companion_internal.graph_colocations(
        vertex_table,
        edge_table,
        vertex_key,
        colocation_group
    )
    VALUES (
        vertex_regclass::text,
        edge_regclass::text,
        btrim(p_vertex_key)::name,
        btrim(p_colocation_group)
    )
    ON CONFLICT (vertex_table, edge_table, vertex_key, colocation_group) DO NOTHING;
END;
$$;

CREATE FUNCTION companion_internal.register_graphql_distributed_graph(
    p_graph_name text,
    p_vertex_table text,
    p_edge_table text
)
RETURNS void
LANGUAGE plpgsql
VOLATILE
AS $$
DECLARE
    vertex_regclass regclass;
    edge_regclass regclass;
BEGIN
    IF p_graph_name IS NULL OR btrim(p_graph_name) = '' THEN
        RAISE EXCEPTION 'graph_name must not be empty';
    END IF;
    vertex_regclass := p_vertex_table::regclass;
    edge_regclass := p_edge_table::regclass;
    IF NOT EXISTS (
        SELECT 1
        FROM companion_internal.graph_colocations
        WHERE vertex_table = vertex_regclass::text
          AND edge_table = edge_regclass::text
    ) THEN
        RAISE EXCEPTION 'graph colocation must be registered before GraphQL graph metadata';
    END IF;

    INSERT INTO companion_internal.graphql_distributed_graphs(
        graph_name,
        vertex_table,
        edge_table
    )
    VALUES (
        btrim(p_graph_name),
        vertex_regclass::text,
        edge_regclass::text
    )
    ON CONFLICT (graph_name) DO UPDATE
    SET vertex_table = EXCLUDED.vertex_table,
        edge_table = EXCLUDED.edge_table,
        registered_at = now();
END;
$$;

-- FEATURE: JS2
-- FEATURE: M13
CREATE VIEW companion_json_schemas AS
SELECT
    schema_name,
    schema_document,
    registered_at
FROM companion_internal.json_schemas;

CREATE VIEW companion_jsonschema_triggers AS
SELECT
    trigger_id,
    table_name,
    json_column,
    schema_name,
    timing,
    trigger_name,
    trigger_sql,
    installed_at
FROM companion_internal.jsonschema_triggers;

CREATE FUNCTION companion_internal.register_json_schema(
    p_schema_name text,
    p_schema_document jsonb
)
RETURNS void
LANGUAGE plpgsql
VOLATILE
AS $$
BEGIN
    IF p_schema_name IS NULL OR btrim(p_schema_name) = '' THEN
        RAISE EXCEPTION 'schema_name must not be empty';
    END IF;
    IF p_schema_document IS NULL OR jsonb_typeof(p_schema_document) <> 'object' THEN
        RAISE EXCEPTION 'schema_document must be a JSON object';
    END IF;

    INSERT INTO companion_internal.json_schemas(schema_name, schema_document)
    VALUES (btrim(p_schema_name), p_schema_document)
    ON CONFLICT (schema_name) DO UPDATE
    SET schema_document = EXCLUDED.schema_document,
        registered_at = now();
END;
$$;

CREATE FUNCTION companion_internal.json_schema_matches(
    p_schema_name text,
    p_document jsonb
)
RETURNS boolean
LANGUAGE plpgsql
STABLE
AS $$
DECLARE
    schema_doc jsonb;
    required_field text;
BEGIN
    SELECT schema_document
    INTO schema_doc
    FROM companion_internal.json_schemas
    WHERE schema_name = btrim(p_schema_name);
    IF NOT FOUND THEN
        RAISE EXCEPTION 'json schema is not registered: %', p_schema_name;
    END IF;
    IF p_document IS NULL THEN
        RETURN false;
    END IF;

    IF schema_doc ? 'type' THEN
        IF schema_doc->>'type' <> jsonb_typeof(p_document) THEN
            RETURN false;
        END IF;
    END IF;

    IF schema_doc ? 'required' THEN
        FOR required_field IN
            SELECT value
            FROM jsonb_array_elements_text(schema_doc->'required') AS required(value)
        LOOP
            IF NOT p_document ? required_field THEN
                RETURN false;
            END IF;
        END LOOP;
    END IF;

    RETURN true;
END;
$$;

CREATE FUNCTION companion_internal.enforce_jsonschema_trigger()
RETURNS trigger
LANGUAGE plpgsql
VOLATILE
AS $$
DECLARE
    json_column name := TG_ARGV[0]::name;
    schema_name text := TG_ARGV[1];
    row_json jsonb := to_jsonb(NEW);
    document jsonb;
BEGIN
    document := row_json->json_column::text;
    IF NOT companion_internal.json_schema_matches(schema_name, document) THEN
        RAISE EXCEPTION 'json document does not match registered schema';
    END IF;
    RETURN NEW;
END;
$$;

CREATE FUNCTION companion_internal.install_jsonschema_trigger(
    p_table_name text,
    p_json_column text,
    p_schema_name text,
    p_timing text
)
RETURNS text
LANGUAGE plpgsql
VOLATILE
AS $$
DECLARE
    table_regclass regclass;
    normalized_timing text;
    trigger_name name;
    trigger_sql text;
BEGIN
    IF p_table_name IS NULL OR btrim(p_table_name) = '' THEN
        RAISE EXCEPTION 'table_name must not be empty';
    END IF;
    table_regclass := p_table_name::regclass;
    IF p_json_column IS NULL OR btrim(p_json_column) = '' THEN
        RAISE EXCEPTION 'json_column must not be empty';
    END IF;
    IF NOT companion_internal.table_has_column(table_regclass::text, btrim(p_json_column)::name) THEN
        RAISE EXCEPTION 'json_column does not exist on table';
    END IF;
    IF NOT EXISTS (
        SELECT 1
        FROM companion_internal.json_schemas
        WHERE schema_name = btrim(p_schema_name)
    ) THEN
        RAISE EXCEPTION 'schema_name is not registered';
    END IF;
    normalized_timing := upper(btrim(p_timing));
    IF normalized_timing NOT IN ('BEFORE INSERT OR UPDATE', 'AFTER INSERT OR UPDATE') THEN
        RAISE EXCEPTION 'timing must be BEFORE INSERT OR UPDATE or AFTER INSERT OR UPDATE';
    END IF;

    trigger_name := (
        'companion_jsonschema_' || substr(md5(table_regclass::text || ':' || p_json_column || ':' || p_schema_name), 1, 16)
    )::name;
    trigger_sql := format(
        'CREATE TRIGGER %I %s ON %s FOR EACH ROW EXECUTE FUNCTION companion_internal.enforce_jsonschema_trigger(%L, %L)',
        trigger_name,
        normalized_timing,
        table_regclass,
        btrim(p_json_column),
        btrim(p_schema_name)
    );

    EXECUTE format('DROP TRIGGER IF EXISTS %I ON %s', trigger_name, table_regclass);
    EXECUTE trigger_sql;

    INSERT INTO companion_internal.jsonschema_triggers(
        table_name,
        json_column,
        schema_name,
        timing,
        trigger_name,
        trigger_sql
    )
    VALUES (
        table_regclass::text,
        btrim(p_json_column)::name,
        btrim(p_schema_name),
        normalized_timing,
        trigger_name,
        trigger_sql
    )
    ON CONFLICT (table_name, json_column, schema_name) DO UPDATE
    SET timing = EXCLUDED.timing,
        trigger_name = EXCLUDED.trigger_name,
        trigger_sql = EXCLUDED.trigger_sql,
        installed_at = now();

    RETURN trigger_sql;
END;
$$;

CREATE FUNCTION companion_internal.validate_jsonschema_shard(
    p_table_name regclass,
    p_json_column text,
    p_schema_name text
)
RETURNS TABLE(total_rows bigint, invalid_rows bigint)
LANGUAGE plpgsql
STABLE
AS $$
BEGIN
    IF p_table_name IS NULL THEN
        RAISE EXCEPTION 'table_name must not be null';
    END IF;
    IF p_json_column IS NULL OR btrim(p_json_column) = '' THEN
        RAISE EXCEPTION 'json_column must not be empty';
    END IF;
    IF p_schema_name IS NULL OR btrim(p_schema_name) = '' THEN
        RAISE EXCEPTION 'schema_name must not be empty';
    END IF;

    RETURN QUERY EXECUTE format(
        'SELECT count(*)::bigint, count(*) FILTER (WHERE NOT companion_internal.json_schema_matches(%L, %I))::bigint FROM %s',
        btrim(p_schema_name),
        btrim(p_json_column),
        p_table_name
    );
END;
$$;

-- FEATURE: Geo2
-- FEATURE: Geo3
CREATE VIEW companion_geo_distributions AS
SELECT
    table_name,
    geometry_column,
    distribution_column,
    precision,
    updated_at
FROM companion_internal.geo_distributions;

CREATE VIEW companion_geo_pruning_policies AS
SELECT
    policy_id,
    table_name,
    geometry_column,
    precision,
    updated_at
FROM companion_internal.geo_pruning_policies;

CREATE FUNCTION companion_geo_bucket(
    p_latitude numeric,
    p_longitude numeric,
    p_precision integer
)
RETURNS text
LANGUAGE plpgsql
IMMUTABLE
PARALLEL SAFE
AS $$
BEGIN
    IF p_latitude IS NULL OR p_latitude < -90 OR p_latitude > 90 THEN
        RAISE EXCEPTION 'latitude must be between -90 and 90';
    END IF;
    IF p_longitude IS NULL OR p_longitude < -180 OR p_longitude > 180 THEN
        RAISE EXCEPTION 'longitude must be between -180 and 180';
    END IF;
    IF p_precision IS NULL OR p_precision < 1 OR p_precision > 12 THEN
        RAISE EXCEPTION 'precision must be between 1 and 12';
    END IF;

    RETURN substr(
        md5(
            round(p_latitude::numeric, p_precision)::text
            || ':'
            || round(p_longitude::numeric, p_precision)::text
        ),
        1,
        p_precision
    );
END;
$$;

CREATE FUNCTION companion_internal.add_geohash_column(
    p_table_name text,
    p_geometry_column text,
    p_distribution_column text,
    p_precision integer
)
RETURNS text
LANGUAGE plpgsql
VOLATILE
AS $$
DECLARE
    table_regclass regclass;
    rendered_sql text;
BEGIN
    IF p_table_name IS NULL OR btrim(p_table_name) = '' THEN
        RAISE EXCEPTION 'table_name must not be empty';
    END IF;
    table_regclass := p_table_name::regclass;
    IF p_geometry_column IS NULL OR btrim(p_geometry_column) = '' THEN
        RAISE EXCEPTION 'geometry_column must not be empty';
    END IF;
    IF NOT companion_internal.table_has_column(table_regclass::text, btrim(p_geometry_column)::name) THEN
        RAISE EXCEPTION 'geometry_column does not exist on table';
    END IF;
    IF p_distribution_column IS NULL OR btrim(p_distribution_column) = '' THEN
        RAISE EXCEPTION 'distribution_column must not be empty';
    END IF;
    IF p_precision IS NULL OR p_precision < 1 OR p_precision > 12 THEN
        RAISE EXCEPTION 'precision must be between 1 and 12';
    END IF;

    rendered_sql := format(
        'ALTER TABLE %s ADD COLUMN IF NOT EXISTS %I text;',
        table_regclass,
        btrim(p_distribution_column)
    );
    EXECUTE rendered_sql;

    INSERT INTO companion_internal.geo_distributions(
        table_name,
        geometry_column,
        distribution_column,
        precision
    )
    VALUES (
        table_regclass::text,
        btrim(p_geometry_column)::name,
        btrim(p_distribution_column)::name,
        p_precision
    )
    ON CONFLICT (table_name) DO UPDATE
    SET geometry_column = EXCLUDED.geometry_column,
        distribution_column = EXCLUDED.distribution_column,
        precision = EXCLUDED.precision,
        updated_at = now();

    RETURN rendered_sql;
END;
$$;

CREATE FUNCTION companion_internal.enable_geo_shard_pruning(
    p_table_name text,
    p_geometry_column text,
    p_precision integer
)
RETURNS void
LANGUAGE plpgsql
VOLATILE
AS $$
DECLARE
    table_regclass regclass;
BEGIN
    IF p_table_name IS NULL OR btrim(p_table_name) = '' THEN
        RAISE EXCEPTION 'table_name must not be empty';
    END IF;
    table_regclass := p_table_name::regclass;
    IF p_geometry_column IS NULL OR btrim(p_geometry_column) = '' THEN
        RAISE EXCEPTION 'geometry_column must not be empty';
    END IF;
    IF NOT companion_internal.table_has_column(table_regclass::text, btrim(p_geometry_column)::name) THEN
        RAISE EXCEPTION 'geometry_column does not exist on table';
    END IF;
    IF p_precision IS NULL OR p_precision < 1 OR p_precision > 12 THEN
        RAISE EXCEPTION 'precision must be between 1 and 12';
    END IF;

    INSERT INTO companion_internal.geo_pruning_policies(
        table_name,
        geometry_column,
        precision
    )
    VALUES (
        table_regclass::text,
        btrim(p_geometry_column)::name,
        p_precision
    )
    ON CONFLICT (table_name, geometry_column) DO UPDATE
    SET precision = EXCLUDED.precision,
        updated_at = now();
END;
$$;

-- FEATURE: M1
-- FEATURE: M11
CREATE VIEW companion_migration_runs AS
SELECT
    migration_name,
    table_name,
    lock_timeout_ms,
    backfill_batch_size,
    status,
    started_at,
    completed_at
FROM companion_internal.migration_runs;

CREATE VIEW companion_migration_operations AS
SELECT
    operation_id,
    migration_name,
    operation_type,
    column_name,
    sql_type,
    default_expression,
    new_column_name,
    from_type,
    to_type,
    cast_expression,
    rendered_sql,
    recorded_at
FROM companion_internal.migration_operations;

CREATE VIEW companion_migration_invariant_checks AS
SELECT
    migration_name,
    check_name,
    check_sql,
    last_result,
    passed_at,
    recorded_at
FROM companion_internal.migration_invariant_checks;

CREATE FUNCTION companion_internal.current_migration_name()
RETURNS text
LANGUAGE plpgsql
STABLE
AS $$
DECLARE
    migration_name text;
BEGIN
    migration_name := NULLIF(current_setting('ai_blaise.current_migration_name', true), '');
    IF migration_name IS NULL THEN
        RAISE EXCEPTION 'no active companion migration; call companion_internal.migrate_start first';
    END IF;
    RETURN migration_name;
END;
$$;

CREATE FUNCTION companion_internal.migrate_start(
    p_migration_name text,
    p_table_name text,
    p_lock_timeout_ms integer,
    p_backfill_batch_size integer
)
RETURNS void
LANGUAGE plpgsql
VOLATILE
AS $$
DECLARE
    existing_run record;
    table_regclass regclass;
BEGIN
    IF p_migration_name IS NULL OR btrim(p_migration_name) = '' THEN
        RAISE EXCEPTION 'migration_name must not be empty';
    END IF;
    IF p_table_name IS NULL OR btrim(p_table_name) = '' THEN
        RAISE EXCEPTION 'table_name must not be empty';
    END IF;
    table_regclass := p_table_name::regclass;
    IF p_lock_timeout_ms IS NULL OR p_lock_timeout_ms <= 0 THEN
        RAISE EXCEPTION 'lock_timeout_ms must be greater than zero';
    END IF;
    IF p_backfill_batch_size IS NULL OR p_backfill_batch_size <= 0 THEN
        RAISE EXCEPTION 'backfill_batch_size must be greater than zero';
    END IF;

    SELECT *
    INTO existing_run
    FROM companion_internal.migration_runs
    WHERE migration_name = btrim(p_migration_name)
    FOR UPDATE;

    IF FOUND THEN
        IF existing_run.table_name <> table_regclass::text
            OR existing_run.lock_timeout_ms <> p_lock_timeout_ms
            OR existing_run.backfill_batch_size <> p_backfill_batch_size THEN
            RAISE EXCEPTION 'migration re-entry conflicts with existing run: %', p_migration_name;
        END IF;

        IF existing_run.status = 'completed' THEN
            PERFORM set_config('ai_blaise.current_migration_name', '', true);
            RETURN;
        END IF;

        UPDATE companion_internal.migration_runs
        SET status = 'running'
        WHERE migration_name = btrim(p_migration_name)
          AND status = 'running';

        PERFORM set_config('ai_blaise.current_migration_name', btrim(p_migration_name), true);
        RETURN;
    END IF;

    INSERT INTO companion_internal.migration_runs(
        migration_name,
        table_name,
        lock_timeout_ms,
        backfill_batch_size,
        status
    )
    VALUES (
        btrim(p_migration_name),
        table_regclass::text,
        p_lock_timeout_ms,
        p_backfill_batch_size,
        'running'
    )
    ON CONFLICT (migration_name) DO NOTHING;

    PERFORM set_config('ai_blaise.current_migration_name', btrim(p_migration_name), true);
END;
$$;

CREATE FUNCTION companion_internal.migration_register_invariant(
    p_migration_name text,
    p_check_name text,
    p_check_sql text
)
RETURNS text
LANGUAGE plpgsql
VOLATILE
AS $$
DECLARE
    normalized_sql text;
BEGIN
    IF p_migration_name IS NULL OR btrim(p_migration_name) = '' THEN
        RAISE EXCEPTION 'migration_name must not be empty';
    END IF;
    IF p_check_name IS NULL OR btrim(p_check_name) = '' THEN
        RAISE EXCEPTION 'check_name must not be empty';
    END IF;
    IF p_check_sql IS NULL OR btrim(p_check_sql) = '' THEN
        RAISE EXCEPTION 'check_sql must not be empty';
    END IF;
    normalized_sql := btrim(regexp_replace(btrim(p_check_sql), ';+$', ''));
    IF lower(normalized_sql) !~ '^(select|with)[[:space:]]' OR position(';' in normalized_sql) > 0 THEN
        RAISE EXCEPTION 'data invariant SQL must be a single read-only SELECT or WITH query';
    END IF;
    IF NOT EXISTS (
        SELECT 1
        FROM companion_internal.migration_runs
        WHERE migration_name = btrim(p_migration_name)
          AND status = 'running'
    ) THEN
        RAISE EXCEPTION 'migration is not running: %', p_migration_name;
    END IF;

    INSERT INTO companion_internal.migration_invariant_checks(
        migration_name,
        check_name,
        check_sql
    )
    VALUES (
        btrim(p_migration_name),
        btrim(p_check_name),
        normalized_sql
    )
    ON CONFLICT (migration_name, check_name) DO UPDATE
    SET check_sql = EXCLUDED.check_sql,
        last_result = NULL,
        passed_at = NULL,
        recorded_at = now();

    RETURN btrim(p_check_name);
END;
$$;

CREATE FUNCTION companion_internal.migration_assert_invariants(p_migration_name text)
RETURNS jsonb
LANGUAGE plpgsql
VOLATILE
AS $$
DECLARE
    check_record record;
    check_result jsonb;
    check_result_count integer;
    destructive_operation_count integer;
    passed_count integer := 0;
BEGIN
    IF p_migration_name IS NULL OR btrim(p_migration_name) = '' THEN
        RAISE EXCEPTION 'migration_name must not be empty';
    END IF;
    IF NOT EXISTS (
        SELECT 1
        FROM companion_internal.migration_runs
        WHERE migration_name = btrim(p_migration_name)
          AND status IN ('running', 'completed')
    ) THEN
        RAISE EXCEPTION 'migration is not registered: %', p_migration_name;
    END IF;

    SELECT count(*)
    INTO destructive_operation_count
    FROM companion_internal.migration_operations
    WHERE migration_name = btrim(p_migration_name)
      AND operation_type IN ('drop_column', 'rename_column', 'online_type_change');

    IF destructive_operation_count > 0 AND NOT EXISTS (
        SELECT 1
        FROM companion_internal.migration_invariant_checks
        WHERE migration_name = btrim(p_migration_name)
    ) THEN
        RAISE EXCEPTION 'data invariant check is required before completing migration: %', p_migration_name;
    END IF;

    FOR check_record IN
        SELECT check_name, check_sql
        FROM companion_internal.migration_invariant_checks
        WHERE migration_name = btrim(p_migration_name)
        ORDER BY check_name
    LOOP
        EXECUTE format(
            'SELECT count(*), (jsonb_agg(to_jsonb(invariant_result)) -> 0) FROM (%s) AS invariant_result',
            check_record.check_sql
        )
        INTO check_result_count, check_result;

        IF check_result_count <> 1 THEN
            RAISE EXCEPTION 'data invariant check must return exactly one row: %', check_record.check_name;
        END IF;

        IF COALESCE((check_result->>'passed')::boolean, false) IS DISTINCT FROM true THEN
            UPDATE companion_internal.migration_invariant_checks
            SET last_result = check_result,
                passed_at = NULL
            WHERE migration_name = btrim(p_migration_name)
              AND check_name = check_record.check_name;
            RAISE EXCEPTION 'data invariant check failed: %', check_record.check_name;
        END IF;

        UPDATE companion_internal.migration_invariant_checks
        SET last_result = check_result,
            passed_at = now()
        WHERE migration_name = btrim(p_migration_name)
          AND check_name = check_record.check_name;
        passed_count := passed_count + 1;
    END LOOP;

    RETURN jsonb_build_object(
        'migration_name', btrim(p_migration_name),
        'destructive_operations', destructive_operation_count,
        'passed_checks', passed_count
    );
END;
$$;

CREATE FUNCTION companion_internal.record_migration_operation(
    p_operation_type text,
    p_column_name text,
    p_sql_type text,
    p_default_expression text,
    p_new_column_name text,
    p_from_type text,
    p_to_type text,
    p_cast_expression text,
    p_rendered_sql text
)
RETURNS text
LANGUAGE plpgsql
VOLATILE
AS $$
DECLARE
    active_migration_name text := companion_internal.current_migration_name();
    existing_sql text;
    normalized_operation text := lower(btrim(p_operation_type));
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM companion_internal.migration_runs
        WHERE migration_runs.migration_name = active_migration_name
          AND status = 'running'
    ) THEN
        RAISE EXCEPTION 'migration is not running: %', active_migration_name;
    END IF;

    IF normalized_operation IN ('drop_column', 'rename_column', 'online_type_change')
        AND NOT EXISTS (
            SELECT 1
            FROM companion_internal.migration_invariant_checks
            WHERE migration_invariant_checks.migration_name = active_migration_name
        ) THEN
        RAISE EXCEPTION 'data invariant check is required before destructive migration operation';
    END IF;

    SELECT op.rendered_sql
    INTO existing_sql
    FROM companion_internal.migration_operations AS op
    WHERE op.migration_name = active_migration_name
      AND op.operation_type = normalized_operation
      AND op.column_name = p_column_name
      AND op.sql_type IS NOT DISTINCT FROM p_sql_type
      AND op.default_expression IS NOT DISTINCT FROM p_default_expression
      AND op.new_column_name IS NOT DISTINCT FROM p_new_column_name
      AND op.from_type IS NOT DISTINCT FROM p_from_type
      AND op.to_type IS NOT DISTINCT FROM p_to_type
      AND op.cast_expression IS NOT DISTINCT FROM p_cast_expression
      AND op.rendered_sql = p_rendered_sql
    ORDER BY op.operation_id
    LIMIT 1;

    IF FOUND THEN
        RETURN existing_sql;
    END IF;

    IF EXISTS (
        SELECT 1
        FROM companion_internal.migration_operations AS op
        WHERE op.migration_name = active_migration_name
          AND op.operation_type = normalized_operation
          AND op.column_name = p_column_name
    ) THEN
        RAISE EXCEPTION 'migration operation re-entry conflicts with existing operation: %.%', active_migration_name, p_column_name;
    END IF;

    INSERT INTO companion_internal.migration_operations(
        migration_name,
        operation_type,
        column_name,
        sql_type,
        default_expression,
        new_column_name,
        from_type,
        to_type,
        cast_expression,
        rendered_sql
    )
    VALUES (
        active_migration_name,
        normalized_operation,
        p_column_name,
        p_sql_type,
        p_default_expression,
        p_new_column_name,
        p_from_type,
        p_to_type,
        p_cast_expression,
        p_rendered_sql
    );

    RETURN p_rendered_sql;
END;
$$;

CREATE FUNCTION companion_internal.migration_add_column(
    p_column_name text,
    p_sql_type text,
    p_default_expression text DEFAULT NULL
)
RETURNS text
LANGUAGE plpgsql
VOLATILE
AS $$
DECLARE
    active_migration_name text := companion_internal.current_migration_name();
    migration_table text;
    rendered_sql text;
BEGIN
    IF p_column_name IS NULL OR btrim(p_column_name) = '' THEN
        RAISE EXCEPTION 'column_name must not be empty';
    END IF;
    IF p_sql_type IS NULL OR btrim(p_sql_type) = '' THEN
        RAISE EXCEPTION 'sql_type must not be empty';
    END IF;
    IF p_default_expression IS NOT NULL AND btrim(p_default_expression) = '' THEN
        RAISE EXCEPTION 'default_expression must be null or non-empty';
    END IF;

    SELECT table_name INTO migration_table
    FROM companion_internal.migration_runs
    WHERE migration_name = active_migration_name
      AND status = 'running';
    IF NOT FOUND THEN
        RAISE EXCEPTION 'migration is not running: %', active_migration_name;
    END IF;

    rendered_sql := format(
        'ALTER TABLE %s ADD COLUMN IF NOT EXISTS %I %s%s;',
        migration_table,
        btrim(p_column_name),
        btrim(p_sql_type),
        CASE
            WHEN p_default_expression IS NULL THEN ''
            ELSE ' DEFAULT ' || p_default_expression
        END
    );

    RETURN companion_internal.record_migration_operation(
        'add_column',
        btrim(p_column_name),
        btrim(p_sql_type),
        p_default_expression,
        NULL,
        NULL,
        NULL,
        NULL,
        rendered_sql
    );
END;
$$;

CREATE FUNCTION companion_internal.migration_drop_column(p_column_name text)
RETURNS text
LANGUAGE plpgsql
VOLATILE
AS $$
DECLARE
    active_migration_name text := companion_internal.current_migration_name();
    migration_table text;
    rendered_sql text;
BEGIN
    IF p_column_name IS NULL OR btrim(p_column_name) = '' THEN
        RAISE EXCEPTION 'column_name must not be empty';
    END IF;

    SELECT table_name INTO migration_table
    FROM companion_internal.migration_runs
    WHERE migration_name = active_migration_name
      AND status = 'running';
    IF NOT FOUND THEN
        RAISE EXCEPTION 'migration is not running: %', active_migration_name;
    END IF;

    rendered_sql := format(
        'ALTER TABLE %s DROP COLUMN IF EXISTS %I;',
        migration_table,
        btrim(p_column_name)
    );

    RETURN companion_internal.record_migration_operation(
        'drop_column',
        btrim(p_column_name),
        NULL,
        NULL,
        NULL,
        NULL,
        NULL,
        NULL,
        rendered_sql
    );
END;
$$;

CREATE FUNCTION companion_internal.migration_rename_column(
    p_old_column_name text,
    p_new_column_name text
)
RETURNS text
LANGUAGE plpgsql
VOLATILE
AS $$
DECLARE
    active_migration_name text := companion_internal.current_migration_name();
    migration_table text;
    rendered_sql text;
BEGIN
    IF p_old_column_name IS NULL OR btrim(p_old_column_name) = '' THEN
        RAISE EXCEPTION 'old_column_name must not be empty';
    END IF;
    IF p_new_column_name IS NULL OR btrim(p_new_column_name) = '' THEN
        RAISE EXCEPTION 'new_column_name must not be empty';
    END IF;

    SELECT table_name INTO migration_table
    FROM companion_internal.migration_runs
    WHERE migration_name = active_migration_name
      AND status = 'running';
    IF NOT FOUND THEN
        RAISE EXCEPTION 'migration is not running: %', active_migration_name;
    END IF;

    rendered_sql := format(
        'ALTER TABLE %s RENAME COLUMN %I TO %I;',
        migration_table,
        btrim(p_old_column_name),
        btrim(p_new_column_name)
    );

    RETURN companion_internal.record_migration_operation(
        'rename_column',
        btrim(p_old_column_name),
        NULL,
        NULL,
        btrim(p_new_column_name),
        NULL,
        NULL,
        NULL,
        rendered_sql
    );
END;
$$;

CREATE FUNCTION companion_internal.migration_online_type_change(
    p_column_name text,
    p_from_type text,
    p_to_type text,
    p_cast_expression text
)
RETURNS text
LANGUAGE plpgsql
VOLATILE
AS $$
DECLARE
    active_migration_name text := companion_internal.current_migration_name();
    migration_table text;
    rendered_sql text;
BEGIN
    IF p_column_name IS NULL OR btrim(p_column_name) = '' THEN
        RAISE EXCEPTION 'column_name must not be empty';
    END IF;
    IF p_from_type IS NULL OR btrim(p_from_type) = '' THEN
        RAISE EXCEPTION 'from_type must not be empty';
    END IF;
    IF p_to_type IS NULL OR btrim(p_to_type) = '' THEN
        RAISE EXCEPTION 'to_type must not be empty';
    END IF;
    IF btrim(p_from_type) = btrim(p_to_type) THEN
        RAISE EXCEPTION 'from_type and to_type must differ';
    END IF;
    IF p_cast_expression IS NULL OR btrim(p_cast_expression) = '' THEN
        RAISE EXCEPTION 'cast_expression must not be empty';
    END IF;

    SELECT table_name INTO migration_table
    FROM companion_internal.migration_runs
    WHERE migration_name = active_migration_name
      AND status = 'running';
    IF NOT FOUND THEN
        RAISE EXCEPTION 'migration is not running: %', active_migration_name;
    END IF;

    rendered_sql := format(
        'ALTER TABLE %s ADD COLUMN IF NOT EXISTS %I__ai_blaise_new %s; UPDATE %s SET %I__ai_blaise_new = %s WHERE %I__ai_blaise_new IS NULL;',
        migration_table,
        btrim(p_column_name),
        btrim(p_to_type),
        migration_table,
        btrim(p_column_name),
        p_cast_expression,
        btrim(p_column_name)
    );

    RETURN companion_internal.record_migration_operation(
        'online_type_change',
        btrim(p_column_name),
        NULL,
        NULL,
        NULL,
        btrim(p_from_type),
        btrim(p_to_type),
        btrim(p_cast_expression),
        rendered_sql
    );
END;
$$;

CREATE FUNCTION companion_internal.migrate_complete(p_migration_name text)
RETURNS void
LANGUAGE plpgsql
VOLATILE
AS $$
DECLARE
    current_status text;
BEGIN
    IF p_migration_name IS NULL OR btrim(p_migration_name) = '' THEN
        RAISE EXCEPTION 'migration_name must not be empty';
    END IF;

    SELECT status
    INTO current_status
    FROM companion_internal.migration_runs
    WHERE migration_name = btrim(p_migration_name)
    FOR UPDATE;

    IF NOT FOUND THEN
        RAISE EXCEPTION 'migration is not registered: %', p_migration_name;
    END IF;

    IF current_status = 'completed' THEN
        PERFORM set_config('ai_blaise.current_migration_name', '', true);
        RETURN;
    END IF;

    PERFORM companion_internal.migration_assert_invariants(btrim(p_migration_name));

    UPDATE companion_internal.migration_runs
    SET status = 'completed',
        completed_at = now()
    WHERE migration_name = btrim(p_migration_name)
      AND status = 'running';

    IF NOT FOUND THEN
        RAISE EXCEPTION 'migration is not running: %', p_migration_name;
    END IF;

    PERFORM set_config('ai_blaise.current_migration_name', '', true);
END;
$$;

-- FEATURE: IA3
CREATE VIEW companion_index_advisor_candidates AS
SELECT
    candidate_id,
    workload_window,
    table_name,
    index_name,
    columns,
    index_method,
    estimated_cost_before,
    estimated_cost_after,
    qual_count,
    round(
        ((estimated_cost_before - estimated_cost_after) * 100)
        / estimated_cost_before,
        3
    ) AS improvement_percent,
    created_at
FROM companion_internal.index_advisor_candidates;

CREATE FUNCTION companion_internal.index_advisor_record_candidate(
    p_workload_window text,
    p_table_name text,
    p_index_name name,
    p_columns text[],
    p_index_method text,
    p_estimated_cost_before numeric,
    p_estimated_cost_after numeric,
    p_qual_count bigint
)
RETURNS bigint
LANGUAGE plpgsql
VOLATILE
AS $$
DECLARE
    table_regclass regclass;
    normalized_method text;
    candidate_id bigint;
BEGIN
    IF p_workload_window IS NULL OR btrim(p_workload_window) = '' THEN
        RAISE EXCEPTION 'workload_window must not be empty';
    END IF;
    IF p_table_name IS NULL OR btrim(p_table_name) = '' THEN
        RAISE EXCEPTION 'table_name must not be empty';
    END IF;
    table_regclass := p_table_name::regclass;
    IF p_index_name IS NULL OR btrim(p_index_name::text) = '' THEN
        RAISE EXCEPTION 'index_name must not be empty';
    END IF;
    IF p_columns IS NULL OR cardinality(p_columns) = 0
       OR EXISTS (SELECT 1 FROM unnest(p_columns) AS column_name WHERE btrim(column_name) = '') THEN
        RAISE EXCEPTION 'columns must contain at least one non-empty column';
    END IF;
    normalized_method := lower(btrim(p_index_method));
    IF normalized_method NOT IN ('btree', 'gin', 'gist', 'brin', 'rum', 'hnsw') THEN
        RAISE EXCEPTION 'unsupported index method: %', p_index_method;
    END IF;
    IF p_estimated_cost_before IS NULL OR p_estimated_cost_before <= 0 THEN
        RAISE EXCEPTION 'estimated_cost_before must be greater than zero';
    END IF;
    IF p_estimated_cost_after IS NULL OR p_estimated_cost_after < 0 THEN
        RAISE EXCEPTION 'estimated_cost_after must be non-negative';
    END IF;
    IF p_estimated_cost_after >= p_estimated_cost_before THEN
        RAISE EXCEPTION 'estimated_cost_after must be lower than estimated_cost_before';
    END IF;
    IF p_qual_count IS NULL OR p_qual_count <= 0 THEN
        RAISE EXCEPTION 'qual_count must be greater than zero';
    END IF;

    INSERT INTO companion_internal.index_advisor_candidates(
        workload_window,
        table_name,
        index_name,
        columns,
        index_method,
        estimated_cost_before,
        estimated_cost_after,
        qual_count
    )
    VALUES (
        btrim(p_workload_window),
        table_regclass::text,
        p_index_name,
        ARRAY(SELECT btrim(column_name) FROM unnest(p_columns) AS column_name),
        normalized_method,
        p_estimated_cost_before,
        p_estimated_cost_after,
        p_qual_count
    )
    RETURNING companion_internal.index_advisor_candidates.candidate_id
    INTO candidate_id;

    RETURN candidate_id;
END;
$$;

CREATE FUNCTION companion_index_advisor_ranked(
    p_min_improvement_percent numeric DEFAULT 0
)
RETURNS TABLE(
    candidate_id bigint,
    workload_window text,
    table_name text,
    index_name name,
    columns text[],
    index_method text,
    improvement_percent numeric,
    qual_count bigint,
    create_index_sql text
)
LANGUAGE plpgsql
STABLE
AS $$
BEGIN
    IF p_min_improvement_percent IS NULL OR p_min_improvement_percent < 0 THEN
        RAISE EXCEPTION 'min_improvement_percent must be non-negative';
    END IF;

    RETURN QUERY
    SELECT
        candidates.candidate_id,
        candidates.workload_window,
        candidates.table_name,
        candidates.index_name,
        candidates.columns,
        candidates.index_method,
        round(
            ((candidates.estimated_cost_before - candidates.estimated_cost_after) * 100)
            / candidates.estimated_cost_before,
            3
        ) AS improvement_percent,
        candidates.qual_count,
        format(
            'CREATE INDEX CONCURRENTLY IF NOT EXISTS %I ON %s USING %s (%s);',
            candidates.index_name,
            candidates.table_name,
            candidates.index_method,
            (
                SELECT string_agg(quote_ident(column_name), ', ')
                FROM unnest(candidates.columns) AS column_name
            )
        ) AS create_index_sql
    FROM companion_internal.index_advisor_candidates AS candidates
    WHERE round(
        ((candidates.estimated_cost_before - candidates.estimated_cost_after) * 100)
        / candidates.estimated_cost_before,
        3
    ) >= p_min_improvement_percent
    ORDER BY 7 DESC, candidates.qual_count DESC, candidates.candidate_id;
END;
$$;

-- FEATURE: WH2
CREATE VIEW companion_webhook_registrations AS
SELECT
    registrations.webhook_name,
    registrations.table_name,
    registrations.url,
    registrations.headers,
    registrations.max_retries,
    triggers.events,
    triggers.queue_name,
    triggers.trigger_name,
    registrations.created_at,
    triggers.installed_at
FROM companion_internal.webhook_registrations AS registrations
LEFT JOIN companion_internal.webhook_triggers AS triggers
  ON triggers.webhook_name = registrations.webhook_name;

CREATE VIEW companion_webhook_events AS
SELECT
    event_id,
    webhook_name,
    queue_name,
    table_name,
    operation,
    row_data,
    queued_at
FROM companion_internal.webhook_events;

CREATE FUNCTION companion_internal.webhook_register(
    p_webhook_name text,
    p_table_name text,
    p_url text,
    p_headers jsonb DEFAULT '{}'::jsonb,
    p_max_retries integer DEFAULT 3
)
RETURNS void
LANGUAGE plpgsql
VOLATILE
AS $$
DECLARE
    table_regclass regclass;
BEGIN
    IF p_webhook_name IS NULL OR btrim(p_webhook_name) = '' THEN
        RAISE EXCEPTION 'webhook_name must not be empty';
    END IF;
    IF p_table_name IS NULL OR btrim(p_table_name) = '' THEN
        RAISE EXCEPTION 'table_name must not be empty';
    END IF;
    table_regclass := p_table_name::regclass;
    IF p_url IS NULL OR btrim(p_url) = '' THEN
        RAISE EXCEPTION 'url must not be empty';
    END IF;
    IF p_url !~* '^https?://' THEN
        RAISE EXCEPTION 'url must be http or https';
    END IF;
    IF p_headers IS NULL OR jsonb_typeof(p_headers) <> 'object' THEN
        RAISE EXCEPTION 'headers must be a JSON object';
    END IF;
    IF p_max_retries IS NULL OR p_max_retries <= 0 THEN
        RAISE EXCEPTION 'max_retries must be greater than zero';
    END IF;

    INSERT INTO companion_internal.webhook_registrations(
        webhook_name,
        table_name,
        url,
        headers,
        max_retries
    )
    VALUES (
        btrim(p_webhook_name),
        table_regclass::text,
        btrim(p_url),
        p_headers,
        p_max_retries
    )
    ON CONFLICT (webhook_name) DO UPDATE
    SET table_name = EXCLUDED.table_name,
        url = EXCLUDED.url,
        headers = EXCLUDED.headers,
        max_retries = EXCLUDED.max_retries,
        created_at = now();
END;
$$;

CREATE FUNCTION companion_internal.enqueue_webhook_event()
RETURNS trigger
LANGUAGE plpgsql
VOLATILE
AS $$
DECLARE
    payload jsonb;
BEGIN
    IF TG_OP = 'DELETE' THEN
        payload := to_jsonb(OLD);
    ELSE
        payload := to_jsonb(NEW);
    END IF;

    INSERT INTO companion_internal.webhook_events(
        webhook_name,
        queue_name,
        table_name,
        operation,
        row_data
    )
    VALUES (
        TG_ARGV[0],
        TG_ARGV[1],
        TG_TABLE_SCHEMA || '.' || TG_TABLE_NAME,
        TG_OP,
        payload
    );

    IF TG_OP = 'DELETE' THEN
        RETURN OLD;
    END IF;
    RETURN NEW;
END;
$$;

CREATE FUNCTION companion_internal.install_webhook_trigger(
    p_table_name text,
    p_events text[],
    p_queue_name text,
    p_webhook_name text
)
RETURNS text
LANGUAGE plpgsql
VOLATILE
AS $$
DECLARE
    table_regclass regclass;
    normalized_events text[];
    event_clause text;
    trigger_name name;
    trigger_sql text;
BEGIN
    IF p_table_name IS NULL OR btrim(p_table_name) = '' THEN
        RAISE EXCEPTION 'table_name must not be empty';
    END IF;
    table_regclass := p_table_name::regclass;
    IF p_events IS NULL OR cardinality(p_events) = 0 THEN
        RAISE EXCEPTION 'events must contain at least one event';
    END IF;
    normalized_events := ARRAY(
        SELECT DISTINCT upper(btrim(event_name))
        FROM unnest(p_events) AS event_name
    );
    IF EXISTS (
        SELECT 1
        FROM unnest(normalized_events) AS event_name
        WHERE event_name NOT IN ('INSERT', 'UPDATE', 'DELETE')
           OR event_name = ''
    ) THEN
        RAISE EXCEPTION 'events must be INSERT, UPDATE, or DELETE';
    END IF;
    IF p_queue_name IS NULL OR btrim(p_queue_name) = '' THEN
        RAISE EXCEPTION 'queue_name must not be empty';
    END IF;
    IF p_webhook_name IS NULL OR btrim(p_webhook_name) = '' THEN
        RAISE EXCEPTION 'webhook_name must not be empty';
    END IF;
    IF NOT EXISTS (
        SELECT 1
        FROM companion_internal.webhook_registrations
        WHERE webhook_name = btrim(p_webhook_name)
          AND table_name = table_regclass::text
    ) THEN
        RAISE EXCEPTION 'webhook registration does not exist for table';
    END IF;

    SELECT string_agg(event_name, ' OR ' ORDER BY event_name)
    INTO event_clause
    FROM unnest(normalized_events) AS event_name;

    trigger_name := (
        'companion_webhook_' || substr(md5(btrim(p_webhook_name)), 1, 16)
    )::name;
    trigger_sql := format(
        'CREATE TRIGGER %I AFTER %s ON %s FOR EACH ROW EXECUTE FUNCTION companion_internal.enqueue_webhook_event(%L, %L)',
        trigger_name,
        event_clause,
        table_regclass,
        btrim(p_webhook_name),
        btrim(p_queue_name)
    );

    EXECUTE format('DROP TRIGGER IF EXISTS %I ON %s', trigger_name, table_regclass);
    EXECUTE trigger_sql;

    INSERT INTO companion_internal.webhook_triggers(
        webhook_name,
        table_name,
        events,
        queue_name,
        trigger_name,
        trigger_sql
    )
    VALUES (
        btrim(p_webhook_name),
        table_regclass::text,
        normalized_events,
        btrim(p_queue_name),
        trigger_name,
        trigger_sql
    )
    ON CONFLICT (webhook_name, table_name) DO UPDATE
    SET events = EXCLUDED.events,
        queue_name = EXCLUDED.queue_name,
        trigger_name = EXCLUDED.trigger_name,
        trigger_sql = EXCLUDED.trigger_sql,
        installed_at = now();

    RETURN trigger_sql;
END;
$$;

-- FEATURE: PM3
-- FEATURE: PM4
CREATE VIEW companion_plan_freezes AS
SELECT
    freezes.query_hash,
    freezes.plan_xml,
    freezes.hint_set_name,
    promotions.min_executions,
    promotions.stable_days,
    regressions.max_latency_regression_percent,
    regressions.max_cost_regression_percent,
    freezes.frozen_at
FROM companion_internal.plan_freezes AS freezes
LEFT JOIN companion_internal.plan_promotion_policies AS promotions
  ON promotions.query_hash = freezes.query_hash
LEFT JOIN companion_internal.plan_regression_policies AS regressions
  ON regressions.query_hash = freezes.query_hash;

CREATE FUNCTION companion_internal.plan_freeze(
    p_query_hash text,
    p_plan_xml text,
    p_hint_set_name text
)
RETURNS void
LANGUAGE plpgsql
VOLATILE
AS $$
BEGIN
    IF p_query_hash IS NULL OR btrim(p_query_hash) = '' THEN
        RAISE EXCEPTION 'query_hash must not be empty';
    END IF;
    IF p_plan_xml IS NULL OR btrim(p_plan_xml) = '' THEN
        RAISE EXCEPTION 'plan_xml must not be empty';
    END IF;
    IF p_hint_set_name IS NULL OR btrim(p_hint_set_name) = '' THEN
        RAISE EXCEPTION 'hint_set_name must not be empty';
    END IF;

    INSERT INTO companion_internal.plan_freezes(query_hash, plan_xml, hint_set_name)
    VALUES (btrim(p_query_hash), p_plan_xml, btrim(p_hint_set_name))
    ON CONFLICT (query_hash) DO UPDATE
    SET plan_xml = EXCLUDED.plan_xml,
        hint_set_name = EXCLUDED.hint_set_name,
        frozen_at = now();
END;
$$;

CREATE FUNCTION companion_internal.plan_auto_promote(
    p_query_hash text,
    p_min_executions integer,
    p_stable_days integer
)
RETURNS void
LANGUAGE plpgsql
VOLATILE
AS $$
BEGIN
    IF p_query_hash IS NULL OR btrim(p_query_hash) = '' THEN
        RAISE EXCEPTION 'query_hash must not be empty';
    END IF;
    IF p_min_executions IS NULL OR p_min_executions <= 0 THEN
        RAISE EXCEPTION 'min_executions must be greater than zero';
    END IF;
    IF p_stable_days IS NULL OR p_stable_days <= 0 THEN
        RAISE EXCEPTION 'stable_days must be greater than zero';
    END IF;
    IF NOT EXISTS (
        SELECT 1
        FROM companion_internal.plan_freezes
        WHERE query_hash = btrim(p_query_hash)
    ) THEN
        RAISE EXCEPTION 'query_hash does not reference a frozen plan';
    END IF;

    INSERT INTO companion_internal.plan_promotion_policies(
        query_hash,
        min_executions,
        stable_days
    )
    VALUES (btrim(p_query_hash), p_min_executions, p_stable_days)
    ON CONFLICT (query_hash) DO UPDATE
    SET min_executions = EXCLUDED.min_executions,
        stable_days = EXCLUDED.stable_days,
        updated_at = now();
END;
$$;

CREATE FUNCTION companion_internal.plan_regression_guard(
    p_query_hash text,
    p_max_latency_regression_percent integer,
    p_max_cost_regression_percent integer
)
RETURNS void
LANGUAGE plpgsql
VOLATILE
AS $$
BEGIN
    IF p_query_hash IS NULL OR btrim(p_query_hash) = '' THEN
        RAISE EXCEPTION 'query_hash must not be empty';
    END IF;
    IF p_max_latency_regression_percent IS NULL OR p_max_latency_regression_percent <= 0 THEN
        RAISE EXCEPTION 'max_latency_regression_percent must be greater than zero';
    END IF;
    IF p_max_cost_regression_percent IS NULL OR p_max_cost_regression_percent <= 0 THEN
        RAISE EXCEPTION 'max_cost_regression_percent must be greater than zero';
    END IF;
    IF NOT EXISTS (
        SELECT 1
        FROM companion_internal.plan_freezes
        WHERE query_hash = btrim(p_query_hash)
    ) THEN
        RAISE EXCEPTION 'query_hash does not reference a frozen plan';
    END IF;

    INSERT INTO companion_internal.plan_regression_policies(
        query_hash,
        max_latency_regression_percent,
        max_cost_regression_percent
    )
    VALUES (
        btrim(p_query_hash),
        p_max_latency_regression_percent,
        p_max_cost_regression_percent
    )
    ON CONFLICT (query_hash) DO UPDATE
    SET max_latency_regression_percent = EXCLUDED.max_latency_regression_percent,
        max_cost_regression_percent = EXCLUDED.max_cost_regression_percent,
        updated_at = now();
END;
$$;

CREATE FUNCTION companion_plan_regression_violates(
    p_query_hash text,
    p_baseline_p95_ms bigint,
    p_candidate_p95_ms bigint,
    p_baseline_cost bigint,
    p_candidate_cost bigint
)
RETURNS boolean
LANGUAGE plpgsql
VOLATILE
AS $$
DECLARE
    policy companion_internal.plan_regression_policies%ROWTYPE;
    violates boolean;
BEGIN
    IF p_query_hash IS NULL OR btrim(p_query_hash) = '' THEN
        RAISE EXCEPTION 'query_hash must not be empty';
    END IF;
    IF p_baseline_p95_ms IS NULL OR p_baseline_p95_ms <= 0 THEN
        RAISE EXCEPTION 'baseline_p95_ms must be greater than zero';
    END IF;
    IF p_candidate_p95_ms IS NULL OR p_candidate_p95_ms < 0 THEN
        RAISE EXCEPTION 'candidate_p95_ms must be non-negative';
    END IF;
    IF p_baseline_cost IS NULL OR p_baseline_cost <= 0 THEN
        RAISE EXCEPTION 'baseline_cost must be greater than zero';
    END IF;
    IF p_candidate_cost IS NULL OR p_candidate_cost < 0 THEN
        RAISE EXCEPTION 'candidate_cost must be non-negative';
    END IF;

    SELECT *
    INTO policy
    FROM companion_internal.plan_regression_policies
    WHERE query_hash = btrim(p_query_hash);

    IF NOT FOUND THEN
        RAISE EXCEPTION 'query_hash does not reference a regression policy';
    END IF;

    violates :=
        p_candidate_p95_ms::numeric * 100
            > p_baseline_p95_ms::numeric * (100 + policy.max_latency_regression_percent)
        OR p_candidate_cost::numeric * 100
            > p_baseline_cost::numeric * (100 + policy.max_cost_regression_percent);

    INSERT INTO companion_internal.plan_regression_samples(
        query_hash,
        baseline_p95_ms,
        candidate_p95_ms,
        baseline_cost,
        candidate_cost,
        violates_policy
    )
    VALUES (
        btrim(p_query_hash),
        p_baseline_p95_ms,
        p_candidate_p95_ms,
        p_baseline_cost,
        p_candidate_cost,
        violates
    );

    RETURN violates;
END;
$$;

-- FEATURE: S6
CREATE VIEW companion_shard_placement_generations AS
SELECT
    shard_id,
    generation,
    worker_name,
    updated_at
FROM companion_internal.shard_placement_generations;

CREATE FUNCTION companion_internal.bump_placement_generation(
    p_shard_id bigint,
    p_worker_name text DEFAULT NULL
)
RETURNS bigint
LANGUAGE plpgsql
VOLATILE
AS $$
DECLARE
    next_generation bigint;
BEGIN
    IF p_shard_id IS NULL OR p_shard_id <= 0 THEN
        RAISE EXCEPTION 'shard_id must be greater than zero';
    END IF;
    IF p_worker_name IS NOT NULL AND btrim(p_worker_name) = '' THEN
        RAISE EXCEPTION 'worker_name must be null or non-empty';
    END IF;

    INSERT INTO companion_internal.shard_placement_generations(
        shard_id,
        generation,
        worker_name
    )
    VALUES (
        p_shard_id,
        1,
        NULLIF(btrim(COALESCE(p_worker_name, '')), '')
    )
    ON CONFLICT (shard_id) DO UPDATE
    SET generation = companion_internal.shard_placement_generations.generation + 1,
        worker_name = COALESCE(
            NULLIF(btrim(COALESCE(EXCLUDED.worker_name, '')), ''),
            companion_internal.shard_placement_generations.worker_name
        ),
        updated_at = now()
    RETURNING generation INTO next_generation;

    RETURN next_generation;
END;
$$;

CREATE FUNCTION companion_placement_generation(p_shard_id bigint)
RETURNS bigint
LANGUAGE plpgsql
STABLE
PARALLEL SAFE
AS $$
DECLARE
    current_generation bigint;
BEGIN
    IF p_shard_id IS NULL OR p_shard_id <= 0 THEN
        RAISE EXCEPTION 'shard_id must be greater than zero';
    END IF;

    SELECT generation
    INTO current_generation
    FROM companion_internal.shard_placement_generations
    WHERE shard_id = p_shard_id;

    RETURN COALESCE(current_generation, 0);
END;
$$;

CREATE FUNCTION companion_local_placement_matches(
    p_shard_id bigint,
    p_worker_name text
)
RETURNS boolean
LANGUAGE plpgsql
STABLE
PARALLEL SAFE
AS $$
DECLARE
    recorded_worker text;
BEGIN
    IF p_shard_id IS NULL OR p_shard_id <= 0 THEN
        RAISE EXCEPTION 'shard_id must be greater than zero';
    END IF;
    IF p_worker_name IS NULL OR btrim(p_worker_name) = '' THEN
        RAISE EXCEPTION 'worker_name must not be empty';
    END IF;

    SELECT worker_name
    INTO recorded_worker
    FROM companion_internal.shard_placement_generations
    WHERE shard_id = p_shard_id;

    RETURN recorded_worker = btrim(p_worker_name);
END;
$$;

-- FEATURE: S13
CREATE FUNCTION companion_hash_shard_index(
    p_value text,
    p_shard_count integer
)
RETURNS integer
LANGUAGE plpgsql
IMMUTABLE
PARALLEL SAFE
AS $$
DECLARE
    digest_bytes bytea;
    hash_value bigint;
BEGIN
    IF p_value IS NULL THEN
        RAISE EXCEPTION 'routing value must not be null';
    END IF;
    IF p_shard_count IS NULL OR p_shard_count <= 0 THEN
        RAISE EXCEPTION 'shard_count must be greater than zero';
    END IF;

    digest_bytes := decode(substr(md5(p_value), 1, 8), 'hex');
    hash_value :=
        get_byte(digest_bytes, 0)::bigint * 16777216
      + get_byte(digest_bytes, 1)::bigint * 65536
      + get_byte(digest_bytes, 2)::bigint * 256
      + get_byte(digest_bytes, 3)::bigint;

    RETURN (hash_value % p_shard_count)::integer;
END;
$$;

CREATE FUNCTION companion_range_shard_index(
    p_value numeric,
    p_lower_bound numeric,
    p_upper_bound numeric,
    p_shard_count integer
)
RETURNS integer
LANGUAGE plpgsql
IMMUTABLE
PARALLEL SAFE
AS $$
DECLARE
    value_span numeric;
    shard_index integer;
BEGIN
    IF p_value IS NULL THEN
        RAISE EXCEPTION 'routing value must not be null';
    END IF;
    IF p_lower_bound IS NULL OR p_upper_bound IS NULL THEN
        RAISE EXCEPTION 'range bounds must not be null';
    END IF;
    IF p_upper_bound <= p_lower_bound THEN
        RAISE EXCEPTION 'upper_bound must be greater than lower_bound';
    END IF;
    IF p_shard_count IS NULL OR p_shard_count <= 0 THEN
        RAISE EXCEPTION 'shard_count must be greater than zero';
    END IF;
    IF p_value < p_lower_bound OR p_value >= p_upper_bound THEN
        RAISE EXCEPTION 'range routing value is outside shard bounds';
    END IF;

    value_span := p_upper_bound - p_lower_bound;
    shard_index := floor(((p_value - p_lower_bound) * p_shard_count) / value_span)::integer;

    IF shard_index >= p_shard_count THEN
        RETURN p_shard_count - 1;
    END IF;
    RETURN shard_index;
END;
$$;

-- FEATURE: Auth2
CREATE FUNCTION companion_set_session_claims(
    uid text,
    claim_role text,
    tenant_id text,
    jwt_id text DEFAULT NULL,
    is_local boolean DEFAULT false
)
RETURNS void
LANGUAGE plpgsql
VOLATILE
AS $$
BEGIN
    IF uid IS NULL OR btrim(uid) = '' THEN
        RAISE EXCEPTION 'uid claim must not be empty';
    END IF;
    IF claim_role IS NULL OR btrim(claim_role) = '' THEN
        RAISE EXCEPTION 'role claim must not be empty';
    END IF;
    IF tenant_id IS NULL OR btrim(tenant_id) = '' THEN
        RAISE EXCEPTION 'tenant_id claim must not be empty';
    END IF;
    IF jwt_id IS NOT NULL AND btrim(jwt_id) = '' THEN
        RAISE EXCEPTION 'jwt_id claim must be null or non-empty';
    END IF;

    PERFORM set_config('ai_blaise.claim.uid', btrim(uid), is_local);
    PERFORM set_config('ai_blaise.claim.role', btrim(claim_role), is_local);
    PERFORM set_config('ai_blaise.claim.tenant_id', btrim(tenant_id), is_local);
    PERFORM set_config(
        'ai_blaise.claim.jwt_id',
        COALESCE(NULLIF(btrim(jwt_id), ''), ''),
        is_local
    );
END;
$$;

CREATE FUNCTION companion_current_session_claims()
RETURNS TABLE(uid text, role text, tenant_id text, jwt_id text)
LANGUAGE sql
STABLE
PARALLEL SAFE
AS $$
    SELECT
        NULLIF(current_setting('ai_blaise.claim.uid', true), '') AS uid,
        NULLIF(current_setting('ai_blaise.claim.role', true), '') AS role,
        NULLIF(current_setting('ai_blaise.claim.tenant_id', true), '') AS tenant_id,
        NULLIF(current_setting('ai_blaise.claim.jwt_id', true), '') AS jwt_id
$$;

CREATE FUNCTION companion_current_tenant_id()
RETURNS text
LANGUAGE sql
STABLE
PARALLEL SAFE
AS $$
    SELECT NULLIF(current_setting('ai_blaise.claim.tenant_id', true), '')
$$;

-- FEATURE: Sec2
CREATE FUNCTION companion_internal.base64url_encode(input bytea)
RETURNS text
LANGUAGE sql
IMMUTABLE
STRICT
PARALLEL SAFE
AS $$
    SELECT rtrim(
        translate(regexp_replace(encode(input, 'base64'), '\s', '', 'g'), '+/', '-_'),
        '='
    )
$$;

CREATE FUNCTION companion_internal.base64url_decode(input text)
RETURNS bytea
LANGUAGE sql
IMMUTABLE
STRICT
PARALLEL SAFE
AS $$
    WITH normalized AS (
        SELECT translate(input, '-_', '+/') AS value
    ),
    padded AS (
        SELECT value || repeat('=', (4 - length(value) % 4) % 4) AS value
        FROM normalized
    )
    SELECT decode(value, 'base64')
    FROM padded
$$;

CREATE FUNCTION companion_internal.jwt_audience_matches(
    payload jsonb,
    p_expected_audience text
)
RETURNS boolean
LANGUAGE sql
STABLE
PARALLEL SAFE
AS $$
    SELECT CASE jsonb_typeof(payload->'aud')
        WHEN 'string' THEN payload->>'aud' = p_expected_audience
        WHEN 'array' THEN EXISTS (
            SELECT 1
            FROM jsonb_array_elements_text(payload->'aud') AS audience(value)
            WHERE audience.value = p_expected_audience
        )
        ELSE false
    END
$$;

CREATE FUNCTION companion_verify_jwt_hs256(
    p_token text,
    p_shared_secret text,
    p_expected_issuer text,
    p_expected_audience text,
    p_leeway_seconds integer DEFAULT 0
)
RETURNS TABLE(
    uid text,
    role text,
    tenant_id text,
    jwt_id text,
    issuer text,
    audience text,
    expires_at timestamptz
)
LANGUAGE plpgsql
VOLATILE
AS $$
DECLARE
    parts text[];
    header_json jsonb;
    payload_json jsonb;
    signing_input text;
    expected_signature text;
    exp_epoch numeric;
    nbf_epoch numeric;
    now_epoch numeric;
BEGIN
    PERFORM companion_internal.require_visible_function('hmac', 'pgcrypto');

    IF p_token IS NULL OR btrim(p_token) = '' THEN
        RAISE EXCEPTION 'JWT token must not be empty';
    END IF;
    IF p_shared_secret IS NULL OR btrim(p_shared_secret) = '' THEN
        RAISE EXCEPTION 'JWT shared secret must not be empty';
    END IF;
    IF p_expected_issuer IS NULL OR btrim(p_expected_issuer) = '' THEN
        RAISE EXCEPTION 'JWT expected issuer must not be empty';
    END IF;
    IF p_expected_audience IS NULL OR btrim(p_expected_audience) = '' THEN
        RAISE EXCEPTION 'JWT expected audience must not be empty';
    END IF;
    IF p_leeway_seconds IS NULL OR p_leeway_seconds < 0 THEN
        RAISE EXCEPTION 'JWT leeway seconds must be non-negative';
    END IF;

    parts := string_to_array(p_token, '.');
    IF COALESCE(array_length(parts, 1), 0) <> 3
       OR parts[1] = ''
       OR parts[2] = ''
       OR parts[3] = '' THEN
        RAISE EXCEPTION 'JWT token must have three non-empty segments';
    END IF;

    header_json := convert_from(companion_internal.base64url_decode(parts[1]), 'UTF8')::jsonb;
    payload_json := convert_from(companion_internal.base64url_decode(parts[2]), 'UTF8')::jsonb;

    IF COALESCE(header_json->>'alg', '') <> 'HS256' THEN
        RAISE EXCEPTION 'unsupported JWT alg: %', COALESCE(header_json->>'alg', '<missing>');
    END IF;
    IF header_json ? 'typ' AND upper(header_json->>'typ') <> 'JWT' THEN
        RAISE EXCEPTION 'unsupported JWT typ: %', header_json->>'typ';
    END IF;

    signing_input := parts[1] || '.' || parts[2];
    expected_signature := companion_internal.base64url_encode(
        hmac(signing_input, p_shared_secret, 'sha256')
    );
    IF parts[3] <> expected_signature THEN
        RAISE EXCEPTION 'JWT signature verification failed';
    END IF;

    IF payload_json->>'iss' <> p_expected_issuer THEN
        RAISE EXCEPTION 'JWT issuer mismatch';
    END IF;
    IF NOT companion_internal.jwt_audience_matches(payload_json, p_expected_audience) THEN
        RAISE EXCEPTION 'JWT audience mismatch';
    END IF;
    IF NOT payload_json ? 'exp' THEN
        RAISE EXCEPTION 'JWT exp claim must be present';
    END IF;

    exp_epoch := (payload_json->>'exp')::numeric;
    now_epoch := extract(epoch FROM clock_timestamp());
    IF exp_epoch + p_leeway_seconds < now_epoch THEN
        RAISE EXCEPTION 'JWT has expired';
    END IF;

    IF payload_json ? 'nbf' THEN
        nbf_epoch := (payload_json->>'nbf')::numeric;
        IF nbf_epoch - p_leeway_seconds > now_epoch THEN
            RAISE EXCEPTION 'JWT is not yet valid';
        END IF;
    END IF;

    IF payload_json->>'sub' IS NULL OR btrim(payload_json->>'sub') = '' THEN
        RAISE EXCEPTION 'JWT sub claim must not be empty';
    END IF;
    IF payload_json->>'role' IS NULL OR btrim(payload_json->>'role') = '' THEN
        RAISE EXCEPTION 'JWT role claim must not be empty';
    END IF;
    IF payload_json->>'tenant_id' IS NULL OR btrim(payload_json->>'tenant_id') = '' THEN
        RAISE EXCEPTION 'JWT tenant_id claim must not be empty';
    END IF;
    IF payload_json ? 'jti' AND btrim(payload_json->>'jti') = '' THEN
        RAISE EXCEPTION 'JWT jti claim must be absent or non-empty';
    END IF;

    RETURN QUERY SELECT
        btrim(payload_json->>'sub'),
        btrim(payload_json->>'role'),
        btrim(payload_json->>'tenant_id'),
        NULLIF(btrim(COALESCE(payload_json->>'jti', '')), ''),
        payload_json->>'iss',
        CASE
            WHEN jsonb_typeof(payload_json->'aud') = 'string' THEN payload_json->>'aud'
            ELSE p_expected_audience
        END,
        to_timestamp(exp_epoch::double precision);
END;
$$;

-- FEATURE: Sec1
CREATE FUNCTION companion_require_tenant_id()
RETURNS text
LANGUAGE plpgsql
STABLE
PARALLEL SAFE
AS $$
DECLARE
    current_tenant_id text;
BEGIN
    current_tenant_id := companion_current_tenant_id();
    IF current_tenant_id IS NULL OR btrim(current_tenant_id) = '' THEN
        RAISE EXCEPTION 'tenant_id claim must be set for RLS';
    END IF;
    RETURN current_tenant_id;
END;
$$;

CREATE FUNCTION companion_tenant_id_matches(row_tenant_id text)
RETURNS boolean
LANGUAGE sql
STABLE
PARALLEL SAFE
AS $$
    SELECT row_tenant_id IS NOT NULL
       AND companion_current_tenant_id() IS NOT NULL
       AND row_tenant_id = companion_current_tenant_id()
$$;

CREATE FUNCTION companion_tenant_id_matches(row_tenant_id uuid)
RETURNS boolean
LANGUAGE sql
STABLE
PARALLEL SAFE
AS $$
    SELECT companion_tenant_id_matches(row_tenant_id::text)
$$;

CREATE FUNCTION companion_internal.identifier_list(input text)
RETURNS text[]
LANGUAGE sql
IMMUTABLE
PARALLEL SAFE
AS $$
    SELECT COALESCE(
        array_agg(btrim(part)) FILTER (WHERE btrim(part) <> ''),
        ARRAY[]::text[]
    )
    FROM unnest(string_to_array(COALESCE(input, ''), ',')) AS part
$$;

CREATE FUNCTION companion_internal.visible_function_exists(function_name name)
RETURNS boolean
LANGUAGE sql
STABLE
PARALLEL SAFE
AS $$
    SELECT EXISTS (
        SELECT 1
        FROM pg_proc AS proc
        JOIN pg_namespace AS namespace ON namespace.oid = proc.pronamespace
        WHERE proc.proname = function_name
          AND pg_function_is_visible(proc.oid)
    )
$$;

CREATE FUNCTION companion_internal.require_visible_function(
    function_name name,
    extension_name text
)
RETURNS void
LANGUAGE plpgsql
STABLE
AS $$
BEGIN
    IF NOT companion_internal.visible_function_exists(function_name) THEN
        RAISE EXCEPTION '% requires visible function % from extension %',
            current_query(), function_name, extension_name;
    END IF;
END;
$$;

-- FEATURE: Sec5
-- FEATURE: Sec6
CREATE TABLE IF NOT EXISTS companion_internal.ledger_entries (
    ledger_sequence bigserial PRIMARY KEY,
    transfer_id text NOT NULL UNIQUE,
    debit_account text NOT NULL,
    credit_account text NOT NULL,
    amount_cents bigint NOT NULL CHECK (amount_cents > 0),
    currency text NOT NULL,
    previous_hash text NOT NULL,
    entry_hash text NOT NULL UNIQUE,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS companion_internal.ledger_seals (
    seal_sequence bigserial PRIMARY KEY,
    transfer_id text NOT NULL UNIQUE
        REFERENCES companion_internal.ledger_entries(transfer_id),
    hmac_algorithm text NOT NULL,
    seal text NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE FUNCTION companion_internal.prevent_ledger_mutation()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    RAISE EXCEPTION 'companion ledger is append-only';
END;
$$;

CREATE TRIGGER companion_ledger_entries_append_only
BEFORE UPDATE OR DELETE ON companion_internal.ledger_entries
FOR EACH ROW EXECUTE FUNCTION companion_internal.prevent_ledger_mutation();

CREATE TRIGGER companion_ledger_seals_append_only
BEFORE UPDATE OR DELETE ON companion_internal.ledger_seals
FOR EACH ROW EXECUTE FUNCTION companion_internal.prevent_ledger_mutation();

CREATE VIEW companion_ledger_entries AS
SELECT
    entries.ledger_sequence,
    entries.transfer_id,
    entries.debit_account,
    entries.credit_account,
    entries.amount_cents,
    entries.currency,
    entries.previous_hash,
    entries.entry_hash,
    seals.hmac_algorithm,
    seals.seal,
    entries.created_at
FROM companion_internal.ledger_entries AS entries
LEFT JOIN companion_internal.ledger_seals AS seals
  ON seals.transfer_id = entries.transfer_id;

CREATE FUNCTION companion_internal.ledger_transfer(
    p_transfer_id text,
    p_debit_account text,
    p_credit_account text,
    p_amount_cents bigint,
    p_currency text,
    p_previous_hash text
)
RETURNS text
LANGUAGE plpgsql
VOLATILE
AS $$
DECLARE
    canonical_payload text;
    computed_hash text;
BEGIN
    PERFORM companion_internal.require_visible_function('digest', 'pgcrypto');

    IF p_transfer_id IS NULL OR btrim(p_transfer_id) = '' THEN
        RAISE EXCEPTION 'transfer_id must not be empty';
    END IF;
    IF p_debit_account IS NULL OR btrim(p_debit_account) = '' THEN
        RAISE EXCEPTION 'debit_account must not be empty';
    END IF;
    IF p_credit_account IS NULL OR btrim(p_credit_account) = '' THEN
        RAISE EXCEPTION 'credit_account must not be empty';
    END IF;
    IF btrim(p_debit_account) = btrim(p_credit_account) THEN
        RAISE EXCEPTION 'debit_account and credit_account must differ';
    END IF;
    IF p_amount_cents IS NULL OR p_amount_cents <= 0 THEN
        RAISE EXCEPTION 'amount_cents must be greater than zero';
    END IF;
    IF p_currency IS NULL OR btrim(p_currency) = '' THEN
        RAISE EXCEPTION 'currency must not be empty';
    END IF;
    IF p_previous_hash IS NULL OR btrim(p_previous_hash) = '' THEN
        RAISE EXCEPTION 'previous_hash must not be empty';
    END IF;
    IF btrim(p_previous_hash) <> 'genesis'
       AND NOT EXISTS (
           SELECT 1
           FROM companion_internal.ledger_entries AS entries
           WHERE entries.entry_hash = btrim(p_previous_hash)
       ) THEN
        RAISE EXCEPTION 'previous_hash does not reference an existing ledger entry';
    END IF;

    canonical_payload := concat_ws(
        '|',
        btrim(p_transfer_id),
        btrim(p_debit_account),
        btrim(p_credit_account),
        p_amount_cents::text,
        upper(btrim(p_currency)),
        btrim(p_previous_hash)
    );
    computed_hash := encode(digest(canonical_payload, 'sha256'), 'hex');

    INSERT INTO companion_internal.ledger_entries(
        transfer_id,
        debit_account,
        credit_account,
        amount_cents,
        currency,
        previous_hash,
        entry_hash
    )
    VALUES (
        btrim(p_transfer_id),
        btrim(p_debit_account),
        btrim(p_credit_account),
        p_amount_cents,
        upper(btrim(p_currency)),
        btrim(p_previous_hash),
        computed_hash
    );

    RETURN computed_hash;
END;
$$;

CREATE FUNCTION companion_ledger_chain_valid()
RETURNS boolean
LANGUAGE sql
STABLE
AS $$
    WITH ordered AS (
        SELECT
            ledger_sequence,
            previous_hash,
            lag(entry_hash) OVER (ORDER BY ledger_sequence) AS expected_previous_hash
        FROM companion_internal.ledger_entries
    )
    SELECT NOT EXISTS (
        SELECT 1
        FROM ordered
        WHERE previous_hash <> COALESCE(expected_previous_hash, 'genesis')
    )
$$;

CREATE FUNCTION companion_ledger_seal(
    p_transfer_id text,
    p_secret text,
    p_algorithm text DEFAULT 'hmac-sha256'
)
RETURNS text
LANGUAGE plpgsql
VOLATILE
AS $$
DECLARE
    entry companion_internal.ledger_entries%ROWTYPE;
    hmac_type text;
    computed_seal text;
BEGIN
    PERFORM companion_internal.require_visible_function('hmac', 'pgcrypto');

    IF p_transfer_id IS NULL OR btrim(p_transfer_id) = '' THEN
        RAISE EXCEPTION 'transfer_id must not be empty';
    END IF;
    IF p_secret IS NULL OR p_secret = '' THEN
        RAISE EXCEPTION 'ledger seal secret must not be empty';
    END IF;
    IF lower(btrim(p_algorithm)) = 'hmac-sha256' THEN
        hmac_type := 'sha256';
    ELSIF lower(btrim(p_algorithm)) = 'hmac-sha512' THEN
        hmac_type := 'sha512';
    ELSE
        RAISE EXCEPTION 'unsupported ledger HMAC algorithm: %', p_algorithm;
    END IF;

    SELECT *
    INTO entry
    FROM companion_internal.ledger_entries AS entries
    WHERE entries.transfer_id = btrim(p_transfer_id);
    IF NOT FOUND THEN
        RAISE EXCEPTION 'ledger transfer_id does not exist: %', p_transfer_id;
    END IF;

    computed_seal := encode(hmac(entry.entry_hash, p_secret, hmac_type), 'hex');
    INSERT INTO companion_internal.ledger_seals(transfer_id, hmac_algorithm, seal)
    VALUES (entry.transfer_id, lower(btrim(p_algorithm)), computed_seal);

    RETURN computed_seal;
END;
$$;

CREATE FUNCTION companion_internal.record_timescale_bridge_state(
    feature_id text,
    object_name text,
    parameters jsonb DEFAULT '{}'::jsonb
)
RETURNS void
LANGUAGE plpgsql
VOLATILE
AS $$
BEGIN
    IF btrim(feature_id) = '' THEN
        RAISE EXCEPTION 'feature_id must not be empty';
    END IF;
    IF btrim(object_name) = '' THEN
        RAISE EXCEPTION 'object_name must not be empty';
    END IF;

    INSERT INTO companion_internal.timescale_bridge_state(
        feature_id,
        object_name,
        parameters
    )
    VALUES (
        feature_id,
        object_name,
        COALESCE(parameters, '{}'::jsonb)
    );
END;
$$;

CREATE FUNCTION companion_internal.create_worker_hypertables(
    table_name regclass,
    distribution_column name,
    chunk_time_interval interval,
    shard_count integer
)
RETURNS void
LANGUAGE plpgsql
VOLATILE
AS $$
BEGIN
    IF shard_count <= 0 THEN
        RAISE EXCEPTION 'shard_count must be greater than zero';
    END IF;

    PERFORM companion_internal.record_timescale_bridge_state(
        'TS1',
        table_name::text,
        jsonb_build_object(
            'distribution_column', distribution_column::text,
            'chunk_time_interval', chunk_time_interval::text,
            'shard_count', shard_count
        )
    );
END;
$$;

CREATE FUNCTION companion_internal.add_compression_policy_distributed(
    table_name regclass,
    older_than interval,
    segment_by text[] DEFAULT ARRAY[]::text[],
    order_by text[] DEFAULT ARRAY[]::text[]
)
RETURNS void
LANGUAGE plpgsql
VOLATILE
AS $$
BEGIN
    PERFORM companion_internal.record_timescale_bridge_state(
        'TS2',
        table_name::text,
        jsonb_build_object(
            'older_than', older_than::text,
            'segment_by', COALESCE(segment_by, ARRAY[]::text[]),
            'order_by', COALESCE(order_by, ARRAY[]::text[])
        )
    );
END;
$$;

CREATE FUNCTION companion_internal.add_retention_policy_distributed(
    table_name regclass,
    drop_after interval
)
RETURNS void
LANGUAGE plpgsql
VOLATILE
AS $$
BEGIN
    PERFORM companion_internal.record_timescale_bridge_state(
        'TS4',
        table_name::text,
        jsonb_build_object('drop_after', drop_after::text)
    );
END;
$$;

CREATE FUNCTION companion_internal.add_reorder_policy_distributed(
    table_name regclass,
    index_name name
)
RETURNS void
LANGUAGE plpgsql
VOLATILE
AS $$
BEGIN
    PERFORM companion_internal.record_timescale_bridge_state(
        'TS12',
        table_name::text,
        jsonb_build_object('index_name', index_name::text)
    );
END;
$$;

CREATE FUNCTION companion_internal.add_continuous_aggregate_distributed(
    view_name text,
    view_query text,
    refresh_start interval,
    refresh_end interval,
    schedule interval
)
RETURNS void
LANGUAGE plpgsql
VOLATILE
AS $$
BEGIN
    IF btrim(view_name) = '' THEN
        RAISE EXCEPTION 'view_name must not be empty';
    END IF;
    IF btrim(view_query) = '' THEN
        RAISE EXCEPTION 'view_query must not be empty';
    END IF;

    PERFORM companion_internal.record_timescale_bridge_state(
        'TS3',
        view_name,
        jsonb_build_object(
            'view_query', view_query,
            'refresh_start', refresh_start::text,
            'refresh_end', refresh_end::text,
            'schedule', schedule::text
        )
    );
END;
$$;

CREATE FUNCTION companion_internal.enable_time_range_shard_pruner(
    distributed_table regclass,
    time_column name
)
RETURNS void
LANGUAGE plpgsql
VOLATILE
AS $$
BEGIN
    PERFORM companion_internal.record_timescale_bridge_state(
        'TS5',
        distributed_table::text,
        jsonb_build_object('time_column', time_column::text)
    );
END;
$$;

CREATE FUNCTION companion_query_percentiles()
RETURNS TABLE(
    query text,
    calls bigint,
    mean_ms numeric,
    p95_ms numeric,
    p99_ms numeric,
    p999_ms numeric
)
LANGUAGE plpgsql
STABLE
AS $$
BEGIN
    IF to_regclass('pg_stat_statements') IS NULL THEN
        RETURN;
    END IF;

    RETURN QUERY EXECUTE
        $sql$
            SELECT
                query::text,
                calls::bigint,
                round(mean_exec_time::numeric, 3) AS mean_ms,
                round(GREATEST(mean_exec_time, mean_exec_time + (1.645 * stddev_exec_time))::numeric, 3) AS p95_ms,
                round(GREATEST(mean_exec_time, mean_exec_time + (2.326 * stddev_exec_time))::numeric, 3) AS p99_ms,
                round(GREATEST(mean_exec_time, mean_exec_time + (3.090 * stddev_exec_time))::numeric, 3) AS p999_ms
            FROM pg_stat_statements
            WHERE calls > 0
            ORDER BY total_exec_time DESC
            LIMIT 100
        $sql$;
END;
$$;

CREATE VIEW companion_pg_stat_statements_p95 AS
SELECT * FROM companion_query_percentiles();

CREATE FUNCTION companion_pg_stat_local_activity()
RETURNS TABLE(
    database_name name,
    node_addr inet,
    active_sessions bigint,
    idle_in_transaction_sessions bigint,
    waiting_sessions bigint
)
LANGUAGE sql
STABLE
PARALLEL SAFE
AS $$
    SELECT
        activity.datname,
        inet_server_addr(),
        count(*) FILTER (WHERE activity.state = 'active')::bigint,
        count(*) FILTER (WHERE activity.state = 'idle in transaction')::bigint,
        count(*) FILTER (WHERE activity.wait_event IS NOT NULL)::bigint
    FROM pg_stat_activity AS activity
    WHERE activity.datname IS NOT NULL
    GROUP BY activity.datname, inet_server_addr()
$$;

CREATE VIEW companion_pg_stat_local_activity AS
SELECT * FROM companion_pg_stat_local_activity();

CREATE FUNCTION companion_pg_stat_distributed()
RETURNS TABLE(
    database_name name,
    node_addr inet,
    active_sessions bigint,
    idle_in_transaction_sessions bigint,
    waiting_sessions bigint
)
LANGUAGE sql
STABLE
PARALLEL SAFE
AS $$
    SELECT * FROM companion_pg_stat_local_activity()
$$;

CREATE VIEW companion_pg_stat_distributed AS
SELECT * FROM companion_pg_stat_distributed();

CREATE FUNCTION companion_pg_dist_replication_lag()
RETURNS TABLE(
    application_name text,
    client_addr inet,
    state text,
    write_lag interval,
    flush_lag interval,
    replay_lag interval,
    lag_bytes numeric
)
LANGUAGE sql
STABLE
PARALLEL SAFE
AS $$
    SELECT
        replication.application_name::text,
        replication.client_addr,
        replication.state::text,
        replication.write_lag,
        replication.flush_lag,
        replication.replay_lag,
        pg_wal_lsn_diff(pg_current_wal_lsn(), replication.replay_lsn)::numeric
    FROM pg_stat_replication AS replication
$$;

CREATE VIEW companion_pg_dist_replication_lag AS
SELECT * FROM companion_pg_dist_replication_lag();

CREATE FUNCTION companion_idle_transactions(max_idle interval DEFAULT '60 seconds'::interval)
RETURNS TABLE(
    pid integer,
    usename name,
    application_name text,
    client_addr inet,
    idle_for interval,
    state text
)
LANGUAGE plpgsql
STABLE
AS $$
BEGIN
    IF max_idle <= '0 seconds'::interval THEN
        RAISE EXCEPTION 'max_idle must be greater than zero';
    END IF;

    RETURN QUERY
    SELECT
        activity.pid,
        activity.usename,
        activity.application_name::text,
        activity.client_addr,
        now() - activity.xact_start,
        activity.state::text
    FROM pg_stat_activity AS activity
    WHERE activity.state = 'idle in transaction'
      AND activity.xact_start IS NOT NULL
      AND now() - activity.xact_start >= max_idle;
END;
$$;

CREATE VIEW companion_idle_transactions_over_60s AS
SELECT * FROM companion_idle_transactions('60 seconds'::interval);

CREATE FUNCTION companion_distribute_hypertable_plan(
    table_name regclass,
    distribution_column name,
    chunk_time_interval interval,
    shard_count integer DEFAULT 32
)
RETURNS text
LANGUAGE plpgsql
STABLE
AS $$
BEGIN
    IF shard_count <= 0 THEN
        RAISE EXCEPTION 'shard_count must be greater than zero';
    END IF;

    RETURN format(
$plan$SELECT create_hypertable(%1$L::regclass, %2$L, chunk_time_interval => %3$L::interval, if_not_exists => true);
SELECT create_distributed_table(%1$L::regclass, %2$L, shard_count => %4$s);
SELECT companion_internal.create_worker_hypertables(%1$L::regclass, %2$L, %3$L::interval, %4$s);$plan$,
        table_name::text,
        distribution_column::text,
        chunk_time_interval::text,
        shard_count
    );
END;
$$;

CREATE FUNCTION distribute_hypertable(
    table_name text,
    dist_col text,
    chunk_time_interval text,
    num_shards integer DEFAULT 32
)
RETURNS text
LANGUAGE sql
STABLE
AS $$
    SELECT companion_distribute_hypertable_plan(
        table_name::regclass,
        dist_col::name,
        chunk_time_interval::interval,
        num_shards
    )
$$;

CREATE FUNCTION apply_distribute_hypertable(
    table_name text,
    dist_col text,
    chunk_time_interval text,
    num_shards integer DEFAULT 32
)
RETURNS void
LANGUAGE plpgsql
VOLATILE
AS $$
DECLARE
    table_regclass regclass := table_name::regclass;
BEGIN
    IF num_shards <= 0 THEN
        RAISE EXCEPTION 'num_shards must be greater than zero';
    END IF;

    PERFORM companion_internal.require_visible_function(
        'create_hypertable',
        'timescaledb'
    );
    PERFORM companion_internal.require_visible_function(
        'create_distributed_table',
        'citus'
    );

    EXECUTE format(
        'SELECT create_hypertable(%L::regclass, %L, chunk_time_interval => %L::interval, if_not_exists => true)',
        table_regclass::text,
        dist_col,
        chunk_time_interval
    );
    EXECUTE format(
        'SELECT create_distributed_table(%L::regclass, %L, shard_count => %s)',
        table_regclass::text,
        dist_col,
        num_shards
    );
    PERFORM companion_internal.create_worker_hypertables(
        table_regclass,
        dist_col::name,
        chunk_time_interval::interval,
        num_shards
    );
END;
$$;

CREATE FUNCTION companion_add_compression_policy_distributed_plan(
    table_name regclass,
    older_than interval,
    segment_by text[] DEFAULT ARRAY[]::text[],
    order_by text[] DEFAULT ARRAY[]::text[]
)
RETURNS text
LANGUAGE plpgsql
STABLE
AS $$
BEGIN
    RETURN format(
$plan$ALTER TABLE %1$s SET (timescaledb.compress);
SELECT add_compression_policy(%1$L::regclass, %2$L::interval);
SELECT companion_internal.add_compression_policy_distributed(%1$L::regclass, %2$L::interval, %3$L::text[], %4$L::text[]);$plan$,
        table_name::text,
        older_than::text,
        segment_by::text,
        order_by::text
    );
END;
$$;

CREATE FUNCTION add_compression_policy_distributed(
    table_name text,
    older_than text,
    segment_by text,
    order_by text
)
RETURNS text
LANGUAGE sql
STABLE
AS $$
    SELECT companion_add_compression_policy_distributed_plan(
        table_name::regclass,
        older_than::interval,
        companion_internal.identifier_list(segment_by),
        companion_internal.identifier_list(order_by)
    )
$$;

CREATE FUNCTION apply_compression_policy_distributed(
    table_name text,
    older_than text,
    segment_by text DEFAULT '',
    order_by text DEFAULT ''
)
RETURNS void
LANGUAGE plpgsql
VOLATILE
AS $$
DECLARE
    table_regclass regclass := table_name::regclass;
    segment_columns text[] := companion_internal.identifier_list(segment_by);
    order_columns text[] := companion_internal.identifier_list(order_by);
    compression_options text := 'timescaledb.compress';
BEGIN
    PERFORM companion_internal.require_visible_function(
        'add_compression_policy',
        'timescaledb'
    );

    IF cardinality(segment_columns) > 0 THEN
        compression_options := compression_options || format(
            ', timescaledb.compress_segmentby = %L',
            array_to_string(segment_columns, ',')
        );
    END IF;
    IF cardinality(order_columns) > 0 THEN
        compression_options := compression_options || format(
            ', timescaledb.compress_orderby = %L',
            array_to_string(order_columns, ',')
        );
    END IF;

    EXECUTE format('ALTER TABLE %s SET (%s)', table_regclass, compression_options);
    EXECUTE format(
        'SELECT add_compression_policy(%L::regclass, %L::interval)',
        table_regclass::text,
        older_than
    );
    PERFORM companion_internal.add_compression_policy_distributed(
        table_regclass,
        older_than::interval,
        segment_columns,
        order_columns
    );
END;
$$;

CREATE FUNCTION companion_add_retention_policy_distributed_plan(
    table_name regclass,
    drop_after interval
)
RETURNS text
LANGUAGE plpgsql
STABLE
AS $$
BEGIN
    RETURN format(
$plan$SELECT add_retention_policy(%1$L::regclass, %2$L::interval);
SELECT companion_internal.add_retention_policy_distributed(%1$L::regclass, %2$L::interval);$plan$,
        table_name::text,
        drop_after::text
    );
END;
$$;

CREATE FUNCTION add_retention_policy_distributed(
    table_name text,
    drop_after text
)
RETURNS text
LANGUAGE sql
STABLE
AS $$
    SELECT companion_add_retention_policy_distributed_plan(
        table_name::regclass,
        drop_after::interval
    )
$$;

CREATE FUNCTION apply_retention_policy_distributed(
    table_name text,
    drop_after text
)
RETURNS void
LANGUAGE plpgsql
VOLATILE
AS $$
DECLARE
    table_regclass regclass := table_name::regclass;
BEGIN
    PERFORM companion_internal.require_visible_function(
        'add_retention_policy',
        'timescaledb'
    );

    EXECUTE format(
        'SELECT add_retention_policy(%L::regclass, %L::interval)',
        table_regclass::text,
        drop_after
    );
    PERFORM companion_internal.add_retention_policy_distributed(
        table_regclass,
        drop_after::interval
    );
END;
$$;

CREATE FUNCTION companion_add_reorder_policy_distributed_plan(
    table_name regclass,
    index_name name
)
RETURNS text
LANGUAGE plpgsql
STABLE
AS $$
BEGIN
    RETURN format(
$plan$SELECT add_reorder_policy(%1$L::regclass, %2$L);
SELECT companion_internal.add_reorder_policy_distributed(%1$L::regclass, %2$L);$plan$,
        table_name::text,
        index_name::text
    );
END;
$$;

CREATE FUNCTION add_reorder_policy_distributed(
    table_name text,
    index_name text
)
RETURNS text
LANGUAGE sql
STABLE
AS $$
    SELECT companion_add_reorder_policy_distributed_plan(
        table_name::regclass,
        index_name::name
    )
$$;

CREATE FUNCTION apply_reorder_policy_distributed(
    table_name text,
    index_name text
)
RETURNS void
LANGUAGE plpgsql
VOLATILE
AS $$
DECLARE
    table_regclass regclass := table_name::regclass;
BEGIN
    PERFORM companion_internal.require_visible_function(
        'add_reorder_policy',
        'timescaledb'
    );

    EXECUTE format(
        'SELECT add_reorder_policy(%L::regclass, %L)',
        table_regclass::text,
        index_name
    );
    PERFORM companion_internal.add_reorder_policy_distributed(
        table_regclass,
        index_name::name
    );
END;
$$;

CREATE FUNCTION companion_add_continuous_aggregate_distributed_plan(
    view_name text,
    view_query text,
    refresh_start interval,
    refresh_end interval,
    schedule interval
)
RETURNS text
LANGUAGE plpgsql
STABLE
AS $$
BEGIN
    IF btrim(view_name) = '' THEN
        RAISE EXCEPTION 'view_name must not be empty';
    END IF;
    IF btrim(view_query) = '' THEN
        RAISE EXCEPTION 'view_query must not be empty';
    END IF;

    RETURN format(
$plan$CREATE MATERIALIZED VIEW %1$I
WITH (timescaledb.continuous) AS
%2$s
WITH NO DATA;
SELECT add_continuous_aggregate_policy(%1$L, start_offset => %3$L::interval, end_offset => %4$L::interval, schedule_interval => %5$L::interval);
SELECT companion_internal.add_continuous_aggregate_distributed(%1$L, %2$L, %3$L::interval, %4$L::interval, %5$L::interval);$plan$,
        view_name,
        view_query,
        refresh_start::text,
        refresh_end::text,
        schedule::text
    );
END;
$$;

CREATE FUNCTION add_continuous_aggregate_distributed(
    name text,
    query text,
    refresh_start text,
    refresh_end text,
    schedule text
)
RETURNS text
LANGUAGE sql
STABLE
AS $$
    SELECT companion_add_continuous_aggregate_distributed_plan(
        name,
        query,
        refresh_start::interval,
        refresh_end::interval,
        schedule::interval
    )
$$;

CREATE FUNCTION apply_continuous_aggregate_distributed(
    name text,
    query text,
    refresh_start text,
    refresh_end text,
    schedule text
)
RETURNS void
LANGUAGE plpgsql
VOLATILE
AS $$
BEGIN
    IF btrim(name) = '' THEN
        RAISE EXCEPTION 'name must not be empty';
    END IF;
    IF btrim(query) = '' THEN
        RAISE EXCEPTION 'query must not be empty';
    END IF;

    PERFORM companion_internal.require_visible_function(
        'add_continuous_aggregate_policy',
        'timescaledb'
    );

    EXECUTE format(
        'CREATE MATERIALIZED VIEW %I WITH (timescaledb.continuous) AS %s WITH NO DATA',
        name,
        query
    );
    EXECUTE format(
        'SELECT add_continuous_aggregate_policy(%L, start_offset => %L::interval, end_offset => %L::interval, schedule_interval => %L::interval)',
        name,
        refresh_start,
        refresh_end,
        schedule
    );
    PERFORM companion_internal.add_continuous_aggregate_distributed(
        name,
        query,
        refresh_start::interval,
        refresh_end::interval,
        schedule::interval
    );
END;
$$;

CREATE FUNCTION companion_time_range_shard_pruner_plan(
    distributed_table regclass,
    time_column name
)
RETURNS text
LANGUAGE plpgsql
STABLE
AS $$
BEGIN
    RETURN format(
$plan$SELECT companion_internal.enable_time_range_shard_pruner(%1$L::regclass, %2$L);$plan$,
        distributed_table::text,
        time_column::text
    );
END;
$$;

CREATE FUNCTION time_range_shard_pruner(
    distributed_table text,
    time_column text
)
RETURNS text
LANGUAGE sql
STABLE
AS $$
    SELECT companion_time_range_shard_pruner_plan(
        distributed_table::regclass,
        time_column::name
    )
$$;

CREATE FUNCTION apply_time_range_shard_pruner(
    distributed_table text,
    time_column text
)
RETURNS void
LANGUAGE plpgsql
VOLATILE
AS $$
BEGIN
    PERFORM companion_internal.enable_time_range_shard_pruner(
        distributed_table::regclass,
        time_column::name
    );
END;
$$;

CREATE VIEW companion_timescale_bridge_state AS
SELECT
    bridge_id,
    feature_id,
    object_name,
    parameters,
    applied_at
FROM companion_internal.timescale_bridge_state;
-- FEATURE: C10
-- FEATURE: M2
-- FEATURE: M14
-- FEATURE: M15

-- F1-style two-version invariant (2VI) extensions. Adds:
--   * companion.schema_job_phase_log         (phase transition audit)
--   * companion.worker_schema_lease          (per-worker acknowledgement)
--   * companion.cluster_alarms               (2VI violation alarms)
--   * companion_internal.schema_job_phase_log_insert
--   * companion_internal.schema_job_phase_log_rollback
--   * companion_internal.worker_schema_lease_upsert
--   * companion_internal.worker_schema_lease_revoke
--   * companion_internal.schema_job_rollback_to
--   * companion_internal.schema_job_cleanup_backfill
--   * companion_internal.schema_job_drop_added_column
--   * companion_internal.verify_two_version_invariant
--   * companion_internal.raise_cluster_alarm
--   * companion_two_version_invariant_state (view)
--
-- All names are namespaced under companion_internal/companion. No upstream
-- Citus or TimescaleDB identifier is touched.

CREATE TABLE IF NOT EXISTS companion_internal.schema_job_phase_log (
    log_id bigserial PRIMARY KEY,
    job_name text NOT NULL
        REFERENCES companion_internal.schema_jobs(job_name)
        ON DELETE CASCADE,
    from_state text NOT NULL CHECK (
        from_state IN ('delete_only', 'write_only', 'backfill', 'public', 'paused', 'canceled')
    ),
    to_state text NOT NULL CHECK (
        to_state IN ('delete_only', 'write_only', 'backfill', 'public', 'paused', 'canceled')
    ),
    started_at timestamptz NOT NULL,
    completed_at timestamptz NOT NULL,
    workers_acknowledged text[] NOT NULL DEFAULT ARRAY[]::text[],
    gate text NOT NULL CHECK (
        gate IN ('wait_forever', 'skip_missing', 'rollback_on_timeout')
    ),
    is_rollback boolean NOT NULL DEFAULT false,
    recorded_at timestamptz NOT NULL DEFAULT now(),
    CHECK (completed_at >= started_at)
);

CREATE INDEX IF NOT EXISTS schema_job_phase_log_job_name_idx
    ON companion_internal.schema_job_phase_log(job_name, recorded_at);

CREATE TABLE IF NOT EXISTS companion_internal.worker_schema_lease (
    worker_id text NOT NULL,
    job_name text NOT NULL
        REFERENCES companion_internal.schema_jobs(job_name)
        ON DELETE CASCADE,
    schema_version_id text NOT NULL,
    phase text NOT NULL CHECK (
        phase IN ('delete_only', 'write_only', 'backfill', 'public', 'paused', 'canceled')
    ),
    expires_at timestamptz NOT NULL,
    updated_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (worker_id, job_name)
);

CREATE INDEX IF NOT EXISTS worker_schema_lease_job_idx
    ON companion_internal.worker_schema_lease(job_name, expires_at);

CREATE TABLE IF NOT EXISTS companion_internal.cluster_alarms (
    alarm_id bigserial PRIMARY KEY,
    alarm_kind text NOT NULL,
    severity text NOT NULL CHECK (severity IN ('info', 'warning', 'critical')),
    job_name text,
    detail jsonb NOT NULL DEFAULT '{}'::jsonb,
    raised_at timestamptz NOT NULL DEFAULT now(),
    cleared_at timestamptz
);

CREATE INDEX IF NOT EXISTS cluster_alarms_active_idx
    ON companion_internal.cluster_alarms(alarm_kind, raised_at)
    WHERE cleared_at IS NULL;

CREATE VIEW companion_schema_job_phase_log AS
SELECT
    log_id,
    job_name,
    from_state,
    to_state,
    started_at,
    completed_at,
    workers_acknowledged,
    gate,
    is_rollback,
    recorded_at
FROM companion_internal.schema_job_phase_log;

CREATE VIEW companion_worker_schema_lease AS
SELECT
    worker_id,
    job_name,
    schema_version_id,
    phase,
    expires_at,
    updated_at
FROM companion_internal.worker_schema_lease;

CREATE VIEW companion_cluster_alarms AS
SELECT
    alarm_id,
    alarm_kind,
    severity,
    job_name,
    detail,
    raised_at,
    cleared_at
FROM companion_internal.cluster_alarms;

CREATE FUNCTION companion_internal.schema_job_phase_log_insert(
    p_job_name text,
    p_from_state text,
    p_to_state text,
    p_started_at timestamptz,
    p_completed_at timestamptz,
    p_workers_acknowledged text[],
    p_gate text
)
RETURNS bigint
LANGUAGE plpgsql
VOLATILE
AS $$
DECLARE
    inserted_id bigint;
BEGIN
    IF p_job_name IS NULL OR btrim(p_job_name) = '' THEN
        RAISE EXCEPTION 'job_name must not be empty';
    END IF;
    IF p_workers_acknowledged IS NULL THEN
        RAISE EXCEPTION 'workers_acknowledged must not be null';
    END IF;

    SELECT log_id
    INTO inserted_id
    FROM companion_internal.schema_job_phase_log
    WHERE job_name = btrim(p_job_name)
      AND from_state = lower(btrim(p_from_state))
      AND to_state = lower(btrim(p_to_state))
      AND started_at = p_started_at
      AND completed_at = p_completed_at
      AND workers_acknowledged = p_workers_acknowledged
      AND gate = lower(btrim(p_gate))
      AND is_rollback = false
    ORDER BY log_id
    LIMIT 1;

    IF FOUND THEN
        RETURN inserted_id;
    END IF;

    IF NOT EXISTS (
        SELECT 1
        FROM companion_internal.schema_jobs
        WHERE job_name = btrim(p_job_name)
    ) THEN
        RAISE EXCEPTION 'schema job is not registered: %', p_job_name;
    END IF;

    INSERT INTO companion_internal.schema_job_phase_log(
        job_name, from_state, to_state, started_at, completed_at,
        workers_acknowledged, gate, is_rollback
    )
    VALUES (
        btrim(p_job_name),
        lower(btrim(p_from_state)),
        lower(btrim(p_to_state)),
        p_started_at,
        p_completed_at,
        p_workers_acknowledged,
        lower(btrim(p_gate)),
        false
    )
    RETURNING log_id INTO inserted_id;

    RETURN inserted_id;
END;
$$;

CREATE FUNCTION companion_internal.schema_job_phase_log_rollback(
    p_job_name text,
    p_from_state text,
    p_to_state text,
    p_recorded_at timestamptz
)
RETURNS bigint
LANGUAGE plpgsql
VOLATILE
AS $$
DECLARE
    inserted_id bigint;
BEGIN
    IF p_job_name IS NULL OR btrim(p_job_name) = '' THEN
        RAISE EXCEPTION 'job_name must not be empty';
    END IF;

    INSERT INTO companion_internal.schema_job_phase_log(
        job_name, from_state, to_state, started_at, completed_at,
        workers_acknowledged, gate, is_rollback
    )
    VALUES (
        btrim(p_job_name),
        lower(btrim(p_from_state)),
        lower(btrim(p_to_state)),
        p_recorded_at,
        p_recorded_at,
        ARRAY[]::text[],
        'rollback_on_timeout',
        true
    )
    RETURNING log_id INTO inserted_id;

    RETURN inserted_id;
END;
$$;

CREATE FUNCTION companion_internal.worker_schema_lease_upsert(
    p_worker_id text,
    p_job_name text,
    p_schema_version_id text,
    p_phase text,
    p_expires_at timestamptz
)
RETURNS void
LANGUAGE plpgsql
VOLATILE
AS $$
BEGIN
    IF p_worker_id IS NULL OR btrim(p_worker_id) = '' THEN
        RAISE EXCEPTION 'worker_id must not be empty';
    END IF;
    IF p_job_name IS NULL OR btrim(p_job_name) = '' THEN
        RAISE EXCEPTION 'job_name must not be empty';
    END IF;
    IF p_schema_version_id IS NULL OR btrim(p_schema_version_id) = '' THEN
        RAISE EXCEPTION 'schema_version_id must not be empty';
    END IF;
    IF lower(btrim(p_phase)) NOT IN ('delete_only', 'write_only', 'backfill', 'public', 'paused', 'canceled') THEN
        RAISE EXCEPTION 'unsupported phase: %', p_phase;
    END IF;
    IF p_expires_at IS NULL THEN
        RAISE EXCEPTION 'expires_at must not be null';
    END IF;
    IF NOT EXISTS (
        SELECT 1
        FROM companion_internal.schema_jobs
        WHERE job_name = btrim(p_job_name)
    ) THEN
        RAISE EXCEPTION 'schema job is not registered: %', p_job_name;
    END IF;

    INSERT INTO companion_internal.worker_schema_lease(
        worker_id, job_name, schema_version_id, phase, expires_at, updated_at
    )
    VALUES (
        btrim(p_worker_id),
        btrim(p_job_name),
        btrim(p_schema_version_id),
        lower(btrim(p_phase)),
        p_expires_at,
        now()
    )
    ON CONFLICT (worker_id, job_name) DO UPDATE
    SET schema_version_id = EXCLUDED.schema_version_id,
        phase = EXCLUDED.phase,
        expires_at = EXCLUDED.expires_at,
        updated_at = now();
END;
$$;

CREATE FUNCTION companion_internal.worker_schema_lease_revoke(
    p_worker_id text,
    p_job_name text
)
RETURNS void
LANGUAGE plpgsql
VOLATILE
AS $$
BEGIN
    IF p_worker_id IS NULL OR btrim(p_worker_id) = '' THEN
        RAISE EXCEPTION 'worker_id must not be empty';
    END IF;
    IF p_job_name IS NULL OR btrim(p_job_name) = '' THEN
        RAISE EXCEPTION 'job_name must not be empty';
    END IF;

    DELETE FROM companion_internal.worker_schema_lease
    WHERE worker_id = btrim(p_worker_id)
      AND job_name = btrim(p_job_name);
END;
$$;

CREATE FUNCTION companion_internal.schema_job_rollback_to(
    p_target_state text
)
RETURNS text
LANGUAGE plpgsql
VOLATILE
AS $$
DECLARE
    normalized text;
BEGIN
    normalized := lower(btrim(p_target_state));
    IF normalized NOT IN ('delete_only', 'write_only', 'backfill', 'public', 'paused', 'canceled') THEN
        RAISE EXCEPTION 'unsupported rollback target: %', p_target_state;
    END IF;
    RETURN normalized;
END;
$$;

CREATE FUNCTION companion_internal.schema_job_cleanup_backfill(
    p_table text,
    p_column text
)
RETURNS bigint
LANGUAGE plpgsql
VOLATILE
AS $$
DECLARE
    qualified regclass;
    rows_cleaned bigint := 0;
    stmt text;
BEGIN
    IF p_table IS NULL OR btrim(p_table) = '' THEN
        RAISE EXCEPTION 'table must not be empty';
    END IF;
    IF p_column IS NULL OR btrim(p_column) = '' THEN
        RAISE EXCEPTION 'column must not be empty';
    END IF;
    qualified := btrim(p_table)::regclass;
    stmt := format('UPDATE %s SET %I = NULL WHERE %I IS NOT NULL', qualified, btrim(p_column), btrim(p_column));
    EXECUTE stmt;
    GET DIAGNOSTICS rows_cleaned = ROW_COUNT;
    RETURN rows_cleaned;
END;
$$;

CREATE FUNCTION companion_internal.schema_job_drop_added_column(
    p_table text,
    p_column text
)
RETURNS void
LANGUAGE plpgsql
VOLATILE
AS $$
DECLARE
    qualified regclass;
BEGIN
    IF p_table IS NULL OR btrim(p_table) = '' THEN
        RAISE EXCEPTION 'table must not be empty';
    END IF;
    IF p_column IS NULL OR btrim(p_column) = '' THEN
        RAISE EXCEPTION 'column must not be empty';
    END IF;
    qualified := btrim(p_table)::regclass;
    EXECUTE format('ALTER TABLE %s DROP COLUMN IF EXISTS %I', qualified, btrim(p_column));
END;
$$;

CREATE FUNCTION companion_internal.raise_cluster_alarm(
    p_alarm_kind text,
    p_severity text,
    p_job_name text,
    p_detail jsonb
)
RETURNS bigint
LANGUAGE plpgsql
VOLATILE
AS $$
DECLARE
    inserted_alarm_id bigint;
BEGIN
    IF p_alarm_kind IS NULL OR btrim(p_alarm_kind) = '' THEN
        RAISE EXCEPTION 'alarm_kind must not be empty';
    END IF;
    IF lower(btrim(p_severity)) NOT IN ('info', 'warning', 'critical') THEN
        RAISE EXCEPTION 'unsupported severity: %', p_severity;
    END IF;

    INSERT INTO companion_internal.cluster_alarms(
        alarm_kind, severity, job_name, detail
    )
    VALUES (
        btrim(p_alarm_kind),
        lower(btrim(p_severity)),
        NULLIF(btrim(COALESCE(p_job_name, '')), ''),
        COALESCE(p_detail, '{}'::jsonb)
    )
    RETURNING cluster_alarms.alarm_id INTO inserted_alarm_id;

    RETURN inserted_alarm_id;
END;
$$;

-- Returns a JSON report of the F1 two-version invariant. The companion sets
-- a cluster_alarms row when more than two distinct active schema versions are
-- observed in flight. Continuous monitor calls this via pg_cron.
CREATE FUNCTION companion_internal.verify_two_version_invariant()
RETURNS jsonb
LANGUAGE plpgsql
VOLATILE
AS $$
DECLARE
    job_summary jsonb;
    violators jsonb;
    violation_count integer;
    inflight_versions integer;
BEGIN
    SELECT count(DISTINCT schema_version_id)
    INTO inflight_versions
    FROM companion_internal.worker_schema_lease
    WHERE expires_at > now()
      AND phase IN ('delete_only', 'write_only', 'backfill', 'public');

    SELECT COALESCE(jsonb_agg(row_to_json(t)), '[]'::jsonb)
    INTO violators
    FROM (
        SELECT job_name, count(DISTINCT schema_version_id) AS distinct_versions
        FROM companion_internal.worker_schema_lease
        WHERE expires_at > now()
          AND phase IN ('delete_only', 'write_only', 'backfill', 'public')
        GROUP BY job_name
        HAVING count(DISTINCT schema_version_id) > 2
    ) AS t;

    violation_count := jsonb_array_length(violators);

    job_summary := jsonb_build_object(
        'checked_at', now(),
        'inflight_versions', inflight_versions,
        'violations', violators,
        'violation_count', violation_count,
        'invariant_max_versions', 2
    );

    IF violation_count > 0 THEN
        PERFORM companion_internal.raise_cluster_alarm(
            'two_version_invariant_violation',
            'critical',
            NULL,
            job_summary
        );
    END IF;

    RETURN job_summary;
END;
$$;

CREATE VIEW companion_two_version_invariant_state AS
SELECT
    job_name,
    count(DISTINCT schema_version_id) AS distinct_versions,
    array_agg(DISTINCT schema_version_id ORDER BY schema_version_id) AS schema_versions,
    max(expires_at) AS latest_lease_expiry
FROM companion_internal.worker_schema_lease
WHERE expires_at > now()
GROUP BY job_name;


-- FEATURE: T5

CREATE FUNCTION companion_internal.validate_txn_intents(p_intents jsonb)
RETURNS void
LANGUAGE plpgsql
IMMUTABLE
AS $$
DECLARE
    intent jsonb;
BEGIN
    IF p_intents IS NULL OR jsonb_typeof(p_intents) <> 'array' THEN
        RAISE EXCEPTION 'intents must be a jsonb array';
    END IF;
    IF jsonb_array_length(p_intents) = 0 THEN
        RAISE EXCEPTION 'intents must not be empty';
    END IF;
    FOR intent IN SELECT value FROM jsonb_array_elements(p_intents) LOOP
        IF COALESCE((intent->>'shard_id')::bigint, 0) <= 0 THEN
            RAISE EXCEPTION 'intent shard_id must be greater than zero';
        END IF;
        IF btrim(COALESCE(intent->>'key_range', '')) = '' THEN
            RAISE EXCEPTION 'intent key_range must not be empty';
        END IF;
        IF COALESCE((intent->>'required_acks')::integer, 0) <= 0 THEN
            RAISE EXCEPTION 'intent required_acks must be greater than zero';
        END IF;
        IF COALESCE((intent->>'replica_acks')::integer, 0) < 0 THEN
            RAISE EXCEPTION 'intent replica_acks must be non-negative';
        END IF;
    END LOOP;
END;
$$;

CREATE FUNCTION companion.txn_stage(
    p_txn_id text,
    p_coordinator text,
    p_staging_physical_ms bigint,
    p_intents jsonb
)
RETURNS jsonb
LANGUAGE plpgsql
VOLATILE
AS $$
DECLARE
    inserted companion_internal.txn_status_records%ROWTYPE;
BEGIN
    IF p_txn_id IS NULL OR btrim(p_txn_id) = '' THEN
        RAISE EXCEPTION 'txn_id must not be empty';
    END IF;
    IF p_coordinator IS NULL OR btrim(p_coordinator) = '' THEN
        RAISE EXCEPTION 'coordinator must not be empty';
    END IF;
    IF p_staging_physical_ms IS NULL OR p_staging_physical_ms <= 0 THEN
        RAISE EXCEPTION 'staging_physical_ms must be greater than zero';
    END IF;
    PERFORM companion_internal.validate_txn_intents(p_intents);
    IF EXISTS (
        SELECT 1 FROM companion_internal.txn_status_records
        WHERE txn_id = btrim(p_txn_id)
    ) THEN
        RAISE EXCEPTION 'txn_id already staged: %', p_txn_id;
    END IF;

    INSERT INTO companion_internal.txn_status_records(
        txn_id, coordinator, status, staging_physical_ms, intents
    )
    VALUES (
        btrim(p_txn_id), btrim(p_coordinator), 'staging', p_staging_physical_ms, p_intents
    )
    RETURNING * INTO inserted;

    RETURN jsonb_build_object(
        'txn_id', inserted.txn_id,
        'coordinator', inserted.coordinator,
        'status', inserted.status,
        'raft_index', inserted.raft_index,
        'intent_count', jsonb_array_length(inserted.intents)
    );
END;
$$;

CREATE FUNCTION companion.txn_finalize(
    p_txn_id text,
    p_observed_physical_ms bigint
)
RETURNS jsonb
LANGUAGE plpgsql
VOLATILE
AS $$
DECLARE
    record companion_internal.txn_status_records%ROWTYPE;
    max_staging_ms bigint := 5000;
    all_evidence boolean;
    decision text;
    next_status text;
BEGIN
    IF p_txn_id IS NULL OR btrim(p_txn_id) = '' THEN
        RAISE EXCEPTION 'txn_id must not be empty';
    END IF;
    IF p_observed_physical_ms IS NULL OR p_observed_physical_ms <= 0 THEN
        RAISE EXCEPTION 'observed_physical_ms must be greater than zero';
    END IF;

    SELECT * INTO record
    FROM companion_internal.txn_status_records
    WHERE txn_id = btrim(p_txn_id)
    FOR UPDATE;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'unknown txn_id: %', p_txn_id;
    END IF;

    IF record.status = 'committed' THEN
        decision := 'already_committed';
        next_status := 'committed';
    ELSIF record.status = 'aborted' THEN
        decision := 'already_aborted';
        next_status := 'aborted';
    ELSIF p_observed_physical_ms > record.staging_physical_ms + max_staging_ms THEN
        decision := 'abort_stale_staging_record';
        next_status := 'aborted';
    ELSE
        SELECT bool_and(
            COALESCE((intent->>'replica_acks')::integer, 0)
              >= COALESCE((intent->>'required_acks')::integer, 0)
        )
        INTO all_evidence
        FROM jsonb_array_elements(record.intents) AS intent;
        IF all_evidence THEN
            decision := 'commit';
            next_status := 'committed';
        ELSE
            decision := 'wait_for_replication_evidence';
            next_status := 'staging';
        END IF;
    END IF;

    UPDATE companion_internal.txn_status_records
    SET status = next_status,
        observed_physical_ms = p_observed_physical_ms,
        updated_at = now()
    WHERE txn_id = record.txn_id
    RETURNING * INTO record;

    RETURN jsonb_build_object(
        'txn_id', record.txn_id,
        'decision', decision,
        'status', record.status,
        'raft_index', record.raft_index,
        'observed_physical_ms', record.observed_physical_ms
    );
END;
$$;

CREATE FUNCTION companion_txn_stage(
    p_txn_id text,
    p_coordinator text,
    p_staging_physical_ms bigint,
    p_intents jsonb
)
RETURNS jsonb
LANGUAGE sql
VOLATILE
AS $$
    SELECT companion.txn_stage($1, $2, $3, $4)
$$;

CREATE FUNCTION companion_txn_finalize(
    p_txn_id text,
    p_observed_physical_ms bigint
)
RETURNS jsonb
LANGUAGE sql
VOLATILE
AS $$
    SELECT companion.txn_finalize($1, $2)
$$;
