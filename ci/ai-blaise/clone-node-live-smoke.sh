#!/usr/bin/env bash
set -euo pipefail

# FEATURE: S3
# Live bounded Citus clone-node proof. This smoke starts a real Citus
# coordinator, a primary worker, and a physical streaming replica clone. It
# drives companion-rendered SQL through citus_add_clone_node and
# citus_promote_clone_and_rebalance, then verifies catch-up/promotion and
# distributed-table data preservation. It does not claim Kubernetes clone
# orchestration, CSI snapshots, automatic capacity policy, WAN/cross-region
# clone, service/DNS retargeting, or production traffic cutover.

repo_root="$(git rev-parse --show-toplevel)"
cd "${repo_root}"
if [[ -f "${HOME}/.cargo/env" ]]; then
  # shellcheck disable=SC1091
  source "${HOME}/.cargo/env"
fi

if [[ "${REQUIRE_DOCKER:-0}" != "1" ]]; then
  echo "REQUIRE_DOCKER=1 is required for clone node live smoke" >&2
  exit 2
fi

need_cmd() {
  command -v "$1" >/dev/null 2>&1 || {
    echo "$1 is required for clone node live smoke" >&2
    exit 1
  }
}

need_cmd cargo
need_cmd docker
need_cmd python3

image="${AI_BLAISE_CITUS_COHAB_IMAGE:-ai-blaise-citus-timescale-cohabitation:local}"
if ! docker image inspect "${image}" >/dev/null 2>&1; then
  echo "missing Citus cohabitation image: ${image}" >&2
  exit 1
fi

network="ai-blaise-s3-clone-${$}-${RANDOM}"
coordinator="${network}-coord"
primary_worker="${network}-primary"
clone_worker="${network}-clone"
cleanup() {
  docker rm -f "${coordinator}" "${primary_worker}" "${clone_worker}" >/dev/null 2>&1 || true
  docker network rm "${network}" >/dev/null 2>&1 || true
}
trap cleanup EXIT

docker network create "${network}" >/dev/null

run_primary_container() {
  docker run -d \
    --name "$1" \
    --network "${network}" \
    -e POSTGRES_PASSWORD=postgres \
    -e POSTGRES_HOST_AUTH_METHOD=trust \
    -e POSTGRES_DB=postgres \
    "${image}" \
    -c shared_preload_libraries=timescaledb,citus \
    -c citus.cohabit_extensions=timescaledb \
    -c wal_level=replica \
    -c max_wal_senders=10 \
    -c max_replication_slots=10 >/dev/null
}

wait_for_postgres_ready() {
  local container="$1"
  for _ in $(seq 1 90); do
    if docker exec "${container}" pg_isready -U postgres >/dev/null 2>&1 \
      && docker exec "${container}" psql -U postgres -d postgres -Atqc 'SELECT 1' >/dev/null 2>&1; then
      return 0
    fi
    sleep 1
  done
  docker logs "${container}" >&2 || true
  echo "${container} did not become ready" >&2
  exit 1
}

ensure_citus_extension() {
  local container="$1"
  local output=""
  docker exec "${container}" bash -c "echo 'host replication all all trust' >> \$PGDATA/pg_hba.conf && pg_ctl -D \$PGDATA reload" >/dev/null
  if ! output="$(docker exec "${container}" psql -U postgres -d postgres -v ON_ERROR_STOP=1 -q -c 'CREATE EXTENSION IF NOT EXISTS citus;' 2>&1)"; then
    if ! grep -Eq 'citus_setup_ssl|server closed the connection|connection to server was lost|database system is shutting down' <<<"${output}"; then
      printf '%s\n' "${output}" >&2
      exit 1
    fi
    wait_for_postgres_ready "${container}"
    docker exec "${container}" psql -U postgres -d postgres -v ON_ERROR_STOP=1 -q -c 'CREATE EXTENSION IF NOT EXISTS citus;' >/dev/null
  fi
}

