#!/usr/bin/env bash
# Validate ai-blaise benchmark evidence against explicit SLO thresholds.
#
# Modes:
#   exploratory (default): verify JSON shape for local/PR smoke artifacts and
#     report missing/scaffold/under-threshold evidence as warnings.
#   release: fail closed when required benchmark artifacts, release drivers, or
#     measured data are missing, scaffolded, malformed, or below threshold.

set -euo pipefail

repo_root="$(cd "$(dirname "$0")/../.." && pwd)"
cd "${repo_root}"

mode="${1:-${PERF_EVIDENCE_MODE:-exploratory}}"
case "${mode}" in
  exploratory|local|quick)
    mode="exploratory"
    ;;
  release|production-release|full)
    mode="release"
    ;;
  *)
    echo "usage: $0 [exploratory|release]" >&2
    exit 2
    ;;
esac

threshold_file="${PERF_EVIDENCE_THRESHOLDS:-${repo_root}/benchmarks/performance-evidence-thresholds.json}"
results_root="${BENCH_RESULTS_ROOT:-${repo_root}/benchmarks/results}"
scope="${PERF_EVIDENCE_SCOPE:-all}"

if [[ "${mode}" == "release" ]]; then
  tag="${BENCH_RESULT_TAG:-release}"
else
  tag="${BENCH_RESULT_TAG:-quick}"
fi

python3 - "${repo_root}" "${mode}" "${tag}" "${scope}" "${threshold_file}" "${results_root}" <<'PY'
from __future__ import annotations

import glob
import json
import pathlib
import sys
from typing import Any

repo_root = pathlib.Path(sys.argv[1])
mode = sys.argv[2]
tag = sys.argv[3]
scope = sys.argv[4]
threshold_file = pathlib.Path(sys.argv[5])
results_root = pathlib.Path(sys.argv[6])
release = mode == "release"

failures: list[str] = []
warnings: list[str] = []
checked: list[str] = []


def rel(path: pathlib.Path) -> str:
    try:
        return str(path.relative_to(repo_root))
    except ValueError:
        return str(path)


def add_failure(message: str) -> None:
    failures.append(message)


def add_warning(message: str) -> None:
    warnings.append(message)


def add_issue(message: str) -> None:
    if release:
        add_failure(message)
    else:
        add_warning(message)


def load_json(path: pathlib.Path, label: str, required: bool) -> Any | None:
    if not path.is_file():
        message = f"{label}: missing evidence artifact {rel(path)}"
        if required:
            add_issue(message)
        else:
            add_warning(message)
        return None
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except json.JSONDecodeError as exc:
        add_failure(f"{label}: malformed JSON in {rel(path)}: {exc}")
        return None


def get_path(payload: Any, dotted: str) -> Any:
    current = payload
    for part in dotted.split("."):
        if isinstance(current, dict) and part in current:
            current = current[part]
        else:
            return None
    return current


def as_float(value: Any, label: str, metric: str) -> float | None:
    if isinstance(value, bool) or value is None:
        add_issue(f"{label}: metric {metric} is missing or non-numeric")
        return None
    try:
        return float(value)
    except (TypeError, ValueError):
        add_issue(f"{label}: metric {metric} is non-numeric: {value!r}")
        return None


def check_min(label: str, metric: str, value: Any, minimum: float) -> None:
    numeric = as_float(value, label, metric)
    if numeric is None:
        return
    checked.append(f"{label}.{metric}")
    if numeric < minimum:
        add_issue(f"{label}: {metric}={numeric:g} below minimum {minimum:g}")


def check_max(label: str, metric: str, value: Any, maximum: float) -> None:
    numeric = as_float(value, label, metric)
    if numeric is None:
        return
    checked.append(f"{label}.{metric}")
    if numeric > maximum:
        add_issue(f"{label}: {metric}={numeric:g} above maximum {maximum:g}")


def note_text(payload: Any) -> str:
    if isinstance(payload, dict):
        return str(payload.get("note", ""))
    return ""


def is_scaffold(payload: Any) -> bool:
    if not isinstance(payload, dict):
        return False
    return payload.get("mode") == "scaffold" or "scaffold" in note_text(payload).lower()


def require_no_scaffold(label: str, payload: Any, driver: str | None = None) -> None:
    if is_scaffold(payload):
        suffix = f"; required release driver/data: {driver}" if driver else ""
        add_issue(f"{label}: scaffold evidence is not production evidence{suffix}")


def require_mode(label: str, payload: Any, expected: str) -> None:
    if not isinstance(payload, dict):
        add_issue(f"{label}: evidence is not a JSON object")
        return
    actual = payload.get("mode")
    if release and actual != expected:
        add_failure(f"{label}: release evidence requires mode={expected!r}, got {actual!r}")
    elif actual != expected:
        add_warning(f"{label}: exploratory artifact mode={actual!r}; release requires {expected!r}")


