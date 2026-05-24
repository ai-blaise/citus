# Releasing

A release is eligible only after
`make -f Makefile.ai-blaise preflight-release`,
`make -f Makefile.ai-blaise gate-close`,
`make -f Makefile.ai-blaise v2-acceptance-check`, and the GitHub release
acceptance workflows are green for the exact commit. See
[`DEVELOPER_ENVIRONMENT.md`](DEVELOPER_ENVIRONMENT.md) for the local versus
release preflight boundary.

`gate-close` is a live release gate, not a best-effort local smoke. Its
Docker-backed smoke targets must fail closed if Docker is unavailable; direct
smoke scripts may skip only for exploratory runs outside the release gate.
Rendered Helm checks must also fail closed under the Makefile release path
through `REQUIRE_HELM=1`; missing Helm is not valid release evidence.

Benchmark scaffolds are also local-only. Under `gate-close`, scaffold JSON
records (`mode=scaffold` or `scaffold-only` notes) fail the release path so a
missing `psql`, Postgres endpoint, or benchmark driver cannot masquerade as
measured performance evidence.

Those checks are release prerequisites, not a waiver for alpha features. A
production release must also pass:

```bash
ci/ai-blaise/production-readiness-check.sh production-release
ci/ai-blaise/production-gap-audit.sh
ci/ai-blaise/docs-evidence-boundary-check.sh
PERF_EVIDENCE_MODE=release BENCH_RESULT_TAG=release \
  make -f Makefile.ai-blaise performance-evidence-release-check
```

The production-release mode intentionally fails while any release-scope custom
feature is still alpha, contract-only, or model-only without measured evidence.
The production gap audit keeps V2 acceptance models, canonical contract
runners, and smoke-test scaffolding from being misread as production evidence
unless the corresponding feature entry has measured runtime evidence and an
explicit status promotion.

Before publishing images, run the lightweight release-operator packaging gate:

```bash
make -f Makefile.ai-blaise release-publishability-check
```

That gate does not run the full upstream Citus matrix. It verifies the custom
Rust app image matrix, rejects mutable release tags, requires explicit registry
and tag inputs for pushes, and validates the generated digest manifest when one
is present. For the final release candidate, make the manifest mandatory:

```bash
REQUIRE_PUBLISHED_DIGESTS=1 \
  RELEASE_DIGEST_MANIFEST=artifacts/ai-blaise-image-digests.tsv \
  ci/ai-blaise/release-publishability-check.sh
```

Publish the app image matrix only with explicit source and image identity:

```bash
IMAGE_REGISTRY=ghcr.io/ai-blaise \
  TAG="${RELEASE_TAG}" \
  SOURCE_REVISION="$(git rev-parse --verify HEAD)" \
  DIGEST_FILE=artifacts/ai-blaise-image-digests.tsv \
  PUSH=true \
  scripts/citus-scale/build-app-images.sh
```

`scripts/citus-scale/build-app-images.sh` fails a push that omits
`IMAGE_REGISTRY` or `TAG`, uses a mutable tag such as `latest`, or cannot record
an immutable `sha256:` digest for every pushed image. The generated manifest is
the command-center image handoff: it contains `source_revision`, repository,
full image tag, immutable digest, package, binary, and push status for every app
image consumed by the command-center chart.

Release artifacts must include:

- source tag
- overlay images
- `artifacts/ai-blaise-image-digests.tsv` from the explicit publish command
  above, verified with `REQUIRE_PUBLISHED_DIGESTS=1`
- SBOMs
- signed container images
- updated `NEW_FEATURES.md`
- updated `BENCHMARKS.md`
- benchmark JSON artifacts validated against `benchmarks/performance-evidence-thresholds.json`
- production-readiness audit evidence for every release-scope custom feature
