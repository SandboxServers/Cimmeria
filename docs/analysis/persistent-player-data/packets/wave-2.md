# Wave 2 packets: the container change (PP-04 to PP-08)

One PR. Five packets on disjoint files, built in parallel against the contract
below, combined by the review-fixer onto the PP-04 branch. Ops creates the
worktrees from the merged wave 1 main. Source of truth for the design:
[work-packets.md](../work-packets.md) (contract + wave 2 specs),
[README.md](../README.md) (D-PP decisions) and
[OWNER-DECISIONS.md](../OWNER-DECISIONS.md) (D1-D8).

## Rules every wave 2 packet inlines

- Windows, PowerShell only. No bash, no docker, no WSL locally. Shell scripts
  and docker builds are verified **in CI only** (README D-PP6, workflow
  `pr-container.yml`); say "CI only" in the report, never claim they ran.
- No `git stash`, no `git worktree prune`, no push, no `cargo` (no Rust here).
  Commit locally once, then stop.
- Line endings: `docker/**`, `*.sh`, `.github/workflows/*.yml` and
  `.github/dependabot.yml` are **LF** (`.gitattributes`). `docs/**/*.md` is CRLF.
  Write LF files with LF only. After writing, check with
  `(Get-Content -Raw <file>) -match "`r"` that an LF file has no CR.
- Shell scripts are POSIX `sh` (`#!/bin/sh`, `set -eu`). No arrays, no
  `[[ ]]`, no `local` abuse beyond what dash supports (dash supports `local`),
  no `echo -e`, no process substitution, no `pipefail`.
- Files: soft cap 500 lines, hard cap 700.
- No IPs, credentials or account names in comments (project memory rule). The
  fixed dev password `w-testing` already in the repo is fine.
- Report file: `<worktree>\.claude\reports\persistent-player-data\<packet>-coder.md`
  (PP-06: `-database-persistence.md`, PP-08: `-testing.md`).
- Return exactly one line: `<packet> <done|blocked|failed> <commit-sha|-> <report-path>`.
- Commit message ends with the attribution lines the dispatch brief gives.

## Contract excerpt (build against these names exactly)

Image layout (all read-only, root-owned, set up by PP-04):

| Path | What |
|---|---|
| `/opt/cimmeria/db/fresh.dump` | `pg_dump -Fc` of the baked `sgw` DB (resources + empty player tables + `cimmeria_meta`) |
| `/opt/cimmeria/db/image-id` | one line: `CIMMERIA_GIT_SHA` or the `unknown` fallback plus `-` plus the first 12 hex of `db-source.sha256` |
| `/opt/cimmeria/db/db-source.sha256` | one line: sha256 of the concatenated `db/database.sql` + `db/sgw/**` + `db/resources/**` + `db/persistent/**` file contents, sorted by path |
| `/opt/cimmeria/db/persistent/` | the wave 1 `db/persistent/` directory: `meta.sql`, `reconcile.sql`, `seed-accounts-stage.sql`, `seed-accounts.sql`, `reseed-public-tables.txt`, `persistent-public-tables.txt` |
| `/opt/cimmeria/bin/db-sync.sh`, `db-rebuild.sh` | the two scripts, mode 0755 |
| `/var/backups/cimmeria` | backup dir, a `VOLUME`, owned by `postgres` |

Database objects (wave 1, `db/persistent/meta.sql`): schema `cimmeria_meta`,
table `cimmeria_meta.db_state` (one row; columns used here: `state` text
`'baked'|'live'`, `image_id` text, `applied_at` timestamptz,
`last_failed_image_id` text, `last_failed_at` timestamptz). The SQL files read
the session setting `cimmeria.image_id` (`current_setting('cimmeria.image_id')`);
the shell sets it with `PGOPTIONS="-c cimmeria.image_id=<id>"`.

Environment variables (container): `CIMMERIA_DB_FORCE_REBUILD` (1/true),
`CIMMERIA_DB_RESTORE` (path or file name under `/var/backups/cimmeria`, DR
only), `CIMMERIA_DB_BACKUP_KEEP` (default 10). Compose-only (not read by the
container): `CIMMERIA_BACKUP_DIR` (host dir, default `./backups`).

Backup name: `sgw-<UTC yyyymmddThhmmssZ>-<old image_id or 'pre-meta'>.dump`,
written with `pg_dump -Fc` of the live `sgw`.

db-sync log lines (stdout of the oneshot; one per boot decision), prefix
`[db-sync] `: `restore`, `first-boot`, `adopt`, `rebuild`, `skip`, each
followed by free text. Failures: `[db-sync] ERROR ...` and non-zero exit.

State table, evaluated in this order (first match wins):

1. `CIMMERIA_DB_RESTORE` set and the named backup exists and
   `cimmeria_meta.restore_done` marker (see PP-05 step 4) does not name it:
   `restore`.
2. `sgw` has `cimmeria_meta.db_state` with `state = 'baked'`: `first-boot`
   (set `state = 'live'`, `image_id` = the baked id; no rebuild, nothing to
   carry).
3. `sgw` has no `cimmeria_meta` schema: `adopt` (backup, then rebuild with
   adoption, i.e. no carry of `cimmeria_meta`; result is `live`).
4. `CIMMERIA_DB_FORCE_REBUILD` true, or `db_state.image_id` differs from
   `/opt/cimmeria/db/image-id`: `rebuild` (backup first, then
   `db-rebuild.sh sgw <new image id>`).
5. else `skip`.

Rebuild (`db-rebuild.sh <source_db> <image_id>`), ten steps, fail closed:
1 drop leftover `sgw_next`; 2 create `sgw_next` and restore
`/opt/cimmeria/db/fresh.dump` into it; 3 apply `reconcile.sql` pre-checks;
4 carry player data (`pg_dump -Fc` of the source, `pg_restore --data-only`
with the TOC filtered to player schemas and `public.account`, minus the tables
in `reseed-public-tables.txt`); 4a `seed-accounts-stage.sql`; 5 `reconcile.sql`
(dangling-id deletes with JSON log rows, world relocation); 5a
`seed-accounts.sql` (insert-only top-up of missing seed accounts); 6 sequence
fix-up; 7 `ANALYZE`; 8 count check; 9 write `db_state` (`live`, new id);
10 swap: terminate connections, rename `sgw` to `sgw_prev` and `sgw_next` to
`sgw`. Any error: drop `sgw_next`, exit non-zero, `sgw` untouched.

