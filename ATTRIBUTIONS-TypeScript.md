# ATTRIBUTIONS - TypeScript

Checked on 2026-09-05: neither `tools/citus-schema-designer/` nor
`tools/citus-admin/` contains a `package.json` or npm/pnpm lockfile. Their
checked-in implementations are Rust. No imported npm dependency graph is
attributed here; the former lists of anticipated front-end packages were
not dependency evidence and have been removed.

## Upstream boundary

DrawDB and WhoDB remain possible UI integration references, not vendored
front-end implementations in those directories. Do not assume their current
licenses apply to a future or historical import. For the reviewed WhoDB
revision, [WhoDB's license at `95dcf00f2d237296b1758b73d88c0b68459f81de`](https://github.com/clidey/whodb/blob/95dcf00f2d237296b1758b73d88c0b68459f81de/LICENSE) is Apache-2.0.
The prior assertion that a GPL WhoDB frontend could simply be relabeled
AGPL by importing it into this repository was unsupported and is withdrawn.

Before importing either project, record the immutable revision, review all
applicable license and notice files, retain required attribution, and resolve
the exact dependency graph. No upstream license texts have been installed in
these directories merely by writing this record.

## When front-end source is imported

Generate the inventory from the committed lockfile and inspect the resolved
packages' license files, including packages that use `license-file` or
custom notices. Declared ranges in `package.json` do not identify a complete
transitive graph. Add and test npm dependency-policy enforcement before
claiming it exists. The current `ci/ai-blaise/license-check.sh` checks this
record's presence and Rust metadata, not TypeScript dependencies.

The current tools are included in [ATTRIBUTIONS-Rust.md](ATTRIBUTIONS-Rust.md).
See [ATTRIBUTIONS-Go.md](ATTRIBUTIONS-Go.md) for the separate backend boundary.
