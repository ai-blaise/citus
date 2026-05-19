# Releasing

A release is eligible only after `make -f Makefile.ai-blaise gate-close` and
the release acceptance gates are green.

Release artifacts must include:

- source tag
- overlay images
- SBOMs
- signed container images
- updated `NEW_FEATURES.md`
- updated `BENCHMARKS.md`
