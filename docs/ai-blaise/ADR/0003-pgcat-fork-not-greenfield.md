# ADR 0003: Fork pgcat for the Pool, Do Not Write a Greenfield Pooler

## Status

Accepted (2026-05-21)

## Context

The overlay needs a Postgres pooler that is shard-aware, plan-cache
coherent across placement generations, settings-bucket aware (so
`citus.*` GUCs do not silently mix between borrowed sessions), and able
to multiplex backend PIDs so `pg_cancel_backend()` round-trips
correctly. None of pgbouncer, Supavisor, or PgDog ship the full set;
the choice is between forking an existing Rust pooler or starting
from scratch in `pool/`.

## Decision

Fork `perplexityai/pgcat` (MIT) into `pool/`. The upstream is
Perplexity-maintained, runs at production scale, and already implements
the load-bearing primitives we need to keep: connection pooling on
`tokio` + `bb8`, mirroring, prepared-statement support, `rustls` TLS,
the `pgcat`/`pgbouncer` virtual admin DB, and the SIGHUP hot-reload
path. The diff we carry is listed in plan §6.4 — parser swap to
`pg_query`, Citus-aware routing via `pg_dist_shard` and
`pg_dist_placement`, shared generation-counter plan cache, settings
buckets, virtual-PID multiplexing, token-bucket admission, HTAP
classifier routing to `sidecar/analytical`, JWT verification through
`sidecar/auth`, GeoIP routing via `maxminddb`, and TLS session-ticket
reuse.

## Alternatives considered

- Greenfield Rust pooler. Rejected — protocol coverage (extended query,
  COPY, replication, prepared statements, SCRAM, GSSAPI) is years of
  work, and every gap is a production incident. pgcat already has it.
- Fork PgDog (AGPL). Rejected — relicensing to AGPL is fine for us
  (the overlay is AGPL-3.0), but pgcat's architecture (especially the
  admin DB and configuration model) is closer to our target and easier
  to keep in sync with upstream MIT changes.
- Use pgbouncer with extensions. Rejected — C, single-threaded, no
  prepared-statement passthrough for transaction pooling, no Rust
  ecosystem alignment with the rest of the overlay.
- Use Supavisor (Elixir + Rust via Rustler). Rejected — Erlang VM
  adds a non-Rust dependency, and the Rust portion does not own
  routing.

## Consequences

- Positive: we inherit a tested protocol implementation and connection
  manager. The feature gap closes quickly because the substrate is
  sound.
- Positive: Cargo workspace shares crates with `companion/`,
  `sidecar/`, and `operator/` (`pg_query`, `rustls`, `tokio-postgres`,
  `tracing`, `prometheus`), keeping dependency versions coherent.
- Negative: upstream pgcat does not know about Citus, so the routing
  and plan-cache diffs are non-trivial and must be carried as a fork
  delta rather than upstreamed.
- Risks: pgcat upstream may diverge on internals we depend on (the
  session router, the admin DB schema). Mitigation: track upstream
  weekly via a label on PRs that touch core; isolate Citus-specific
  logic behind a `routing` module so rebasing is mechanical.

## References

- Plan §6.4 (`pool/` — Rust pgcat fork)
- Plan §4.5 (connection amplification)
- `perplexityai/pgcat` — upstream MIT
- `pgcat`'s settings-bucket pool design (upstream issue tracker)
