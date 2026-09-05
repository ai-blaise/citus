# Companion 0.1.2 SQL security and recovery — development evidence

Historical, source-specific record: its stock-image override is no longer
accepted by the current harness. A later source-built fixture run used changed
harness bytes and is outside this isolated source checkpoint; the hashes and
commands below remain authoritative only for this historical receipt.

Both native PostgreSQL 17 and PostgreSQL 18 runs passed on 2026-09-04 using
the existing `instance-20260415-20260415-235136` VM in `asia-south1-b`.
No cloud resources were created. This host is not a trusted release builder;
these results are bounded development evidence, not promotion provenance.

The test was executed against the files below, whose hashes were independently
matched between the local checkout and the VM after both runs. The repository
base was `e10607031da0ccd2cb3fd948b22902959dcd5f9a` with uncommitted changes.
This is not a whole-repository or operand-image receipt.

## Images and scope

| Major | Exact linux/amd64 image configuration used |
| --- | --- |
| 17 | `sha256:7bade6d532592ca8ce7ee32def7399dad2607c4ea5583839fc4352a095a11ea6` |
| 18 | `sha256:a10c981235b4f635e65df0cfb66a5598064628128505dbc6a3ed4ca303717521` |

The transferred official PostgreSQL parent manifest-list identities were
`postgres@sha256:051f7b7b3abdd564d5d1bd1e8c4b9c1b6e77087d1dd22020ede611c096a272e0`
(17) and
`postgres@sha256:1c59e2c3c818eaa0f0628f695b36e7c9e362d6b219b36a54a32df645cbd7e1af`
(18). Docker archive loading did not preserve their original manifest-list
association, so these runs selected the verified image configuration IDs
directly, not floating tags.

For each major, the script reported:

```text
version=0.1.2
tables=44
sequences=24
routines=153
populated_restore=passed
public_deny=passed
explicit_grants=preserved
downgrade=denied
failed_transaction_rollback=passed
delegated_grants=denied
```

The failure cases check rollback of both routine ACLs and `pg_init_privs`
before extension update and before transaction commit. A delegated grant
chain is rejected without mutation. The positive case preserves an existing
EXECUTE grant option through a real custom-format dump and clean restore,
compares all table rows by deterministic hashes, and verifies the next value
of every restored sequence. Unrelated functions and a non-member overload
remain executable. An actual unprivileged extension call is denied.

Each run used a network-isolated container. Test databases, temporary dumps,
and test containers were removed on completion. No production database was
modified. The runs do not establish worker-shard recovery, external object
storage recovery, operator rolling upgrades, or image publication.

## Exact tested files

Paths are relative to the repository root. Changes to these files invalidate
the corresponding result until the test is rerun.

| File | SHA-256 |
| --- | --- |
| `images/citus-pg-overlay/extensions/ai_blaise_citus.control` | `9238ca90c91c3632ca0997b2d66d0b1ef4f21317e2143bb958cbbec222574b16` |
| `images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.0.sql` | `c23c0887753118915c12b40ee6058ddd8920d95c33258353448c68b4e6c0ddb5` |
| `images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.0--0.1.1.sql` | `d1bfe3ad5f122b10b3786fdb2cb4a4f34b43fb421752c95ef50c619b77db7070` |
| `images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.1--0.1.0.sql` | `6a5c56cd621117cdcc9caef77b2b4541849ceeafa8992f2457138e549c6ab005` |
| `images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.1--0.1.2.sql` | `fe6738bccce024a60296f31e8eddb82d2e31c5445cf23c393cac998448c89722` |
| `images/citus-pg-overlay/upgrades/ai_blaise_citus--0.1.2.sql` | `351031464536f119ec6dac1917d4a8cfde18d524aa6f25b2d2df400c4e31c8aa` |
| `ci/ai-blaise/extension-security-backup-smoke.sh` | `8de733fcd0a3c09481b7fdfb21df487272a6eff2a398d07ca60855244354b885` |
| `ci/ai-blaise/sql/extension-backup-seed.sql` | `068b65a1cdaf03e73506d279d27ec6aa04d422b80cd5b0ca46df9a7d2b69e354` |
| `ci/ai-blaise/sql/extension-backup-state.sql` | `e7101459410b1a4711f8a0ab9b2332423ec6aca75e73c7b17a67e1c61d46c023` |
| `ci/ai-blaise/sql/extension-security-assert.sql` | `9cda4c6692cbc4b2c12d5591db8f75f934656ac11ff8303e8dab32779454a5b6` |

Reproduce with the matching immutable image available locally:

```bash
EXTENSION_SECURITY_PG_MAJOR=17 \
EXTENSION_SECURITY_IMAGE=sha256:7bade6d532592ca8ce7ee32def7399dad2607c4ea5583839fc4352a095a11ea6 \
bash ci/ai-blaise/extension-security-backup-smoke.sh
```

Repeat with the PG18 major and image ID above. CI and the Make target run both
majors using the pinned official manifest-list references. Operational details
are in [the security and backup runbook](../RUNBOOKS/companion-security-backup.md).