The wave 1 files may not exist yet in your worktree if wave 1 is not merged:
Ops branches wave 2 from the merged wave 1 main, so they will. If
`db/persistent/meta.sql` is missing when you start, return `blocked`.

---

## PP-04 Image build: bake the dump, meta and ids

- **Agent:** packet-coder. Worktree `pp-04`, branch `persistent-player-data/pp-04-image-bake`.
- **Commit subject:** `feat(docker): bake fresh.dump, cimmeria_meta and image id into the image`
- **Files:** `docker/Dockerfile` only.
- **Depends on:** wave 1 (`db/persistent/meta.sql`); contract paths of PP-05 (`docker/db-sync.sh`, `docker/s6/db-sync/`) and PP-06 (`docker/db-rebuild.sh`). Those files land from sibling packets; the COPY lines name them by the contract paths.

### Steps

Read `docker/Dockerfile` first. Anchors are by text, line numbers are approximate.

1. **Header comment (lines 3-11).** Replace the sentence that says the runtime
   image carries zero `.sql` files with: the runtime image now carries
   `/opt/cimmeria/db/` (the baked `fresh.dump`, `image-id`, `db-source.sha256`
   and the `persistent/` SQL) and `/opt/cimmeria/bin/` (`db-sync.sh`,
   `db-rebuild.sh`); the pgdata is baked once, copied to the named volume on
   first start only, and `db-sync` decides at every boot whether to keep,
   adopt or rebuild.
2. **`db-init` stage (lines ~120-151).**
   - Directly after the stage's `FROM`, add `ARG CIMMERIA_GIT_SHA=unknown`
     (the one at line ~93 is builder-only; ARGs do not cross stages). Add
     `ARG CIMMERIA_GIT_SHA=unknown` at the top of the stage body.
   - `COPY db /tmp/db` stays. In the single `RUN` that does initdb /
     `pg_ctl start` / `CREATE DATABASE sgw` / `psql -f /tmp/db/database.sql`
     / `pg_ctl stop` / `rm -rf /tmp/db`, insert **before** the `pg_ctl stop`
     (and before `rm -rf /tmp/db`), in this order, inside the same `RUN`
     (same `&&` / `;` style the stage already uses, running as the user the
     stage already uses):
     1. `mkdir -p /tmp/out`
     2. Compute the source hash:
        `( cd /tmp/db && find database.sql sgw resources persistent -type f | LC_ALL=C sort | xargs cat | sha256sum | cut -d' ' -f1 ) > /tmp/out/db-source.sha256`
     3. Image id:
        `printf '%s-%s\n' "$CIMMERIA_GIT_SHA" "$(cut -c1-12 /tmp/out/db-source.sha256)" > /tmp/out/image-id`
     4. Apply meta: `psql -v ON_ERROR_STOP=1 -U w-testing -d sgw -f /tmp/db/persistent/meta.sql`
     5. Insert the baked row:
        `psql -v ON_ERROR_STOP=1 -U w-testing -d sgw -c "INSERT INTO cimmeria_meta.db_state (state, image_id) VALUES ('baked', '$(cat /tmp/out/image-id)')"`
        If the wave 1 `meta.sql` already inserts a row, replace this INSERT with
        `UPDATE cimmeria_meta.db_state SET state='baked', image_id=...`. Check by
        reading `db/persistent/meta.sql`.
     6. Dump: `pg_dump -Fc -U w-testing -d sgw -f /tmp/out/fresh.dump`
     7. `cp /tmp/db/persistent/* /tmp/out/` is **not** wanted; copy the
        persistent dir in the runtime stage instead (step 3 below), so keep
        `db/persistent` available: change `rm -rf /tmp/db` so it keeps
        `/tmp/db/persistent`:
        `mkdir -p /tmp/out/persistent && cp -a /tmp/db/persistent/. /tmp/out/persistent/ && rm -rf /tmp/db`.
   - `/tmp/out` is then the stage's export dir.
3. **Runtime stage.** After the existing pgdata COPY lines (~249-259) add:
   ```dockerfile
   COPY --from=db-init /tmp/out/fresh.dump /tmp/out/image-id /tmp/out/db-source.sha256 /opt/cimmeria/db/
   COPY --from=db-init /tmp/out/persistent/ /opt/cimmeria/db/persistent/
   COPY docker/db-sync.sh docker/db-rebuild.sh /opt/cimmeria/bin/
   RUN chmod 0755 /opt/cimmeria/bin/db-sync.sh /opt/cimmeria/bin/db-rebuild.sh \
    && mkdir -p /var/backups/cimmeria \
    && chown postgres:postgres /var/backups/cimmeria
   ```
   Check how the existing pgdata is copied: the entrypoint (PP-05) reads the
   baked data from `/var/lib/postgresql/initial-data`. If the Dockerfile
   already COPYs the baked pgdata to `/var/lib/postgresql/initial-data`, keep
   that line unchanged. Note: the pgdata copied from `db-init` must be the one
   **after** the meta/baked-row step; since the new steps run inside the same
   `RUN` before `pg_ctl stop`, it is.
4. **s6 service.** In the block that copies the s6 service tree (~262-269) the
   `docker/s6/` directory already goes in wholesale; confirm that `COPY`
   picks up `docker/s6/db-sync/`. Then in the `RUN` that does
   `touch .../user/contents.d/...` add `db-sync` next to `postgres` and
   `cimmeria-server` (`touch /etc/s6-overlay/s6-rc.d/user/contents.d/db-sync`).
   In the `chmod` for the service scripts, add the oneshot's `up` file:
   `chmod 0755 /etc/s6-overlay/s6-rc.d/db-sync/up`. Use the same path prefix
   as the surrounding lines.
