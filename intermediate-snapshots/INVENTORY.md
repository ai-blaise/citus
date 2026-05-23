# Intermediate subagent worktree snapshots

Captured 2026-05-23 from the gcloud experiment-playground VM. Each tarball is a worker's entire working tree (committed + uncommitted, excluding target/ and .git/) preserved before the worker exited.

Extract with: `tar -xzf <snapshot>.tar.gz -C <dest>`

| Snapshot | Branch HEAD | Dirty | Ahead | Scope | Size |
|---|---|---:|---:|---|---:|
| `citus.tar.gz` | `46e2f2e4cc` | 12 | 0 | sidecar-auth-jwt-runtime | 8.3M |
| `api-trio.tar.gz` | `46e2f2e4cc` | 7 | 0 | sidecar-api-trio-postgrest-graphql-edge | 8.3M |
| `backup-walg.tar.gz` | `d1fad0780e` | 12 | 0 | sidecar-backup-walg-runtime | 8.2M |
| `bundle1-source-build.tar.gz` | `d1fad0780e` | 15 | 0 | bundle1-source-build-9-packages | 8.2M |
| `cdc-realtime.tar.gz` | `46e2f2e4cc` | 23 | 0 | sidecar-cdc-realtime-runtime | 8.3M |
| `citus-patches-0003-0005.tar.gz` | `dce196ea67` | 1 | 0 | citus-patches-0003-0005-quilt | 8.2M |
| `crd-versioning.tar.gz` | `a65ce2811c` | 1 | 0 | crd-api-versioning-conversion-webhooks | 8.3M |
| `dr-drills-chaos.tar.gz` | `5c0641f22c` | 0 | 1 | dr-drills-chaos-verification | 8.3M |
| `microbenches-26-bundled.tar.gz` | `ebb44d361d` | 0 | 1 | microbenches-26-bundled-extensions | 8.3M |
| `otel-logs-schema.tar.gz` | `5fd665fd95` | 1 | 0 | otel-trace-propagation-logs-schema | 8.3M |
| `patches-0004-0006.tar.gz` | `014911fee3` | 4 | 0 | citus-patches-0004-0006-router-planner | 8.3M |
| `patches-0007-0008.tar.gz` | `014911fee3` | 3 | 0 | citus-patches-0007-0008-cohabit | 8.3M |
| `pg16-rebalance-flake-fix.tar.gz` | `9e65a9d6bf` | 1 | 1 | pg16-background-rebalance-flake-fix | 97M |
| `pg18-smoke-matrix.tar.gz` | `a86800027a` | 1 | 0 | pg18-smoke-matrix | 8.3M |
| `pool-depth-hardening.tar.gz` | `46e2f2e4cc` | 12 | 0 | pool-depth-hardening-12-concerns | 8.3M |
| `promotions-chunk-1.tar.gz` | `53cca518c0` | 1 | 0 | new-features-promotions-chunk-1 | 8.3M |
| `raft-hlc-txn-triad.tar.gz` | `46e2f2e4cc` | 21 | 0 | sidecar-raft-hlc-txn-status-triad | 8.3M |
| `reconcilers-batch-a.tar.gz` | `014911fee3` | 11 | 0 | reconcilers-batch-a-tenant-region-survival-backup | 8.3M |
| `reconcilers-batch-b.tar.gz` | `46e2f2e4cc` | 10 | 0 | reconcilers-batch-b-federation-search-webhook-function | 8.3M |
| `reconcilers-batch-c.tar.gz` | `46e2f2e4cc` | 9 | 0 | reconcilers-batch-c-repack-migration-conflict-sidecar | 8.3M |
| `sidecar-ha-autoscale-layer.tar.gz` | `2fc0b5cfa5` | 38 | 1 | sidecar-ha-autoscale-topology-spread-17 | 8.3M |
| `ts-2-28-matrix.tar.gz` | `014911fee3` | 15 | 0 | ts-2-28-cohab-matrix | 8.3M |
| `vectorizer-runtime.tar.gz` | `d1fad0780e` | 9 | 0 | sidecar-vectorizer-real-runtime | 8.2M |
