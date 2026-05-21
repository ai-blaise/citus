# benchmarks

> Production boundary: unless a feature is explicitly `Status: production-ready`
> in `docs/ai-blaise/NEW_FEATURES.md`, the surfaces listed here are alpha
> contracts. Deterministic canonical reports and local runtime models are CI
> artifacts, not production evidence; promotion requires live VM/container or
> Kubernetes evidence recorded in `docs/ai-blaise/PRODUCTION_READINESS_AUDIT.md`
> and guarded by `ci/ai-blaise/production-gap-audit.sh`.

Benchmark target directory for TPC-C, sysbench, Timescale ingest, search,
vectorizer, HTAP, and failover gates. Until measured harness output is added
and recorded in the production-readiness audit, this directory is planning
scaffolding rather than benchmark evidence.
