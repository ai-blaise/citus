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

## Addendum 2026-05-27 — pool/wire pgproto3 Rust port

The upstream pgcat fork covers session-mode pooling, prepared-statement
support, SCRAM, and the simple-query data plane. It does NOT expose a
reusable, bidirectional PostgreSQL v3 codec to other workspace crates and
its protocol parsing is private. To graduate `FEATURE: T7` from "byte-
transparent simple-query only" to "byte-transparent simple-query AND typed
extended-query `Parse`/`Bind`/`Describe`/`Execute`/`Sync`/`Flush` frame
parsing on the live data plane", `pool/wire/` was added as a no-dep Rust
workspace crate ported from the message-shape semantics of jackc/pgx
`pgproto3` (MIT) — the only widely-used codec whose explicit design goal
is bidirectional use by drivers, proxies, servers, and load balancers.

This is a refinement of the original ADR, not a reversal. The pool itself
remains a pgcat fork. `pool/wire/` is invoked by `pool/src/proxy.rs`'s
`forward_client_to_upstream` on the live data path; the pool stays
byte-transparent for every forwarded frame, with the codec observing and
counting frames into atomic counters exposed at `/metrics`. Shard-aware
routing of an extended-query pipeline and multi-`Sync` transaction-aware
batching remain alpha-deferred under the same T7 contract.

See:

- `pool/wire/src/lib.rs` — codec entry point + MIT attribution to jackc/pgx
- `docs/ai-blaise/ARCHITECTURE.md` — pool data plane section
- `ci/ai-blaise/pool-extended-query-pipeline-live-smoke.sh` — codec direct
- `ci/ai-blaise/pool-extended-query-through-pool-live-smoke.sh` — codec through pool

## References

- Plan §6.4 (`pool/` — Rust pgcat fork)
- Plan §4.5 (connection amplification)
- `perplexityai/pgcat` — upstream MIT
- `pgcat`'s settings-bucket pool design (upstream issue tracker)
- `jackc/pgx` `pgproto3` (MIT) — reference for the bidirectional v3 codec