5. **ENV (~325-327).** Next to the other `S6_*` variables add
   `S6_BEHAVIOUR_IF_STAGE2_FAILS=2` (stop the container if a service fails).
   Check `S6_CMD_WAIT_FOR_SERVICES_MAXTIME`: if the Dockerfile sets it to a
   non-zero value, set it to `0` (no limit) so a long rebuild cannot kill
   startup; if it is unset, add `S6_CMD_WAIT_FOR_SERVICES_MAXTIME=0`.
6. **VOLUME (~334).** Change to
   `VOLUME ["/var/lib/postgresql/data", "/var/log/cimmeria", "/var/backups/cimmeria"]`.
7. **HEALTHCHECK (~344).** Change `--start-period=60s` to `--start-period=300s`.
   Update the adjacent comment to say it covers a resources rebuild.

### Checks

- Local: none possible (no docker). `git diff --stat` shows only
  `docker/Dockerfile`; verify no CR characters.
- CI only: the `pr-container.yml` image build and PP-08's scenarios.
  State in the report: "image build and boot verified in CI only".

### Tests that fail when reverted

PP-08 scenario 1 (first boot needs `fresh.dump`, `/opt/cimmeria/db/image-id`
and `db_state = baked`). Removing the bake makes `db-sync` log `adopt`, not
`first-boot`, failing it.

### Acceptance

The image builds; `/opt/cimmeria/db/{fresh.dump,image-id,db-source.sha256,persistent/}`
and `/opt/cimmeria/bin/*.sh` exist; `/var/backups/cimmeria` is a VOLUME;
`S6_BEHAVIOUR_IF_STAGE2_FAILS=2`; start-period 300s; the header comment is
accurate.

### Docs owed

None here (PP-10 owns `docs/operations/container.md`).

---

## PP-05 Entrypoint, s6 oneshot and the db-sync state machine

- **Agent:** packet-coder. Worktree `pp-05`, branch `persistent-player-data/pp-05-db-sync`.
- **Commit subject:** `feat(docker): keep pgdata across starts; db-sync oneshot state machine`
- **Files:** `docker/entrypoint.sh`, `docker/db-sync.sh` (new), `docker/s6/db-sync/type` (new), `docker/s6/db-sync/up` (new), `docker/s6/db-sync/timeout-up` (new), `docker/s6/db-sync/dependencies.d/postgres` (new, empty), `docker/s6/cimmeria-server/dependencies.d/db-sync` (new, empty).
- **Depends on:** the contract. `docker/db-rebuild.sh` is PP-06's; call it only as `/opt/cimmeria/bin/db-rebuild.sh <source_db> <image_id>`.

### Steps

1. **`docker/entrypoint.sh`.** Read it fully. The wipe is at lines ~58-69.
   - Replace the wipe block with:
     ```sh
     PGDATA="${PGDATA:-/var/lib/postgresql/data}"
     INITIAL=/var/lib/postgresql/initial-data
     if [ ! -f "$PGDATA/PG_VERSION" ]; then
       echo "entrypoint: empty pgdata, copying the baked cluster"
       mkdir -p "$PGDATA"
       cp -a "$INITIAL"/. "$PGDATA"/
     else
       echo "entrypoint: keeping existing pgdata"
     fi
     ```
     Keep whatever variable names and source path the current script already
     uses for the baked copy (read it; the Dockerfile COPYs it). Do not
     invent a path the Dockerfile does not create.
   - Keep the `chmod 700` of pgdata, the `chown` of the log dir and the
     Discord block, and the final `exec /init "$@"`. Add after the log
     chown: `mkdir -p /var/backups/cimmeria && chown postgres:postgres /var/backups/cimmeria`.
   - Rewrite the header comment (lines ~2-48): the entrypoint no longer wipes
     pgdata; it seeds the volume from the baked cluster only when
     `PG_VERSION` is absent; `db-sync` (s6 oneshot) then decides
     keep/adopt/rebuild; why: owner D2, a `docker restart` or image update
     must keep accounts and characters; a full wipe is removing the named
     volume by hand.
2. **s6 oneshot files** under `docker/s6/db-sync/` (all LF, no BOM):
   - `type`: the single line `oneshot` (newline-terminated).
   - `timeout-up`: `0`.
   - `dependencies.d/postgres`: empty file.
   - `up`: s6-rc oneshot `up` is parsed by execline, so it must be a single
     command line that runs a shell wrapper:
     `/command/with-contenv sh /opt/cimmeria/bin/db-sync.sh`
     (one line, no shebang line needed; it is not executed as a script).
     If Dockerfile `chmod`s `up` (PP-04 does), that is harmless.
   - `docker/s6/cimmeria-server/dependencies.d/db-sync`: empty file. The server
     run script's own `pg_isready`/`--check-db` wait stays.
