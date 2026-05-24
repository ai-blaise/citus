#!/usr/bin/env bash
set -euo pipefail

# FEATURE: T2

repo_root="$(git rev-parse --show-toplevel)"
cd "${repo_root}"

fail() {
  echo "placement-generation-udf-contract-smoke: $*" >&2
  exit 1
}

require_file() {
  if [[ ! -s "$1" ]]; then
    fail "missing required file: $1"
  fi
}

metadata_c="src/backend/distributed/metadata/metadata_cache.c"
metadata_h="src/include/distributed/metadata_cache.h"
base_sql="src/backend/distributed/sql/citus--8.0-1.sql"
upgrade_sql="src/backend/distributed/sql/citus--14.0-1--15.0-1.sql"
udf_dir="src/backend/distributed/sql/udfs/citus_placement_generation"
udf_latest="${udf_dir}/latest.sql"
udf_current="${udf_dir}/15.0-1.sql"
udf_upstream="${udf_dir}/14.0-1.sql"
router_assist="companion/src/router_assist.rs"
patch_file="patches/0005-placement-generation-counter.patch"
feature_doc="docs/ai-blaise/NEW_FEATURES.md"
pg_cron_smoke="ci/ai-blaise/pg-cron-cohabitation-smoke.sh"

for file in \
  "${metadata_c}" \
  "${metadata_h}" \
  "${base_sql}" \
  "${upgrade_sql}" \
  "${udf_latest}" \
  "${udf_current}" \
  "${udf_upstream}" \
  "${router_assist}" \
  "${patch_file}" \
  "${feature_doc}" \
  "${pg_cron_smoke}"
do
  require_file "${file}"
done

cmp -s "${udf_latest}" "${udf_current}" || fail "latest UDF SQL must match 15.0-1 snapshot"
cmp -s "${udf_latest}" "${udf_upstream}" || fail "latest UDF SQL must match upstream 14.0-1 snapshot"

for required in \
  "PG_FUNCTION_INFO_V1(citus_placement_generation)" \
  "citus_placement_generation(PG_FUNCTION_ARGS)" \
  "PG_RETURN_INT64((int64) placementGeneration)"
do
  grep -Fq "${required}" "${metadata_c}" || fail "metadata C file lost ${required}"
done

for required in \
  "extern uint64 CurrentPlacementGeneration(void);" \
  "extern void BumpPlacementGeneration(void);"
do
  grep -Fq "${required}" "${metadata_h}" || fail "metadata header lost ${required}"
done

for file in "${base_sql}" "${udf_latest}" "${udf_current}" "${udf_upstream}"; do
  grep -Fq "CREATE OR REPLACE FUNCTION pg_catalog.citus_placement_generation()" "${file}" || \
    grep -Fq "CREATE FUNCTION pg_catalog.citus_placement_generation()" "${file}" || \
    fail "${file} does not expose pg_catalog.citus_placement_generation()"
  grep -Fq "RETURNS bigint" "${file}" || fail "${file} does not return bigint"
  grep -Fq "AS 'MODULE_PATHNAME', \$\$citus_placement_generation\$\$" "${file}" || \
    fail "${file} does not bind the C symbol"
  grep -Fq "GRANT EXECUTE ON FUNCTION pg_catalog.citus_placement_generation() TO PUBLIC" "${file}" || \
    fail "${file} does not leave the read-only UDF executable by pool users"
done

grep -Fq '#include "udfs/citus_placement_generation/15.0-1.sql"' "${upgrade_sql}" || \
  fail "15.0 upgrade SQL must include the placement-generation UDF"
grep -Fq "SELECT pg_catalog.citus_placement_generation();" "${router_assist}" || \
  fail "companion router_assist must poll the installed pg_catalog UDF"

for required in \
  "src/backend/distributed/sql/citus--8.0-1.sql" \
  "src/backend/distributed/sql/udfs/citus_placement_generation/14.0-1.sql" \
  "src/backend/distributed/sql/udfs/citus_placement_generation/latest.sql" \
  "GRANT EXECUTE ON FUNCTION pg_catalog.citus_placement_generation() TO PUBLIC"
do
  grep -Fq "${required}" "${patch_file}" || fail "patch artifact lost ${required}"
done

grep -Fq "placement-generation-udf-contract-smoke.sh" "${feature_doc}" || \
  fail "NEW_FEATURES.md must reference the placement-generation UDF smoke"
grep -Fq "pg-cron-cohabitation-smoke.sh" "${feature_doc}" || \
  fail "NEW_FEATURES.md must reference the live patched-Citus runtime smoke"
for required in \
  "placement_generation_after_first_distribution" \
  "placement_generation_after_second_distribution" \
  "placement_generation_placements" \
  "citus_shard_count_parameter_status" \
  "SET citus.shard_count TO 7" \
  "production latency"
do
  grep -Fq "${required}" "${feature_doc}" || \
    fail "NEW_FEATURES.md lost T2 runtime proof/boundary: ${required}"
done
for required in \
  "placement_generation_after_first_distribution" \
  "placement_generation_after_second_distribution" \
  "placement_generation_placements" \
  "citus_shard_count_parameter_status" \
  "SET citus.shard_count TO 7"
do
  grep -Fq "${required}" "${pg_cron_smoke}" || \
    fail "pg-cron cohabitation smoke lost T2 runtime proof: ${required}"
done

printf 'placement_generation_udf_contract_smoke\tudf_snapshots=3\tbase_sql=true\tupgrade_sql=true\tpatch_artifact=true\tlive_runtime_gate=true\n'
