#!/bin/sh
# Boot-time database decision for the cimmeria image. Runs as the s6 oneshot
# `db-sync` after postgres starts and before cimmeria-server. Each start logs
# one decision line (restore, first-boot, adopt, rebuild or skip) and exits 0,
# or logs an ERROR and exits 1 with `sgw` untouched.
#
# Owner decision D2: accounts and characters survive a restart or an image
# update. `sgw` is only ever renamed here, never dropped or wiped.
#
# Rules, first match wins:
#   1. CIMMERIA_DB_RESTORE names a backup not yet in restore_done: restore
#   2. db_state.state = baked: first-boot (mark live, nothing to carry)
#   3. no cimmeria_meta schema: adopt (backup, then rebuild keeping players)
#   4. CIMMERIA_DB_FORCE_REBUILD, or image_id differs: rebuild (backup first)
#   5. otherwise: skip
set -eu

# Postgres owns the database, so run as its OS user. The postgres run script
# drops privileges with s6-setuidgid the same way.
if [ "$(id -u)" = 0 ]; then
    exec s6-setuidgid postgres sh "$0" "$@"
fi

log() { echo "[db-sync] $*"; }

DB=sgw
IDFILE=/opt/cimmeria/db/image-id
BACKUP_DIR=/var/backups/cimmeria
KEEP="${CIMMERIA_DB_BACKUP_KEEP:-10}"
FORCE="${CIMMERIA_DB_FORCE_REBUILD:-}"
RESTORE="${CIMMERIA_DB_RESTORE:-}"

case "$KEEP" in
    ''|*[!0-9]*) log "ERROR bad CIMMERIA_DB_BACKUP_KEEP"; exit 1 ;;
esac
if [ "$KEEP" -lt 1 ]; then
    log "ERROR bad CIMMERIA_DB_BACKUP_KEEP"
    exit 1
fi

NEW_ID=$(cat "$IDFILE")
export PGPASSWORD=w-testing
psqlq() { PGOPTIONS= psql -h 127.0.0.1 -U w-testing -d "$1" -tAXq -v ON_ERROR_STOP=1 -c "$2"; }

n=0
until pg_isready -h 127.0.0.1 -q; do
    n=$((n + 1))
    if [ "$n" -ge 120 ]; then
        log "ERROR postgres not ready after 120s"
        exit 1
    fi
    sleep 1
done

