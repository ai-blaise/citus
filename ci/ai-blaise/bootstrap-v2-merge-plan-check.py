#!/usr/bin/env python3
"""Validate the bootstrap-v2 PR merge plan.

The checker is intentionally metadata-only: it reads the JSON plan and,
optionally, live GitHub PR metadata. It never edits the worktree, branches, or
pull requests.
"""

from __future__ import annotations

import argparse
import json
import pathlib
import subprocess
import sys
from typing import Any


DEFAULT_PLAN = pathlib.Path("docs/ai-blaise/bootstrap-v2-merge-plan.json")
LIVE_FIELDS = [
    "number",
    "title",
    "baseRefName",
    "headRefName",
    "isDraft",
    "mergeStateStatus",
    "reviewDecision",
    "url",
    "updatedAt",
]
VALID_STATUSES = {"ready", "draft-blocked", "blocked"}


def load_json(path: pathlib.Path) -> Any:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except FileNotFoundError:
        raise SystemExit(f"plan file not found: {path}")
    except json.JSONDecodeError as exc:
        raise SystemExit(f"invalid JSON in {path}: {exc}") from exc


def require_mapping(value: Any, path: str, errors: list[str]) -> dict[str, Any]:
    if not isinstance(value, dict):
        errors.append(f"{path} must be an object")
        return {}
    return value


def require_list(value: Any, path: str, errors: list[str]) -> list[Any]:
    if not isinstance(value, list):
        errors.append(f"{path} must be a list")
        return []
    return value


