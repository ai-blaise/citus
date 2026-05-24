#!/usr/bin/env bash
# benchmarks/microbenches/compare-to-baseline.sh
#
# Runs the aggregate microbench runner, then compares each measured qps to the
# baseline.json next to the corresponding bench.sh. Exits non-zero if any
# measured run regressed by more than `regression_threshold_pct` (default 10%)
# from its baseline.
#
# Scaffold results (mode=scaffold) are not compared in exploratory mode. In
# release mode they are explicit failures because they are not production
# evidence.

set -euo pipefail

HARNESS_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "${HARNESS_DIR}/../.." && pwd)"
# shellcheck source=../common/lib.sh
# shellcheck disable=SC1091
source "${REPO_ROOT}/benchmarks/common/lib.sh"

bash "${HARNESS_DIR}/run-all.sh" >/dev/null

aggregate="${HARNESS_DIR}/results-latest.json"
if [[ ! -s "${aggregate}" ]]; then
  bench_die "compare-to-baseline.sh: missing aggregate ${aggregate}"
fi

python3 - "${HARNESS_DIR}" "${aggregate}" <<'PY'
import json
import os
import pathlib
import sys

harness_dir = pathlib.Path(sys.argv[1])
aggregate = pathlib.Path(sys.argv[2])
release_mode = os.environ.get("AI_BLAISE_RELEASE_MODE") == "1" or os.environ.get("BENCH_REQUIRE_MEASURED") == "1"

report = json.loads(aggregate.read_text())
results = report.get("results", [])

regressions: list[str] = []
checked = 0
scaffolds = 0
missing_baseline: list[str] = []

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
    f"regressions={len(regressions)}"
)

if missing_baseline:
    print("missing baselines:")
    for ext in missing_baseline:
        print(f"  - {ext}")

if regressions:
    print("regressions:", file=sys.stderr)
    for line in regressions:
        print(f"  - {line}", file=sys.stderr)
    sys.exit(1)

if release_mode and scaffolds:
    print(
        "release mode requires measured microbench results; "
        f"found {scaffolds} scaffold result(s)",
        file=sys.stderr,
    )
    sys.exit(1)
PY
