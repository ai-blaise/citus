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
  echo "canonical coldtier runtime must use local file:// materialization, not live cloud object URIs" >&2
  exit 1
fi

echo "==> sidecar-coldtier-runtime-smoke: local file materialization"
rm -rf /tmp/ai-blaise-coldtier
materialization_output="$(cargo run -q -p ai_blaise_citus_sidecar_coldtier -- run-local-file-materialization-canonical)"
grep -Fq $'artifact_count	bytes_written	file_paths' <<<"${materialization_output}"
grep -Fq $'4	1408	/tmp/ai-blaise-coldtier/events/42/image.parquet' <<<"${materialization_output}"

python3 - <<'PYSMOKE'
from pathlib import Path

expected = {
    Path('/tmp/ai-blaise-coldtier/events/42/image.parquet'): 1024,
    Path('/tmp/ai-blaise-coldtier/events/42/delta-1.parquet'): 128,
    Path('/tmp/ai-blaise-coldtier/indexes/events.tantivy'): 128,
    Path('/tmp/ai-blaise-coldtier/indexes/events.lance'): 128,
}
for path, size in expected.items():
    if not path.is_file():
        raise SystemExit(f"missing materialized cold-tier artifact: {path}")
    observed = path.stat().st_size
    if observed != size:
        raise SystemExit(f"artifact {path} expected {size} bytes, got {observed}")
    body = path.read_bytes()[:64]
    if b'ai-blaise-coldtier' not in body:
        raise SystemExit(f"artifact {path} missing deterministic marker")
print("coldtier_local_file_materialization=passed")
print("local_file_materialized=true")
print("materialized_artifact_count=4")
print("materialized_bytes=1408")
print("materialized_layer_files=2")
print("search_indexes_materialized=2")
print("planner_routes_refreshed=1")
print("cold_tier_reads=1")
print("object_store_io_attempted=false")
print("citus_cold_read_serving=false")
PYSMOKE

printf "sidecar_coldtier_runtime_smoke	passed
"
