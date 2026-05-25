# syntax=docker/dockerfile:1
# FEATURE: Bundle1 Search1 G1 JS1 PM1 IA1 WF1 F2 PGC1 PGC2
#
# Fast contract target: the default final stage keeps the existing PG17/PG18
# overlay contract cheap enough for every PR. It installs the ai_blaise_citus
# SQL fallback and manifests, but does not claim that the full binary bundle is
# present.
#
# Source-build targets: Bundle1 PG17 source-build targets live in named stages
# and are invoked explicitly by ci/ai-blaise/sql-extension-smoke.sh when
# BUNDLE1_BUILD_IMAGE=1. They compile the PGDG-missing PG17 extensions from
# pinned tags and verified commits. The path is intentionally PG17-only until
# each upstream extension lands PG18 support.
#
#   Fast PG17 contract:
#     docker build -f images/citus-pg-overlay/Dockerfile \
#       --build-arg PG_MAJOR=17 --build-arg BASE_IMAGE=postgres:17-bookworm .
#   Source-build light PG17 image:
#     docker build -f images/citus-pg-overlay/Dockerfile \
#       --target bundle1-final-light \
#       --build-arg PG_MAJOR=17 --build-arg BASE_IMAGE=postgres:17-bookworm .
#   Source-build heavy PG17 image:
#     docker build -f images/citus-pg-overlay/Dockerfile \
#       --target bundle1-final-full \
#       --build-arg PG_MAJOR=17 --build-arg BASE_IMAGE=postgres:17-bookworm .

ARG PG_MAJOR=17
ARG BASE_IMAGE=postgres:${PG_MAJOR}-bookworm
ARG AI_BLAISE_EXTENSION_BUNDLE_VERSION=0.1.0
ARG AI_BLAISE_SOURCE_GIT_SHA=unknown
ARG AI_BLAISE_SOURCE_TREE_STATE=unknown

ARG CITUS_TAG=v13.3.0
ARG PGSODIUM_TAG=v3.1.9
ARG PGSODIUM_REF=7222ebc5ed87084a68d526aef977be0f4eb319a2
ARG TOPN_TAG=v2.7.0
ARG TOPN_REF=f636ff1b3586025c81fb84c20483412f3991ed84
ARG PG_SEARCH_TAG=v0.20.11
ARG PG_SEARCH_REF=cd1ba46a116c5a98bd6fe9ae370a2f260aee1394
ARG PG_JSONSCHEMA_TAG=v0.3.4
ARG PG_JSONSCHEMA_REF=cbe74b570d38aa0c4d42914e7a118bcb3adaee7a
ARG PLRUST_TAG=v1.2.8
ARG PLRUST_REF=bd76906a43c05a2afdb7839263431a066f5b42fb
ARG PLV8_TAG=v3.2.4
ARG PLV8_REF=cafc37f7aee850de5478773a4e56f7fadfad8e00
ARG PG_GRAPHQL_TAG=v1.6.1
ARG PG_GRAPHQL_REF=66d4c551db213000506fd858676269ba8f801a44

