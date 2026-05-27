# Architecture

The fork combines upstream Citus with ai-blaise overlay components.

## Component Map

| Path | Role |
| --- | --- |
| `patches/` | Minimal quilt series against upstream Citus or PostgreSQL |
| `companion/` | Rust `pgrx` extension for Citus-adjacent SQL surfaces |
| `sidecar/` | Crash-isolated Rust daemons for analytical, CDC, auth, storage, and coordination work |
| `pool/` | Shard-aware pgcat fork |
| `pool/wire/` | PostgreSQL v3 wire-protocol codec (`ai_blaise_citus_pool_wire`), Rust port of jackc/pgx `pgproto3` (MIT); drives extended-query frame parsing on the live `serve` data plane |
| `operator/` | Rust CRD contract models, apply-plan builders, and probe runtime |
| `e2e/` | Executable critical-path acceptance contracts |
| `ai-blaise/command-center (helm/charts/citus-cluster + deploy/citus-cluster)` | ai-blaise Helm chart and CRDs |
| `tools/` | CLI, TUI, LSP, admin UI, schema designer, watch, and MCP |

## Critical Path

The first end-to-end acceptance path is:

1. Use the integrated TS6 cohabitation source changes; the historical patch
   files in `patches/` remain as rebase/reference artifacts.
2. Build `companion/citus_timescale`.
3. Convert the `Hypertable` CRD model in `operator/` into a guarded apply plan.
4. Drive `e2e/` Timescale-on-Citus acceptance.
5. Prove a Timescale hypertable partition under a Citus distributed parent works
   in an end-to-end cluster test.

The companion crate starts with pure Rust planning and validation types, then
exposes them through two database surfaces: a `pg18`-gated pgrx module and an
`ai_blaise_citus` SQL fallback extension. The SQL fallback files are copied by
the `FEATURE: Bundle1` alpha operand-image contract and mounted directly by the
SQL runtime smokes. That proves `CREATE EXTENSION ai_blaise_citus`, but it is
not production evidence for the full operand image because the required binary
extension bundle is not yet proven by a real operand image build/initdb smoke.
The Hypertable operator apply-plan path now wraps those companion plans in a
guarded apply plan: create the extension, validate
`companion_feature_status()`, validate the configured Timescale/Citus
cohabitation precondition, then run the ordered TS1/TS2/TS3/TS4/TS5 SQL.
`ci/ai-blaise/timescale-cohabitation-smoke.sh` provides the current live
cohabitation proof for the integrated TS6 hook-chain source and the TS18
installable bridge-state SQL. TS1/TS2/TS3/TS4/TS5/TS12 are production-ready for
the bounded live SQL apply/catalog-state surface under the pinned cohabitation
image, and `ci/ai-blaise/operator-hypertable-live-smoke.sh` proves TS7
Kubernetes controller/status reconciliation for that same bounded bridge
surface. Multi-worker fanout, rebalance, background policy completion,
continuous aggregate refresh correctness, and planner pushdown remain outside
that claim.

## Pool data plane (`FEATURE: T7`)

The pool listens on the configured `serve` port and proxies traffic to a
PostgreSQL upstream. After the StartupMessage envelope is admitted (CIDR
allowlist, auth-sidecar introspection, tenant quota, settings-bucket
fingerprint), the connection bridges client <-> upstream in two directions:

- **Client -> upstream**: `pool/src/proxy.rs::forward_client_to_upstream`
  decodes each PostgreSQL v3 wire frame via the `ai_blaise_citus_pool_wire`
  crate, increments the matching atomic counter on `PoolProxyState`, and
  forwards the bytes through verbatim. Byte-transparency is preserved on
  every frame; the codec is observation-only on the hot path. Frame counts
  are exposed at `/metrics` as
  `ai_blaise_citus_pool_ext_query_frames_total{frame="Parse|Bind|Describe|Execute|Sync|Flush|Close|Query|CopyData|Terminate|Other"}`.
- **Upstream -> client**: `pool/src/proxy.rs::copy_and_shutdown` is a plain
  `io::copy`; the pool does not decode backend frames.

The same codec covers cancel-request and FATAL `ErrorResponse` envelopes
emitted by the pool itself, and the startup-tap parse in
`pool/src/trace_tap.rs`. The crate ships every PostgreSQL v3 message type
the pool may need to inspect or rewrite: 12 frontend frames, 20 backend
frames, the 4 startup envelopes (Startup/Cancel/SSL/GSSENC), and the 11
Authentication sub-codes plus the four `p`-tag frontend responses
(Password / SASL initial / SASL response / GSS). 46 round-trip unit tests
keep encode/decode parity, and two live smokes drive the end-to-end path:
`ci/ai-blaise/pool-extended-query-pipeline-live-smoke.sh` (codec direct to
postgres) and `ci/ai-blaise/pool-extended-query-through-pool-live-smoke.sh`
(codec through the pool, with `/metrics` scrape verification).

## Architecture Decision Records

Cross-cutting decisions are recorded under `docs/ai-blaise/ADR/`:

- [`0001-fork-not-rewrite.md`](ADR/0001-fork-not-rewrite.md) — fork
  `citusdata/citus` rather than rewriting.
- [`0002-overlay-not-patch.md`](ADR/0002-overlay-not-patch.md) — new
  code lives in non-overlapping overlay directories; only `patches/`
  touches upstream files.
- [`0003-pgcat-fork-not-greenfield.md`](ADR/0003-pgcat-fork-not-greenfield.md)
  — `pool/` forks `perplexityai/pgcat` (MIT).
- [`0004-pg_lake-fork-for-analytical.md`](ADR/0004-pg_lake-fork-for-analytical.md)
  — `sidecar/analytical` forks `Snowflake-Labs/pg_lake` (Apache 2.0)
  with a Rust DataFusion + DuckDB engine.
- [`0005-rust-kube-rs-not-go.md`](ADR/0005-rust-kube-rs-not-go.md) —
  the operator is Rust + `kube-rs`, joining the same Cargo workspace
  as companion / sidecars / pool / tools.
- [`0006-cnpg-substrate-not-bypass.md`](ADR/0006-cnpg-substrate-not-bypass.md)
  — CloudNativePG manages Postgres lifecycle; our operator layers
  Citus-specific topology on top.
- [`0007-raft-per-shardgroup.md`](ADR/0007-raft-per-shardgroup.md) —
  one `raft-rs` group per shard group for placement, lease, and
  split/merge decisions.
