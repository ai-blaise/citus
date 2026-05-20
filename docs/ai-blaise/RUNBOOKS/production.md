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

The first 15 gates are not a blanket production certification for every custom
feature. They verify the V2 acceptance model and the current deployment path.
Before any production promotion, run the production-readiness audit in release
mode and block promotion while alpha or contract-only features remain in
release scope:

```bash
ci/ai-blaise/production-readiness-check.sh production-release
ci/ai-blaise/production-gap-audit.sh
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
  scripts/citus-scale/build-app-images.sh
```

Production traffic tests must use these images, not substitute responder
containers. Production values start the operator and pool with `serve`, while
alpha sidecars remain disabled by default until their feature status is
promoted with measured production evidence. The Kubernetes smoke intentionally
exercises the sidecar image matrix with the chart defaults so image, probe, and
metrics regressions are still caught before a sidecar can be promoted. The pool
must have `AI_BLAISE_POOL_UPSTREAM_ADDR` set and must answer a real PostgreSQL
client query through the `postgres` service port. Probe-only traffic is
insufficient for production signoff.
Run `REQUIRE_DOCKER=1 ci/ai-blaise/pool-proxy-smoke.sh` or the Kubernetes
equivalent before promotion; the accepted result is a successful SQL query
through the pool data port plus ready admin probes and pool traffic metrics.
For a complete VM/container proof, run `ci/ai-blaise/kind-production-smoke.sh`;
it builds the app images, installs the Helm chart, creates a real PostgreSQL
upstream, verifies live SQL through the pool Kubernetes service, and
port-forwards into the live operator plus every sidecar deployment to assert
`/healthz`, `/readyz`, and `/metrics` from the actual pods. It also
port-forwards each pool pod after the SQL smoke and aggregates pool request
metrics across replicas.

## Hardening Controls

- `FEATURE: Sec7`: API keys and cloud credentials are referenced by external
  secret names only.
- `FEATURE: A9`: vector provider keys are bound through external secret
  references rather than database rows.
- `FEATURE: Sec8`: TLS is required for clients, Postgres backends, and
  sidecar-to-sidecar HTTP.
- `FEATURE: Sec9`: release images carry SBOM and cosign attestation records.
- `FEATURE: Sec13`: pool CIDR allowlists are reviewed with every ingress
  change.
- `FEATURE: RT5`: realtime compatibility is verified against Phoenix-channel
  client behavior before a release is promoted.

## Exit Criteria

- All release-scope gates are green for the exact commit and image digest.
- No release-scope feature remains alpha, contract-only, or model-only without
  explicit measured production evidence.
- Runbook evidence links CI runs, Helm render output, image attestations, and
  smoke-test logs.
- Rollback commands and PITR checkpoint are present in the release record.
