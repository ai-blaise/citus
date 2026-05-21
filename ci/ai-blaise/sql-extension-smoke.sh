#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
extension_dir="${repo_root}/images/citus-pg-overlay/extensions"
control_file="${extension_dir}/ai_blaise_citus.control"
sql_file="${extension_dir}/ai_blaise_citus--0.1.0.sql"
postgres_image="${SQL_EXTENSION_SMOKE_IMAGE:-postgres:17}"
require_docker="${REQUIRE_DOCKER:-0}"

for file in "${control_file}" "${sql_file}"; do
  if [[ ! -s "${file}" ]]; then
    echo "missing SQL extension smoke artifact: ${file}" >&2
    exit 1
  fi
done

if ! command -v docker >/dev/null 2>&1; then
  if [[ "${require_docker}" == "1" ]]; then
    echo "docker is required for SQL extension smoke" >&2
    exit 1
  fi
  echo "docker unavailable; skipping SQL extension smoke"
  exit 0
fi

container="ai-blaise-sql-extension-smoke-${RANDOM}-$$"
cleanup() {
  docker rm -f "${container}" >/dev/null 2>&1 || true
}
trap cleanup EXIT

docker run \
  --name "${container}" \
  -e POSTGRES_PASSWORD=postgres \
  -v "${control_file}:/usr/share/postgresql/17/extension/ai_blaise_citus.control:ro" \
  -v "${sql_file}:/usr/share/postgresql/17/extension/ai_blaise_citus--0.1.0.sql:ro" \
  -d "${postgres_image}" \
  -c shared_preload_libraries=pg_stat_statements >/dev/null

init_complete=0
for _ in $(seq 1 120); do
  if docker logs "${container}" 2>&1 | grep -q "PostgreSQL init process complete"; then
    init_complete=1
    break
  fi
  sleep 1
done

if [[ "${init_complete}" != "1" ]]; then
  docker logs "${container}" >&2 || true
  echo "postgres container did not finish init scripts" >&2
  exit 1
fi

ready=0
for _ in $(seq 1 60); do
  if docker exec "${container}" psql -U postgres -Atqc 'SELECT 1' >/dev/null 2>&1; then
    ready=1
    break
  fi
  sleep 1
done

if [[ "${ready}" != "1" ]]; then
  docker logs "${container}" >&2 || true
  echo "postgres container did not become ready" >&2
  exit 1
fi

docker exec -i "${container}" psql -U postgres -v ON_ERROR_STOP=1 <<'SQL'
CREATE EXTENSION pg_stat_statements;
CREATE EXTENSION pgcrypto;
SELECT pg_stat_statements_reset();
SELECT 1 AS ai_blaise_pg_stat_statements_seed;
CREATE EXTENSION ai_blaise_citus;
CREATE TABLE timescale_smoke_metrics (
  metric_time timestamptz NOT NULL,
  value double precision NOT NULL
);
CREATE INDEX timescale_smoke_metrics_metric_time_idx
ON timescale_smoke_metrics(metric_time);

