# Architecture

The fork combines upstream Citus with ai-blaise overlay components.

## Component Map

| Path | Role |
| --- | --- |
| `patches/` | Minimal quilt series against upstream Citus or PostgreSQL |
| `companion/` | Rust `pgrx` extension for Citus-adjacent SQL surfaces |
| `sidecar/` | Crash-isolated Rust daemons for analytical, CDC, auth, storage, and coordination work |
| `pool/` | Shard-aware pgcat fork |
| `operator/` | Rust CRD contract models, apply-plan builders, and probe runtime |
| `e2e/` | Executable critical-path acceptance contracts |
| `deploy/k8s/` | ai-blaise Helm chart and CRDs |
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
installable bridge-state SQL. The broader TS1/TS2/TS3/TS4/TS5/TS12 distributed
feature entries remain alpha until multi-worker fanout, rebalance, and
operator reconciliation are implemented and measured end to end.

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
