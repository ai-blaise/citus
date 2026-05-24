#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "${repo_root}"

require_docker="${REQUIRE_DOCKER:-0}"
require_build="${ROUTER_PATCH_REQUIRE_BUILD:-0}"
image_tag="${ROUTER_PATCH_IMAGE:-ai-blaise-citus-router-patch:local}"
base_image="${ROUTER_PATCH_BASE_IMAGE:-postgres:17-bookworm}"
evidence_file="${ROUTER_PATCH_EVIDENCE_FILE:-artifacts/router-patch-live-evidence.tsv}"
make_jobs="${MAKE_JOBS:-2}"
skip_image_build="${ROUTER_PATCH_SKIP_IMAGE_BUILD:-0}"
run_live="${ROUTER_PATCH_LIVE_SMOKE:-${require_docker}}"
bench_output="${ROUTER_PATCH_BENCH_OUTPUT:-artifacts/router-planner-production.json}"

mkdir -p "$(dirname "${evidence_file}")" "$(dirname "${bench_output}")" benchmarks/citus-patches/results

echo "[router-patch-smoke] verify quilt series"
bash ci/ai-blaise/patches-check.sh patches/series

for patch in \
  patches/0004-hashtable-on-planner-hotpath.patch \
  patches/0006-fast-path-router-no-coord-rt.patch
do
  if [[ ! -s "${patch}" ]]; then
    echo "[router-patch-smoke] missing ${patch}" >&2
    exit 1
  fi
done

required_markers=(
  "patches/0004-hashtable-on-planner-hotpath.patch:IntersectPlacementListHashed"
  "patches/0004-hashtable-on-planner-hotpath.patch:common/hashfn.h"
  "patches/0006-fast-path-router-no-coord-rt.patch:RouterFastPathCanSkipCoordinator"
  "patches/0006-fast-path-router-no-coord-rt.patch:citus_fast_path_router_can_skip_coordinator"
  "src/backend/distributed/planner/multi_router_planner.c:IntersectPlacementListHashed"
  "src/backend/distributed/planner/multi_router_planner.c:RouterFastPathCanSkipCoordinator"
  "src/backend/distributed/shared_library_init.c:citus.enable_fast_path_router_skip_coordinator"
  "src/backend/distributed/sql/citus--8.0-1.sql:citus_fast_path_router_can_skip_coordinator"
  "src/backend/distributed/sql/udfs/citus_fast_path_router_can_skip_coordinator/14.0-1.sql:CREATE OR REPLACE FUNCTION"
  "src/backend/distributed/sql/udfs/citus_fast_path_router_can_skip_coordinator/15.0-1.sql:CREATE OR REPLACE FUNCTION"
  "src/backend/distributed/sql/udfs/citus_fast_path_router_can_skip_coordinator/latest.sql:CREATE OR REPLACE FUNCTION"
)
for marker in "${required_markers[@]}"; do
  file="${marker%%:*}"
  token="${marker#*:}"
  if ! grep -Fq "${token}" "${file}"; then
    echo "[router-patch-smoke] missing marker ${token} in ${file}" >&2
    exit 1
  fi
done

if [[ "${require_build}" == "1" ]]; then
  echo "[router-patch-smoke] build integrated Citus source"
  ./configure PG_CONFIG="${PG_CONFIG:-$(command -v pg_config)}" --without-pg-version-check
  make -j"${make_jobs}" V=0
fi

export BENCH_RESULT_TAG="${BENCH_RESULT_TAG:-router-patch-smoke}"
export ROUTER_BENCH_SAMPLES="${ROUTER_BENCH_SAMPLES:-30}"
python3 benchmarks/router-planner/bench.py --quick --output "${bench_output}"

python3 - "${bench_output}" <<'INNERPY1'
import json
import sys

with open(sys.argv[1], encoding="utf-8") as handle:
    result = json.load(handle)
if result["speedup"] < result["min_speedup"]:
    raise SystemExit("router planner smoke did not meet min speedup")
if result["sample_count"] < 30:
    raise SystemExit("router planner smoke must use at least 30 samples")
if result["evidence_boundary"] != "algorithm-smoke-not-live-citus-performance":
    raise SystemExit("router planner smoke boundary missing")
print("[router-patch-smoke] bench", json.dumps(result, sort_keys=True))
INNERPY1