CREATE TABLE timescale_bridge_call_log (
  function_name text NOT NULL,
  relation_name text,
  argument_summary jsonb NOT NULL DEFAULT '{}'::jsonb,
  called_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE control_plane_smoke_orders (
  order_id bigserial PRIMARY KEY,
  tenant_id text NOT NULL,
  amount_cents integer NOT NULL,
  created_at timestamptz NOT NULL DEFAULT now(),
  metadata jsonb NOT NULL DEFAULT '{}'::jsonb
);

CREATE FUNCTION create_hypertable(
  table_name regclass,
  time_column text,
  chunk_time_interval interval DEFAULT NULL,
  if_not_exists boolean DEFAULT false
)
RETURNS void
LANGUAGE plpgsql
AS $$
BEGIN
  INSERT INTO timescale_bridge_call_log(function_name, relation_name, argument_summary)
  VALUES (
    'create_hypertable',
    table_name::text,
    jsonb_build_object(
      'time_column', time_column,
      'chunk_time_interval', chunk_time_interval::text,
      'if_not_exists', if_not_exists
    )
  );
END;
$$;

CREATE FUNCTION create_distributed_table(
  table_name regclass,
  distribution_column text,
  shard_count integer DEFAULT 32
)
RETURNS void
LANGUAGE plpgsql
AS $$
BEGIN
  INSERT INTO timescale_bridge_call_log(function_name, relation_name, argument_summary)
  VALUES (
    'create_distributed_table',
    table_name::text,
    jsonb_build_object(
      'distribution_column', distribution_column,
      'shard_count', shard_count
    )
  );
END;
$$;

CREATE FUNCTION add_retention_policy(
  table_name regclass,
  drop_after interval
)
RETURNS void
LANGUAGE plpgsql
AS $$
BEGIN
  INSERT INTO timescale_bridge_call_log(function_name, relation_name, argument_summary)
  VALUES (
    'add_retention_policy',
    table_name::text,
    jsonb_build_object('drop_after', drop_after::text)
  );
END;
$$;

CREATE FUNCTION add_reorder_policy(
  table_name regclass,
  index_name text
)
RETURNS void
LANGUAGE plpgsql
AS $$
BEGIN
  INSERT INTO timescale_bridge_call_log(function_name, relation_name, argument_summary)
  VALUES (
    'add_reorder_policy',
    table_name::text,
    jsonb_build_object('index_name', index_name)
  );
END;
$$;

DO $$
DECLARE
  status_count integer;
  planned_count integer;
  plan_sql text;
  bridge_features integer;
  jwt_header_segment text;
  jwt_payload_segment text;
  jwt_signing_input text;
  jwt_token text;
  expired_payload_segment text;
  expired_token text;
  missing_tenant_payload_segment text;
  missing_tenant_token text;
  jwt_claims record;
  generation_one bigint;
  generation_two bigint;
  hash_index integer;
  hash_index_again integer;
  range_index integer;
  plan_violation boolean;
  migration_sql text;
  advisor_sql text;
  webhook_trigger_sql text;
  webhook_event_count integer;
BEGIN
  SELECT count(*) INTO status_count FROM companion_feature_status();
  IF status_count < 60 THEN
    RAISE EXCEPTION 'expected at least 60 companion feature rows, got %', status_count;
  END IF;

  SELECT count(*) INTO planned_count
  FROM companion_feature_status()
  WHERE status = 'planned';
  IF planned_count <> 0 THEN
    RAISE EXCEPTION 'companion_feature_status returned % planned rows', planned_count;
  END IF;

  IF NOT EXISTS (SELECT 1 FROM companion_feature_status() WHERE feature_id = 'TS1') THEN
    RAISE EXCEPTION 'companion_feature_status must include TS1';
  END IF;
  IF NOT EXISTS (SELECT 1 FROM companion_feature_status() WHERE feature_id = 'TS5') THEN
    RAISE EXCEPTION 'companion_feature_status must include TS5';
  END IF;
  IF NOT EXISTS (
    SELECT 1
    FROM companion_feature_status()
    WHERE feature_id = 'TS18'
      AND status = 'sql-runtime'
  ) THEN
    RAISE EXCEPTION 'companion_feature_status must include sql-runtime TS18';
  END IF;
  IF (
    SELECT count(*)
    FROM companion_feature_status()
    WHERE feature_id IN ('Auth2', 'Sec1', 'Sec2', 'Sec5', 'Sec6', 'S6', 'S13', 'PM3', 'PM4', 'M1', 'M11', 'IA3', 'WH2', 'O1', 'O2', 'O3', 'R4')
      AND status = 'sql-runtime'
  ) <> 17 THEN
    RAISE EXCEPTION 'companion_feature_status must mark Auth2, Sec1, Sec2, Sec5, Sec6, S6, S13, PM3, PM4, M1, M11, IA3, WH2, and observability features as sql-runtime';
  END IF;

  PERFORM companion_internal.migrate_start(
    'orders-expand-contract',
    'control_plane_smoke_orders',
    5000,
    1000
  );
  migration_sql := companion_internal.migration_add_column(
    'region',
    'text',
    '''us-east1'''
  );
  IF migration_sql NOT LIKE 'ALTER TABLE control_plane_smoke_orders ADD COLUMN IF NOT EXISTS region text DEFAULT %' THEN
    RAISE EXCEPTION 'M1 migration_add_column did not render bounded expand DDL: %', migration_sql;
  END IF;
  migration_sql := companion_internal.migration_online_type_change(
    'amount_cents',
    'integer',
    'bigint',
    'amount_cents::bigint'
  );
  IF migration_sql NOT LIKE '%amount_cents__ai_blaise_new bigint%' THEN
    RAISE EXCEPTION 'M11 online type-change helper did not render shadow-column DDL: %', migration_sql;
  END IF;
  PERFORM companion_internal.migrate_complete('orders-expand-contract');
  IF NOT EXISTS (
    SELECT 1
    FROM companion_migration_runs
    WHERE migration_name = 'orders-expand-contract'
      AND table_name = 'control_plane_smoke_orders'
      AND status = 'completed'
  ) THEN
    RAISE EXCEPTION 'M1 migration run was not completed and visible';
  END IF;
  IF (
    SELECT count(*)
    FROM companion_migration_operations
    WHERE migration_name = 'orders-expand-contract'
  ) <> 2 THEN
    RAISE EXCEPTION 'M1/M11 migration operations were not recorded';
  END IF;
  BEGIN
    PERFORM companion_internal.migration_drop_column('orphan_column');
    RAISE EXCEPTION 'M1 migration operation ran without migrate_start';
  EXCEPTION
    WHEN raise_exception THEN
      IF SQLERRM <> 'no active companion migration; call companion_internal.migrate_start first' THEN
        RAISE;
      END IF;
  END;
  PERFORM companion_internal.migrate_start(
    'orders-bad-type-change',
    'control_plane_smoke_orders',
    5000,
    1000
  );
  BEGIN
    PERFORM companion_internal.migration_online_type_change(
      'amount_cents',
      'integer',
      'integer',
      'amount_cents'
    );
    RAISE EXCEPTION 'M11 online type-change accepted identical types';
  EXCEPTION
    WHEN raise_exception THEN
      IF SQLERRM <> 'from_type and to_type must differ' THEN
        RAISE;
      END IF;
  END;

  PERFORM companion_internal.index_advisor_record_candidate(
    '15 minutes',
    'control_plane_smoke_orders',
    'control_plane_orders_tenant_created_idx',
    ARRAY['tenant_id', 'created_at'],
    'btree',
    1000,
    700,
    12
  );
  SELECT create_index_sql
  INTO advisor_sql
  FROM companion_index_advisor_ranked(10)
  WHERE index_name = 'control_plane_orders_tenant_created_idx'::name;
  IF advisor_sql NOT LIKE 'CREATE INDEX CONCURRENTLY IF NOT EXISTS control_plane_orders_tenant_created_idx ON control_plane_smoke_orders USING btree %' THEN
    RAISE EXCEPTION 'IA3 ranked advisor did not render CREATE INDEX CONCURRENTLY SQL: %', advisor_sql;
  END IF;
  BEGIN
    PERFORM companion_internal.index_advisor_record_candidate(
      '15 minutes',
      'control_plane_smoke_orders',
      'control_plane_orders_bad_idx',
      ARRAY['tenant_id'],
      'btree',
      1000,
      1200,
      1
    );
    RAISE EXCEPTION 'IA3 accepted a non-improving candidate';
  EXCEPTION
    WHEN raise_exception THEN
      IF SQLERRM <> 'estimated_cost_after must be lower than estimated_cost_before' THEN
        RAISE;
      END IF;
  END;

  PERFORM companion_internal.webhook_register(
    'orders-webhook',
    'control_plane_smoke_orders',
    'https://hooks.example.test/orders',
    '{"Authorization": "secret://webhooks/orders"}'::jsonb,
    5
  );
  webhook_trigger_sql := companion_internal.install_webhook_trigger(
    'control_plane_smoke_orders',
    ARRAY['INSERT', 'UPDATE'],
    'companion.webhook_queue',
    'orders-webhook'
  );
  IF webhook_trigger_sql NOT LIKE 'CREATE TRIGGER companion_webhook_% AFTER INSERT OR UPDATE ON control_plane_smoke_orders%' THEN
    RAISE EXCEPTION 'WH2 install_webhook_trigger did not render/install trigger SQL: %', webhook_trigger_sql;
  END IF;
  INSERT INTO control_plane_smoke_orders(tenant_id, amount_cents, metadata)
  VALUES ('tenant-a', 100, '{"source":"insert"}'::jsonb);
  UPDATE control_plane_smoke_orders
  SET amount_cents = 125
  WHERE tenant_id = 'tenant-a';
  SELECT count(*)
  INTO webhook_event_count
  FROM companion_webhook_events
  WHERE webhook_name = 'orders-webhook'
    AND queue_name = 'companion.webhook_queue';
  IF webhook_event_count <> 2 THEN
    RAISE EXCEPTION 'WH2 webhook trigger did not enqueue INSERT and UPDATE rows';
  END IF;
  BEGIN
    PERFORM companion_internal.webhook_register(
      'bad-webhook',
      'control_plane_smoke_orders',
      'secret://orders',
      '{}'::jsonb,
      1
    );
    RAISE EXCEPTION 'WH2 accepted a non-http webhook URL';
  EXCEPTION
    WHEN raise_exception THEN
      IF SQLERRM <> 'url must be http or https' THEN
        RAISE;
      END IF;
  END;

  PERFORM companion_internal.plan_freeze('query-hash-1', '<Plan><Node /></Plan>', 'orders_hint');
  PERFORM companion_internal.plan_auto_promote('query-hash-1', 100, 7);
  PERFORM companion_internal.plan_regression_guard('query-hash-1', 10, 20);
  IF NOT EXISTS (
    SELECT 1
    FROM companion_plan_freezes
    WHERE query_hash = 'query-hash-1'
      AND hint_set_name = 'orders_hint'
      AND min_executions = 100
      AND stable_days = 7
      AND max_latency_regression_percent = 10
      AND max_cost_regression_percent = 20
  ) THEN
    RAISE EXCEPTION 'PM3 plan freeze state was not visible with policy metadata';
  END IF;
  plan_violation := companion_plan_regression_violates(
    'query-hash-1',
    100,
    112,
    1000,
    1000
  );
  IF NOT plan_violation THEN
    RAISE EXCEPTION 'PM4 regression guard did not flag latency regression';
  END IF;
  plan_violation := companion_plan_regression_violates(
    'query-hash-1',
    100,
    105,
    1000,
    1100
  );
  IF plan_violation THEN
    RAISE EXCEPTION 'PM4 regression guard flagged an allowed candidate';
  END IF;
  IF (
    SELECT count(*)
    FROM companion_internal.plan_regression_samples
    WHERE query_hash = 'query-hash-1'
  ) <> 2 THEN
    RAISE EXCEPTION 'PM4 regression samples were not recorded';
  END IF;
  BEGIN
    PERFORM companion_internal.plan_freeze('', '<Plan />', 'orders_hint');
    RAISE EXCEPTION 'PM3 plan_freeze accepted an empty query hash';
  EXCEPTION
    WHEN raise_exception THEN
      IF SQLERRM <> 'query_hash must not be empty' THEN
        RAISE;
      END IF;
  END;
  BEGIN
    PERFORM companion_internal.plan_regression_guard('missing-query-hash', 10, 20);
    RAISE EXCEPTION 'PM4 regression guard accepted an unknown frozen plan';
  EXCEPTION
    WHEN raise_exception THEN
      IF SQLERRM <> 'query_hash does not reference a frozen plan' THEN
        RAISE;
      END IF;
  END;

  generation_one := companion_internal.bump_placement_generation(102008, 'worker-a');
  generation_two := companion_internal.bump_placement_generation(102008, 'worker-a');
  IF generation_one <> 1 OR generation_two <> 2 THEN
    RAISE EXCEPTION 'S6 placement generation did not advance from 1 to 2';
  END IF;
  IF companion_placement_generation(102008) <> 2 THEN
    RAISE EXCEPTION 'S6 companion_placement_generation did not return the latest generation';
  END IF;
  IF companion_placement_generation(102009) <> 0 THEN
    RAISE EXCEPTION 'S6 unknown shard should return generation zero';
  END IF;
  IF NOT companion_local_placement_matches(102008, 'worker-a') THEN
    RAISE EXCEPTION 'S6 local placement helper did not match the recorded worker';
  END IF;
  IF companion_local_placement_matches(102008, 'worker-b') THEN
    RAISE EXCEPTION 'S6 local placement helper matched the wrong worker';
  END IF;
  BEGIN
    PERFORM companion_internal.bump_placement_generation(0, 'worker-a');
    RAISE EXCEPTION 'S6 placement generation accepted shard zero';
  EXCEPTION
    WHEN raise_exception THEN
      IF SQLERRM <> 'shard_id must be greater than zero' THEN
        RAISE;
      END IF;
  END;

  hash_index := companion_hash_shard_index('tenant-a', 8);
  hash_index_again := companion_hash_shard_index('tenant-a', 8);
  IF hash_index <> hash_index_again OR hash_index < 0 OR hash_index >= 8 THEN
    RAISE EXCEPTION 'S13 hash routing helper was not deterministic and bounded';
  END IF;
  range_index := companion_range_shard_index(25, 0, 100, 4);
  IF range_index <> 1 THEN
    RAISE EXCEPTION 'S13 range routing helper returned %, expected 1', range_index;
  END IF;
  BEGIN
    PERFORM companion_hash_shard_index('tenant-a', 0);
    RAISE EXCEPTION 'S13 hash routing helper accepted zero shards';
  EXCEPTION
    WHEN raise_exception THEN
      IF SQLERRM <> 'shard_count must be greater than zero' THEN
        RAISE;
      END IF;
  END;
  BEGIN
    PERFORM companion_range_shard_index(100, 0, 100, 4);
    RAISE EXCEPTION 'S13 range routing helper accepted an out-of-bounds value';
  EXCEPTION
    WHEN raise_exception THEN
      IF SQLERRM <> 'range routing value is outside shard bounds' THEN
        RAISE;
      END IF;
  END;

  PERFORM companion_set_session_claims(
    'user-123',
    'authenticated',
    'tenant-a',
    'jti-123'
  );
  IF companion_current_tenant_id() <> 'tenant-a' THEN
    RAISE EXCEPTION 'companion_current_tenant_id did not return tenant-a';
  END IF;
  IF NOT EXISTS (
    SELECT 1
    FROM companion_current_session_claims()
    WHERE uid = 'user-123'
      AND role = 'authenticated'
      AND tenant_id = 'tenant-a'
      AND jwt_id = 'jti-123'
  ) THEN
    RAISE EXCEPTION 'companion_current_session_claims did not return expected Auth2 claims';
  END IF;
  BEGIN
    PERFORM companion_set_session_claims('', 'authenticated', 'tenant-a');
    RAISE EXCEPTION 'companion_set_session_claims must reject empty uid claim';
  EXCEPTION
    WHEN raise_exception THEN
      IF SQLERRM <> 'uid claim must not be empty' THEN
        RAISE;
      END IF;
  END;

  jwt_header_segment := companion_internal.base64url_encode(
    convert_to('{"alg":"HS256","typ":"JWT"}', 'UTF8')
  );
  jwt_payload_segment := companion_internal.base64url_encode(
    convert_to(
      jsonb_build_object(
        'iss', 'https://auth.example.test',
        'aud', jsonb_build_array('ai-blaise-citus', 'analytics'),
        'sub', 'user-789',
        'role', 'authenticated',
        'tenant_id', 'tenant-c',
        'jti', 'jti-789',
        'exp', floor(extract(epoch FROM clock_timestamp() + interval '1 hour'))::bigint,
        'nbf', floor(extract(epoch FROM clock_timestamp() - interval '1 minute'))::bigint
      )::text,
      'UTF8'
    )
  );
  jwt_signing_input := jwt_header_segment || '.' || jwt_payload_segment;
  jwt_token := jwt_signing_input || '.' || companion_internal.base64url_encode(
    hmac(jwt_signing_input, 'jwt-secret', 'sha256')
  );

  SELECT * INTO jwt_claims
  FROM companion_verify_jwt_hs256(
    jwt_token,
    'jwt-secret',
    'https://auth.example.test',
    'ai-blaise-citus'
  );
  IF jwt_claims.uid <> 'user-789'
     OR jwt_claims.role <> 'authenticated'
     OR jwt_claims.tenant_id <> 'tenant-c'
     OR jwt_claims.jwt_id <> 'jti-789'
     OR jwt_claims.audience <> 'ai-blaise-citus' THEN
    RAISE EXCEPTION 'Sec2 JWT verification did not return expected claims';
  END IF;

  PERFORM companion_set_session_claims(
    jwt_claims.uid,
    jwt_claims.role,
    jwt_claims.tenant_id,
    jwt_claims.jwt_id
  );
  IF companion_current_tenant_id() <> 'tenant-c' THEN
    RAISE EXCEPTION 'Sec2 verified JWT claims did not feed Auth2 tenant claims';
  END IF;

  BEGIN
    PERFORM companion_verify_jwt_hs256(
      jwt_signing_input || '.bad-signature',
      'jwt-secret',
      'https://auth.example.test',
      'ai-blaise-citus'
    );
    RAISE EXCEPTION 'Sec2 JWT verification accepted a bad signature';
  EXCEPTION
    WHEN raise_exception THEN
      IF SQLERRM <> 'JWT signature verification failed' THEN
        RAISE;
      END IF;
  END;

  BEGIN
    PERFORM companion_verify_jwt_hs256(
      jwt_token,
      'jwt-secret',
      'https://auth.example.test',
      'wrong-audience'
    );
    RAISE EXCEPTION 'Sec2 JWT verification accepted a wrong audience';
  EXCEPTION
    WHEN raise_exception THEN
      IF SQLERRM <> 'JWT audience mismatch' THEN
        RAISE;
      END IF;
  END;

  expired_payload_segment := companion_internal.base64url_encode(
    convert_to(
      jsonb_build_object(
        'iss', 'https://auth.example.test',
        'aud', 'ai-blaise-citus',
        'sub', 'user-789',
        'role', 'authenticated',
        'tenant_id', 'tenant-c',
        'exp', floor(extract(epoch FROM clock_timestamp() - interval '1 minute'))::bigint
      )::text,
      'UTF8'
    )
  );
  jwt_signing_input := jwt_header_segment || '.' || expired_payload_segment;
  expired_token := jwt_signing_input || '.' || companion_internal.base64url_encode(
    hmac(jwt_signing_input, 'jwt-secret', 'sha256')
  );
  BEGIN
    PERFORM companion_verify_jwt_hs256(
      expired_token,
      'jwt-secret',
      'https://auth.example.test',
      'ai-blaise-citus'
    );
    RAISE EXCEPTION 'Sec2 JWT verification accepted an expired token';
  EXCEPTION
    WHEN raise_exception THEN
      IF SQLERRM <> 'JWT has expired' THEN
        RAISE;
      END IF;
  END;

  missing_tenant_payload_segment := companion_internal.base64url_encode(
    convert_to(
      jsonb_build_object(
        'iss', 'https://auth.example.test',
        'aud', 'ai-blaise-citus',
        'sub', 'user-789',
        'role', 'authenticated',
        'exp', floor(extract(epoch FROM clock_timestamp() + interval '1 hour'))::bigint
      )::text,
      'UTF8'
    )
  );
  jwt_signing_input := jwt_header_segment || '.' || missing_tenant_payload_segment;
  missing_tenant_token := jwt_signing_input || '.' || companion_internal.base64url_encode(
    hmac(jwt_signing_input, 'jwt-secret', 'sha256')
  );
  BEGIN
    PERFORM companion_verify_jwt_hs256(
      missing_tenant_token,
      'jwt-secret',
      'https://auth.example.test',
      'ai-blaise-citus'
    );
    RAISE EXCEPTION 'Sec2 JWT verification accepted a missing tenant_id claim';
  EXCEPTION
    WHEN raise_exception THEN
      IF SQLERRM <> 'JWT tenant_id claim must not be empty' THEN
        RAISE;
      END IF;
  END;

  plan_sql := distribute_hypertable('timescale_smoke_metrics', 'metric_time', '1 day', 4);
  IF plan_sql NOT LIKE '%create_hypertable%' THEN
    RAISE EXCEPTION 'distribute_hypertable did not render create_hypertable plan: %', plan_sql;
  END IF;

  plan_sql := time_range_shard_pruner('timescale_smoke_metrics', 'metric_time');
  IF plan_sql NOT LIKE '%enable_time_range_shard_pruner%' THEN
    RAISE EXCEPTION 'time_range_shard_pruner did not render pruner plan: %', plan_sql;
  END IF;

  PERFORM apply_distribute_hypertable(
    'timescale_smoke_metrics',
    'metric_time',
    '1 day',
    2
  );
  IF (
    SELECT count(*)
    FROM timescale_bridge_call_log
    WHERE function_name IN ('create_hypertable', 'create_distributed_table')
  ) <> 2 THEN
    RAISE EXCEPTION 'apply_distribute_hypertable did not call both dependency entrypoints';
  END IF;

  PERFORM companion_internal.create_worker_hypertables(
    'timescale_smoke_metrics'::regclass,
    'metric_time'::name,
    '1 day'::interval,
    4
  );
  PERFORM companion_internal.add_compression_policy_distributed(
    'timescale_smoke_metrics'::regclass,
    '7 days'::interval,
    ARRAY['metric_time']::text[],
    ARRAY['metric_time DESC']::text[]
  );
  PERFORM companion_internal.add_retention_policy_distributed(
    'timescale_smoke_metrics'::regclass,
    '90 days'::interval
  );
  PERFORM apply_retention_policy_distributed(
    'timescale_smoke_metrics',
    '90 days'
  );
  IF NOT EXISTS (
    SELECT 1
    FROM timescale_bridge_call_log
    WHERE function_name = 'add_retention_policy'
      AND relation_name = 'timescale_smoke_metrics'
  ) THEN
    RAISE EXCEPTION 'apply_retention_policy_distributed did not call dependency entrypoint';
  END IF;

  PERFORM companion_internal.add_reorder_policy_distributed(
    'timescale_smoke_metrics'::regclass,
    'timescale_smoke_metrics_metric_time_idx'::name
  );
  PERFORM apply_reorder_policy_distributed(
    'timescale_smoke_metrics',
    'timescale_smoke_metrics_metric_time_idx'
  );
  IF NOT EXISTS (
    SELECT 1
    FROM timescale_bridge_call_log
    WHERE function_name = 'add_reorder_policy'
      AND relation_name = 'timescale_smoke_metrics'
  ) THEN
    RAISE EXCEPTION 'apply_reorder_policy_distributed did not call dependency entrypoint';
  END IF;

  PERFORM companion_internal.add_continuous_aggregate_distributed(
    'timescale_smoke_hourly',
    'SELECT time_bucket(''1 hour'', metric_time), avg(value) FROM timescale_smoke_metrics GROUP BY 1',
    '7 days'::interval,
    '1 hour'::interval,
    '1 hour'::interval
  );
  PERFORM companion_internal.enable_time_range_shard_pruner(
    'timescale_smoke_metrics'::regclass,
    'metric_time'::name
  );
  PERFORM apply_time_range_shard_pruner(
    'timescale_smoke_metrics',
    'metric_time'
  );

  BEGIN
    PERFORM apply_compression_policy_distributed(
      'timescale_smoke_metrics',
      '7 days',
      'metric_time',
      'metric_time DESC'
    );
    RAISE EXCEPTION 'apply_compression_policy_distributed must require TimescaleDB dependency';
  EXCEPTION WHEN OTHERS THEN
    IF SQLERRM NOT LIKE '%requires visible function add_compression_policy from extension timescaledb%' THEN
      RAISE;
    END IF;
  END;

  BEGIN
    PERFORM apply_continuous_aggregate_distributed(
      'timescale_smoke_hourly_apply',
      'SELECT metric_time, avg(value) FROM timescale_smoke_metrics GROUP BY 1',
      '7 days',
      '1 hour',
      '1 hour'
    );
    RAISE EXCEPTION 'apply_continuous_aggregate_distributed must require TimescaleDB dependency';
  EXCEPTION WHEN OTHERS THEN
    IF SQLERRM NOT LIKE '%requires visible function add_continuous_aggregate_policy from extension timescaledb%' THEN
      RAISE;
    END IF;
  END;

  SELECT count(DISTINCT feature_id)
  INTO bridge_features
  FROM companion_timescale_bridge_state
  WHERE feature_id IN ('TS1', 'TS2', 'TS3', 'TS4', 'TS5', 'TS12');
  IF bridge_features <> 6 THEN
    RAISE EXCEPTION 'expected six Timescale bridge state feature ids, got %',
      bridge_features;
  END IF;

  IF NOT EXISTS (
    SELECT 1
    FROM companion_timescale_bridge_state
    WHERE feature_id = 'TS1'
      AND object_name = 'timescale_smoke_metrics'
      AND parameters->>'shard_count' = '2'
  ) THEN
    RAISE EXCEPTION 'public apply_distribute_hypertable state was not recorded';
  END IF;

  IF NOT EXISTS (SELECT 1 FROM companion_pg_stat_local_activity) THEN
    RAISE EXCEPTION 'companion_pg_stat_local_activity must report the local postgres node';
  END IF;

  IF NOT EXISTS (SELECT 1 FROM companion_pg_stat_distributed) THEN
    RAISE EXCEPTION 'compatibility companion_pg_stat_distributed view must report the local postgres node';
  END IF;

  IF NOT EXISTS (
    SELECT 1
    FROM companion_pg_stat_statements_p95
    WHERE query LIKE '%ai_blaise_pg_stat_statements_seed%'
      AND calls >= 1
      AND p95_ms >= 0
  ) THEN
    RAISE EXCEPTION 'companion_pg_stat_statements_p95 must report pg_stat_statements rows';
  END IF;

  PERFORM * FROM companion_pg_dist_replication_lag LIMIT 1;
END $$;

DO $$
DECLARE
  first_hash text;
  second_hash text;
  computed_seal text;
BEGIN
  first_hash := companion_internal.ledger_transfer(
    'tr_001',
    'cash',
    'revenue',
    5000,
    'USD',
    'genesis'
  );
  IF first_hash IS NULL OR length(first_hash) <> 64 THEN
    RAISE EXCEPTION 'Sec5 ledger transfer did not return a sha256 entry hash';
  END IF;

  second_hash := companion_internal.ledger_transfer(
    'tr_002',
    'cash',
    'deferred_revenue',
    2500,
    'USD',
    first_hash
  );
  IF second_hash IS NULL OR second_hash = first_hash THEN
    RAISE EXCEPTION 'Sec5 second ledger transfer did not advance the hash chain';
  END IF;
  IF NOT companion_ledger_chain_valid() THEN
    RAISE EXCEPTION 'Sec5 ledger chain should verify after ordered transfers';
  END IF;

  computed_seal := companion_ledger_seal('tr_001', 'ledger-secret', 'hmac-sha256');
  IF computed_seal IS NULL OR length(computed_seal) <> 64 THEN
    RAISE EXCEPTION 'Sec6 ledger seal did not return a sha256 HMAC';
  END IF;
  IF NOT EXISTS (
    SELECT 1
    FROM companion_ledger_entries
    WHERE transfer_id = 'tr_001'
      AND entry_hash = first_hash
      AND hmac_algorithm = 'hmac-sha256'
      AND companion_ledger_entries.seal = computed_seal
  ) THEN
    RAISE EXCEPTION 'Sec6 ledger seal was not visible through companion_ledger_entries';
  END IF;

  BEGIN
    PERFORM companion_internal.ledger_transfer(
      'tr_bad_prev',
      'cash',
      'revenue',
      100,
      'USD',
      'missing-hash'
    );
    RAISE EXCEPTION 'Sec5 ledger transfer accepted a missing previous hash';
  EXCEPTION
    WHEN raise_exception THEN
      IF SQLERRM <> 'previous_hash does not reference an existing ledger entry' THEN
        RAISE;
      END IF;
  END;

  BEGIN
    UPDATE companion_internal.ledger_entries
    SET amount_cents = 1
    WHERE transfer_id = 'tr_001';
    RAISE EXCEPTION 'Sec5 ledger entries must reject mutation';
  EXCEPTION
    WHEN raise_exception THEN
      IF SQLERRM <> 'companion ledger is append-only' THEN
        RAISE;
      END IF;
  END;

  BEGIN
    DELETE FROM companion_internal.ledger_seals
    WHERE transfer_id = 'tr_001';
    RAISE EXCEPTION 'Sec6 ledger seals must reject deletion';
  EXCEPTION
    WHEN raise_exception THEN
      IF SQLERRM <> 'companion ledger is append-only' THEN
        RAISE;
      END IF;
  END;

  BEGIN
    PERFORM companion_ledger_seal('tr_002', 'ledger-secret', 'hmac-md5');
    RAISE EXCEPTION 'Sec6 ledger seal accepted an unsupported algorithm';
  EXCEPTION
    WHEN raise_exception THEN
      IF SQLERRM <> 'unsupported ledger HMAC algorithm: hmac-md5' THEN
        RAISE;
      END IF;
  END;
END $$;

CREATE ROLE ai_blaise_rls_smoke;
CREATE TABLE rls_smoke_orders (
  order_id integer NOT NULL,
  tenant_id text NOT NULL,
  amount integer NOT NULL
);
INSERT INTO rls_smoke_orders(order_id, tenant_id, amount)
VALUES
  (1, 'tenant-a', 100),
  (2, 'tenant-b', 200);
ALTER TABLE rls_smoke_orders ENABLE ROW LEVEL SECURITY;
ALTER TABLE rls_smoke_orders FORCE ROW LEVEL SECURITY;
CREATE POLICY rls_smoke_tenant_isolation ON rls_smoke_orders
USING (companion_tenant_id_matches(tenant_id))
WITH CHECK (companion_tenant_id_matches(tenant_id));
GRANT SELECT, INSERT ON rls_smoke_orders TO ai_blaise_rls_smoke;

SELECT companion_set_session_claims('user-123', 'authenticated', 'tenant-a', 'jti-123');
SET ROLE ai_blaise_rls_smoke;
DO $$
DECLARE
  visible_count integer;
BEGIN
  SELECT count(*) INTO visible_count FROM rls_smoke_orders;
  IF visible_count <> 1 THEN
    RAISE EXCEPTION 'Sec1 RLS tenant-a should see exactly one row, got %',
      visible_count;
  END IF;
  IF NOT companion_tenant_id_matches('tenant-a') THEN
    RAISE EXCEPTION 'companion_tenant_id_matches must accept the active tenant';
  END IF;
  IF companion_tenant_id_matches('tenant-b') THEN
    RAISE EXCEPTION 'companion_tenant_id_matches must reject another tenant';
  END IF;
  INSERT INTO rls_smoke_orders(order_id, tenant_id, amount)
  VALUES (3, 'tenant-a', 300);
  BEGIN
    INSERT INTO rls_smoke_orders(order_id, tenant_id, amount)
    VALUES (4, 'tenant-b', 400);
    RAISE EXCEPTION 'Sec1 RLS WITH CHECK allowed a cross-tenant insert';
  EXCEPTION
    WHEN insufficient_privilege THEN
      NULL;
  END;
END $$;
RESET ROLE;

SELECT companion_set_session_claims('user-456', 'authenticated', 'tenant-b', 'jti-456');
SET ROLE ai_blaise_rls_smoke;
DO $$
DECLARE
  visible_count integer;
BEGIN
  SELECT count(*) INTO visible_count FROM rls_smoke_orders;
  IF visible_count <> 1 THEN
    RAISE EXCEPTION 'Sec1 RLS tenant-b should see exactly one row, got %',
      visible_count;
  END IF;
END $$;
RESET ROLE;

SELECT set_config('ai_blaise.claim.tenant_id', '', false);
SET ROLE ai_blaise_rls_smoke;
DO $$
DECLARE
  visible_count integer;
BEGIN
  SELECT count(*) INTO visible_count FROM rls_smoke_orders;
  IF visible_count <> 0 THEN
    RAISE EXCEPTION 'Sec1 RLS without tenant claim should see zero rows, got %',
      visible_count;
  END IF;
  BEGIN
    PERFORM companion_require_tenant_id();
    RAISE EXCEPTION 'companion_require_tenant_id must fail without tenant claim';
  EXCEPTION
    WHEN raise_exception THEN
      IF SQLERRM <> 'tenant_id claim must be set for RLS' THEN
        RAISE;
      END IF;
  END;
END $$;
RESET ROLE;
SQL

docker exec -d "${container}" sh -c \
  "(printf 'BEGIN;\nSELECT pg_backend_pid();\n'; sleep 60; printf 'COMMIT;\n') | psql -U postgres -v ON_ERROR_STOP=1"

idle_seen=0
for _ in $(seq 1 20); do
  idle_count="$(
    docker exec "${container}" psql -U postgres -Atqv ON_ERROR_STOP=1 \
      -c "SELECT count(*) FROM companion_idle_transactions('100 milliseconds'::interval) WHERE state = 'idle in transaction';"
  )"
  if [[ "${idle_count}" =~ ^[1-9][0-9]*$ ]]; then
    idle_seen=1
    break
  fi
  sleep 1
done

if [[ "${idle_seen}" != "1" ]]; then
  docker exec "${container}" psql -U postgres -v ON_ERROR_STOP=1 \
    -c "SELECT pid, state, xact_start, query FROM pg_stat_activity WHERE datname = current_database() ORDER BY pid;" >&2 || true
  echo "companion_idle_transactions did not detect a real idle transaction" >&2
  exit 1
fi

echo "ai_blaise_citus SQL extension smoke passed with ${postgres_image}"
