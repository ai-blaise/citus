# syntax=docker/dockerfile:1
# FEATURE: Bundle1 TS19 TS20

ARG BASE_IMAGE=postgres:17-bookworm
FROM ${BASE_IMAGE}

ARG MAKE_JOBS=2
ARG AI_BLAISE_EXTENSION_BUNDLE_VERSION=0.1.0

LABEL org.opencontainers.image.source="https://github.com/ai-blaise/citus" \
      org.opencontainers.image.title="ai-blaise/citus pg_cron cohabitation smoke image" \
      org.opencontainers.image.description="PostgreSQL 17 image with pg_cron, this Citus fork, and ai_blaise_citus installed for narrow cohabitation proof" \
      ai-blaise.citus.extension-bundle="${AI_BLAISE_EXTENSION_BUNDLE_VERSION}"

ENV DEBIAN_FRONTEND=noninteractive

RUN set -eux; \
    apt-get update; \
    apt-get install -y --no-install-recommends \
      bison \
      build-essential \
      ca-certificates \
      clang \
      flex \
      libcurl4-openssl-dev \
      libicu-dev \
      libkrb5-dev \
      liblz4-dev \
      libreadline-dev \
      libssl-dev \
      libxml2-dev \
      libxslt-dev \
      libzstd-dev \
      lz4 \
      pkg-config \
      postgresql-17-cron \
      postgresql-server-dev-17 \
      uuid-dev \
      zlib1g-dev; \
    rm -rf /var/lib/apt/lists/*

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
