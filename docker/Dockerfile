# syntax=docker/dockerfile:1.7

# Three-stage build for the self-contained Cimmeria server image:
#   builder  → compiles cimmeria-server against the Rust workspace.
#   db-init  → initialises a Postgres data directory with schema + seeds.
#   runtime  → carries the binary, cooked data, the loaded pgdata, and
#              s6-overlay supervising postgres + cimmeria-server together.
#
# The runtime image carries zero .sql files and zero Rust source — the
# DB is identity-baked at build time and the runtime never re-runs psql.

ARG POSTGRES_IMAGE=postgres:17.9-bookworm
ARG RUST_IMAGE=rust:1-bookworm
ARG RECAST_VERSION=1.6.0
ARG S6_OVERLAY_VERSION=3.2.0.2

# ─── Stage 1: builder ────────────────────────────────────────────
FROM ${RUST_IMAGE} AS builder

ARG RECAST_VERSION

# Native deps the workspace needs at link/build time:
#   clang        — used by cc-rs when compiling Detour C++ sources.
#   mold         — fast linker (matches CI's test.yml).
#   pkg-config   — sqlx + reqwest discovery.
#   libssl-dev   — link-time OpenSSL for native-tls.
#   curl/unzip   — to hydrate external/recast.
RUN apt-get update && apt-get install -y --no-install-recommends \
        clang \
        mold \
        pkg-config \
        libssl-dev \
        curl \
        unzip \
        ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build

# Hydrate external/recast — cimmeria-entity's build.rs compiles the
# Detour sources directly. Pin matches the version in test.yml so the
# image's binary is byte-comparable to CI builds.
RUN mkdir -p external \
    && curl -sL --retry 3 -o /tmp/recast.zip \
        "https://github.com/recastnavigation/recastnavigation/archive/refs/tags/v${RECAST_VERSION}.zip" \
    && unzip -q /tmp/recast.zip -d /tmp/recast_extract \
    && mv "/tmp/recast_extract/recastnavigation-${RECAST_VERSION}" external/recast \
    && rm -rf /tmp/recast.zip /tmp/recast_extract \
    && test -f external/recast/Detour/Source/DetourAssert.cpp

# Workspace skeleton. Cargo needs every member's manifest present even
# when we only build `-p cimmeria-server`, so the tauri/tools members
# come along for the manifest parse but are never compiled.
#
# .cargo/config.toml carries the `[target.x86_64-unknown-linux-gnu]`
# clang + `-fuse-ld=mold` linker config; without it the apt-installed
# clang and mold above are unused and the build falls back to gcc + ld,
# diverging from CI's behaviour and the workspace's tuning notes.
COPY Cargo.toml Cargo.lock ./
COPY .cargo ./.cargo
COPY crates ./crates
COPY entities ./entities
COPY src-tauri ./src-tauri
COPY tools ./tools

ENV CARGO_INCREMENTAL=0

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/build/target \
    cargo build -p cimmeria-server --release \
 && cp target/release/cimmeria-server /tmp/cimmeria-server


# ─── Stage 2: db-init ────────────────────────────────────────────
# Initialise pgdata against a stopped Postgres, load the full schema +
# seed data, then capture the resulting directory. The runtime stage
# COPYs this directory and never sees the .sql files.
FROM ${POSTGRES_IMAGE} AS db-init

ENV POSTGRES_USER=w-testing \
    POSTGRES_PASSWORD=w-testing \
    POSTGRES_DB=sgw \
    PGDATA=/var/lib/postgresql/data \
    LANG=C.UTF-8

COPY db /tmp/db

RUN set -eux; \
    mkdir -p "$PGDATA"; \
    chown -R postgres:postgres "$PGDATA" /tmp/db; \
    su postgres -s /bin/bash -c "echo $POSTGRES_PASSWORD > /tmp/pwfile && \
        initdb --username=$POSTGRES_USER --pwfile=/tmp/pwfile \
               --locale=C.UTF-8 --encoding=UTF8 -D $PGDATA && \
        rm -f /tmp/pwfile"; \
    su postgres -s /bin/bash -c "pg_ctl -D $PGDATA -o '-c listen_addresses=' -w start"; \
    # All psql calls must pass -U $POSTGRES_USER. initdb created exactly one
    # superuser ($POSTGRES_USER, not "postgres"); without -U, psql defaults
    # to the OS user (postgres) and the connection fails with
    # `role "postgres" does not exist` since no such Postgres role was created.
    su postgres -s /bin/bash -c "psql -U \"$POSTGRES_USER\" -d postgres -v ON_ERROR_STOP=1 \
        -c \"CREATE DATABASE $POSTGRES_DB OWNER \\\"$POSTGRES_USER\\\";\""; \
    su postgres -s /bin/bash -c "cd /tmp/db && \
        psql -U \"$POSTGRES_USER\" -d $POSTGRES_DB -v ON_ERROR_STOP=1 -f /tmp/db/database.sql"; \
    su postgres -s /bin/bash -c "pg_ctl -D $PGDATA -m fast -w stop"; \
    rm -rf /tmp/db


# ─── Stage 3: runtime ────────────────────────────────────────────
FROM ${POSTGRES_IMAGE} AS runtime

ARG S6_OVERLAY_VERSION

