# sidecar/storage

Object storage sidecar contracts for metadata rows, presigned URLs, bucket ACLs,
and antivirus integration.

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

S3-compatible file storage service with metadata in PostgreSQL. The runtime
surface deterministically enforces tenant bucket ACLs, object size limits,
presigned URL TTLs, and antivirus quarantine decisions for canonical tests.
