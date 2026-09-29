# sidecar/auth_client

Shared bounded client for centralized Auth3 token introspection.

Production construction accepts only an HTTPS endpoint whose path is exactly
`/auth/introspect`. It disables ambient proxy and platform root discovery,
trusts the CA bundle mounted at `AI_BLAISE_GRAPHQL_AUTH_CA_CERT_PATH`, and
requires a combined PEM client certificate/private-key identity at
`AI_BLAISE_GRAPHQL_AUTH_CLIENT_IDENTITY_PATH`. Certificate hostname validation
remains enabled. Request/connect time is bounded by
`AI_BLAISE_GRAPHQL_AUTH_TIMEOUT_MS` (default 750 ms, maximum five seconds).

The response is capped at 16 KiB and accepted only when it is active, contains
the complete expected claim set, matches the configured issuer and audience,
and is currently valid. Bearer tokens are capped at 16 KiB. Credential files
are read through a single opened handle with a 1 MiB cap so Kubernetes
projected-secret symlinks remain supported without a metadata/read race.
Sensitive request and response buffers are zeroized, and public errors and
debug output omit tokens, endpoints, certificate paths, response bodies, and
identity values.

A test-only constructor accepts literal loopback IP HTTP endpoints for the
crate's hermetic transport unit test. It is removed from non-test builds, so
production configuration has no plaintext construction path. Deployment must
provide a TLS endpoint which validates the client certificate before forwarding
to Auth3.