3. **`docker/db-sync.sh`** (`#!/bin/sh`, `set -eu`). Structure, in this order:
   - Constants: `DB=sgw`, `IDFILE=/opt/cimmeria/db/image-id`,
     `BACKUP_DIR=/var/backups/cimmeria`, `KEEP="${CIMMERIA_DB_BACKUP_KEEP:-10}"`,
     `NEW_ID="$(cat "$IDFILE")"`. Validate `KEEP` is a positive integer, else
     log `[db-sync] ERROR bad CIMMERIA_DB_BACKUP_KEEP` and exit 1.
   - Run as the postgres OS user. Helper:
     `pgsu() { su-exec postgres "$@" 2>/dev/null || ...}` is **not** available
     everywhere: check what the image has (`grep -n "su-exec\|gosu\|runuser\|s6-setuidgid" docker/Dockerfile docker/s6/postgres/run`) and use the same
     mechanism the postgres `run` script uses. If it uses `s6-setuidgid postgres`,
     use `s6-setuidgid postgres`. Define `psqlq() { PGOPTIONS= <as-postgres> psql -h 127.0.0.1 -U w-testing -d "$1" -tAX -v ON_ERROR_STOP=1 -c "$2"; }` with `PGPASSWORD=w-testing`.
   - Wait for postgres: loop up to 120 times, `pg_isready -h 127.0.0.1 -q` then
     `sleep 1`; on timeout log an ERROR and `exit 1`.
   - `log() { echo "[db-sync] $*"; }`.
   - Read state with queries (each tolerant of a missing schema):
     - `HAS_META`: `SELECT to_regclass('cimmeria_meta.db_state') IS NOT NULL`.
     - `STATE`, `CUR_ID`: `SELECT state, image_id FROM cimmeria_meta.db_state` only if `HAS_META = t`.
   - **Backup function** `backup() { ... }`: name
     `sgw-$(date -u +%Y%m%dT%H%M%SZ)-${OLD_ID}.dump` where `OLD_ID` is `CUR_ID`
     or `pre-meta` when there is no meta; `pg_dump -Fc -h 127.0.0.1 -U w-testing -d sgw -f "$BACKUP_DIR/$name.part"`, then `mv` to the final name
     (atomic); on failure remove the `.part`, log ERROR, `exit 1` (fail
     closed: no backup, no rebuild). Retention:
     `ls -1t "$BACKUP_DIR"/sgw-*.dump | tail -n +$((KEEP+1)) | while read f; do rm -f "$f"; done`.
   - **Decision**, in the contract order:
     1. restore: if `CIMMERIA_DB_RESTORE` is non-empty. Resolve to a file: if it
        contains `/` use it as is, else `$BACKUP_DIR/$CIMMERIA_DB_RESTORE`.
        Missing file: log ERROR, exit 1. Idempotency marker: table
        `cimmeria_meta.restore_done(file text primary key, at timestamptz default now())`
        created on demand with `CREATE TABLE IF NOT EXISTS` **in the restored
        DB** after the restore. If the live DB (when it has `cimmeria_meta`)
        already has this file name in `restore_done`, fall through to the
        next rule. Otherwise: terminate other connections to `sgw`, make a
        pre-restore backup via `backup`, create `sgw_restore`, `pg_restore
        --no-owner -d sgw_restore "$file"`, create `cimmeria_meta.restore_done`
        and insert the file name in `sgw_restore`, then rename `sgw` to
        `sgw_prev` (dropping an older `sgw_prev` first) and `sgw_restore` to
        `sgw` in one connection to the `postgres` database; log
        `[db-sync] restore <file>`; `exit 0`. Any failure drops `sgw_restore`
        and exits non-zero leaving `sgw` as it was. Keep this branch small.
     2. first-boot: `HAS_META = t` and `STATE = baked`:
        `UPDATE cimmeria_meta.db_state SET state='live', applied_at=now()`;
        log `[db-sync] first-boot image_id=<CUR_ID>`; exit 0.
     3. adopt: `HAS_META = f`: `backup` (name uses `pre-meta`), log
        `[db-sync] adopt (no cimmeria_meta; keeping player data)`, then
        `/opt/cimmeria/bin/db-rebuild.sh sgw "$NEW_ID"`; on non-zero, log
        `[db-sync] ERROR rebuild failed, sgw untouched` and `exit 1`.
     4. rebuild: `FORCE` true (`1|true|yes|TRUE|True`) or `CUR_ID != NEW_ID`:
        `backup`, log `[db-sync] rebuild reason=<forced|image-changed> old=<CUR_ID> new=<NEW_ID>`,
        run `db-rebuild.sh sgw "$NEW_ID"`; on non-zero record the failure
        (`UPDATE cimmeria_meta.db_state SET last_failed_image_id=$NEW_ID, last_failed_at=now()`
        executed against the untouched `sgw`, ignoring errors from that update)
        then log ERROR and `exit 1`.
        Backup reuse: if `last_failed_image_id = NEW_ID` and a backup file newer
        than `last_failed_at` exists, skip making another backup (avoid a
        backup per crash-loop restart); log `[db-sync] reusing backup <file>`.
     5. skip: log `[db-sync] skip image_id=<CUR_ID>`; exit 0.
   - No code path may `DROP DATABASE sgw` or wipe the volume. `sgw` is only
     ever renamed (restore, rebuild swap).
4. Do not touch `docker/db-rebuild.sh`, `Dockerfile`, or compose.

### Checks

- Local: `Get-Content` the files; confirm LF only, `#!/bin/sh`, and run this
  PowerShell bashism scan: `Select-String -Path docker/db-sync.sh -Pattern '\[\[|\bfunction \w|echo -e|<\(|\$\{[^}]*//|declare|local -a|pipefail'` must return nothing.
- If `shellcheck` is installed, `shellcheck -s sh docker/db-sync.sh`; it is not
  required. CI has no shellcheck step on `docker/*.sh`; the review checks by
  hand and PP-08 runs the script.
- CI only: everything behavioural (PP-08 scenarios).

### Tests that fail when reverted

PP-08 scenario 2 (restart keeps the sentinel: fails if the entrypoint wipe
returns) and scenario 3 / 5 / 6 / 7 for each branch.

### Acceptance

POSIX sh only; a restart of the same image logs `skip` and touches nothing;
every non-zero path leaves `sgw` untouched; log prefixes match the contract.

### Docs owed

None here (PP-10).

---

## PP-06 The container rebuild pipeline

- **Agent:** database-persistence. Worktree `pp-06`, branch `persistent-player-data/pp-06-db-rebuild`.
- **Commit subject:** `feat(docker): blue/green resources rebuild with player-data carry-over`
- **Files:** `docker/db-rebuild.sh` only (new).
- **Depends on:** wave 1: `db/persistent/reconcile.sql`, `seed-accounts-stage.sql`, `seed-accounts.sql`, `reseed-public-tables.txt`, `persistent-public-tables.txt`. Read each before writing the step that uses it, to confirm the interface (session setting `cimmeria.image_id`, expected input tables, whether they `RAISE EXCEPTION` on failure).

### Steps

