# syntax=docker/dockerfile:1.7

# Three-stage build for the self-contained Cimmeria server image:
#   builder  → compiles cimmeria-server against the Rust workspace.
#   db-init  → initialises a Postgres data directory with schema + seeds.
#   runtime  → carries the binary, cooked data, the loaded pgdata, and
#              s6-overlay supervising postgres + cimmeria-server together.
#
# The runtime image carries zero .sql files and zero Rust source — the
# DB is identity-baked at build time and the runtime never re-runs psql.

# Base images pinned by digest, not just tag. Tags are mutable on
# Docker Hub (a compromised account or registry could repoint
# postgres:17.9-bookworm at a malicious image), so the digest is the
# only true integrity guarantee. Dependabot watches these via
# .github/dependabot.yml and opens PRs when upstream rebuilds — without
# that automation, digest-pinning would mean shipping known-CVE base
# images forever.
ARG POSTGRES_IMAGE=postgres:17.9-bookworm@sha256:47f917f7409eacd22fc5dfb1dee634e1b55cf0c01d1a7eb701be2227a03e0641
ARG RUST_IMAGE=rust:1-bookworm@sha256:503651ea31e66ecb74623beabde781059a5978df1595a9e8ed03974d5fec1bf0
ARG RECAST_VERSION=1.6.0
# SHA256 of github.com/recastnavigation/recastnavigation/archive/refs/tags/v${RECAST_VERSION}.zip.
# Verified at build time; bumping RECAST_VERSION requires bumping this too
# (the build will fail loudly with a checksum mismatch if you forget).
ARG RECAST_SHA256=8b50c62177249554b226514b307e6c529934a62b3926822777bf6abb8906a155
ARG S6_OVERLAY_VERSION=3.2.0.2
# SHA256s from the upstream `*.sha256` release assets. Same bump-together
# discipline as RECAST_SHA256 above. amd64 + arm64 are both checked so a
# future multi-arch build doesn't silently skip verification on one side.
ARG S6_OVERLAY_NOARCH_SHA256=6dbcde158a3e78b9bb141d7bcb5ccb421e563523babbe2c64470e76f4fd02dae
ARG S6_OVERLAY_AMD64_SHA256=59289456ab1761e277bd456a95e737c06b03ede99158beb24f12b165a904f478
ARG S6_OVERLAY_ARM64_SHA256=8b22a2eaca4bf0b27a43d36e65c89d2701738f628d1abd0cea5569619f66f785

# ─── Stage 1: builder ────────────────────────────────────────────
FROM ${RUST_IMAGE} AS builder

ARG RECAST_VERSION
ARG RECAST_SHA256

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
    && echo "${RECAST_SHA256}  /tmp/recast.zip" | sha256sum -c - \
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
ARG S6_OVERLAY_NOARCH_SHA256
ARG S6_OVERLAY_AMD64_SHA256
ARG S6_OVERLAY_ARM64_SHA256

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
# longrun service definitions under /etc/s6-overlay/s6-rc.d. Each
# tarball is downloaded to disk, sha256-verified against the pinned
# digest, and only then extracted; the previous `curl | tar` pipe had
# no integrity check.
RUN set -eux; \
    arch="$(dpkg --print-architecture)"; \
    case "$arch" in \
        amd64) s6_arch=x86_64; arch_sha="${S6_OVERLAY_AMD64_SHA256}" ;; \
        arm64) s6_arch=aarch64; arch_sha="${S6_OVERLAY_ARM64_SHA256}" ;; \
        *) echo "Unsupported arch: $arch" >&2; exit 1 ;; \
    esac; \
    curl -sL --retry 3 \
        -o /tmp/s6-noarch.tar.xz \
        "https://github.com/just-containers/s6-overlay/releases/download/v${S6_OVERLAY_VERSION}/s6-overlay-noarch.tar.xz"; \
    echo "${S6_OVERLAY_NOARCH_SHA256}  /tmp/s6-noarch.tar.xz" | sha256sum -c -; \
    tar -C / -Jxpf /tmp/s6-noarch.tar.xz; \
    curl -sL --retry 3 \
        -o /tmp/s6-arch.tar.xz \
        "https://github.com/just-containers/s6-overlay/releases/download/v${S6_OVERLAY_VERSION}/s6-overlay-${s6_arch}.tar.xz"; \
    echo "${arch_sha}  /tmp/s6-arch.tar.xz" | sha256sum -c -; \
    tar -C / -Jxpf /tmp/s6-arch.tar.xz; \
    rm -f /tmp/s6-noarch.tar.xz /tmp/s6-arch.tar.xz

# Dedicated unprivileged user the server runs as. Postgres has its own
# `postgres` user from the base image; the server should not share it
# (different blast radius, different writable surface). Fixed UID/GID
# so volume permissions are predictable across image rebuilds.
RUN groupadd --system --gid 1001 cimmeria \
 && useradd --system --uid 1001 --gid 1001 \
            --home-dir /opt/cimmeria --shell /usr/sbin/nologin cimmeria

# Layout: binary, cooked data, baked pgdata + fallback copy.
RUN mkdir -p /opt/cimmeria/bin /opt/cimmeria/data /var/log/cimmeria
COPY --from=builder --chown=cimmeria:cimmeria /tmp/cimmeria-server /opt/cimmeria/bin/cimmeria-server
COPY --chown=cimmeria:cimmeria data/cache  /opt/cimmeria/data/cache
COPY --chown=cimmeria:cimmeria data/spaces /opt/cimmeria/data/spaces
# The server creates `logs/` relative to its CWD (/opt/cimmeria) at
# startup; /var/log/cimmeria is reserved for future file-logging when
# we wire it up. Both need to be writable as the cimmeria user.
RUN chown -R cimmeria:cimmeria /opt/cimmeria /var/log/cimmeria

# Pre-loaded pgdata in the canonical location, plus an immutable fallback
# that entrypoint.sh restores into bind-mounted empty volumes (Docker
# auto-copies into named volumes but not into bind mounts).
COPY --from=db-init --chown=postgres:postgres /var/lib/postgresql/data \
                                              /var/lib/postgresql/data
COPY --from=db-init --chown=postgres:postgres /var/lib/postgresql/data \
                                              /var/lib/postgresql/initial-data
# Postgres refuses to start unless the data directory is 0700 or 0750
# (`FATAL: data directory ... has invalid permissions`). Cross-stage
# COPY preserves ownership via --chown but not directory mode — initdb
# originally produced 0700, but the COPY result lands at the runtime
# stage's default. Re-assert 0700 here so the image is good out of the
# box; the entrypoint also re-asserts at boot to cover bind-mount cases.
RUN chmod 700 /var/lib/postgresql/data /var/lib/postgresql/initial-data

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

# Internal env — baked into pgdata at build time. Do NOT override at
# `docker run` time: pgdata was initdb'd as $POSTGRES_USER owning
# $POSTGRES_DB with $POSTGRES_PASSWORD, and changing any of these on
# a running container won't re-initialise the DB — it will just
# desync the s6 wait loop / healthcheck (which read these vars to
# call pg_isready) from the actual cluster contents. Reset by
# removing the named volume, not by overriding env.
ENV PGDATA=/var/lib/postgresql/data \
    POSTGRES_USER=w-testing \
    POSTGRES_PASSWORD=w-testing \
    POSTGRES_DB=sgw

# User-overridable env. Defaults match a localhost / single-host run;
# BASE_EXTERNAL must be overridden for any client that isn't on the
# same machine as the container. See docs/operations/container.md for
# the full reference.
ENV AUTH_HOST=0.0.0.0 \
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
