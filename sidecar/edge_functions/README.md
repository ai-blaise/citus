# sidecar/edge_functions

Deno and Bun runtime for declarative edge functions.

Current implemented surface:

- `EdgeFunctionPlan`
- `EdgeFunctionRuntime`
- `FunctionSource`
- `FunctionTrigger`
- `DbCallbackPlan`
- `RuntimeLaunchPlan`
- `InvocationRequest`
- `EdgeFunctionRuntimeHost`
- `EdgeFunctionRuntimeState`
- `EdgeFunctionExecution`
- `canonical_edge_function_report()`
- `canonical_edge_function_runtime_report()`
- `cargo run -p ai_blaise_citus_sidecar_edge_functions -- run-canonical`
- `cargo run -p ai_blaise_citus_sidecar_edge_functions -- run-runtime-canonical`

These contracts cover `FEATURE: EF1`, `FEATURE: EF2`, `FEATURE: EF4`, and
`FEATURE: EF5`.

The runtime surface deterministically validates launch command construction,
configured trigger authorization, DB callback timeout bounds, invocation
accounting, and response sizing for canonical tests.
