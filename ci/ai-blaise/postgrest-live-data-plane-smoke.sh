#!/usr/bin/env bash
set -euo pipefail

# FEATURE: API1 API2 API5
# Live PostgREST data-plane proof for the REST auto-API path. This smoke uses
# a real PostgreSQL server, the official PostgREST binary, the ai-blaise
# supervisor command, and the ai-blaise sidecar proxy over loopback TCP.

repo_root="$(git rev-parse --show-toplevel)"
cd "${repo_root}"

if [[ -f "${HOME}/.cargo/env" ]]; then
  # shellcheck disable=SC1090
  source "${HOME}/.cargo/env"
fi

python3 <<'PY_SMOKE'
import base64
import hashlib
import hmac
import http.client
import json
import os
from pathlib import Path
import shutil
import socket
import subprocess
import sys
import tempfile
import time

TARGET_DIR = os.environ.get("CARGO_TARGET_DIR", os.path.join(os.getcwd(), "target"))
if not os.path.isabs(TARGET_DIR):
    TARGET_DIR = os.path.abspath(TARGET_DIR)

POSTGREST_PKG = "ai_blaise_citus_sidecar_postgrest"
DEFAULT_DATABASE_IMAGE = "ai-blaise-citus-timescale-cohabitation:local"
POSTGRES_IMAGE = os.environ.get("POSTGRES_IMAGE", DEFAULT_DATABASE_IMAGE)
EXPECT_CITUS = os.environ.get("POSTGREST_LIVE_EXPECT_CITUS", "1") != "0"
POSTGREST_IMAGE = os.environ.get("POSTGREST_IMAGE", "postgrest/postgrest:v12.2.12")
JWT_SECRET = "01234567890123456789012345678901"
JWT_AUD = "ai-blaise Citus API"
PG_PASSWORD = "postgres"
AUTHENTICATOR_PASSWORD = "postgrest_authenticator_secret"
ARTIFACT = Path("artifacts/postgrest-live-data-plane-evidence.tsv")
COHAB_DOCKERFILE = Path("images/citus-timescale-cohabitation/Dockerfile")

processes = []
containers = []


def fail(message):
    raise AssertionError(message)


def require_tool(name):
    if shutil.which(name) is None:
        fail(f"{name} is required for live PostgREST data-plane smoke")


def run(args, *, env=None, input_text=None, timeout=180, check=True):
    result = subprocess.run(
        args,
        input=input_text,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        env=env,
        timeout=timeout,
    )
    if check and result.returncode != 0:
        sys.stderr.write(result.stdout)
        sys.stderr.write(result.stderr)
        fail(f"command failed with exit {result.returncode}: {' '.join(args)}")
    return result


def free_port():
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        sock.bind(("127.0.0.1", 0))
        return sock.getsockname()[1]


def request(port, method, path, body=None, headers=None, timeout=10):
    headers = dict(headers or {})
    if body is not None and not any(key.lower() == "content-type" for key in headers):
        headers["content-type"] = "application/json"
    conn = http.client.HTTPConnection("127.0.0.1", port, timeout=timeout)
    conn.request(method, path, body=body, headers=headers)
    response = conn.getresponse()
    response_body = response.read().decode("utf-8")
    conn.close()
    return response.status, response_body


def wait_http(port, path, component):
    last_error = None
    for _ in range(120):
        try:
            status, body = request(port, "GET", path, timeout=2)
            if status < 500:
                return status, body
        except OSError as exc:
            last_error = exc
        for proc, name in list(processes):
            if proc.poll() is not None:
                stdout = proc.stdout.read() if proc.stdout is not None else ""
                stderr = proc.stderr.read() if proc.stderr is not None else ""
                fail(f"{name} exited before {component} became ready\nSTDOUT:\n{stdout}\nSTDERR:\n{stderr}")
        time.sleep(0.25)
    fail(f"{component} did not become ready on port {port}: {last_error}")