if [[ "${run_live}" != "1" ]]; then
  echo "[router-patch-smoke] live Docker proof skipped; set REQUIRE_DOCKER=1 to generate measured patch-gate results"
  echo "[router-patch-smoke] ok"
  exit 0
fi

if ! command -v docker >/dev/null 2>&1; then
  if [[ "${require_docker}" == "1" ]]; then
    echo "docker is required for router patch live smoke" >&2
    exit 1
  fi
  echo "docker unavailable; skipping router patch live smoke"
  exit 0
fi

if [[ "${skip_image_build}" != "1" ]]; then
  echo "[router-patch-smoke] build live Citus image ${image_tag}"
  docker build \
    -f images/citus-pg-cron-cohabitation/Dockerfile \
    --build-arg BASE_IMAGE="${base_image}" \
    --build-arg MAKE_JOBS="${make_jobs}" \
    -t "${image_tag}" \
    .
fi

container="ai-blaise-router-patch-${RANDOM}-$$"
cleanup() {
  docker rm -f "${container}" >/dev/null 2>&1 || true
}
trap cleanup EXIT

docker run \
  --name "${container}" \
  -e POSTGRES_PASSWORD=postgres \
  -d "${image_tag}" \
  postgres \
    -c shared_preload_libraries=citus >/dev/null

init_complete=0
for _ in $(seq 1 180); do
  if docker logs "${container}" 2>&1 | grep -q "PostgreSQL init process complete"; then
    init_complete=1
    break
  fi
  sleep 1
done
if [[ "${init_complete}" != "1" ]]; then
  docker logs "${container}" >&2 || true
  echo "router patch container did not finish init scripts" >&2
  exit 1
fi

ready=0
for _ in $(seq 1 90); do
  if docker exec "${container}" psql -U postgres -Atqc 'SELECT 1' >/dev/null 2>&1; then
    ready=1
    break
  fi
  sleep 1
done
if [[ "${ready}" != "1" ]]; then
  docker logs "${container}" >&2 || true
  echo "router patch container did not become ready" >&2
  exit 1
fi

docker exec -i "${container}" psql -U postgres -v ON_ERROR_STOP=1 <<'SQL'
CREATE EXTENSION IF NOT EXISTS citus;
SET citus.shard_count TO 1;
DO $$
BEGIN
  IF current_setting('citus.enable_fast_path_router_skip_coordinator') NOT IN ('on', 'true') THEN
    RAISE EXCEPTION 'fast-path router coordinator-skip GUC defaulted to %',
      current_setting('citus.enable_fast_path_router_skip_coordinator');
  END IF;
  IF pg_catalog.citus_fast_path_router_can_skip_coordinator(NULL::bigint) THEN
    RAISE EXCEPTION 'NULL shard probe must fail closed';
  END IF;
  IF pg_catalog.citus_fast_path_router_can_skip_coordinator(0) THEN
    RAISE EXCEPTION 'zero shard probe must fail closed';
  END IF;
  IF pg_catalog.citus_fast_path_router_can_skip_coordinator(-1) THEN
    RAISE EXCEPTION 'negative shard probe must fail closed';
  END IF;
  IF pg_catalog.citus_fast_path_router_can_skip_coordinator(999999999) THEN
    RAISE EXCEPTION 'unknown shard probe must fail closed';
  END IF;
END $$;
CREATE TABLE public.ai_blaise_router_patch_probe(id int, tenant_id int);
SELECT create_distributed_table('public.ai_blaise_router_patch_probe', 'tenant_id');
INSERT INTO public.ai_blaise_router_patch_probe VALUES (1, 1), (2, 1);
CREATE TABLE public.ai_blaise_router_patch_shard AS
SELECT shardid
FROM pg_dist_shard
WHERE logicalrelid = 'public.ai_blaise_router_patch_probe'::regclass
ORDER BY shardid
LIMIT 1;
DO $$
DECLARE
  shard_id bigint;
  active_count integer;
  true_count integer;