`#!/bin/sh`, `set -eu`. Usage: `db-rebuild.sh <source_db> <image_id>`. Constants:
`SRC="$1"`, `ID="$2"`, `NEXT="${SRC}_next"`, `PREV="${SRC}_prev"`, `P=/opt/cimmeria/db/persistent`, `FRESH=/opt/cimmeria/db/fresh.dump`, `WORK=$(mktemp -d)` with
`trap 'rm -rf "$WORK"' EXIT`. Helpers: `PSQL="psql -h 127.0.0.1 -U w-testing -v ON_ERROR_STOP=1 -X -q"`,
`export PGPASSWORD=w-testing`, `fail() { echo "[db-rebuild] ERROR $*" >&2; drop_next; exit 1; }`,
`drop_next() { $PSQL -d postgres -c "DROP DATABASE IF EXISTS \"$NEXT\"" || true; }`. Install `trap 'rc=$?; [ $rc -ne 0 ] && drop_next; rm -rf "$WORK"' EXIT` so any `set -e` failure also drops `sgw_next`. `export PGOPTIONS="-c cimmeria.image_id=$ID"` for every psql/pg_restore that runs the SQL files.
Run as the same OS user as db-sync (it is invoked from it).

1. **Drop leftovers:** `DROP DATABASE IF EXISTS "$NEXT"`.
2. **Create and restore fresh:** `CREATE DATABASE "$NEXT"`; `pg_restore --no-owner --exit-on-error -h 127.0.0.1 -U w-testing -d "$NEXT" "$FRESH"`. For speed, apply
   `ALTER DATABASE "$NEXT" SET synchronous_commit = off` right after CREATE
   (`sgw_next` is disposable until the swap; the setting is reset before the swap, step 9).
3. **Pre-checks:** if `reconcile.sql` defines a pre-check phase, run it here per
   its header; otherwise skip this step and say so in a comment `# step 3: none`.
4. **Carry player data.**
   - `pg_dump -Fc -h 127.0.0.1 -U w-testing -d "$SRC" -f "$WORK/src.dump"`.
   - `pg_restore -l "$WORK/src.dump" > "$WORK/toc.full"`.
   - Build the TOC filter plainly, with awk, keeping a TOC line only if it is
     a `TABLE DATA` line whose schema is in the **player schemas** list OR is
     `public account`; and, for `public` tables in
     `persistent-public-tables.txt`, also keep them. TOC TABLE DATA lines look
     like `3456; 0 16409 TABLE DATA public account w-testing`: fields 5 and 6
     (after `TABLE DATA`) are schema and table. Rule, in words so PP-13 can
     port it: keep a line iff it contains ` TABLE DATA ` and (schema is one of
     the player schemas, or `schema.table` is `public.account`, or
     `public.table` is listed in `persistent-public-tables.txt`) and
     `schema.table` is not listed in `reseed-public-tables.txt`. Drop
     everything else (including `SEQUENCE SET` lines; sequences are fixed in
     step 6). Player schemas: read them from the first non-comment lines of
     `persistent-public-tables.txt` if that file defines them; otherwise use
     the schema names found in `db/sgw/*` directory names that hold player
     data (`Players`, `Accounts`, ...): confirm against the wave 1 file and
     write the list as one `PLAYER_SCHEMAS="..."` variable at the top of the
     script with a comment.
   - Exclude list: read `reseed-public-tables.txt` (one `schema.table` per
     line, `#` comments, blank lines ignored; strip CR).
   - `pg_restore --data-only --disable-triggers --exit-on-error -L "$WORK/toc.filtered" -h 127.0.0.1 -U w-testing -d "$NEXT" "$WORK/src.dump"`.
     A COPY rejected by a non-additive schema change fails here: `fail "carry-over rejected: <last error>"` (D-PP7).
   - Before the restore, in `sgw_next`, if the player tables already hold the
     seeds from `fresh.dump` that would conflict, `TRUNCATE` them first
     (list from `reseed-public-tables.txt` is NOT truncated; they are
     reseed tables). Read `seed-accounts-stage.sql` to see how it expects
     `public.account` to look; the carry brings in the live accounts, so
     `public.account` in `sgw_next` is truncated (`CASCADE`-free) just before
     the data restore if `fresh.dump` seeded it.
4a. `$PSQL -d "$NEXT" -f "$P/seed-accounts-stage.sql"`.
5. `$PSQL -d "$NEXT" -f "$P/reconcile.sql"` (reads `cimmeria.image_id`).
5a. `$PSQL -d "$NEXT" -f "$P/seed-accounts.sql"` (insert-only; must not touch existing accounts).
6. **Sequences:** for every sequence owned by a column in a carried table,
   `SELECT setval(seq, GREATEST(max(col), 1))`. Generate with
   `SELECT format('SELECT setval(%L, COALESCE((SELECT max(%I) FROM %I.%I), 1))', pg_get_serial_sequence(...) ...)`
   from `information_schema.columns` where `column_default LIKE 'nextval%'`
   and schema is a player schema or `public` tables kept; run the generated
   statements with `psql -At -f`. Fail on error.
7. `ANALYZE` in `$NEXT`.
8. **Count check:** for each carried table, compare `count(*)` in `$SRC` and
   `$NEXT`. Expected: `src - logged deletes (reconcile log rows for that
   table) + logged seed-account inserts (public.account only)`. Read how
   `reconcile.sql` / `seed-accounts.sql` log their rows (a table
   `cimmeria_meta.reconcile_log` or similar) and compute from that. Mismatch:
   `fail "count mismatch table=<t> src=<n> next=<n> expected=<n>"`.
9. **Write state:** `UPDATE cimmeria_meta.db_state SET state='live', image_id='$ID', applied_at=now()`
   in `$NEXT`; `ALTER DATABASE "$NEXT" RESET synchronous_commit`.
10. **Swap**, connected to `postgres`: `DROP DATABASE IF EXISTS "$PREV"`;
    `SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE datname IN ('$SRC','$NEXT') AND pid <> pg_backend_pid()`;
    then `ALTER DATABASE "$SRC" RENAME TO "$PREV"` and
    `ALTER DATABASE "$NEXT" RENAME TO "$SRC"`; retry the pair up to 5 times
    with `sleep 2` if the rename reports the database is being accessed (the
    healthcheck and the server reconnect). If the second rename fails, rename
    `$PREV` back to `$SRC` and fail. Only after both succeed echo
    `[db-rebuild] swapped; previous database kept as $PREV`.