def wait_sidecar_ready(proc, port):
    for _ in range(120):
        try:
            status, body = request(port, "GET", "/readyz", timeout=2)
            if status == 200 and '"component":"postgrest"' in body:
                return
        except OSError:
            pass
        if proc.poll() is not None:
            stdout = proc.stdout.read() if proc.stdout is not None else ""
            stderr = proc.stderr.read() if proc.stderr is not None else ""
            fail(f"sidecar exited before readiness\nSTDOUT:\n{stdout}\nSTDERR:\n{stderr}")
        time.sleep(0.25)
    fail("sidecar did not become ready")


def terminate_process(proc, name):
    if proc.poll() is not None:
        return
    if proc.stdin is not None:
        try:
            proc.stdin.close()
        except BrokenPipeError:
            pass
    try:
        proc.wait(timeout=10)
    except subprocess.TimeoutExpired:
        proc.terminate()
        try:
            proc.wait(timeout=10)
        except subprocess.TimeoutExpired:
            proc.kill()
            proc.wait(timeout=10)
    if proc.returncode not in (0, -15, -9):
        stdout = proc.stdout.read() if proc.stdout is not None else ""
        stderr = proc.stderr.read() if proc.stderr is not None else ""
        fail(f"{name} exited with {proc.returncode}\nSTDOUT:\n{stdout}\nSTDERR:\n{stderr}")


def cleanup():
    for proc, name in reversed(processes):
        terminate_process(proc, name)
    for container in containers:
        run(["docker", "rm", "-f", container], check=False, timeout=60)


def wait_postgres(container):
    for _ in range(120):
        result = run(["docker", "exec", container, "pg_isready", "-U", "postgres"], check=False, timeout=10)
        if result.returncode == 0:
            return
        time.sleep(0.5)
    fail("PostgreSQL container did not become ready")


def ensure_database_image():
    result = run(["docker", "image", "inspect", POSTGRES_IMAGE], check=False, timeout=30)
    if result.returncode == 0:
        return
    if POSTGRES_IMAGE != DEFAULT_DATABASE_IMAGE:
        fail(f"database image is missing: {POSTGRES_IMAGE}")
    if not COHAB_DOCKERFILE.exists():
        fail(f"missing Citus cohabitation Dockerfile: {COHAB_DOCKERFILE}")
    run([
        "docker", "build",
        "--file", str(COHAB_DOCKERFILE),
        "--build-arg", os.environ.get("TIMESCALE_COHABITATION_BASE_ARG", "BASE_IMAGE=timescale/timescaledb:latest-pg17"),
        "--build-arg", os.environ.get("TIMESCALE_COHABITATION_MAKE_ARG", "MAKE_JOBS=4"),
        "--tag", POSTGRES_IMAGE,
        os.getcwd(),
    ], timeout=1800)


def psql(container, sql):
    run(
        ["docker", "exec", "-i", container, "psql", "-v", "ON_ERROR_STOP=1", "-U", "postgres", "-d", "postgres"],
        input_text=sql,
        timeout=180,
    )


def copy_postgrest_binary(tmpdir):
    run(["docker", "pull", POSTGREST_IMAGE], timeout=300)
    cid = run(["docker", "create", POSTGREST_IMAGE], timeout=60).stdout.strip()
    try:
        for candidate in ("/bin/postgrest", "/usr/local/bin/postgrest", "/postgrest"):
            dest = tmpdir / "postgrest"
            result = run(["docker", "cp", f"{cid}:{candidate}", str(dest)], check=False, timeout=60)
            if result.returncode == 0:
                dest.chmod(0o755)
                version = run([str(dest), "--version"], timeout=30).stdout.strip()
                if "PostgREST" not in version:
                    fail(f"unexpected PostgREST version output: {version}")
                return str(dest), version
    finally:
        run(["docker", "rm", "-f", cid], check=False, timeout=60)
    fail(f"could not copy postgrest binary from {POSTGREST_IMAGE}")


def b64url(raw):
    return base64.urlsafe_b64encode(raw).rstrip(b"=").decode("ascii")