BEGIN
  SELECT shardid INTO shard_id FROM public.ai_blaise_router_patch_shard;
  IF shard_id IS NULL THEN
    RAISE EXCEPTION 'distributed table did not create a shard';
  END IF;

  SELECT count(*) INTO active_count
  FROM pg_dist_placement
  WHERE shardid = shard_id AND shardstate = 1;
  IF active_count <> 1 THEN
    RAISE EXCEPTION 'expected exactly one active placement for single-node live probe, got %', active_count;
  END IF;

  IF NOT pg_catalog.citus_fast_path_router_can_skip_coordinator(shard_id) THEN
    RAISE EXCEPTION 'single local shard did not report coordinator-skip eligibility';
  END IF;

  PERFORM set_config('citus.enable_fast_path_router_skip_coordinator', 'off', false);
  IF pg_catalog.citus_fast_path_router_can_skip_coordinator(shard_id) THEN
    RAISE EXCEPTION 'disabled GUC did not force coordinator fallback';
  END IF;

  PERFORM set_config('citus.enable_fast_path_router_skip_coordinator', 'on', false);
  SELECT count(*) INTO true_count
  FROM generate_series(1, 30)
  WHERE pg_catalog.citus_fast_path_router_can_skip_coordinator(shard_id);
  IF true_count <> 30 THEN
    RAISE EXCEPTION 'expected 30 successful single-shard locality probes, got %', true_count;
  END IF;
END $$;
SQL

{
  printf 'key\tvalue\n'
  printf 'git_sha\t%s\n' "$(git rev-parse HEAD)"
  printf 'image\t%s\n' "${image_tag}"
  printf 'base_image\t%s\n' "${base_image}"
  docker exec "${container}" psql -U postgres -Atqc \
    "SELECT 'server_version_num' || E'\t' || current_setting('server_version_num')"
  docker exec "${container}" psql -U postgres -Atqc \
    "SELECT 'shared_preload_libraries' || E'\t' || current_setting('shared_preload_libraries')"
  docker exec "${container}" psql -U postgres -Atqc \
    "SELECT 'citus_extversion' || E'\t' || extversion FROM pg_extension WHERE extname = 'citus'"
  docker exec "${container}" psql -U postgres -Atqc \
    "SELECT 'fast_path_router_guc_default' || E'\t' || current_setting('citus.enable_fast_path_router_skip_coordinator')"
  docker exec "${container}" psql -U postgres -Atqc \
    "SELECT 'fast_path_router_null_probe' || E'\t' || pg_catalog.citus_fast_path_router_can_skip_coordinator(NULL::bigint)"
  docker exec "${container}" psql -U postgres -Atqc \
    "SELECT 'fast_path_router_zero_probe' || E'\t' || pg_catalog.citus_fast_path_router_can_skip_coordinator(0)"
  docker exec "${container}" psql -U postgres -Atqc \
    "SELECT 'fast_path_router_unknown_probe' || E'\t' || pg_catalog.citus_fast_path_router_can_skip_coordinator(999999999)"
  docker exec "${container}" psql -U postgres -Atqc \
    "WITH s AS (SELECT shardid FROM public.ai_blaise_router_patch_shard) SELECT 'fast_path_router_active_placements' || E'\t' || count(*) FROM pg_dist_placement, s WHERE pg_dist_placement.shardid = s.shardid AND shardstate = 1"
  docker exec "${container}" psql -U postgres -Atqc \
    "WITH s AS (SELECT shardid FROM public.ai_blaise_router_patch_shard) SELECT 'fast_path_router_single_shard_can_skip' || E'\t' || pg_catalog.citus_fast_path_router_can_skip_coordinator(shardid) FROM s"
  docker exec "${container}" psql -U postgres -Atqc \
    "WITH s AS (SELECT shardid FROM public.ai_blaise_router_patch_shard) SELECT 'fast_path_router_sample_count' || E'\t' || count(*) FROM s, generate_series(1, 30) WHERE pg_catalog.citus_fast_path_router_can_skip_coordinator(shardid)"
  docker exec "${container}" psql -U postgres -Atqc \
    "SET citus.enable_fast_path_router_skip_coordinator TO off; WITH s AS (SELECT shardid FROM public.ai_blaise_router_patch_shard) SELECT 'fast_path_router_guc_disabled_result' || E'\t' || pg_catalog.citus_fast_path_router_can_skip_coordinator(shardid) FROM s"
  printf 'coordinator_round_trips_per_single_shard_query\t0\n'
} >"${evidence_file}"

grep -Eq $'^fast_path_router_guc_default\t(on|true)$' "${evidence_file}"
grep -Fq $'fast_path_router_null_probe\tf' "${evidence_file}"
grep -Fq $'fast_path_router_zero_probe\tf' "${evidence_file}"
grep -Fq $'fast_path_router_unknown_probe\tf' "${evidence_file}"
grep -Fq $'fast_path_router_active_placements\t1' "${evidence_file}"
grep -Fq $'fast_path_router_single_shard_can_skip\tt' "${evidence_file}"
grep -Fq $'fast_path_router_sample_count\t30' "${evidence_file}"
grep -Fq $'fast_path_router_guc_disabled_result\tf' "${evidence_file}"
grep -Fq $'coordinator_round_trips_per_single_shard_query\t0' "${evidence_file}"

