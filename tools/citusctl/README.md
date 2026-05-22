# citusctl

> Production boundary: unless a feature is explicitly `Status: production-ready`
> in `docs/ai-blaise/NEW_FEATURES.md`, the surfaces listed here are alpha
> contracts. Deterministic canonical reports and local runtime models are CI
> artifacts, not production evidence; promotion requires live VM/container or
> Kubernetes evidence recorded in `docs/ai-blaise/PRODUCTION_READINESS_AUDIT.md`
> and guarded by `ci/ai-blaise/production-gap-audit.sh`.

Rust CLI for local development, deploys, inspection, backups, migrations,
tenants, webhooks, functions, and search.

Current implemented surface:

- `CitusCtlRequest`
- `ExecutionIntent`
- `CitusCtlCommand`
- `CitusCtlPlan`
- `CitusCtlCanonicalReport`
- `canonical_citusctl_report`
- `parse_request`
- `v2_command_catalog`

These contracts cover `FEATURE: D1`, `FEATURE: D2`, `FEATURE: M8`,
`FEATURE: B3`, `FEATURE: B5`, and `FEATURE: WF2`. The canonical runner
includes `dev up`, plan-gated `apply`, `inspect cluster`, UTC `time-travel`,
and WAL replay validation.

`FEATURE: D2` is production-ready only for the real CLI plan-id guard:
`ci/ai-blaise/citusctl-smoke.sh` requires `citusctl apply` without a plan ID
to fail before any apply-mode command can proceed. The broader command
surfaces listed above remain alpha unless their feature entries say otherwise.
