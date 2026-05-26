#!/usr/bin/env bash
set -euo pipefail

# Live integration smoke for the 2026-05-26 upstream rebase: five citusdata/citus
# main commits folded onto bootstrap-v2 between base 4d54b11bb and HEAD dee8ec140.
#
#   #8587 10c3a8be2  Enforce object ownership for more citus-internal UDFs
#   #8497 10e87b755  Fix wrong results for NOT (x IS DISTINCT FROM y) with recursive planning
#   #8498 810e7fbb9  Fix type mismatch when COLLATE is used with type cast in distributed queries
#   #8593 45e19d4f9  Short-circuit list sort for 0 or 1 elements
#   #8592 dee8ec140  docs: fix minor grammar and wording issues in README
#
# Mode 1 (always): source-level fingerprint verification of each fix, the
# regression-test files it adds, and the schedule wiring. Proves the cherry-picks
# are in the tree and that the upstream-regression harness will exercise them
# when the existing sql-extension-smoke flow builds bundle1-final-light from the
# in-tree source.
#
# Mode 2 (REQUIRE_DOCKER=1): boots citusdata/citus:14.0.0-pg17 (the latest
# stable upstream Citus release tag, pre-dates all five cherry-picks - see
# `git log v14.0.0..upstream/main` for the gap), runs distributed-query
# patterns from the new regression tests, and records whether each pattern
# still triggers the upstream bug as deployed today.
#
# Mode 3 (REQUIRE_DOCKER=1 RUN_BUNDLE1_FIX_VERIFICATION=1): rebuilds the
# bundle1-final-light image from the in-tree cherry-picked Citus source (i.e.
# our fork after this rebase), boots it on a loopback port, and runs the same
# bug-reproducer queries. Each one must now return the correct result, proving
# the cherry-picks integrate end-to-end with our build flow + customizations.
# This is gated behind a separate env var because the bundle1 source-build
# takes ~5-10 minutes even on a 48-core host.

repo_root="$(git rev-parse --show-toplevel)"
cd "${repo_root}"

artifacts_dir="${repo_root}/artifacts"
mkdir -p "${artifacts_dir}"
evidence_tsv="${artifacts_dir}/upstream-rebase-2026-05-25-integration-evidence.tsv"

require_docker="${REQUIRE_DOCKER:-0}"
upstream_image="${UPSTREAM_REBASE_SMOKE_IMAGE:-citusdata/citus:14.0.0-pg17}"

log() { printf '%s %s\n' "[upstream-rebase-2026-05-25]" "$*" >&2; }
fail() { log "FAIL: $*"; exit 1; }

# --- Mode 1: source-level fingerprint verification --------------------------

log "phase 1: source-level fingerprint verification"

expected_base="dee8ec140aff84d8769bdcd859c39c379180fe06"
declared_base="$(awk -F= '/^base=/{print $2}' UPSTREAM_REBASE_BASE)"
declared_date="$(awk -F= '/^capturedAt=/{print $2}' UPSTREAM_REBASE_BASE)"
if [ "${declared_base}" != "${expected_base}" ]; then
  fail "UPSTREAM_REBASE_BASE base=${declared_base} expected=${expected_base}"
fi
if [ "${declared_date}" != "2026-05-26" ]; then
  fail "UPSTREAM_REBASE_BASE capturedAt=${declared_date} expected=2026-05-26"
fi
log "ok: UPSTREAM_REBASE_BASE base=${expected_base} capturedAt=${declared_date}"

if ! grep -q "Status snapshot: 2026-05-26" docs/ai-blaise/UPSTREAM_SYNC.md; then
  fail "UPSTREAM_SYNC.md missing 2026-05-26 status snapshot"
fi
log "ok: UPSTREAM_SYNC.md snapshot date is 2026-05-26"

# #8593 SortList short-circuit: comment + early return for arraySize <= 1
if ! grep -q "lists with 0 or 1 elements are already sorted" \
    src/backend/distributed/utils/listutils.c; then
  fail "SortList short-circuit comment missing from listutils.c"
fi
if ! grep -q "arraySize <= 1" src/backend/distributed/utils/listutils.c; then
  fail "SortList short-circuit guard arraySize <= 1 missing from listutils.c"
fi
log "ok: #8593 SortList short-circuit present in listutils.c"

# #8498 COLLATE+typecast: DEFAULT_COLLATION_OID guard in RelabelTypeToCollateExpr
if ! grep -q "DEFAULT_COLLATION_OID prevents re-detection by IsImproperForDeparseRelabelTypeNode" \
    src/backend/distributed/planner/multi_physical_planner.c; then
  fail "COLLATE+typecast DEFAULT_COLLATION_OID fix missing from multi_physical_planner.c"
