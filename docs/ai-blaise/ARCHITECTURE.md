# Architecture

The fork combines upstream Citus with ai-blaise overlay components.

## Component Map

| Path | Role |
| --- | --- |
| `patches/` | Minimal quilt series against upstream Citus or PostgreSQL |
| `companion/` | Rust `pgrx` extension for Citus-adjacent SQL surfaces |
| `sidecar/` | Crash-isolated Rust daemons for analytical, CDC, auth, storage, and coordination work |
| `pool/` | Shard-aware pgcat fork |
| `operator/` | Rust `kube-rs` CRDs and reconcilers |
| `e2e/` | Executable critical-path acceptance contracts |
| `deploy/k8s/` | ai-blaise Helm chart and CRDs |
| `tools/` | CLI, TUI, LSP, admin UI, schema designer, watch, and MCP |

## Critical Path

The first end-to-end acceptance path is:

1. Apply the TS6 cohabitation patch series:
   `patches/0001-allow-trusted-hook-coextensions.patch` and
   `patches/0002-preserve-trusted-hook-chain-state.patch`.
2. Build `companion/citus_timescale`.
3. Reconcile the `Hypertable` CRD in `operator/`.
4. Drive `e2e/` Timescale-on-Citus acceptance.
5. Prove a Timescale hypertable partition under a Citus distributed parent works
   in an end-to-end cluster test.

The companion crate starts with pure Rust planning and validation types, then
exposes them through two database surfaces: a `pg18`-gated pgrx module and an
`ai_blaise_citus` SQL fallback extension packaged in the operand image. The
Hypertable operator reconcile path now wraps those companion plans in a guarded
apply plan: create the extension, validate `companion_feature_status()`, assert
Timescale/Citus cohabitation, then run the ordered TS1/TS2/TS3/TS4/TS5 SQL.
The SQL fallback is smoke-tested with Docker in CI so
`CREATE EXTENSION ai_blaise_citus` is verified before the compiled pgrx library
becomes the only runtime surface.
