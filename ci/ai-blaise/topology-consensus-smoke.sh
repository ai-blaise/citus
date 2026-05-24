#!/usr/bin/env bash
# Focused topology/consensus hardening smoke for FEATURE: S4/S5/S9/MR6.
#
# This smoke proves the bounded alpha hardening surface: coordinator-less
# topology admission requires a pool entry point, Raft placement plans reject
# unsafe member references, and closed-timestamp follower reads fail closed
# when AS OF is newer than the runtime closed timestamp.

set -euo pipefail

if [[ -f "${HOME}/.cargo/env" ]]; then
  # Keep direct VM invocations aligned with CI shells that already have cargo.
  source "${HOME}/.cargo/env"
fi

repo_root="$(git rev-parse --show-toplevel)"
cd "${repo_root}"

echo "==> topology-consensus-smoke: raft failover canonical"
raft_plan="$(cargo run -q -p ai_blaise_citus_sidecar_raft -- run-canonical)"
echo "${raft_plan}"
raft_row="$(printf '%s\n' "${raft_plan}" | sed -n '2p')"
if [[ "${raft_row}" != *$'\tpromote\tworker-c\torders-2' ]]; then
  echo "topology-consensus-smoke: expected failover promotion to worker-c/orders-2" >&2
  exit 1
fi
if [[ "${raft_row}" != *$'orders-sg\t7\tworker-a\t2\tworker-b,worker-c'* ]]; then
  echo "topology-consensus-smoke: expected quorum-sized orders-sg canonical plan" >&2
  exit 1
fi

echo "==> topology-consensus-smoke: hlc runtime canonical"
hlc_runtime="$(cargo run -q -p ai_blaise_citus_sidecar_hlc -- run-runtime-canonical)"
echo "${hlc_runtime}"
hlc_row="$(printf '%s\n' "${hlc_runtime}" | sed -n '2p')"
IFS=$'\t' read -r shard_group local_physical _ closed_physical _ max_offset max_staleness replica_count peers <<< "${hlc_row}"
if [[ "${shard_group}" != "orders-sg" || "${replica_count}" != "3" ]]; then
  echo "topology-consensus-smoke: unexpected HLC runtime shard group or replica count" >&2
  exit 1
fi
if (( closed_physical > local_physical )); then
  echo "topology-consensus-smoke: closed timestamp moved ahead of local clock" >&2
  exit 1
fi
if [[ "${max_offset}" != "500" || "${max_staleness}" != "5000" || "${peers}" != *"worker-b="* || "${peers}" != *"worker-c="* ]]; then
  echo "topology-consensus-smoke: expected deterministic HLC offset/staleness/peer evidence" >&2
  exit 1
fi

echo "==> topology-consensus-smoke: focused cargo tests"
cargo test -q -p ai_blaise_citus_sidecar_raft --all-targets
cargo test -q -p ai_blaise_citus_sidecar_hlc --all-targets
cargo test -q -p ai_blaise_citus_operator citus_cluster --lib

echo "topology-consensus-smoke passed"