fi
if ! grep -q "RelabelTypeToCollateExpr converts RelabelType nodes for deparsing" \
    src/backend/distributed/planner/multi_physical_planner.c; then
  fail "RelabelTypeToCollateExpr documentation block missing from multi_physical_planner.c"
fi
if [ ! -f src/test/regress/sql/distributed_collations.sql ]; then
  fail "distributed_collations.sql missing"
fi
if ! grep -q "issue #8469" src/test/regress/sql/distributed_collations.sql; then
  fail "distributed_collations.sql missing the #8469 test block"
fi
log "ok: #8498 COLLATE+typecast fix present in multi_physical_planner.c + regression"

# #8497 NOT (x IS DISTINCT FROM y) recursive planning: requiredAttrNumbers fix
if ! grep -q "requiredAttrNumbers the subquery projects them as NULL" \
    src/backend/distributed/planner/recursive_planning.c; then
  fail "NOT IS DISTINCT FROM recursive-planning fix missing from recursive_planning.c"
fi
if [ ! -f src/test/regress/sql/issue_8468.sql ]; then
  fail "issue_8468.sql regression test missing"
fi
if [ ! -f src/test/regress/expected/issue_8468.out ]; then
  fail "issue_8468.out expected output missing"
fi
if ! grep -q "^test: issue_8468" src/test/regress/multi_schedule; then
  fail "issue_8468 not wired into multi_schedule"
fi
log "ok: #8497 NOT (IS DISTINCT FROM) fix present in recursive_planning.c + regression + schedule"

# #8587 object-ownership enforcement
if ! grep -q "EnsureFunctionOwner" src/backend/distributed/metadata/metadata_utility.c; then
  fail "EnsureFunctionOwner helper missing from metadata_utility.c"
fi
if ! grep -q "EnsureSchemaOwner" src/backend/distributed/metadata/metadata_sync.c; then
  fail "EnsureSchemaOwner call missing from metadata_sync.c"
fi
if [ ! -f src/test/regress/sql/metadata_sync_helpers.sql ]; then
  fail "metadata_sync_helpers.sql regression test missing"
fi
if ! grep -q "EnsureFunctionOwner\|EnsureSchemaOwner" \
    src/include/distributed/metadata_utility.h; then
  fail "Ensure*Owner declarations missing from metadata_utility.h"
fi
log "ok: #8587 ownership enforcement present in metadata_sync.c + metadata_utility.c + header + regression"

# #8592 README grammar fixes
if ! grep -q "you can use Citus" README.md; then
  fail "README grammar fix 'you can use Citus' missing"
fi
if ! grep -q "^- \*\*Reference tables\*\*" README.md; then
  fail "README grammar fix '**Reference tables**' missing"
fi
log "ok: #8592 README grammar fixes present"

# --- shared helper: boot a single-coordinator + 1-worker Citus cluster ------

network=""
coord_container=""
worker_container=""

cluster_cleanup() {
  if [ -n "${worker_container}" ]; then
    docker rm -f "${worker_container}" >/dev/null 2>&1 || true
  fi
  if [ -n "${coord_container}" ]; then
    docker rm -f "${coord_container}" >/dev/null 2>&1 || true
  fi
  if [ -n "${network}" ]; then
    docker network rm "${network}" >/dev/null 2>&1 || true
  fi
}
trap cluster_cleanup EXIT

