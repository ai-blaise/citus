CREATE SCHEMA IF NOT EXISTS companion_internal;

CREATE FUNCTION companion_feature_status()
RETURNS TABLE(feature_id text, feature_name text, status text)
LANGUAGE sql
STABLE
PARALLEL SAFE
AS $$
    VALUES
        ('TS1', 'distributed hypertable bridge', 'sql-plan'),
        ('TS2', 'distributed compression policy', 'sql-plan'),
        ('TS3', 'distributed continuous aggregates', 'sql-plan'),
        ('TS4', 'distributed retention policy', 'sql-plan'),
        ('TS5', 'time-range shard pruner', 'sql-plan'),
        ('TS8', 'LSP hypertable invariants', 'sql-plan'),
        ('TS9', 'doctor rules for cohabitation', 'sql-plan'),
        ('TS12', 'distributed reorder policy', 'sql-plan'),
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
        ('O2', 'distributed stats view', 'sql-runtime'),
        ('O3', 'replication lag view', 'sql-runtime'),
        ('R4', 'idle transaction reaper', 'sql-runtime'),
        ('Auth2', 'tenant-aware claims', 'runtime-contract'),
        ('Sec1', 'RLS helpers', 'runtime-contract'),
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
        ('D10', 'production hardening runbook', 'ops-contract'),
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
%2$s;
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
