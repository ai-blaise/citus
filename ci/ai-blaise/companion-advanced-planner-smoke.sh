#!/usr/bin/env bash
set -euo pipefail

export PATH="$HOME/.cargo/bin:$PATH"

# FEATURE: T4
# FEATURE: T10
# FEATURE: T11
# FEATURE: T13
# FEATURE: T14

cargo test -q -p ai_blaise_citus_companion advanced_planner

expected_summary_header=$'surfaces\tlookup_surfaces\tlookup_min_partitions\tmax_batch_rows\tdistsql_worker_tasks\ttransaction_state_surfaces\ttransaction_shard_budget\tpolicy_surfaces\tpolicy_required_inputs\tstorage_domains\tresearch_guards'
expected_summary_row=$'27\t1\t1\t4096\t2\t2\t256\t19\t40\t1\t2'
summary_output="$(cargo run -q -p ai_blaise_citus_companion --bin companion_contracts -- run-advanced-planner-canonical)"

if ! printf '%s\n' "${summary_output}" | grep -Fqx "${expected_summary_header}"; then
  echo "advanced planner canonical header mismatch" >&2
  printf '%s\n' "${summary_output}" >&2
  exit 1
fi

if ! printf '%s\n' "${summary_output}" | grep -Fqx "${expected_summary_row}"; then
  echo "advanced planner canonical row mismatch" >&2
  printf '%s\n' "${summary_output}" >&2
  exit 1
fi

expected_runtime_header=$'scenarios\tcovered_features\tcontract_checks\tfail_closed_checks\tlive_execution_claims\tpatch_smoke_boundaries\tplan_only_boundaries\tdeterministic_boundaries\tresearch_guard_boundaries'
expected_runtime_row=$'27\t27\t96\t5\t0\t1\t4\t20\t2'
runtime_output="$(cargo run -q -p ai_blaise_citus_companion --bin companion_contracts -- run-advanced-planner-runtime-canonical)"

if ! printf '%s\n' "${runtime_output}" | grep -Fqx "${expected_runtime_header}"; then
  echo "advanced planner runtime header mismatch" >&2
  printf '%s\n' "${runtime_output}" >&2
  exit 1
fi

if ! printf '%s\n' "${runtime_output}" | grep -Fqx "${expected_runtime_row}"; then
  echo "advanced planner runtime row mismatch" >&2
  printf '%s\n' "${runtime_output}" >&2
  exit 1
fi

printf 'companion_advanced_planner_smoke\tsurfaces=27\truntime_scenarios=27\tfail_closed_checks=5\tlive_execution_claims=0\n'
