# W1/D9 source checkpoint — 2026-09-05

This is a tested development checkpoint, not a release or completion receipt.
Bundle1 and D9 remain alpha in this snapshot; the production audit explicitly
blocks release. No image was published or promoted and no new GCP resource was
created.

## Scope and source identity

The checkpoint starts at `e10607031da0ccd2cb3fd948b22902959dcd5f9a` and contains
the isolated 73-path W1/D9 dependency closure, plus this receipt and the
packaging patch described below. Other concurrent working-tree changes are
excluded. The closure contains:

- Default preload inclusion, explicit light/full image targets, complete SQL
  installation, and a no-override default-boot smoke with negative controls.
- Companion 0.1.2's routine-ACL security floor, privilege-preserving upgrade,
  and configuration-dump registration for state tables and sequences.
- Source-verified real-Citus test fixtures, their consumers, and version/gate
  wiring needed to run the current security and recovery tests.
- Accurate development-evidence boundaries and operator test-version bindings.
  The operator changes are test-only, not new controller functionality.

The frozen 73-path candidate has these identities:

| Identity | Value |
| --- | --- |
| Base-to-candidate mode/content manifest SHA-256 | `d188f6f9bcabbcde32356eee41bf031c21ccda6ed0d7180e6c1e79986986b40a` |
| Complete file/symlink manifest SHA-256 | `9810c87a48718547fae9e63dfeb2226b813cc11dba727365adfb31c960ce1a91` |
| Exact Git-backed static-test tree | `992060c2d03ed6249b7a1f767d8fdfee4ae22515` |
| Transferred native candidate archive SHA-256 | `c8871841c97bc300fa738e0f2f54d3cd8c18be279b02dd2c5f7a84fcd37834e0` |

The static tree, selected native input-content identity, base Git tree, and a
later checkpoint commit are distinct identities. The native fixtures honestly
report the base revision plus dirty source state; they do not claim a clean
build of this subsequent commit.

## Verification

On the exact Git-backed static snapshot: 52 basic fixture tests and 16 Timescale
fixture tests passed; the D9 graph guard passed six graph and five forward-only
regressions. Image and Bundle1 contracts, 155 shell syntax checks, 11 Python AST
checks, two workflow YAML parses, whitespace and changed-document links passed.
Wrong preload order, a missing required library, a version-check bypass, an
incorrect target label, and unverified metadata promotion were rejected by the
W1 negative controls. The production audit found all 276 source IDs and document
headings, with 274 ready and two alpha; this audit count is not an independent
production qualification of every feature.

Root independently reran image, Bundle1, upgrade/rollback and production-gap
checks. The two Git-aware guards first refused the plain exported candidate
because it has no `.git`; both passed when rerun on its exact Git-backed test
snapshot. No guard was bypassed or weakened for that rerun.

## Current native security and recovery evidence

The current `extension-security-backup-smoke.sh` ran on the existing
`instance-20260415-20260415-235136` in `asia-south1-b`. PG17 ran from
20:53:38–20:54:35 UTC; PG18 ran from 20:54:59–20:55:55 UTC. Both exited
successfully using source-built fixtures with locked official PostgreSQL parent
images, Citus extension version `15.0-1`, and `release-target=false`.

Both validated 44 state tables, 24 sequences and 153 routines; populated
dump/restore, PUBLIC execution denial, preservation of explicit grants and grant
options, downgrade refusal, rollback after injected failures, and rejection of
delegated grants passed. The current harness hash is
`016d9751f71c79418b9f794f931bd5d5ad55ab0f97e7db28f4159cbe247e5b55`.
The transition, wrapper and three SQL inputs match the earlier historical
receipt; this run additionally verifies the current source-built-Citus harness.

| Artifact | Identity / SHA-256 |
| --- | --- |
| PG17 fixture image | `sha256:3a0d79cc4048ba86d146c4db7a0e3446209babec8c073c0829ee69ab9a1c98cf` |
| PG18 fixture image | `sha256:275e8ce8e4f57a97d9fd7fb885ffca286dfe2898741d11bf7548efc2b82fcc95` |
| Selected source-content identity, both images | `c072dfa5d75daf93a64b81ee01ec2e043078f749dceefd69e7dcaf0bc45a260d` |
| PG17 terminal log | `1b2b9330e87dd3b73e0f337abc576fb56d1fc09b96fa558f60c23eaffcc0e341` |
| PG18 terminal log | `966581ae5bf2783dede92d7acbd576d0c2c25de457c3ecedeeedb0f8d30dec8e` |
| Native receipt | `9cb0fa9f5bd06a912952840999912fee7e5cd9c81e9deaff290994faaf7e2b24` |

The remote artifacts remain under
`/home/spencer/chimera-w1-d9-security-candidate-20260905/`. The receipt records
the full parent references, image-inspection/history hashes and all six tested
source-input hashes. Local copies of the receipt and terminal logs were checked
against those hashes before this checkpoint.

## Packaging patch and remaining boundary

`patches/packaging/ci-install-complete-extension-artifacts.patch` represents the
already-applied change to upstream-owned `ci/build-citus.sh`: install the full
extension SQL graph or fail, and package using the explicit file list. It is an
upstream-targetable packaging patch, deliberately not an entry in the eight
Citus runtime patches in `patches/series`. It must not be reapplied to the
already-modified checkout.

