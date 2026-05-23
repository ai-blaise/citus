#!/usr/bin/env bash
# Parallel-commit smoke for FEATURE: T5.
#
# Drives the in-process txn-status runtime through a multi-shard staging +
# finalize round-trip, plus a microbenchmark that proves parallel-commit
# latency is at most 0.6x of the 2PC baseline (gate 3, latency ≤40% lower
# than distributed 2PC).

set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "${repo_root}"

echo "==> parallel-commits-smoke: run-runtime-canonical"
canonical_output=$(cargo run -q -p ai_blaise_citus_sidecar_txn_status -- run-runtime-canonical)
echo "${canonical_output}"

expected_header=$'raft_group\tvoters\tmax_staging_ms\ttxn_id\tstaged_status\tstaged_raft_index\tfinalize_decision\tfinalized_status\tfinalized_raft_index\tintent_count'
actual_header=$(printf '%s\n' "${canonical_output}" | head -n 1)
if [[ "${actual_header}" != "${expected_header}" ]]; then
  echo "parallel-commits-smoke: runtime header mismatch" >&2
  echo "  expected: ${expected_header}" >&2
  echo "  actual:   ${actual_header}" >&2
  exit 1
fi

data_row=$(printf '%s\n' "${canonical_output}" | sed -n '2p')
if [[ "${data_row}" != *"commit"* ]]; then
  echo "parallel-commits-smoke: expected commit decision" >&2
  exit 1
fi
if [[ "${data_row}" != *"committed"* ]]; then
  echo "parallel-commits-smoke: expected committed final status" >&2
  exit 1
fi

echo "==> parallel-commits-smoke: run-parallel-commit-microbench 5"
microbench_output=$(cargo run -q -p ai_blaise_citus_sidecar_txn_status -- run-parallel-commit-microbench 5)
echo "${microbench_output}"

micro_row=$(printf '%s\n' "${microbench_output}" | sed -n '2p')
two_phase=$(printf '%s\n' "${micro_row}" | cut -f2)
parallel=$(printf '%s\n' "${micro_row}" | cut -f3)
speedup=$(printf '%s\n' "${micro_row}" | cut -f4)

if [[ -z "${two_phase}" || -z "${parallel}" || -z "${speedup}" ]]; then
  echo "parallel-commits-smoke: malformed microbench row" >&2
  exit 1
fi

# Gate 3 latency check: parallel_commit / two_phase_commit <= 0.6.
ratio_under_threshold=$(awk -v p="${parallel}" -v t="${two_phase}" 'BEGIN {
  if (t == 0) { print "0"; exit }
  ratio = p / t;
  if (ratio <= 0.6) { print "1" } else { print "0" }
}')

if [[ "${ratio_under_threshold}" != "1" ]]; then
  echo "parallel-commits-smoke: latency ratio ${parallel}/${two_phase} exceeds 0.6x threshold" >&2
  exit 1
fi

echo "==> parallel-commits-smoke: cargo test integration round-trip"
cargo test -p ai_blaise_citus_sidecar_txn_status --test parallel_commit_round_trip -- --nocapture

echo "parallel-commits-smoke passed (speedup=${speedup}, latency ratio=$(awk -v p="${parallel}" -v t="${two_phase}" 'BEGIN { print p/t }'))"
