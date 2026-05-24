# syntax=docker/dockerfile:1
# FEATURE: TS6 TS18

ARG BASE_IMAGE=timescale/timescaledb:latest-pg17
FROM ${BASE_IMAGE}

ARG MAKE_JOBS=4
ARG AI_BLAISE_EXTENSION_BUNDLE_VERSION=0.1.0

LABEL org.opencontainers.image.source="https://github.com/ai-blaise/citus" \
      org.opencontainers.image.title="ai-blaise/citus Timescale cohabitation smoke image" \
      org.opencontainers.image.description="TimescaleDB image with this Citus fork and ai_blaise_citus installed for hook-chain cohabitation proof" \
      ai-blaise.citus.extension-bundle="${AI_BLAISE_EXTENSION_BUNDLE_VERSION}"

USER root

RUN set -eux; \
    apk add --no-cache \
      autoconf \
      automake \
      bash \
      bison \
      build-base \
      clang19 \
      curl-dev \
      flex \
      icu-dev \
      krb5-dev \
      libxml2-dev \
      libxslt-dev \
      libtool \
      lz4-dev \
      llvm19-dev \
      openssl-dev \
      perl \
      python3 \
      readline-dev \
      zlib-dev \
      zstd-dev

WORKDIR /build/citus
COPY . /build/citus

RUN set -eux; \
    ./configure PG_CONFIG="$(command -v pg_config)" --without-pg-version-check; \
    make -j"${MAKE_JOBS}"; \
    make install; \
    extension_dir="$(pg_config --sharedir)/extension"; \
    install -m 0644 images/citus-pg-overlay/extensions/ai_blaise_citus.control \
      "${extension_dir}/ai_blaise_citus.control"; \
    install -m 0644 images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.0.sql \
      "${extension_dir}/ai_blaise_citus--0.1.0.sql"; \
    rm -rf /build/citus
