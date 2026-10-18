#!/usr/bin/env bash
# Container persistence smoke (campaign persistent-player-data, PP-08): boots
# the server image against its own named volume and walks the db-sync state
# table end to end: first-boot, skip, forced rebuild with seed-account top-up,
# dangling-item reconcile, fail-closed rebuild, adopt, DR restore, full wipe.
# Run by .github/workflows/pr-container.yml; needs Docker. Never touches the
# operator volume (cimmeria-server-pgdata).
#
# Usage:
#   tools/container-persistence-smoke.sh <image> <logs-dir>
#
# Prints "PASS scenario N" after each scenario; exits 0 only if all pass.

set -u

IMAGE="${1:?usage: container-persistence-smoke.sh <image> <logs-dir>}"
LOG_DIR="${2:?usage: container-persistence-smoke.sh <image> <logs-dir>}"
VOL=cimmeria-persist-smoke-pgdata
NAME=cimmeria-persist
BAK="$(mktemp -d)"
chmod 777 "$BAK"
mkdir -p "$LOG_DIR"

# Ids that cannot collide with seed data (seed ids are small).
S1=900000001
S2=900000002
TMP_ITEM=900000003

# Number of [db-sync] lines that belong to earlier boots of this container;
# boot_lines/assert_log only look at lines after it.
MARK=0

err() { echo "::error::$*"; exit 1; }

cleanup() {
  rc=$?
  echo "::group::container state"
  docker inspect -f 'Status: {{.State.Status}}  ExitCode: {{.State.ExitCode}}' "$NAME" || true
  echo "::endgroup::"
  echo "::group::container logs (docker logs)"
  docker logs "$NAME" 2>&1 | tee "$LOG_DIR/docker-logs.txt" || true
  echo "::endgroup::"
  docker cp "$NAME:/var/log/cimmeria/." "$LOG_DIR/server-logs" >/dev/null 2>&1 || true
  ls -l "$BAK" > "$LOG_DIR/backups.txt" 2>&1 || true
  docker rm -f "$NAME" >/dev/null 2>&1 || true
  docker volume rm -f "$VOL" >/dev/null 2>&1 || true
  rm -rf "$BAK"
  exit "$rc"
}
trap cleanup EXIT

all_lines() { docker logs "$NAME" 2>&1 | grep '^\[db-sync\]' || true; }
boot_lines() { all_lines | tail -n +$((MARK + 1)); }
assert_log() {
  boot_lines | grep -q "^\[db-sync\] $1" || {
    echo "boot lines:"; boot_lines
    err "expected '[db-sync] $1' in this boot's lines"
  }
}
refute_log() {
  if boot_lines | grep -q "^\[db-sync\] $1"; then
    boot_lines
    err "did not expect '[db-sync] $1' in this boot's lines"
  fi
}

# start [extra docker run args]: data lives on the volume.
start() {
  docker run -d --name "$NAME" \
    -v "$VOL:/var/lib/postgresql/data" \
    -v "$BAK:/var/backups/cimmeria" \
    "$@" "$IMAGE" >/dev/null || err "docker run failed"
}
recreate() { docker rm -f "$NAME" >/dev/null 2>&1 || true; MARK=0; start "$@"; }
restart() { MARK="$(all_lines | wc -l)"; docker restart "$NAME" >/dev/null || err "docker restart failed"; }

state() { docker inspect -f '{{.State.Status}}' "$NAME" 2>/dev/null || echo missing; }

# Ready = postgres answers and this boot has logged a [db-sync] decision.
wait_db() {
  i=0
  while [ "$i" -lt 150 ]; do
    case "$(state)" in
      exited|dead|missing) err "container exited before becoming ready" ;;
    esac
    if docker exec "$NAME" pg_isready -h 127.0.0.1 -q >/dev/null 2>&1 \
       && [ -n "$(boot_lines)" ]; then
      return 0
    fi
    sleep 2; i=$((i + 1))
  done
  err "container not ready within 300s"
}
wait_exit() {
  i=0
  while [ "$i" -lt 150 ]; do
    [ "$(state)" = exited ] && return 0
    sleep 2; i=$((i + 1))
  done
  err "container did not exit within 300s"
}

psql_in() {
  docker exec -e PGPASSWORD=w-testing "$NAME" psql -h 127.0.0.1 -U w-testing \
    -d "${DB:-sgw}" -tAX -v ON_ERROR_STOP=1 -c "$1"
}
assert_eq() { # <what> <expected> <actual>
  [ "$2" = "$3" ] || err "$1: expected '$2', got '$3'"
}

