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
        ('PM3', 'plan freeze companion module', 'sql-plan'),
        ('PM4', 'plan regression detection', 'sql-plan'),
        ('IA3', 'companion index advisor', 'sql-plan'),
        ('Sec5', 'immutable ledger', 'sql-plan'),
        ('Sec6', 'ledger HMAC tamper evidence', 'sql-plan'),
        ('M1', 'pgroll-style expand-contract migrations', 'sql-plan'),
        ('M11', 'online column-type migration', 'sql-plan'),
        ('WH2', 'companion webhook helpers', 'sql-plan'),
        ('O1', 'query percentile views', 'sql-runtime'),
        ('O2', 'local activity stats view', 'sql-runtime'),
        ('O3', 'replication lag view', 'sql-runtime'),
        ('R4', 'idle transaction detector', 'sql-runtime'),
        ('Auth2', 'tenant-aware claims', 'sql-runtime'),
        ('Sec1', 'RLS helpers', 'sql-runtime'),
        ('Sec2', 'JWT verification UDF', 'runtime-contract'),
        ('S6', 'placement generation helpers', 'runtime-contract'),
        ('S13', 'range routing helpers', 'runtime-contract'),
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
