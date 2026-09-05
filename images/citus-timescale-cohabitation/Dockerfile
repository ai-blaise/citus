# syntax=docker/dockerfile:1
# FEATURE: TS6 TS18
#
# Source-built Citus + vendor TimescaleDB fixture for companion cohabitation
# tests. The exact vendor image stays intact as the base; this is test-only and
# is not a Bundle1 or release operand.

ARG BASE_IMAGE
FROM ${BASE_IMAGE}

ARG AI_BLAISE_COHAB_FIXTURE_ID
ARG AI_BLAISE_SOURCE_CONTENT_SHA256
ARG AI_BLAISE_SOURCE_GIT_SHA
ARG AI_BLAISE_SOURCE_GIT_TREE
ARG AI_BLAISE_SOURCE_TREE_STATE
ARG BASE_IMAGE
ARG CITUS_EXTENSION_VERSION
ARG COMPANION_EXTENSION_VERSION
ARG MAKE_JOBS=2
ARG PG_MAJOR=17
ARG TIMESCALEDB_MINOR=2.27
ARG WITH_LLVM=no

LABEL org.opencontainers.image.source="https://github.com/ai-blaise/citus" \
      org.opencontainers.image.revision="${AI_BLAISE_SOURCE_GIT_SHA}" \
      org.opencontainers.image.title="ai-blaise Citus + TimescaleDB test fixture" \
      org.opencontainers.image.description="Exact vendor TimescaleDB image with source-built Citus and companion SQL for cohabitation tests" \
      ai-blaise.citus.test-fixture.timescale="true" \
      ai-blaise.citus.test-fixture.scope="source-built-citus-timescaledb-companion-test-only" \
      ai-blaise.citus.test-fixture.release-target="false" \
      ai-blaise.citus.test-fixture.pg-major="${PG_MAJOR}" \
      ai-blaise.citus.test-fixture.base-image="${BASE_IMAGE}" \
      ai-blaise.citus.test-fixture.timescaledb-minor="${TIMESCALEDB_MINOR}" \
      ai-blaise.citus.test-fixture.citus-extension-version="${CITUS_EXTENSION_VERSION}" \
      ai-blaise.citus.test-fixture.companion-extension-version="${COMPANION_EXTENSION_VERSION}" \
      ai-blaise.citus.test-fixture.timescale-id="${AI_BLAISE_COHAB_FIXTURE_ID}" \
      ai-blaise.citus.source-content-sha256="${AI_BLAISE_SOURCE_CONTENT_SHA256}" \
      ai-blaise.citus.source-git-sha="${AI_BLAISE_SOURCE_GIT_SHA}" \
      ai-blaise.citus.source-git-tree="${AI_BLAISE_SOURCE_GIT_TREE}" \
      ai-blaise.citus.source-tree-state="${AI_BLAISE_SOURCE_TREE_STATE}"

USER root
ENV DEBIAN_FRONTEND=noninteractive

RUN set -eux; \
    case "${TIMESCALEDB_MINOR}:${BASE_IMAGE}" in \
      "2.27:docker.io/timescale/timescaledb-ha:pg17-ts2.27@sha256:4f61167e11c7c95bedf96433c720d671a53aa29ad7f52b142b529a6d0e9f0b20"|\
      "2.28:docker.io/timescale/timescaledb-ha:pg17-ts2.28@sha256:bc9e09875460aa69fb536362fef7c8e92c51ad6aab3d13f91a2487d3547dc71a") ;; \
      *) exit 1 ;; \
    esac; \
    test "${PG_MAJOR}" = "17"; \
    test "$(pg_config --version | cut -d' ' -f2 | cut -d. -f1)" = "${PG_MAJOR}"; \
    timescaledb_control="$(pg_config --sharedir)/extension/timescaledb.control"; \
    test -s "${timescaledb_control}"; \
    timescaledb_version="$(sed -n "s/^default_version = '\([^']*\)'$/\1/p" "${timescaledb_control}")"; \
    case "${timescaledb_version}" in "${TIMESCALEDB_MINOR}"|"${TIMESCALEDB_MINOR}".*) ;; *) exit 1 ;; esac; \
    postgresql_package_version="$(dpkg-query -W -f='${Version}' "postgresql-${PG_MAJOR}")"; \
    test -n "${postgresql_package_version}"; \
    install -d -m 0755 /build/vendor-baseline; \
    printf '%s\n' "${postgresql_package_version}" > /build/vendor-baseline/postgresql-package-version; \
    pg_config --version > /build/vendor-baseline/pg-config-version; \
    "$(pg_config --bindir)/postgres" --version > /build/vendor-baseline/postgres-version; \
    sha256sum "$(pg_config --bindir)/postgres" > /build/vendor-baseline/postgres.sha256; \
    sha256sum "${timescaledb_control}" > /build/vendor-baseline/timescaledb-control.sha256; \
    find "$(pg_config --pkglibdir)" -maxdepth 1 -type f -name 'timescaledb*.so' -print0 \
      | LC_ALL=C sort -z | xargs -0 sha256sum > /build/vendor-baseline/timescaledb-libraries.sha256; \
    test -s /build/vendor-baseline/timescaledb-libraries.sha256; \
    command -v apt-get; \
    apt-get update; \
    apt-get install -y --no-install-recommends \
      autoconf \
      bison \
      build-essential \
      ca-certificates \
      flex \
      libcurl4-openssl-dev \
      libicu-dev \
      libkrb5-dev \
      liblz4-dev \
      libreadline-dev \
      libssl-dev \
      libtool \
      libxml2-dev \
      libxslt1-dev \
      libzstd-dev \
      perl \
      pkg-config \
      "postgresql-server-dev-${PG_MAJOR}=${postgresql_package_version}" \
      python3 \
      uuid-dev \
      zlib1g-dev; \
    rm -rf /var/lib/apt/lists/*; \
    test -s "$(pg_config --includedir-server)/postgres.h"; \
    test "$(dpkg-query -W -f='${Version}' "postgresql-${PG_MAJOR}")" = "${postgresql_package_version}"; \
    test "$(pg_config --version)" = "$(cat /build/vendor-baseline/pg-config-version)"; \
    test "$("$(pg_config --bindir)/postgres" --version)" = "$(cat /build/vendor-baseline/postgres-version)"; \
    sha256sum --check /build/vendor-baseline/postgres.sha256; \
    sha256sum --check /build/vendor-baseline/timescaledb-control.sha256; \
    sha256sum --check /build/vendor-baseline/timescaledb-libraries.sha256

