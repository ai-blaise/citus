# End-to-End Acceptance

The critical-path acceptance harness starts in `e2e/`. It is a pure Rust model
paired with the kind/PostgreSQL smoke harness, and it exercises the same
contract the real cluster test must satisfy:

1. validate the `CitusCluster` topology and extension surface
2. validate the `ShardGroup` topology-aware placement policy
3. preload Citus and TimescaleDB together
4. configure `citus.cohabit_extensions = 'timescaledb'`
5. create a `Hypertable` spec with compression, retention, and CAGG policy
6. map that spec through the operator reconciler into a guarded apply plan
7. record the gates the database-backed runner must prove

The first scenario is `TimescaleOnCitusAcceptance::canonical_metrics()`.
`cargo run -p ai_blaise_citus_e2e --bin timescale_apply_plan` emits the
canonical guarded SQL script for that scenario, including `CREATE EXTENSION
ai_blaise_citus`, `companion_feature_status()` checks, the Timescale/Citus
cohabitation guard, and the ordered TS1/TS2/TS3/TS4/TS5 companion SQL.

`V2OperatorCatalogAcceptance::canonical_platform()` validates the rest of the
V2 operator catalog in one model: branches, tenants, regions, survival goals,
backups, vectorizers, sidecar deployments, migrations, conflict policies,
federation, search indexes, webhooks, functions, and scheduled repacks.

`V2RuntimeContractAcceptance::canonical_runtime()` validates the pool and
sidecar runtime layer: CDC delivery, realtime, auth, storage, backup/restore,
repack, analytical mirrors, settings-bucket pooling, fast-path routing,
mirroring, HTAP routing, pipelining, TLS reuse, tenant admission, Geo routing,
and token-cache behavior.

The V2 release-gate report is not measured production evidence by itself. It
records the canonical gate shape and expected thresholds that live harnesses
must later prove. `ci/ai-blaise/production-gap-audit.sh` keeps that boundary
machine-checkable so canonical model output cannot promote alpha features.


## Live Kubernetes Traffic

`ci/ai-blaise/live-k8s-e2e.sh` is the live deployment and traffic harness for
real clusters and kind. Because the chart lives in `ai-blaise/command-center`,
real runs must provide `CHART_DIR` or `COMMAND_CENTER_DIR` plus image refs that
the target cluster can pull or that kind can load through `LOCAL_IMAGE_REFS`.
The harness waits for Kubernetes readiness, probes HTTP endpoints through
port-forwards, runs `psql` through the exposed PostgreSQL service path, collects
logs/events on failure, and tears down safely by default.

`ci/ai-blaise/deploy-check.sh` and the default `kind-production-smoke` mode are
CI-safe dry-runs. They validate rendering when a chart is supplied, but they do
not send HTTP or SQL traffic and must not be cited as production runtime
evidence.
