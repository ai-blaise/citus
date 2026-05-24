#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "${repo_root}"

export PATH="${HOME}/.cargo/bin:${PATH}"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-/tmp/cargo-target-auth-smoke}"
port="${AI_BLAISE_AUTH_SMOKE_PORT:-18081}"
base_url="http://127.0.0.1:${port}"
secret="${AI_BLAISE_AUTH_HS256_SECRET:-auth-smoke-secret-32-bytes-minimum-key-material}"
log_file="$(mktemp -t auth-sidecar-smoke.XXXXXX.log)"
server_pid=""
pg_container=""

cleanup() {
  if [[ -n "${server_pid}" ]] && kill -0 "${server_pid}" >/dev/null 2>&1; then
    kill "${server_pid}" >/dev/null 2>&1 || true
    wait "${server_pid}" >/dev/null 2>&1 || true
  fi
  if [[ -n "${pg_container}" ]]; then
    docker rm -f "${pg_container}" >/dev/null 2>&1 || true
  fi
  rm -f "${log_file}"
}
trap cleanup EXIT

curl_json() {
  local method="$1"
  local path="$2"
  local body="${3:-}"
  local output
  output="$(mktemp -t auth-sidecar-curl.XXXXXX)"
  local status
  if [[ "${method}" == "GET" ]]; then
    status="$(curl -sS -o "${output}" -w '%{http_code}' "${base_url}${path}")"
  else
    status="$(curl -sS -o "${output}" -w '%{http_code}' -X "${method}" -H 'content-type: application/json' --data "${body}" "${base_url}${path}")"
  fi
  printf '%s\t%s\n' "${status}" "$(cat "${output}")"
  rm -f "${output}"
}

require_status() {
  local expected="$1"
  local response="$2"
  local status="${response%%$'\t'*}"
  if [[ "${status}" != "${expected}" ]]; then
    echo "expected HTTP ${expected}, got ${status}: ${response}" >&2
    return 1
  fi
}

json_field() {
  local json="$1"
  local field="$2"
  python3 - "${json}" "${field}" <<'PY'
import json
import sys
body = json.loads(sys.argv[1])
value = body[sys.argv[2]]
print(value)
PY
}

python3 - <<'PY'
import socket
s=socket.socket()
try:
    s.connect(("127.0.0.1", int(__import__('os').environ.get('AI_BLAISE_AUTH_SMOKE_PORT', '18081'))))
except OSError:
    raise SystemExit(0)
raise SystemExit("auth smoke port is already in use")
PY

AI_BLAISE_LISTEN_ADDR="127.0.0.1:${port}" \
AI_BLAISE_AUTH_ISSUER="https://auth.example.com" \
AI_BLAISE_AUTH_AUDIENCE="postgres" \
AI_BLAISE_AUTH_TTL_SECONDS="300" \
AI_BLAISE_AUTH_HS256_SECRET="${secret}" \
cargo run -q -p ai_blaise_citus_sidecar_auth -- serve >"${log_file}" 2>&1 &
server_pid="$!"

for _ in $(seq 1 80); do
  if curl -fsS "${base_url}/healthz" >/dev/null 2>&1; then
    break
  fi
  if ! kill -0 "${server_pid}" >/dev/null 2>&1; then
    cat "${log_file}" >&2 || true
    echo "auth sidecar exited before becoming healthy" >&2
    exit 1
  fi
  sleep 0.25
done

health="$(curl_json GET /healthz)"
ready="$(curl_json GET /readyz)"
metrics="$(curl_json GET /metrics)"
require_status 200 "${health}"
require_status 200 "${ready}"
require_status 200 "${metrics}"
[[ "${metrics}" == *'ai_blaise_sidecar_ready{component="auth-sidecar"} 1'* ]]

register="$(curl_json POST /auth/users '{"username":"alice","password":"hunter2-correct-horse","role":"authenticated","tenant_id":"tenant-a"}')"
require_status 201 "${register}"
login="$(curl_json POST /auth/login '{"username":"alice","password":"hunter2-correct-horse"}')"
require_status 200 "${login}"
login_body="${login#*$'\t'}"
access_token="$(json_field "${login_body}" access_token)"
refresh_token="$(json_field "${login_body}" refresh_token)"

verify="$(curl_json POST /auth/verify "{\"token\":\"${access_token}\"}")"
require_status 200 "${verify}"
[[ "${verify}" == *'"tenant_id":"tenant-a"'* ]]

introspect="$(curl_json POST /auth/introspect "{\"token\":\"${access_token}\"}")"
require_status 200 "${introspect}"
[[ "${introspect}" == *'"active":true'* ]]