Echo one `[db-rebuild] step N ...` line per step. On any failure the single
line is `[db-rebuild] ERROR <cause>` on stderr.

### Checks

- Local: LF only; the PowerShell bashism scan from PP-05; no `DROP DATABASE`
  of `"$SRC"` anywhere (`Select-String 'DROP DATABASE.*\$SRC'` returns nothing).
- CI only: PP-08 scenarios 3, 4, 5, 6.
- A second read-only `database-persistence` design pass is requested by the
  reviewer (work-packets.md).

### Tests that fail when reverted

PP-08 scenario 3 (resources restored, sentinel kept, seed account back, backup
and `sgw_prev` exist), scenario 4 (dangling row removed and logged) and
scenario 5 (failure leaves `sgw` intact).

### Acceptance

Any error, including a rejected COPY, drops `sgw_next` and exits non-zero with
a one-line cause; the count check matches; `sgw_prev` exists after a swap;
`sgw` is never dropped.

### Docs owed

None here (PP-10).

---

## PP-07 Compose, env example and dependabot

- **Agent:** packet-coder. Worktree `pp-07`, branch `persistent-player-data/pp-07-compose`.
- **Commit subject:** `feat(docker): external pgdata volume and backup mount in compose`
- **Files:** `docker/compose.yml`, `docker/.env.example`, `.github/dependabot.yml`.
- **Depends on:** the contract (volume name, env names).

### Steps

1. **`docker/compose.yml`** (read it first).
   - In the `cimmeria-server` service `volumes:` (lines ~125-129, currently
     only the logs bind), add:
     ```yaml
           - cimmeria-server-pgdata:/var/lib/postgresql/data
           - ${CIMMERIA_BACKUP_DIR:-./backups}:/var/backups/cimmeria
     ```
     matching the existing indentation and quoting style.
   - In the service `environment:` add passthroughs, matching the existing
     style (`- NAME=${NAME:-default}` or map form, whichever the file uses):
     `CIMMERIA_DB_FORCE_REBUILD=${CIMMERIA_DB_FORCE_REBUILD:-}`,
     `CIMMERIA_DB_RESTORE=${CIMMERIA_DB_RESTORE:-}`,
     `CIMMERIA_DB_BACKUP_KEEP=${CIMMERIA_DB_BACKUP_KEEP:-10}`.
   - Add a top-level block after the existing `networks:`:
     ```yaml
     volumes:
       # Player data lives here and survives `docker restart`, image updates
       # and `docker compose down -v` (an external volume is never removed by
       # compose). Create it once: docker volume create cimmeria-server-pgdata
       # Removing it by hand is the deliberate full wipe.
       cimmeria-server-pgdata:
         external: true
         name: cimmeria-server-pgdata
     ```
   - Rewrite the stale comments at ~104-106 and ~122-124 (they say the DB is
     rebuilt from the image on every start / that no state is persisted):
     the database now persists on `cimmeria-server-pgdata`; each image update
     rebuilds only the resources tables and keeps accounts and characters;
     backups go to `CIMMERIA_BACKUP_DIR` (newest `CIMMERIA_DB_BACKUP_KEEP`).
   - Watchtower comment at ~152 (`WATCHTOWER_REMOVE_VOLUMES`): Watchtower's
     `--remove-volumes` removes only the old container's anonymous volumes;
     an external named volume is not anonymous, so it is never removed. Say
     exactly that, and note the report must say how it was verified (reading
     the Watchtower docs/flag behaviour in the repo, not a run). Do not
     change the setting's value.
2. **`docker/.env.example`**: append a section:
   ```
   # --- Database persistence --------------------------------------------
   # Host directory for pre-rebuild database backups (newest N kept).
   # CIMMERIA_BACKUP_DIR=./backups
   # How many backups to keep.
   # CIMMERIA_DB_BACKUP_KEEP=10
   # Set to 1 for one start to rebuild resources from the image seeds
   # (resources only; player data kept). Unset it afterwards.
   # CIMMERIA_DB_FORCE_REBUILD=
   # Disaster recovery only; leave unset. File name under the backup dir
   # to restore over the live database (a safety backup is taken first).
   # CIMMERIA_DB_RESTORE=
   ```
3. **`.github/dependabot.yml`**: in the docker ecosystem block (lines ~16-27)
   add (indent to match the block):
   ```yaml
       ignore:
         - dependency-name: "postgres"
           update-types: ["version-update:semver-major"]
   ```
   (A Postgres major needs a pg_upgrade plan; owner D7.) Keep YAML valid.

### Checks

- Local: LF only; parse YAML with
  `python -c "import yaml,sys;[yaml.safe_load(open(f)) for f in sys.argv[1:]]" docker/compose.yml .github/dependabot.yml`
  (if Python or PyYAML is missing, say so and skip).
- CI only: `docker compose config` runs in PP-08's job (step below in PP-08).
- No change to ports or to existing env entries.

### Tests that fail when reverted

PP-08 scenario 8 (`docker rm -v` / compose down -v leaves the external volume)
and the whole persistence smoke if the volume mount is dropped.

### Acceptance

`compose.yml` mounts the external volume and the backup bind, passes the three
env vars, comments are accurate; `.env.example` documents them;
dependabot ignores postgres majors.

### Docs owed

None here (PP-10).

---

## PP-08 CI persistence smoke

- **Agent:** testing-validation-engineer. Worktree `pp-08`, branch `persistent-player-data/pp-08-persistence-smoke`.
- **Commit subject:** `test(ci): container persistence smoke`
- **Files:** `tools/container-persistence-smoke.sh` (new), `.github/workflows/pr-container.yml`, `docker/Dockerfile.dockerignore` (see Design issues 4: add the new script next to `tools/container-smoke.sh` in the exclude list).
- **Depends on:** the contract; runs against the combined wave 2 branch.

### Script design

Use `tools/container-smoke.sh` as the style template (read it): `set -u`,
`err()`, a cleanup trap printing `::group::` log blocks, and its psql helper.

`tools/container-persistence-smoke.sh <image> <logs-dir>`:

