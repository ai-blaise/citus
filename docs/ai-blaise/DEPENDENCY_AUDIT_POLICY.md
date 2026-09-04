# Dependency audit policy

The release and operator gates run `cargo audit --deny warnings`. Every
vulnerability, unsoundness advisory, yanked release, and unmaintained-package
warning therefore fails CI unless its RustSec ID appears in the narrow
repository policy at `.cargo/audit.toml`.

## Approved exception

| Advisory | Package | Reachability | Rationale and containment | Removal condition |
| --- | --- | --- | --- | --- |
| `RUSTSEC-2021-0127` | `serde_cbor` 0.11.2 | `ai_blaise_citus_companion --features pg18` -> `pgrx` 0.18 -> `serde_cbor`; absent from the default companion graph and the complete `ai_blaise_citus_operator` graph | This is an unmaintained-package notice, not a known vulnerability. The companion exposes no `PostgresType` derive or application CBOR input surface. `pgrx` owns the dependency, and the latest published `pgrx` 0.19.2 still declares `serde_cbor = "0.11.2"`, so no fixed upstream release exists. `pgrx` 0.19.2 also requires Rust 1.96 while this repository deliberately pins 1.95; upgrading it would not remove the warning. | Remove the exception as soon as pgrx publishes a release that removes `serde_cbor`, or replace the pgrx integration if upstream does not. Re-review on every pgrx or Rust toolchain bump and no later than 2026-12-04. |

`ci/ai-blaise/cargo-audit-policy-smoke.sh` enforces both halves of this policy:
the warnings-denied workspace audit must pass using the single approved ID, and
the approved package must remain absent from the operator's dependency graph.
Adding another exception requires updating this document and the smoke's exact
allowlist; a broad advisory-class waiver is not permitted.

The 2026-09-04 hardening sweep removed all other findings rather than waiving
them. The operator moved to `kube`/`kube-runtime` 4.2 and no longer resolves the
unmaintained `backoff`, `instant`, or `rustls-pemfile` paths. Lockfile refreshes
resolved `event-listener` 5.4.2, `chacha20` 0.10.2, `rand` 0.10.2, and `anyhow`
1.0.104. The analytical sidecar moved from DataFusion 48 to 55 (and aligned
Parquet 59), removing the unmaintained `paste` dependency instead of granting
an exception.
