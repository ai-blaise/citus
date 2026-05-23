# sidecar/auth

Production boundary: the only production-ready surface in this crate is the
local HS256 auth sidecar runtime documented as `FEATURE: Auth1` in
`docs/ai-blaise/NEW_FEATURES.md`. OIDC exchanges, WebAuthn ceremonies,
persistent runtime loading from the auth schema, pool data-plane token auth,
RS256/JWKS discovery, and key rotation remain alpha until separately live-gated.

The sidecar exposes:

- `POST /auth/users` for local user enrollment.
- `POST /auth/login` for password and optional TOTP login.
- `POST /auth/refresh` for refresh-token exchange.
- `POST /auth/verify` for JWT verification.
- `POST /auth/introspect` for RFC 7662-style active/inactive results.
- `POST /auth/logout` for JTI revocation.
- `POST /auth/mfa/totp/enroll` and `/auth/mfa/totp/verify` for TOTP.
- `/healthz`, `/readyz`, `/metrics`, and `/drain` from the shared sidecar runtime.

`POST /auth/mfa/webauthn/register`, `POST /auth/mfa/webauthn/finish`,
`GET /auth/oidc/login`, and `GET /auth/oidc/callback` are intentional
fail-closed alpha boundaries that return `501`.

Required serve-mode configuration:

- `AI_BLAISE_AUTH_HS256_SECRET`: at least 32 bytes.
- `AI_BLAISE_AUTH_ISSUER`: defaults to `https://auth.example.com`.
- `AI_BLAISE_AUTH_AUDIENCE`: defaults to `postgres`.
- `AI_BLAISE_AUTH_TTL_SECONDS`: defaults to `3600` and must be nonzero.
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