LABEL org.opencontainers.image.title="cimmeria-server" \
      org.opencontainers.image.description="Self-contained Cimmeria (Stargate Worlds emulator) server with bundled Postgres + game data." \
      org.opencontainers.image.source="https://github.com/sandboxservers/cimmeria" \
      org.opencontainers.image.licenses="MIT"

# Runtime native deps:
#   xz-utils, ca-certificates — needed to extract s6-overlay.
#   curl                       — required for the s6-overlay download below.
RUN apt-get update && apt-get install -y --no-install-recommends \
        xz-utils \
        ca-certificates \
        curl \
    && rm -rf /var/lib/apt/lists/*

# Install s6-overlay v3 — handles signals, zombie reaping, and the
# longrun service definitions under /etc/s6-overlay/s6-rc.d.
RUN set -eux; \
    arch="$(dpkg --print-architecture)"; \
    case "$arch" in \
        amd64) s6_arch=x86_64 ;; \
        arm64) s6_arch=aarch64 ;; \
        *) echo "Unsupported arch: $arch" >&2; exit 1 ;; \
    esac; \
    curl -sL --retry 3 \
        "https://github.com/just-containers/s6-overlay/releases/download/v${S6_OVERLAY_VERSION}/s6-overlay-noarch.tar.xz" \
        | tar -C / -Jxpf -; \
    curl -sL --retry 3 \
        "https://github.com/just-containers/s6-overlay/releases/download/v${S6_OVERLAY_VERSION}/s6-overlay-${s6_arch}.tar.xz" \
        | tar -C / -Jxpf -

# Layout: binary, cooked data, baked pgdata + fallback copy.
RUN mkdir -p /opt/cimmeria/bin /opt/cimmeria/data /var/log/cimmeria
COPY --from=builder /tmp/cimmeria-server /opt/cimmeria/bin/cimmeria-server
COPY data/cache  /opt/cimmeria/data/cache
COPY data/spaces /opt/cimmeria/data/spaces

# Pre-loaded pgdata in the canonical location, plus an immutable fallback
# that entrypoint.sh restores into bind-mounted empty volumes (Docker
# auto-copies into named volumes but not into bind mounts).
COPY --from=db-init --chown=postgres:postgres /var/lib/postgresql/data \
                                              /var/lib/postgresql/data
COPY --from=db-init --chown=postgres:postgres /var/lib/postgresql/data \
                                              /var/lib/postgresql/initial-data

# s6-rc service definitions.
COPY docker/s6/postgres          /etc/s6-overlay/s6-rc.d/postgres
COPY docker/s6/cimmeria-server   /etc/s6-overlay/s6-rc.d/cimmeria-server
RUN mkdir -p /etc/s6-overlay/s6-rc.d/user/contents.d \
 && touch    /etc/s6-overlay/s6-rc.d/user/contents.d/postgres \
             /etc/s6-overlay/s6-rc.d/user/contents.d/cimmeria-server \
 && chmod +x /etc/s6-overlay/s6-rc.d/postgres/run \
             /etc/s6-overlay/s6-rc.d/cimmeria-server/run \
             /etc/s6-overlay/s6-rc.d/cimmeria-server/finish

# Wrapper that self-heals empty bind-mount pgdata then hands off to s6.
COPY docker/entrypoint.sh /usr/local/bin/cimmeria-entrypoint
RUN chmod +x /usr/local/bin/cimmeria-entrypoint

# Defaults — every var is overridable at `docker run` time. See the
# server's documentation for the full reference. BASE_EXTERNAL must be
# overridden for non-localhost play.
ENV PGDATA=/var/lib/postgresql/data \
    POSTGRES_USER=w-testing \
    POSTGRES_PASSWORD=w-testing \
    POSTGRES_DB=sgw \
    AUTH_HOST=0.0.0.0 \
    AUTH_PORT=13001 \
    LOGON_PORT=8081 \
    BASE_HOST=0.0.0.0 \
    BASE_EXTERNAL=127.0.0.1 \
    BASE_PORT=32832 \
    CELL_PORT=50000 \
    ADMIN_PORT=8443 \
    # Libpq-style on purpose — orchestrator_postgres::ensure_postgresql_running
    # parses host= and port= tokens, not URL DSNs. A URL form would make the
    # auto-start probe default to localhost:5433 and emit spurious warnings
    # in the container even though sqlx itself accepts either form.
    DB_URL="host=127.0.0.1 port=5432 user=w-testing password=w-testing dbname=sgw" \
    DEVELOPER_MODE=true \
    RUST_LOG=info \
    S6_KEEP_ENV=1

# Ports actually bound by the Rust server. The legacy Python console
# (port 8989 in the original C++ BaseApp) is not implemented in Rust,
# so we don't EXPOSE it.
EXPOSE 13001 32832/udp 50000/udp 8081 8443

VOLUME ["/var/lib/postgresql/data", "/var/log/cimmeria"]

# Liveness probe: postgres accepts connections AND the auth TCP listener
# (the port game clients connect to) is bound. Bash's /dev/tcp gives us a
# zero-dep TCP probe without pulling in netcat/curl just for this.
HEALTHCHECK --interval=30s --timeout=10s --start-period=60s --retries=3 \
    CMD pg_isready -h 127.0.0.1 -U "${POSTGRES_USER}" -d "${POSTGRES_DB}" >/dev/null \
        && bash -c "exec 3<>/dev/tcp/127.0.0.1/${AUTH_PORT}" \
        || exit 1

ENTRYPOINT ["/usr/local/bin/cimmeria-entrypoint"]
