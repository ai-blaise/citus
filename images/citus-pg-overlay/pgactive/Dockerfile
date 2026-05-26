FROM postgres:17-bookworm AS pgactive-build
ENV DEBIAN_FRONTEND=noninteractive
RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential autoconf automake libtool git curl ca-certificates \
    flex bison \
    libssl-dev libxml2-dev libxslt-dev libreadline-dev libicu-dev \
    libkrb5-dev liblz4-dev libzstd-dev libcurl4-openssl-dev \
    libpam0g-dev libldap2-dev libsystemd-dev libselinux1-dev \
    zlib1g-dev libperl-dev libpython3-dev libxml2-dev libxslt-dev \
    uuid-dev tcl-dev gettext pkg-config postgresql-server-dev-17 libpq-dev
# Postgres ships static libs under pkglibdir, but pgactive's Makefile links
# via libdir. Bridge the path so -lpgcommon / -lpgport resolve.
RUN cp /usr/lib/postgresql/17/lib/libpgcommon.a /usr/lib/postgresql/17/lib/libpgport.a /usr/lib/postgresql/17/lib/libpgfeutils.a /usr/lib/x86_64-linux-gnu/ && \
    cp /usr/lib/postgresql/17/lib/libpgport.a /usr/lib/x86_64-linux-gnu/
RUN cd /tmp && git clone --depth=1 https://github.com/aws/pgactive.git pgactive && \
    cd pgactive && \
    PG_CONFIG=/usr/bin/pg_config ./autogen.sh && \
    ./configure --with-pgconfig=/usr/bin/pg_config && \
    make -j4 USE_PGXS=1 && \
    make USE_PGXS=1 install && \
    echo 'BUILD-DONE'

FROM postgres:17-bookworm
COPY --from=pgactive-build /usr/lib/postgresql/17/lib/pgactive.so /usr/lib/postgresql/17/lib/
COPY --from=pgactive-build /usr/share/postgresql/17/extension/pgactive.control /usr/share/postgresql/17/extension/
COPY --from=pgactive-build /usr/share/postgresql/17/extension/ /usr/share/postgresql/17/extension/
