# syntax=docker/dockerfile:1.7

# FEATURE: D13

ARG RUST_VERSION=1
ARG DEBIAN_VERSION=bookworm

FROM rust:${RUST_VERSION}-${DEBIAN_VERSION} AS builder
WORKDIR /workspace

ARG PACKAGE
ARG BIN

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

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && groupadd --system --gid 65532 ai-blaise \
    && useradd --system --uid 65532 --gid 65532 --home-dir /nonexistent --shell /usr/sbin/nologin ai-blaise

COPY --from=builder /tmp/ai-blaise-app /usr/local/bin/ai-blaise-app

USER 65532:65532
EXPOSE 8080
ENTRYPOINT ["/usr/local/bin/ai-blaise-app"]
CMD ["serve"]
