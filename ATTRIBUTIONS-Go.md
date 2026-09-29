# ATTRIBUTIONS - Go

Checked on 2026-09-05: `tools/citus-admin/` has no `go.mod` or `go.sum`.
Its checked-in implementation is Rust, so no imported Go dependency graph
is attributed here. Earlier lists of anticipated routers, drivers, and other
modules were design guesses, not resolved dependencies, and have been removed.

## Upstream boundary

WhoDB remains a reference for a possible administration UI integration, not
a vendored Go implementation in this tree. The reviewed upstream revision uses
Apache-2.0; see [WhoDB's license at `95dcf00f2d237296b1758b73d88c0b68459f81de`](https://github.com/clidey/whodb/blob/95dcf00f2d237296b1758b73d88c0b68459f81de/LICENSE).
The earlier claim that this repository would simply relicense a GPL WhoDB
fork to AGPL was unsupported and is withdrawn. Any future import must record
its exact revision and applicable licenses, retain the required notices, and
review its own dependency graph. This observation does not change the license
of any older upstream revision.

## When Go source is imported

Record the resolved modules from `go list -m -json all`, then inspect each
module's actual license files at the resolved version. That command identifies
modules; it does not supply a canonical license field. Add a real Go
dependency-policy check before claiming CI enforcement. The current
`ci/ai-blaise/license-check.sh` only checks that this record exists and scans
Rust metadata; it does not scan Go dependencies.

See [ATTRIBUTIONS-Rust.md](ATTRIBUTIONS-Rust.md) for the existing Rust graph
and [ATTRIBUTIONS-TypeScript.md](ATTRIBUTIONS-TypeScript.md) for the separate
front-end import boundary.
