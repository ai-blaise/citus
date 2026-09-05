#!/usr/bin/env bash
set -euo pipefail

# FEATURE: TS10
# FEATURE: TS11
# Live bounded Timescale advanced proof. This smoke starts a real Citus plus
# TimescaleDB cohabitation container, creates and refreshes a two-level
# continuous aggregate hierarchy, records Timescale segmentby compression
# metadata, and materializes companion SQL bloom-filter rows for segmentby keys.
# It does not claim native Timescale bloom filters, planner integration,
# compressed-chunk scan pruning, multi-worker fanout, or Kubernetes traffic.

repo_root="$(git rev-parse --show-toplevel)"
cd "${repo_root}"
if [[ -f "${HOME}/.cargo/env" ]]; then
  # shellcheck disable=SC1091
  source "${HOME}/.cargo/env"
fi

if [[ "${REQUIRE_DOCKER:-0}" != "1" ]]; then
  echo "REQUIRE_DOCKER=1 is required for Timescale advanced live smoke" >&2
  exit 2
fi

need_cmd() {
  command -v "$1" >/dev/null 2>&1 || {
    echo "$1 is required for Timescale advanced live smoke" >&2
    exit 1
  }
}

need_cmd cargo
need_cmd docker
need_cmd python3

image="${AI_BLAISE_CITUS_COHAB_IMAGE:-ai-blaise-citus-timescale-cohabitation:local}"
if ! docker image inspect "${image}" >/dev/null 2>&1; then
  echo "missing Citus/Timescale cohabitation image: ${image}" >&2
  exit 1
fi

container="ai-blaise-ts10-ts11-advanced-${$}-${RANDOM}"
cleanup() {
  docker rm -f "${container}" >/dev/null 2>&1 || true
}
trap cleanup EXIT

docker run -d --name "${container}" \
  -e POSTGRES_PASSWORD=postgres \
  -e POSTGRES_DB=postgres \
  "${image}" \
  -c shared_preload_libraries=timescaledb,citus \
  -c citus.cohabit_extensions=timescaledb >/dev/null

ready=0
for _ in $(seq 1 90); do
  if docker exec "${container}" pg_isready -U postgres >/dev/null 2>&1 \
    && docker exec "${container}" psql -U postgres -d postgres -Atqc "SELECT 1" >/dev/null 2>&1; then
    ready=1
    break
  fi
  sleep 2
done
if [[ "${ready}" != "1" ]]; then
  echo "postgres did not become ready in ${container}" >&2
  docker logs "${container}" >&2 || true
  exit 1
fi

docker exec -i "${container}" psql -v ON_ERROR_STOP=1 -U postgres -d postgres <<'SQL'
CREATE EXTENSION IF NOT EXISTS citus;
CREATE EXTENSION IF NOT EXISTS timescaledb;
CREATE EXTENSION IF NOT EXISTS ai_blaise_citus;
DO $$
BEGIN
  IF (SELECT extversion FROM pg_extension WHERE extname = 'ai_blaise_citus')
      IS DISTINCT FROM '0.1.2' THEN
    RAISE EXCEPTION 'expected shipped ai_blaise_citus version 0.1.2';
  END IF;
END $$;
DROP MATERIALIZED VIEW IF EXISTS public.ts10_daily CASCADE;
DROP MATERIALIZED VIEW IF EXISTS public.ts10_hourly CASCADE;
DROP TABLE IF EXISTS public.ts11_segmentby_bloom_filters;
DROP TABLE IF EXISTS public.ts10_ts11_metrics CASCADE;
CREATE TABLE public.ts10_ts11_metrics(
  metric_time timestamptz NOT NULL,
  tenant_id integer NOT NULL,
  device_id integer NOT NULL,
  value double precision NOT NULL
);
SELECT create_hypertable('public.ts10_ts11_metrics', 'metric_time', if_not_exists => true);
SELECT create_distributed_table('public.ts10_ts11_metrics', 'tenant_id');
INSERT INTO public.ts10_ts11_metrics
SELECT
  now() - (generated_minute || ' minutes')::interval,
  (generated_minute % 4) + 1,
  (generated_minute % 16) + 1,
  generated_minute::double precision
FROM generate_series(1, 240) AS generated_minute;
SQL

ts_sql="$(cargo run -q -p ai_blaise_citus_companion --bin companion_contracts -- run-timescale-advanced-sql-canonical)"
ts_output="$(docker exec -i "${container}" psql -v ON_ERROR_STOP=1 -U postgres -d postgres -X -q -AtF $'\t' <<SQL
${ts_sql}
SQL
)"

ts_file="$(mktemp)"
printf '%s\n' "${ts_output}" > "${ts_file}"
python3 - "${ts_file}" <<'PY_CHECK'
from pathlib import Path
import sys

lines = [line for line in Path(sys.argv[1]).read_text(encoding="utf-8").splitlines() if line]
values = {}
details = {}
for line in lines:
    parts = line.split("\t")
    if len(parts) >= 2:
        values[parts[0]] = parts[1]
        details[parts[0]] = parts[2] if len(parts) > 2 else ""
if values.get("hierarchical_cagg_count") != "2":
    raise SystemExit(f"expected two continuous aggregates, got {values.get('hierarchical_cagg_count')}; output={lines!r}")
if int(values.get("hierarchical_cagg_daily_rows", "0")) < 1:
    raise SystemExit(f"expected refreshed daily CAGG rows, got {values.get('hierarchical_cagg_daily_rows')}")
if values.get("compression_segmentby_columns") != "2":
    raise SystemExit(f"expected two segmentby compression columns, got {values.get('compression_segmentby_columns')} detail={details.get('compression_segmentby_columns')}")
if details.get("compression_segmentby_columns") != "tenant_id,device_id":
    raise SystemExit(f"unexpected segmentby detail: {details.get('compression_segmentby_columns')}")
if values.get("segmentby_bloom_rows") != "16":
    raise SystemExit(f"expected 16 segmentby bloom rows, got {values.get('segmentby_bloom_rows')}")
if details.get("segmentby_bloom_rows") != "2048:3":
    raise SystemExit(f"expected bloom detail 2048:3, got {details.get('segmentby_bloom_rows')}")
if values.get("native_timescale_bloom_filter") != "false":
    raise SystemExit("native Timescale bloom filter marker must remain false")
if values.get("planner_integration_exercised") != "false":
    raise SystemExit("planner integration marker must remain false")
PY_CHECK
rm -f "${ts_file}"

daily_rows="$(printf '%s\n' "${ts_output}" | awk -F '\t' '$1 == "hierarchical_cagg_daily_rows" {print $2; exit}')"

printf 'timescale_advanced_live=passed\n'
printf 'hierarchical_cagg_count=2\n'
printf 'hierarchical_cagg_daily_rows=%s\n' "${daily_rows}"
printf 'compression_segmentby_columns=2\n'
printf 'compression_segmentby_detail=tenant_id,device_id\n'
printf 'segmentby_bloom_rows=16\n'
printf 'segmentby_bloom_bit_count=2048\n'
printf 'segmentby_bloom_hash_count=3\n'
printf 'native_timescale_bloom_filter=false\n'
printf 'planner_integration_exercised=false\n'
printf 'compressed_chunk_scan_pruning_exercised=false\n'
printf 'multi_worker_fanout_exercised=false\n'
printf 'kubernetes_traffic_exercised=false\n'
printf 'timescale_advanced_live\tpassed\n'
