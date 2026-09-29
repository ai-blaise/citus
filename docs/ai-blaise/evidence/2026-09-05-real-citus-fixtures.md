# Real-Citus SQL fixtures — bounded development evidence

These receipts predate adoption of the updated **CHIMERA — FINAL PRODUCTION
PLAN**. They remain evidence only for the fork, sources, and images below.
PG16 is outside the new PG17.11/18.6/19beta3 rewrite matrix, and the historical
`ai_blaise_citus` companion is not the planned third `chimera` SQL extension.
None of these receipts proves the integrated Chimera operand, Apache-only
TimescaleDB, or a new-plan milestone. The new plan is recorded in
`ai-blaise/chimera:docs/porting/production-plan-2026-09-05.md` with its adoption
record alongside it.

On 2026-09-05 the source-built test fixtures passed the SQL-extension matrix
on PostgreSQL 16, 17, and 18, and the extension canary and security/recovery
matrices on PostgreSQL 17 and 18. These runs used native Linux/arm64 containers
in the existing local Lima development VM. They did not run on the GCP H200
hosts, publish an image, or qualify a release.

The checkout was based on `e10607031da0ccd2cb3fd948b22902959dcd5f9a` with
uncommitted changes. That Git commit is provenance, not the identity of the
tested worktree. The materializer's path/type/mode/content-framed source
SHA-256 was
`c072dfa5d75daf93a64b81ee01ec2e043078f749dceefd69e7dcaf0bc45a260d`
for all three images. The selected context includes the actual Citus source,
both extension build paths, and the companion install/upgrade SQL; it excludes
ignored host build products. Image reuse verifies the source identity, exact
parent reference, PostgreSQL major, Citus version, and test-only scope.

## Exact runtime images

| PostgreSQL | Verified local immutable Docker image identity |
| --- | --- |
| 16.15, Debian `16.15-1.pgdg12+2` | `sha256:c6273bd84ee12794112a0fa9b0043a20c21a3a71c9e6af23be4923caf0fd1b09` |
| 17.11, Debian `17.11-1.pgdg12+2` | `sha256:1b7ee236af39055df3e65137e54a7c1c67ef60d5d360a122851f383f961a2a31` |
| 18.6, Debian `18.6-1.pgdg12+2` | `sha256:b037990f5e3e26076f2ba0866d15ef0af440884345a9b79b8cb3f37fd1dcc58c` |

The fixture builds installed the real `citus` and `citus_columnar` libraries
and their complete generated SQL inventories. The Citus default was `15.0-1`;
the companion default was `0.1.2`. A successful source build against this
development Citus version is not a production-support or performance claim.
The immutable official PostgreSQL parent references are recorded in
[`base-images.lock.tsv`](../../../images/citus-test-fixture/base-images.lock.tsv).

## Observed results

| Harness | Executed majors | Result and boundary |
| --- | --- | --- |
| `sql-extension-smoke.sh` | 16, 17, 18 | Passed all SQL assertions and real idle-transaction observation. The bridge creates actual Citus hash distribution on `metric_time`, exactly two shards, and a successful inserted-row readback. Timescale dependency functions remain explicit call-contract fixtures in this lane; this is not real Timescale cohabitation proof. |
| `canary-upgrade-rollback-smoke.sh` | 17, 18 | Passed exact `0.1.0 → 0.1.1 → 0.1.0` paths, update-event presence/removal, default update to `0.1.2`, and both bare/default and explicit-current extension creation. This is a single-node extension graph test, not an operator rolling upgrade. |
| `extension-security-backup-smoke.sh` | 17, 18 | Passed populated restore of 44 tables, 24 sequences, and 153 routines; PUBLIC denial; explicit-grant preservation; rejected downgrade; failed-transaction rollback; and delegated-grant rejection. This does not prove distributed worker recovery or object-store PITR. |
| `observability-replication-smoke.sh` | 17 | Passed primary activity and streaming-replication lag assertions with the same real-Citus image on both nodes. A post-backup insert was committed on the primary and observed on the running standby. This is PostgreSQL physical replication, not a Citus coordinator/worker rollout. |