WORKDIR /build/citus
COPY configure configure.ac Makefile.global.in Makefile aclocal.m4 autogen.sh prep_buildtree ./
COPY config ./config
COPY src ./src
COPY vendor ./vendor
COPY images/citus-pg-overlay/extensions/ai_blaise_citus.control /build/companion/
COPY images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.0.sql /build/companion/
COPY images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.0--0.1.1.sql /build/companion/
COPY images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.1--0.1.0.sql /build/companion/
COPY images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.1--0.1.2.sql /build/companion/
COPY images/citus-pg-overlay/upgrades/ai_blaise_citus--0.1.2.sql /build/companion/archive/

RUN set -eux; \
    ./configure PG_CONFIG="$(command -v pg_config)" --without-pg-version-check; \
    make -j"${MAKE_JOBS}" with_llvm="${WITH_LLVM}"; \
    make install-all with_llvm="${WITH_LLVM}"; \
    extension_dir="$(pg_config --sharedir)/extension"; \
    test -s "$(pg_config --pkglibdir)/citus.so"; \
    test -s "$(pg_config --pkglibdir)/citus_columnar.so"; \
    test "$(sed -n "s/^default_version = '\([^']*\)'$/\1/p" "${extension_dir}/citus.control")" = "${CITUS_EXTENSION_VERSION}"; \
    for sql_file in src/backend/distributed/build/sql/citus--*.sql src/backend/columnar/build/sql/citus_columnar--*.sql; do \
      test -s "${sql_file}"; \
      cmp "${sql_file}" "${extension_dir}/$(basename "${sql_file}")"; \
    done; \
    install -D -m 0644 /build/companion/archive/ai_blaise_citus--0.1.2.sql \
      /usr/local/share/ai-blaise/citus/upgrades/ai_blaise_citus--0.1.2.sql; \
    install -m 0644 /build/companion/ai_blaise_citus.control "${extension_dir}/"; \
    install -m 0644 /build/companion/ai_blaise_citus--0.1.0.sql "${extension_dir}/"; \
    install -m 0644 /build/companion/ai_blaise_citus--0.1.0--0.1.1.sql "${extension_dir}/"; \
    install -m 0644 /build/companion/ai_blaise_citus--0.1.1--0.1.0.sql "${extension_dir}/"; \
    install -m 0644 /build/companion/ai_blaise_citus--0.1.1--0.1.2.sql "${extension_dir}/"; \
    test "$(sed -n "s/^default_version = '\([^']*\)'$/\1/p" "${extension_dir}/ai_blaise_citus.control")" = "${COMPANION_EXTENSION_VERSION}"; \
    test -s "${extension_dir}/timescaledb.control"; \
    test "$(dpkg-query -W -f='${Version}' "postgresql-${PG_MAJOR}")" = "$(cat /build/vendor-baseline/postgresql-package-version)"; \
    test "$(pg_config --version)" = "$(cat /build/vendor-baseline/pg-config-version)"; \
    test "$("$(pg_config --bindir)/postgres" --version)" = "$(cat /build/vendor-baseline/postgres-version)"; \
    sha256sum --check /build/vendor-baseline/postgres.sha256; \
    sha256sum --check /build/vendor-baseline/timescaledb-control.sha256; \
    sha256sum --check /build/vendor-baseline/timescaledb-libraries.sha256; \
    rm -rf /build/citus /build/companion /build/vendor-baseline

USER postgres
