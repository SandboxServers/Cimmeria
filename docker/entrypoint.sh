#!/bin/sh
# Self-heal an empty bind-mounted pgdata then hand off to s6-overlay.
#
# Why: Docker auto-copies image contents into a NAMED volume on first
# run, so `-v cimmeria-data:/var/lib/postgresql/data` works out of the
# box. But if the user bind-mounts an empty host directory
# (`-v $(pwd)/pgdata:/var/lib/postgresql/data`) Docker will NOT copy the
# baked pgdata in — postgres would start against an empty directory and
# refuse to come up. We detect that case and restore from
# /var/lib/postgresql/initial-data, the immutable fallback baked next to
# the canonical pgdata at image build time.
set -eu

PGDATA="${PGDATA:-/var/lib/postgresql/data}"
FALLBACK="/var/lib/postgresql/initial-data"

if [ ! -s "${PGDATA}/PG_VERSION" ]; then
    echo "[cimmeria-entrypoint] empty pgdata detected — restoring from ${FALLBACK}"
    if [ ! -s "${FALLBACK}/PG_VERSION" ]; then
        echo "[cimmeria-entrypoint] FATAL: fallback pgdata at ${FALLBACK} is missing or empty" >&2
        exit 1
    fi
    cp -a "${FALLBACK}/." "${PGDATA}/"
    chown -R postgres:postgres "${PGDATA}"
fi

# Always re-assert pgdata mode = 0700. Postgres refuses to start
# otherwise (it requires 0700 or 0750), and the directory mode can
# drift on us — bind-mounted host directories arrive with the host's
# umask, named volumes inherit the mountpoint's mode, and Docker's
# cross-stage COPY doesn't always preserve directory mode.
chmod 700 "${PGDATA}"

# Hand control to s6-overlay v3, which brings up postgres and the
# server in dependency order and reaps zombies.
exec /init "$@"