def jwt_for(tenant):
    header = {"alg": "HS256", "typ": "JWT"}
    now = int(time.time())
    # PostgREST must switch to role=web_user while RLS policies consume tenant_id.
    payload = {
        "aud": JWT_AUD,
        "exp": now + 3600,
        "iat": now,
        "role": "web_user",
        "tenant_id": tenant,
    }
    signing_input = ".".join(
        b64url(json.dumps(part, separators=(",", ":")).encode("utf-8"))
        for part in (header, payload)
    )
    signature = hmac.new(JWT_SECRET.encode("utf-8"), signing_input.encode("ascii"), hashlib.sha256).digest()
    return signing_input + "." + b64url(signature)


def auth_headers(tenant, extra=None):
    headers = {"authorization": f"Bearer {jwt_for(tenant)}"}
    if extra:
        headers.update(extra)
    return headers


def parse_json(body):
    try:
        return json.loads(body)
    except json.JSONDecodeError as exc:
        fail(f"response was not JSON: {body!r}: {exc}")


def assert_rows_only(rows, tenant):
    if not rows:
        fail(f"expected at least one row for {tenant}")
    tenants = {row["tenant_id"] for row in rows}
    if tenants != {tenant}:
        fail(f"tenant isolation failed, saw tenants {tenants}: {rows}")


