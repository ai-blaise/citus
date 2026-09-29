# Script wiring audit

Audit date: 2026-09-05, before the feature-gate migration in the same working tree.

This is a source-inventory audit, not an acceptance receipt. It records the
observed wiring of top-level `ci/ai-blaise/*.sh` and `ci/ai-blaise/*.py` files
in the working tree, including untracked files that are not ignored. It does
not claim that a referenced script ran, passed, used release-built artifacts,
or produced promotion evidence. Nested SQL fixtures, benchmark scripts outside
`ci/ai-blaise`, ignored files, and generated artifacts are outside this count.

The historical plan called for restoring a script-wiring guard and adopting or
deleting 46 dead scripts. That number is not reproduced by the current tree.
No implementation of that guard, and no hard-coded 46-script set, is present in
the repository or its reachable Git history. Reintroducing the number would be
a stale constant rather than a source-derived invariant. The B3 migration
remains incomplete until every current execution-unwired script below is
adopted, rewritten, or deliberately retired.

## Current inventory

The recorded snapshot contains 136 top-level shell or Python scripts. Later
gate-migration regression tests are wired in their corresponding workflows;
rerun the method below for the current file set rather than treating this
snapshot's counts or content digest as a freshness claim.

| Category | Count | Meaning |
| --- | ---: | --- |
| Referenced literally by `Makefile.ai-blaise` | 118 | The path appears in the current Make surface. |
| Referenced literally by a workflow or local action | 84 | The path appears under `.github/workflows` or `.github/actions`. |
| Referenced by both Make and GitHub automation | 79 | Intersection of the preceding two sets. |
| Make only | 39 | Locally invocable through Make but absent from GitHub automation. |
| Workflow/action only | 5 | Automated but without a Make entry point. |
| No direct Make or workflow/action reference | 13 | Requires invocation-graph review; a prose or static read is not execution. |
| Verified indirect runner | 1 | `live-k8s-e2e.sh` is invoked by rooted wrapper scripts. |
| Execution-unwired | 12 | No verified path from a current Make/workflow execution root. |

For the observed 136 paths, the length-framed sorted path digest is
`2e36160c6a9b41db607f38c3c323586c4a4f7f381d232395b53ddf9a63664520`.
The corresponding length-framed path-and-file-bytes digest is
`4fd009727e55d474b71e14785a98d34cf3c29c62ce67a9d42251823b4ebd4ec7`.
These are audit coordinates for this dirty working-tree observation, not
signed source or release provenance.

The one indirect runner is
`ci/ai-blaise/live-k8s-e2e.sh`. It is actually invoked by
`ci/ai-blaise/deploy-check.sh`, `ci/ai-blaise/kind-production-smoke.sh`, and
`ci/ai-blaise/live-k8s-e2e-contract-smoke.sh`. It is therefore a required
runner entry point, not an orphan. Comments in
`k8s-production-values-live-smoke.sh` and static reads in
`production-gap-audit.sh` are not needed to establish that result.

Four execution-unwired files have literal references from another source file,
but none of those references executes the file:

- `f3-iceberg-federation-live-smoke.sh` is assigned to a variable and checked
  for executable presence by the also-unwired L1 composite script.
- `operator-branch-lifecycle-live-smoke.sh`,
  `operator-multiregion-contracts-smoke.sh`, and
  `postgrest-live-data-plane-smoke.sh` are read as text by
  `production-gap-audit.sh` to check source phrases.

Those static dependencies may be legitimate guard inputs, but they do not make
the live or canonical programs run. Documentation and `docs/features.tsv`
references likewise describe intent; they are never invocation edges.

## Execution-unwired decisions

