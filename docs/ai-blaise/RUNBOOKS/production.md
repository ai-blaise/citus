# Production Runbook

`FEATURE: D10`

Production deployments must run through the continuous gates before release.

## Required Gates

1. Cohabitation.
2. Plan-cache invalidation.
3. Latency.
4. HA.
5. Branching.
6. Vectorizer.
7. Search.
8. HTAP.
9. Multi-region.
10. Performance.
11. Chaos.
12. Upstream merge.
13. Slop scan.
14. Feature docs.
15. License.
16. Production-readiness audit.
17. Production gap audit.
18. Docs evidence boundary audit.

These gates are not a blanket production certification for every custom
feature. They verify the V2 acceptance model, the current deployment path, and
the docs evidence boundaries. Before any production promotion, run the
production-readiness audit in release mode and block promotion while alpha or
contract-only features remain in release scope:

```bash
ci/ai-blaise/production-readiness-check.sh production-release
ci/ai-blaise/production-gap-audit.sh
ci/ai-blaise/docs-evidence-boundary-check.sh
```

The production-gap-audit gate is intentionally conservative: it asserts that
V2 acceptance is a modeled prerequisite, that production-release mode remains
blocked while alpha features exist, and that SQL/Kubernetes smoke tests still
exercise live runtime behavior.

## Runtime Image Gate

`FEATURE: D13`

Before promotion, build the real Rust runtime image matrix:

```bash
IMAGE_REGISTRY=ghcr.io/ai-blaise TAG="${RELEASE_TAG}" \
  DIGEST_FILE=artifacts/ai-blaise-image-digests.tsv PUSH=true \
  scripts/citus-scale/build-app-images.sh
```

For release builds, `scripts/citus-scale/build-app-images.sh` writes
`artifacts/ai-blaise-image-digests.tsv` with repository, tag, package, binary,
push status, and immutable repo digest. A pushed image without a reported
`sha256:` digest fails the build. Use the operator and pool rows from that
manifest as `OPERATOR_IMAGE_DIGEST` and `POOL_IMAGE_DIGEST` for production
render/install.

Production traffic tests must use these images, not substitute responder
containers. Production values start the operator and pool with `serve`, service
images default to `serve`, and the `citusctl` tool image defaults to `plan
inspect cluster`. Alpha sidecars remain disabled by default until their feature
status is promoted with measured production evidence. The Kubernetes smoke
intentionally exercises the sidecar image matrix with the chart defaults so
image, probe, and metrics regressions are still caught before a sidecar can be
promoted; it also runs the built `citusctl` image as a Job and requires the
expected plan output. The pool must have `AI_BLAISE_POOL_UPSTREAM_ADDR` set and
must answer a real PostgreSQL client query through the `postgres` service port.
Probe-only traffic is insufficient for production signoff.
Run `REQUIRE_DOCKER=1 ci/ai-blaise/pool-proxy-smoke.sh` or the Kubernetes
equivalent before promotion; the accepted result is a successful SQL query
through the pool data port plus ready admin probes and pool traffic metrics.
For a complete VM/container proof, run `ci/ai-blaise/kind-production-smoke.sh`;
it builds the app images, installs the explicit `values-exhaustive.yaml`
image-matrix profile, creates a real PostgreSQL upstream, verifies live SQL
through the pool Kubernetes service, and port-forwards into the live operator
plus every sidecar deployment to assert `/healthz`, `/readyz`, and `/metrics`
from the actual pods. It also installs the default `values.yaml` profile with
direct Helm and the `values-prod.yaml` profile through the deploy wrapper,
verifying that production-safe values run the operator and pool with alpha
sidecars/tools disabled while pool SQL/admin traffic still works. The deploy
workflow and `gate-close` run this smoke at larger integration boundaries.

The Makefile smoke targets set `REQUIRE_DOCKER=1` for the Docker-backed live
smokes, including pool proxy, SQL extension, real TimescaleDB bridge, real
Citus+TimescaleDB cohabitation, and primary/standby observability
replication. Missing Docker is therefore a release gate failure, not skipped
evidence. Running the direct scripts without `REQUIRE_DOCKER=1` is only for
exploratory local checks.

