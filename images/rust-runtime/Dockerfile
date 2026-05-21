# syntax=docker/dockerfile:1.7

# FEATURE: D13

ARG RUST_VERSION=1
ARG DEBIAN_VERSION=bookworm

FROM rust:${RUST_VERSION}-${DEBIAN_VERSION} AS builder
WORKDIR /workspace

ARG PACKAGE
ARG BIN
ARG DEFAULT_ARGS=serve

COPY Cargo.toml Cargo.lock ./
COPY companion ./companion
COPY operator ./operator
COPY pool ./pool
COPY sidecar ./sidecar
COPY tools ./tools
COPY e2e ./e2e

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/workspace/target \
    cargo build --release -p "${PACKAGE}" --bin "${BIN}" \
    && cp "/workspace/target/release/${BIN}" /tmp/ai-blaise-app

FROM debian:${DEBIAN_VERSION}-slim AS runtime
ARG DEFAULT_ARGS=serve

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && groupadd --system --gid 65532 ai-blaise \
    && useradd --system --uid 65532 --gid 65532 --home-dir /nonexistent --shell /usr/sbin/nologin ai-blaise

COPY --from=builder /tmp/ai-blaise-app /usr/local/bin/ai-blaise-app

ENV AI_BLAISE_DEFAULT_ARGS="${DEFAULT_ARGS}"
RUN printf '%s\n' \
    '#!/usr/bin/env sh' \
    'set -eu' \
    'if [ "$#" -eq 0 ]; then' \
    '  set -- ${AI_BLAISE_DEFAULT_ARGS}' \
    'fi' \
    'exec /usr/local/bin/ai-blaise-app "$@"' \
    > /usr/local/bin/ai-blaise-entrypoint \
    && chmod +x /usr/local/bin/ai-blaise-entrypoint

USER 65532:65532
EXPOSE 8080
ENTRYPOINT ["/usr/local/bin/ai-blaise-entrypoint"]
CMD []