- Names: `VOL=cimmeria-persist-smoke-pgdata`, `BAK` temp dir (`mktemp -d`,
  `chmod 777` so the container's postgres user can write), `NAME=cimmeria-persist`.
  The smoke uses its **own** volume name, never `cimmeria-server-pgdata`.
- Helpers:
  - `start()` runs `docker run -d --name "$NAME" -v "$VOL:/var/lib/postgresql/data" -v "$BAK:/var/backups/cimmeria" "$@" "$IMAGE"` (extra `-e` args from callers).
  - `wait_db()` waits (max 300 s, 2 s steps) until `docker exec "$NAME" pg_isready -h 127.0.0.1 -q` succeeds **and** `[db-sync]` appears in `docker logs`; fails with `err` if the container exits first.
  - `wait_exit()` waits (max 300 s) for status `exited`.
  - `psql_in() { docker exec -e PGPASSWORD=w-testing "$NAME" psql -h 127.0.0.1 -U w-testing -d "${DB:-sgw}" -tAX -v ON_ERROR_STOP=1 -c "$1"; }`
  - `dbsync_lines() { docker logs "$NAME" 2>&1 | grep '^\[db-sync\]'; }` and
    `assert_log() { dbsync_lines | grep -q "$1" || err "expected '[db-sync] $1'"; }` (use the last boot's lines: record `docker logs --since` timestamp before each restart, or count lines before and compare with `tail -n +N`).
  - `assert_eq()` compares a psql result to an expected string.
  - `recreate()` = `docker rm -f "$NAME"` then `start` (data lives on the volume).
- Cleanup trap: copy `docker logs` and `/var/log/cimmeria` to the logs dir,
  `docker rm -f`, `docker volume rm -f "$VOL"`, `rm -rf "$BAK"`.

Scenarios (each asserts with `psql_in` and `[db-sync]` lines):

1. **first boot:** `docker volume create "$VOL"`; `start`; `wait_db`;
   assert `first-boot`; `SELECT count(*) FROM public.account` >= 14; `SELECT state FROM cimmeria_meta.db_state` = `live`.
2. **restart:** insert a sentinel. Create player, item and inventory rows
   without enumerating columns by copying player 62 (read
   `db/sgw/Players/Seed/sgw_player.sql` to confirm the id exists and which
   account it belongs to; characters exist only for accounts 2-9):
   ```sql
   CREATE TEMP TABLE s AS SELECT * FROM sgw_player WHERE character_id = 62;
   UPDATE s SET character_id = 900000001, name = 'PersistSentinel', account_id = 2 ...;
   INSERT INTO sgw_player SELECT * FROM s;
   ```
   Use the exact primary key and unique columns of `sgw_player` (character
   id, character name, any `account_id` + slot uniqueness): read the table
   DDL under `db/sgw/Players/` and set every unique column to a value that
   cannot collide. Then `docker restart "$NAME"`, `wait_db`, assert `skip`
   in the new boot's lines, and the sentinel row still present.
3. **forced rebuild with seed-account top-up:** pick account ids from
   `db/sgw/Accounts/Seed/account.sql`: accounts 10-15 own **no** characters
   (characters exist only for accounts 2-9). Use account 15 for the delete
   and account 13 for the change. Mutate a resources row (`UPDATE resources.items SET <name column> = 'MUTATED' WHERE item_id = (SELECT min(item_id) FROM resources.items)`; read the table DDL and use a text column that exists, record the original value first); `DELETE FROM public.account WHERE account_id = 15`;
   `UPDATE public.account SET accesslevel = 9 WHERE account_id = 13`. Then
   `recreate -e CIMMERIA_DB_FORCE_REBUILD=1`, `wait_db`, assert `rebuild`.
   Assert: the resources row equals the recorded original; the sentinel is
   **kept** (D3); account 15 exists again; exactly one `insert_seed_account`
   log row (query the reconcile log table named in `reconcile.sql` /
   `seed-accounts.sql`, read them for the exact table and `action` column);
   account 13 still has `accesslevel = 9` (D6, existing accounts untouched);
   `ls "$BAK"/sgw-*.dump` has a file; `SELECT 1 FROM pg_database WHERE datname='sgw_prev'` = 1. Remember this backup file name for scenario 7.
4. **dangling:** insert a temporary `resources.items` row (copy a row, new
   `item_id` >= 10000 as the inventory requires, unique name) and an
   `sgw_inventory` row for the sentinel (`item_id` = that row, a unique
   `(character_id, container_id, slot_id)`, copying an existing inventory
   row's other columns the same way as the player). Then
   `DELETE`-free: since a rebuild reseeds resources from the image, the
   temporary item vanishes from `resources.items` on the rebuild. Force a
   rebuild (as step 3), assert the inventory row is gone and the log table
   has a delete row for it (JSON log, D5).
5. **fail closed:** `ALTER TABLE sgw_player DROP CONSTRAINT level_sanity`
   (read the actual CHECK name under `db/sgw/Players/`), insert a sentinel2
   row with `level = 99`, then `recreate -e CIMMERIA_DB_FORCE_REBUILD=1`,
   `wait_exit`; assert exit code non-zero, `[db-sync] ERROR` in the logs, and
   that `sgw_next` does not exist. Then `recreate` **without** the force flag:
   `wait_db`... note this start also sees a failed state; db-sync must not
   rebuild unless forced or the image id changed. The image id has not
   changed since scenario 3, so assert `skip` and that sentinel2 is still
   there (`sgw` intact). Clean up: delete sentinel2 and re-add the CHECK
   only if it can be added back cleanly (`NOT VALID` is acceptable).
6. **adopt:** `DROP SCHEMA cimmeria_meta CASCADE`; `docker restart`; assert
   `adopt`; sentinel still there; `cimmeria_meta.db_state` exists again.
7. **restore (DR):** delete the sentinel; `recreate -e CIMMERIA_DB_RESTORE=<scenario 3 backup file name>`; `wait_db`; assert `restore`; the sentinel is back; then
   `docker restart "$NAME"` (the env var is still set because `docker restart`
   keeps the container's env) and assert the new boot logs `skip`, not `restore`.
8. **full wipe:** `docker rm -f "$NAME"`; `docker volume rm "$VOL"`;
   `docker volume create "$VOL"`; `start`; assert `first-boot` and the
   sentinel is gone. Also: `docker rm -v "$NAME"` after that start, then
   `docker volume inspect "$VOL"` must still succeed (the named volume
   survives `rm -v`; compose `down -v` ignores external volumes).

Exit 0 only if all asserts pass; print `PASS scenario N` after each.

### Workflow edits (`.github/workflows/pr-container.yml`)

- PR `paths` (lines ~28-34): add `'db/sgw/**'`, `'db/persistent/**'`,
  `'tools/container-persistence-smoke.sh'`. Push `paths` (~35-52): add
  `'tools/container-persistence-smoke.sh'` (it already has `db/**`).
  Update the header comment's PR path list sentence to mention the persistence
  smoke and the DB seeds.
- After the `dbcheck` refusal step (ends ~line 231) and before the `report`
  job, add two steps in `build-and-smoke`, using the same SHA-pinned
  `actions/upload-artifact@043fb46d1a93c77aae656e7c1c64a875d1fc6a0a # v7`:
  ```yaml
        - name: validate docker/compose.yml
          run: docker compose -f docker/compose.yml config -q
          env:
            CIMMERIA_BACKUP_DIR: ${{ runner.temp }}/backups

        - name: persistence smoke (keep / rebuild / restore / wipe)
          run: |
            bash tools/container-persistence-smoke.sh \
              "cimmeria-server:pr-${{ github.event.pull_request.number || 'dispatch' }}" \
              "${{ runner.temp }}/container-persistence-logs"

        - name: upload persistence smoke logs
          if: failure()
          uses: actions/upload-artifact@043fb46d1a93c77aae656e7c1c64a875d1fc6a0a # v7
          with:
            name: container-persistence-logs
            path: ${{ runner.temp }}/container-persistence-logs
            if-no-files-found: ignore
            retention-days: 14
  ```
  The compose `config` step needs the external volume to exist only at `up`,
  not at `config`; if `config -q` complains about the external volume in the
  CI docker version, add `docker volume create cimmeria-server-pgdata` before
  it and `docker volume rm cimmeria-server-pgdata` after (`if: always()`).
  Indentation: steps are at 6 spaces under `steps:`; the snippets above show
  8 only because of this document's list nesting; match the file.
- Raise `timeout-minutes` from 30 to 45 (the smoke adds several 1-5 minute
  boots). Update its comment.

### Checks

- Local: LF only for the script and the workflow; the PowerShell bashism scan
  from PP-05 on the script. `python -c "import yaml;yaml.safe_load(open('.github/workflows/pr-container.yml'))"`
  if Python and PyYAML exist. The script and the docker run are **CI only**.
- Report notes: the exact seed account ids used (15 deleted, 13 changed), the
  CHECK name dropped, the table/column names read from the DDL.

### Tests that fail when reverted

This is the integration test. It fails when the entrypoint wipe comes back
(scenario 2 loses the sentinel), when `db-sync` never reports `skip`
(scenario 2), when the rebuild wipes player data (scenario 3), or when the
fail-closed path touches `sgw` (scenario 5). The review-fixer proves the wipe
case once with a throwaway commit in CI, or argues it from scenario 2.

### Acceptance

The job is green on the combined wave 2 branch; all eight scenarios print
PASS; compose config is valid; logs upload on failure.

### Docs owed

`tools/README.md` catalog row for the new script and the TESTING.md
container-tests entry belong to PP-16 (note in the report).

---

## Design issues (found while writing; the design was not changed)

1. **s6 oneshot `up` is execline, not sh.** The `up` file is parsed by
   execline. PP-05 therefore makes it a one-line wrapper that execs
   `with-contenv sh db-sync.sh`. The work-packets text says "runs under
   with-contenv"; this is how.
2. **db-sync must run as the postgres OS user for `pg_dump`/backup files**, and
   the bind-mounted `/var/backups/cimmeria` needs `postgres` write access. A
   host bind created by Docker is root-owned and the entrypoint `chown` fixes
   it, but a read-only or rootless bind would fail closed (db-sync exits 1).
   Worth stating in the operator doc (PP-10).
3. **`S6_BEHAVIOUR_IF_STAGE2_FAILS=2` needs CI proof.** Scenario 5 asserts the
   container stops when db-sync fails; if s6 keeps the container up instead,
   the scenario fails and the value or the server `dependencies.d` is wrong.
   Not provable without docker.
4. **`docker/Dockerfile.dockerignore` excludes `tools/container-smoke.sh`** and
   does not name the new script; the build context ignores `tools/` content
   anyway unless COPYd, so it is harmless, but the exclude list should name
   the new script for symmetry. PP-08 touches that file, which is outside the
   work-packets file list for PP-08 (`tools/...` and the workflow only).
5. **The 30-minute CI timeout is too short** for the build plus eight
   scenarios with 300 s boot ceilings; PP-08 raises it to 45. Cold-cache runs
   may need more.
6. **TOC filter for sequence sets.** `pg_restore --data-only` with a `-L`
   list drops `SEQUENCE SET` lines when they are not selected; PP-06 resets
   sequences in step 6 itself. The contract does not say which; make it
   explicit in the contract so PP-13 does the same.
7. **`last_failed_*` bookkeeping on a forced failure.** After a failed forced
   rebuild the next unforced boot logs `skip` (image id unchanged) and the
   operator is not told that the forced rebuild failed. Consider a louder
   `[db-sync] skip (last rebuild failed at <time>)` line.
8. **Dependabot digest pins.** If the Dockerfile pins `postgres` by digest,
   the `version-update:semver-major` ignore only filters tag bumps; check the
   `FROM` lines.
9. **Seed account 15 is `charlie`** and ids 10-14 are lab accounts with
   `test_account = true`; if a future seed gives them characters, scenario 3's
   delete would cascade or fail. The script reads the seed at run time to
   guard this (assert the account owns no characters before deleting).
