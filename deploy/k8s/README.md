# deploy/k8s

Slim Kubernetes install surface for ai-blaise/citus. This chart owns only the
ai-blaise overlay components and CRDs; third-party platform charts remain
externally operated.

The initial Helm chart lives at `deploy/k8s/helm/citus-overlay` and includes
operator, pool, sidecar, optional tools, and CRD packaging contracts. It does
not vendor CNPG, monitoring, secrets, ingress, storage, or backup platform
charts; those remain external platform responsibilities.
