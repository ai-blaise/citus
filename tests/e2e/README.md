# tests/e2e

> Production boundary: unless a feature is explicitly `Status: production-ready`
> in `docs/ai-blaise/NEW_FEATURES.md`, the surfaces listed here are alpha
> contracts. Deterministic canonical reports and local runtime models are CI
> artifacts, not production evidence; promotion requires live VM/container or
> Kubernetes evidence recorded in `docs/ai-blaise/PRODUCTION_READINESS_AUDIT.md`
> and guarded by `ci/ai-blaise/production-gap-audit.sh`.

`kind-timescale-citus-smoke.sh` is the executable kind smoke harness for the
Timescale-on-Citus path. Contract mode runs by default and verifies the chart
and CRD artifacts are present. Live mode requires:

```bash
RUN_KIND_SMOKE=1 \
SMOKE_DB_IMAGE=<image-with-postgres-citus-timescaledb-companion> \
tests/e2e/kind-timescale-citus-smoke.sh
```

Live mode creates a kind cluster, Helm-renders the overlay chart, validates CRD
and chart YAML with `kubectl --dry-run=client`, starts the operand image, loads
Citus and TimescaleDB together, and verifies `companion_feature_status()` when
the `ai_blaise_citus` companion extension is available.

Contract mode is not cohabitation production evidence, and live mode is not run
by default. A live run counts as evidence only when the operand image digest,
command log, and CI or VM run are recorded in the production-readiness audit.

End-to-end acceptance gates run here as the implementation matures.
