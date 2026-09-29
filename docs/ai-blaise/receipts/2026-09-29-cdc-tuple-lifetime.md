# CDC tuple-lifetime repair: source-bound native qualification

## Candidate and scope

Qualification source: `077a3e4243f875841d8f7ede4e305f1b7ad63032`, tree `51ed3de06ef40720c783801e308a5dc0e29e3895`.
Ownership repair: `35c39b7d2be0438119d020dc6000c6da8bc9de6a`.
Test-only failing-first parent: `679cd22d00d7ed17f1254a4798f4528d18b309b0`.
Original preserved fork candidate: `cd119257cdcba890d0c887a235ec6dc39d7e0cf9`.

The repair carries upstream `211afb11fd5074b12621dd2947260ca570f918c2`
with explicit PG16 embedded-tuple ownership. It retains the real owning
allocations through volatile out-slots, restores original reorder-buffer
storage, and closes relations through PG_FINALLY on normal and error paths.
The separate child commit adds narrowly scoped test-fixture output-plugin
admission and an operator-controlled production upgrade runbook.

These results qualify this CDC change only. They do not establish whole-fork
CI success, production readiness of every feature, a publication, a new
Chimera operand lock, or an upstream-source pin update. No such action was
performed by this phase.

## Failing-first ownership witness

The native harness mechanically extracts four actual product functions and
runs them in a PostgreSQL backend with real heap tuples and PG error handling.
On its test-only parent, each of PG16.14, PG17.11 and PG18.6 had 8 passing and
16 failing cases (actual exit 1). On the ownership repair each had 24 passing
and 0 failing cases (actual exit 0): 72 passing ownership/error-path controls.
The PG16 host attempt with missing development headers remains a separate
apparatus failure, not a product result.

The decoder bytes at the full-suite child are unchanged:
`387682f0907c51453ffead0279385f38924f554eff59cf87b608c561138e0267`. Extracted helper SHA256:
`8a7160d68f25a17f46058774644b1f4c67d97ad0b5e43a945b24401261bdacc4`.
See [native witness boundary](../../../ci/ai-blaise/cdc-ownership-native.md).

## Full native extension builds and live CDC

All three runs compiled and installed the exact frozen candidate, then ran
all 16 unmodified CDC TAP assertions files, including the real
`006_cdc_schema_change_and_move.pl`. Missing TAP support or a missing positive
test summary is a failure in the owned launcher, not a skip.

| PostgreSQL | Files / assertions | Actual exit | Completed UTC | TAP log SHA256 |
| --- | --- | --- | --- | --- |
| PostgreSQL 16.14 (Debian 16.14-1.pgdg12+1) | 16 / 43 | 0 | 2026-09-29T23:26:51Z | `a2cdb70427be9665450ad05c1f711c1c25719dea650dd2c41006e43686deebd8` |
| PostgreSQL 17.11 (Debian 17.11-1.pgdg12+2) | 16 / 43 | 0 | 2026-09-29T23:26:54Z | `00d19a24d18b117b279fad43d68df87a67fc8456055b9d2a26a73de35aa10840` |
| PostgreSQL 18.6 (Debian 18.6-1.pgdg12+2) | 16 / 43 | 0 | 2026-09-29T23:26:56Z | `bdeb6560dbd7ac79979393e826e06055375f31c22c312d47f1416f7ab789c425` |

Total: 129 passing live assertions. Containers were private, without external
network or host-published ports, capped at 8 CPUs, 16 GiB and 2,048 PIDs each.
The existing VM B was used; no cloud resources were created. Test dependency
packages were added only to derived test images; exact versions and immutable
image IDs are in the manifests. PostgreSQL versions were verified unchanged
by dependency provisioning. Every owned container was stopped and removed;
the source checkout was clean before and after every qualification.

## Preserved red evidence and security compatibility

The initial full suites on `35c39b7d` passed PG16.14 (16 files / 43 assertions)
but failed PG17.11 and PG18.6 (16 files / 25 assertions observed, actual exit 2).
Eight test files could not load `citus` or `wal2json` because the new
`output_plugin_libraries` security policy had not explicitly admitted them.
These are retained original failures, not retrospectively relabeled passes.

The compatibility child detects GUC availability, preserves existing policy,
explicitly admits only required fixture decoders, reloads, and verifies exact
readback. It does not disable the restriction or mutate production policy.
See [operator admission runbook](../CDC_OUTPUT_PLUGIN_ADMISSION.md), including
the primary PostgreSQL security-release references and upgrade checks.

The older PG18 server SIGSEGV remains preserved independently. Public fork
job conclusions also remain open: PG17 query-generator had a coordinator
backend SIGKILL followed by incomplete shutdown and timeout (kill cause not
proven); PG18 check-multi had missing relation OID 51966 during worker
`citus_internal.add_partition_metadata`, originating at metadata_cache.c:1545;
32 historical flakyness jobs failed. The inspected flakyness sample showed
public-schema helper dependencies obstructing the 9.5 downgrade. This CDC
matrix does not prove those failures fixed or establish a cause for all samples.

## Evidence custody

VM B evidence root: `/home/spencer/citus-cdc-resume-sep29-evidence`.
The original six native runs, original full-suite runs, raw CI job/artifact
responses and historical crash are sealed in
[`2026-09-29-cdc-baseline-evidence.json`](2026-09-29-cdc-baseline-evidence.json),
SHA256 `55ae964247f8a7e5672e335eae809767b5c1976e0e4df4cf02a143554c138276`.
All 702 original artifact hashes were reverified before this final seal.

The fresh live runs and launchers are sealed in
[`2026-09-29-cdc-qualified-evidence.json`](2026-09-29-cdc-qualified-evidence.json),
SHA256 `6cbf0d0f96431384d1b9992fb9b9b01a15b365a75b6bc7c6f06e1a5ebc14685b`. Raw paths, sizes and SHA256 values are explicit.
The receipt commit is documentation only; it does not retarget any historical
source identity or change the qualified product tree.
