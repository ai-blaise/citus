# Bundled Extensions

The canonical operand-image extension contract lives in
`images/citus-pg-overlay/extension-manifest.tsv`. This is a manifest/init
contract, not production evidence that every binary package is installed in a
runnable operand image. `FEATURE: Bundle1` remains alpha until a real operand
image build smoke verifies the required extension control files and initdb
extension creation end to end.

## Required Bundle

The required bundle records the intended extension set for ai-blaise/citus
Postgres operand images. It covers the V2 plan's mandatory Citus, TimescaleDB,
vector, search, graph, JSON Schema, observability, security, geo, and
online-maintenance substrates.

Required entries are statically validated by `ci/ai-blaise/image-check.sh`.
Cluster initialization uses
`images/citus-pg-overlay/initdb.d/00-ai-blaise-extensions.sql` as the
deterministic extension creation order and intentionally fails when a required
extension control file is absent.

The overlay also installs `ai_blaise_citus`, a local SQL fallback companion
extension. It exposes `companion_feature_status()` plus pgrx-compatible
Timescale-on-Citus plan helpers, including distributed hypertable and
time-range shard-pruner plans, in the operand image even before the compiled
pgrx companion library is loaded, so smoke tests and operators have a stable
extension name to target.

## Optional Bundle

Optional entries are chart- or image-build flags. They are kept in the same
manifest so licensing, packaging, and hard-block rules remain reviewable in one
place.

## Hard Blocks

Hard-block entries are not bundled because they replace heap access methods,
install conflicting planner or transaction hooks, or compete with Citus shard
management. Adding a blocked extension to the required or optional bundle must
first change the manifest and explain the conflict resolution in an ADR.
