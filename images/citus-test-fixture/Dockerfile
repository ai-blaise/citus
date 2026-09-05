# syntax=docker/dockerfile:1
# FEATURE: A10 A11
#
# Source-built Citus fixture for companion integration tests. This image is a
# test input, not the Bundle1 release operand.

ARG BASE_IMAGE

FROM ${BASE_IMAGE} AS citus-build
ARG CITUS_EXTENSION_VERSION
ARG MAKE_JOBS=2
ARG PG_MAJOR
ENV DEBIAN_FRONTEND=noninteractive

RUN set -eux; \
    case "${PG_MAJOR}" in 16|17|18) ;; *) exit 1 ;; esac; \
    test "$(pg_config --version | cut -d' ' -f2 | cut -d. -f1)" = "${PG_MAJOR}"; \
    test -n "${PG_VERSION}"; \
    apt-get update; \
    apt-get install -y --no-install-recommends \
      autoconf \
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
      libxslt1-dev \
      libzstd-dev \
      llvm-dev \
      pkg-config \
      "postgresql-server-dev-${PG_MAJOR}=${PG_VERSION}" \
      python3 \
      uuid-dev \
      zlib1g-dev; \
    rm -rf /var/lib/apt/lists/*; \
    test -s "$(pg_config --includedir-server)/postgres.h"; \
    install -d /out

WORKDIR /src/citus
COPY configure configure.ac Makefile.global.in Makefile aclocal.m4 autogen.sh prep_buildtree ./
COPY config ./config
COPY src ./src
COPY vendor ./vendor

RUN set -eux; \
    ./configure PG_CONFIG="$(command -v pg_config)"; \
    make -j"${MAKE_JOBS}"; \
    make install-all DESTDIR=/out; \
    test -s "/out$(pg_config --pkglibdir)/citus.so"; \
    test -s "/out$(pg_config --pkglibdir)/citus_columnar.so"; \
    test -s "/out$(pg_config --sharedir)/extension/citus.control"; \
    test -s "/out$(pg_config --sharedir)/extension/citus_columnar.control"; \
    extension_dir="/out$(pg_config --sharedir)/extension"; \
    test "$(sed -n "s/^default_version = '\([^']*\)'$/\1/p" "${extension_dir}/citus.control")" = "${CITUS_EXTENSION_VERSION}"; \
    for sql_file in src/backend/distributed/build/sql/citus--*.sql src/backend/columnar/build/sql/citus_columnar--*.sql; do \
      test -s "${sql_file}"; \
      cmp "${sql_file}" "${extension_dir}/$(basename "${sql_file}")"; \
    done

# Upstream composes the default Citus version from its historical install SQL
# and upgrade chain; it does not ship a direct citus--<default_version>.sql.
# Verify every generated install/upgrade/downgrade script above. Consumers then
# execute CREATE EXTENSION against the installed control file's default version.

FROM ${BASE_IMAGE} AS companion-test-fixture
ARG AI_BLAISE_FIXTURE_ID
ARG AI_BLAISE_SOURCE_CONTENT_SHA256
ARG AI_BLAISE_SOURCE_GIT_SHA
ARG AI_BLAISE_SOURCE_GIT_TREE
ARG AI_BLAISE_SOURCE_TREE_STATE
ARG BASE_IMAGE
ARG CITUS_EXTENSION_VERSION
ARG PG_MAJOR

LABEL org.opencontainers.image.revision="${AI_BLAISE_SOURCE_GIT_SHA}" \
      ai-blaise.citus.test-fixture="true" \
      ai-blaise.citus.test-fixture.scope="source-built-companion-test-only" \
      ai-blaise.citus.test-fixture.release-target="false" \
      ai-blaise.citus.test-fixture.pg-major="${PG_MAJOR}" \
      ai-blaise.citus.test-fixture.base-image="${BASE_IMAGE}" \
      ai-blaise.citus.test-fixture.citus-extension-version="${CITUS_EXTENSION_VERSION}" \
      ai-blaise.citus.test-fixture.id="${AI_BLAISE_FIXTURE_ID}" \
      ai-blaise.citus.source-content-sha256="${AI_BLAISE_SOURCE_CONTENT_SHA256}" \
      ai-blaise.citus.source-git-sha="${AI_BLAISE_SOURCE_GIT_SHA}" \
      ai-blaise.citus.source-git-tree="${AI_BLAISE_SOURCE_GIT_TREE}" \
      ai-blaise.citus.source-tree-state="${AI_BLAISE_SOURCE_TREE_STATE}"

ENV DEBIAN_FRONTEND=noninteractive
RUN set -eux; \
    apt-get update; \
    apt-get install -y --no-install-recommends \
      libcurl4 \
      libicu72 \
      libkrb5-3 \
      liblz4-1 \
      libssl3 \
      libxml2 \
      libxslt1.1 \
      libzstd1; \
    rm -rf /var/lib/apt/lists/*

COPY --from=citus-build /out/ /
COPY images/citus-pg-overlay/extensions/ai_blaise_citus.control /usr/local/share/ai-blaise-test-fixture/
COPY images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.0.sql /usr/local/share/ai-blaise-test-fixture/
COPY images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.0--0.1.1.sql /usr/local/share/ai-blaise-test-fixture/
COPY images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.1--0.1.0.sql /usr/local/share/ai-blaise-test-fixture/
COPY images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.1--0.1.2.sql /usr/local/share/ai-blaise-test-fixture/

RUN set -eux; \
    test "$(pg_config --version | cut -d' ' -f2 | cut -d. -f1)" = "${PG_MAJOR}"; \
    extension_dir="$(pg_config --sharedir)/extension"; \
    install -d -m 0755 "${extension_dir}"; \
    install -m 0644 /usr/local/share/ai-blaise-test-fixture/ai_blaise_citus.control "${extension_dir}/"; \
    install -m 0644 /usr/local/share/ai-blaise-test-fixture/ai_blaise_citus--0.1.0.sql "${extension_dir}/"; \
    install -m 0644 /usr/local/share/ai-blaise-test-fixture/ai_blaise_citus--0.1.0--0.1.1.sql "${extension_dir}/"; \
    install -m 0644 /usr/local/share/ai-blaise-test-fixture/ai_blaise_citus--0.1.1--0.1.0.sql "${extension_dir}/"; \
    install -m 0644 /usr/local/share/ai-blaise-test-fixture/ai_blaise_citus--0.1.1--0.1.2.sql "${extension_dir}/"; \
    test -s "$(pg_config --pkglibdir)/citus.so"; \
    test -s "${extension_dir}/citus.control"; \
    test -s "${extension_dir}/ai_blaise_citus.control"

HEALTHCHECK --interval=10s --timeout=5s --start-period=30s --retries=12 \
  CMD pg_isready -U "${POSTGRES_USER:-postgres}" || exit 1

CMD ["postgres", "-c", "shared_preload_libraries=citus"]
