-- FEATURE: Auth1
-- FEATURE: Auth2
-- FEATURE: Auth4
-- FEATURE: Auth5
--
-- Auth sidecar persistence schema. Owned by the auth sidecar service account
-- (`auth_service_role`); the pool, companion, and operator do not read these
-- tables directly. SQL session claim helpers (`companion_set_session_claims`)
-- ship from the companion extension and consume verified claims handed to
-- them by the pool after the auth sidecar validates a JWT.
--
-- This migration is applied by the operator-managed auth-schema CronJob; it
-- is also runnable manually via `psql -f` against the auth-sidecar database.

CREATE EXTENSION IF NOT EXISTS pgcrypto;

CREATE SCHEMA IF NOT EXISTS auth;

-- auth_users: identity records. Passwords are stored as PBKDF2-SHA256 hashes
-- ({"v":1,"iter":...,"salt":"...","hash":"..."}) so the runtime can rotate
-- the iteration count and salt length without a schema change.
CREATE TABLE IF NOT EXISTS auth.auth_users (
    id              uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    username        text NOT NULL,
    tenant_id       text NOT NULL,
    role            text NOT NULL DEFAULT 'authenticated',
    password_hash   jsonb NOT NULL,
    mfa_required    boolean NOT NULL DEFAULT false,
    disabled_at     timestamptz,
    created_at      timestamptz NOT NULL DEFAULT now(),
    updated_at      timestamptz NOT NULL DEFAULT now(),
    UNIQUE (tenant_id, username)
);

CREATE INDEX IF NOT EXISTS auth_users_tenant_id_idx
    ON auth.auth_users (tenant_id);
CREATE INDEX IF NOT EXISTS auth_users_username_idx
    ON auth.auth_users (username);

-- auth_sessions: durable shape for refresh tokens + issued access tokens (by
-- JTI). The current runtime smoke uses an in-memory session map; persistent
-- runtime loading from these tables remains alpha until separately live-gated.
CREATE TABLE IF NOT EXISTS auth.auth_sessions (
    jti             text PRIMARY KEY,
    user_id         uuid NOT NULL REFERENCES auth.auth_users(id) ON DELETE CASCADE,
    tenant_id       text NOT NULL,
    refresh_token   text NOT NULL UNIQUE,
    issued_at       timestamptz NOT NULL DEFAULT now(),
    expires_at      timestamptz NOT NULL,
    revoked_at      timestamptz,
    mfa_verified    boolean NOT NULL DEFAULT false
);

CREATE INDEX IF NOT EXISTS auth_sessions_user_id_idx
    ON auth.auth_sessions (user_id);
CREATE INDEX IF NOT EXISTS auth_sessions_tenant_id_idx
    ON auth.auth_sessions (tenant_id);
CREATE INDEX IF NOT EXISTS auth_sessions_expires_at_idx
    ON auth.auth_sessions (expires_at);

-- auth_mfa_totp: one TOTP secret per user, base32-encoded.
CREATE TABLE IF NOT EXISTS auth.auth_mfa_totp (
    user_id         uuid PRIMARY KEY REFERENCES auth.auth_users(id) ON DELETE CASCADE,
    secret_base32   text NOT NULL,
    period_seconds  int NOT NULL DEFAULT 30,
    digits          int NOT NULL DEFAULT 6,
    algorithm       text NOT NULL DEFAULT 'SHA1',
    enrolled_at     timestamptz NOT NULL DEFAULT now()
);

-- auth_mfa_webauthn: one row per registered credential. The runtime ships
-- this surface as `Status: alpha` until the webauthn ceremony plumbing is
-- promoted; the table exists so the migration is forward-compatible.
CREATE TABLE IF NOT EXISTS auth.auth_mfa_webauthn (
    credential_id     text PRIMARY KEY,
    user_id           uuid NOT NULL REFERENCES auth.auth_users(id) ON DELETE CASCADE,
    public_key_cose   bytea NOT NULL,
    sign_count        bigint NOT NULL DEFAULT 0,
    transports        text[] NOT NULL DEFAULT ARRAY[]::text[],
    registered_at     timestamptz NOT NULL DEFAULT now(),
    last_used_at      timestamptz
);

CREATE INDEX IF NOT EXISTS auth_mfa_webauthn_user_id_idx
    ON auth.auth_mfa_webauthn (user_id);

-- auth_oidc_providers: declarative OIDC client configuration. Secrets are
-- mounted from External Secrets Operator-backed Kubernetes Secrets and only
-- referenced here by name.
CREATE TABLE IF NOT EXISTS auth.auth_oidc_providers (
    name                    text PRIMARY KEY,
    issuer_url              text NOT NULL,
    client_id_secret_ref    text NOT NULL,
    client_secret_ref       text NOT NULL,
    scopes                  text[] NOT NULL,
    enabled                 boolean NOT NULL DEFAULT true,
    created_at              timestamptz NOT NULL DEFAULT now()
);

-- auth_revoked_jtis: durable shape for a persistent JTI revocation list. The
-- current runtime keeps revocations in memory; persistent seeding and GC remain
-- alpha until separately live-gated.
CREATE TABLE IF NOT EXISTS auth.auth_revoked_jtis (
    jti           text PRIMARY KEY,
    revoked_at    timestamptz NOT NULL DEFAULT now(),
    expires_at    timestamptz NOT NULL,
    reason        text
);

CREATE INDEX IF NOT EXISTS auth_revoked_jtis_expires_at_idx
    ON auth.auth_revoked_jtis (expires_at);
