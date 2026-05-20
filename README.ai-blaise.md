# ai-blaise/citus

`ai-blaise/citus` is an upstream-minimal fork of `citusdata/citus`.

The fork keeps Citus source changes in `patches/` and implements new
capabilities in overlay directories:

- `companion/` for the Rust `pgrx` companion extension
- `sidecar/` for out-of-process Rust daemons
- `pool/` for the shard-aware pgcat-derived pooler
- `operator/` for Kubernetes CRDs and reconcilers
- `tools/` for CLI, TUI, LSP, admin UI, schema designer, watch, and MCP tools
- `deploy/k8s/` for the ai-blaise Helm chart and CRD bundle
- `docs/ai-blaise/` for fork docs, ADRs, runbooks, and the feature register

The canonical list of functionality added beyond vanilla Citus is
[`docs/ai-blaise/NEW_FEATURES.md`](docs/ai-blaise/NEW_FEATURES.md).
