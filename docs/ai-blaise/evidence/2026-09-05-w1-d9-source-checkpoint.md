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
