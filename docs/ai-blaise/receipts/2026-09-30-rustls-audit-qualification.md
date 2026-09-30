# Rustls audit and V2 wording qualification — 2026-09-30

## Qualified source and change boundary

This bounded local qualification applies to commit `d10eeb6f4929a6779146bf6a022106f28c9350cc`, tree `3299d66142e6a5a93a2876d825a8ada07966d425`. Its baseline is `e6efe2aed9a7e413f00e60faadf6dfe0c381ad6e`, tree `3534bb867093a2b7b5aba293649a1ad6a0711ef9`. Adding this receipt and its evidence does not expand the source qualification to an untested implementation.

Exactly three source files changed:

- `operator/Cargo.toml`: raise the compatible Rustls minimum to `0.23.45`, retaining the existing `ring`/`std` features and disabled default features.
- `Cargo.lock`: resolve Rustls `0.23.40 → 0.23.45` and rustls-webpki `0.103.13 → 0.103.15`, changing only their versions and checksums. No other package or dependency list changes.
- `docs/ai-blaise/NEW_FEATURES.md`: remove only “future” from the existing migration caveat so it no longer matches the unchanged V2 stale-wording guard. The distributed-trigger prerequisite and the scoped DDL, propagation, rollback and real least-role requirements remain.

The native decoder, PostgreSQL fixtures, gate scripts, audit policy and toolchain are unchanged. The existing narrowly scoped serde_cbor policy exception remains; no Rustls waiver or ignored advisory was added. Rustls `0.23.45` is a patched compatible release for [GHSA-2mjx-qc3c-rqvc / RUSTSEC-2026-0285](https://github.com/rustls/rustls/security/advisories/GHSA-2mjx-qc3c-rqvc).

## Genuine baseline failures

The unchanged baseline was exercised on 2026-09-30, 09:26:59–09:27:05 UTC. The audit actually exited 1 with `RUSTSEC-2026-0285`; the V2 checker actually exited 1 because the accurate migration caveat contained the broad guard's word “future”. Both failures and both baseline source/tree witnesses are retained. They have not been relabeled successful.

Original hosted failures at the baseline are retained separately:

| Workflow | Run | Failed job | Cause |
| --- | --- | --- | --- |
| Operator | 36646192338 | 109669613189 | Vulnerable locked Rustls rejected by the audit policy |
| Pool | 36646192219 | 109669612773 | Vulnerable locked Rustls rejected by the audit policy |
| Production readiness | 36646192341 | 109669613296 | Vulnerable locked Rustls rejected by the audit policy |
| V2 closure | 36646192248 | 109669613133 | Unchanged stale-wording guard matched the migration caveat |

Each original job log and its original jobs-page metadata is included in the evidence package. These are actual failed predecessors, not expected release-ledger failures, and this local qualification does not change their recorded conclusions. Checks skipped after those failed jobs are not inferred to have passed.

## Repaired-source checks

All commands ran on the exact qualified commit/tree, with clean and identical source snapshots before and after both runs.

| Run | Check | Exit |
| --- | --- | --- |
| Initial | Audit policy | 0 |
| Initial | Dependency-tree inventory | 0 |
| Initial | Formatting | 0 |
| Initial | Whole-workspace, all-target check | 0 |
| Initial | Five-package consumer tests | 0 |
| Initial | Five-package consumer clippy | 0 |
| Initial | Full V2 closure | 0 |
| Initial | Whitespace check | 0 |
| Supplemental | Additional-consumer tests | 0 |
| Supplemental | Additional-consumer clippy | 0 |

The initial run completed at 09:43:23 UTC; the supplemental run completed at 09:50:50 UTC. The audit evaluated 602 locked dependencies. The whole workspace contains 31 member packages, or 32 manifests including the virtual-root manifest. The initial consumer selection covered Operator, Pool, Pool wire, Vectorizer and CDC. The resolved dependency tree also identified `e2e`, `edge_functions` and `realtime`; those received the supplemental tests and clippy run.

| Test run | Rust harness summaries | Passed | Failed | Ignored |
| --- | ---: | ---: | ---: | ---: |
| Initial consumer tests | 13 | 522 | 0 | 1 |
| Supplemental consumers | 9 | 81 | 0 | 0 |
| Combined | 22 | 603 | 0 | 1 |

These are summed Rust harness test executions, not a claim of globally unique test cases. The ignored live PostgreSQL-and-NATS Docker replay remains ignored. These checks do not establish new live broker/TLS, rollout or deployment acceptance.

## Evidence custody and synthetic controls

The [custody manifest](2026-09-30-rustls-audit-evidence/manifest.json) records 38 selected proof files: the 36 originally selected qualification, predecessor and sealer files, plus the synthetic-control module and its result log. It identifies every original and stored gzip copy with byte lengths and SHA-256 hashes. It also records exact baseline and qualified identities, unchanged policy hashes, all eight initial and two supplemental step results, all four repaired-source witness files, and counts parsed from the actual Rust harness summaries.

The nine synthetic unittest methods and their refusal subcases actually passed, with process exit 0. They exercised arbitrary-byte gzip round trips; refusal when inspected bytes differ from later captured bytes; symlink and overwrite refusal; failed or absent harness summaries; equal-but-wrong and mismatched source witnesses; missing, duplicate, reordered, nonzero and malformed step rows; derived mixed-harness counts; and refusal when an inspected file is absent from the captured set. The controls also exercised valid witnesses and step rows. Git calls were blocked in the test process, and the real checkout and qualification proofs were not mutated.

Synthetic fixtures were preserved at `/home/spencer/citus-rustls-repair-sep30-run/seal-custody-tests-nkjlod9h`. The controls are evidence about the sealer, not additional product test executions, and are not added to the 603 passing Rust test executions above.

The sealer verified inspected bytes against captured bytes, rehashed original, stored and decoded files, and rechecked the exact qualified source before writing and reading back the manifest. Original VM evidence remains untouched. Compressed copies preserve the original bytes, including whitespace and line endings.

| Proof | SHA-256 |
| --- | --- |
| Custody manifest | `7bd1755c8d39e808a167d4e9ba6ebba4f8e87e514551adf5ff2df0b2c0492b9c` |
| Sealer source | `43f682afac5ca45000b1e8a4bfccced311e96dd2375683b11eed85e5ef6455eb` |
| Synthetic-control module | `cc2365eb5d1cf89652ff7078ff66adfe41103d05900e3a1113d0e2baf2bf043b` |
| Synthetic-control result log | `c09c75b1ec91d9e36372f3b8d72c70d2df2233d67a47adef882e90068df53a00` |
| Initial qualification log, original decoded bytes | `5d00566ae2f5d058cd1da7fb58c7a034785d15ff568212fdca421f02fb4f65f8` |
| Supplemental log, original decoded bytes | `cd7995ff760e7a1192312d61b6326915e5b87e7fe6fb3a48baac529e2e6c3fb1` |

The final sealer output is retained separately at `/home/spencer/citus-rustls-repair-sep30-run/seal-result.log`, SHA-256 `3c7e7434b32df6ec549ca1cb1ad1223f7af46e022ca4d7785b4e19de9f9ef983`. That output was produced after capture and is not one of the 38 manifest-listed proof files. The manifest is the package's independently rehashable inventory.

Independent root review subsequently verified all 38 stored/decoded files against 551,319 original bytes, exact source witnesses, the 603 derived passing executions, all four actual failed hosted predecessors, unchanged policy hashes and the three-file production boundary. Review helper SHA-256 `814bd810e1d7c523eaee9d4f7f48dbc867a03b70d2ed0c1613c8435e011d8cb9`; terminal review log SHA-256 `b822e63c303b336b29ea8e490e2f81bb2858d0384613138145b4ac2cebf3f930`. Those VM-only review files are external to the original 38-file seal.

## Remaining acceptance boundaries

This closes only the bounded local Rustls dependency repair and V2 wording qualification. It does not establish successful hosted required CI for a subsequent receipt commit, signed/public image release, whole-fork production readiness, resolution of other PostgreSQL 17/18 findings, Chimera promotion, or latest-upstream reconciliation. The distributed-trigger prerequisite remains open under its existing requirements. No CI retry, image dispatch, new resource, live key operation or deployment was performed for this evidence seal.