# Read the state. Each query tolerates a missing schema via to_regclass.
HAS_META=$(psqlq "$DB" "SELECT to_regclass('cimmeria_meta.db_state') IS NOT NULL")
STATE=""
CUR_ID=""
if [ "$HAS_META" = t ]; then
    ROW=$(psqlq "$DB" "SELECT coalesce(state, ''), coalesce(image_id, '') FROM cimmeria_meta.db_state")
    STATE=${ROW%%|*}
    CUR_ID=${ROW#*|}
fi

# Take a dump of the live sgw. Written as .part, then renamed (atomic).
# Fail closed: no backup means no rebuild or restore.
backup() {
    OLD=${CUR_ID:-pre-meta}
    if [ "$HAS_META" != t ]; then
        OLD=pre-meta
    fi
    NAME="sgw-$(date -u +%Y%m%dT%H%M%SZ)-${OLD}.dump"
    if ! pg_dump -Fc -h 127.0.0.1 -U w-testing -d "$DB" -f "$BACKUP_DIR/$NAME.part"; then
        rm -f "$BACKUP_DIR/$NAME.part"
        log "ERROR backup failed, sgw untouched"
        exit 1
    fi
    if ! mv "$BACKUP_DIR/$NAME.part" "$BACKUP_DIR/$NAME"; then
        log "ERROR backup rename failed, sgw untouched"
        exit 1
    fi
    log "backup $BACKUP_DIR/$NAME"
    ls -1t "$BACKUP_DIR"/sgw-*.dump | tail -n +$((KEEP + 1)) | while read -r f; do
        rm -f "$f"
    done
}

# Reuse the newest backup during a crash loop of the same failing image.
# Only when that image failed to rebuild and the newest backup is newer than
# the recorded failure.
reuse_backup() {
    FAILED=$(psqlq "$DB" "SELECT coalesce(last_failed_image_id, ''), coalesce(extract(epoch FROM last_failed_at)::bigint, 0) FROM cimmeria_meta.db_state") || return 1
    FAILED_ID=${FAILED%%|*}
    FAILED_EPOCH=${FAILED#*|}
    [ "$FAILED_ID" = "$NEW_ID" ] || return 1
    NEWEST=$(ls -1t "$BACKUP_DIR"/sgw-*.dump 2>/dev/null | head -n 1)
    [ -n "$NEWEST" ] || return 1
    # The failure is recorded after the backup was taken, so the failed boot
    # touches the backup it used (see the rebuild failure path): mtime >= failure.
    [ "$(stat -c %Y "$NEWEST")" -ge "$FAILED_EPOCH" ] || return 1
    NAME=${NEWEST##*/}
    log "reusing backup $NEWEST"
}

# True when the live sgw already has this restore file in restore_done.
restore_marked() {
    [ "$(psqlq "$DB" "SELECT to_regclass('cimmeria_meta.restore_done') IS NOT NULL")" = t ] || return 1
    [ "$(psqlq "$DB" "SELECT count(*) FROM cimmeria_meta.restore_done WHERE file = '$SQL_FILE'")" != 0 ]
}

# Restore FILE into sgw_restore, then swap it in as sgw (old sgw -> sgw_prev).
# Any failure drops sgw_restore through the EXIT trap and leaves sgw as it was.
restore_db() {
    trap 'rc=$?; if [ "$rc" -ne 0 ]; then log "ERROR restore failed, sgw untouched"; psqlq postgres "DROP DATABASE IF EXISTS sgw_restore" >/dev/null 2>&1 || true; fi' EXIT
    psqlq postgres "SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE datname = '$DB' AND pid <> pg_backend_pid()" >/dev/null
    backup
    psqlq postgres "DROP DATABASE IF EXISTS sgw_restore"
    psqlq postgres "CREATE DATABASE sgw_restore"
    pg_restore --no-owner -h 127.0.0.1 -U w-testing -d sgw_restore "$FILE"
    psqlq sgw_restore "CREATE SCHEMA IF NOT EXISTS cimmeria_meta; CREATE TABLE IF NOT EXISTS cimmeria_meta.restore_done (file text primary key, at timestamptz default now()); INSERT INTO cimmeria_meta.restore_done (file) VALUES ('$SQL_FILE') ON CONFLICT DO NOTHING"
    # The restore took a while: drop any session (healthcheck) that reconnected to sgw
    # since the first terminate, or the rename below fails with "being accessed".
    psqlq postgres "SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE datname = '$DB' AND pid <> pg_backend_pid()" >/dev/null
    psql -h 127.0.0.1 -U w-testing -d postgres -X -q -v ON_ERROR_STOP=1 <<EOF
DROP DATABASE IF EXISTS sgw_prev;
BEGIN;
ALTER DATABASE $DB RENAME TO sgw_prev;
ALTER DATABASE sgw_restore RENAME TO $DB;
COMMIT;
EOF
    trap - EXIT
    log "restore $FILE"
    exit 0
}

# Rule 1: restore.
if [ -n "$RESTORE" ]; then
    case "$RESTORE" in
        */*) FILE="$RESTORE" ;;
        *) FILE="$BACKUP_DIR/$RESTORE" ;;
    esac
    if [ ! -f "$FILE" ]; then
        log "ERROR restore file not found: $FILE"
        exit 1
    fi
    SQL_FILE=$(printf '%s' "$FILE" | sed "s/'/''/g")
    if ! restore_marked; then
        restore_db
    fi
fi

# Rule 2: first boot of the baked image.
if [ "$HAS_META" = t ] && [ "$STATE" = baked ]; then
    # db_state has no applied_at column (db/persistent/meta.sql); first_seeded_at is the stamp.
    psqlq "$DB" "UPDATE cimmeria_meta.db_state SET state = 'live', first_seeded_at = coalesce(first_seeded_at, now())"
    log "first-boot image_id=$CUR_ID"
    exit 0
fi

# Rule 3: adopt a database that predates cimmeria_meta. Keep its players.
if [ "$HAS_META" != t ]; then
    backup
    log "adopt (no cimmeria_meta; keeping player data)"
    if ! /opt/cimmeria/bin/db-rebuild.sh "$DB" "$NEW_ID"; then
        log "ERROR rebuild failed, sgw untouched"
        exit 1
    fi
    exit 0
fi

# Rule 4: rebuild when forced or when the image changed.
FORCED=0
case "$FORCE" in
    1|true|yes|TRUE|True) FORCED=1 ;;
esac
if [ "$FORCED" = 1 ] || [ "$CUR_ID" != "$NEW_ID" ]; then
    REASON=image-changed
    if [ "$FORCED" = 1 ]; then
        REASON=forced
    fi
    if ! reuse_backup; then
        backup
    fi
    log "rebuild reason=$REASON old=$CUR_ID new=$NEW_ID"
    if ! /opt/cimmeria/bin/db-rebuild.sh "$DB" "$NEW_ID"; then
        psqlq "$DB" "UPDATE cimmeria_meta.db_state SET last_failed_image_id = '$NEW_ID', last_failed_at = now()" >/dev/null 2>&1 || true
        # Keep the backup newer than the failure so a crash-looping restart
        # reuses it instead of dumping again and evicting older backups.
        touch "$BACKUP_DIR/$NAME" 2>/dev/null || true
        log "ERROR rebuild failed, sgw untouched"
        exit 1
    fi
    exit 0
fi

# Rule 5: nothing to do. The server starts after this, so an earlier failure
# marker is now stale (and must not make a later forced rebuild reuse a backup
# that predates the data the server is about to change): report it, then clear it.
LAST_FAILED=$(psqlq "$DB" "SELECT coalesce(last_failed_image_id, '') FROM cimmeria_meta.db_state" 2>/dev/null) || LAST_FAILED=""
if [ -n "$LAST_FAILED" ]; then
    log "skip image_id=$CUR_ID (an earlier rebuild to $LAST_FAILED failed; set CIMMERIA_DB_FORCE_REBUILD=1 to retry)"
    psqlq "$DB" "UPDATE cimmeria_meta.db_state SET last_failed_image_id = NULL, last_failed_at = NULL" >/dev/null 2>&1 || true
else
    log "skip image_id=$CUR_ID"
fi
exit 0
