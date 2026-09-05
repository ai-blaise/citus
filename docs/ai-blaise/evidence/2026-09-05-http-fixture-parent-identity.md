# HTTP fixture parent identity correction — 2026-09-05

This is a test-fixture correction, not a production image or feature-readiness
promotion. The four-path candidate starts at Citus
`43e7b309ff3fdfdc82bf6d18107490393b125ce0`; its source tree is
`b940a99696b43b0cec5f800627a4d50bfc00abf1`. This receipt is additional metadata.

## Defect and correction

The canonical [image-contract job](https://github.com/ai-blaise/citus/actions/runs/33993073146/job/101378646607)
passed the image contracts and real-Citus SQL smoke on PG16/17/18, then failed
before the A10/A11 fixture boot. The wrapper assumed that Docker's local image
ID could be appended to a repository name as a registry manifest digest.
That is not portable across Docker image stores.

The wrapper now uses only the content-derived tag returned by the shared fixture
builder. It verifies the tag's exact image ID before and after use and requires
the child rootfs to extend the immutable parent's complete layer prefix.
Both cached and new images use the same label/ancestry verifier. The wrapper
does not accept an externally supplied or unchecked parent tag.

| Candidate path | SHA-256 |
| --- | --- |
| `ci/ai-blaise/build-real-citus-http-test-fixture.sh` | `4fd96ab78fd1907a12a0ffcb6c3b3a6d4815a7da9965b675cb90177f832b76ad` |
| `ci/ai-blaise/real-citus-test-fixture-contract.py` | `236614ce14d7a39946729a5133427fb6c8fd8e67baf45543db4ee649c17a7c16` |
| `ci/ai-blaise/real-citus-test-fixture-contract_test.py` | `379a32d5e183f68fbb1825c7e295e37b8e5c84e251eb92167f05c44b5afe1639` |
| `images/citus-test-fixture/Dockerfile.http` | `d08a556a81538a6b910a93230d0def4a4391510ad01f00ae70f3f359be5068e6` |

The 55 contract tests passed locally and on the second authorized VM. Negative
controls reject removed prechecks, a bare image-ID parent substitution, parent
tag drift, and wrong rootfs ancestry. Root independently reran all 55 tests,
shell syntax, the image contracts and whitespace checks on the isolated candidate.

## Native verification and limits

The existing second VM used
`/home/spencer/citus-ci-http-fix-43e7-20260905`. A new wrapper built from the
previously verified parent
`sha256:3a0d79cc4048ba86d146c4db7a0e3446209babec8c073c0829ee69ab9a1c98cf`
between 21:45:08 and 21:45:18 UTC. The resulting child was
`sha256:5ebeab322feb547c90467f07001f2d3834f814ab6ec3a039b8f1a659e6ed3bbe`,
with 23 layers extending the parent's exact 22-layer prefix. PostgreSQL remained
17.11 and the HTTP package was pinned to `1.7.2-2.pgdg12+1`.

The subsequent cached-wrapper A10/A11 smoke passed at 21:45:30 UTC: five chat
chunks, five executed SQL rows, and its validated execution boundary. The
provider was a local mock, not an external model endpoint. No GCP resource was
created.

| Native artifact | SHA-256 |
| --- | --- |
| `http-builder.log` | `c555fa6829b254bbf3a281cee37954d81a1afee44993cdcd28e4cef272cfccda` |
| `a10-a11-live.log` | `1d41f3a4bc6473d337ead69bcc4f7def5caffc2c7136dbf0e47a77f4d59f4a65` |
| `a10-a11-ai-sql-evidence.tsv` | `6e6ea5a0228b58369a904291ce159cf972e5d298d7c3e4b2630a0f09cdc0f042` |
| `image-check.log` | `2cbdfea82f9d72af3145541025dd3de5226ccbaacf68e274ee2ce14afa10a6d8` |

The TSV reports base HEAD 43e7b309, not the four dirty source edits; the path
hashes above bind those edits separately. The immutable parent retains its
historical e106 dirty-source provenance and selected content identity
`c072dfa5d75daf93a64b81ee01ec2e043078f749dceefd69e7dcaf0bc45a260d`.
This verifies the wrapper against that parent, not a fresh rebuild of the
canonical Citus commit, a full-image boot, or a production release.
