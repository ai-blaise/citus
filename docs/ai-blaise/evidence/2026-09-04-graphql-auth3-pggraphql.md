# GraphQL/Auth3/pg_graphql — development evidence

The bounded API3 path passed on 2026-09-04 using the existing
`instance-20260415-20260415-235136` development VM in `asia-south1-b`. No
cloud resource was created. This host is not a trusted release builder, so the
result is diagnostic evidence and is not a production-promotion receipt.

The run used the already-loaded full overlay image
`ai-blaise-citus-overlay:b1-current-0.1.2-full-aebc81b`, whose local image ID
was
`sha256:93bbc5b351a468df14c7a61ed53dde6be7153dbc3ff4b01b76d7ee17a26b`.
The repository base was `e10607031da0ccd2cb3fd948b22902959dcd5f9a` with
uncommitted changes. Dependencies were resolved offline from the exact
`Cargo.lock`; no registry pull or dependency substitution occurred during the
run.

The live smoke reported:

```text
graphql_pggraphql_live=passed
auth3_introspection=true
mtls_client_certificate_required=true
tenant_a_rows=1
tenant_b_rows=1
rls_cross_tenant_hidden=true
body_identity_rejected=true
revoked_token_rejected=true
graphql_resolve_executed=true
```

The test started the real Auth3 process on `127.0.0.1` behind a disposable TLS
proxy that required a client certificate, and proved that a client without a
certificate was rejected. The GraphQL process presented its configured client
identity, introspected exactly one bearer token over HTTPS using an explicit
CA, installed only the returned claims, and called `graphql.resolve` in the
same PostgreSQL transaction. Two tokens mapped to separate tenants and each
saw exactly its own RLS-filtered row. Request-body `jwt_claims` and `tenant_id`
were rejected, and logout/revocation caused the formerly valid token to fail
closed. The database container, anonymous volume, processes, credentials, and
isolated source/target directories were removed after the run.

## Exact tested sources

The aggregate receipt is
`1f0c02cfc00f17bd7c87f3016c7f4b629fcdad479aff689bbb26dade2d5ccd37`.
It is SHA-256 over the table's ordered paths, where each entry contributes UTF-8
path bytes, a NUL byte, the content length as an unsigned eight-byte big-endian
integer, and the file bytes. Files were hash-compared after transfer; the final
GraphQL library and smoke hashes were compared again immediately before the
successful run.

| File | SHA-256 |
| --- | --- |
| `Cargo.toml` | `760d2ca046b0c10bc5dc97cb7d3b55ec5db37a9927baf860edfcfd5467e2296b` |
| `Cargo.lock` | `77e36361c84a9e808938843fb98bbd5681750cdac064da7563e07349fbe03bcd` |
| `sidecar/shared/Cargo.toml` | `5b0ecc51e608f591329ab5bfe2a4fe9d013dc35961ce5efcfcaa5ee91c209d1d` |
| `sidecar/shared/src/lib.rs` | `a603ad8f0ec79c5009ad97a0f66b04e756ad03b7a5d25c82fa28d7a5a9d942de` |
| `sidecar/auth/Cargo.toml` | `5b12fe5ddd4d850876c722a329457f4fef0ee0ef8e172c662e23ac588fd5599f` |
| `sidecar/auth/src/lib.rs` | `47cd53ccfbd15aa71d0c0fc05c71726ac32e075466a9ab0e87673de29de08f7e` |
| `sidecar/auth/src/main.rs` | `2cbc99d7ab7da3edc1865b5a2c324ee38831966a64078fc6ed289640afbfbf48` |
| `sidecar/auth_client/Cargo.toml` | `0dfb8945070df8763a07b2396708816e043d3590dc549b38a2c2a4bc0ee160ae` |
| `sidecar/auth_client/src/lib.rs` | `dc865a75d03fa7de9ea95f00fbddf87848568e056ea035cdc7a76bfbe167933a` |
| `sidecar/graphql/Cargo.toml` | `dc9aaa71c4bc256e4bdf1c37cf364bcf0609b960c7cb1380060bf5ca238274dd` |
| `sidecar/graphql/src/lib.rs` | `a2818d2ca2996b60a6539b6cbb59cfef205349cb4b83d4237a1677218178ca07` |
| `sidecar/graphql/src/main.rs` | `0d9a87306c02faf2f2783f123dc9fbf83c687a8d89502c22833eb62f64a51cbc` |
| `ci/ai-blaise/graphql-pggraphql-live-smoke.sh` | `2d9b56964327c6367310a7006538ee6945e889d4e5df762831a57b4a2528e1ac` |

Changing any listed file invalidates this receipt until the smoke is rerun.

## Unproven production boundaries

This run does not prove production proxy or certificate deployment and
rotation, Kubernetes network policy, secure Auth3 enrollment/administration,
durable or replica-consistent identity/session/revocation state, GraphQL HA or
multiworker behavior, database reconnect and failover, query-cost and response
limits, sustained load, or subscriptions. The authenticated WebSocket route
continues to return HTTP 501. API3 therefore remains alpha.
