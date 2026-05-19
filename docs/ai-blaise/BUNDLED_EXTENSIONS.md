# Bundled Extensions

The canonical operand-image extension contract lives in
`images/citus-pg-overlay/extension-manifest.tsv`.

## Required Bundle

The required bundle is installed for every ai-blaise/citus Postgres operand
image. It covers the V2 plan's mandatory Citus, TimescaleDB, vector, search,
graph, JSON Schema, observability, security, geo, and online-maintenance
substrates.

Required entries are validated by `ci/ai-blaise/image-check.sh`. Cluster
initialization uses `images/citus-pg-overlay/initdb.d/00-ai-blaise-extensions.sql`
as the deterministic extension creation order.

## Optional Bundle

Optional entries are chart- or image-build flags. They are kept in the same
manifest so licensing, packaging, and hard-block rules remain reviewable in one
place.

## Hard Blocks

Hard-block entries are not bundled because they replace heap access methods,
install conflicting planner or transaction hooks, or compete with Citus shard
management. Adding a blocked extension to the required or optional bundle must
first change the manifest and explain the conflict resolution in an ADR.
