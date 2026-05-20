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

## Runtime Image Gate

`FEATURE: D13`

Before promotion, build the real Rust runtime image matrix:

```bash
IMAGE_REGISTRY=ghcr.io/ai-blaise TAG="${RELEASE_TAG}" \
  scripts/citus-scale/build-app-images.sh
```

Production traffic tests must use these images, not substitute responder
containers. The Kubernetes chart starts the operator, pool, and sidecars with
`serve`. The pool must have `AI_BLAISE_POOL_UPSTREAM_ADDR` set and must answer
a real PostgreSQL client query through the `postgres` service port. Probe-only
traffic is insufficient for production signoff.
Run `REQUIRE_DOCKER=1 ci/ai-blaise/pool-proxy-smoke.sh` or the Kubernetes
equivalent before promotion; the accepted result is a successful SQL query
through the pool data port plus ready admin probes and pool traffic metrics.
For a complete VM/container proof, run `ci/ai-blaise/kind-production-smoke.sh`;
it builds the app images, installs the Helm chart, creates a real PostgreSQL
upstream, and verifies live SQL through the pool Kubernetes service.

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

- All gates are green for the exact commit and image digest.
- Runbook evidence links CI runs, Helm render output, image attestations, and
  smoke-test logs.
- Rollback commands and PITR checkpoint are present in the release record.
