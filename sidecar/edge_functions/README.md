# sidecar/edge_functions

> Production boundary: unless a feature is explicitly `Status: production-ready`
> in `docs/ai-blaise/NEW_FEATURES.md`, the surfaces listed here are alpha
> contracts. Deterministic canonical reports and local runtime models are CI
> artifacts, not production evidence; promotion requires live VM/container or
> Kubernetes evidence recorded in `docs/ai-blaise/PRODUCTION_READINESS_AUDIT.md`
> and guarded by `ci/ai-blaise/production-gap-audit.sh`.

Deno and Bun runtime boundary for declarative edge functions.

Implemented surface:

- `EdgeFunctionPlan`, `EdgeFunctionRuntime`, `FunctionSource`,
  `FunctionTrigger`, `DbCallbackPlan`, `RuntimeLaunchPlan`, and
  `InvocationRequest` validation.
- Deno/Bun launch-command rendering with env secret references and optional UDS
  database callback socket.
- Runtime host plan-only invocation accounting, trigger authorization, callback
  timeout bounds, deterministic response sizing, and fail-closed rejection for
  unsupported external Deno/Bun execution.
- Registry surface for function registration, listing, invocation, scheduled
  trigger discovery, CDC event matching, and safe UDS callback statement checks.
- HTTP front door for `/healthz`, `/readyz`, `/metrics`, `GET /functions`,
  `POST /functions`, and `POST /functions/<name>`.
- `cargo run -p ai_blaise_citus_sidecar_edge_functions -- run-canonical`.
- `cargo run -p ai_blaise_citus_sidecar_edge_functions -- run-runtime-canonical`.
- `cargo run -p ai_blaise_citus_sidecar_edge_functions -- run-registry-canonical`.
- `cargo run -p ai_blaise_citus_sidecar_edge_functions -- run-bun-runtime-canonical`.
- `bash ci/ai-blaise/sidecar-edge-functions-runtime-smoke.sh` builds the binary, boots the live Rust HTTP server, verifies registry/invocation planning, and checks fail-closed request boundaries.
- `bash ci/ai-blaise/sidecar-api-runtime-smoke.sh` builds the binary and verifies probe/drain fail-closed behavior.
- `bash ci/ai-blaise/api-trio-runtime-smoke.sh` boots the service and verifies
  readiness, registry listing, and canonical invocation over real TCP.

These contracts cover `FEATURE: EF1`, `FEATURE: EF2`, `FEATURE: EF4`, and
`FEATURE: EF5`, and mirror the `FEATURE: EF3` declarative CRD shape at runtime.
They do not prove sandboxed user-code execution. EF1, EF2, EF4, and EF5 remain alpha until the Deno/Bun worker process path and production isolation controls are live-smoked.