| Path | Actual current role | Required next decision |
| --- | --- | --- |
| `ci/ai-blaise/bootstrap-v2-merge-plan-check.py` | Offline and live validation of the now-historical bootstrap-v2 merge-plan metadata. Its current callers are documentation commands only. | Either wire deterministic `--offline` validation into the upstream-sync surface and retain `--live` as an explicit credentialed/manual operation, or retire it when the latest-upstream selection workflow supersedes the old merge plan. Do not make the closed historical plan a runtime acceptance gate. |
| `ci/ai-blaise/cnpg-citus-chart-live-smoke.sh` | New B2 diagnostic that consumes an independently frozen Command Center chart archive, exact Rust operator and operand images, a pinned CNPG manifest/image, and kind to exercise one coordinator plus two logical workers and the chart-owned SQL test. The external chart is not part of the Citus/Chimera three-source port map. | Keep unwired until its first exact-input native run and review complete. Then adopt as an explicit, pinned, manual or scheduled cross-repository diagnostic. A reused development VM result remains non-promotion evidence. |
| `ci/ai-blaise/f3-iceberg-federation-live-smoke.sh` | Real Docker-backed Apache Iceberg REST catalog round trip. L1 currently checks only that this file exists; it does not execute it. The script currently uses `tabulario/iceberg-rest:latest`. | Pin and verify the catalog image, use isolated ports and complete container/volume cleanup, then add an explicit Make and suitable scheduled sidecar workflow entry or retire the claim. Do not wire the floating-image form into a release gate. |
| `ci/ai-blaise/l1-pg-lake-analytical-live-smoke.sh` | Runs an analytical canonical model, checks for a source label, then treats the existence of L3/L5/F3 scripts and prose markers as composite evidence. It does not execute those three live paths. | Rewrite it to consume source-bound successful constituent receipts or execute the required smokes, or retire/reclassify it as a model/evidence-only helper. The current file must not be adopted as a live acceptance proof. |
| `ci/ai-blaise/l13-motherduck-connector-live-smoke.sh` | Exercises local canonical binding and accounting and confirms fail-closed behavior without a MotherDuck token. It performs no MotherDuck cloud session. | Rename or reclassify it as a contract/model smoke, then wire that bounded deterministic check into the analytical sidecar job if still useful. Keep real MotherDuck execution as a separately credentialed acceptance requirement. |
| `ci/ai-blaise/mr9-regional-failover-live-smoke.sh` | Docker simulation of base-backup, loss of one PostgreSQL container, and recovery into another. Defaults include floating `postgres:17-bookworm` and `busybox` images. | Pin all images and source identity, isolate scratch/ports, prove complete volume cleanup, and then schedule the bounded drill explicitly; otherwise retire the production-evidence claim. It is not proof of geographic or Kubernetes failover. |
| `ci/ai-blaise/operator-branch-lifecycle-live-smoke.sh` | Heavy kind/CSI snapshot, restore, suspend, and service-cutover diagnostic. `production-gap-audit.sh` reads its text but never runs it. It currently uses a tagged kind node and a live Git clone of the CSI driver branch. | Lock and verify every cluster/CSI input and add scoped diagnostics/cleanup before adopting it as a scheduled or manual operator job. Keep cloud-provider and multi-zone claims separate. |
| `ci/ai-blaise/operator-multiregion-contracts-smoke.sh` | Deterministic Rust operator unit tests plus canonical multi-region contract output; no external live infrastructure. `production-gap-audit.sh` only reads the file. | Add a direct Make target and execute it in the operator workflow. This is a fast contract check, not regional-failover evidence. |
| `ci/ai-blaise/postgrest-live-data-plane-smoke.sh` | Real loopback PostgreSQL, PostgREST, supervisor, and sidecar data-plane exercise. The audit reads its source but does not execute it. Defaults rely on a local database-image tag and a PostgREST version tag rather than exact verified image identities. | Bind both images and source bytes, preserve secret-safe artifacts and cleanup, then add an explicit Make/live sidecar workflow job. A one-host diagnostic remains distinct from HA or promotion evidence. |
| `ci/ai-blaise/s7-pgactive-active-active-live-smoke.sh` | Single-node pgactive runtime exercise using a prebuilt local `pgactive-pg17:test` tag. It explicitly does not perform the multi-host `pgactive_init_copy` bootstrap. | Define and verify the image build/provenance contract before scheduled adoption. Keep the missing multi-host bootstrap as a separate red requirement; do not promote it from this single-node result. |
| `ci/ai-blaise/t6-pg18-io-uring-live-smoke.sh` | Linux-kernel-dependent PostgreSQL 18 `io_uring` exercise. It defaults to floating `postgres:18-bookworm` and installs available extensions at runtime. | Pin the base and package inputs, require a capability-qualified Linux runner, and wire it only to an explicit scheduled/manual job. Do not make unsupported generic runners silently skip the requirement. |
| `ci/ai-blaise/topology-consensus-smoke.sh` | Deterministic Raft/HLC/operator canonical assertions and focused Rust tests for S4/S5/S9/MR6. | Add a direct Make target and an appropriate operator/e2e-model workflow invocation. Preserve its bounded contract status; it is not a live distributed consensus or failure-recovery receipt. |

