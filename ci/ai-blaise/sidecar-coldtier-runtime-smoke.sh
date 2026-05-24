#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "${repo_root}"

echo "==> sidecar-coldtier-runtime-smoke: tests"
cargo test -q -p ai_blaise_citus_sidecar_coldtier --all-targets

echo "==> sidecar-coldtier-runtime-smoke: run-canonical"
plan_output="$(cargo run -q -p ai_blaise_citus_sidecar_coldtier -- run-canonical)"
grep -Fq $'42	public.events	hot	cold	iceberg	file:///tmp/ai-blaise-coldtier/events/42	2	body,embedding' <<<"${plan_output}"

echo "==> sidecar-coldtier-runtime-smoke: run-runtime-canonical"
runtime_output="$(cargo run -q -p ai_blaise_citus_sidecar_coldtier -- run-runtime-canonical)"
grep -Fq "search_index_bytes_written" <<<"${runtime_output}"
grep -Fq "file:///tmp/ai-blaise-coldtier/indexes/events.tantivy" <<<"${runtime_output}"
grep -Fq "file:///tmp/ai-blaise-coldtier/indexes/events.lance" <<<"${runtime_output}"
grep -Fq $'	256	' <<<"${runtime_output}"

if grep -Eq 's3://|gs://|az://' <<<"${runtime_output}"; then
  echo "canonical coldtier runtime must use local file:// simulation, not live cloud object URIs" >&2
  exit 1
fi

printf "sidecar_coldtier_runtime_smoke	passed
"
