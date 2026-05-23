#!/usr/bin/env bash
# 3-node Raft round-trip smoke for FEATURE: S5.
#
# Drives the in-process sidecar runtime through one election + one proposal
# and verifies that every voter committed the entry. The runtime canonical
# runner emits a deterministic TSV; the smoke then re-runs the integration
# test that exercises a multi-message round trip.

set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "${repo_root}"

echo "==> sidecar-raft-smoke: run-runtime-canonical"
canonical_output=$(cargo run -q -p ai_blaise_citus_sidecar_raft -- run-runtime-canonical)
echo "${canonical_output}"

# Validate the TSV header carries the runtime fields the production
# audit reads.
expected_header=$'elected_leader\tterm\tcommitted_index\tcommitted_payload\tcommit_indices\tlast_log_indices'
actual_header=$(printf '%s\n' "${canonical_output}" | head -n 1)
if [[ "${actual_header}" != "${expected_header}" ]]; then
  echo "sidecar-raft-smoke: header mismatch" >&2
  echo "  expected: ${expected_header}" >&2
  echo "  actual:   ${actual_header}" >&2
  exit 1
fi

# Verify the canonical leader, payload, and majority commit.
data_row=$(printf '%s\n' "${canonical_output}" | sed -n '2p')
if [[ "${data_row}" != *"worker-a"* ]]; then
  echo "sidecar-raft-smoke: expected worker-a leader" >&2
  exit 1
fi
if [[ "${data_row}" != *"shard-placement-canonical"* ]]; then
  echo "sidecar-raft-smoke: expected canonical placement payload" >&2
  exit 1
fi
if [[ "${data_row}" != *"worker-a=1"*"worker-b=1"*"worker-c=1"* ]]; then
  echo "sidecar-raft-smoke: every voter must commit at index 1" >&2
  exit 1
fi

echo "==> sidecar-raft-smoke: run-durable-canonical"
durable_output=$(cargo run -q -p ai_blaise_citus_sidecar_raft -- run-durable-canonical)
echo "${durable_output}"
durable_header=$(printf '%s\n' "${durable_output}" | head -n 1)
expected_durable_header=$'appended_entries\treplayed_entries\tsnapshot_index\tsnapshot_term\tlog_path\tsnapshot_path'
if [[ "${durable_header}" != "${expected_durable_header}" ]]; then
  echo "sidecar-raft-smoke: durable header mismatch" >&2
  exit 1
fi
durable_row=$(printf '%s\n' "${durable_output}" | sed -n '2p')
if [[ "${durable_row}" != $'2\t2\t2\t1'* ]]; then
  echo "sidecar-raft-smoke: durable log/snapshot round trip did not replay expected watermark" >&2
  exit 1
fi

echo "==> sidecar-raft-smoke: cargo test integration round-trip"
cargo test -p ai_blaise_citus_sidecar_raft --test raft_round_trip -- --nocapture

echo "sidecar-raft-smoke passed"
