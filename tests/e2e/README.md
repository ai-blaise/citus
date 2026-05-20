# tests/e2e

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

End-to-end acceptance gates run here as the implementation matures.