def result_path(pattern: str) -> pathlib.Path:
    return results_root / pattern.format(tag=tag)


def load_thresholds() -> dict[str, Any] | None:
    payload = load_json(threshold_file, "threshold-manifest", required=True)
    if payload is None:
        return None
    if not isinstance(payload, dict):
        add_failure("threshold-manifest: root must be a JSON object")
        return None
    if payload.get("schema_version") != 1:
        add_failure("threshold-manifest: schema_version must be 1")
        return None
    return payload


thresholds = load_thresholds()
if thresholds is None:
    print("performance_evidence_check\tstatus=fail\tthreshold_manifest=invalid")
    for line in failures:
        print(f"FAIL\t{line}", file=sys.stderr)
    sys.exit(1)


def check_tpcc(config: dict[str, Any]) -> None:
    label = "tpcc"
    payload = load_json(result_path(config["result"]), label, required=release)
    if payload is None:
        return
    require_mode(label, payload, config["required_mode"])
    require_no_scaffold(label, payload, config.get("release_driver"))
    t = config["thresholds"]
    check_min(label, "tpmC", get_path(payload, "tpmC"), float(t["tpmC_min"]))
    check_max(label, "latency_ms.p99", get_path(payload, "latency_ms.p99"), float(t["latency_ms.p99_max"]))
    if "error_rate" in payload:
        check_max(label, "error_rate", payload.get("error_rate"), 0.005)
    else:
        check_max(label, "errors", payload.get("errors"), float(t["errors_max"]))


def check_sysbench(config: dict[str, Any]) -> None:
    for workload, workload_config in config["workloads"].items():
        label = f"sysbench.{workload}"
        payload = load_json(result_path(workload_config["result"]), label, required=release)
        if payload is None:
            continue
        require_mode(label, payload, config["required_mode"])
        require_no_scaffold(label, payload, config.get("release_driver"))
        check_min(label, "tps", payload.get("tps"), float(workload_config["tps_min"]))
        check_max(
            label,
            "latency_ms_p95",
            payload.get("latency_ms_p95"),
            float(workload_config["latency_ms_p95_max"]),
        )


def check_timescale(config: dict[str, Any]) -> None:
    label = "timescale_ingest"
    payload = load_json(result_path(config["result"]), label, required=release)
    if payload is None:
        return
    require_mode(label, payload, config["required_mode"])
    require_no_scaffold(label, payload, config.get("release_driver"))
    t = config["thresholds"]
    check_min(label, "rows_per_s", payload.get("rows_per_s"), float(t["rows_per_s_min"]))
    check_min(
        label,
        "compression_ratio",
        payload.get("compression_ratio"),
        float(t["compression_ratio_min"]),
    )
    check_max(label, "lag_ms", payload.get("lag_ms"), float(t["lag_ms_max"]))


def check_chaos(config: dict[str, Any]) -> None:
    label = "chaos"
    payload = load_json(result_path(config["result"]), label, required=release)
    if payload is None:
        return
    require_mode(label, payload, config["required_mode"])
    if not isinstance(payload, dict):
        add_issue("chaos: evidence is not a JSON object")
        return
    scenarios = payload.get("scenarios")
    if not isinstance(scenarios, list):
        add_issue("chaos: scenarios must be a JSON array")
        return
    by_name = {
        scenario.get("scenario"): scenario
        for scenario in scenarios
        if isinstance(scenario, dict) and scenario.get("scenario")
    }
    threshold = config["thresholds"]
    for scenario_name in config["required_scenarios"]:
        scenario = by_name.get(scenario_name)
        scenario_label = f"chaos.{scenario_name}"
        if scenario is None:
            add_issue(f"{scenario_label}: missing scenario result")
            continue
        require_no_scaffold(scenario_label, scenario, config.get("release_driver"))
        check_max(
            scenario_label,
            "traffic_error_rate",
            scenario.get("traffic_error_rate"),
            float(threshold["traffic_error_rate_max"]),
        )
        check_max(
            scenario_label,
            "recovery_p99_ms",
            scenario.get("recovery_p99_ms"),
            float(threshold["recovery_p99_ms_max"]),
        )
        if threshold.get("data_intact_required") and scenario.get("data_intact") is not True:
            add_issue(f"{scenario_label}: data_intact must be true")
        else:
            checked.append(f"{scenario_label}.data_intact")


def check_core(config: dict[str, Any]) -> None:
    check_tpcc(config["tpcc"])
    check_sysbench(config["sysbench"])
    check_timescale(config["timescale_ingest"])
    check_chaos(config["chaos"])


def load_baseline(path: pathlib.Path, label: str) -> dict[str, Any] | None:
    payload = load_json(path, label, required=release)
    if payload is None:
        return None
    if not isinstance(payload, dict):
        add_issue(f"{label}: baseline must be a JSON object")
        return None
    return payload