boot_citus_cluster() {
  local image="$1"
  local tag="$2"
  local shared_preload="$3"

  network="upstream-rebase-net-${tag}-$$"
  coord_container="upstream-rebase-coord-${tag}-$$"
  worker_container="upstream-rebase-worker-${tag}-$$"

  docker network create "${network}" >/dev/null

  # PGSODIUM_KEY: bundle1 image preloads pgsodium which is fail-closed on a
  # missing key. Provide a deterministic dev-only 64-hex key so initdb passes
  # without an external secret mount. The upstream Citus image ignores this.
  local pgsodium_dev_key="0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
  docker run -d --name "${coord_container}" --network "${network}" --network-alias coord \
    -e POSTGRES_PASSWORD=postgres \
    -e POSTGRES_HOST_AUTH_METHOD=trust \
    -e PGSODIUM_KEY="${pgsodium_dev_key}" \
    "${image}" \
    postgres -c "shared_preload_libraries=${shared_preload}" >/dev/null
  docker run -d --name "${worker_container}" --network "${network}" --network-alias worker \
    -e POSTGRES_PASSWORD=postgres \
    -e POSTGRES_HOST_AUTH_METHOD=trust \
    -e PGSODIUM_KEY="${pgsodium_dev_key}" \
    "${image}" \
    postgres -c "shared_preload_libraries=${shared_preload}" >/dev/null

  # Wait for actual SQL connectivity, not just pg_isready. Bundle1 image
  # restarts postgres mid-initdb (to apply config changes), so an early
  # pg_isready can succeed during the initial startup right before the
  # restart — a SELECT 1 only succeeds when the final postgres is serving.
  local ready_coord=0 ready_worker=0
  for _ in $(seq 1 180); do
    if [ "${ready_coord}" = 0 ] \
        && docker exec "${coord_container}" psql -U postgres -d postgres -tA \
             -c "SELECT 1;" >/dev/null 2>&1; then
      ready_coord=1
    fi
    if [ "${ready_worker}" = 0 ] \
        && docker exec "${worker_container}" psql -U postgres -d postgres -tA \
             -c "SELECT 1;" >/dev/null 2>&1; then
      ready_worker=1
    fi
    if [ "${ready_coord}" = 1 ] && [ "${ready_worker}" = 1 ]; then
      break
    fi
    sleep 1
  done
  [ "${ready_coord}" = 1 ] || fail "${image} coordinator did not become ready"
  [ "${ready_worker}" = 1 ] || fail "${image} worker did not become ready"

  for c in "${coord_container}" "${worker_container}"; do
    docker exec "${c}" psql -U postgres -d postgres -v ON_ERROR_STOP=1 \
      -c "CREATE EXTENSION IF NOT EXISTS citus;" >/dev/null
  done
  docker exec "${coord_container}" psql -U postgres -d postgres -v ON_ERROR_STOP=1 -c "
    SELECT citus_set_coordinator_host('coord', 5432);
    SELECT master_add_node('worker', 5432);
  " >/dev/null
}