force() { recreate -e CIMMERIA_DB_FORCE_REBUILD=1; wait_db; }

# ---- 1. first boot -------------------------------------------------------
docker volume rm -f "$VOL" >/dev/null 2>&1 || true
docker volume create "$VOL" >/dev/null
start
wait_db
assert_log "first-boot"
n="$(psql_in "SELECT count(*) FROM public.account")"
[ "$n" -ge 14 ] || err "scenario 1: expected >= 14 seed accounts, got $n"
assert_eq "scenario 1 state" live "$(psql_in "SELECT state FROM cimmeria_meta.db_state")"
echo "PASS scenario 1"

# ---- 2. restart keeps player data ---------------------------------------
# Copy seed player 62 (account 2); player_name is UNIQUE and player_id the PK.
psql_in "CREATE TEMP TABLE s AS SELECT * FROM sgw_player WHERE player_id = 62;
  UPDATE s SET player_id = $S1, player_name = 'PersistSentinel', account_id = 2;
  INSERT INTO sgw_player SELECT * FROM s;" >/dev/null
assert_eq "scenario 2 setup" 1 "$(psql_in "SELECT count(*) FROM sgw_player WHERE player_id = $S1")"
restart
wait_db
assert_log "skip"
refute_log "rebuild"
assert_eq "scenario 2 sentinel kept" PersistSentinel \
  "$(psql_in "SELECT player_name FROM sgw_player WHERE player_id = $S1")"
echo "PASS scenario 2"

# ---- 3. forced rebuild with seed-account top-up --------------------------
# Accounts 13 and 15 own no characters (characters exist for accounts 2-9);
# guard it so a future seed that changes that fails here, not in a cascade.
OWNED="$(psql_in "SELECT count(*) FROM sgw_player WHERE account_id IN (13, 15)")"
assert_eq "scenario 3 guard: accounts 13/15 own no characters" 0 "$OWNED"
ORIG_NAME="$(psql_in "SELECT name FROM resources.items WHERE item_id = (SELECT min(item_id) FROM resources.items)")"
[ -n "$ORIG_NAME" ] || err "scenario 3: no resources.items row to mutate"
psql_in "UPDATE resources.items SET name = 'MUTATED' WHERE item_id = (SELECT min(item_id) FROM resources.items);
  DELETE FROM public.account WHERE account_id = 15;
  UPDATE public.account SET accesslevel = 9 WHERE account_id = 13;" >/dev/null
force
assert_log "rebuild"
assert_eq "scenario 3 resources reseeded" "$ORIG_NAME" \
  "$(psql_in "SELECT name FROM resources.items WHERE item_id = (SELECT min(item_id) FROM resources.items)")"
assert_eq "scenario 3 sentinel kept (D3)" 1 "$(psql_in "SELECT count(*) FROM sgw_player WHERE player_id = $S1")"
assert_eq "scenario 3 account 15 re-added" 1 "$(psql_in "SELECT count(*) FROM public.account WHERE account_id = 15")"
assert_eq "scenario 3 insert_seed_account rows" 1 \
  "$(psql_in "SELECT count(*) FROM cimmeria_meta.reconcile_log WHERE action = 'insert_seed_account'")"
assert_eq "scenario 3 account 13 untouched (D6)" 9 \
  "$(psql_in "SELECT accesslevel FROM public.account WHERE account_id = 13")"
BACKUP="$(ls -1t "$BAK"/sgw-*.dump 2>/dev/null | head -1)"
[ -n "$BACKUP" ] || err "scenario 3: no sgw-*.dump in the backup dir"
BACKUP_NAME="$(basename "$BACKUP")"
assert_eq "scenario 3 sgw_prev exists" 1 \
  "$(DB=postgres psql_in "SELECT 1 FROM pg_database WHERE datname = 'sgw_prev'")"
echo "PASS scenario 3"

# ---- 4. dangling item reconcile ------------------------------------------
# A temp resources.items row and an inventory row of the sentinel that points
# at it (type_id). The rebuild reseeds resources, so the item vanishes and the
# reconcile must delete the inventory row and log it (JSON, D5).
psql_in "CREATE TEMP TABLE t AS SELECT * FROM resources.items WHERE item_id = (SELECT min(item_id) FROM resources.items);
  UPDATE t SET item_id = $TMP_ITEM, name = 'PersistTempItem';
  INSERT INTO resources.items SELECT * FROM t;
  INSERT INTO sgw_inventory (item_id, container_id, slot_id, type_id, character_id)
    VALUES ($TMP_ITEM, 1, 900, $TMP_ITEM, $S1);" >/dev/null
