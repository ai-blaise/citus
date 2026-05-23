#!/usr/bin/env python3
"""Router planner patch smoke benchmark.

This intentionally measures the algorithmic boundary of patches 0004/0006 in a
portable Python harness. It is not a substitute for a full Citus build and live
cluster performance run.
"""

from __future__ import annotations

import argparse
import json
import math
import os
from pathlib import Path
from statistics import median
from time import perf_counter_ns


def build_placements(count: int, offset: int = 0) -> list[tuple[str, int, int]]:
    placements: list[tuple[str, int, int]] = []
    for index in range(count):
        endpoint = offset + index
        placements.append(
            (f"worker-{endpoint:05d}", 5400 + endpoint % 97, endpoint % 7)
        )
    return placements


def linear_intersect(
    lhs: list[tuple[str, int, int]], rhs: list[tuple[str, int, int]]
) -> list[tuple[str, int, int]]:
    result: list[tuple[str, int, int]] = []
    for lhs_name, lhs_port, _ in lhs:
        for rhs_placement in rhs:
            rhs_name, rhs_port, _ = rhs_placement
            if rhs_port == lhs_port and rhs_name == lhs_name:
                result.append(rhs_placement)
                break
    return result


def hashed_intersect(
    lhs: list[tuple[str, int, int]], rhs: list[tuple[str, int, int]]
) -> list[tuple[str, int, int]]:
    rhs_index: dict[tuple[str, int], tuple[str, int, int]] = {}
    for placement in rhs:
        name, port, _ = placement
        rhs_index.setdefault((name, port), placement)
    return [
        rhs_index[(name, port)] for name, port, _ in lhs if (name, port) in rhs_index
    ]


def can_skip_coordinator(
    active_placements: list[tuple[str, int, int]],
    local_group_id: int,
    enabled: bool = True,
) -> bool:
    if not enabled or len(active_placements) != 1:
        return False
    return active_placements[0][2] == local_group_id


def time_call(func, lhs, rhs, iterations: int) -> float:
    start = perf_counter_ns()
    expected = None
    for _ in range(iterations):
        expected = func(lhs, rhs)
    elapsed_ns = perf_counter_ns() - start
    if expected is None:
        raise AssertionError("benchmark did not run")
    return elapsed_ns / iterations / 1000.0


def run(args: argparse.Namespace) -> dict[str, object]:
    lhs = build_placements(args.placements, 0)
    rhs = build_placements(args.placements, args.skew)
    overlap = build_placements(args.overlap, 0)
    rhs[: args.overlap] = overlap

    linear_result = linear_intersect(lhs, rhs)
    hashed_result = hashed_intersect(lhs, rhs)
    if linear_result != hashed_result:
        raise AssertionError("hashed intersection changed legacy placement semantics")

    linear_samples = [
        time_call(linear_intersect, lhs, rhs, args.iterations)
        for _ in range(args.samples)
    ]
    hashed_samples = [
        time_call(hashed_intersect, lhs, rhs, args.iterations)
        for _ in range(args.samples)
    ]
    linear_us = median(linear_samples)
    hashed_us = median(hashed_samples)
    speedup = linear_us / hashed_us if hashed_us > 0.0 else math.inf

    single_local = [("worker-local", 5432, 7)]
    single_remote = [("worker-remote", 5433, 8)]
    replicated = [("worker-a", 5432, 7), ("worker-b", 5433, 8)]
    if not can_skip_coordinator(single_local, 7):
        raise AssertionError("single local placement should skip coordinator")
    if can_skip_coordinator(single_remote, 7):
        raise AssertionError("remote placement must not skip coordinator")
    if can_skip_coordinator(replicated, 7):
        raise AssertionError("replicated placement must not skip coordinator")
    if can_skip_coordinator(single_local, 7, enabled=False):
        raise AssertionError("disabled GUC must force coordinator path")

    if speedup < args.min_speedup:
        raise AssertionError(
            f"hashed planner smoke speedup {speedup:.2f}x below {args.min_speedup:.2f}x"
        )

    return {
        "harness": "router-planner",
        "mode": "quick" if args.quick else "full",
        "placements": args.placements,
        "overlap": args.overlap,
        "iterations": args.iterations,
        "linear_us_per_call": round(linear_us, 3),
        "hashed_us_per_call": round(hashed_us, 3),
        "speedup": round(speedup, 3),
        "min_speedup": args.min_speedup,
        "coordinator_skip_cases": {
            "single_local": True,
            "single_remote": False,
            "replicated": False,
            "disabled": False,
        },
        "evidence_boundary": "algorithm-smoke-not-live-citus-performance",
    }


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--quick", action="store_true")
    parser.add_argument(
        "--placements",
        type=int,
        default=int(os.getenv("ROUTER_BENCH_PLACEMENTS", "192")),
    )
    parser.add_argument(
        "--overlap", type=int, default=int(os.getenv("ROUTER_BENCH_OVERLAP", "96"))
    )
    parser.add_argument(
        "--skew", type=int, default=int(os.getenv("ROUTER_BENCH_SKEW", "10000"))
    )
    parser.add_argument(
        "--iterations",
        type=int,
        default=int(os.getenv("ROUTER_BENCH_ITERATIONS", "160")),
    )
    parser.add_argument(
        "--samples", type=int, default=int(os.getenv("ROUTER_BENCH_SAMPLES", "5"))
    )
    parser.add_argument(
        "--min-speedup",
        type=float,
        default=float(os.getenv("ROUTER_BENCH_MIN_SPEEDUP", "1.5")),
    )
    parser.add_argument(
        "--output",
        type=Path,
        default=Path("benchmarks/results")
        / f"router-planner-{os.getenv('BENCH_RESULT_TAG', 'quick')}.json",
    )
    return parser.parse_args()


def main() -> None:
    args = parse_args()
    result = run(args)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, sort_keys=True) + "\n", encoding="utf-8")
    print(json.dumps(result, sort_keys=True))


if __name__ == "__main__":
    main()
