# Agentmemory VM Recovery

This note records the recovery path for the project agentmemory service used by
the ai-blaise/Citus production-readiness work on the `experiment-playground` VM.

## Service Contract

- Host: `127.0.0.1`
- Port: `3911`
- Mode: `scaleable-database-infra`
- Service: `agentmemory-scaleable-database-infra.service`
- State directory: `/home/spencer/.agentmemory-scaleable-database-infra`
- Memory file: `/home/spencer/.agentmemory-scaleable-database-infra/standalone.json`
- Server file: `/home/spencer/.agentmemory-scaleable-database-infra/server.py`
- Backup root: `/home/spencer/agentmemory-backups`

The service is a local-only isolated JSON REST server. It exposes health,
search, save, export, audit, and MCP-compatible tool endpoints under
`/agentmemory/*`.

## Recovery Steps

Before changing anything, preserve the current VM state if it exists:

```bash
stamp=$(date -u +%Y%m%dT%H%M%SZ)
base="$HOME/.agentmemory-scaleable-database-infra"
backup_root="$HOME/agentmemory-backups"
backup="$backup_root/scaleable-database-infra-$stamp"
mkdir -p "$backup_root"

if [ -e "$base" ]; then
  cp -a "$base" "$backup"
else
  mkdir -p "$backup"
  printf 'No existing VM agentmemory path at %s before recovery.\n' "$base" \
    > "$backup/NO_EXISTING_VM_STATE.txt"
fi
```

Restore `server.py` and `standalone.json` into the state directory from the
preserved `scaleable-database-infra` source of truth, then validate the JSON and
record checksums:

```bash
mkdir -p "$HOME/.agentmemory-scaleable-database-infra"
chmod 700 "$HOME/.agentmemory-scaleable-database-infra"
chmod 755 "$HOME/.agentmemory-scaleable-database-infra/server.py"
chmod 600 "$HOME/.agentmemory-scaleable-database-infra/standalone.json"
python3 -m json.tool \
  "$HOME/.agentmemory-scaleable-database-infra/standalone.json" >/dev/null
sha256sum \
  "$HOME/.agentmemory-scaleable-database-infra/server.py" \
  "$HOME/.agentmemory-scaleable-database-infra/standalone.json"
```

Install or refresh the user service at
`/home/spencer/.config/systemd/user/agentmemory-scaleable-database-infra.service`:

```ini
[Unit]
Description=Agentmemory isolated JSON store for scaleable-database-infra
After=network.target

[Service]
Type=simple
Environment=PYTHONUNBUFFERED=1
WorkingDirectory=%h/.agentmemory-scaleable-database-infra
ExecStart=/usr/bin/python3 %h/.agentmemory-scaleable-database-infra/server.py --host 127.0.0.1 --port 3911 --memory %h/.agentmemory-scaleable-database-infra/standalone.json
Restart=on-failure
RestartSec=2
UMask=0077
NoNewPrivileges=true

[Install]
WantedBy=default.target
```

Enable persistence and start the service. Keep `UMask=0077` in the user
unit because `server.py` atomically rewrites `standalone.json` on memory writes;
without the unit umask, future writes can recreate the JSON file with broader
default permissions.

```bash
sudo loginctl enable-linger spencer
systemctl --user daemon-reload
systemctl --user enable --now agentmemory-scaleable-database-infra.service
chmod 600 "$HOME/.agentmemory-scaleable-database-infra/standalone.json"
```

## Verification

Run these checks after any recovery:

```bash
systemctl --user is-active agentmemory-scaleable-database-infra.service
systemctl --user is-enabled agentmemory-scaleable-database-infra.service
loginctl show-user spencer -p Linger
curl -fsS http://127.0.0.1:3911/agentmemory/health
curl -fsS http://127.0.0.1:3911/agentmemory/mcp/tools
stat -c "%a %U %G %n" \
  "$HOME/.agentmemory-scaleable-database-infra/standalone.json"
```

For preservation, snapshot all pre-existing `mem:memories` IDs and content
hashes before the write test, save a harmless recovery checkpoint through
`POST /agentmemory/remember`, search for the returned ID, and then compare the
post-write store. The existing memory set must have zero missing IDs and zero
changed content hashes.

The recovery on 2026-05-24 used backup marker
`/home/spencer/agentmemory-backups/scaleable-database-infra-20260524T023944Z/NO_EXISTING_VM_STATE.txt`,
preserved all 760 source memories, added `mem_20260524T024048Z_nxrcty`, and left
the service healthy after restart with 761 memories and 914 audit records.
