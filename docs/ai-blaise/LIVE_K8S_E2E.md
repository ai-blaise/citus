# Live Kubernetes E2E Harness

`ci/ai-blaise/live-k8s-e2e.sh` is the production traffic harness that remains
in this repository after the Helm chart moved to `ai-blaise/command-center`.
It never vendors or regenerates `deploy/k8s/`; it renders the external chart
provided by `CHART_DIR` or `COMMAND_CENTER_DIR`, installs it only in real modes,
and records artifacts under `artifacts/live-k8s-e2e/`.

## Modes

- `LIVE_K8S_MODE=dry-run`: CI-safe contract smoke. If a chart is supplied, the
  harness runs `helm lint`, `helm template`, and client-side Kubernetes dry-run
  validation, then prints the rendered image set. It does not send live HTTP or
  SQL traffic and is not runtime evidence.
- `LIVE_K8S_MODE=real`: installs into the current Kubernetes context, waits for
  rollouts and ready pods, then sends traffic through `kubectl port-forward`.
- `LIVE_K8S_MODE=kind`: creates or reuses a kind cluster, loads any
  `LOCAL_IMAGE_REFS`, installs the same chart, then runs the same traffic path.

## Required Inputs For Real Traffic

```bash
COMMAND_CENTER_DIR=/path/to/command-center LIVE_K8S_MODE=kind LOCAL_IMAGE_REFS='registry.local/citus:dev' AI_BLAISE_STACK_IMAGE_REF=registry.local/citus:dev ci/ai-blaise/kind-production-smoke.sh
```

Use `CHART_DIR=/path/to/helm/charts/citus-cluster` when the command-center
checkout has a nonstandard layout. For charts whose image values are not simple
`image.repository` plus `image.tag`, pass chart-specific overrides through
`HELM_SET_ARGS='--set-string key=value ...'`.

The harness checks rendered image references before a real install. If a branch
image is not published yet, pass locally built image refs through
`LOCAL_IMAGE_REFS` for kind and through the chart image overrides. Setting
`ALLOW_UNPUBLISHED_IMAGES=1` only disables the preflight; it does not make image
pull failures acceptable evidence.

## Traffic Contract

Real mode defaults to fail-closed in `ci/ai-blaise/kind-production-smoke.sh`:
`REQUIRE_HTTP=1` and `REQUIRE_SQL=1`. The harness discovers HTTP service ports
named `http`, `admin`, `metrics`, or `web` and probes `/healthz`, `/readyz`, and
`/metrics`. It discovers SQL services by port `5432` or PostgreSQL-like port
names and runs a write/read `psql` script through a local port-forward.

Set `HTTP_SERVICE`/`HTTP_PORT` or `SQL_SERVICE`/`SQL_PORT` when the chart uses
nonstandard service names. Set `SQL_TEST_FILE` or `SQL_TEST_SQL` for a
release-specific SQL workload.

## Failure Artifacts

On failure, the harness collects namespace events, workload descriptions,
service state, pod logs, port-forward logs, and `helm get all`. It tears down by
default; use `KEEP_NAMESPACE_ON_FAILURE=1` while debugging a live cluster.
Namespace deletion is limited to namespaces that did not exist before the
harness invoked Helm, so a failed test cannot remove a pre-existing namespace.
