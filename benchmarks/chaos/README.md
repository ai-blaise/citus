# benchmarks/chaos

Chaos harness for the V2 chaos acceptance gate (gate 11).

Each scenario is a small bash script under `scenarios/`. We intentionally use
hand-rolled `kubectl` + `iptables` / `tc` over Litmus to avoid the
cluster-install overhead on the experiment VM. Litmus is an option for the
nightly path when the kind cluster has the headroom; the per-scenario shape
is shared.

## Scenarios

| Scenario           | Mechanism                                          |
| ------------------ | -------------------------------------------------- |
| `kill-coordinator` | `kubectl delete pod -l role=coordinator --force`   |
| `kill-worker`      | `kubectl delete pod` on a randomly chosen worker   |
| `network-partition` | Apply a deny-all `NetworkPolicy` to one worker     |
| `disk-full`        | `fallocate -l 1G` inside `/var/lib/postgresql/chaos` |
| `slow-disk`        | `tc qdisc add dev lo root netem delay 50ms`        |

## Assertions

Each scenario records:

- `traffic_error_rate`: pool-observed error rate during the fault window.
- `recovery_p99_ms`: time from fault injection to first ready healthcheck.
- `data_intact`: boolean; whether commits replayed cleanly.

Acceptance thresholds (alpha):

| Assertion                | Threshold              |
| ------------------------ | ---------------------- |
| Pool error rate          | < `CHAOS_TRAFFIC_ERROR_BUDGET` (default 5%) |
| Recovery p99             | < `CHAOS_RECOVERY_BUDGET_MS` (default 5000ms) |
| Lost commits             | 0                      |

## Run

```sh
# Quick mode (CI smoke; soft-passes when no cluster is reachable):
make -f Makefile.ai-blaise bench-chaos

# Full release run against a kind cluster provisioned by
# `ci/ai-blaise/kind-production-smoke.sh`:
BENCH_QUICK=0 CHAOS_NAMESPACE=ai-blaise-citus CHAOS_CLUSTER=primary \
  ./run.sh
```

## Soft-pass behaviour

If `kubectl` is not on `PATH`, or the cluster is unreachable, each scenario
writes a scaffold result JSON with `traffic_error_rate=0`, `recovery_p99_ms=0`,
and a `note` flag, then exits 0. Full mode (`BENCH_QUICK=0`) fails hard if the
cluster is not reachable.

## Result files

Per scenario: `benchmarks/results/chaos-<scenario>-<BENCH_RESULT_TAG>.json`.
Combined summary: `benchmarks/results/chaos-<BENCH_RESULT_TAG>.json` aggregates
all per-scenario rows in the `scenarios` list.
