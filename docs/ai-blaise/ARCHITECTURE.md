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
| `deploy/k8s/` | ai-blaise Helm chart and CRDs |
| `tools/` | CLI, TUI, LSP, admin UI, schema designer, watch, and MCP |

## Critical Path

The first end-to-end acceptance path is:

1. Apply `patches/0001-allow-trusted-hook-coextensions.patch`.
2. Build `companion/citus_timescale`.
3. Reconcile the `Hypertable` CRD in `operator/`.
4. Prove a Timescale hypertable partition under a Citus distributed parent works
   in an end-to-end cluster test.
