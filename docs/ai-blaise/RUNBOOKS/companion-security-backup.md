# Companion 0.1.2 security and logical recovery

`FEATURE: D9`

Version 0.1.2 is a forward-only security floor. The 0.1.0 install SQL and
0.1.0↔0.1.1 transitions remain unchanged. Its scope is the SQL overlay, not
upstream Citus cluster recovery, a runtime authorization service, or the Rust
rewrite's implementation.

## Privileges

The migration revokes PUBLIC privileges on precisely the 153 routines that
belong to `ai_blaise_citus`, resolved through PostgreSQL extension membership
and schema-qualified signatures. It does not revoke privileges on unrelated
objects, overloads outside the extension, a whole schema, or future objects
through global default privileges. Routines remain invoker-rights functions;
no SECURITY DEFINER elevation is introduced.

There is no default application role or blanket application grant. Before the
upgrade, inventory runtime callers and explicitly approve the exact routines,
schema USAGE, tables, and sequences each role needs. Granting EXECUTE alone does
not grant access to data used by an invoker-rights routine. Revoking PUBLIC
EXECUTE does not, by itself, authenticate sidecar requests or make body-sourced
tenant claims trustworthy.

An update that changes routine ACLs can record an existing administrator-issued
grant as an extension initial privilege in `pg_init_privs`. A subsequent `pg_dump` may omit that
grant because it appears to be supplied by the extension; a clean restore then
loses it. The populated recovery smoke reproduced this on PostgreSQL 17.
PostgreSQL documents the initial-privilege mechanism in its
[extension packaging guide](https://www.postgresql.org/docs/17/extend-extensions.html).

For this reason, bare `ALTER EXTENSION ... UPDATE` rejects existing non-owner
routine grants with SQLSTATE `55000`. Do not bypass this check or edit system
catalogs. From version 0.1.1, use the shipped administrative script:

```bash
psql -X -v ON_ERROR_STOP=1 \
  -f /usr/local/share/ai-blaise/citus/upgrades/ai_blaise_citus--0.1.2.sql
```

Supply connection settings through the deployment's existing protected libpq
configuration, not a password-bearing command line. Run in a new unassumed
superuser session during the maintenance window; stop other administrative
grant, ownership, extension, and schema changes first. The script captures
owner-issued grants and their grant options in temporary tables, removes them
before entering the extension update, and restores them afterward. Everything
runs in one transaction and one session. It verifies routine identity and exact
non-PUBLIC ACL equality before commit. An error or disconnected session rolls
back the entire operation; no intermediate revoke is committed.

Delegated grant chains and unexpected routine inventories are rejected before
mutation. Review such deployments explicitly; the script does not replace a
grant made by one role with a grant attributed to another role. Capture the
original grant graph in protected upgrade evidence and prepare an approved
replay under its original grantors. Do not convert those cases into an
automatic broad grant.

From 0.1.0, first use the historical explicit update to 0.1.1, preserving and
checking any existing ACLs before proceeding. Fresh installations without
custom routine default privileges resolve the install/update chain directly
to 0.1.2. Custom default privileges that grant routines to non-owner roles also
require explicit review; they must not silently become a backup exception.

## Backup coverage

All 44 extension-owned tables and their 24 bigserial sequences are explicitly
registered with `pg_extension_config_dump`. The table list includes operational
state, jobs and queues, ledger data, search documents, provider references,
storage attachment metadata, alarms, and extension upgrade events. No rows are
seeded during extension installation, so empty filters preserve the entire
contents without duplicating installation data on restore. Sequence state is
registered separately; registering the parent table is insufficient.

These are sensitive database backups. They can include customer data, webhook
headers, provider secret references, and executable job metadata. Encrypt them,
restrict reader and restore authority, and apply the deployment's retention
policy. Never publish them as CI artifacts or diagnostic logs. Logical database
backup does not include actual object-storage contents, external secrets,
cluster-wide roles, worker shard data in other databases, or WAL/PITR history.
Those need their existing independent backup and recovery contracts.

Restore into an isolated cluster with compatible, verified extension files and
roles already installed. Keep sidecars, schedulers, webhook delivery, and writes
stopped. Reconcile captured queues and jobs against external effects, invalidate
expired worker leases, and verify referenced secrets and object storage before
enabling work. This migration preserves state; it does not automatically replay
jobs or assert that externally completed effects can be repeated safely.

## Rollback and evidence

PostgreSQL cannot unregister configuration-dump tables without dissociating them
from the extension. Reopening PUBLIC execution is not an acceptable security
rollback. No 0.1.2→0.1.1 or 0.1.2→0.1.0 SQL edge is shipped. Roll back using a
pre-upgrade backup and PITR into a separate cluster, with an approved data-loss
window and traffic cutover. A post-upgrade logical dump is not a substitute for
a pre-upgrade recovery point.

`ci/ai-blaise/extension-security-backup-smoke.sh` requires an immutable
PostgreSQL image and an explicit major (17 or 18). It uses a network-isolated
temporary container, inserts one non-default row into each of the 44 tables,
advances all 24 sequences, and performs a custom-format dump and clean restore.
It compares every row through deterministic hashes, verifies sequence restart,
checks all 153 routine ACLs, denies an actual unprivileged call and unsafe
downgrade, preserves unrelated overloads, and checks that a pre-upgrade explicit
grant option survives restore. Temporary dumps and containers are removed.

Retain both major-version results with the exact source and image identities.
A passing SQL recovery test is not evidence of Citus multi-node rollback,
operand image publication, operator rolling upgrades, or disaster recovery for
external services.