FROM ${BASE_IMAGE} AS build-base
ARG PG_MAJOR
ENV DEBIAN_FRONTEND=noninteractive
RUN set -eux; \
    test "${PG_MAJOR}" = "17"; \
    apt-get update; \
    apt-get install -y --no-install-recommends \
      bison \
      build-essential \
      ca-certificates \
      clang \
      cmake \
      curl \
      flex \
      git \
      libcurl4-openssl-dev \
      libicu-dev \
      libkrb5-dev \
      liblz4-dev \
      libreadline-dev \
      libsodium-dev \
      libssl-dev \
      libxml2-dev \
      libxslt-dev \
      libzstd-dev \
      llvm-dev \
      lz4 \
      pkg-config \
      postgresql-server-dev-${PG_MAJOR} \
      python3 \
      python3-pip \
      uuid-dev \
      zlib1g-dev; \
    rm -rf /var/lib/apt/lists/*; \
    install -d /out

FROM build-base AS rust-pgrx-base
ARG PG_MAJOR
ENV CARGO_HOME=/root/.cargo \
    RUSTUP_HOME=/root/.rustup \
    PATH=/root/.cargo/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin
RUN set -eux; \
    curl --proto =https --tlsv1.2 -sSf https://sh.rustup.rs \
      | sh -s -- -y --default-toolchain stable --profile minimal; \
    cargo install --locked cargo-pgrx --version 0.16.1; \
    cargo pgrx init --pg${PG_MAJOR} /usr/bin/pg_config

FROM build-base AS build-pgsodium
ARG PGSODIUM_TAG
ARG PGSODIUM_REF
WORKDIR /src
RUN set -eux; \
    git clone --branch "${PGSODIUM_TAG}" --depth 1 https://github.com/michelp/pgsodium.git pgsodium; \
    cd pgsodium; \
    test "$(git rev-parse HEAD)" = "${PGSODIUM_REF}"; \
    make USE_PGXS=1; \
    make USE_PGXS=1 install DESTDIR=/out

FROM build-base AS build-topn
ARG TOPN_TAG
ARG TOPN_REF
WORKDIR /src
RUN set -eux; \
    git clone --branch "${TOPN_TAG}" --depth 1 https://github.com/citusdata/postgresql-topn.git postgresql-topn; \
    cd postgresql-topn; \
    test "$(git rev-parse HEAD)" = "${TOPN_REF}"; \
    make USE_PGXS=1; \
    make USE_PGXS=1 install DESTDIR=/out

FROM rust-pgrx-base AS build-pg-jsonschema
ARG PG_MAJOR
ARG PG_JSONSCHEMA_TAG
ARG PG_JSONSCHEMA_REF
WORKDIR /src
RUN set -eux; \
    git clone --branch "${PG_JSONSCHEMA_TAG}" --depth 1 https://github.com/supabase/pg_jsonschema.git pg_jsonschema; \
    cd pg_jsonschema; \
    test "$(git rev-parse HEAD)" = "${PG_JSONSCHEMA_REF}"; \
    cargo pgrx package --pg-config /usr/bin/pg_config; \
    cp -r target/release/pg_jsonschema-pg${PG_MAJOR}/. /out/

FROM rust-pgrx-base AS build-pg-graphql
ARG PG_MAJOR
ARG PG_GRAPHQL_TAG
ARG PG_GRAPHQL_REF
WORKDIR /src
RUN set -eux; \
    git clone --branch "${PG_GRAPHQL_TAG}" --depth 1 https://github.com/supabase/pg_graphql.git pg_graphql; \
    cd pg_graphql; \
    test "$(git rev-parse HEAD)" = "${PG_GRAPHQL_REF}"; \
    cargo pgrx package --pg-config /usr/bin/pg_config; \
    cp -r target/release/pg_graphql-pg${PG_MAJOR}/. /out/

FROM rust-pgrx-base AS build-pg-search
ARG PG_MAJOR
ARG PG_SEARCH_TAG
ARG PG_SEARCH_REF
WORKDIR /src
RUN set -eux; \
    git clone --branch "${PG_SEARCH_TAG}" --depth 1 https://github.com/paradedb/paradedb.git paradedb; \
    cd paradedb; \
    test "$(git rev-parse HEAD)" = "${PG_SEARCH_REF}"; \
    cd pg_search; \
    cargo pgrx package --pg-config /usr/bin/pg_config; \
    cp -r ../target/release/pg_search-pg${PG_MAJOR}/. /out/

# plrust is intentionally not source-built in this Dockerfile. The latest
# pinned upstream tag v1.2.8 (bd76906...) exposes only pg13/pg14/pg15/pg16 pgrx
# features and pins pgrx 0.11.0, so a PG17 build would require carrying a forked
# compatibility patch. That would violate the upstream-minimal rule. The
# manifest, docs, and smoke keep plrust alpha until upstream lands PG17 support.

FROM build-base AS build-plv8
ARG PLV8_TAG
ARG PLV8_REF
RUN set -eux; \
    apt-get update; \
    apt-get install -y --no-install-recommends g++-12 ninja-build python3-setuptools; \
    rm -rf /var/lib/apt/lists/*
WORKDIR /src
RUN set -eux; \
    git clone --branch "${PLV8_TAG}" --depth 1 https://github.com/plv8/plv8.git plv8; \
    cd plv8; \
    test "$(git rev-parse HEAD)" = "${PLV8_REF}"; \
    USE_ICU=1 make; \
    USE_ICU=1 make install DESTDIR=/out

FROM build-base AS build-citus
ARG CITUS_TAG
WORKDIR /src/citus
COPY configure configure.ac Makefile.global.in Makefile aclocal.m4 autogen.sh /src/citus/
COPY src /src/citus/src
COPY config /src/citus/config
COPY vendor /src/citus/vendor
COPY prep_buildtree /src/citus/prep_buildtree
RUN set -eux; \
    ./configure PG_CONFIG=/usr/bin/pg_config --without-pg-version-check; \
    make -j2; \
    make install DESTDIR=/out; \
    install -d /out/usr/local/share; \
    echo "${CITUS_TAG}" >/out/usr/local/share/ai-blaise-citus-upstream-tag

FROM ${BASE_IMAGE} AS bundle1-source-runtime
ARG PG_MAJOR
ARG AI_BLAISE_EXTENSION_BUNDLE_VERSION
ARG AI_BLAISE_SOURCE_GIT_SHA
ARG AI_BLAISE_SOURCE_TREE_STATE
ARG CITUS_TAG
ARG PGSODIUM_TAG
ARG PGSODIUM_REF
ARG TOPN_TAG
ARG TOPN_REF
ARG PG_SEARCH_TAG
ARG PG_SEARCH_REF
ARG PG_JSONSCHEMA_TAG
ARG PG_JSONSCHEMA_REF
ARG PLRUST_TAG
ARG PLRUST_REF
ARG PLV8_TAG
ARG PLV8_REF
ARG PG_GRAPHQL_TAG
ARG PG_GRAPHQL_REF
LABEL org.opencontainers.image.source="https://github.com/ai-blaise/citus" \
      org.opencontainers.image.title="ai-blaise/citus PostgreSQL overlay source-build" \
      org.opencontainers.image.description="PG17 source-build path for Bundle1 extensions missing from PGDG" \
      ai-blaise.citus.pg-major="${PG_MAJOR}" \
      ai-blaise.citus.source-git-sha="${AI_BLAISE_SOURCE_GIT_SHA}" \
      ai-blaise.citus.source-tree-state="${AI_BLAISE_SOURCE_TREE_STATE}" \
      ai-blaise.citus.extension-bundle="${AI_BLAISE_EXTENSION_BUNDLE_VERSION}" \
      ai-blaise.citus.bundle1.contract-lock="images/citus-pg-overlay/bundle1-source-build.lock.tsv" \
      ai-blaise.citus.bundle1.evidence-scope="source-build-subset-no-complete-initdb" \
      ai-blaise.citus.bundle1.full-initdb-path="false" \
      ai-blaise.citus.bundle1.citus.tag="${CITUS_TAG}" \
      ai-blaise.citus.bundle1.pgsodium.tag="${PGSODIUM_TAG}" \
      ai-blaise.citus.bundle1.pgsodium.ref="${PGSODIUM_REF}" \
      ai-blaise.citus.bundle1.topn.tag="${TOPN_TAG}" \
      ai-blaise.citus.bundle1.topn.ref="${TOPN_REF}" \
      ai-blaise.citus.bundle1.pg_search.tag="${PG_SEARCH_TAG}" \
      ai-blaise.citus.bundle1.pg_search.ref="${PG_SEARCH_REF}" \
      ai-blaise.citus.bundle1.pg_jsonschema.tag="${PG_JSONSCHEMA_TAG}" \
      ai-blaise.citus.bundle1.pg_jsonschema.ref="${PG_JSONSCHEMA_REF}" \
      ai-blaise.citus.bundle1.plrust.tag="${PLRUST_TAG}" \
      ai-blaise.citus.bundle1.plrust.ref="${PLRUST_REF}" \
      ai-blaise.citus.bundle1.plrust.status="alpha-upstream-pg17-blocked" \
      ai-blaise.citus.bundle1.plv8.tag="${PLV8_TAG}" \
      ai-blaise.citus.bundle1.plv8.ref="${PLV8_REF}" \
      ai-blaise.citus.bundle1.pg_graphql.tag="${PG_GRAPHQL_TAG}" \
      ai-blaise.citus.bundle1.pg_graphql.ref="${PG_GRAPHQL_REF}"
ENV DEBIAN_FRONTEND=noninteractive
RUN set -eux; \
    apt-get update; \
    apt-get install -y --no-install-recommends libsodium23 libicu72 libstdc++6; \
    apt-get clean; \
    rm -rf /var/lib/apt/lists/*; \
    install -d -m 0755 /etc/postgresql/ai-blaise \
      /usr/local/share/ai-blaise/citus \
      /usr/local/share/ai-blaise/citus/extensions \
      "/usr/share/postgresql/${PG_MAJOR}/extension"
COPY images/citus-pg-overlay/extension-manifest.tsv /usr/local/share/ai-blaise/citus/extension-manifest.tsv
COPY images/citus-pg-overlay/bundle1-source-build.lock.tsv /usr/local/share/ai-blaise/citus/bundle1-source-build.lock.tsv
COPY images/citus-pg-overlay/shared-preload-libraries.conf /usr/local/share/ai-blaise/citus/shared-preload-libraries.conf
COPY images/citus-pg-overlay/extensions/ai_blaise_citus-upgrade-manifest.tsv /usr/local/share/ai-blaise/citus/ai_blaise_citus-upgrade-manifest.tsv
COPY images/citus-pg-overlay/extensions/ai_blaise_citus.control /usr/local/share/ai-blaise/citus/extensions/ai_blaise_citus.control
COPY images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.0.sql /usr/local/share/ai-blaise/citus/extensions/ai_blaise_citus--0.1.0.sql
COPY images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.0--0.1.1.sql /usr/local/share/ai-blaise/citus/extensions/ai_blaise_citus--0.1.0--0.1.1.sql
COPY images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.1--0.1.0.sql /usr/local/share/ai-blaise/citus/extensions/ai_blaise_citus--0.1.1--0.1.0.sql
COPY images/citus-pg-overlay/extensions/pg_warm.control /usr/local/share/ai-blaise/citus/extensions/pg_warm.control
COPY images/citus-pg-overlay/extensions/pg_warm--0.1.0.sql /usr/local/share/ai-blaise/citus/extensions/pg_warm--0.1.0.sql
COPY images/citus-pg-overlay/bin/pgsodium_getkey /usr/local/share/ai-blaise/citus/pgsodium_getkey
RUN set -eux; \
    cp /usr/local/share/ai-blaise/citus/shared-preload-libraries.conf /etc/postgresql/ai-blaise/shared-preload-libraries.conf; \
    install -m 0755 /usr/local/share/ai-blaise/citus/pgsodium_getkey "/usr/share/postgresql/${PG_MAJOR}/extension/pgsodium_getkey"; \
    cp /usr/local/share/ai-blaise/citus/extensions/ai_blaise_citus.control "/usr/share/postgresql/${PG_MAJOR}/extension/ai_blaise_citus.control"; \
    cp /usr/local/share/ai-blaise/citus/extensions/ai_blaise_citus--0.1.0.sql "/usr/share/postgresql/${PG_MAJOR}/extension/ai_blaise_citus--0.1.0.sql"; \
    cp /usr/local/share/ai-blaise/citus/extensions/ai_blaise_citus--0.1.0--0.1.1.sql "/usr/share/postgresql/${PG_MAJOR}/extension/ai_blaise_citus--0.1.0--0.1.1.sql"; \
    cp /usr/local/share/ai-blaise/citus/extensions/ai_blaise_citus--0.1.1--0.1.0.sql "/usr/share/postgresql/${PG_MAJOR}/extension/ai_blaise_citus--0.1.1--0.1.0.sql"; \
    test -s /usr/local/share/ai-blaise/citus/ai_blaise_citus-upgrade-manifest.tsv; \
    test -s /usr/local/share/ai-blaise/citus/bundle1-source-build.lock.tsv; \
    cp /usr/local/share/ai-blaise/citus/extensions/pg_warm.control "/usr/share/postgresql/${PG_MAJOR}/extension/pg_warm.control"; \
    cp /usr/local/share/ai-blaise/citus/extensions/pg_warm--0.1.0.sql "/usr/share/postgresql/${PG_MAJOR}/extension/pg_warm--0.1.0.sql"; \
    sed -n 1,120p /usr/local/share/ai-blaise/citus/extension-manifest.tsv >/usr/local/share/ai-blaise/citus/required-extension-preview.tsv
HEALTHCHECK --interval=30s --timeout=5s --start-period=30s --retries=3 \
  CMD pg_isready -U "${POSTGRES_USER:-postgres}" || exit 1

FROM bundle1-source-runtime AS bundle1-final-light
COPY --from=build-pgsodium      /out/ /
COPY --from=build-topn          /out/ /
COPY --from=build-pg-jsonschema /out/ /
COPY --from=build-pg-graphql    /out/ /
COPY --from=build-citus         /out/ /

FROM bundle1-final-light AS bundle1-final-full
COPY --from=build-pg-search /out/ /
COPY --from=build-plv8      /out/ /

FROM ${BASE_IMAGE} AS bundle1-contract
ARG PG_MAJOR
ARG AI_BLAISE_EXTENSION_BUNDLE_VERSION
ARG AI_BLAISE_SOURCE_GIT_SHA
ARG AI_BLAISE_SOURCE_TREE_STATE
LABEL org.opencontainers.image.source="https://github.com/ai-blaise/citus" \
      org.opencontainers.image.title="ai-blaise/citus PostgreSQL overlay" \
      org.opencontainers.image.description="PostgreSQL operand image contract for Citus, TimescaleDB, companion, and bundled extensions" \
      ai-blaise.citus.pg-major="${PG_MAJOR}" \
      ai-blaise.citus.source-git-sha="${AI_BLAISE_SOURCE_GIT_SHA}" \
      ai-blaise.citus.source-tree-state="${AI_BLAISE_SOURCE_TREE_STATE}" \
      ai-blaise.citus.extension-bundle="${AI_BLAISE_EXTENSION_BUNDLE_VERSION}" \
      ai-blaise.citus.bundle1.contract-lock="images/citus-pg-overlay/bundle1-source-build.lock.tsv" \
      ai-blaise.citus.bundle1.evidence-scope="manifest-initdb-contract-no-binary-bundle" \
      ai-blaise.citus.bundle1.full-initdb-path="unverified" \
      ai-blaise.citus.pg-core-patches="patches/postgres/series" \
      ai-blaise.citus.pg-core-patch-targets="logical-commit-clock per-subtrans-commit-ts"
COPY images/citus-pg-overlay/extension-manifest.tsv /usr/local/share/ai-blaise/citus/extension-manifest.tsv
COPY images/citus-pg-overlay/bundle1-source-build.lock.tsv /usr/local/share/ai-blaise/citus/bundle1-source-build.lock.tsv
COPY images/citus-pg-overlay/shared-preload-libraries.conf /usr/local/share/ai-blaise/citus/shared-preload-libraries.conf
COPY images/citus-pg-overlay/extensions/ai_blaise_citus.control /usr/local/share/ai-blaise/citus/extensions/ai_blaise_citus.control
COPY images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.0.sql /usr/local/share/ai-blaise/citus/extensions/ai_blaise_citus--0.1.0.sql
COPY images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.0--0.1.1.sql /usr/local/share/ai-blaise/citus/extensions/ai_blaise_citus--0.1.0--0.1.1.sql
COPY images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.1--0.1.0.sql /usr/local/share/ai-blaise/citus/extensions/ai_blaise_citus--0.1.1--0.1.0.sql
COPY images/citus-pg-overlay/extensions/pg_warm.control /usr/local/share/ai-blaise/citus/extensions/pg_warm.control
COPY images/citus-pg-overlay/extensions/pg_warm--0.1.0.sql /usr/local/share/ai-blaise/citus/extensions/pg_warm--0.1.0.sql
COPY images/citus-pg-overlay/initdb.d/00-ai-blaise-extensions.sql /docker-entrypoint-initdb.d/00-ai-blaise-extensions.sql
RUN set -eux; \
    install -d -m 0755 /etc/postgresql/ai-blaise; \
    install -d -m 0755 "/usr/share/postgresql/${PG_MAJOR}/extension"; \
    cp /usr/local/share/ai-blaise/citus/shared-preload-libraries.conf /etc/postgresql/ai-blaise/shared-preload-libraries.conf; \
    test -s /usr/local/share/ai-blaise/citus/bundle1-source-build.lock.tsv; \
    cp /usr/local/share/ai-blaise/citus/extensions/ai_blaise_citus.control "/usr/share/postgresql/${PG_MAJOR}/extension/ai_blaise_citus.control"; \
    cp /usr/local/share/ai-blaise/citus/extensions/ai_blaise_citus--0.1.0.sql "/usr/share/postgresql/${PG_MAJOR}/extension/ai_blaise_citus--0.1.0.sql"; \
    cp /usr/local/share/ai-blaise/citus/extensions/ai_blaise_citus--0.1.0--0.1.1.sql "/usr/share/postgresql/${PG_MAJOR}/extension/ai_blaise_citus--0.1.0--0.1.1.sql"; \
    cp /usr/local/share/ai-blaise/citus/extensions/ai_blaise_citus--0.1.1--0.1.0.sql "/usr/share/postgresql/${PG_MAJOR}/extension/ai_blaise_citus--0.1.1--0.1.0.sql"; \
    cp /usr/local/share/ai-blaise/citus/extensions/pg_warm.control "/usr/share/postgresql/${PG_MAJOR}/extension/pg_warm.control"; \
    cp /usr/local/share/ai-blaise/citus/extensions/pg_warm--0.1.0.sql "/usr/share/postgresql/${PG_MAJOR}/extension/pg_warm--0.1.0.sql"; \
    sed -n 1,120p /usr/local/share/ai-blaise/citus/extension-manifest.tsv >/usr/local/share/ai-blaise/citus/required-extension-preview.tsv
HEALTHCHECK --interval=30s --timeout=5s --start-period=30s --retries=3 \
  CMD pg_isready -U "${POSTGRES_USER:-postgres}" || exit 1
