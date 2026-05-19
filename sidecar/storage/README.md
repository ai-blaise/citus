# sidecar/storage

Object storage sidecar contracts for metadata rows, presigned URLs, bucket ACLs,
and antivirus integration.

Current implemented surface:

- `StorageSidecarPlan`
- `BucketPolicy`
- `ObjectMetadataRecord`
- `PresignedUrlPlan`
- `AntivirusPlan`

These contracts cover `FEATURE: Sto1`, `FEATURE: Sto3`, `FEATURE: Sto4`, and
`FEATURE: Sto5`.

S3-compatible file storage service with metadata in PostgreSQL.
