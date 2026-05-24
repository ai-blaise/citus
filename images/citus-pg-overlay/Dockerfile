# syntax=docker/dockerfile:1
# FEATURE: Bundle1 Search1 G1 JS1 PM1 IA1 WF1 F2 PGC1 PGC2
#
# PG version matrix: this Dockerfile is build-arg parameterized so the same
# overlay contract emits a PG17 or PG18 operand image. The Makefile targets
# `build-image-pg17` and `build-image-pg18` (and the `image-contract` CI
# matrix) drive both. Operators choose a major via Helm `postgresMajor` and
# match it against the image tag they pull.
#
#   PG17:  docker build --build-arg PG_MAJOR=17 --build-arg BASE_IMAGE=postgres:17-bookworm .
#   PG18:  docker build --build-arg PG_MAJOR=18 --build-arg BASE_IMAGE=postgres:18-bookworm .

ARG PG_MAJOR=17
ARG BASE_IMAGE=postgres:${PG_MAJOR}-bookworm
FROM ${BASE_IMAGE}

# Re-declare ARG after FROM so the build stage can interpolate it.
ARG PG_MAJOR
ARG AI_BLAISE_EXTENSION_BUNDLE_VERSION=0.1.0

LABEL org.opencontainers.image.source="https://github.com/ai-blaise/citus" \
      org.opencontainers.image.title="ai-blaise/citus PostgreSQL overlay" \
      org.opencontainers.image.description="PostgreSQL operand image contract for Citus, TimescaleDB, companion, and bundled extensions" \
      ai-blaise.citus.pg-major="${PG_MAJOR}" \
      ai-blaise.citus.extension-bundle="${AI_BLAISE_EXTENSION_BUNDLE_VERSION}" \
      ai-blaise.citus.pg-core-patches="patches/postgres/series" \
      ai-blaise.citus.pg-core-patch-targets="logical-commit-clock per-subtrans-commit-ts"

COPY extension-manifest.tsv /usr/local/share/ai-blaise/citus/extension-manifest.tsv
COPY extensions/ai_blaise_citus-upgrade-manifest.tsv /usr/local/share/ai-blaise/citus/ai_blaise_citus-upgrade-manifest.tsv
COPY shared-preload-libraries.conf /usr/local/share/ai-blaise/citus/shared-preload-libraries.conf
COPY extensions/ai_blaise_citus.control /usr/local/share/ai-blaise/citus/extensions/ai_blaise_citus.control
COPY extensions/ai_blaise_citus--0.1.0.sql /usr/local/share/ai-blaise/citus/extensions/ai_blaise_citus--0.1.0.sql
COPY initdb.d/00-ai-blaise-extensions.sql /docker-entrypoint-initdb.d/00-ai-blaise-extensions.sql

# Postgres-core quilt patches (Spock-derived) live at patches/postgres/ in the
# repo root, outside this Dockerfile's build context. This overlay still ships
# against the prebuilt postgres:N-bookworm base image, so the patches are not
# applied here. The image labels above declare the contract: a downstream
# custom-PG-compile pipeline must apply patches/postgres/series to PG source
# (matching ${PG_MAJOR}) before configure/make. Until that pipeline lands, the
# overlay remains alpha for FEATURE: PGC1 and FEATURE: PGC2.

RUN set -eux; \
    install -d -m 0755 /etc/postgresql/ai-blaise; \
    install -d -m 0755 "/usr/share/postgresql/${PG_MAJOR}/extension"; \
    cp /usr/local/share/ai-blaise/citus/shared-preload-libraries.conf /etc/postgresql/ai-blaise/shared-preload-libraries.conf; \
    cp /usr/local/share/ai-blaise/citus/extensions/ai_blaise_citus.control "/usr/share/postgresql/${PG_MAJOR}/extension/ai_blaise_citus.control"; \
    cp /usr/local/share/ai-blaise/citus/extensions/ai_blaise_citus--0.1.0.sql "/usr/share/postgresql/${PG_MAJOR}/extension/ai_blaise_citus--0.1.0.sql"; \
    sed -n '1,120p' /usr/local/share/ai-blaise/citus/extension-manifest.tsv >/usr/local/share/ai-blaise/citus/required-extension-preview.tsv; \
    test -s /usr/local/share/ai-blaise/citus/ai_blaise_citus-upgrade-manifest.tsv

HEALTHCHECK --interval=30s --timeout=5s --start-period=30s --retries=3 \
  CMD pg_isready -U "${POSTGRES_USER:-postgres}" || exit 1