def validate_plan_shape(plan: dict[str, Any]) -> list[str]:
    errors: list[str] = []

    if plan.get("schema_version") != 1:
        errors.append("schema_version must be 1")

    metadata = require_mapping(plan.get("metadata"), "metadata", errors)
    for key in [
        "repository",
        "base_branch",
        "reference_branch",
        "pr_number_min",
        "captured_at",
    ]:
        if key not in metadata:
            errors.append(f"metadata.{key} is required")

    base_branch = metadata.get("base_branch")
    pr_number_min = metadata.get("pr_number_min")
    if not isinstance(pr_number_min, int):
        errors.append("metadata.pr_number_min must be an integer")
        pr_number_min = 0

    batches = require_list(plan.get("integration_batches"), "integration_batches", errors)
    batch_ids: set[str] = set()
    batch_prs: list[int] = []
    batch_order_by_id: dict[str, int] = {}
    for index, raw_batch in enumerate(batches):
        batch = require_mapping(raw_batch, f"integration_batches[{index}]", errors)
        batch_id = batch.get("id")
        if not isinstance(batch_id, str) or not batch_id:
            errors.append(f"integration_batches[{index}].id must be a non-empty string")
            continue
        if batch_id in batch_ids:
            errors.append(f"duplicate integration batch id: {batch_id}")
        batch_ids.add(batch_id)
        batch_order_by_id[batch_id] = index
        if not isinstance(batch.get("run_expensive_citus_matrix_after"), bool):
            errors.append(
                f"integration_batches[{index}].run_expensive_citus_matrix_after must be boolean"
            )
        for number in require_list(batch.get("prs"), f"integration_batches[{index}].prs", errors):
            if not isinstance(number, int):
                errors.append(f"integration_batches[{index}].prs entries must be integers")
            else:
                batch_prs.append(number)

    matrix_policy = require_mapping(
        plan.get("expensive_citus_matrix_policy"),
        "expensive_citus_matrix_policy",
        errors,
    )
    for key in ["run_full_matrix_after_batches", "run_immediately_when", "do_not_run_when"]:
        require_list(matrix_policy.get(key), f"expensive_citus_matrix_policy.{key}", errors)
    for batch_id in matrix_policy.get("run_full_matrix_after_batches", []):
        if batch_id not in batch_ids:
            errors.append(
                "expensive_citus_matrix_policy.run_full_matrix_after_batches "
                f"references unknown batch {batch_id!r}"
            )

    prs = require_list(plan.get("pull_requests"), "pull_requests", errors)
    numbers: dict[int, dict[str, Any]] = {}
    orders: dict[int, int] = {}
    planned_batch_prs: set[int] = set(batch_prs)
    planned_prs: set[int] = set()

    for index, raw_pr in enumerate(prs):
        pr = require_mapping(raw_pr, f"pull_requests[{index}]", errors)
        for key in [
            "number",
            "order",
            "batch",
            "title",
            "head",
            "base",
            "status",
            "is_draft",
            "dependency_hints",
            "known_blockers",
            "watch_paths",
        ]:
            if key not in pr:
                errors.append(f"pull_requests[{index}].{key} is required")

        number = pr.get("number")
        order = pr.get("order")
        if not isinstance(number, int):
            errors.append(f"pull_requests[{index}].number must be an integer")
            continue
        if number < pr_number_min:
            errors.append(f"PR #{number} is below metadata.pr_number_min={pr_number_min}")
        if number in numbers:
            errors.append(f"duplicate PR number: {number}")
        numbers[number] = pr
        planned_prs.add(number)

        if not isinstance(order, int):
            errors.append(f"pull_requests[{index}].order must be an integer")
        elif order in orders:
            errors.append(f"duplicate merge order {order}: PR #{orders[order]} and PR #{number}")
        else:
            orders[order] = number

        if pr.get("base") != base_branch:
            errors.append(f"PR #{number} base must be {base_branch!r}")
        if pr.get("batch") not in batch_ids:
            errors.append(f"PR #{number} references unknown batch {pr.get('batch')!r}")
        if pr.get("status") not in VALID_STATUSES:
            errors.append(f"PR #{number} has invalid status {pr.get('status')!r}")
        if not isinstance(pr.get("is_draft"), bool):
            errors.append(f"PR #{number}.is_draft must be boolean")

        dependencies = require_list(pr.get("dependency_hints"), f"PR #{number}.dependency_hints", errors)
        blockers = require_list(pr.get("known_blockers"), f"PR #{number}.known_blockers", errors)
        require_list(pr.get("watch_paths"), f"PR #{number}.watch_paths", errors)
        if pr.get("status") in {"draft-blocked", "blocked"} and not blockers:
            errors.append(
                f"PR #{number} status {pr.get('status')!r} requires known_blockers"
            )
        if pr.get("is_draft") and pr.get("status") != "draft-blocked":
            errors.append(f"PR #{number} is_draft=true must use status draft-blocked")

        for dependency in dependencies:
            if not isinstance(dependency, int):
                errors.append(f"PR #{number} dependency_hints entries must be integers")

    expected_orders = list(range(1, len(prs) + 1))
    if sorted(orders) != expected_orders:
        errors.append(
            "merge orders must be contiguous 1..N; observed "
            + ",".join(str(order) for order in sorted(orders))
        )

    missing_from_batches = sorted(planned_prs - planned_batch_prs)
    extra_in_batches = sorted(planned_batch_prs - planned_prs)
    if missing_from_batches:
        errors.append("PRs missing from integration_batches: " + comma_prs(missing_from_batches))
    if extra_in_batches:
        errors.append("integration_batches references unknown PRs: " + comma_prs(extra_in_batches))

    duplicate_batch_prs = sorted({number for number in batch_prs if batch_prs.count(number) > 1})
    if duplicate_batch_prs:
        errors.append("PRs listed in multiple integration batches: " + comma_prs(duplicate_batch_prs))

    for number, pr in sorted(numbers.items()):
        order = pr.get("order")
        batch = pr.get("batch")
        for dependency in pr.get("dependency_hints", []):
            dependency_pr = numbers.get(dependency)
            if dependency_pr is None:
                errors.append(f"PR #{number} depends on unknown PR #{dependency}")
                continue
            if isinstance(order, int) and dependency_pr.get("order", 0) >= order:
                errors.append(
                    f"PR #{number} depends on PR #{dependency}, but dependency order "
                    f"{dependency_pr.get('order')} is not before {order}"
                )
            if batch in batch_order_by_id and dependency_pr.get("batch") in batch_order_by_id:
                if batch_order_by_id[dependency_pr["batch"]] > batch_order_by_id[batch]:
                    errors.append(
                        f"PR #{number} batch {batch!r} is before dependency "
                        f"PR #{dependency} batch {dependency_pr.get('batch')!r}"
                    )

    return errors


def comma_prs(numbers: list[int]) -> str:
    return ", ".join(f"#{number}" for number in numbers)


def load_metadata_json(path_text: str) -> list[dict[str, Any]]:
    if path_text == "-":
        raw = sys.stdin.read()
    else:
        raw = pathlib.Path(path_text).read_text(encoding="utf-8")
    try:
        data = json.loads(raw)
    except json.JSONDecodeError as exc:
        raise SystemExit(f"invalid metadata JSON: {exc}") from exc
    if not isinstance(data, list):
        raise SystemExit("metadata JSON must be a list from gh pr list")
    return data


def fetch_live_metadata(plan: dict[str, Any]) -> list[dict[str, Any]]:
    metadata = plan["metadata"]
    command = [
        "gh",
        "pr",
        "list",
        "--repo",
        metadata["repository"],
        "--state",
        "open",
        "--base",
        metadata["base_branch"],
        "--limit",
        "200",
        "--json",
        ",".join(LIVE_FIELDS),
    ]
    try:
        completed = subprocess.run(
            command,
            check=True,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )
    except FileNotFoundError as exc:
        raise SystemExit("gh is required for --live validation") from exc
    except subprocess.CalledProcessError as exc:
        raise SystemExit(f"gh pr list failed: {exc.stderr.strip()}") from exc
    return json.loads(completed.stdout)


