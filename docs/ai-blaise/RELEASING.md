# Releasing

A release is eligible only after `make -f Makefile.ai-blaise gate-close`,
`make -f Makefile.ai-blaise v2-acceptance-check`, and the GitHub release
acceptance workflows are green for the exact commit.

Release artifacts must include:

- source tag
- overlay images
- SBOMs
- signed container images
- updated `NEW_FEATURES.md`
- updated `BENCHMARKS.md`
