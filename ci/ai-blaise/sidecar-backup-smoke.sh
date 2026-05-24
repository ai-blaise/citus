#!/usr/bin/env bash
set -euo pipefail

# FEATURE: B1 B3 B4 B6

# End-to-end smoke for the backup sidecar HTTP runtime. The smoke replaces the
# `wal-g`, `pg_ctl`, and `psql` binaries with deterministic local fakes so the
# harness exercises the real WalgRunner, PITR, queryable-branch, and HTTP code
# paths without provisioning cloud object storage.

repo_root="$(git rev-parse --show-toplevel)"
cd "${repo_root}"

work_dir="$(mktemp -d -t ai-blaise-backup-smoke.XXXXXX)"
trap true EXIT

walg_stub="${work_dir}/wal-g"
pg_ctl_stub="${work_dir}/pg_ctl"
psql_stub="${work_dir}/psql"
pgdata="${work_dir}/pgdata"
restore_root="${work_dir}/restores"
branch_root="${work_dir}/branches"

mkdir -p "${pgdata}" "${restore_root}" "${branch_root}"

cat <<'STUB' > "${walg_stub}"
#!/usr/bin/env bash
set -euo pipefail

cmd="${1:-}"

case "${cmd}" in
  backup-push)
    echo "{\"BackupName\":\"base_000000010000000000000001\",\"Pgdata\":\"${2:-}\"}"
    ;;
  wal-show)
    cat <<'JSON'
{"timeline":1,"start_segment":"000000010000000000000001","end_segment":"000000010000000000000010","segment_range_count":16}
JSON
    ;;
  backup-fetch)
    target="${2:-}"
    mkdir -p "${target}"
    printf "fetched backup at %s to %s\n" "${4:-unknown-time}" "${target}"
    ;;
  delete)
    echo "deleted backups older than ${4:-0} retention"
    ;;
  backup-list)
    cat <<'JSON'
[{"backup_name":"base_000000010000000000000001","time":"2026-05-19T12:00:00Z","wal_file_name":"000000010000000000000001","start_lsn":"0/1000000"}]
JSON
    ;;
  *)
    echo "wal-g stub: unknown command ${cmd}" >&2
    exit 64
    ;;
esac
STUB
chmod +x "${walg_stub}"

# Lightweight pg_ctl + psql stubs so the queryable-branch tests do not require
# the system PostgreSQL service. The smoke verifies the engine state machine
# and HTTP wiring; live PG verification happens in the kind smoke.
cat <<'STUB' > "${pg_ctl_stub}"
#!/usr/bin/env bash
set -euo pipefail
echo "pg_ctl stub invoked: $*"
STUB
chmod +x "${pg_ctl_stub}"

cat <<'STUB' > "${psql_stub}"
#!/usr/bin/env bash
set -euo pipefail
echo "1|on"
STUB
chmod +x "${psql_stub}"

cargo build -q -p ai_blaise_citus_sidecar_backup

backup_binary="$(cargo metadata --no-deps --format-version 1 2>/dev/null \
  | python3 -c 'import json,sys;data=json.load(sys.stdin);print(data["target_directory"])')/debug/ai_blaise_citus_sidecar_backup"

if [[ ! -x "${backup_binary}" ]]; then
  echo "missing backup binary at ${backup_binary}" >&2
  exit 1
fi

python3 - "$walg_stub" "$pg_ctl_stub" "$psql_stub" "$pgdata" "$restore_root" "$branch_root" "$backup_binary" <<'PY'
import http.client
import json
import os
import socket
import subprocess
import sys
import time
import urllib.parse


walg_stub, pg_ctl_stub, psql_stub, pgdata, restore_root, branch_root, backup_binary = sys.argv[1:8]


def free_port():
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        sock.bind(("127.0.0.1", 0))
        return sock.getsockname()[1]


port = free_port()
env = os.environ.copy()
env["AI_BLAISE_LISTEN_ADDR"] = f"127.0.0.1:{port}"
env["AI_BLAISE_BACKUP_WALG_BINARY"] = walg_stub
env["AI_BLAISE_BACKUP_PG_CTL_BINARY"] = pg_ctl_stub
env["AI_BLAISE_BACKUP_PSQL_BINARY"] = psql_stub
env["AI_BLAISE_BACKUP_PRIMARY_PGDATA"] = pgdata
env["AI_BLAISE_BACKUP_RESTORE_ROOT"] = restore_root
env["AI_BLAISE_BACKUP_BRANCH_ROOT"] = branch_root
env["AI_BLAISE_BACKUP_OLDEST_WAL_TIME"] = "2026-05-19T00:00:00Z"
env["AI_BLAISE_BACKUP_LATEST_WAL_TIME"] = "2026-05-19T23:59:59Z"
env["AI_BLAISE_BACKUP_DISABLE_SCHEDULER"] = "1"

stderr_log = open(os.path.join(os.path.dirname(walg_stub), "backup-server.stderr.log"), "w")
stdout_log = open(os.path.join(os.path.dirname(walg_stub), "backup-server.stdout.log"), "w")
proc = subprocess.Popen(
    [backup_binary, "serve"],
    stdout=stdout_log,
    stderr=stderr_log,
    text=True,
    env=env,
)


def request(method, path, body=None):
    conn = http.client.HTTPConnection("127.0.0.1", port, timeout=15)
    headers = {"connection": "close"}
    if body is not None:
        headers["content-type"] = "application/json"
        headers["content-length"] = str(len(body))
    conn.request(method, path, body=body, headers=headers)
    response = conn.getresponse()
    data = response.read().decode("utf-8")
    conn.close()
    return response.status, data


