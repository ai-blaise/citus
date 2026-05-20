# syntax=docker/dockerfile:1
# FEATURE: Bundle1 Search1 G1 JS1 PM1 IA1 WF1 F2

ARG BASE_IMAGE=postgres:18-bookworm
FROM ${BASE_IMAGE}

ARG PG_MAJOR=18
ARG AI_BLAISE_EXTENSION_BUNDLE_VERSION=0.1.0

LABEL org.opencontainers.image.source="https://github.com/ai-blaise/citus" \
      org.opencontainers.image.title="ai-blaise/citus PostgreSQL overlay" \
      org.opencontainers.image.description="PostgreSQL operand image contract for Citus, TimescaleDB, companion, and bundled extensions" \
      ai-blaise.citus.pg-major="${PG_MAJOR}" \
      ai-blaise.citus.extension-bundle="${AI_BLAISE_EXTENSION_BUNDLE_VERSION}"

COPY extension-manifest.tsv /usr/local/share/ai-blaise/citus/extension-manifest.tsv
COPY shared-preload-libraries.conf /usr/local/share/ai-blaise/citus/shared-preload-libraries.conf
COPY extensions/ai_blaise_citus.control /usr/local/share/ai-blaise/citus/extensions/ai_blaise_citus.control
COPY extensions/ai_blaise_citus--0.1.0.sql /usr/local/share/ai-blaise/citus/extensions/ai_blaise_citus--0.1.0.sql
COPY initdb.d/00-ai-blaise-extensions.sql /docker-entrypoint-initdb.d/00-ai-blaise-extensions.sql

RUN set -eux; \
    install -d -m 0755 /etc/postgresql/ai-blaise; \
    install -d -m 0755 "/usr/share/postgresql/${PG_MAJOR}/extension"; \
    cp /usr/local/share/ai-blaise/citus/shared-preload-libraries.conf /etc/postgresql/ai-blaise/shared-preload-libraries.conf; \
    cp /usr/local/share/ai-blaise/citus/extensions/ai_blaise_citus.control "/usr/share/postgresql/${PG_MAJOR}/extension/ai_blaise_citus.control"; \
    cp /usr/local/share/ai-blaise/citus/extensions/ai_blaise_citus--0.1.0.sql "/usr/share/postgresql/${PG_MAJOR}/extension/ai_blaise_citus--0.1.0.sql"; \
    sed -n '1,120p' /usr/local/share/ai-blaise/citus/extension-manifest.tsv >/usr/local/share/ai-blaise/citus/required-extension-preview.tsv

HEALTHCHECK --interval=30s --timeout=5s --start-period=30s --retries=3 \
  CMD pg_isready -U "${POSTGRES_USER:-postgres}" || exit 1
