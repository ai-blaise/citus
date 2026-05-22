# Bundled-extension microbench feature map

This file is the canonical source-side anchor for the 26 `MB<n>` feature IDs
that name the always-on bundled-extension microbenchmarks. The
`features-doc-check.sh` and `production-readiness-check.sh` audits scan the
`scripts/` tree for `FEATURE:` markers; the executable surface for each
feature lives under `benchmarks/microbenches/<ext>/`, and the documentation
register entry lives under `docs/ai-blaise/NEW_FEATURES.md`.

| Feature ID | Extension          | Microbench directory                          |
| ---------- | ------------------ | --------------------------------------------- |
| FEATURE: MB1  | timescaledb       | `benchmarks/microbenches/timescaledb/`        |
| FEATURE: MB2  | citus             | `benchmarks/microbenches/citus/`              |
| FEATURE: MB3  | pgvector          | `benchmarks/microbenches/pgvector/`           |
| FEATURE: MB4  | pg_cron           | `benchmarks/microbenches/pg_cron/`            |
| FEATURE: MB5  | pg_partman        | `benchmarks/microbenches/pg_partman/`         |
| FEATURE: MB6  | pgaudit           | `benchmarks/microbenches/pgaudit/`            |
| FEATURE: MB7  | pgsodium          | `benchmarks/microbenches/pgsodium/`           |
| FEATURE: MB8  | postgresql-hll    | `benchmarks/microbenches/postgresql-hll/`     |
| FEATURE: MB9  | postgresql-topn   | `benchmarks/microbenches/postgresql-topn/`    |
| FEATURE: MB10 | tdigest           | `benchmarks/microbenches/tdigest/`            |
| FEATURE: MB11 | pgnodemx          | `benchmarks/microbenches/pgnodemx/`           |
| FEATURE: MB12 | postgis           | `benchmarks/microbenches/postgis/`            |
| FEATURE: MB13 | pg_search         | `benchmarks/microbenches/pg_search/`          |
| FEATURE: MB14 | pg_graphql        | `benchmarks/microbenches/pg_graphql/`         |
| FEATURE: MB15 | pg_jsonschema     | `benchmarks/microbenches/pg_jsonschema/`      |
| FEATURE: MB16 | age               | `benchmarks/microbenches/age/`                |
| FEATURE: MB17 | plrust            | `benchmarks/microbenches/plrust/`             |
| FEATURE: MB18 | plv8              | `benchmarks/microbenches/plv8/`               |
| FEATURE: MB19 | pg_uuidv7         | `benchmarks/microbenches/pg_uuidv7/`          |
| FEATURE: MB20 | pg_repack         | `benchmarks/microbenches/pg_repack/`          |
| FEATURE: MB21 | pg_failover_slots | `benchmarks/microbenches/pg_failover_slots/`  |
| FEATURE: MB22 | pg_warm           | `benchmarks/microbenches/pg_warm/`            |
| FEATURE: MB23 | pgcrypto          | `benchmarks/microbenches/pgcrypto/`           |
| FEATURE: MB24 | pg_trgm           | `benchmarks/microbenches/pg_trgm/`            |
| FEATURE: MB25 | citext            | `benchmarks/microbenches/citext/`             |
| FEATURE: MB26 | rum               | `benchmarks/microbenches/rum/`                |

Each microbench directory ships `setup.sql`, `bench.sql`, `bench.sh`,
`baseline.json`, and `README.md`. The aggregate runner is
`benchmarks/microbenches/run-all.sh`; the regression gate is
`benchmarks/microbenches/compare-to-baseline.sh`. The PR-time smoke is
`ci/ai-blaise/bundled-ext-microbenches-smoke.sh`; the nightly full-row-count
workflow is `.github/workflows/ci-microbench.yml`.