def main():
    require_tool("cargo")
    require_tool("docker")

    run(["cargo", "build", "-q", "-p", POSTGREST_PKG], timeout=300)
    sidecar_binary = os.path.join(TARGET_DIR, "debug", POSTGREST_PKG)
    if not os.path.isfile(sidecar_binary) or not os.access(sidecar_binary, os.X_OK):
        fail(f"missing sidecar binary: {sidecar_binary}")

    pg_port = free_port()
    postgrest_port = free_port()
    sidecar_port = free_port()
    pg_name = f"ai-blaise-postgrest-pg-{os.getpid()}-{int(time.time())}"

    ensure_database_image()
    docker_run = [
        "docker", "run", "-d", "--name", pg_name,
        "-e", f"POSTGRES_PASSWORD={PG_PASSWORD}",
        "-p", f"127.0.0.1:{pg_port}:5432",
        POSTGRES_IMAGE,
    ]
    if EXPECT_CITUS:
        docker_run.extend([
            "postgres",
            "-c", "shared_preload_libraries=timescaledb,citus",
            "-c", "citus.cohabit_extensions=timescaledb",
        ])
    run(docker_run, timeout=300)
    containers.append(pg_name)
    wait_postgres(pg_name)

    citus_setup = """
CREATE EXTENSION IF NOT EXISTS citus;
""" if EXPECT_CITUS else ""
    citus_distribution = """
SELECT create_distributed_table('public.orders', 'tenant_id');
DO $$
BEGIN
  IF NOT EXISTS (
    SELECT 1 FROM pg_dist_partition WHERE logicalrelid = 'public.orders'::regclass
  ) THEN
    RAISE EXCEPTION 'orders table was not registered in pg_dist_partition';
  END IF;
END $$;
""" if EXPECT_CITUS else ""

    psql(pg_name, f"""
{citus_setup}
DO $$
BEGIN
  IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'web_anon') THEN
    CREATE ROLE web_anon NOLOGIN;
  END IF;
  IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'web_user') THEN
    CREATE ROLE web_user NOLOGIN;
  END IF;
  IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'postgrest_authenticator') THEN
    CREATE ROLE postgrest_authenticator LOGIN PASSWORD '{AUTHENTICATOR_PASSWORD}';
  END IF;
END $$;
ALTER ROLE postgrest_authenticator WITH LOGIN PASSWORD '{AUTHENTICATOR_PASSWORD}';
GRANT web_anon TO postgrest_authenticator;
GRANT web_user TO postgrest_authenticator;
CREATE TABLE public.orders (
  id bigserial,
  tenant_id text NOT NULL,
  item text NOT NULL,
  total numeric NOT NULL CHECK (total >= 0),
  PRIMARY KEY (tenant_id, id)
);
{citus_distribution}
ALTER TABLE public.orders ENABLE ROW LEVEL SECURITY;
CREATE POLICY orders_tenant_select ON public.orders
  FOR SELECT TO web_user
  USING (tenant_id = (current_setting('request.jwt.claims', true)::json ->> 'tenant_id'));
CREATE POLICY orders_tenant_insert ON public.orders
  FOR INSERT TO web_user
  WITH CHECK (tenant_id = (current_setting('request.jwt.claims', true)::json ->> 'tenant_id'));
INSERT INTO public.orders (tenant_id, item, total) VALUES
  ('tenant-a', 'alpha', 10),
  ('tenant-b', 'beta', 20);
CREATE SCHEMA api;
CREATE VIEW api.orders WITH (security_invoker=true) AS
  SELECT id, tenant_id, item, total FROM public.orders;
GRANT USAGE ON SCHEMA public, api TO web_anon, web_user;
GRANT SELECT, INSERT ON public.orders TO web_user;
GRANT SELECT, INSERT ON api.orders TO web_user;
GRANT USAGE, SELECT ON SEQUENCE public.orders_id_seq TO web_user;
""")

    with tempfile.TemporaryDirectory(prefix="ai-blaise-postgrest-live-") as tmp:
        tmpdir = Path(tmp)
        postgrest_binary, postgrest_version = copy_postgrest_binary(tmpdir)
        config_path = str(tmpdir / "postgrest.conf")
        db_uri = f"postgresql://postgrest_authenticator:{AUTHENTICATOR_PASSWORD}@127.0.0.1:{pg_port}/postgres"

        dependency_env = os.environ.copy()
        dependency_env.update({
            "POSTGREST_DB_URI": db_uri,
            "POSTGREST_JWT_SECRET": JWT_SECRET,
            "AI_BLAISE_POSTGREST_BINARY": postgrest_binary,
            "AI_BLAISE_POSTGREST_PORT": str(postgrest_port),
        })
        dependency_report = run([sidecar_binary, "check-runtime-dependencies"], env=dependency_env, timeout=60).stdout
        if db_uri in dependency_report or JWT_SECRET in dependency_report:
            fail("dependency report leaked database URI or JWT secret")
        if "public,api" not in dependency_report:
            fail(f"dependency report lost schema list: {dependency_report}")

        supervisor_env = dependency_env.copy()
        supervisor_env.update({
            "AI_BLAISE_POSTGREST_CONFIG_PATH": config_path,
            "AI_BLAISE_POSTGREST_EXIT_ON_STDIN_EOF": "1",
        })
        supervisor = subprocess.Popen(
            [sidecar_binary, "run-live-postgrest"],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            env=supervisor_env,
        )
        processes.append((supervisor, "postgrest supervisor"))
        wait_http(postgrest_port, "/", "PostgREST upstream")

        conf = Path(config_path).read_text()
        if db_uri in conf or JWT_SECRET in conf:
            fail("generated postgrest.conf leaked database URI or JWT secret")
        for required in (
            'db-uri = "env:POSTGREST_DB_URI"',
            'jwt-secret = "env:POSTGREST_JWT_SECRET"',
            'jwt-role-claim-key = ".role"',
            'db-schemas = "public,api"',
        ):
            if required not in conf:
                fail(f"generated postgrest.conf missing {required!r}: {conf}")

        sidecar_env = os.environ.copy()
        sidecar_env.update({
            "AI_BLAISE_LISTEN_ADDR": f"127.0.0.1:{sidecar_port}",
            "AI_BLAISE_POSTGREST_UPSTREAM": f"127.0.0.1:{postgrest_port}",
        })
        sidecar = subprocess.Popen(
            [sidecar_binary, "serve"],
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            env=sidecar_env,
        )
        processes.append((sidecar, "postgrest sidecar proxy"))
        wait_sidecar_ready(sidecar, sidecar_port)

        status, body = request(sidecar_port, "GET", "/postgrest.conf")
        assert status == 200, body
        if db_uri in body or JWT_SECRET in body:
            fail("sidecar /postgrest.conf leaked database URI or JWT secret")

        status, body = request(sidecar_port, "GET", "/api/orders?select=id,tenant_id,item,total&order=id.asc")
        if status not in (401, 403):
            fail(f"unauthenticated API request should fail closed, got {status}: {body}")
        if "tenant-a" in body or "tenant-b" in body:
            fail(f"unauthenticated response leaked tenant rows: {body}")

        status, body = request(
            sidecar_port,
            "GET",
            "/api/orders?select=id,tenant_id,item,total&order=id.asc",
            headers=auth_headers("tenant-a"),
        )
        assert status == 200, body
        tenant_a_rows = parse_json(body)
        assert_rows_only(tenant_a_rows, "tenant-a")

        status, body = request(
            sidecar_port,
            "GET",
            "/api/orders?select=id,tenant_id,item,total&order=id.asc",
            headers=auth_headers("tenant-b"),
        )
        assert status == 200, body
        tenant_b_rows = parse_json(body)
        assert_rows_only(tenant_b_rows, "tenant-b")

        insert_body = json.dumps({"tenant_id": "tenant-a", "item": "sidecar-insert", "total": 42})
        status, body = request(
            sidecar_port,
            "POST",
            "/api/public/orders?select=id,tenant_id,item,total",
            body=insert_body,
            headers=auth_headers("tenant-a", {"prefer": "return=representation"}),
        )
        if status not in (200, 201):
            fail(f"tenant insert failed with {status}: {body}")
        inserted = parse_json(body)
        assert_rows_only(inserted, "tenant-a")
        if inserted[0]["item"] != "sidecar-insert":
            fail(f"insert returned wrong row: {inserted}")

        # The cross-tenant INSERT must be rejected by PostgreSQL RLS, not by a mock.
        malicious_body = json.dumps({"tenant_id": "tenant-b", "item": "cross-tenant", "total": 99})
        status, body = request(
            sidecar_port,
            "POST",
            "/api/public/orders?select=id,tenant_id,item,total",
            body=malicious_body,
            headers=auth_headers("tenant-a", {"prefer": "return=representation"}),
        )
        if status < 400:
            fail(f"cross-tenant insert unexpectedly succeeded: {status}: {body}")

        status, body = request(
            sidecar_port,
            "GET",
            "/api/public/orders?select=id,tenant_id,item,total&item=eq.cross-tenant",
            headers=auth_headers("tenant-b"),
        )
        assert status == 200, body
        if parse_json(body):
            fail(f"cross-tenant row was inserted despite RLS failure: {body}")

        status, body = request(sidecar_port, "PUT", "/api/orders", headers=auth_headers("tenant-a"))
        assert status == 405, body
        assert "method not allowed" in body, body

        ARTIFACT.parent.mkdir(parents=True, exist_ok=True)
        ARTIFACT.write_text(
            "feature\tassertion\tstatus\tdetail\n"
            f"API1\tsupervised_postgrest_child\tpassed\t{postgrest_version}\n"
            "API1\tsidecar_proxy_table_backed_get_post\tpassed\t/api/public/orders served by upstream PostgREST\n"
            f"API2\tcitus_distributed_table\tpassed\tEXPECT_CITUS={EXPECT_CITUS} image={POSTGRES_IMAGE} pg_dist_partition asserted\n"
            "API2\tdistributed_view_profile\tpassed\t/api/orders routed with Accept-Profile api to api.orders\n"
            "API5\tjwt_rls_tenant_isolation\tpassed\ttenant JWT role web_user plus tenant_id claim enforced SELECT and INSERT policies\n"
            "API5\tsecret_non_disclosure\tpassed\tdependency report and postgrest.conf retained env refs without URI/JWT leakage\n"
        )

    print("ai_blaise_citus live PostgREST data-plane smoke passed")


try:
    main()
finally:
    cleanup()
PY_SMOKE
