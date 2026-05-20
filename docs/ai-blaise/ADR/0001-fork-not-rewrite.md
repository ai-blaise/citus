# ADR 0001: Fork Citus, Do Not Rewrite

## Status

Accepted.

## Decision

`ai-blaise/citus` starts as a fork of `citusdata/citus` and layers new
capabilities through overlay directories and a small patch queue.

## Consequences

- Existing Citus behavior remains the compatibility baseline.
- Upstream sync must stay routine and automated.
- New features must avoid unnecessary edits to upstream files.
