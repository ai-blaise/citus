# sidecar/auth

> Production boundary: unless a feature is explicitly `Status: production-ready`
> in `docs/ai-blaise/NEW_FEATURES.md`, the surfaces listed here are alpha
> contracts. Deterministic canonical reports and local runtime models are CI
> artifacts, not production evidence; promotion requires live VM/container or
> Kubernetes evidence recorded in `docs/ai-blaise/PRODUCTION_READINESS_AUDIT.md`
> and guarded by `ci/ai-blaise/production-gap-audit.sh`.

JWT issuer, token introspection, OIDC, and MFA contracts for the auth sidecar.

Current implemented surface:

- `JwtIssueRequest`
- `JwtIssuePlan`
- `TokenClaims`
- `TokenIntrospectionPlan`
- `OidcProviderConfig`
- `MfaPolicy`
- `AuthSidecarPlan`
- `canonical_auth_report()`
- `cargo run -p ai_blaise_citus_sidecar_auth -- run-canonical`

These contracts cover `FEATURE: Auth1`, `FEATURE: Auth2`, `FEATURE: Auth4`,
and `FEATURE: Auth5`.

JWT and OIDC service that backs pool authentication and SQL RLS helpers.