The first safe wiring candidates are the deterministic
`operator-multiregion-contracts-smoke.sh` and
`topology-consensus-smoke.sh`. The L13 check can follow only after its name and
scope stop implying external live execution. The remaining live programs need
their stated input/provenance and cleanup repairs before automation adopts
them. L1 and the historical merge-plan checker require an explicit keep,
rewrite, or retirement decision rather than automatic wiring.

## Reproduce the inventory

Run from the repository root. This script derives the file set and literal
Make/workflow references from current bytes; it does not guess execution from
documentation:

```sh
python3 - <<'PY'
import hashlib
import pathlib
import subprocess

root = pathlib.Path(".").resolve()
listed = subprocess.check_output(
    ["git", "ls-files", "-co", "--exclude-standard", "-z"]
)
paths = sorted(pathlib.Path(raw.decode()) for raw in listed.split(b"\0") if raw)
scripts = [
    path for path in paths
    if path.parent == pathlib.Path("ci/ai-blaise")
    and path.suffix in {".sh", ".py"}
]
make_text = (root / "Makefile.ai-blaise").read_text(encoding="utf-8")
automation_text = "\n".join(
    (root / path).read_text(encoding="utf-8")
    for path in paths
    if path.as_posix().startswith((".github/workflows/", ".github/actions/"))
    and (root / path).is_file()
)
make_refs = {path for path in scripts if path.as_posix() in make_text}
automation_refs = {path for path in scripts if path.as_posix() in automation_text}

path_digest = hashlib.sha256()
content_digest = hashlib.sha256()
for path in scripts:
    encoded = path.as_posix().encode()
    payload = (root / path).read_bytes()
    path_digest.update(len(encoded).to_bytes(8, "big"))
    path_digest.update(encoded)
    content_digest.update(len(encoded).to_bytes(8, "big"))
    content_digest.update(encoded)
    content_digest.update(len(payload).to_bytes(8, "big"))
    content_digest.update(payload)

print(f"scripts={len(scripts)}")
print(
    f"make={len(make_refs)} workflow_or_action={len(automation_refs)} "
    f"both={len(make_refs & automation_refs)} "
    f"make_only={len(make_refs - automation_refs)} "
    f"workflow_only={len(automation_refs - make_refs)}"
)
print(f"path_digest={path_digest.hexdigest()}")
print(f"path_content_digest={content_digest.hexdigest()}")
print("no_direct_make_or_workflow_reference:")
for path in sorted(set(scripts) - make_refs - automation_refs):
    print(path)
PY
```

For each path printed in the final section, inspect every literal call site:

```sh
while IFS= read -r script; do
  printf '\n%s\n' "$script"
  rg -nF "$script" Makefile.ai-blaise .github ci/ai-blaise || true
  rg -nF "${script##*/}" ci/ai-blaise || true
done < <(
  python3 - <<'PY'
import pathlib
import subprocess

root = pathlib.Path(".")
listed = subprocess.check_output(
    ["git", "ls-files", "-co", "--exclude-standard", "-z"]
)
paths = sorted(pathlib.Path(raw.decode()) for raw in listed.split(b"\0") if raw)
scripts = [
    path for path in paths
    if path.parent == pathlib.Path("ci/ai-blaise")
    and path.suffix in {".sh", ".py"}
]
make_text = (root / "Makefile.ai-blaise").read_text(encoding="utf-8")
automation_text = "\n".join(
    path.read_text(encoding="utf-8")
    for path in paths
    if path.as_posix().startswith((".github/workflows/", ".github/actions/"))
    and path.is_file()
)
for path in scripts:
    if path.as_posix() not in make_text and path.as_posix() not in automation_text:
        print(path)
PY
)
```

Classify an edge as execution only when the call site actually invokes or
sources the target (for example, `bash`, `exec bash`, `python3`, `source`, or
`.`). Assigning a path, checking existence, reading bytes in a validator,
mentioning it in an error string, or citing it in documentation is a literal
reference but not execution. Any future automated guard must preserve this
distinction and derive its expected set from current source rather than embed a
numeric target.
