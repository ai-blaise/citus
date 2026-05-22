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
