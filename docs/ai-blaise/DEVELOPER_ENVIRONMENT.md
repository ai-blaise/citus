# Developer and Release Environment

The ai-blaise overlay has two intentionally different execution modes.

Local exploratory mode keeps fast contract checks runnable on a stripped-down
VM. Missing live dependencies may produce labeled scaffold results, but those
results are not production evidence.

Release mode fails closed before expensive work starts. Run the preflight first:

```bash
make -f Makefile.ai-blaise preflight-release
```

`gate-close` runs the same release preflight and exports
`AI_BLAISE_RELEASE_MODE=1` plus `BENCH_REQUIRE_MEASURED=1` to its prerequisites.
That makes missing tools, empty Make targets, and scaffold benchmark results
hard failures instead of quiet skips.

## Preflight Targets

| Target | Purpose |
| --- | --- |
| `make -f Makefile.ai-blaise preflight-local` | Reports missing release-only tools without failing. Use this before exploratory local work. |
| `make -f Makefile.ai-blaise preflight-style` | Fails when Python/C style tools needed by `check-style` are missing. |
| `make -f Makefile.ai-blaise preflight-release` | Fails when release-mode tools, evidence scripts, Docker daemon access, or Make target wiring are missing. |

The release preflight checks these command-line tools because the Make targets
and scripts call them directly: `cargo`, `docker`, `helm`, `kubectl`, `jq`,
`psql`, `rustfmt`, `black`, and `shellcheck`. It also checks `python3`, `make`,
`git`, and `kind` because the release gate uses Python validators, recursive
Make orchestration, upstream sync checks, and a kind production smoke.

## Make Target Rules

Release evidence targets must not be declared `.PHONY` without a recipe. The
preflight audits `Makefile.ai-blaise` for that condition because GNU Make treats
an empty phony target as already complete. If a future evidence script is not
ready yet, keep the Make recipe and let the preflight fail with the exact missing
artifact path rather than silently passing.

The release gate currently routes these live evidence targets through preflight
wrappers:

- `deploy-check` requires `ci/ai-blaise/deploy-check.sh` and Helm.
- `kind-production-smoke` requires `ci/ai-blaise/kind-production-smoke.sh`,
  Docker, kind, kubectl, Helm, and psql.

## Benchmark Results

Exploratory benchmark scripts may write JSON with either `mode=scaffold` or a
`note` containing `scaffold-only`. That is acceptable for local wiring checks
and CI smoke coverage on machines without a live Postgres or Kubernetes target.

Release mode rejects those scaffold records through
`ci/ai-blaise/env-preflight.sh assert-measured-results`. A release benchmark run
must produce measured JSON for every required harness result; otherwise the gate
fails and reports the exact scaffold file and note.

## License Metadata

`ci/ai-blaise/license-check.sh` can still run locally without `cargo` or `jq`,
but it prints that the SPDX metadata scan was skipped. In release mode those
tools are required so GPL dependency checks cannot be bypassed by PATH drift.

## Adding New Scripts

When adding a new release or production-readiness script:

1. Add an explicit Make recipe, not only a `.PHONY` entry.
2. Add the tool to `ci/ai-blaise/env-preflight.sh` if the script invokes it.
3. Decide whether missing infrastructure is exploratory scaffold output or a
   release-mode failure, then encode that distinction with
   `AI_BLAISE_RELEASE_MODE` or `BENCH_REQUIRE_MEASURED`.
4. Ensure any local scaffold output includes `mode=scaffold` or a
   `scaffold-only` note so it cannot be mistaken for production evidence.
