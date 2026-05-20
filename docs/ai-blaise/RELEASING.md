# Releasing

A release is eligible only after `make -f Makefile.ai-blaise gate-close`,
`make -f Makefile.ai-blaise v2-acceptance-check`, and the GitHub release
acceptance workflows are green for the exact commit.

Those checks are release prerequisites, not a waiver for alpha features. A
production release must also pass:

```bash
ci/ai-blaise/production-readiness-check.sh production-release
```

The production-release mode intentionally fails while any release-scope custom
feature is still alpha, contract-only, or model-only without measured evidence.

Release artifacts must include:

- source tag
- overlay images
- SBOMs
- signed container images
- updated `NEW_FEATURES.md`
- updated `BENCHMARKS.md`
- production-readiness audit evidence for every release-scope custom feature
