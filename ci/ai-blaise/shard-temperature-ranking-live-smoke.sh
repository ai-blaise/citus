#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "${repo_root}"

if [[ "${REQUIRE_DOCKER:-0}" != "1" ]]; then
  echo "REQUIRE_DOCKER=1 is required for shard temperature live Citus smoke" >&2
  exit 2
fi

image="${AI_BLAISE_CITUS_COHAB_IMAGE:-ai-blaise-citus-timescale-cohabitation:local}"
if ! docker image inspect "${image}" >/dev/null 2>&1; then
  echo "missing Citus cohabitation image: ${image}" >&2
  exit 1
fi

container="ai-blaise-r12-temperature-${$}-${RANDOM}"
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
DROP TABLE IF EXISTS public.temperature_orders CASCADE;
DROP TABLE IF EXISTS public.ai_blaise_shard_temperature_samples;
CREATE TABLE public.temperature_orders (
  tenant_id integer NOT NULL,
  order_id integer NOT NULL,
  total numeric NOT NULL,
  PRIMARY KEY (tenant_id, order_id)
);
SELECT create_distributed_table('public.temperature_orders', 'tenant_id');
CREATE TABLE public.ai_blaise_shard_temperature_samples (
  shard_id bigint PRIMARY KEY,
  read_ops_per_min numeric NOT NULL CHECK (read_ops_per_min >= 0),
  write_ops_per_min numeric NOT NULL CHECK (write_ops_per_min >= 0),
  bytes_read_per_min bigint NOT NULL CHECK (bytes_read_per_min >= 0),
  bytes_written_per_min bigint NOT NULL CHECK (bytes_written_per_min >= 0),
  cold_age_seconds integer NOT NULL CHECK (cold_age_seconds >= 0),
  sample_valid boolean NOT NULL DEFAULT true
);
WITH ranked_shards AS (
  SELECT shardid, row_number() OVER (ORDER BY shardid) AS rownum
  FROM pg_dist_shard
  WHERE logicalrelid = 'public.temperature_orders'::regclass
  ORDER BY shardid
  LIMIT 3
)
INSERT INTO public.ai_blaise_shard_temperature_samples (
  shard_id,
  read_ops_per_min,
  write_ops_per_min,
  bytes_read_per_min,
  bytes_written_per_min,
  cold_age_seconds,
  sample_valid
)
SELECT
  shardid,
  CASE rownum WHEN 1 THEN 120 WHEN 2 THEN 12 ELSE 0 END,
  CASE rownum WHEN 1 THEN 60 WHEN 2 THEN 3 ELSE 0 END,
  CASE rownum WHEN 1 THEN 8388608 WHEN 2 THEN 1048576 ELSE 0 END,
  CASE rownum WHEN 1 THEN 4194304 WHEN 2 THEN 524288 ELSE 0 END,
  CASE rownum WHEN 1 THEN 60 WHEN 2 THEN 7200 ELSE 172800 END,
  true
FROM ranked_shards;
DO $$
BEGIN
  IF (SELECT count(*) FROM public.ai_blaise_shard_temperature_samples) <> 3 THEN
    RAISE EXCEPTION 'expected exactly three shard temperature samples';
  END IF;
END $$;
SQL

ranking_sql="$(cargo run -q -p ai_blaise_citus_companion --bin companion_contracts -- run-shard-temperature-ranking-sql-canonical)"
ranking_output="$(docker exec -i "${container}" psql -v ON_ERROR_STOP=1 -U postgres -d postgres -X -q -AtF $'\t' <<SQL
${ranking_sql}
SQL
)"

ranking_file="$(mktemp)"
printf '%s\n' "${ranking_output}" > "${ranking_file}"
python3 - "${ranking_file}" <<'PY'
from decimal import Decimal
from pathlib import Path
import sys

path = Path(sys.argv[1])
lines = [line for line in path.read_text(encoding="utf-8").splitlines() if line]
if len(lines) != 3:
    raise SystemExit(f"expected 3 ranked shards, got {len(lines)}: {lines!r}")
rows = [line.split("\t") for line in lines]
if any(len(row) != 9 for row in rows):
    raise SystemExit(f"expected 9 columns per ranked row: {rows!r}")
if {row[3] for row in rows} != {"hot", "warm", "cold"}:
    raise SystemExit(f"target tier coverage mismatch: {rows!r}")
if [row[3] for row in rows] != ["hot", "warm", "cold"]:
    raise SystemExit(f"temperature order mismatch: {rows!r}")
if any(row[1] != "public.temperature_orders" for row in rows):
    raise SystemExit(f"unexpected table names: {rows!r}")
scores = [Decimal(row[2]) for row in rows]
if not (scores[0] > scores[1] > scores[2]):
    raise SystemExit(f"scores not strictly descending: {scores!r}")
if len({row[0] for row in rows}) != 3:
    raise SystemExit(f"shard ids not unique: {rows!r}")
if [int(row[4]) for row in rows] != [1, 2, 3]:
    raise SystemExit(f"unexpected dense ranks: {rows!r}")
PY
rm -f "${ranking_file}"

printf 'shard_temperature_ranking_live=passed\n'
printf 'citus_pg_dist_shard_joined=true\n'
printf 'temperature_scores_ranked=true\n'
printf 'hot_shards=1\n'
printf 'warm_shards=1\n'
printf 'cold_shards=1\n'
printf 'automatic_tier_movement=false\n'
printf 'coldtier_moves_executed=false\n'
printf 'operator_tier_movement_executed=false\n'
printf 'shard_temperature_ranking_live\tpassed\n'
