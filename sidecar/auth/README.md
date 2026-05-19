# sidecar/auth

JWT issuer, token introspection, OIDC, and MFA contracts for the auth sidecar.

Current implemented surface:

- `JwtIssueRequest`
- `JwtIssuePlan`
- `TokenClaims`
- `TokenIntrospectionPlan`
- `OidcProviderConfig`
- `MfaPolicy`
- `AuthSidecarPlan`

These contracts cover `FEATURE: Auth1`, `FEATURE: Auth2`, `FEATURE: Auth4`,
and `FEATURE: Auth5`.

JWT and OIDC service that backs pool authentication and SQL RLS helpers.
