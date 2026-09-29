# sidecar/storage

> Production boundary: unless a feature is explicitly `Status: production-ready`
> in `docs/ai-blaise/NEW_FEATURES.md`, the surfaces listed here are alpha
> contracts. Deterministic canonical reports and local runtime models are CI
> artifacts, not production evidence; promotion requires live VM/container or
> Kubernetes evidence recorded in `docs/ai-blaise/PRODUCTION_READINESS_AUDIT.md`
> and guarded by `ci/ai-blaise/production-gap-audit.sh`.

Object storage sidecar contracts for metadata rows, presigning policy, bucket
ACLs, and antivirus integration.

No provider-backed signer is configured in this crate. The production HTTP
boundary therefore returns `503` for `POST /storage/presign` and never returns a
placeholder URL. `PresignedUrlPlan` models the intended policy contract only;
it is not evidence that a usable URL was issued.

Current implemented surface:

- `StorageSidecarPlan`
- `BucketPolicy`
- `ObjectMetadataRecord`
- `PresignedUrlPlan`
- `AntivirusPlan`
- `ObjectUploadRequest`
- `StorageRuntime`
- `StorageRuntimeState`
- `canonical_storage_report()`
- `canonical_storage_runtime_report()`
- `cargo run -p ai_blaise_citus_sidecar_storage -- run-canonical`
- `cargo run -p ai_blaise_citus_sidecar_storage -- run-runtime-canonical`

These contracts cover `FEATURE: Sto1`, `FEATURE: Sto3`, `FEATURE: Sto4`, and
`FEATURE: Sto5`.

S3-compatible file storage contract with metadata in PostgreSQL. The local
runtime model deterministically exercises object-size and antivirus quarantine
decisions for canonical tests. Tenant authentication, provider-backed object
I/O, and real URL signing remain unimplemented and fail closed.
