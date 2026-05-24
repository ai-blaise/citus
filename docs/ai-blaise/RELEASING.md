# Releasing

A release is eligible only after `make -f Makefile.ai-blaise gate-close`,
`make -f Makefile.ai-blaise v2-acceptance-check`, and the GitHub release
acceptance workflows are green for the exact commit.

`gate-close` is a live release gate, not a best-effort local smoke. Its
Docker-backed smoke targets must fail closed if Docker is unavailable; direct
smoke scripts may skip only for exploratory runs outside the release gate.
Rendered Helm checks must also fail closed under the Makefile release path
through `REQUIRE_HELM=1`; missing Helm is not valid release evidence. The
upgrade/rollback compatibility guard must also be green for the exact commit:

```bash
make -f Makefile.ai-blaise upgrade-rollback-guardrails
```

Those checks are release prerequisites, not a waiver for alpha features. A
production release must also pass:

```bash
ci/ai-blaise/production-readiness-check.sh production-release
ci/ai-blaise/production-gap-audit.sh
```

The production-release mode intentionally fails while any release-scope custom
feature is still alpha, contract-only, or model-only without measured evidence.
The production gap audit keeps V2 acceptance models, canonical contract
runners, and smoke-test scaffolding from being misread as production evidence
unless the corresponding feature entry has measured runtime evidence and an
explicit status promotion.

Release artifacts must include:

- source tag
- overlay images
- `artifacts/ai-blaise-image-digests.tsv` from
  `scripts/citus-scale/build-app-images.sh`
- SBOMs
- signed container images
- updated `NEW_FEATURES.md`
- updated `BENCHMARKS.md`
- `images/citus-pg-overlay/extensions/ai_blaise_citus-upgrade-manifest.tsv`
  matching the shipped companion SQL files
- `make -f Makefile.ai-blaise upgrade-rollback-guardrails` output for the
  release commit
- production-readiness audit evidence for every release-scope custom feature
