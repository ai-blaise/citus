# Releasing

A release is eligible only after `make -f Makefile.ai-blaise gate-close`,
`make -f Makefile.ai-blaise v2-acceptance-check`,
`make -f Makefile.ai-blaise release-gate-monitor`, and the GitHub release
acceptance workflows are green for the exact commit.

`gate-close` is a live release gate, not a best-effort local smoke. Its
Docker-backed smoke targets must fail closed if Docker is unavailable; direct
smoke scripts may skip only for exploratory runs outside the release gate.
Rendered Helm checks must also fail closed under the Makefile release path
through `REQUIRE_HELM=1`; missing Helm is not valid release evidence.

Those checks are release prerequisites, not a waiver for alpha features. A
production release must also pass:

```bash
ci/ai-blaise/production-readiness-check.sh production-release
ci/ai-blaise/production-gap-audit.sh
ci/ai-blaise/docs-evidence-boundary-check.sh
```

The production-release mode intentionally fails while any release-scope custom
feature is still alpha, contract-only, or model-only without measured evidence.
The production gap audit keeps V2 acceptance models, canonical contract
runners, and smoke-test scaffolding from being misread as production evidence
unless the corresponding feature entry has measured runtime evidence and an
explicit status promotion. The release gate monitor adds parallel matrix
monitoring for PR checks while preserving that local evidence boundary.

Release artifacts must include:

- source tag
- overlay images
- `artifacts/ai-blaise-image-digests.tsv` from
  `scripts/citus-scale/build-app-images.sh`
- SBOMs
- signed container images
- updated `NEW_FEATURES.md`
- updated `BENCHMARKS.md`
- production-readiness audit evidence for every release-scope custom feature
