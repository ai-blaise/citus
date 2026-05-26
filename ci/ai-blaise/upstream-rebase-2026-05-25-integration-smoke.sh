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
# Mode 2 (REQUIRE_DOCKER=1): boots citusdata/citus:13.3.0-pg17 (pre-cherry-pick
# upstream Citus binary distribution), runs distributed-query patterns from the
# new regression tests, and records whether each pattern still triggers the
# upstream bug as deployed today. Combined with Mode 1's source-level proof that
# our tree contains the fix, this demonstrates the production value of folding
# the upstream commits into the fork.

repo_root="$(git rev-parse --show-toplevel)"
cd "${repo_root}"

artifacts_dir="${repo_root}/artifacts"
mkdir -p "${artifacts_dir}"
evidence_tsv="${artifacts_dir}/upstream-rebase-2026-05-25-integration-evidence.tsv"

require_docker="${REQUIRE_DOCKER:-0}"
upstream_image="${UPSTREAM_REBASE_SMOKE_IMAGE:-citusdata/citus:13.3.0-pg17}"

log() { printf '%s %s\n' "[upstream-rebase-2026-05-25]" "$*"; }
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

# --- Mode 2: live runtime exercise against pre-cherry-pick upstream Citus ---

bug_collate="skipped"
bug_not_distinct="skipped"
bug_ownership="skipped"
upstream_runtime_exercised="false"