run_bug_reproducers() {
  local target_label="$1"
  local _bug_collate _bug_not_distinct _bug_ownership

  # pgsodium installs an event trigger trg_mask_update that references the
  # pgsodium.enable_event_trigger GUC which is only set by the pgsodium-
  # preloaded image. With pgsodium SQL-loaded but not preloaded, the trigger
  # fires on every CREATE TABLE and errors. Drop it so DDL doesn't choke.
  docker exec "${coord_container}" psql -U postgres -d postgres -c \
    "DROP EXTENSION IF EXISTS pgsodium CASCADE;" >/dev/null 2>&1 || true
  docker exec "${worker_container}" psql -U postgres -d postgres -c \
    "DROP EXTENSION IF EXISTS pgsodium CASCADE;" >/dev/null 2>&1 || true

  docker exec "${coord_container}" psql -U postgres -d postgres -v ON_ERROR_STOP=1 -c "
    DROP TABLE IF EXISTS r_collate, r_t5, r_t2_child, r_t1_parent, r_t0, r_owned CASCADE;
    DROP ROLE IF EXISTS r_other;
    SET citus.shard_count = 4;
    CREATE TABLE r_collate (c0 inet, c1 inet);
    SELECT create_distributed_table('r_collate', 'c0');
    INSERT INTO r_collate(c1, c0) VALUES
      ('144.150.228.243', '230.194.119.117'),
      ('22.171.214.19',   '138.53.199.60'),
      ('14.25.58.22',     '103.167.89.59');
  " >/dev/null

  # #8498 reproducer: GROUP BY (...::VARCHAR COLLATE \"C\") on distributed
  #   table. Pre-fix: 'attribute N of type record has wrong type'.
  if docker exec "${coord_container}" psql -U postgres -d postgres -v ON_ERROR_STOP=1 -c "
        SELECT SUM(agg0)
        FROM (
            SELECT ALL SUM(0.5) AS agg0
            FROM ONLY r_collate
            GROUP BY (((('fooText')||(r_collate.c1)))::VARCHAR COLLATE \"C\")
        ) sub;
      " >/dev/null 2>&1; then
    _bug_collate="behaves_correctly"
  else
    _bug_collate="bug_observed"
  fi

  # #8497 reproducer: TLP branch_true + branch_false must equal original.
  # Mirrors the upstream issue_8468.sql shape: distributed t5 + INHERITS parent
  # + local t0, LEFT OUTER JOIN that triggers recursive-planning wrap of the
  # local table whose restriction columns must be projected to evaluate the
  # outer WHERE correctly. Pre-fix: branch_false=0 (NULL-projection of the
  # restriction column makes NOT (0 IS DISTINCT FROM NULL) always FALSE).
  docker exec "${coord_container}" psql -U postgres -d postgres -v ON_ERROR_STOP=1 -c "
    CREATE TABLE r_t5(c0 float);
    SELECT create_distributed_table('r_t5', 'c0');
    INSERT INTO r_t5(c0) VALUES (0.009452163), (1.4691802E9), (0.005109378),
      (0.6941109), (0.7013781), (0.8670044), (-1.6739732E9), (-4.5730365E8);
    CREATE TABLE r_t1_parent(c0 float);
    CREATE TABLE r_t2_child(c0 float, c1 char(20), c2 decimal) INHERITS(r_t1_parent);
    INSERT INTO r_t1_parent(c0) VALUES(-1.6739732E9), (0), (0.32921866), ('-Infinity');
    INSERT INTO r_t2_child(c1, c2, c0) VALUES('', 0.19, 0), ('test', 0.33, 0.89),
      ('abc', 0.58, 0.68), ('', 0.18, 0.74), ('', 0.22, 0.71);
    CREATE TABLE r_t0(c0 float);
    INSERT INTO r_t0(c0) VALUES(-1.9619044E9), (0.18373421), (6.175733E8), (0.58579546);
  " >/dev/null
  local orig br_true br_false
  orig=$(docker exec "${coord_container}" psql -U postgres -d postgres -tA -c "
    SELECT count(*) FROM (
      SELECT r_t5.c0 FROM r_t1_parent, r_t0 LEFT OUTER JOIN r_t5 ON (True)
    ) sub;
  " | tr -d '[:space:]')
  br_true=$(docker exec "${coord_container}" psql -U postgres -d postgres -tA -c "
    SELECT count(*) FROM (
      SELECT r_t5.c0 FROM r_t1_parent, r_t0 LEFT OUTER JOIN r_t5 ON (True)
      WHERE (0::double precision IS DISTINCT FROM r_t1_parent.c0)
    ) sub;
  " | tr -d '[:space:]')
  br_false=$(docker exec "${coord_container}" psql -U postgres -d postgres -tA -c "
    SELECT count(*) FROM (
      SELECT r_t5.c0 FROM r_t1_parent, r_t0 LEFT OUTER JOIN r_t5 ON (True)
      WHERE NOT (0::double precision IS DISTINCT FROM r_t1_parent.c0)
    ) sub;
  " | tr -d '[:space:]')
  log "${target_label} #8497 TLP counts: original=${orig} true=${br_true} false=${br_false}"
  if [ "$((br_true + br_false))" -eq "${orig}" ]; then
    _bug_not_distinct="behaves_correctly"
  else
    _bug_not_distinct="bug_observed"
  fi

  # #8587 reproducer: non-owner attempts citus_internal_update_relation_colocation.
  # ROLE create + GRANT must be in separate transactions from the distributed
  # table create to avoid Citus parallel-op constraint.
  docker exec "${coord_container}" psql -U postgres -d postgres -v ON_ERROR_STOP=1 -c "
    CREATE TABLE r_owned(id int);
    SELECT create_distributed_table('r_owned', 'id');
  " >/dev/null
  docker exec "${coord_container}" psql -U postgres -d postgres -v ON_ERROR_STOP=1 -c "
    CREATE ROLE r_other LOGIN;
    GRANT USAGE ON SCHEMA pg_catalog TO r_other;
  " >/dev/null
  if docker exec "${coord_container}" psql -U postgres -d postgres -tA -v ON_ERROR_STOP=1 -c "
      SET ROLE r_other;
      SELECT pg_catalog.citus_internal_update_relation_colocation('r_owned'::regclass, 0);
    " >/dev/null 2>&1; then
    _bug_ownership="bug_observed"
  else
    _bug_ownership="behaves_correctly"
  fi

  printf '%s\t%s\t%s\n' "${_bug_collate}" "${_bug_not_distinct}" "${_bug_ownership}"
}

# --- Mode 2: live runtime exercise against pre-cherry-pick upstream Citus ---

bug_collate_upstream="skipped"
bug_not_distinct_upstream="skipped"
bug_ownership_upstream="skipped"
upstream_runtime_exercised="false"

bug_collate_fork="skipped"
bug_not_distinct_fork="skipped"
bug_ownership_fork="skipped"
fork_runtime_exercised="false"

if [ "${require_docker}" = "1" ]; then
  if ! command -v docker >/dev/null 2>&1; then
    fail "docker not on PATH but REQUIRE_DOCKER=1"
  fi

  upstream_runtime_exercised="true"
  log "phase 2: live runtime exercise against ${upstream_image} (pre-cherry-pick)"
  boot_citus_cluster "${upstream_image}" "upstream" "citus"
  read -r bug_collate_upstream bug_not_distinct_upstream bug_ownership_upstream \
    < <(run_bug_reproducers "upstream@${upstream_image}")
  log "#8498 COLLATE+typecast on ${upstream_image}: ${bug_collate_upstream}"
  log "#8497 NOT (IS DISTINCT FROM) on ${upstream_image}: ${bug_not_distinct_upstream}"
  log "#8587 ownership enforcement on ${upstream_image}: ${bug_ownership_upstream}"
  cluster_cleanup
  network=""; coord_container=""; worker_container=""
fi

# --- Mode 3: live runtime exercise against our cherry-picked bundle1 image --

if [ "${require_docker}" = "1" ] && [ "${RUN_BUNDLE1_FIX_VERIFICATION:-0}" = "1" ]; then
  fork_image="${FORK_BUNDLE1_IMAGE:-ai-blaise-citus-overlay:upstream-rebase-fix-verification}"
  log "phase 3: building bundle1-final-light from in-tree cherry-picked Citus"

  docker build \
    -f images/citus-pg-overlay/Dockerfile \
    --target bundle1-final-light \
    --build-arg PG_MAJOR=17 \
    --build-arg BASE_IMAGE=postgres:17-bookworm \
    -t "${fork_image}" \
    . >/dev/null

  log "phase 3: live runtime exercise against ${fork_image} (with cherry-picks)"
  # Bundle1 shared_preload set minus pgsodium (which gates the whole startup
  # on a real key file). pgsodium still gets created via the initdb SQL with
  # PGSODIUM_KEY env, which is sufficient for everything we test here.
  boot_citus_cluster "${fork_image}" "fork" \
    "citus,timescaledb,pgaudit,pgauditlogtofile,pg_cron,age,pg_failover_slots,pgnodemx"
  fork_runtime_exercised="true"
  read -r bug_collate_fork bug_not_distinct_fork bug_ownership_fork \
    < <(run_bug_reproducers "fork@${fork_image}")
  log "#8498 COLLATE+typecast on ${fork_image}: ${bug_collate_fork}"
  log "#8497 NOT (IS DISTINCT FROM) on ${fork_image}: ${bug_not_distinct_fork}"
  log "#8587 ownership enforcement on ${fork_image}: ${bug_ownership_fork}"

  # Each fix must produce the correct result on the cherry-picked image.
  if [ "${bug_collate_fork}" != "behaves_correctly" ]; then
    fail "#8498 COLLATE+typecast still buggy on cherry-picked bundle1 image"
  fi
  if [ "${bug_not_distinct_fork}" != "behaves_correctly" ]; then
    fail "#8497 NOT (IS DISTINCT FROM) still buggy on cherry-picked bundle1 image"
  fi
  if [ "${bug_ownership_fork}" != "behaves_correctly" ]; then
    fail "#8587 ownership enforcement still bypassable on cherry-picked bundle1 image"
  fi
fi

# --- emit evidence ----------------------------------------------------------

ts="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
{
  printf 'timestamp\tupstream_base\tupstream_head\tcommits_folded\tregression_files_landed\tschedule_wired\treadme_grammar_fixes\tupstream_runtime_exercised\tupstream_8498_collate\tupstream_8497_not_distinct\tupstream_8587_ownership\tfork_runtime_exercised\tfork_8498_collate\tfork_8497_not_distinct\tfork_8587_ownership\tevidence_boundary\n'
  printf '%s\t4d54b11bbab52f71b76c316432e878a1bc38206c\tdee8ec140aff84d8769bdcd859c39c379180fe06\t5\t3\ttrue\t2\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\tupstream-rebase-2026-05-25-integration\n' \
    "${ts}" \
    "${upstream_runtime_exercised}" "${bug_collate_upstream}" "${bug_not_distinct_upstream}" "${bug_ownership_upstream}" \
    "${fork_runtime_exercised}" "${bug_collate_fork}" "${bug_not_distinct_fork}" "${bug_ownership_fork}"
} > "${evidence_tsv}"

log "evidence row written to ${evidence_tsv}"
log "upstream-rebase-2026-05-25 integration smoke passed"
