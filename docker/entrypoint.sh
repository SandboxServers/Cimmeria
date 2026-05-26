#!/bin/sh
# Reseed pgdata on every container start, then hand off to s6-overlay.
#
# # Why unconditional reseed
#
# The colo deploy intent is "fresh DB on every watchtower swap." The
# Dockerfile bakes pgdata into the image, declares `VOLUME` on
# /var/lib/postgresql/data, and the compose.yml leaves that volume
# unmanaged so Docker creates an anonymous one per container. The
# original assumption was that `WATCHTOWER_REMOVE_VOLUMES=true` would
# drop the anonymous volume on each swap.
#
# It doesn't, because of how watchtower swaps containers:
#   1. Inspect old container → finds anonymous volume V mounted at
#      /var/lib/postgresql/data.
#   2. Create new container with the same mount spec → V is now
#      attached to the new container.
#   3. Stop + `docker rm -v` old container → Docker refuses to delete V
#      because the new container is still using it.
#
# Net effect: the anonymous volume survives every swap, the prior
# `if [ ! -s "${PGDATA}/PG_VERSION" ]` guard never trips, and the
# database persists across deploys. Operators saw this on the colo at
# 2026-05-26: the volume's CreatedAt predated every watchtower swap
# in the previous 24 hours.
#
# Owning the reseed at the entrypoint sidesteps the volume-mechanics
# problem entirely: regardless of whether the volume is anonymous,
# named, project-scoped, bind-mounted, or persisted by the container
# runtime, the contents get reset to the image-baked fallback on
# every start. The trade-off is that a `docker restart cimmeria`
# (no swap) also wipes the DB — but the original design intent was
# already "ephemeral DB, fresh every deploy," so that's a non-issue.
#
# # Why a fallback path at all
#
# Two scenarios where the entrypoint's mount differs from the baked
# pgdata location:
#   - Bind-mounted host directory (`-v ./pgdata:/var/lib/postgresql/data`):
#     Docker does NOT auto-copy image contents into bind mounts, so
#     the directory arrives empty and postgres would refuse to start.
#   - Named volume populated by a non-Cimmeria image, or a stale
#     anonymous volume from a watchtower swap (the case above).
#
# The fallback at /var/lib/postgresql/initial-data is an immutable
# COPY of the same baked pgdata layer, kept separate so the entrypoint
# always has a clean source to copy from.
set -eu

PGDATA="${PGDATA:-/var/lib/postgresql/data}"
FALLBACK="/var/lib/postgresql/initial-data"

if [ ! -s "${FALLBACK}/PG_VERSION" ]; then
    echo "[cimmeria-entrypoint] FATAL: fallback pgdata at ${FALLBACK} is missing or empty" >&2
    exit 1
fi

echo "[cimmeria-entrypoint] reseeding pgdata from ${FALLBACK}"
# Clear current contents (including dot-files); use -- to guard against
# a future PGDATA value that starts with '-'. The ${PGDATA:?} guard
# fails the script if PGDATA is somehow empty rather than `rm -rf /*`.
rm -rf -- "${PGDATA:?}"/* 2>/dev/null || true
# Match-anything glob for dot-files separately — POSIX sh doesn't have
# a single token that covers both, and a bare `.*` would match `.` and
# `..` (catastrophe). The `?` is shorthand for "any single char" so
# `.?*` matches `.X`/`..X`/etc but not the bare directory refs.
rm -rf -- "${PGDATA:?}"/.?* 2>/dev/null || true
cp -a "${FALLBACK}/." "${PGDATA}/"
chown -R postgres:postgres "${PGDATA}"

# Always re-assert pgdata mode = 0700. Postgres refuses to start
# otherwise (it requires 0700 or 0750), and the directory mode can
# drift on us — bind-mounted host directories arrive with the host's
# umask, named volumes inherit the mountpoint's mode, and Docker's
# cross-stage COPY doesn't always preserve directory mode.
chmod 700 "${PGDATA}"

# Hand control to s6-overlay v3, which brings up postgres and the
# server in dependency order and reaps zombies.
exec /init "$@"
