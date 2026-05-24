#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "${repo_root}"

if [[ -f "${HOME}/.cargo/env" ]]; then
  # shellcheck source=/dev/null
  source "${HOME}/.cargo/env"
fi

expected=$'ai-blaise-citus-repack-weekly-orders\tpg_repack\t5\tusers-add-display-name\t8\twrite_only\tupdate_origin_differs\tapply_remote_if_newer\t3\tai-blaise-citus-sidecar-primary-realtime\t2\t4'

cargo test -q -p ai_blaise_citus_operator

output="$(cargo run -q -p ai_blaise_citus_operator -- run-reconcile-plans-batch-c)"
if [[ "${output}" != "${expected}" ]]; then
  echo "Batch C reconcile plan contract changed." >&2
  echo "Expected: ${expected}" >&2
  echo "Actual: ${output}" >&2
  exit 1
fi

grep -Fq "FEATURE: R7" operator/src/reconcile/scheduled_repack.rs
grep -Fq "FEATURE: C4" operator/src/reconcile/conflict_policy.rs
grep -Fq "FEATURE: C5" operator/src/reconcile/conflict_policy.rs
grep -Fq "FEATURE: C9" operator/src/reconcile/migration.rs
grep -Fq "FEATURE: M3" operator/src/reconcile/migration.rs
grep -Fq "FEATURE: M14" operator/src/reconcile/migration.rs
grep -Fq "FEATURE: O5" operator/src/reconcile/sidecar.rs

conflict_sql="$(cargo run -q -p ai_blaise_citus_operator -- run-conflict-policy-runtime-canonical)"
for phrase in \
  "FEATURE: C4 C5 live conflict-policy metadata apply" \
  "accounts-lww" \
  "accounts-merge" \
  "update_origin_differs" \
  "apply_remote_if_newer" \
  "update_exists" \
  "merge_function" \
  "public.merge_remote_into_local(jsonb,jsonb)" \
  "replication_conflict_status"; do
  grep -Fq "${phrase}" <<<"${conflict_sql}"
done

companion_report="$(cargo run -q -p ai_blaise_citus_companion --bin companion_runtime_depth_a -- run-canonical)"
grep -Fq $'features\tfeature_ids\tmigration_phases\tmigration_sql_batches' <<<"${companion_report}"
grep -Fq $'5\tM1,M11,R6,C4,C5\t6\t4\t9\t12\t3\t1\t7\t7\t2\t14' <<<"${companion_report}"
grep -Fq "companion.replication_conflict_audit" companion/src/replication_conflict.rs

require_docker="${REQUIRE_DOCKER:-0}"
conflict_image="${CONFLICT_POLICY_IMAGE:-ai-blaise-citus-pg-cron-cohabitation:local}"
container=""
cleanup() {
  if [[ -n "${container}" ]]; then
    docker rm -f "${container}" >/dev/null 2>&1 || true
  fi
}
trap cleanup EXIT

if ! command -v docker >/dev/null 2>&1; then
  if [[ "${require_docker}" == "1" ]]; then
    echo "docker is required for conflict-policy runtime smoke" >&2
    exit 1
  fi
  printf 'operator_reconcilers_batch_c\t%s\n' "${output}"
  printf 'conflict_policy_runtime_smoke\tskipped_no_docker\n'
  exit 0
fi

if ! docker image inspect "${conflict_image}" >/dev/null 2>&1; then
  if [[ "${require_docker}" == "1" ]]; then
    echo "missing conflict-policy runtime image: ${conflict_image}" >&2
    exit 1
  fi
  printf 'operator_reconcilers_batch_c\t%s\n' "${output}"
  printf 'conflict_policy_runtime_smoke\tskipped_missing_image\t%s\n' "${conflict_image}"
  exit 0
fi

container="ai-blaise-conflict-policy-${RANDOM}-$$"
docker run \
  --name "${container}" \
  -e POSTGRES_HOST_AUTH_METHOD=trust \
  -d "${conflict_image}" \
  postgres \
    -c shared_preload_libraries=pg_cron,citus \
    -c citus.cohabit_extensions=pg_cron \
    -c cron.database_name=postgres >/dev/null

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
  echo "conflict-policy container did not finish init scripts" >&2
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
  echo "conflict-policy container did not become ready" >&2
  exit 1
fi

docker exec -i "${container}" psql -U postgres -v ON_ERROR_STOP=1 <<'SQL'
CREATE EXTENSION IF NOT EXISTS citus;
CREATE EXTENSION IF NOT EXISTS ai_blaise_citus;
CREATE OR REPLACE FUNCTION public.merge_remote_into_local(local_row jsonb, remote_row jsonb)
RETURNS jsonb
LANGUAGE sql
IMMUTABLE
AS $$ SELECT remote_row $$;
SQL

printf '%s\n' "${conflict_sql}" | docker exec -i "${container}" psql -U postgres -v ON_ERROR_STOP=1

extension_name="$(docker exec "${container}" psql -U postgres -Atqc "SELECT extname FROM pg_extension WHERE extname = 'ai_blaise_citus'")"
status_table="$(docker exec "${container}" psql -U postgres -Atqc "SELECT CASE WHEN to_regclass('companion_internal.replication_conflict_status') IS NOT NULL THEN 'true' ELSE 'false' END")"
policy_rows="$(docker exec "${container}" psql -U postgres -AtX -F $'\t' -c "SELECT policy_name, table_name, conflict_class, resolution, coalesce(custom_function, '<NULL>') FROM companion_internal.replication_conflict_policies ORDER BY policy_name")"

if [[ "${extension_name}" != "ai_blaise_citus" ]]; then
  echo "ai_blaise_citus extension was not installed in conflict-policy runtime smoke" >&2
  exit 1
fi
if [[ "${status_table}" != "true" ]]; then
  echo "conflict-policy status table was not created" >&2
  exit 1
fi
grep -Fxq $'accounts-lww\tpublic.reference_accounts\tupdate_origin_differs\tapply_remote_if_newer\t<NULL>' <<<"${policy_rows}"
grep -Fxq $'accounts-merge\tpublic.reference_accounts\tupdate_exists\tmerge_function\tpublic.merge_remote_into_local' <<<"${policy_rows}"

printf 'operator_reconcilers_batch_c\t%s\n' "${output}"
printf 'conflict_policy_live_extension\t%s\n' "${extension_name}"
printf 'conflict_policy_live_status_table\t%s\n' "${status_table}"
while IFS= read -r row; do
  printf 'conflict_policy_live_row\t%s\n' "${row}"
done <<<"${policy_rows}"
printf 'conflict_policy_companion_taxonomy\tconflict_classes\t7\taudit_table\tcompanion.replication_conflict_audit\n'
