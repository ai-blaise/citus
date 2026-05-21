-- FEATURE: TS18

CREATE SCHEMA IF NOT EXISTS companion_internal;

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
        ('TS9', 'doctor rules for cohabitation', 'sql-plan'),
        ('TS12', 'distributed reorder policy', 'sql-runtime'),
        ('TS18', 'executable Timescale bridge state', 'sql-runtime'),
        ('TS13', 'distributed time_bucket_gapfill', 'sql-plan'),
        ('TS14', 'distributed metric toolkit aggregates', 'sql-plan'),
        ('TS15', 'distributed approximate toolkit aggregates', 'sql-plan'),
        ('TS16', 'distributed downsampler toolkit aggregates', 'sql-plan'),
        ('TS17', 'distributed state toolkit aggregates', 'sql-plan'),
        ('A1', 'pgai-compatible vectorizer DSL', 'sql-plan'),
        ('Search2', 'distributed BM25 search index', 'sql-plan'),
        ('Search3', 'hybrid BM25 and vector ranking', 'sql-plan'),
        ('Search9', 'reranker UDF plan', 'sql-plan'),
        ('G2', 'distributed graph bridge', 'sql-plan'),
        ('G3', 'graph colocation policy', 'sql-plan'),
        ('API4', 'GraphQL distributed graph metadata', 'sql-plan'),
        ('JS2', 'distributed JSON Schema validation', 'sql-plan'),
        ('M13', 'JSON Schema validation triggers', 'sql-plan'),
        ('Geo2', 'geo-aware distribution', 'sql-plan'),
        ('Geo3', 'geo shard pruning', 'sql-plan'),
        ('T8', 'toolkit two-step aggregate pushdown', 'sql-plan'),
        ('L9', 'worker partial aggregate pushdown', 'sql-plan'),
        ('M7', 'pre-flight cohabit-extension check', 'sql-plan'),
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
        ('C10', 'online schema job state machine', 'runtime-contract'),
        ('M2', 'gh-ost-style online DDL', 'runtime-contract'),
        ('S14', 'tenant migration online', 'runtime-contract'),
        ('TO3', 'tenant migration online', 'runtime-contract'),
        ('TO4', 'tenant archive', 'runtime-contract'),
        ('TO5', 'tenant region affinity', 'runtime-contract'),
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
        ('T7', 'pipelined client protocol in pool', 'ops-contract')
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
    ON CONFLICT (migration_name) DO UPDATE
    SET table_name = EXCLUDED.table_name,
        lock_timeout_ms = EXCLUDED.lock_timeout_ms,
        backfill_batch_size = EXCLUDED.backfill_batch_size,
        status = 'running',
        started_at = now(),
        completed_at = NULL;

    PERFORM set_config('ai_blaise.current_migration_name', btrim(p_migration_name), true);
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
    migration_name text := companion_internal.current_migration_name();
BEGIN
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
        migration_name,
        p_operation_type,
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
BEGIN
    IF p_migration_name IS NULL OR btrim(p_migration_name) = '' THEN
        RAISE EXCEPTION 'migration_name must not be empty';
    END IF;

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