run_primary_container "${coordinator}"
run_primary_container "${primary_worker}"
for container in "${coordinator}" "${primary_worker}"; do
  wait_for_postgres_ready "${container}"
  ensure_citus_extension "${container}"
done

docker exec -i "${coordinator}" psql -v ON_ERROR_STOP=1 -U postgres -d postgres -q <<SQL
SELECT citus_set_coordinator_host('${coordinator}', 5432);
SQL

setup_sql="$(cargo run -q -p ai_blaise_citus_companion --bin companion_contracts -- run-clone-node-setup-sql-canonical)"
setup_output="$(docker exec -i "${coordinator}" psql -v ON_ERROR_STOP=1 -U postgres -d postgres -X -q -AtF $'\t' \
  -v s3_primary_worker="${primary_worker}" <<SQL
${setup_sql}
SQL
)"

setup_file="$(mktemp)"
printf '%s\n' "${setup_output}" > "${setup_file}"
python3 - "${setup_file}" <<'PY_SETUP'
from pathlib import Path
import sys
values = {}
for line in Path(sys.argv[1]).read_text().splitlines():
    parts = line.split("\t")
    if len(parts) >= 2:
        values[parts[0]] = parts[1]
required = {
    "s3_primary_nodeid": None,
    "s3_rows_before_clone": "20",
    "s3_sum_before_clone": "5060",
    "s3_placements_before_clone": "4",
    "s3_expected_rows": "20",
    "s3_expected_sum": "5060",
}
missing = [key for key in required if key not in values]
if missing:
    raise SystemExit(f"missing S3 setup markers {missing}: {values}")
for key, expected in required.items():
    if expected is not None and values[key] != expected:
        raise SystemExit(f"{key} expected {expected}, got {values[key]}")
if int(values["s3_primary_nodeid"]) <= 0:
    raise SystemExit(f"invalid primary node id {values['s3_primary_nodeid']}")
PY_SETUP
rm -f "${setup_file}"

docker run -d \
  --name "${clone_worker}" \
  --network "${network}" \
  -e POSTGRES_PASSWORD=postgres \
  -e POSTGRES_HOST_AUTH_METHOD=trust \
  -e POSTGRES_DB=postgres \
  "${image}" \
  bash -ceu "rm -rf \"\$PGDATA\"/*; until pg_basebackup -h '${primary_worker}' -U postgres -D \"\$PGDATA\" -R -X stream; do sleep 1; done; exec postgres -D \"\$PGDATA\" -c shared_preload_libraries=timescaledb,citus -c citus.cohabit_extensions=timescaledb -c hot_standby=on" >/dev/null
wait_for_postgres_ready "${clone_worker}"

clone_recovery_before="$(docker exec "${clone_worker}" psql -U postgres -d postgres -Atqc 'SELECT pg_is_in_recovery();')"
if [[ "${clone_recovery_before}" != "t" ]]; then
  echo "expected clone worker to be in recovery before promotion, got ${clone_recovery_before}" >&2
  exit 1
fi

promote_sql="$(cargo run -q -p ai_blaise_citus_companion --bin companion_contracts -- run-clone-node-promote-sql-canonical)"
promote_output="$(docker exec -i "${coordinator}" psql -v ON_ERROR_STOP=1 -U postgres -d postgres -X -q -AtF $'\t' \
  -v s3_primary_worker="${primary_worker}" \
  -v s3_clone_worker="${clone_worker}" <<SQL
${promote_sql}
SQL
)"

clone_recovery_after="$(docker exec "${clone_worker}" psql -U postgres -d postgres -Atqc 'SELECT pg_is_in_recovery();')"
if [[ "${clone_recovery_after}" != "f" ]]; then
  echo "expected clone worker promoted out of recovery, got ${clone_recovery_after}" >&2
  exit 1
fi

promote_file="$(mktemp)"
printf '%s\n' "${promote_output}" > "${promote_file}"
python3 - "${promote_file}" <<'PY_PROMOTE'
from pathlib import Path
import sys
values = {}
for line in Path(sys.argv[1]).read_text().splitlines():
    parts = line.split("\t")
    if len(parts) >= 2:
        values[parts[0]] = parts[1]
