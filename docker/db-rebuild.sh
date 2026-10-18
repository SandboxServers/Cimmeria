#!/bin/sh
# db-rebuild.sh <source_db> <image_id>
#
# Blue/green rebuild of the game database (campaign persistent-player-data).
# Builds "<source_db>_next" from the baked fresh dump, carries the player data
# over from <source_db>, reconciles it against the new resources, checks row
# counts and swaps the two by renaming. The live database is never dropped:
# the previous one is kept as "<source_db>_prev". Any error drops the _next
# database, prints one "[db-rebuild] ERROR <cause>" line and exits non-zero,
# leaving <source_db> untouched. Called by db-sync.sh; POSIX sh.
set -eu

SRC="${1:?usage: db-rebuild.sh <source_db> <image_id>}"
ID="${2:?usage: db-rebuild.sh <source_db> <image_id>}"
NEXT="${SRC}_next"
PREV="${SRC}_prev"
P=/opt/cimmeria/db/persistent
FRESH=/opt/cimmeria/db/fresh.dump
SHAFILE=/opt/cimmeria/db/db-source.sha256

# Player data lives only in the public schema (every table named in
# persistent-public-tables.txt is public), so there are no extra player
# schemas. Space separated, for a future non-public player schema.
PLAYER_SCHEMAS=""

PGPASSWORD=w-testing
export PGPASSWORD
PGOPTIONS="-c cimmeria.image_id=$ID"
export PGOPTIONS
PSQL="psql -h 127.0.0.1 -U w-testing -v ON_ERROR_STOP=1 -X -q"
PGR="-h 127.0.0.1 -U w-testing"

WORK=$(mktemp -d)

drop_next() {
    $PSQL -d postgres -c "DROP DATABASE IF EXISTS \"$NEXT\"" >/dev/null 2>&1 || true
}
fail() {
    echo "[db-rebuild] ERROR $*" >&2
    exit 1
}
# Any exit that is not success (fail, or a set -e abort) drops the _next copy.
trap 'rc=$?; if [ "$rc" -ne 0 ]; then drop_next; fi; rm -rf "$WORK"' EXIT

# Last non-empty line of a captured stderr file, for one-line causes.
lasterr() {
    grep -v '^[[:space:]]*$' "$1" 2>/dev/null | tail -n 1 || true
}

# Names from a table list: strip CR, comments, blanks; bare names get "public.".
read_list() {
    tr -d '\r' < "$1" | sed -e 's/#.*$//' -e 's/[[:space:]]//g' -e '/^$/d' \
        | awk '{ if (index($0, ".") == 0) print "public." $0; else print $0 }'
}

# Run a SQL file in $NEXT; the cause is the last stderr line.
run_sql() {
    $PSQL -d "$NEXT" -f "$1" >/dev/null 2>"$WORK/err" || fail "$2 failed: $(lasterr "$WORK/err")"
}

[ -f "$FRESH" ] || fail "missing $FRESH"
for f in reconcile.sql reseed-public-tables.txt persistent-public-tables.txt; do
    [ -f "$P/$f" ] || fail "missing $P/$f"
done

read_list "$P/persistent-public-tables.txt" > "$WORK/persistent.list"
read_list "$P/reseed-public-tables.txt" > "$WORK/reseed.list"
# Carried tables: persistent list, minus reseed tables, plus public.account.
{ cat "$WORK/persistent.list"; echo public.account; } | sort -u \
    | grep -vxF -f "$WORK/reseed.list" > "$WORK/carry.list" || true
[ -s "$WORK/carry.list" ] || fail "carry list is empty"

# step 1
echo "[db-rebuild] step 1 drop leftover $NEXT"
$PSQL -d postgres -c "DROP DATABASE IF EXISTS \"$NEXT\"" >/dev/null 2>"$WORK/err" \
    || fail "drop leftover $NEXT failed: $(lasterr "$WORK/err")"

# step 2
echo "[db-rebuild] step 2 create $NEXT and restore the fresh dump"
$PSQL -d postgres -c "CREATE DATABASE \"$NEXT\"" 2>"$WORK/err" || fail "create $NEXT failed: $(lasterr "$WORK/err")"
# sgw_next is disposable until the swap; step 9 resets this.
$PSQL -d postgres -c "ALTER DATABASE \"$NEXT\" SET synchronous_commit = off" 2>"$WORK/err" \
    || fail "alter $NEXT failed: $(lasterr "$WORK/err")"
pg_restore --no-owner --exit-on-error $PGR -d "$NEXT" "$FRESH" 2>"$WORK/err" \
    || fail "fresh dump restore failed: $(lasterr "$WORK/err")"

# step 3: none (reconcile.sql has no pre-check phase)
echo "[db-rebuild] step 3 pre-checks: none"

