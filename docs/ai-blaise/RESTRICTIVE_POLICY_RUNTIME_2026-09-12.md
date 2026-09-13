# Restrictive-policy reconstruction: bounded native qualification

2026-09-12: corrected RLS 7 and metadata 10 both PASS on three private
PostgreSQL 17.11 postmasters on one macOS arm64 host. This is not whole-fork/CC
readiness, a multi-host proof, or an upstream PR.

## Source and tested artifact

Parent: ai-blaise/citus bootstrap-v2
624ad5a762581b5908a0e38e44a7c3477aeb9535.
Official reference: f1728591d33bd13d113d2a8d66bb99c377c31116, not merged here.
The common serializer emits AS RESTRICTIVE only for restrictive policies;
ordinary permissive serialization is unchanged. FEATURE: Sec1 marks the code.
Patch 0012 represents the actual in-tree C, two SQL regressions and two observed
expected additions. The image compiles in-tree source: do not apply it twice.

Source SHA256:

- policy.c ecc47d02634b5092eb9a62a4861de99b5a18bb7e182aeb4f44bfd880c3d0887d
- RLS SQL a5a2b4fe84f1f6bcdd8835dccf5bae0e3873f5e3b666f9db68710b73d12e5e20
- metadata SQL 131ca86aac1dbf65b7088d74aa3bddc72762ead418a857aec9ede44a5697708c
- RLS expected cc8657e76ef6ef2984da55cdd6fe5613ad39514f3f6e402894a1954207891897
- metadata expected 564988995f547c0e2b750bcd19ed7c75ab93c5299ee8a1bb7e39782d170c816e
- patch 0012 82697430c843cbf67b224d974f9265354192177519f7497c40e2cad2d2f487d0
- corrected citus.dylib 46d2f58bf6845066d6f7502b07bafe8e43ad7c00f36ff5a6f0f2d06bc51d1272

Both native operands were built/staged earlier at
/private/tmp/citus-policy-full-build-20260912.4qjjxy,
result 2d08df74a494366d8691c8ec144e42bf4a2df838da2213d163ad76629a4105ab.
Identical warning categories on both operands were retained: overridden check
target and unused safeclib length variable; the build was not warning-free.
Final oracle-only qualification reused the same corrected module without rebuild.

## Negative evidence and observed oracles

Four prior /private/tmp/cpg-rbufPv baseline/corrected RLS 7/metadata 10 lifetimes
remain FAIL/pg_regress_nonzero. Baseline native catalogs, least-role rows,
forbidden writes and real movement/recreation reproduced the defect. Corrected
behavior was right, but the new blocks were absent from the old expected files.
No historical failure was relabeled PASS. Complete four-run identities:
 /private/tmp/citus-policy-rls-expected-append-20260912.6E3mnl/FOUR_RUN_REVIEW.md
7146dcec85c4e8ddb7c97e9b19e6620b54804f84c23b40146b94fc7b77d22546.

The reviewed additions insert 230 observed RLS lines before cleanup and append
260 observed metadata lines at EOF. Every incumbent expected byte is retained.
No rows, error text or normalizer rules were invented to make a comparison pass.

The local default 10% disk reserve initially refused tiny moves. No user data
was deleted. The final private fixture uses the original runner's supported
server-option/userPgOptions-style seam to set reserve 2.0% with space checking
still true. Actual startup reads verified both on all three postmasters.
Product default 10% and production configurations are unchanged. This does not
qualify default-reserve operation or the disk-space policy.

## Final schedules and results

Candidate: /private/tmp/citus-policy-final-20260912.gUA8PR/source.
Controller: /private/tmp/citus-policy-final-control-20260912.d6cfbn.
Runtime roots: /private/tmp/cpg-2ZVc6t/c-r and c-m.
Each owner invoked the unchanged --run-approved controller once, selecting one
genuine schedule with the existing normal pg_regress/custom-diff flow.

| Run | Actual tests | Elapsed seconds | Result |
| --- | --- | ---: | --- |
| c-r | 7 PASS | 11.72349650002434 | status 0 / passed |
| c-m | 10 PASS | 15.557585708011175 | status 0 / passed |

RLS 7: single_node_enterprise; multi_test_helpers + multi_test_helpers_superuser
(original parallel group); multi_cluster_management; multi_test_catalog_views;
multi_data_types; multi_alter_table_row_level_security.
Metadata10: multi_test_helpers; multi_test_helpers_superuser;
multi_cluster_management; multi_table_ddl; multi_sequence_default;
alter_database_propagation; alter_role_propagation; multi_test_catalog_views;
multi_drop_extension; multi_metadata_sync.

Every final named normalized result equals its expected bytes. RLS covers
physical-placement modes, actual non-owner/non-superuser/non-BYPASSRLS rows,
both rejected writes with unchanged counts, and successful placement movement.
Metadata covers reconstruction text, immediate DDL, activation, actual shell
absence/full recreation, independent restrictions, least-role reads and both
workers active at completion. Each schedule kept one complete database lifetime.

## Evidence and remaining limits

All 229 source bindings matched before/after both runs:
490e3587b6474f2b298a9bb13b2e8c3b425f6b330a5a033c742b3142dd59f92b.
Corrected installation manifests matched
ae5dca5dabeaef050e064def0363b4bc1b6b5b43e4b3502c4484caa994ca052a.
Exact raw names are in each evidence/020-pg-regress.stdout. Independent external
RECONCILIATION.json 7c959d43f58920f9b81165bdbe3802a713b7d121219f561d8076f58aac5bfa4c
matches 48 records and 204 streams, 17 named executions, normal reaping of 54 children
and both controllers, no forced fallback/signals, source/output rename custody
and actual startup/endpoint proofs. Generated keys/account data were not read.
Detailed process polling and endpoint/normalizer limits stay in that evidence.

c-r owner /private/tmp/citus-regress-owner-d_5le_kz;
result 8eae6f71f3c70779b463d5a8e0cd767d4b1edc919400bc011eeb5807f7c4cb91.
c-m owner /private/tmp/citus-regress-owner-b3bfsdxj;
result 6a7eea08680a4c559686d9a0cbd55bafbdbfb5a810e3161380242f27e14e393e.
After passing, only this receipt and the narrow NEW_FEATURES status/reference
were authored; qualified C/SQL/expected/series/patch bytes stayed unchanged.

No broad release suite, image redeployment, upstream-main merge, trigger opt-in,
CC migration/storage activation, Chimera generated mapping/native port,
multi-host performance or whole-product readiness is established. Independent
trigger DDL/propagation/rollback prerequisites remain. No unsafe-trigger GUC was enabled by this correction.
Existing Sec1 helper readiness and other features' alpha statuses are unchanged.