refresh="$(curl_json POST /auth/refresh "{\"refresh_token\":\"${refresh_token}\"}")"
require_status 200 "${refresh}"
refresh_body="${refresh#*$'\t'}"
access_token="$(json_field "${refresh_body}" access_token)"

logout="$(curl_json POST /auth/logout "{\"token\":\"${access_token}\"}")"
require_status 200 "${logout}"
verify_after_logout="$(curl_json POST /auth/verify "{\"token\":\"${access_token}\"}")"
require_status 401 "${verify_after_logout}"
introspect_after_logout="$(curl_json POST /auth/introspect "{\"token\":\"${access_token}\"}")"
require_status 200 "${introspect_after_logout}"
[[ "${introspect_after_logout}" == *'"active":false'* ]]
refresh_after_logout="$(curl_json POST /auth/refresh "{\"refresh_token\":\"${refresh_token}\"}")"
require_status 401 "${refresh_after_logout}"

register_bob="$(curl_json POST /auth/users '{"username":"bob","password":"hunter2-correct-horse","role":"authenticated","tenant_id":"tenant-b"}')"
require_status 201 "${register_bob}"
enroll="$(curl_json POST /auth/mfa/totp/enroll '{"username":"bob"}')"
require_status 200 "${enroll}"
enroll_body="${enroll#*$'\t'}"
secret_base32="$(json_field "${enroll_body}" secret_base32)"
missing_totp="$(curl_json POST /auth/login '{"username":"bob","password":"hunter2-correct-horse"}')"
require_status 401 "${missing_totp}"
totp_code="$(python3 - "${secret_base32}" <<'PY'
import base64
import hmac
import hashlib
import struct
import sys
import time
secret = base64.b32decode(sys.argv[1] + '=' * ((8 - len(sys.argv[1]) % 8) % 8))
counter = int(time.time() // 30)
digest = hmac.new(secret, struct.pack('>Q', counter), hashlib.sha1).digest()
offset = digest[-1] & 0x0F
code = (struct.unpack('>I', digest[offset:offset+4])[0] & 0x7fffffff) % 1_000_000
print(f"{code:06d}")
PY
)"
login_bob="$(curl_json POST /auth/login "{\"username\":\"bob\",\"password\":\"hunter2-correct-horse\",\"totp_code\":\"${totp_code}\"}")"
require_status 200 "${login_bob}"
[[ "${login_bob}" == *'"mfa_verified":true'* ]]

webauthn="$(curl_json POST /auth/mfa/webauthn/register '{}')"
require_status 501 "${webauthn}"
oidc="$(curl_json GET /auth/oidc/login)"
require_status 501 "${oidc}"

if [[ "${REQUIRE_DOCKER:-0}" == "1" ]]; then
  command -v docker >/dev/null
  pg_container="auth-sidecar-smoke-${RANDOM}-${RANDOM}"
  docker run -d --name "${pg_container}" -e POSTGRES_PASSWORD=postgres postgres:17 >/dev/null
  pg_ready=0
  init_complete=0
  for _ in $(seq 1 120); do
    if docker logs "${pg_container}" 2>&1 | grep -Fq 'PostgreSQL init process complete'; then
      init_complete=1
    fi
    if [[ "${init_complete}" == "1" ]] && docker exec "${pg_container}" psql -U postgres -d postgres -Atqc 'SELECT 1' 2>/dev/null | grep -qx '1'; then
      pg_ready=1
      break
    fi
    sleep 0.5
  done
  if [[ "${pg_ready}" != "1" ]]; then
    docker logs "${pg_container}" >&2 || true
    echo "auth schema smoke postgres container did not become SQL-ready" >&2
    exit 1
  fi
  docker exec -i "${pg_container}" psql -v ON_ERROR_STOP=1 -U postgres -d postgres < sidecar/auth/migrations/0001_auth_schema.sql >/tmp/auth-schema-smoke.out
  docker exec "${pg_container}" psql -v ON_ERROR_STOP=1 -U postgres -d postgres -Atc "select count(*) from information_schema.tables where table_schema='auth' and table_name in ('auth_users','auth_sessions','auth_mfa_totp','auth_mfa_webauthn','auth_oidc_providers','auth_revoked_jtis');" | grep -qx '6'
fi

printf 'auth_sidecar_smoke\tjwt=true\tintrospection=true\tlogout=true\trefresh=true\ttotp=true\twebauthn_alpha=true\toidc_alpha=true\tdocker_schema=%s\n' "${REQUIRE_DOCKER:-0}"