# step 4
echo "[db-rebuild] step 4 carry player data from $SRC"
pg_dump -Fc $PGR -d "$SRC" -f "$WORK/src.dump" 2>"$WORK/err" || fail "dump of $SRC failed: $(lasterr "$WORK/err")"
pg_restore -l "$WORK/src.dump" > "$WORK/toc.full" 2>"$WORK/err" || fail "toc of dump failed: $(lasterr "$WORK/err")"
# Keep a TOC line iff it is a TABLE DATA line and schema.table is in carry.list
# (persistent public tables plus public.account, minus the reseed tables) or the
# schema is a player schema (and the table is not a reseed table). Everything
# else, SEQUENCE SET lines included, is dropped; step 6 fixes sequences.
awk -v schemas="$PLAYER_SCHEMAS" -v carry="$WORK/carry.list" -v reseed="$WORK/reseed.list" '
    BEGIN {
        n = split(schemas, s, " "); for (i = 1; i <= n; i++) ps[s[i]] = 1
        while ((getline l < carry) > 0) keep[l] = 1
        while ((getline l < reseed) > 0) skip[l] = 1
    }
    {
        for (i = 1; i < NF; i++) if ($i == "TABLE" && $(i + 1) == "DATA") break
        if (i >= NF - 2) next
        key = $(i + 2) "." $(i + 3)
        if (key in skip) next
        if ((key in keep) || ($(i + 2) in ps)) print
    }' "$WORK/toc.full" > "$WORK/toc.filtered"
[ -s "$WORK/toc.filtered" ] || fail "carry TOC is empty; source dump has no player tables"
# fresh.dump may hold seeded rows in carried tables (account); the live rows
# replace them. One TRUNCATE for the whole set so foreign keys never block it.
TLIST=$(sed -e 's/^\([^.]*\)\.\(.*\)$/"\1"."\2"/' "$WORK/carry.list" | paste -sd, -)
$PSQL -d "$NEXT" -c "TRUNCATE $TLIST" 2>"$WORK/err" || fail "truncate of carried tables failed: $(lasterr "$WORK/err")"
pg_restore --data-only --disable-triggers --exit-on-error -L "$WORK/toc.filtered" \
    $PGR -d "$NEXT" "$WORK/src.dump" 2>"$WORK/err" \
    || fail "carry-over rejected: $(lasterr "$WORK/err")"

# step 4a (the top-up SQL ships separately from the wave 1 files)
echo "[db-rebuild] step 4a stage seed accounts"
if [ -f "$P/seed-accounts-stage.sql" ]; then
    run_sql "$P/seed-accounts-stage.sql" "seed-accounts-stage.sql"
else
    echo "[db-rebuild] step 4a skipped: $P/seed-accounts-stage.sql not in this image"
fi

# step 5
echo "[db-rebuild] step 5 reconcile"
run_sql "$P/reconcile.sql" "reconcile.sql"

# step 5a
echo "[db-rebuild] step 5a seed account top-up"
if [ -f "$P/seed-accounts.sql" ]; then
    run_sql "$P/seed-accounts.sql" "seed-accounts.sql"
else
    echo "[db-rebuild] step 5a skipped: $P/seed-accounts.sql not in this image"
fi

# Carry the DR restore marker (db-sync.sh, CIMMERIA_DB_RESTORE) across the swap.
# Without it a CIMMERIA_DB_RESTORE left in the environment would restore the
# same backup again on the first boot after this rebuild.
HAS_RD=$($PSQL -d "$SRC" -At -c "SELECT to_regclass('cimmeria_meta.restore_done') IS NOT NULL" 2>/dev/null || true)
if [ "$HAS_RD" = t ]; then
    echo "[db-rebuild] step 5b carry restore marker"
    $PSQL -d "$SRC" -c "COPY cimmeria_meta.restore_done TO STDOUT" > "$WORK/restore_done.copy" 2>"$WORK/err" \
        || fail "read of restore_done failed: $(lasterr "$WORK/err")"
    {
        echo "CREATE TABLE IF NOT EXISTS cimmeria_meta.restore_done (file text primary key, at timestamptz default now());"
        echo "COPY cimmeria_meta.restore_done FROM STDIN;"
        cat "$WORK/restore_done.copy"
        printf '%s\n' '\.'
    } | $PSQL -d "$NEXT" >/dev/null 2>"$WORK/err" || fail "carry of restore_done failed: $(lasterr "$WORK/err")"
fi

# step 6: setval for every sequence owned by a carried column
echo "[db-rebuild] step 6 sequences"
INLIST=$(sed -e "s/^\([^.]*\)\.\(.*\)\$/('\1','\2')/" "$WORK/carry.list" | paste -sd, -)
$PSQL -d "$NEXT" -At -c "
    SELECT format('SELECT setval(%L, GREATEST(COALESCE((SELECT max(%I) FROM %I.%I), 1), 1), true)',
                  pg_get_serial_sequence(format('%I.%I', table_schema, table_name), column_name),
                  column_name, table_schema, table_name)
    FROM information_schema.columns
    WHERE column_default LIKE 'nextval%'
      AND pg_get_serial_sequence(format('%I.%I', table_schema, table_name), column_name) IS NOT NULL
      AND ((table_schema, table_name) IN ($INLIST))" > "$WORK/setval.sql" 2>"$WORK/err" \
    || fail "sequence query failed: $(lasterr "$WORK/err")"
