# deploy/

The Kubernetes deployment artifacts (Helm chart, values files, CRD bundle, Argo
Application, RBAC, ServiceMonitor, PrometheusRule, NetworkPolicy, PriorityClass,
PodDisruptionBudget, HorizontalPodAutoscaler, sidecar HA + topology-spread layer)
that used to live under `deploy/k8s/` were folded into `ai-blaise/command-center`
on 2026-05-22 per Spencer's directive.

## Canonical location

- Chart: <https://github.com/ai-blaise/command-center/tree/main/helm/charts/citus-cluster>
- Argo Application + values: <https://github.com/ai-blaise/command-center/tree/main/deploy/citus-cluster>
- Migration notes: `deploy/citus-cluster/MIGRATION.md` in command-center.

## What stays here

This repository keeps:

- The Rust extension overlay and operand image source (`companion/`, `operator/`,
  `pool/`, `sidecar/`, `tools/`, `e2e/`, `tests/`, `src/`).
- Citus and PG-core patches (`patches/`).
- The PG-overlay container image build (`images/citus-pg-overlay/`).
- ai-blaise CI workflows that build, test, and publish the images consumed by
  the command-center chart.

## Deploying

Use the command-center chart with Helm or via Argo CD against
`ai-blaise/command-center/gitops/apps/13-citus-cluster.yaml`. Operand images are
still published from this repository.


## Live Kubernetes E2E

This repo now ships the external-chart-aware traffic harness at
`ci/ai-blaise/live-k8s-e2e.sh`. Use `CHART_DIR` to point directly at the
command-center chart, or `COMMAND_CENTER_DIR` to point at a checkout containing
`helm/charts/citus-cluster`.

Dry-run contract smoke:

```bash
COMMAND_CENTER_DIR=/path/to/command-center ci/ai-blaise/deploy-check.sh
```

Real kind traffic with locally built or published images:

```bash
COMMAND_CENTER_DIR=/path/to/command-center \
LIVE_K8S_MODE=kind \
LOCAL_IMAGE_REFS='registry.local/citus:dev' \
AI_BLAISE_STACK_IMAGE_REF=registry.local/citus:dev \
ci/ai-blaise/kind-production-smoke.sh
```

For richer command-center charts, pass chart-specific image values through
`HELM_SET_ARGS`. The harness fails real mode when required image refs, HTTP
probe targets, or SQL service traffic are missing; dry-run output is not live
runtime evidence.
