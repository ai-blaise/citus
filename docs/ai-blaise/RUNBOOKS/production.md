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