if [ -s "$WORK/setval.sql" ]; then
    $PSQL -d "$NEXT" -At -f "$WORK/setval.sql" >/dev/null 2>"$WORK/err" \
        || fail "sequence fix-up failed: $(lasterr "$WORK/err")"
fi

# step 7
echo "[db-rebuild] step 7 analyze"
$PSQL -d "$NEXT" -c "ANALYZE" 2>"$WORK/err" || fail "analyze failed: $(lasterr "$WORK/err")"

# step 8: expected = src - logged item deletes + logged seed-account inserts
echo "[db-rebuild] step 8 count check"
while IFS= read -r qt; do
    sch=${qt%%.*}
    tbl=${qt#*.}
    # ONLY: sgw_inventory inherits sgw_inventory_base and each is counted alone.
    cs=$($PSQL -d "$SRC" -At -c "SELECT count(*) FROM ONLY \"$sch\".\"$tbl\"" 2>"$WORK/err") \
        || fail "count of $qt in $SRC failed: $(lasterr "$WORK/err")"
    cn=$($PSQL -d "$NEXT" -At -c "SELECT count(*) FROM ONLY \"$sch\".\"$tbl\"" 2>"$WORK/err") \
        || fail "count of $qt in $NEXT failed: $(lasterr "$WORK/err")"
    del=$($PSQL -d "$NEXT" -At -c "SELECT count(*) FROM cimmeria_meta.reconcile_log WHERE image_id = '$ID' AND action = 'delete_dangling_item' AND table_name = '$tbl'" 2>"$WORK/err") \
        || fail "reconcile log read failed: $(lasterr "$WORK/err")"
    ins=0
    if [ "$qt" = "public.account" ]; then
        ins=$($PSQL -d "$NEXT" -At -c "SELECT count(*) FROM cimmeria_meta.reconcile_log WHERE image_id = '$ID' AND action = 'insert_seed_account'" 2>"$WORK/err") \
            || fail "reconcile log read failed: $(lasterr "$WORK/err")"
    fi
    exp=$((cs - del + ins))
    if [ "$cn" -ne "$exp" ]; then
        fail "count mismatch table=$qt src=$cs next=$cn expected=$exp"
    fi
done < "$WORK/carry.list"

# step 9
echo "[db-rebuild] step 9 write state"
OLD=$($PSQL -d "$SRC" -At -c "SELECT COALESCE(rebuild_count, 0) FROM cimmeria_meta.db_state WHERE id = 1" 2>/dev/null || true)
case "$OLD" in
    ''|*[!0-9]*) OLD=0 ;;
esac
SHA=""
if [ -f "$SHAFILE" ]; then SHA=$(tr -d '\r\n ' < "$SHAFILE"); fi
# State columns are those of db/persistent/meta.sql (last_rebuild_at, rebuild_count).
# psql does not interpolate :'var' in a -c string, so the statement goes in on stdin.
$PSQL -d "$NEXT" -v "id=$ID" -v "old=$OLD" -v "sha=$SHA" 2>"$WORK/err" <<'SQL' || fail "state write failed: $(lasterr "$WORK/err")"
UPDATE cimmeria_meta.db_state
   SET state = 'live', image_id = :'id',
       db_source_sha256 = CASE WHEN :'sha' = '' THEN db_source_sha256 ELSE :'sha' END,
       first_seeded_at = COALESCE(first_seeded_at, now()),
       last_rebuild_at = now(),
       rebuild_count = :'old'::integer + 1
 WHERE id = 1;
SQL
$PSQL -d postgres -c "ALTER DATABASE \"$NEXT\" RESET synchronous_commit" 2>"$WORK/err" \
    || fail "reset synchronous_commit failed: $(lasterr "$WORK/err")"

# step 10: swap by rename; the live database is renamed, never dropped
echo "[db-rebuild] step 10 swap"
$PSQL -d postgres -c "DROP DATABASE IF EXISTS \"$PREV\"" 2>"$WORK/err" || fail "drop old $PREV failed: $(lasterr "$WORK/err")"

# rename_db <from> <to>: terminate other sessions, rename, retry 5 times (the
# healthcheck and the server reconnect between the terminate and the rename).
rename_db() {
    n=0
    while [ "$n" -lt 5 ]; do
        n=$((n + 1))
        $PSQL -d postgres -At -c "SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE datname IN ('$SRC','$NEXT','$PREV') AND pid <> pg_backend_pid()" >/dev/null 2>&1 || true
        if $PSQL -d postgres -c "ALTER DATABASE \"$1\" RENAME TO \"$2\"" 2>"$WORK/err"; then
            return 0
        fi
        sleep 2
    done
    return 1
}

rename_db "$SRC" "$PREV" || fail "rename $SRC to $PREV failed: $(lasterr "$WORK/err")"
if ! rename_db "$NEXT" "$SRC"; then
    cause=$(lasterr "$WORK/err")
    if rename_db "$PREV" "$SRC"; then
        fail "rename $NEXT to $SRC failed, $SRC restored: $cause"
    fi
    fail "rename $NEXT to $SRC failed and restore of $SRC failed (previous data is in $PREV): $cause"
fi
echo "[db-rebuild] swapped; previous database kept as $PREV"
