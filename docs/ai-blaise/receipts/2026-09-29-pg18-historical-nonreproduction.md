# PG18 historical check-multi: non-reproduction evidence

## Outcome and limitations

The original PG18 `merge_repartition1` missing-relation failure did **not**
reproduce in either of two ordinary full-suite comparison runs. Both passed
all 194 tests, including the original concurrent `merge_repartition1` and
`merge_schema_sharding` pair. This is not bug closure, evidence that the CDC
repair caused a fix, a waiver of the failed public CI job, or latest-minor
PostgreSQL release acceptance. The earlier base passed too.

| Source | Actual suite result | Actual exit | Completed UTC | Log SHA256 |
| --- | --- | --- | --- | --- |
| Candidate `e6efe2aed9a7e413f00e60faadf6dfe0c381ad6e` | 194 / 194 pass | 0 | 2026-09-29T23:45:03Z | `4b94806a0817df82df2610d45ce93579af0eab18587cd688e53abc62f111d53e` |
| Pre-repair base `cd119257cdcba890d0c887a235ec6dc39d7e0cf9` | 194 / 194 pass | 0 | 2026-09-29T23:45:01Z | `16c22f7a67749fca603af7db1bbeea95c8a6f160ad8e91d99df27d7318c54e35` |

## Reproduction apparatus

Both runs used the original public job's immutable test image:
`ghcr.io/citusdata/exttester@sha256:c5156ea345c57bb4e85d522e18adca4d20b8f11beeccd77aab065022b990e36d`,
PostgreSQL 18.4 (Debian 18.4-1.pgdg12+1). Each source was independently
recompiled with the original CI's `CFLAGS=-Werror`, `--enable-coverage`, and
`--with-security-flags`, using a tracked Git archive and an out-of-tree
`build-18`. The test-stage configure and complete `make -C src/test/regress
check-multi` schedule were unchanged. The metadata source and relevant SQL
were verified byte-identical between the two compared commits.

This reproduces source, test image, compiler flags, SQL, and scheduled
concurrency, not a bit-identical historical compiled artifact or every hosted
CI timing/resource characteristic. Different host and timing conditions may
matter for an intermittent failure. Original test assertions were not changed,
skipped, serialized, or marked expected-failure.

The existing VM B ran two private containers, each capped at 8 CPUs, 16 GiB,
2,048 PIDs and 1 GiB shared memory. They had no external network or published
host ports. The historical CI's SYS_NICE/seccomp options were retained without
a host Docker socket or host PID namespace. Both clusters stopped normally;
both owned containers were removed, actual exits were zero, and no OOM kill
was recorded. The shared source checkout remained clean.

## Original failure remains open

Public run `36588894453`, job `109477918656`, failed one of 194 tests.
At 15:58:17.217 UTC the worker on port 57637 reported missing relation OID
51966 in `GetCitusTableCacheEntry`, metadata_cache.c:1545, while executing
`citus_internal.add_partition_metadata` for
`merge_repartition1_schema.citus_target`. The coordinator propagated that
error; its later colocation and cleanup failures were derivative. Original
raw job and ZIP evidence remain in the prior CDC baseline seal.

Read-only source inspection identified a hypothesis, not a proven cause:
`EnsurePartitionMetadataIsSane` selects a colocation peer and then calls
`DistPartitionKeyOrError`; `ColocationGroupTableList` holds an access-share
lock on the metadata table, not an explicit relation lock on each selected
peer. The concurrent schema test performs ordinary schema drops. Existing
coordinator locking may prevent the hypothesized interleaving, so this source
observation alone does not establish a defect or justify a fix.

No debugger/ptrace race probe, force-unlock, catalog edit, or source patch was
executed. The separately drafted diagnostic harness was not run and is not
qualified evidence. The historical SIGKILL/query-generator timeout, earlier
CDC SIGSEGV, and other CI failures retain their own conclusions and identities.

## Retained evidence

VM evidence root: `/home/spencer/citus-cdc-resume-sep29-evidence`.
The complete runs remain under `metadata-historical-candidate-v1` and
`metadata-historical-base-v1`, including build/configuration logs, normalized
and unmodified SQL results, postmaster logs, stopped fixture snapshots,
immutable image metadata and exact source identities.

[`2026-09-29-pg18-historical-evidence.json`](2026-09-29-pg18-historical-evidence.json)
seals 833 explicit ordinary-run artifacts and the source-equivalence
witness. SHA256: `760c1db17713a3741a48a57db8c091334045ec70cb0c39e13c8b70bc62f827fc`. This receipt adds evidence only and does not
retarget the prior CDC qualification, a production lock, or any upstream pin.