The SQL matrix's RLS role receives explicit EXECUTE grants on only the three
read/predicate helpers used by that fixture. It checks that the role lacks
EXECUTE on the session-claim setter. These checks establish the exercised
function ACL and row-policy behavior; they do not establish a general claim
authentication boundary for arbitrary SQL clients.

Two obsolete test assumptions were corrected before the successful SQL run:
the public Citus call stub was removed in favor of real catalog/data assertions,
and the RLS role was given the exact helper grants required after the `0.1.2`
PUBLIC-privilege revocation. No extension security policy was relaxed to make
the test pass. Earlier failed attempts are not successful evidence.

All eight successful cases used disposable test databases and removed
their own containers and volumes on exit. The optional Bundle1 source-build
branch of `sql-extension-smoke.sh` was not enabled. No B1–B6 or M0–M11 gate is
advanced by this document. Timescale licensing, complete operand boot,
multi-node rolling updates, trusted promotion provenance, and comparative
performance remain separate requirements.

## Exact tested harness files

Paths are relative to the repository root. A change to a relevant harness,
SQL input, materializer, build recipe, or runtime image requires new affected
evidence. The image's source identity covers its selected source inputs; it
does not substitute for the separate test-harness hashes below.

| File | SHA-256 |
| --- | --- |
| `ci/ai-blaise/build-real-citus-test-fixture.sh` | `467f1ca93007f1b7dc62f9864ed5a14990740f37e81e9dd98b16a4c1af5161d2` |
| `ci/ai-blaise/materialize-real-citus-test-fixture.py` | `0bb71121f40ec1de78da3bb94dad301efbe693ca99e98cdbc7b4c96765b25fe6` |
| `images/citus-test-fixture/Dockerfile` | `d3b7ba212656cf82944958d87ed2eb3e53d63aff02b346518268d2e90b16073a` |
| `images/citus-test-fixture/base-images.lock.tsv` | `4548c1b503a6072e9f0066c34f057bece703bb1c91f94247ae696e7ae51f0e4a` |
| `ci/ai-blaise/sql-extension-smoke.sh` | `6b0368e0a439b753f48b3e5c6f2fc4a60b83811e2b31ac57499497f9890f9619` |
| `ci/ai-blaise/canary-upgrade-rollback-smoke.sh` | `4d7d923885966709567096d4b1051f7600be606c5cfb4ec39fa7cb754f0b0bad` |
| `ci/ai-blaise/extension-security-backup-smoke.sh` | `016d9751f71c79418b9f794f931bd5d5ad55ab0f97e7db28f4159cbe247e5b55` |
| `ci/ai-blaise/observability-replication-smoke.sh` | `ef1da99a369cdd855bf1a007867929721518f66496b0196c716d94d1f25fb0ca` |
| `ci/ai-blaise/sql/extension-backup-seed.sql` | `068b65a1cdaf03e73506d279d27ec6aa04d422b80cd5b0ca46df9a7d2b69e354` |
| `ci/ai-blaise/sql/extension-backup-state.sql` | `e7101459410b1a4711f8a0ab9b2332423ec6aca75e73c7b17a67e1c61d46c023` |
| `ci/ai-blaise/sql/extension-security-assert.sql` | `9cda4c6692cbc4b2c12d5591db8f75f934656ac11ff8303e8dab32779454a5b6` |

## Reproduction

Use the matching source bytes and a Docker daemon with sufficient build space:

```bash
REQUIRE_DOCKER=1 bash ci/ai-blaise/sql-extension-smoke.sh
REQUIRE_DOCKER=1 bash ci/ai-blaise/canary-upgrade-rollback-smoke.sh
EXTENSION_SECURITY_PG_MAJOR=17 bash ci/ai-blaise/extension-security-backup-smoke.sh
EXTENSION_SECURITY_PG_MAJOR=18 bash ci/ai-blaise/extension-security-backup-smoke.sh
REQUIRE_DOCKER=1 bash ci/ai-blaise/observability-replication-smoke.sh
```

The builder selects the locked parent and source-bound fixture for each major.
For a single-major run, `CITUS_TEST_FIXTURE_IMAGE=sha256:...` may select an
already available matching fixture; all identity checks still apply. The old
stock-image override variables are intentionally rejected. The
[2026-09-04 security record](2026-09-04-companion-security-backup.md) describes
its own earlier source and image bytes and is not evidence for these harness
changes.
