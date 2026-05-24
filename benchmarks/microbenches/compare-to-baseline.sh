#!/usr/bin/env bash
# benchmarks/microbenches/compare-to-baseline.sh
#
# Runs the aggregate microbench runner, then compares each measured qps to the
# baseline.json next to the corresponding bench.sh. Exits non-zero if any
# measured run regressed by more than `regression_threshold_pct` (default 10%)
# from its baseline.
#
# Scaffold results (mode=scaffold) are not compared — they only fire when
# psql or the target Postgres endpoint is unavailable on an exploratory runner.
# In full/release mode, scaffold or missing-baseline evidence fails closed.

set -euo pipefail

HARNESS_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "${HARNESS_DIR}/../.." && pwd)"
# shellcheck source=../common/lib.sh
source "${REPO_ROOT}/benchmarks/common/lib.sh"

release_mode=0
if [[ "${BENCH_QUICK}" == "0" || "${BENCH_RESULT_TAG}" == "release" || "${PERF_EVIDENCE_MODE:-}" == "release" ]]; then
  release_mode=1
fi

bash "${HARNESS_DIR}/run-all.sh" >/dev/null

aggregate="${HARNESS_DIR}/results-latest.json"
if [[ ! -s "${aggregate}" ]]; then
  bench_die "compare-to-baseline.sh: missing aggregate ${aggregate}"
fi

python3 - "${HARNESS_DIR}" "${aggregate}" "${release_mode}" <<'PY'
import json
import pathlib
import sys

harness_dir = pathlib.Path(sys.argv[1])
aggregate = pathlib.Path(sys.argv[2])
release_mode = sys.argv[3] == "1"

report = json.loads(aggregate.read_text())
results = report.get("results", [])

regressions: list[str] = []
checked = 0
scaffolds = 0
missing_baseline: list[str] = []
malformed: list[str] = []
expected_count = len([p for p in harness_dir.iterdir() if p.is_dir()])

for result in results:
    ext = result.get("ext", "")
    mode = result.get("mode", "")
    if mode != "measured":
        scaffolds += 1
        continue

    baseline_path = harness_dir / ext / "baseline.json"
    if not baseline_path.is_file():
        missing_baseline.append(ext)
        continue

    baseline = json.loads(baseline_path.read_text())
    base_qps = float(baseline.get("qps", 0))
    measured_qps = float(result.get("qps", 0))
    threshold_pct = float(baseline.get("regression_threshold_pct", 10))

    if not ext or measured_qps <= 0:
        malformed.append(ext or "<missing-ext>")
        continue

    if base_qps <= 0:
        # Baseline did not set a meaningful threshold (e.g. seconds-based
        # benches like pg_repack); skip the qps comparison.
        continue

    checked += 1
    ratio_pct = (measured_qps / base_qps) * 100.0
    if ratio_pct < (100.0 - threshold_pct):
        regressions.append(
            f"{ext}: measured_qps={measured_qps:.2f} "
            f"baseline_qps={base_qps:.2f} "
            f"ratio={ratio_pct:.1f}% threshold={100.0 - threshold_pct:.1f}%"
        )

print(
    f"compare-to-baseline summary: "
    f"results={len(results)} "
    f"measured_checked={checked} "
    f"scaffolds={scaffolds} "
    f"missing_baseline={len(missing_baseline)} "
    f"malformed={len(malformed)} "
    f"regressions={len(regressions)}"
)

release_failures: list[str] = []
if release_mode:
    if len(results) < expected_count:
        release_failures.append(
            f"expected at least {expected_count} microbench results, got {len(results)}"
        )
    if scaffolds:
        release_failures.append(
            f"{scaffolds} scaffold result(s) in release/full mode; required psql/cluster/extensions missing"
        )
    if missing_baseline:
        release_failures.append(
            f"missing baseline(s): {', '.join(sorted(missing_baseline))}"
        )
    if malformed:
        release_failures.append(
            f"malformed measured result(s): {', '.join(sorted(malformed))}"
        )
    if checked == 0:
        release_failures.append("no measured microbench result was compared")

if missing_baseline:
    print("missing baselines:")
    for ext in missing_baseline:
        print(f"  - {ext}")

if malformed:
    print("malformed measured results:")
    for ext in malformed:
        print(f"  - {ext}")

if regressions:
    print("regressions:", file=sys.stderr)
    for line in regressions:
        print(f"  - {line}", file=sys.stderr)
    sys.exit(1)

if release_failures:
    print("release-mode evidence failures:", file=sys.stderr)
    for line in release_failures:
        print(f"  - {line}", file=sys.stderr)
    sys.exit(1)
PY
