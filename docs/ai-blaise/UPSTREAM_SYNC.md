# Upstream Sync

The fork checks patch applicability against `citusdata/citus` every 14 days and
on every V2 acceptance run. The default upstream target is `release-14.0`, which
matches the V2 upstream-merge gate; override it with `UPSTREAM_REF` only for
explicit backport or forward-port drills.

The sync job intentionally starts as a dry-run gate. Once the first overlay
release stabilizes, it should be extended to open a PR on
`chore/upstream-sync-YYYY-MM-DD` with:

- upstream Citus changes
- refreshed patch applicability output
- any patch rebases needed to keep `patches/series` clean