assert_eq "scenario 4 setup" 1 "$(psql_in "SELECT count(*) FROM sgw_inventory WHERE character_id = $S1 AND slot_id = 900")"
force
assert_log "rebuild"
assert_eq "scenario 4 temp item gone" 0 "$(psql_in "SELECT count(*) FROM resources.items WHERE item_id = $TMP_ITEM")"
assert_eq "scenario 4 dangling inventory row deleted" 0 \
  "$(psql_in "SELECT count(*) FROM sgw_inventory WHERE character_id = $S1 AND slot_id = 900")"
assert_eq "scenario 4 delete logged" 1 \
  "$(psql_in "SELECT count(*) FROM cimmeria_meta.reconcile_log WHERE action = 'delete_dangling_item' AND table_name = 'sgw_inventory' AND (detail->>'character_id')::int = $S1")"
assert_eq "scenario 4 sentinel kept" 1 "$(psql_in "SELECT count(*) FROM sgw_player WHERE player_id = $S1")"
echo "PASS scenario 4"

# ---- 5. fail closed -------------------------------------------------------
# The rebuilt schema has level_sanity (level 0..50); a row that violates it
# cannot be carried, so the rebuild must stop and leave sgw untouched.
psql_in "ALTER TABLE sgw_player DROP CONSTRAINT level_sanity;
  CREATE TEMP TABLE s2 AS SELECT * FROM sgw_player WHERE player_id = $S1;
  UPDATE s2 SET player_id = $S2, player_name = 'PersistSentinel2', level = 99;
  INSERT INTO sgw_player SELECT * FROM s2;" >/dev/null
recreate -e CIMMERIA_DB_FORCE_REBUILD=1
wait_exit
CODE="$(docker inspect -f '{{.State.ExitCode}}' "$NAME")"
[ "$CODE" != 0 ] || err "scenario 5: forced rebuild with an uncarriable row exited 0"
assert_log "ERROR"
# Unforced boot: image id unchanged since scenario 3, so db-sync skips and sgw
# is intact (sentinel2 present). The failed container is gone, so sgw_next is
# checked here: the failed run must not have left it behind.
recreate
wait_db
assert_log "skip"
assert_eq "scenario 5 sentinel2 intact" 1 "$(psql_in "SELECT count(*) FROM sgw_player WHERE player_id = $S2")"
assert_eq "scenario 5 sgw_next absent" 0 \
  "$(DB=postgres psql_in "SELECT count(*) FROM pg_database WHERE datname = 'sgw_next'")"
psql_in "DELETE FROM sgw_player WHERE player_id = $S2;
  ALTER TABLE sgw_player ADD CONSTRAINT level_sanity CHECK (((level >= 0) AND (level <= 50))) NOT VALID;" >/dev/null
echo "PASS scenario 5"

# ---- 6. adopt -------------------------------------------------------------
psql_in "DROP SCHEMA cimmeria_meta CASCADE" >/dev/null
restart
wait_db
assert_log "adopt"
assert_eq "scenario 6 sentinel kept" 1 "$(psql_in "SELECT count(*) FROM sgw_player WHERE player_id = $S1")"
assert_eq "scenario 6 db_state back" live "$(psql_in "SELECT state FROM cimmeria_meta.db_state")"
echo "PASS scenario 6"

# ---- 7. disaster-recovery restore ----------------------------------------
psql_in "DELETE FROM sgw_player WHERE player_id = $S1" >/dev/null
recreate -e "CIMMERIA_DB_RESTORE=$BACKUP_NAME"
wait_db
assert_log "restore"
assert_eq "scenario 7 sentinel restored" 1 "$(psql_in "SELECT count(*) FROM sgw_player WHERE player_id = $S1")"
restart   # docker restart keeps the env var; the restore marker must stop a repeat
wait_db
assert_log "skip"
refute_log "restore"
echo "PASS scenario 7"

# ---- 8. full wipe ---------------------------------------------------------
docker rm -f "$NAME" >/dev/null 2>&1 || true
docker volume rm "$VOL" >/dev/null || err "scenario 8: volume rm failed"
docker volume create "$VOL" >/dev/null
MARK=0
start
wait_db
assert_log "first-boot"
assert_eq "scenario 8 sentinel gone" 0 "$(psql_in "SELECT count(*) FROM sgw_player WHERE player_id = $S1")"
docker rm -v -f "$NAME" >/dev/null || err "scenario 8: rm -v failed"
docker volume inspect "$VOL" >/dev/null 2>&1 || err "scenario 8: named volume did not survive rm -v"
echo "PASS scenario 8"
echo "container persistence smoke passed"
