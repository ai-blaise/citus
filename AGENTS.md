# ai-blaise/citus Agent Notes

This fork follows the `scaleable_database_infra` project rules stored in the
agentmemory instance on `127.0.0.1:3911`.

## Working Rules

- Treat every change as production code.
- Keep upstream Citus source untouched in the working tree unless a change is
  represented as a small, upstream-targetable patch under `patches/`.
- Prefer overlay directories for ai-blaise functionality: `companion/`,
  `sidecar/`, `pool/`, `operator/`, `tools/`, `deploy/k8s/`, and
  `docs/ai-blaise/`.
- Every feature-bearing change must update
  `docs/ai-blaise/NEW_FEATURES.md` and include a stable `FEATURE:` marker in
  the primary source file when source exists.
- Use Rule 10 for each work unit: implement, test and verify, iterate, write
  docs, clean up, then commit and push.
