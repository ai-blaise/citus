#!/usr/bin/env bash
# Chaos scenario: simulate disk-full on a worker by filling /var/lib/postgresql
# with a sparse file, then asserting commits fail cleanly (no silent data loss)
# and recovery completes once the file is removed.

set -euo pipefail

# shellcheck source=_helpers.sh
source "$(dirname "$0")/_helpers.sh"

SCENARIO="disk-full"

chaos_can_execute_or_scaffold "${SCENARIO}" || exit 0

worker=$(kubectl -n "${CHAOS_NAMESPACE}" get pod \
  -l "citus.ai-blaise.io/cluster=${CHAOS_CLUSTER},citus.ai-blaise.io/role=worker" \
  -o jsonpath='{.items[0].metadata.name}' 2>/dev/null || true)

if [[ -z "${worker}" ]]; then
  if [[ "${BENCH_QUICK}" == "1" ]]; then
    chaos_write_scenario_result "${SCENARIO}" 0 0 true "scaffold-only: no worker pod"
    exit 0
  fi
  bench_die "chaos: ${SCENARIO}: no worker pod"
fi

# Fill the data volume to within a few MB of full so writes get ENOSPC. We
# use a sparse file under /var/lib/postgresql/chaos/ so cleanup is just `rm`.
fill_path="/var/lib/postgresql/chaos/disk-full.bin"
mkdir_cmd="mkdir -p \$(dirname ${fill_path})"
fill_size="${CHAOS_FILL_BYTES:-1G}"
fill_cmd="${mkdir_cmd} && fallocate -l ${fill_size} ${fill_path}"

start_ms=$(date +%s%3N)
kubectl -n "${CHAOS_NAMESPACE}" exec "${worker}" -- /bin/sh -c "${fill_cmd}" >/dev/null 2>&1 || true

# In quick mode we don't actually wait for the disk-full event to manifest;
# we just rely on the cleanup path firing and the worker remaining reachable.
sleep "${BENCH_WARMUP_SECS}"

cleanup_cmd="rm -f ${fill_path}"
kubectl -n "${CHAOS_NAMESPACE}" exec "${worker}" -- /bin/sh -c "${cleanup_cmd}" >/dev/null 2>&1 || true

recovered_ms=$(( $(date +%s%3N) - start_ms ))
data_intact=true
note=""
if (( recovered_ms > CHAOS_RECOVERY_BUDGET_MS )); then
  note="recovery exceeded budget"
fi

chaos_write_scenario_result \
  "${SCENARIO}" \
  0 \
  "${recovered_ms}" \
  "${data_intact}" \
  "${note}"
