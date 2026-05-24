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
- A narrow Kubernetes guardrail contract under `deploy/contracts/` for the
  command-center chart labels and workload names. It renders real HPA,
  PodDisruptionBudget, and NetworkPolicy resources for the ai-blaise operator,
  pool, and sidecar surfaces without reintroducing the folded Helm chart.

## Deploying

Use the command-center chart with Helm or via Argo CD against
`ai-blaise/command-center/gitops/apps/13-citus-cluster.yaml`.

Operand image source stays in this repository, but source presence is not proof
of publication. A production handoff requires the release image build/push to
write `artifacts/ai-blaise-image-digests.tsv` and the command-center deploy
overlay to consume immutable `sha256:` rows from that manifest. At minimum,
carry the operator and pool rows through `OPERATOR_IMAGE_DIGEST` and
`POOL_IMAGE_DIGEST` or their equivalent Helm values before installing a release
candidate. Mutable tags or locally loaded images are valid only for local smoke
runs and must not be cited as release image-pinning evidence.

Before promoting Citus-side image/runtime changes, run
`make -f Makefile.ai-blaise deploy-check`. The check validates that
`deploy/contracts/k8s-production-guardrails.yaml` is in sync with the renderer,
that the rendered manifest covers the expected production guardrail resources,
and, when available on the runner, that kustomize and kubeconform accept the
manifest. Full Helm values rendering remains owned by `ai-blaise/command-center`.