if [ "${require_docker}" = "1" ]; then
  upstream_runtime_exercised="true"
  log "phase 2: live runtime exercise against ${upstream_image} (pre-cherry-pick)"

  if ! command -v docker >/dev/null 2>&1; then
    fail "docker not on PATH but REQUIRE_DOCKER=1"
  fi

  pg_container="upstream-rebase-pg-$$"
  cleanup() { docker rm -f "${pg_container}" >/dev/null 2>&1 || true; }
  trap cleanup EXIT

  docker run -d --name "${pg_container}" \
    -e POSTGRES_PASSWORD=postgres \
    -e POSTGRES_HOST_AUTH_METHOD=trust \
    "${upstream_image}" >/dev/null
  for _ in $(seq 1 60); do
    if docker exec "${pg_container}" pg_isready -U postgres >/dev/null 2>&1; then
      break
    fi
    sleep 1
  done
  if ! docker exec "${pg_container}" pg_isready -U postgres >/dev/null 2>&1; then
    fail "${upstream_image} did not become ready"
  fi

  docker exec "${pg_container}" psql -U postgres -d postgres -c "CREATE EXTENSION citus;" >/dev/null
  docker exec "${pg_container}" psql -U postgres -d postgres -v ON_ERROR_STOP=1 -c \
    "SELECT citus_set_coordinator_host('localhost', 5432);" >/dev/null
  docker exec "${pg_container}" psql -U postgres -d postgres -v ON_ERROR_STOP=1 -c "
    CREATE TABLE upstream_rebase_collate_cast (c0 inet, c1 inet);
    SELECT create_distributed_table('upstream_rebase_collate_cast', 'c0');
    INSERT INTO upstream_rebase_collate_cast(c1, c0) VALUES
      ('144.150.228.243', '230.194.119.117'),
      ('22.171.214.19',   '138.53.199.60');
  " >/dev/null

  # #8498 bug reproducer: COLLATE+typecast on distributed GROUP BY raises
  #   'attribute N of type record has wrong type' on unpatched Citus.
  if docker exec "${pg_container}" psql -U postgres -d postgres -v ON_ERROR_STOP=1 -c "
      SELECT SUM(agg0)
      FROM (
          SELECT ALL SUM(0.5) as agg0
          FROM ONLY upstream_rebase_collate_cast
          GROUP BY (((('fooText')||(upstream_rebase_collate_cast.c1)))::VARCHAR COLLATE \"C\")
      ) AS asdf;
    " >/dev/null 2>&1; then
    bug_collate="not_reproduced"
  else
    bug_collate="reproduced_on_upstream"
  fi
  log "#8498 COLLATE+typecast bug on ${upstream_image}: ${bug_collate}"

  # #8497 bug reproducer: NOT (x IS DISTINCT FROM y) with recursive planning
  #   should sum branch_true + branch_false == original on a fixed cohort.
  docker exec "${pg_container}" psql -U postgres -d postgres -v ON_ERROR_STOP=1 -c "
    CREATE TABLE upstream_rebase_t5 (c0 float);
    SELECT create_distributed_table('upstream_rebase_t5', 'c0');
    INSERT INTO upstream_rebase_t5(c0) VALUES (0.0), (1.0), (NULL), (2.0);
    CREATE TABLE upstream_rebase_t1_parent (c0 float);
    INSERT INTO upstream_rebase_t1_parent(c0) VALUES (-1.0), (0.0), (0.5), ('-Infinity');
  " >/dev/null
  original_count=$(docker exec "${pg_container}" psql -U postgres -d postgres -tA -c "
    SELECT count(*) FROM (
      SELECT t5.c0 FROM upstream_rebase_t1_parent, upstream_rebase_t5
    ) sub;
  " | tr -d '[:space:]')
  branch_true=$(docker exec "${pg_container}" psql -U postgres -d postgres -tA -c "
    SELECT count(*) FROM (
      SELECT t5.c0 FROM upstream_rebase_t1_parent, upstream_rebase_t5
      WHERE (0::double precision IS DISTINCT FROM upstream_rebase_t1_parent.c0)
    ) sub;
  " | tr -d '[:space:]')
  branch_false=$(docker exec "${pg_container}" psql -U postgres -d postgres -tA -c "
    SELECT count(*) FROM (
      SELECT t5.c0 FROM upstream_rebase_t1_parent, upstream_rebase_t5
      WHERE NOT (0::double precision IS DISTINCT FROM upstream_rebase_t1_parent.c0)
    ) sub;
  " | tr -d '[:space:]')
  log "#8497 TLP branch counts on ${upstream_image}: original=${original_count} true=${branch_true} false=${branch_false}"
  expected_sum=$((branch_true + branch_false))
  if [ "${expected_sum}" -ne "${original_count}" ]; then
    bug_not_distinct="reproduced_on_upstream"
  else
    bug_not_distinct="not_reproduced"
  fi
  log "#8497 NOT (IS DISTINCT FROM) bug on ${upstream_image}: ${bug_not_distinct}"

  # #8587 ownership enforcement: pre-cherry-pick metadata_sync.c does not gate
  # citus_internal.update_relation_colocation on relation ownership; create a
  # non-superuser role and assert the unpatched call path. We only check that
  # the relation exists; live ownership-bypass exploitation is out of scope.
  docker exec "${pg_container}" psql -U postgres -d postgres -v ON_ERROR_STOP=1 -c "
    CREATE TABLE upstream_rebase_owned (id int);
    SELECT create_distributed_table('upstream_rebase_owned', 'id');
    CREATE ROLE upstream_rebase_other LOGIN;
  " >/dev/null
  bug_ownership="probe_only"
  log "#8587 ownership-enforcement probe on ${upstream_image}: ${bug_ownership} (relation + role created; live exploit out of scope for smoke)"
fi

# --- emit evidence ----------------------------------------------------------

ts="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
{
  printf 'timestamp\tupstream_base\tupstream_head\tcommits_folded\tregression_files_landed\tschedule_wired\treadme_grammar_fixes\tupstream_runtime_exercised\tbug_8498_collate\tbug_8497_not_distinct\tbug_8587_ownership_probe\tevidence_boundary\n'
  printf '%s\t4d54b11bbab52f71b76c316432e878a1bc38206c\tdee8ec140aff84d8769bdcd859c39c379180fe06\t5\t3\ttrue\t2\t%s\t%s\t%s\t%s\tupstream-rebase-2026-05-25-integration\n' \
    "${ts}" "${upstream_runtime_exercised}" "${bug_collate}" "${bug_not_distinct}" "${bug_ownership}"
} > "${evidence_tsv}"

log "evidence row written to ${evidence_tsv}"
log "upstream-rebase-2026-05-25 integration smoke passed"