def validate_live_metadata(
    plan: dict[str, Any],
    live_prs: list[dict[str, Any]],
    strict_open_set: bool,
) -> tuple[list[str], list[str]]:
    errors: list[str] = []
    warnings: list[str] = []
    metadata = plan["metadata"]
    pr_number_min = metadata["pr_number_min"]
    live_prs = [pr for pr in live_prs if pr.get("number", 0) >= pr_number_min]
    planned_by_number = {pr["number"]: pr for pr in plan["pull_requests"]}
    live_by_number = {pr["number"]: pr for pr in live_prs}

    missing_live = sorted(set(planned_by_number) - set(live_by_number))
    extra_live = sorted(set(live_by_number) - set(planned_by_number))
    if missing_live:
        errors.append("planned PRs are not currently open: " + comma_prs(missing_live))
    if extra_live:
        message = "open bootstrap-v2 PRs missing from plan: " + comma_prs(extra_live)
        if strict_open_set:
            errors.append(message)
        else:
            warnings.append(message)

    for number in sorted(set(planned_by_number) & set(live_by_number)):
        planned = planned_by_number[number]
        live = live_by_number[number]
        comparisons = [
            ("title", planned.get("title"), live.get("title")),
            ("head", planned.get("head"), live.get("headRefName")),
            ("base", planned.get("base"), live.get("baseRefName")),
            ("is_draft", planned.get("is_draft"), live.get("isDraft")),
        ]
        for field, expected, observed in comparisons:
            if expected != observed:
                errors.append(
                    f"PR #{number} {field} drift: plan={expected!r} live={observed!r}"
                )
    return errors, warnings


def print_order(plan: dict[str, Any]) -> None:
    print("order\tpr\tstatus\tdraft\tbatch\thead\ttitle")
    for pr in sorted(plan["pull_requests"], key=lambda item: item["order"]):
        print(
            f"{pr['order']}\t#{pr['number']}\t{pr['status']}\t"
            f"{str(pr['is_draft']).lower()}\t{pr['batch']}\t{pr['head']}\t{pr['title']}"
        )


def summarize(plan: dict[str, Any], live_checked: bool) -> str:
    prs = plan["pull_requests"]
    drafts = sorted(pr["number"] for pr in prs if pr["is_draft"])
    matrix_batches = [
        batch["id"]
        for batch in plan["integration_batches"]
        if batch["run_expensive_citus_matrix_after"]
    ]
    return (
        "bootstrap_v2_merge_plan_check ok\t"
        f"prs={len(prs)}\t"
        f"draft_blockers={comma_prs(drafts) if drafts else 'none'}\t"
        f"live_checked={str(live_checked).lower()}\t"
        f"matrix_batches={','.join(matrix_batches)}"
    )


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--plan", default=str(DEFAULT_PLAN), help="path to merge plan JSON")
    parser.add_argument("--offline", action="store_true", help="validate the local JSON plan only")
    parser.add_argument("--live", action="store_true", help="query gh for live open PR metadata")
    parser.add_argument(
        "--strict-open-set",
        action="store_true",
        help="fail when live GitHub has extra open bootstrap-v2 PRs not listed in the plan",
    )
    parser.add_argument(
        "--metadata-json",
        help="validate against a saved gh pr list JSON file; use - for stdin",
    )
    parser.add_argument(
        "--require-no-drafts",
        action="store_true",
        help="fail if any planned or live PR is still draft",
    )
    parser.add_argument("--print-order", action="store_true", help="print the planned order as TSV")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    if args.live and args.metadata_json:
        print("choose only one of --live or --metadata-json", file=sys.stderr)
        return 2

    plan = load_json(pathlib.Path(args.plan))
    if not isinstance(plan, dict):
        print("plan root must be an object", file=sys.stderr)
        return 1

    errors = validate_plan_shape(plan)
    warnings: list[str] = []
    live_checked = False
    if args.live or args.metadata_json:
        live_metadata = fetch_live_metadata(plan) if args.live else load_metadata_json(args.metadata_json)
        live_errors, live_warnings = validate_live_metadata(
            plan, live_metadata, args.strict_open_set
        )
        errors.extend(live_errors)
        warnings.extend(live_warnings)
        live_checked = True

    if args.require_no_drafts:
        drafts = [pr["number"] for pr in plan.get("pull_requests", []) if pr.get("is_draft")]
        if drafts:
            errors.append("draft blockers remain: " + comma_prs(sorted(drafts)))

    if errors:
        for error in errors:
            print(f"bootstrap-v2 merge plan error: {error}", file=sys.stderr)
        return 1

    for warning in warnings:
        print(f"bootstrap-v2 merge plan warning: {warning}", file=sys.stderr)

    if args.print_order:
        print_order(plan)
    else:
        print(summarize(plan, live_checked))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
