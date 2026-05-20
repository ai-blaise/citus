# Bootstrap V2 Fork

`ai-blaise/citus` branch `bootstrap-v2` is the source of truth for Citus command-center database changes.

- Rebase cadence: quarterly from `citusdata/citus` `main`.
- Platform boundary: consumed by `ai-blaise/platform` through wire PostgreSQL, Helm, and runtime validation only.
- Citus fork changes stay in `ai-blaise/citus`; there is no alternate provider fallback.
