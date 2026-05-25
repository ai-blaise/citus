# postgres_fdw Credential Rotation Runbook

This runbook covers the production-ready `FEATURE: F4` boundary: rotating an
existing `postgres_fdw` user mapping without embedding secret values in the
rendered plan. It assumes the foreign server, remote role, and foreign tables
already exist.

## Preconditions

- The new remote password has already been staged in the operator secret store.
- The old and new secret references are distinct.
- The validation table is a known foreign table owned by the rotation scope.
- Operators can tolerate disconnecting cached `postgres_fdw` connections for
  the mapping being rotated.

## Procedure

1. Render the companion plan:

   ```bash
   cargo run -q -p ai_blaise_citus_companion --bin companion_contracts -- run-fdw-credential-rotation-sql-canonical
   ```

2. Execute the rendered SQL with the new password supplied as a psql variable:

   ```bash
   psql "$DATABASE_URL" -v fdw_new_password="$NEW_FDW_PASSWORD" -f fdw-rotation.sql
   ```

3. Confirm the validation query returned `fdw_rotation_valid = true`.

4. Run an application read that crosses the rotated foreign table.

5. Remove or expire the old secret only after the validation read and
   application read both succeed.

## Rollback

Render the same plan with the previous password supplied as
`fdw_new_password`, execute it, and keep the new secret staged until the
rollback validation read succeeds. The helper disconnects cached FDW sessions
before and after the `ALTER USER MAPPING` so a stale connection is not mistaken
for a successful rollback.

## Evidence

`REQUIRE_DOCKER=1 ci/ai-blaise/fdw-credential-rotation-live-smoke.sh` is the
release evidence for this boundary. It proves that an old password is rejected
after the remote credential changes, that the generated plan contains no secret
literals, and that the rotated user mapping can read the remote table.

## Non-Claims

- No managed secret backend is reconciled by this repository.
- No Kubernetes `ExternalSecret` object is created or updated.
- No application pool is drained beyond `postgres_fdw_disconnect_all()`.
- No cross-region FDW topology change is performed.
