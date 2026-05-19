# Upstream Sync

The fork checks patch applicability against `citusdata/citus` every 14 days.

The sync job intentionally starts as a dry-run gate. Once the first overlay
release stabilizes, it should be extended to open a PR on
`chore/upstream-sync-YYYY-MM-DD` with:

- upstream Citus changes
- refreshed patch applicability output
- any patch rebases needed to keep `patches/series` clean
