#!/bin/sh
# Seed pgdata once, then hand off to s6-overlay.
#
# The entrypoint no longer wipes pgdata. It copies the baked cluster into
# the volume only when PG_VERSION is absent: a new or empty volume, or a
# bind mount that arrives empty. An existing cluster is kept as it is, so
# accounts and characters survive `docker restart` and image updates
# (owner decision D2). Removing the named volume by hand is the only full
# wipe.
#
# Whether the existing `sgw` database is kept, adopted, rebuilt or restored
# is decided per boot by the s6 oneshot `db-sync` (docker/db-sync.sh), which
# runs after postgres starts and before cimmeria-server.
#
# The fallback at /var/lib/postgresql/initial-data is an immutable COPY of
# the baked pgdata layer, kept separate so the entrypoint always has a clean
# source to seed from.
set -eu

PGDATA="${PGDATA:-/var/lib/postgresql/data}"
FALLBACK="/var/lib/postgresql/initial-data"

if [ ! -f "$PGDATA/PG_VERSION" ]; then
    if [ ! -s "${FALLBACK}/PG_VERSION" ]; then
        echo "[cimmeria-entrypoint] FATAL: fallback pgdata at ${FALLBACK} is missing or empty" >&2
        exit 1
    fi
    echo "[cimmeria-entrypoint] empty pgdata, seeding from ${FALLBACK}"
    mkdir -p "$PGDATA"
    cp -a "${FALLBACK}/." "${PGDATA}/"
    chown -R postgres:postgres "${PGDATA}"
else
    echo "[cimmeria-entrypoint] keeping existing pgdata"
fi

# Always re-assert pgdata mode = 0700. Postgres refuses to start
# otherwise (it requires 0700 or 0750), and the directory mode can
# drift on us — bind-mounted host directories arrive with the host's
# umask, named volumes inherit the mountpoint's mode, and Docker's
# cross-stage COPY doesn't always preserve directory mode.
chmod 700 "${PGDATA}"

# /opt/cimmeria/logs -> /var/log/cimmeria. A host bind mount there arrives
# owned by root, and the server (uid cimmeria) could not write its logs.
chown cimmeria:cimmeria /var/log/cimmeria

# Backup directory for db-sync (owned by postgres, a VOLUME in the image).
mkdir -p /var/backups/cimmeria && chown postgres:postgres /var/backups/cimmeria

# Discord config from the environment (docker/compose.discord.yml). The
# environment survives watchtower's recreate; a compose `configs:` copy
# does not. Unset means "leave config/discord.toml alone", so a
# bind-mounted file keeps working.
if [ -n "${DISCORD_CONFIG_TOML:-}" ]; then
    mkdir -p /opt/cimmeria/config
    printf '%s\n' "${DISCORD_CONFIG_TOML}" > /opt/cimmeria/config/discord.toml
    chown cimmeria:cimmeria /opt/cimmeria/config/discord.toml
    chmod 0440 /opt/cimmeria/config/discord.toml
    echo "[cimmeria-entrypoint] wrote config/discord.toml from DISCORD_CONFIG_TOML"
fi

# Hand control to s6-overlay v3, which brings up postgres, db-sync and the
# server in dependency order and reaps zombies.
exec /init "$@"
