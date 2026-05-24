#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "${repo_root}"

output="$(cargo run -q -p ai_blaise_citus_sidecar_repack -- run-canonical)"
header="$(printf '%s\n' "${output}" | sed -n '1p')"
row="$(printf '%s\n' "${output}" | sed -n '2p')"

expected_header=$'target\tstrategy\tschedule\tmax_concurrency\tlock_timeout_ms\tshard_count\tfirst_shard_id\tfirst_worker\tfirst_table\tpg_major\tpg_repack_available\tpg19_repack_concurrently_available\tdry_run\texecuted\tevidence_boundary\texecutable\targs'
if [[ "${header}" != "${expected_header}" ]]; then
  echo "unexpected repack canonical header" >&2
  printf '%s\n' "${header}" >&2
  exit 1
fi

IFS=$'\t' read -r target strategy schedule max_concurrency lock_timeout_ms shard_count first_shard_id first_worker first_table pg_major pg_repack_available pg19_repack_concurrently_available dry_run executed evidence_boundary executable args <<<"${row}"

[[ "${target}" == "public.orders" ]]
[[ "${strategy}" == "pg_repack" ]]
[[ "${schedule}" == "0 3 * * 0" ]]
[[ "${max_concurrency}" == "2" ]]
[[ "${lock_timeout_ms}" == "500" ]]
[[ "${shard_count}" == "2" ]]
[[ "${first_shard_id}" == "102008" ]]
[[ "${first_worker}" == "worker-a" ]]
[[ "${first_table}" == "public.orders_102008" ]]
[[ "${pg_major}" == "18" ]]
[[ "${pg_repack_available}" == "true" ]]
[[ "${pg19_repack_concurrently_available}" == "false" ]]
[[ "${dry_run}" == "true" ]]
[[ "${executed}" == "false" ]]
[[ "${evidence_boundary}" == "dry-run-plan-only" ]]
[[ "${executable}" == "pg_repack" ]]
[[ "${args}" == "--table public.orders --jobs 2" ]]

printf 'sidecar_repack_smoke	strategy=%s	dry_run=%s	executed=%s	evidence_boundary=%s
' "${strategy}" "${dry_run}" "${executed}" "${evidence_boundary}"