def server_logs():
    stderr_log.flush()
    stdout_log.flush()
    stderr_path = stderr_log.name
    stdout_path = stdout_log.name
    try:
        stderr_data = open(stderr_path).read()
    except OSError:
        stderr_data = "<unavailable>"
    try:
        stdout_data = open(stdout_path).read()
    except OSError:
        stdout_data = "<unavailable>"
    return f"\n--- backup server stderr ---\n{stderr_data}\n--- backup server stdout ---\n{stdout_data}\n"


def fail(reason, status=None, data=None):
    logs = server_logs()
    if proc.poll() is None:
        try:
            proc.terminate()
            proc.wait(timeout=20)
        except subprocess.TimeoutExpired:
            proc.kill()
            proc.wait(timeout=20)
    raise AssertionError(
        f"backup smoke failed: {reason} (status={status}, data={data}){logs}"
    )


try:
    for _ in range(60):
        try:
            status, data = request("GET", "/readyz")
            if status == 200 and '"component":"backup"' in data:
                break
        except OSError:
            pass
        if proc.poll() is not None:
            fail(f"backup sidecar exited early with code {proc.returncode}")
        time.sleep(0.5)
    else:
        fail("backup sidecar HTTP server did not become ready")

    status, data = request("GET", "/metrics")
    if status != 200 or "ai_blaise_backup_completed_base_backups 0" not in data:
        fail("metrics endpoint missing backup counters", status, data)

    status, data = request("POST", "/backups/run")
    if status != 202:
        fail("base backup did not return 202", status, data)
    artifact = json.loads(data)
    if artifact.get("cluster") != "prod" or artifact.get("encrypted") is not True:
        fail("base backup artifact incorrect", status, data)

    status, data = request("GET", "/backups/status")
    if status != 200 or '"completed_base_backups":1' not in data or '"operation":"base_backup"' not in data:
        fail("/backups/status missing base backup state", status, data)

    status, data = request("POST", "/backups/delete-old", '{"retention_full":7}')
    if status != 202 or '"operation":"delete_old"' not in data:
        fail("/backups/delete-old did not prune retention", status, data)

    status, data = request("GET", "/backups/status")
    if status != 200 or '"retention_deletions":1' not in data:
        fail("/backups/status missing retention deletion accounting", status, data)

    status, data = request("GET", "/backups")
    if status != 200 or "base_000000010000000000000001" not in data:
        fail("/backups did not list stub output", status, data)

    status, data = request("GET", "/wal/status")
    if status != 200 or "2026-05-19T00:00:00Z" not in data:
        fail("/wal/status missing oldest WAL time", status, data)

    out_of_window = json.dumps(
        {
            "cluster": "prod",
            "source_archive_uri": "s3://backups/prod",
            "target_time": "2024-01-01T00:00:00Z",
            "target_cluster": "restore-out-of-window",
        },
        separators=(",", ":"),
    )
    status, data = request("POST", "/pitr/restore", out_of_window)
    if status != 400 or "outside available WAL window" not in data:
        fail("out-of-window PITR target should fail", status, data)

    in_window = json.dumps(
        {
            "cluster": "prod",
            "source_archive_uri": "s3://backups/prod",
            "target_time": "2026-05-19T12:00:00Z",
            "target_cluster": "restore-prod",
        },
        separators=(",", ":"),
    )
    status, data = request("POST", "/pitr/restore", in_window)
    if status != 202 or '"status":"succeeded"' not in data:
        fail("in-window PITR restore did not record success", status, data)
    job_id = json.loads(data)["job_id"]

    status, data = request("GET", f"/pitr/status/{urllib.parse.quote(job_id)}")
    if status != 200 or '"status":"succeeded"' not in data:
        fail("/pitr/status did not return succeeded job", status, data)

    bad_port = json.dumps(
        {
            "branch_name": "prod-bad-port",
            "source_archive_uri": "s3://backups/prod",
            "target_time": "2026-05-19T12:00:00Z",
            "port": 80,
        },
        separators=(",", ":"),
    )
    status, data = request("POST", "/branches/queryable", bad_port)
    if status != 400 or "port must be between" not in data:
        fail("/branches/queryable should reject privileged port", status, data)

    malformed_port = '{"branch_name":"prod-malformed-port","source_archive_uri":"s3://backups/prod","target_time":"2026-05-19T12:00:00Z","port":6543junk}'
    status, data = request("POST", "/branches/queryable", malformed_port)
    if status != 400 or "malformed backup sidecar HTTP request" not in data:
        fail("/branches/queryable should reject numeric port with junk suffix", status, data)

    queryable = json.dumps(
        {
            "branch_name": "prod-at-noon",
            "source_archive_uri": "s3://backups/prod",
            "target_time": "2026-05-19T12:00:00Z",
            "port": 6543,
        },
        separators=(",", ":"),
    )
    status, data = request("POST", "/branches/queryable", queryable)
    if status != 201 or '"port":6543' not in data or '"read_only":true' not in data:
        fail("/branches/queryable did not create branch", status, data)

    status, data = request("GET", "/branches/queryable")
    if status != 200 or "prod-at-noon" not in data:
        fail("/branches/queryable list missing created branch", status, data)

    status, data = request("POST", "/branches/queryable", queryable)
    if status != 400 or "already exists" not in data:
        fail("duplicate queryable branch should fail", status, data)

finally:
    proc.terminate()
    try:
        proc.wait(timeout=20)
    except subprocess.TimeoutExpired:
        proc.kill()
        proc.wait(timeout=20)

if proc.returncode not in (0, -15):
    stderr = proc.stderr.read() if proc.stderr is not None else ""
    print(stderr, file=sys.stderr)
    raise SystemExit(proc.returncode)

print("ai_blaise_citus_sidecar_backup HTTP smoke passed")
PY
