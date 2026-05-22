#!/usr/bin/env python3
"""Aggregate per-harness benchmarks/results/*.json into a single baseline JSON.

The aggregate is consumed by `ci-baseline-nightly.yml` (uploaded as the run
artifact) and by `benchmarks/baselines/` snapshots committed to the repo.
Schema is documented in `docs/ai-blaise/BENCHMARKS.md`.

Env:
  BENCH_RESULT_TAG  result tag used when the harnesses wrote their JSON
                    (default: env, else 'quick').
  BENCH_BASELINE_DATE  ISO date inserted into the aggregate (default: today UTC).
  BENCH_BASELINE_HOST  human-readable host label (default: '<unknown>').
  BENCH_BASELINE_OUT   output path (default: prints to stdout).
"""

from __future__ import annotations

import datetime as dt
import json
import os
import pathlib
import platform
import subprocess
import sys


def repo_root() -> pathlib.Path:
    here = pathlib.Path(__file__).resolve()
    for candidate in (here.parents[2], here.parents[1]):
        if (candidate / "benchmarks" / "results").exists():
            return candidate
    raise SystemExit("benchmarks/results not found")


def env(name: str, default: str) -> str:
    value = os.environ.get(name)
    return value if value else default


def load_result(path: pathlib.Path) -> dict:
    if not path.exists():
        return {"missing": True, "path": str(path)}
    try:
        return json.loads(path.read_text())
    except json.JSONDecodeError as exc:
        return {"missing": True, "path": str(path), "error": str(exc)}


def git_sha(repo: pathlib.Path) -> str:
    try:
        out = subprocess.check_output(
            ["git", "rev-parse", "HEAD"], cwd=repo, text=True, stderr=subprocess.DEVNULL
        )
        return out.strip()[:10]
    except (subprocess.CalledProcessError, FileNotFoundError):
        return env("GITHUB_SHA", "unknown")[:10]


def detect_specs() -> dict:
    cores = os.cpu_count() or 0
    ram_gb = 0
    try:
        with open("/proc/meminfo", "r", encoding="utf-8") as fh:
            for line in fh:
                if line.startswith("MemTotal:"):
                    kb = int(line.split()[1])
                    ram_gb = round(kb / (1024 * 1024), 1)
                    break
    except FileNotFoundError:
        pass
    return {"cores": cores, "ram_gb": ram_gb, "platform": platform.platform()}


def main() -> int:
    repo = repo_root()
    tag = env("BENCH_RESULT_TAG", "quick")
    results = repo / "benchmarks" / "results"

    aggregate: dict = {
        "date": env("BENCH_BASELINE_DATE", dt.datetime.utcnow().strftime("%Y-%m-%d")),
        "main_sha": env("BENCH_BASELINE_SHA", git_sha(repo)),
        "host": env("BENCH_BASELINE_HOST", platform.node() or "<unknown>"),
        "specs": detect_specs(),
        "tag": tag,
        "results": {
            "tpcc": load_result(results / f"tpcc-{tag}.json"),
            "sysbench_read_only": load_result(results / f"sysbench-oltp_read_only-{tag}.json"),
            "sysbench_write_only": load_result(results / f"sysbench-oltp_write_only-{tag}.json"),
            "sysbench_read_write": load_result(results / f"sysbench-oltp_read_write-{tag}.json"),
            "sysbench_point_select": load_result(
                results / f"sysbench-oltp_point_select-{tag}.json"
            ),
            "timescale_ingest": load_result(results / f"timescale-ingest-{tag}.json"),
            "chaos": load_result(results / f"chaos-{tag}.json"),
        },
    }

    text = json.dumps(aggregate, indent=2, sort_keys=True) + "\n"
    out_path = env("BENCH_BASELINE_OUT", "")
    if out_path:
        pathlib.Path(out_path).parent.mkdir(parents=True, exist_ok=True)
        pathlib.Path(out_path).write_text(text)
        print(f"[baseline-aggregate] wrote {out_path}")
    else:
        sys.stdout.write(text)
    return 0


if __name__ == "__main__":
    sys.exit(main())