def check_microbench_result(
    result: dict[str, Any],
    config: dict[str, Any],
    seen_exts: set[str],
) -> None:
    ext = str(result.get("ext", ""))
    label = f"microbench.{ext or '<missing-ext>'}"
    if not ext:
        add_issue("microbench: result missing ext")
        return
    seen_exts.add(ext)
    expected_mode = config["required_result_mode"]
    actual_mode = result.get("mode")
    if actual_mode != expected_mode:
        require_no_scaffold(label, result, config.get("release_driver"))
        if release:
            add_failure(f"{label}: release evidence requires mode={expected_mode!r}, got {actual_mode!r}")
        else:
            add_warning(f"{label}: exploratory result mode={actual_mode!r}; release requires {expected_mode!r}")
        return

    baseline_path = repo_root / config["baseline_path"].format(ext=ext)
    baseline = load_baseline(baseline_path, f"{label}.baseline")
    if baseline is None:
        return

    base_qps = as_float(baseline.get("qps"), label, "baseline.qps")
    measured_qps = as_float(result.get("qps"), label, "qps")
    if base_qps is None or measured_qps is None:
        return
    threshold_pct = float(
        baseline.get(
            "regression_threshold_pct",
            config.get("regression_threshold_default_pct", 10),
        )
    )
    minimum = base_qps * ((100.0 - threshold_pct) / 100.0)
    checked.append(f"{label}.qps_vs_baseline")
    if measured_qps < minimum:
        add_issue(
            f"{label}: qps={measured_qps:g} below baseline floor {minimum:g} "
            f"(baseline={base_qps:g}, threshold_pct={threshold_pct:g})"
        )


def check_microbenches(config: dict[str, Any]) -> None:
    aggregate_path = repo_root / config["aggregate"]
    aggregate = load_json(aggregate_path, "microbench.aggregate", required=release)
    seen_exts: set[str] = set()

    if aggregate is None:
        pattern = str(repo_root / config["individual_glob"].format(tag=tag))
        individual_paths = [pathlib.Path(path) for path in sorted(glob.glob(pattern))]
        if individual_paths:
            add_warning(
                f"microbench.aggregate: using {len(individual_paths)} individual "
                f"{tag!r} results for exploratory validation"
            )
        elif not release:
            add_warning("microbench.aggregate: no individual exploratory results found")
        for path in individual_paths:
            payload = load_json(path, f"microbench.individual.{path.name}", required=False)
            if isinstance(payload, dict):
                check_microbench_result(payload, config, seen_exts)
        return

    if not isinstance(aggregate, dict):
        add_issue("microbench.aggregate: aggregate must be a JSON object")
        return
    results = aggregate.get("results")
    if not isinstance(results, list):
        add_issue("microbench.aggregate: results must be a JSON array")
        return
    count = int(aggregate.get("count", len(results)) or 0)
    failures_count = int(aggregate.get("failures", 0) or 0)
    minimum = int(config["minimum_count"])
    if count < minimum or len(results) < minimum:
        add_issue(
            f"microbench.aggregate: expected at least {minimum} results, "
            f"got count={count} array={len(results)}"
        )
    else:
        checked.append("microbench.aggregate.count")
    if failures_count != 0:
        add_issue(f"microbench.aggregate: failures={failures_count}, expected 0")
    else:
        checked.append("microbench.aggregate.failures")

    aggregate_mode = str(aggregate.get("mode", ""))
    if release and aggregate_mode not in {"full", "0"}:
        add_failure(f"microbench.aggregate: release requires mode='full', got {aggregate_mode!r}")
    elif not release and aggregate_mode not in {"quick", "1", "full", "0"}:
        add_warning(f"microbench.aggregate: unexpected mode={aggregate_mode!r}")

    for result in results:
        if isinstance(result, dict):
            check_microbench_result(result, config, seen_exts)
        else:
            add_issue(f"microbench.aggregate: result entry is not an object: {result!r}")


valid_scopes = {"all", "core", "microbench", "microbenches"}
if scope not in valid_scopes:
    add_failure(f"invalid PERF_EVIDENCE_SCOPE={scope!r}; expected one of {sorted(valid_scopes)}")
else:
    if scope in {"all", "core"}:
        check_core(thresholds["core_harnesses"])
    if scope in {"all", "microbench", "microbenches"}:
        check_microbenches(thresholds["microbenches"])

status = "fail" if failures else "pass"
print(
    "performance_evidence_check\t"
    f"status={status}\t"
    f"mode={mode}\t"
    f"tag={tag}\t"
    f"scope={scope}\t"
    f"thresholds={rel(threshold_file)}\t"
    f"results_root={rel(results_root)}\t"
    f"checked={len(checked)}\t"
    f"warnings={len(warnings)}\t"
    f"failures={len(failures)}"
)

for line in warnings:
    print(f"WARN\t{line}")
for line in failures:
    print(f"FAIL\t{line}", file=sys.stderr)

sys.exit(1 if failures else 0)
PY
