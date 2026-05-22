# Releasing

A release is eligible only after `make -f Makefile.ai-blaise gate-close`,
`make -f Makefile.ai-blaise v2-acceptance-check`, and the GitHub release
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
- production-readiness audit evidence for every release-scope custom feature
- a refreshed `benchmarks/baselines/<date>-baseline.json` recorded against the
  release SHA (Gates 10 and 11 read this file via
  `e2e/src/release_gates.rs::PERFORMANCE_BASELINE_PATH`)

## Documentation site

The published docs at `https://ai-blaise.github.io/citus/` are built by
mkdocs Material from `docs/` and `mkdocs.yml`. The CI flow is:

- `.github/workflows/ci-docs-build.yml` runs `mkdocs build --strict` on every
  PR that touches `docs/**` or `mkdocs.yml` and uploads the rendered site as
  an artifact.
- `.github/workflows/ci-docs-publish.yml` runs on push to `main` and on
  manual dispatch; it uses `mike` to publish a versioned subtree to the
  `gh-pages` branch and updates the `latest` alias.

### One-time GitHub Pages setup (manual)

GitHub Pages source must be set to the `gh-pages` branch in repo settings.
Until that switch is flipped, the `ci-docs-publish` workflow pushes the
rendered site to `gh-pages` but the site at `ai-blaise.github.io/citus`
returns a 404.

1. Visit `Settings -> Pages` on `https://github.com/ai-blaise/citus`.
2. Set "Build and deployment" -> Source = "Deploy from a branch".
3. Set Branch = `gh-pages`, folder = `/ (root)`.
4. Save.
5. Verify `https://ai-blaise.github.io/citus/` resolves within a few minutes.

The first push to `gh-pages` happens automatically when `main` next changes
under `docs/` or `mkdocs.yml`. To force an initial publish, run the
`docs-publish` workflow manually via the Actions tab (`workflow_dispatch`).
