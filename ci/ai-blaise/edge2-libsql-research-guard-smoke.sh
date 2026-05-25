#!/usr/bin/env bash
set -euo pipefail

export PATH="$HOME/.cargo/bin:$PATH"

# FEATURE: Edge2

cargo test -q -p ai_blaise_citus_companion edge2_libsql_research_guard_is_fail_closed

expected_header=$'feature_id	guard_status	decision_record	blocked_integration	promotion_evidence	forbidden_claims	live_execution_claims	replication_adapter_claimed	workload_isolation_claimed	production_query_routing_claimed'
expected_row=$'Edge2	fail-closed	docs/ai-blaise/ADR/0009-libsql-read-tier-research-guard.md	libsql production read tier	5	4	0	false	false	false'
output="$(cargo run -q -p ai_blaise_citus_companion --bin companion_contracts -- run-libsql-read-tier-guard-canonical)"

if ! printf '%s
' "${output}" | grep -Fqx "${expected_header}"; then
  echo "Edge2 libsql research guard header mismatch" >&2
  printf '%s
' "${output}" >&2
  exit 1
fi

if ! printf '%s
' "${output}" | grep -Fqx "${expected_row}"; then
  echo "Edge2 libsql research guard row mismatch" >&2
  printf '%s
' "${output}" >&2
  exit 1
fi

advanced_output="$(cargo run -q -p ai_blaise_citus_companion --bin companion_contracts -- run-advanced-planner-runtime-canonical)"
if ! printf '%s
' "${advanced_output}" | grep -Fqx $'27	27	96	5	0	1	4	20	2'; then
  echo "advanced planner runtime guard boundary mismatch" >&2
  printf '%s
' "${advanced_output}" >&2
  exit 1
fi

printf 'edge2_libsql_research_guard_smoke	feature_id=Edge2	guard_status=fail-closed	live_execution_claims=0	replication_adapter_claimed=false	workload_isolation_claimed=false	production_query_routing_claimed=false
'