required = [
    "s3_clone_nodeid",
    "s3_rows_before_promote",
    "s3_sum_before_promote",
    "s3_clone_role_before_promote",
    "s3_clone_active_before_promote",
    "s3_promote_clone_and_rebalance_executed",
    "s3_rows_after_promote",
    "s3_sum_after_promote",
    "s3_clone_role_after_promote",
    "s3_clone_active_after_promote",
    "s3_clone_should_have_shards_after_promote",
    "s3_clone_shard_placements_after",
    "s3_primary_shard_placements_after",
    "kubernetes_clone_orchestration_exercised",
    "csi_snapshot_exercised",
    "automatic_capacity_policy_exercised",
    "production_traffic_cutover_exercised",
]
missing = [key for key in required if key not in values]
if missing:
    raise SystemExit(f"missing S3 promote markers {missing}: {values}")
if int(values["s3_clone_nodeid"]) <= 0:
    raise SystemExit("clone node id must be positive")
expected_pairs = {
    "s3_rows_before_promote": "20",
    "s3_sum_before_promote": "5060",
    "s3_clone_role_before_promote": "unavailable",
    "s3_clone_active_before_promote": "false",
    "s3_promote_clone_and_rebalance_executed": "true",
    "s3_rows_after_promote": "20",
    "s3_sum_after_promote": "5060",
    "s3_clone_role_after_promote": "primary",
    "s3_clone_active_after_promote": "true",
    "s3_clone_should_have_shards_after_promote": "true",
    "kubernetes_clone_orchestration_exercised": "false",
    "csi_snapshot_exercised": "false",
    "automatic_capacity_policy_exercised": "false",
    "production_traffic_cutover_exercised": "false",
}
for key, expected in expected_pairs.items():
    if values[key] != expected:
        raise SystemExit(f"{key} expected {expected}, got {values[key]}")
clone_placements = int(values["s3_clone_shard_placements_after"])
primary_placements = int(values["s3_primary_shard_placements_after"])
if clone_placements <= 0:
    raise SystemExit(f"expected promoted clone to own placements, got {clone_placements}")
if primary_placements <= 0:
    raise SystemExit(f"expected original primary to retain placements, got {primary_placements}")
if clone_placements + primary_placements != 4:
    raise SystemExit(f"expected four placements after promotion, got primary={primary_placements} clone={clone_placements}")
PY_PROMOTE
rm -f "${promote_file}"

clone_nodeid="$(printf '%s\n' "${promote_output}" | awk -F '\t' '$1 == "s3_clone_nodeid" {print $2; exit}')"
clone_placements="$(printf '%s\n' "${promote_output}" | awk -F '\t' '$1 == "s3_clone_shard_placements_after" {print $2; exit}')"
primary_placements="$(printf '%s\n' "${promote_output}" | awk -F '\t' '$1 == "s3_primary_shard_placements_after" {print $2; exit}')"

printf 'clone_node_live=passed\n'
printf 'pg_basebackup_clone_in_recovery_before=true\n'
printf 'citus_add_clone_node_executed=true\n'
printf 'citus_promote_clone_and_rebalance_executed=true\n'
printf 'pg_promote_clone_recovery_after=false\n'
printf 'clone_nodeid=%s\n' "${clone_nodeid}"
printf 'clone_rows_preserved=20\n'
printf 'clone_sum_preserved=5060\n'
printf 'clone_role_after_promote=primary\n'
printf 'clone_active_after_promote=true\n'
printf 'clone_should_have_shards_after_promote=true\n'
printf 'clone_shard_placements_after=%s\n' "${clone_placements}"
printf 'primary_shard_placements_after=%s\n' "${primary_placements}"
printf 'kubernetes_clone_orchestration_exercised=false\n'
printf 'csi_snapshot_exercised=false\n'
printf 'automatic_capacity_policy_exercised=false\n'
printf 'production_traffic_cutover_exercised=false\n'
printf 'clone_node_live\tpassed\n'
