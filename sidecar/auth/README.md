# sidecar/auth

Production boundary: this crate now backs `FEATURE: Auth1` HS256 auth,
`FEATURE: Auth4` OIDC provider pre-exchange contracts, and `FEATURE: Auth5`
TOTP policy enforcement. External OIDC token exchange, ID-token/JWKS
verification, account linking, WebAuthn ceremonies, persistent runtime loading
from the auth schema, pool data-plane token auth, RS256 issuance, and key
rotation remain outside this boundary until separately live-gated.

The sidecar exposes:

- `POST /auth/users` for local user enrollment.
- `POST /auth/login` for password and optional TOTP login.
- `POST /auth/refresh` for refresh-token exchange.
- `POST /auth/verify` for JWT verification.
- `POST /auth/introspect` for RFC 7662-style active/inactive results.
- `POST /auth/logout` for JTI revocation.
- `POST /auth/mfa/totp/enroll` and `/auth/mfa/totp/verify` for TOTP.
- `GET /auth/oidc/login` for authorization URL generation with state/nonce.
- `GET /auth/oidc/callback` for state/nonce/redirect validation before failing
  closed at the intentionally unimplemented IdP token exchange.
- `/healthz`, `/readyz`, `/metrics`, and `/drain` from the shared sidecar runtime.

`POST /auth/mfa/webauthn/register` and `POST /auth/mfa/webauthn/finish` are
intentional fail-closed alpha boundaries that return `501`.

Required serve-mode configuration:

- `AI_BLAISE_AUTH_HS256_SECRET`: at least 32 bytes.
- `AI_BLAISE_AUTH_ISSUER`: defaults to `https://auth.example.com`.
- `AI_BLAISE_AUTH_AUDIENCE`: defaults to `postgres`.
- `AI_BLAISE_AUTH_TTL_SECONDS`: defaults to `3600` and must be nonzero.
- `AI_BLAISE_AUTH_MFA_MAX_ATTEMPTS`: defaults to `5` and must be nonzero.
- `AI_BLAISE_AUTH_OIDC_PROVIDER_NAME`: optional single OIDC provider name.
- `AI_BLAISE_AUTH_OIDC_ISSUER`: required when an OIDC provider is configured.
- `AI_BLAISE_AUTH_OIDC_AUTHORIZATION_ENDPOINT`: required provider authorize URL.
- `AI_BLAISE_AUTH_OIDC_CLIENT_ID`: required provider client ID.
- `AI_BLAISE_AUTH_OIDC_CLIENT_SECRET_REF`: required secret reference for token exchange.
- `AI_BLAISE_AUTH_OIDC_REDIRECT_URIS`: comma-separated allowlist.
- `AI_BLAISE_AUTH_OIDC_SCOPES`: comma-separated scopes; must include `openid`.
- `AI_BLAISE_LISTEN_ADDR`: defaults to `0.0.0.0:8080`.

The durable auth schema lives in `migrations/0001_auth_schema.sql`. The runtime
currently uses an in-memory user/session/JTI store; persistent loading from the
schema is still alpha. The migration can be applied by an operator-managed job
or manually via `psql -f`.

Verification:

```sh
cargo test -p ai_blaise_citus_sidecar_auth
bash ci/ai-blaise/auth-sidecar-smoke.sh
REQUIRE_DOCKER=1 bash ci/ai-blaise/auth-sidecar-smoke.sh
```
