#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "${repo_root}"

echo "[router-patch-smoke] verify quilt series"
bash ci/ai-blaise/patches-check.sh patches/series

for patch in \
  patches/0004-hashtable-on-planner-hotpath.patch \
  patches/0006-fast-path-router-no-coord-rt.patch
do
  if [[ ! -s "${patch}" ]]; then
    echo "[router-patch-smoke] missing ${patch}" >&2
    exit 1
  fi
done

if ! grep -q "IntersectPlacementListHashed" patches/0004-hashtable-on-planner-hotpath.patch; then
  echo "[router-patch-smoke] 0004 missing hashed intersection" >&2
  exit 1
fi
if ! grep -q "RouterFastPathCanSkipCoordinator" patches/0006-fast-path-router-no-coord-rt.patch; then
  echo "[router-patch-smoke] 0006 missing coordinator skip probe" >&2
  exit 1
fi

export BENCH_RESULT_TAG="${BENCH_RESULT_TAG:-quick}"
python3 benchmarks/router-planner/bench.py --quick

result="benchmarks/results/router-planner-${BENCH_RESULT_TAG}.json"
if [[ ! -s "${result}" ]]; then
  echo "[router-patch-smoke] missing result ${result}" >&2
  exit 1
fi

python3 - "${result}" <<'PY'
import json
import sys

with open(sys.argv[1], encoding="utf-8") as handle:
    result = json.load(handle)
if result["speedup"] < result["min_speedup"]:
    raise SystemExit("router planner smoke did not meet min speedup")
if result["evidence_boundary"] != "algorithm-smoke-not-live-citus-performance":
    raise SystemExit("router planner smoke boundary missing")
print("[router-patch-smoke] result", json.dumps(result, sort_keys=True))
PY

echo "[router-patch-smoke] ok"