The prior full Bundle1 default-boot result is documented separately in
[its dirty-candidate receipt](2026-09-05-bundle1-full-default-boot-dirty-candidate.md).
It is not a clean rebuild of this checkpoint. Mutable external dependencies,
trusted release provenance, full release publication, cluster recovery/rolling
upgrade, complete feature qualification, and Chimera's M gates remain open.
Neither these security fixtures nor the static checks close those requirements.

## Fixture-test Git isolation follow-up

A test invocation against the exported candidate inherited `GIT_DIR` and
`GIT_WORK_TREE`. A nested fixture `git init` consequently persisted the candidate
path as the live repository's `core.worktree`. Publication used an explicit live
worktree and verified every staged blob; the task-created configuration override
was then removed, and the live top-level and empty index were verified. The
Git-backed static snapshot tests described above did not rely on that override.

Disposable-repository test scopes now remove inherited `GIT_*` overrides and
disable global/system Git configuration, while preserving ordinary environment
settings and restoring the complete environment afterward. Production
source-provenance operations are unchanged. A regression runs the three actual
temporary-repository cases with overrides pointing only at a disposable parent
sentinel; its configuration, index, refs, objects, and source bytes and modes
must remain identical. The existing CI test entry point includes this regression.

The Citus fixture suite passed all 55 tests and the Timescale suite passed all
16 after this test-only follow-up. These tests do not add native release evidence
or change the checkpoint's alpha boundary.

## Later canonical W1 boot evidence

The original checkpoint and dirty-context observations above predate the clean
CI run in this section. A later canonical push of commit
`43e7b309ff3fdfdc82bf6d18107490393b125ce0` ran the
[`image-contract` workflow](https://github.com/ai-blaise/citus/actions/runs/33993073146)
from Git tree `b940a99696b43b0cec5f800627a4d50bfc00abf1`. Both PG17
operands used the locked parent
`postgres:17-bookworm@sha256:7bade6d532592ca8ce7ee32def7399dad2607c4ea5583839fc4352a095a11ea6`,
recorded an image ID, and booted through the stock PostgreSQL entry point with
no command override.

| Operand | Job | Result | Image ID | Official log SHA-256 |
| --- | --- | --- | --- | --- |
| `bundle1-final-light` | [`101378646774`](https://github.com/ai-blaise/citus/actions/runs/33993073146/job/101378646774) | Build and default boot passed | `sha256:97e016d1daffae11519feb05a2c49d1a0fd18e0e0ce4012986fac10bebdc4422` | `65d1fdce0dfac731027263737b139e0e562b8b062c58acb069312efabb921939` |
| `bundle1-final-full` | [`101378646744`](https://github.com/ai-blaise/citus/actions/runs/33993073146/job/101378646744) | Build and default boot passed | `sha256:1dc292e84f5f233782a37da4baed40c42386eb39e246d12f627aff537f1e12e1` | `67cb7fe860ba05c6c51c297f7c4ff2c4b6fac4f6bb7b14f3b1d3c485722fc741` |

The full build ran from 21:27:19 through 22:04:02 UTC and its no-override
default boot passed at 22:04:07 UTC. The smoke verified PG17, Citus `15.0-1`,
`ai_blaise_citus` `0.1.2`, all 26 required SQL extensions, the one reviewed
preload-only library, target `bundle1-final-full`, scope
`full-bundle-required-minus-plrust`, clean source provenance, and
`release-target=true`. The light smoke separately verified its closed 24-SQL
extension subset and one preload-only library.

The source and executable contract inputs for that observation are:

| Input | SHA-256 |
| --- | --- |
| `.github/workflows/ci-image.yml` | `bd5f9800d9e501997a3f9f82fd3896d2132296a0aca812a93078e03d7a936a22` |
| `images/citus-pg-overlay/Dockerfile` | `6125ca677cdce83e56d34e38848163634a297abc06108228de2cc73d0cdf4b0c` |
| `images/citus-pg-overlay/bundle1-source-build.lock.tsv` | `440224d931c5265c6a8aba2684b0ac407677659d874370048c85fa3c34a39105` |
| `images/citus-pg-overlay/extension-manifest.tsv` | `87553eb715ad52c521db0d77f2201dc60e27088cc56fac4f55bfaea11c5d902f` |
| `images/citus-pg-overlay/shared-preload-libraries.conf` | `4ace37465396061af27dc30527ca3d299253e4300dc917fd02e119f94486b47b` |
| `ci/ai-blaise/bundle1-default-boot-smoke.sh` | `2fdbf228c2578299a21c506fdecf9a4e4ebc89cdc6c434092cefa02702e592c6` |
| `ci/ai-blaise/bundle1-contract-check.py` | `2c2091edf71ca7d1a8bfbcf74561a18c14b67b8640a5f54e17ad1efa98ae6fe0` |
| `ci/ai-blaise/image-check.sh` | `d80a6cf645996135c96604c50e2b30e08274f29ac331c21d5790d693a9909fab` |

This closes only the corrected plan's W1 source-bound full-image boot
prerequisite. The workflow did not publish the image. A local image ID and a
`release-target=true` label are not a registry digest, signature, provenance
attestation, or production release. B1, mutable-dependency sealing, repeatable
release publication, cluster recovery and rolling upgrade, complete feature
qualification, Chimera's W2 harness/source freeze, W3 measurements, and every
M0–M11 gate remain open.