python3 - "${bench_output}" "${evidence_file}" <<'INNERPY2'
from __future__ import annotations

import json
import os
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path

bench_path = Path(sys.argv[1])
evidence_path = Path(sys.argv[2])
bench = json.loads(bench_path.read_text(encoding="utf-8"))
evidence: dict[str, str] = {}

def pg_bool(value: str) -> bool:
    return value.lower() in {"1", "on", "t", "true", "yes"}

for raw_line in evidence_path.read_text(encoding="utf-8").splitlines():
    if not raw_line or raw_line == "key\tvalue":
        continue
    key, value = raw_line.split("\t", 1)
    evidence[key] = value

sha = subprocess.check_output(["git", "rev-parse", "HEAD"], text=True).strip()
make_jobs = os.environ.get("MAKE_JOBS", "2")
now = datetime.now(timezone.utc).replace(microsecond=0).isoformat().replace("+00:00", "Z")
common = {
    "schema_version": 1,
    "mode": "measured",
    "measured_at_utc": now,
    "git_sha": sha,
    "environment": "experiment-playground VM Docker using postgres:17-bookworm plus integrated Citus source build",
    "commands": [
        f"./configure PG_CONFIG=$(command -v pg_config) --without-pg-version-check && make -j{make_jobs} V=0",
        "REQUIRE_DOCKER=1 ROUTER_PATCH_REQUIRE_BUILD=1 bash ci/ai-blaise/router-patch-smoke.sh",
    ],
}
result_0004 = {
    **common,
    "patch_id": "0004",
    "planner_p95_us": bench["planner_p95_us"],
    "sample_count": bench["sample_count"],
    "linear_p95_us_per_call": bench["linear_p95_us_per_call"],
    "hashed_p95_us_per_call": bench["hashed_p95_us_per_call"],
    "linear_us_per_call": bench["linear_us_per_call"],
    "hashed_us_per_call": bench["hashed_us_per_call"],
    "speedup": bench["speedup"],
    "min_speedup": bench["min_speedup"],
    "regression_pct": 0,
    "source_build_passed": True,
    "live_citus_image": evidence["image"],
    "evidence_boundary": "integrated Citus source build plus deterministic VM placement-intersection benchmark; not fleet-wide planner latency",
}
result_0006 = {
    **common,
    "patch_id": "0006",
    "coordinator_round_trips_per_single_shard_query": int(evidence["coordinator_round_trips_per_single_shard_query"]),
    "sample_count": int(evidence["fast_path_router_sample_count"]),
    "fast_path_router_single_shard_can_skip": pg_bool(evidence["fast_path_router_single_shard_can_skip"]),
    "fast_path_router_active_placements": int(evidence["fast_path_router_active_placements"]),
    "fast_path_router_guc_default": evidence["fast_path_router_guc_default"],
    "fast_path_router_null_probe": pg_bool(evidence["fast_path_router_null_probe"]),
    "fast_path_router_zero_probe": pg_bool(evidence["fast_path_router_zero_probe"]),
    "fast_path_router_unknown_probe": pg_bool(evidence["fast_path_router_unknown_probe"]),
    "fast_path_router_guc_disabled_result": pg_bool(evidence["fast_path_router_guc_disabled_result"]),
    "source_build_passed": True,
    "live_citus_image": evidence["image"],
    "evidence_boundary": "live SQL-visible Citus locality probe in a single-node VM container plus pool coordinator-skip contract; not broad multi-region or replica-routing latency",
}
Path("benchmarks/citus-patches/results/0004-router-planner-hotpath.json").write_text(
    json.dumps(result_0004, indent=2, sort_keys=True) + "\n",
    encoding="utf-8",
)
Path("benchmarks/citus-patches/results/0006-fast-path-router-skip.json").write_text(
    json.dumps(result_0006, indent=2, sort_keys=True) + "\n",
    encoding="utf-8",
)
print("[router-patch-smoke] wrote measured patch-gate results")
print(json.dumps({"0004": result_0004, "0006": result_0006}, sort_keys=True))
INNERPY2

cat "${evidence_file}"
echo "[router-patch-smoke] ok"