The Makefile release gate also runs the image and deploy contract checks
directly. After the 2026-05-22 chart fold, this repository's `deploy-check`
validates the Citus-side HPA, PodDisruptionBudget, and NetworkPolicy contract
under `deploy/contracts/`; full Helm values rendering remains owned by
`ai-blaise/command-center`. When kustomize or kubeconform is present on the
runner, the Citus-side check also renders and schema-validates the manifest.

The `values-prod.yaml` phase of the Kubernetes smoke installs through
`scripts/citus-scale/deploy.sh MODE=install`, so the production-safe deploy
wrapper install path is live-gated. The optional `tools` Deployment is dev-only
until separately promoted; production evidence executes the built `citusctl`
image through the smoke Job. The Argo application manifest is a GitOps render
contract unless an Argo controller-backed sync is run and recorded.

The `scripts/citus-scale/deploy.sh` deploy wrapper defaults to
`values-prod.yaml` through `DEPLOY_PROFILE=prod`. Rendering dev or exhaustive
profiles is allowed for review and smoke-test work, but installing any
non-production or custom values file is blocked unless `ALLOW_ALPHA_INSTALL=1`
is set explicitly for that run.

Production renders and installs require immutable operator and pool image
digests. The default `values.yaml` profile and `values-prod.yaml` both set
`global.requireImageDigest: true`; pass
`OPERATOR_IMAGE_DIGEST=sha256:...` and `POOL_IMAGE_DIGEST=sha256:...` (or the
equivalent Helm values) for release candidates. `ALLOW_MUTABLE_IMAGE_TAGS=1`
is only for local/dev smoke work with locally loaded images and must not be
used as release image-pinning evidence. GitOps sync intentionally fails closed
until the release branch or deployment overlay supplies the operator and pool
digests.

## Hardening Controls

Most runtime and security controls are alpha intent, not active production
enforcement, until the Helm chart renders the corresponding runtime settings,
ExternalSecret, TLS, and release-attestation objects and the VM smoke verifies
them. Production values keep those alpha controls disabled by default; enabling
any of them requires measured evidence and a feature status promotion.

`FEATURE: Sec13` pool CIDR access control is production-ready for the pool data
port: Helm renders `AI_BLAISE_POOL_CLIENT_CIDR_ALLOWLIST` into the pool
deployment, renders a matching NetworkPolicy when an allowlist is configured,
the pool rejects PostgreSQL clients outside the allowlist before opening an
upstream connection, and the Docker/kind smokes verify both allowed and denied
SQL traffic plus `ai_blaise_citus_pool_rejected_connections_total`.
The Citus-side deployment contract also renders the pool NetworkPolicy shape
that command-center must preserve, including the pool data-port allowlist,
admin probe access, and matching selectors for the folded chart labels.

- `FEATURE: Sec7`: API keys and cloud credentials will be referenced by
  external secret names only after ExternalSecret rendering is implemented and
  verified.
- `FEATURE: A9`: vector provider keys will be bound through external secret
  references rather than database rows after the same rendering and smoke proof
  exists.
- `FEATURE: Sec8`: TLS for clients, Postgres backends, and sidecar-to-sidecar
  HTTP remains an alpha intent until certificates, mounts, probes, and traffic
  tests are wired.
- `FEATURE: Sec9`: SBOM and cosign attestation records remain release-intent
  gates until the release workflow publishes and verifies them for each image.
- `FEATURE: T6`: PG18 `io_uring` remains alpha intent until the operand image
  and kernel/runtime compatibility smoke prove it on the target node class.
- `FEATURE: T7`: pool protocol pipelining remains alpha intent until the pool
  data path enforces it and live SQL traffic proves correctness under pipelined
  clients.
- `FEATURE: RT5`: realtime compatibility is verified against Phoenix-channel
  client behavior before a release is promoted.

## Exit Criteria

- All release-scope gates are green for the exact commit and image digest.
- `values.yaml` and `values-prod.yaml` render with immutable operator and pool
  image digests.
- No release-scope feature remains alpha, contract-only, or model-only without
  explicit measured production evidence.
- Runbook evidence links CI runs, Helm render output, image attestations, and
  smoke-test logs.
- Rollback commands and PITR checkpoint are present in the release record.
