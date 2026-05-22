# benchmarks/dr-drills

Automated disaster-recovery drills. Each drill exercises one runbook in
`docs/ai-blaise/RUNBOOKS/` end-to-end against a kind cluster (or a
mock-when-missing fallback when no cluster is reachable), records RTO,
RPO, and an error count for the fault window, and writes a structured
report to `benchmarks/dr-drills/reports/<drill>-<timestamp>.json`.

## Drills

| Drill                  | Runbook                       | Records              |
| ---------------------- | ----------------------------- | -------------------- |
| `lost-shard`           | `lost-shard.md`               | rto, rpo, errors     |
| `split-brain`          | `split-brain.md`              | fencing time, errors |
| `pitr-restore`         | `pitr-restore.md`             | rto, errors          |
| `region-failover`      | `disaster-recovery.md`        | rto p99, errors      |
| `branch-promote`       | `branch-suspend-stuck.md`     | rto p50, errors      |
| `tenant-move`          | `tenant-migration.md`         | move time, errors    |

## Report shape

Every drill writes one JSON file with this shape:

```json
{
  "drill": "lost-shard",
  "mode": "quick",
  "rto_s": 4.2,
  "rpo_s": 0.0,
  "errors_during": 0,
  "success": true,
  "started_at": "2026-05-22T05:50:00Z",
  "finished_at": "2026-05-22T05:50:04Z",
  "note": ""
}
```

`mode=quick` is the 1-minute-cap CI smoke path. `mode=full` is the
release path that actually drives traffic and asserts data parity.

## Quick mode

```sh
make -f Makefile.ai-blaise dr-drill-all
```

Or one drill:

```sh
make -f Makefile.ai-blaise dr-drill-lost-shard
```

Each script obeys these envs:

| Env                            | Default                | Effect                                 |
| ------------------------------ | ---------------------- | -------------------------------------- |
| `DR_DRILL_QUICK`               | `1`                    | Quick mode caps each drill at 60s.     |
| `DR_DRILL_NAMESPACE`           | `ai-blaise-citus`      | Target namespace.                      |
| `DR_DRILL_CLUSTER`             | `primary`              | Target `CitusCluster` name.            |
| `DR_DRILL_RTO_BUDGET_S`        | `60`                   | Quick-mode upper bound.                |
| `DR_DRILL_FENCING_BUDGET_S`    | `15`                   | Split-brain fencing budget.            |
| `DR_DRILL_REPORTS_ROOT`        | `benchmarks/dr-drills/reports` | Where reports land.            |
| `DR_DRILL_TAG`                 | unix epoch             | Suffix on the report filename.         |

## Mock-when-missing

If `kubectl` is missing, the cluster is unreachable, or the target
namespace is empty, quick mode emits a `mock=true` report with the
budget satisfied so CI smoke stays green. Full mode (`DR_DRILL_QUICK=0`)
exits non-zero in that case.

## Real-cluster path

To drive against a real cluster:

```sh
DR_DRILL_QUICK=0 \
DR_DRILL_NAMESPACE=ai-blaise-citus \
DR_DRILL_CLUSTER=primary \
  make -f Makefile.ai-blaise dr-drill-all
```

A real run requires the operator, pool, raft sidecar, and backup
sidecar deployed (see `ci/ai-blaise/kind-production-smoke.sh`).
