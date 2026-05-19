# Extension Cohabitation

Citus normally requires its hook registrations to be first in
`shared_preload_libraries`. The ai-blaise fork carries
`patches/0001-allow-trusted-hook-coextensions.patch` to make this guard
explicitly configurable for validated co-extensions.

## Configuration

```conf
shared_preload_libraries = 'timescaledb,citus'
citus.cohabit_extensions = 'timescaledb'
```

`citus.cohabit_extensions` is a postmaster-level allowlist. It should contain
only extensions whose hook chain with Citus has been tested in this fork's
cohabitation suite.

The initial required cohabitation target is TimescaleDB. The bridge shape is a
PostgreSQL declarative-partitioned parent distributed by Citus with Timescale
hypertable partitions on workers.
