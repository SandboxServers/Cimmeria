# Persistent player data: wave 1 packets

> Type: packets (prescriptive). Campaign: [README.md](../README.md). Architect's requirements and the contract: [work-packets.md](../work-packets.md). Owner answers: [OWNER-DECISIONS.md](../OWNER-DECISIONS.md).
>
> Wave 1 = PP-01, PP-17 (stacked on PP-01), PP-02, PP-03, PP-15. One packet-review-fixer covers all five; ops ships one PR.

## Shared header (every packet inlines this)

### Repo rules you must follow

1. Work only in your own worktree (path in your brief). Never touch the root checkout, and never touch the play database `sgw`. Your test database is `sgw_pp_<nn>`.
2. PowerShell only, absolute paths, no bash. Start every tool call with `Set-Location <worktree>;`.
3. Never run `cargo` directly. Every compile goes through the lane, in this order: fmt, clippy, tests. Run them in the foreground. Check that the lane log lands under `logs/<worktree>/`.
   - `pwsh tools/build-lane/lane.ps1 cargo fmt -p <crate>`
   - `pwsh tools/build-lane/lane.ps1 cargo clippy -p <crate> --all-targets -- -D warnings`
   - `pwsh tools/build-lane/lane.ps1 cargo nextest run -p <crate>`
   - live-DB tests: `pwsh tools/build-lane/live-db-test.ps1 <test-name substring>`
   - Plain pwsh tests (`tools/build-lane/test-*.ps1`) are not cargo: `pwsh -NoProfile -File <absolute path>`.
4. Live-DB tests: gate with `require_db_or_skip!()`, put `live_db` in the fn name, take the URL from the test pool (never `DATABASE_URL`), and use `0x7000_xxxx` / `i32`-sized sentinels. Clean up by exact sentinel.
5. File caps: soft 500 lines, hard 700. Split along natural seams.
6. Docs under `docs/` are CRLF. Keep them CRLF. Never hand-edit a `<!-- gen:NAME -->` block.
7. Seeds are the source of truth: no `db/scripts/*.sql` migrations. Never touch `db/database.sql`, and never reference `cimmeria_meta`, `cimmeria_seed` or `db/persistent/` from anything under `db/sgw/`, `db/resources/` or `db/database.sql`.
8. Git: one local commit with the subject given in the packet and these two trailer lines at the end of the message. No `git stash`, no `git worktree prune`, no push, no PR.
   - `Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>`
   - `Claude-Session: https://claude.ai/code/session_01HmEMKDEBUMeCrR5YzkAMZJ`
9. No IPs, credentials or account names in any file you write (the sentinel names below are fine).
10. Report: write the full report to `<worktree>\.claude\reports\persistent-player-data\<packet>-coder.md` (for a domain agent, `-<role>.md`), then return exactly one line: `<packet> <done|blocked|failed> <commit-sha|-> <report-path>`. The report must say which tests you ran, which you could not run, and for every new test how it fails when the change is reverted (and that you checked it).

### Contract excerpt (shared names; do not rename)

Schema `cimmeria_meta` (created by `db/persistent/meta.sql`, owner PP-01):

```sql
cimmeria_meta.db_state (
  id                   smallint PRIMARY KEY CHECK (id = 1),
  state                text NOT NULL CHECK (state IN ('baked','live')),
  image_id             text NOT NULL,
  db_source_sha256     text NOT NULL,
  first_seeded_at      timestamptz,
  last_rebuild_at      timestamptz,
  rebuild_count        integer NOT NULL DEFAULT 0,
  last_failed_image_id text,
  last_failed_at       timestamptz,
  last_restore_sha256  text
)
cimmeria_meta.reconcile_log (
  id          bigserial PRIMARY KEY,
  logged_at   timestamptz NOT NULL DEFAULT now(),
  image_id    text NOT NULL,
  action      text NOT NULL,   -- 'delete_dangling_item' | 'relocate_player' | 'count_dangling_ref' | 'sequence_fixup'
                               -- | 'insert_seed_account' | 'skip_seed_account'
  table_name  text NOT NULL,
  detail      jsonb NOT NULL
)
```

`reconcile.sql` and `seed-accounts.sql` read the image id with `current_setting('cimmeria.image_id')`. The smokes set it with `SET LOCAL`.

Rebuild order (context only; wave 2 wires it): fresh restore, `seed-accounts-stage.sql` (4a), carry-over of player data, `seed-accounts.sql` (5a), `reconcile.sql` (6), post-data (PKs, FKs, indexes). The SQL files therefore run **without PKs, FKs or indexes** and must also work in a full test DB.

Dangling-id policy (D-PP9, owner D5): items whose `type_id` is missing from `resources.items` are deleted with the full row logged as JSON. A player in a removed world moves to world `Castle_CellBlock` (looked up by name) at -334.231, 73.472, -228.026, logged. If that world is missing the SQL RAISEs. Non-FK ids are left in place and counted.

---

## PP-01 Meta schema, reconcile SQL and the reconcile smoke

- **Agent:** database-persistence. Worktree `pp-01`. Test DB `sgw_pp_01`.
- **Commit subject:** `feat(db): cimmeria_meta schema and persistent-data reconcile SQL`
- **Branch/worktree:** created by ops: `pwsh tools/build-lane/mk-worktree.ps1 persistent-player-data/pp-01-reconcile-sql pp-01`.

### Design choices already made (do not revisit)

- The SQL files use **unqualified names for `public` tables** (`sgw_player`, `sgw_inventory`, ...) and qualified names for `resources.*` and `cimmeria_meta.*`. In production `search_path` is `public`, so the effect is identical. The smoke puts a scratch schema first on `search_path` so it never locks or modifies the shared test tables.
- `DELETE FROM ONLY` is used on purpose: `sgw_inventory` inherits `sgw_inventory_base` (`CREATE TABLE sgw_inventory (...) INHERITS (sgw_inventory_base)`). Each table is cleaned separately and each row is logged under the table it lives in.
- `reconcile.sql` logs `count_dangling_ref` only when the count is greater than 0, and logs `sequence_fixup` only when it changed a value. A normal rebuild therefore logs nothing.
- Missing fallback world always RAISEs, even when no player needs it (fail closed, D-PP9).

### Files

1. Create `db/persistent/meta.sql`.
2. Create `db/persistent/reconcile.sql`.
3. Create `tools/persistent_reconcile_smoke.sql`.
4. Edit `crates/base/src/base/smoke_tests.rs` (132 lines now; stays well under the cap).

Before writing, confirm these facts with Grep/Read and stop with `blocked` if one is false:

- `db/resources/**/Tables/` defines `stargates(stargate_id)`, `abilities(ability_id)`, `disciplines(discipline_id)`, `blueprints(blueprint_id)`, `respawners(respawner_id)`, `missions(mission_id)`, `mission_steps(step_id)`, `items(item_id)` and `worlds(world_id, world)`.
- `db/sgw/Inventory/Tables/sgw_inventory.sql` and `sgw_inventory_base.sql` exist and `sgw_inventory` has `INHERITS (sgw_inventory_base)`.

### Step 1: `db/persistent/meta.sql`

Plain SQL, no `\` commands, no BEGIN/COMMIT. Idempotent.

```sql
-- cimmeria_meta: bookkeeping for the persistent-player-data rebuild (campaign
-- persistent-player-data). Never referenced from db/database.sql. Runs as the DB
-- owner; safe to run twice.
CREATE SCHEMA IF NOT EXISTS cimmeria_meta;

CREATE TABLE IF NOT EXISTS cimmeria_meta.db_state (
    id                   smallint PRIMARY KEY CHECK (id = 1),
    state                text NOT NULL CHECK (state IN ('baked', 'live')),
    image_id             text NOT NULL,
    db_source_sha256     text NOT NULL,
    first_seeded_at      timestamptz,
    last_rebuild_at      timestamptz,
    rebuild_count        integer NOT NULL DEFAULT 0,
    last_failed_image_id text,
    last_failed_at       timestamptz,
    last_restore_sha256  text
);

CREATE TABLE IF NOT EXISTS cimmeria_meta.reconcile_log (
    id         bigserial PRIMARY KEY,
    logged_at  timestamptz NOT NULL DEFAULT now(),
    image_id   text NOT NULL,
    action     text NOT NULL,
    table_name text NOT NULL,
    detail     jsonb NOT NULL
);
```

Copy the column list exactly from the contract excerpt above. Comment each `action` value in the file as in the contract.

### Step 2: `db/persistent/reconcile.sql`

Plain SQL plus `DO` blocks, no transaction control, no `\` commands. Four DO blocks in this order, each starting with a header comment. Each block reads the image id with `current_setting('cimmeria.image_id')` (an unset setting raises, which is wanted). Use this shape.

**Block 1, dangling items:**

```sql
DO $reconcile_items$
DECLARE
    img text := current_setting('cimmeria.image_id');
    t   text;
BEGIN
    -- sgw_inventory INHERITS sgw_inventory_base: ONLY keeps each delete and its
    -- log row on the table that actually holds the row.
    FOREACH t IN ARRAY ARRAY['sgw_inventory', 'sgw_inventory_base',
                             'sgw_gate_mail_item', 'sgw_organization_vault_items']
    LOOP
        EXECUTE format($q$
            WITH del AS (
                DELETE FROM ONLY %1$I x
                WHERE NOT EXISTS (SELECT 1 FROM resources.items i WHERE i.item_id = x.type_id)
                RETURNING x.*
            )
            INSERT INTO cimmeria_meta.reconcile_log (image_id, action, table_name, detail)
            SELECT %2$L, 'delete_dangling_item', %1$L, to_jsonb(del) FROM del
        $q$, t, img);
    END LOOP;
END
$reconcile_items$;
```

**Block 2, relocate players in removed worlds:**

```sql
DO $reconcile_worlds$
DECLARE
    img       text := current_setting('cimmeria.image_id');
    fb_id     integer;
    r         record;
BEGIN
    SELECT w.world_id INTO fb_id FROM resources.worlds w WHERE w.world = 'Castle_CellBlock';
    IF fb_id IS NULL THEN
        RAISE EXCEPTION 'reconcile: fallback world Castle_CellBlock is missing from resources.worlds';
    END IF;
    FOR r IN
        SELECT p.player_id, p.world_id, p.world_location, p.pos_x, p.pos_y, p.pos_z
        FROM sgw_player p
        WHERE NOT EXISTS (SELECT 1 FROM resources.worlds w WHERE w.world = p.world_location)
           OR (p.world_id IS NOT NULL
               AND NOT EXISTS (SELECT 1 FROM resources.worlds w WHERE w.world_id = p.world_id))
        ORDER BY p.player_id
    LOOP
        UPDATE sgw_player
           SET world_id = fb_id, world_location = 'Castle_CellBlock',
               pos_x = -334.231, pos_y = 73.472, pos_z = -228.026
         WHERE player_id = r.player_id;
        INSERT INTO cimmeria_meta.reconcile_log (image_id, action, table_name, detail)
        VALUES (img, 'relocate_player', 'sgw_player', jsonb_build_object(
            'player_id', r.player_id,
            'old', jsonb_build_object('world_id', r.world_id, 'world_location', r.world_location,
                                      'pos_x', r.pos_x, 'pos_y', r.pos_y, 'pos_z', r.pos_z),
            'new', jsonb_build_object('world_id', fb_id, 'world_location', 'Castle_CellBlock',
                                      'pos_x', -334.231, 'pos_y', 73.472, 'pos_z', -228.026)));
    END LOOP;
END
$reconcile_worlds$;
```

**Block 3, count (never delete) the F9 non-FK ids.** Two lists: array columns of `sgw_player`, and scalar columns of `sgw_mission`. Log one `count_dangling_ref` row per column with a non-zero count; `detail` is `{"column": ..., "resource": ..., "count": N}`.

| Source table | Column | Array? | Resource table | Resource column |
|---|---|---|---|---|
| `sgw_player` | `known_stargates` | yes | `stargates` | `stargate_id` |
| `sgw_player` | `abilities` | yes | `abilities` | `ability_id` |
| `sgw_player` | `trained_abilities` | yes | `abilities` | `ability_id` |
| `sgw_player` | `discipline_ids` | yes | `disciplines` | `discipline_id` |
| `sgw_player` | `blueprint_ids` | yes | `blueprints` | `blueprint_id` |
| `sgw_player` | `known_respawners` | yes | `respawners` | `respawner_id` |
| `sgw_mission` | `mission_id` | no | `missions` | `mission_id` |
| `sgw_mission` | `current_step_id` | no (nullable) | `mission_steps` | `step_id` |

Loop over a `VALUES` list of these rows (`src_table, col, is_array, res_table, res_col`). Per row build one `EXECUTE format(...)` that returns the count:

- array: `SELECT count(*) FROM %I s, LATERAL unnest(s.%I) AS v(id) WHERE NOT EXISTS (SELECT 1 FROM resources.%I r WHERE r.%I = v.id)`
- scalar: `SELECT count(*) FROM %I s WHERE s.%I IS NOT NULL AND NOT EXISTS (SELECT 1 FROM resources.%I r WHERE r.%I = s.%I)`

`INTO` a `bigint`; if greater than 0, insert the log row (`table_name` = the source table).

**Block 4, sequence fix-up.** Every sequence that is `OWNED BY` a column of a `public` table is set to `GREATEST(its current value, max(column))`. Find them through `pg_depend` (`deptype = 'a'`), not by name:

```sql
DO $reconcile_seq$
DECLARE
    img text := current_setting('cimmeria.image_id');
    r   record;
    mx  bigint;
    cur bigint;
BEGIN
    FOR r IN
        SELECT s.oid::regclass AS seq, t.oid::regclass AS tbl, a.attname AS col, t.relname AS tname
        FROM pg_class s
        JOIN pg_depend d ON d.classid = 'pg_class'::regclass AND d.objid = s.oid AND d.deptype = 'a'
        JOIN pg_class t ON t.oid = d.refobjid
        JOIN pg_namespace n ON n.oid = t.relnamespace AND n.nspname = 'public'
        JOIN pg_attribute a ON a.attrelid = t.oid AND a.attnum = d.refobjsubid
        WHERE s.relkind = 'S'
        ORDER BY s.oid
    LOOP
        EXECUTE format('SELECT max(%I)::bigint FROM %s', r.col, r.tbl) INTO mx;
        EXECUTE format('SELECT CASE WHEN is_called THEN last_value ELSE 0 END FROM %s', r.seq) INTO cur;
        IF mx IS NOT NULL AND mx > cur THEN
            PERFORM setval(r.seq, mx, true);
            INSERT INTO cimmeria_meta.reconcile_log (image_id, action, table_name, detail)
            VALUES (img, 'sequence_fixup', r.tname,
                    jsonb_build_object('sequence', r.seq::text, 'column', r.col, 'old', cur, 'new', mx));
        END IF;
    END LOOP;
END
$reconcile_seq$;
```

(`max(%I)` over an inheritance parent includes its children, which is wanted here.) If a sequence owner column is not an integer type, `max(...)::bigint` raises; that is a loud failure and correct. Do not add exception handling.

Add a file header comment: what it does, when it runs, the unqualified-name convention, and that it never touches PKs/FKs.

### Step 3: `tools/persistent_reconcile_smoke.sql`

Same shape as `tools/progression_smoke.sql` (read its first and last 15 lines first). The harness (Step 4) replaces each line that is exactly `--#include meta.sql` or `--#include reconcile.sql` with that file's text, because the SQL cannot `\ir`.

Structure:

```sql
\set ON_ERROR_STOP on
BEGIN;

--#include meta.sql

SET LOCAL cimmeria.image_id = 'pp01-smoke';

-- Scratch copies of the public tables, first on search_path, so the shared test
-- tables are never locked or modified. sgw_inventory INHERITS sgw_inventory_base
-- like the real schema.
CREATE SCHEMA pp01_probe;
CREATE TABLE pp01_probe.sgw_inventory_base (LIKE public.sgw_inventory_base INCLUDING DEFAULTS);
CREATE TABLE pp01_probe.sgw_inventory () INHERITS (pp01_probe.sgw_inventory_base);
CREATE TABLE pp01_probe.sgw_gate_mail_item (LIKE public.sgw_gate_mail_item INCLUDING DEFAULTS);
CREATE TABLE pp01_probe.sgw_organization_vault_items (LIKE public.sgw_organization_vault_items INCLUDING DEFAULTS);
CREATE TABLE pp01_probe.sgw_player (LIKE public.sgw_player INCLUDING DEFAULTS);
CREATE TABLE pp01_probe.sgw_mission (LIKE public.sgw_mission INCLUDING DEFAULTS);
SET LOCAL search_path = pp01_probe, public;
```

Before writing the inserts, Read `db/sgw/Inventory/Tables/sgw_inventory_base.sql` and `sgw_inventory.sql` and list their NOT NULL columns without defaults. The sentinel inserts below must supply exactly those.

Sentinels (all fit in `i32`; none exists in the seeds):

| Name | Value |
|---|---|
| dangling item type | 2147000001 |
| dangling player | 2147000010, account 2147000009, world_id 2147000011, world_location `'NoSuchWorld'` |
| removed stargate / ability / mission / step ids | 2147000012 / 2147000013 / 2147000014 / 2147000015 |

Inserts, each commented with the case it covers:

1. One row into `pp01_probe.sgw_inventory` (child) with `type_id = 2147000001` and one row into `pp01_probe.sgw_inventory_base` (base) with the same dangling `type_id`. Use the column list of the existing insert in `crates/base-methods/.../player_load/meta.rs` (`character_id, type_id, stack_size, slot_id, container_id, bound, durability, charges, ammo, cur_ammo_type`) plus whatever Step-3 reading shows is NOT NULL.
2. One row into `sgw_gate_mail_item` with `type_id = 2147000001` (columns per `db/sgw/Mail/Tables/sgw_gate_mail_item.sql`: `mail_id 2147000002, item_id 2147000003` and so on; for `ammo_type` use `(enum_range(NULL::resources."EAmmoType"))[1]`, for `ammo_types` use `ARRAY[(enum_range(NULL::resources."EAmmoType"))[1]]`). Note: LIKE does not copy CHECK constraints, so `item_id >= 10000` is not enforced here, but use values above 10000 anyway.
3. One row into `sgw_organization_vault_items` with `type_id = 2147000001` (`item_id 2147000004, org_id 2147000005, org_type 1, container_id 19, slot_id 0, deposited_by_player_id 2147000006`, rest as above).
4. One good control row into each of the three non-inheriting inventory-like tables is not needed; instead insert **one control row** into `pp01_probe.sgw_inventory` whose `type_id` is `(SELECT min(item_id) FROM resources.items)` and assert later that it survived.
5. A player in a fake world: `INSERT INTO sgw_player (account_id, player_id, alignment, archetype, gender, player_name, extra_name, world_location, bodyset, pos_x, pos_y, pos_z, skin_color_id, world_id, known_stargates, abilities) VALUES (2147000009, 2147000010, 0, 1, 1, 'pp01-sentinel', '', 'NoSuchWorld', 'BS_HumanMale.BS_HumanMale', 1, 2, 3, 0, 2147000011, ARRAY[2147000012], ARRAY[2147000013])`.
6. A mission row for that player: `INSERT INTO sgw_mission (player_id, mission_id, current_step_id, status, completed_step_ids, completed_objective_ids, active_objective_ids, failed_objective_ids) VALUES (2147000010, 2147000014, 2147000015, 1, '{}', '{}', '{}', '{}')`.
7. The sequence probe, in `public` (DDL is rolled back): `CREATE TABLE public.zz_pp01_seq_probe (id serial PRIMARY KEY); INSERT INTO public.zz_pp01_seq_probe SELECT generate_series(1, 5); SELECT setval(pg_get_serial_sequence('public.zz_pp01_seq_probe', 'id'), 1, false);`

Then `--#include reconcile.sql`, then one `DO $$ ... $$;` assert block that raises with a clear message on each failure. The asserts:

- `count(*) FROM cimmeria_meta.reconcile_log WHERE image_id = 'pp01-smoke' AND action = 'delete_dangling_item' AND detail->>'type_id' = '2147000001'` is exactly 4, with exactly one row each for `table_name` = `sgw_inventory`, `sgw_inventory_base`, `sgw_gate_mail_item`, `sgw_organization_vault_items` (the inheritance case: the child row is logged once, under the child).
- Every logged `detail` for those rows is a full row (`detail ? 'item_id'`).
- None of the four dangling rows remain (`SELECT count(*) FROM ONLY ...` for the inventory pair), and the control row in `sgw_inventory` is still there.
- The player now has `world_location = 'Castle_CellBlock'`, `world_id = (SELECT world_id FROM resources.worlds WHERE world = 'Castle_CellBlock')`, and `abs(pos_x + 334.231) < 0.01`. One `relocate_player` row exists with `detail->>'player_id' = '2147000010'` and `detail->'old'->>'world_location' = 'NoSuchWorld'`.
- The player's `known_stargates` is still `{2147000012}` (nothing deleted), and `count_dangling_ref` rows exist for `known_stargates`, `abilities`, `mission_id` and `current_step_id` with `(detail->>'count')::int >= 1` (use `>=` so a stale seed cannot make the test brittle, but assert each column name is present).
- `nextval(pg_get_serial_sequence('public.zz_pp01_seq_probe', 'id'))` returns 6, and one `sequence_fixup` row has `table_name = 'zz_pp01_seq_probe'`.
- Idempotence: run `--#include reconcile.sql` a second time right before the assert block and assert the number of `delete_dangling_item` and `relocate_player` rows did not grow.

End the file with `ROLLBACK;`.

### Step 4: `crates/base/src/base/smoke_tests.rs`

Read the file first. Add, next to the existing consts (use the same `include_str!` path style as `PROGRESSION_SMOKE_SQL`):

```rust
const PERSISTENT_META_SQL: &str = include_str!("../../../../db/persistent/meta.sql");
const PERSISTENT_RECONCILE_SQL: &str = include_str!("../../../../db/persistent/reconcile.sql");
const PERSISTENT_RECONCILE_SMOKE_SQL: &str =
    include_str!("../../../../tools/persistent_reconcile_smoke.sql");

/// Replace each `--#include <file>` line with that persistent SQL file.
/// The smokes cannot `\ir` (they run through sqlx), so the harness expands them.
/// PP-17 adds `seed-accounts-stage.sql` and `seed-accounts.sql` here.
fn expand_persistent_includes(sql: &str) -> String {
    let mut out = String::new();
    for line in sql.lines() {
        match line.trim() {
            "--#include meta.sql" => out.push_str(PERSISTENT_META_SQL),
            "--#include reconcile.sql" => out.push_str(PERSISTENT_RECONCILE_SQL),
            other if other.starts_with("--#include ") => {
                panic!("smoke_tests: unknown include marker: {other}")
            }
            _ => out.push_str(line),
        }
        out.push('\n');
    }
    out
}
```

Two tests:

```rust
#[tokio::test]
async fn live_db_persistent_reconcile_smoke_passes_against_seed_data() {
    let pool = require_db_or_skip!();
    let sql = strip_psql_metacommands(&expand_persistent_includes(PERSISTENT_RECONCILE_SMOKE_SQL));
    sqlx::raw_sql(AssertSqlSafe(sql))
        .execute(&pool)
        .await
        .expect("persistent_reconcile_smoke.sql must pass: see the RAISE message");
}

#[tokio::test]
async fn live_db_persistent_reconcile_raises_when_fallback_world_missing() {
    let pool = require_db_or_skip!();
    // No BEGIN/ROLLBACK in the SQL: the sqlx transaction is dropped without commit.
    let mut tx = pool.begin().await.expect("begin");
    let sql = format!(
        "{}\nSET LOCAL cimmeria.image_id = 'pp01-raise';\n\
         UPDATE resources.worlds SET world = 'Castle_CellBlock_pp01_renamed' \
         WHERE world = 'Castle_CellBlock';\n{}",
        PERSISTENT_META_SQL, PERSISTENT_RECONCILE_SQL
    );
    let err = sqlx::raw_sql(AssertSqlSafe(sql))
        .execute(&mut *tx)
        .await
        .expect_err("reconcile must RAISE when the fallback world is missing");
    assert!(
        err.to_string().contains("fallback world Castle_CellBlock"),
        "unexpected error: {err}"
    );
}
```

Check how the existing test module imports `AssertSqlSafe` and `strip_psql_metacommands` and match it. If `UPDATE resources.worlds` is blocked by an FK on the `world` column, report `blocked` with the constraint name instead of improvising.

### Lane checks (run in this order, all in the foreground)

```powershell
Set-Location <worktree>; pwsh tools/build-lane/lane.ps1 cargo fmt -p cimmeria-base
Set-Location <worktree>; pwsh tools/build-lane/lane.ps1 cargo clippy -p cimmeria-base --all-targets -- -D warnings
Set-Location <worktree>; pwsh tools/build-lane/live-db-test.ps1 persistent_reconcile
```

The crate name of `crates/base` is `cimmeria-base` (verified in its Cargo.toml). `live-db-test.ps1` reloads `sgw_pp_01` from the worktree seeds first.

### Revert proof (put in the report; do it, do not just claim it)

Temporarily replace `db/persistent/reconcile.sql` with an empty file and rerun the live-DB command: the smoke fails (its first assert, "4 delete rows" gets 0) and the RAISE test fails (`expect_err` gets `Ok`). Restore the file before committing. Second proof: change `DELETE FROM ONLY %1$I` to `DELETE FROM %1$I` and rerun: the smoke fails on the per-table log rows. Restore.

### Acceptance

- `meta.sql` and `reconcile.sql` are plain SQL with no psql meta-commands and no transaction control, and `meta.sql` runs twice without error (the smoke includes it once; the RAISE test includes it in a second transaction on a DB where the first was rolled back, so also run `psql -f` of it twice against `sgw_pp_01` once by hand and note the result).
- Both live-DB tests pass; both fail on the reverts above.
- No file outside the four listed is changed.

### Docs owed

None (PP-10 writes the reference). In the report, list `tools/persistent_reconcile_smoke.sql` for the tools catalog.

---

## PP-17 Seed-account top-up SQL and smoke

- **Agent:** database-persistence. Worktree `pp-17`, **stacked on the `pp-01` branch** (ops creates it from `pp-01`'s branch, so `meta.sql` and the harness exist). Test DB `sgw_pp_17`.
- **Commit subject:** `feat(db): insert missing seed accounts on every rebuild`
- **Depends on:** PP-01 (`meta.sql`, `reconcile_log`, `expand_persistent_includes`).

### Design choices already made

- Both files use the **unqualified table name `account`** (and `cimmeria_seed.account`) so that the smoke can run them against a scratch copy first on `search_path` without locking the shared `public.account`. In production `search_path` is `public`, so it is `public.account`.
- The stage file empties `account` with `TRUNCATE`. The contract rule "no UPDATE or DELETE on `public.account`" is about `seed-accounts.sql` and about carried rows; the stage runs before the carry-over, when `account` holds only the fresh seed rows.
- Only the `account` table is touched. No characters are created.

### Files

1. Create `db/persistent/seed-accounts-stage.sql`.
2. Create `db/persistent/seed-accounts.sql`.
3. Create `tools/persistent_seed_accounts_smoke.sql`.
4. Edit `crates/base/src/base/smoke_tests.rs`.

Read `db/sgw/Accounts/Tables/account.sql` and `db/sgw/Accounts/Seed/account.sql` first (14 seed rows, ids 2 to 15; columns: `account_id, account_name, password, password_hash_v2, password_algo, accesslevel, enabled, test_account`).

### Step 1: `db/persistent/seed-accounts-stage.sql`

```sql
-- Step 4a of the rebuild. Runs in sgw_next right after the fresh seed data load:
-- account holds exactly the seed rows. Move them aside so the carried-over
-- accounts can land in an empty table, then top-up (seed-accounts.sql) decides
-- which seed rows are missing. Plain SQL, no transaction control. No PK needed.
CREATE SCHEMA IF NOT EXISTS cimmeria_seed;
DROP TABLE IF EXISTS cimmeria_seed.account;
CREATE TABLE cimmeria_seed.account AS SELECT * FROM account;
TRUNCATE account;
```

`CREATE TABLE ... AS` keeps every column and value. `TRUNCATE` (not DELETE) is deliberate; see the design choices.

### Step 2: `db/persistent/seed-accounts.sql`

One DO block, then the schema drop. Use this shape:

```sql
DO $seed_accounts$
DECLARE
    img       text := current_setting('cimmeria.image_id');
    s         record;
    id_hit    boolean;
    name_hit  boolean;
    same_row  boolean;
BEGIN
    IF to_regclass('cimmeria_seed.account') IS NULL THEN
        RAISE EXCEPTION 'seed-accounts: cimmeria_seed.account is missing; run seed-accounts-stage.sql first';
    END IF;
    FOR s IN SELECT * FROM cimmeria_seed.account ORDER BY account_id LOOP
        id_hit   := EXISTS (SELECT 1 FROM account a WHERE a.account_id = s.account_id);
        name_hit := EXISTS (SELECT 1 FROM account a WHERE a.account_name = s.account_name);
        same_row := EXISTS (SELECT 1 FROM account a
                            WHERE a.account_id = s.account_id AND a.account_name = s.account_name);
        IF same_row THEN
            CONTINUE;  -- already there (normal rebuild): silent no-op, never updated
        ELSIF id_hit OR name_hit THEN
            INSERT INTO cimmeria_meta.reconcile_log (image_id, action, table_name, detail)
            VALUES (img, 'skip_seed_account', 'account', jsonb_build_object(
                'seed', to_jsonb(s),
                'blocked_by', to_jsonb(blk)));
        ELSE
            INSERT INTO account
            SELECT * FROM jsonb_populate_record(NULL::account, to_jsonb(s));
            INSERT INTO cimmeria_meta.reconcile_log (image_id, action, table_name, detail)
            VALUES (img, 'insert_seed_account', 'account', to_jsonb(s));
        END IF;
    END LOOP;
END
$seed_accounts$;

DROP SCHEMA cimmeria_seed CASCADE;
```

Add `blk text[];` to the DECLARE list. In the `ELSIF id_hit OR name_hit THEN` branch, before the log insert, build it:

```sql
blk := ARRAY[]::text[];
IF id_hit THEN blk := blk || 'id'::text; END IF;
IF name_hit THEN blk := blk || 'name'::text; END IF;
```

Rules: no `UPDATE` and no `DELETE` anywhere in the file; the final `DROP SCHEMA` runs also when the loop inserted nothing; the file does not create `cimmeria_meta` (PP-01's `meta.sql` runs earlier in the rebuild).

### Step 3: `tools/persistent_seed_accounts_smoke.sql`

```sql
\set ON_ERROR_STOP on
BEGIN;
--#include meta.sql
SET LOCAL cimmeria.image_id = 'pp17-smoke';
CREATE SCHEMA pp17_probe;
CREATE TABLE pp17_probe.account (LIKE public.account INCLUDING DEFAULTS);
SET LOCAL search_path = pp17_probe, public;
-- Simulate the fresh seed load: the seed rows only.
INSERT INTO pp17_probe.account SELECT * FROM public.account WHERE account_id BETWEEN 2 AND 15;
--#include seed-accounts-stage.sql
```

Then a DO block asserting the stage: `cimmeria_seed.account` has exactly the 14 seed rows (`count(*)` equals `(SELECT count(*) FROM public.account WHERE account_id BETWEEN 2 AND 15)`, and `<> 0`) and `pp17_probe.account` is empty.

Then simulate the carry-over (plain INSERT/SELECT statements, each commented):

- Carry every seed row except ids 5, 6, 7 and 8 unchanged: `INSERT INTO pp17_probe.account SELECT * FROM cimmeria_seed.account WHERE account_id NOT IN (5, 6, 7, 8);`
- Id 5 carried with a changed value: `INSERT ... SELECT account_id, account_name, password, password_hash_v2, password_algo, 99, enabled, test_account FROM cimmeria_seed.account WHERE account_id = 5;` (order the columns as `account.sql` declares them: `accesslevel` is the sixth column).
- Id 6 missing on purpose (the top-up must restore it).
- Id 7 held by a different name: `INSERT ... SELECT account_id, 'pp17-sentinel-a', ... FROM cimmeria_seed.account WHERE account_id = 7;` (the seed row 7 is blocked by id).
- Id 8's name held by another id: `INSERT ... SELECT 2147000020, account_name, ... FROM cimmeria_seed.account WHERE account_id = 8;` (the seed row 8 is blocked by name; id 8 itself is absent).

Then `--#include seed-accounts.sql`, then one assert DO block:

- Account 6 is back with its seed values (`account_name`, `accesslevel` equal to the seed row's).
- Account 5 still has `accesslevel = 99` (not upserted).
- Account 7 still has name `pp17-sentinel-a` (`count(*) WHERE account_id = 7` is 1); account 2147000020 is untouched and no `account_id = 8` row exists.
- The log for `image_id = 'pp17-smoke'` has exactly one `insert_seed_account` (detail account_id 6) and exactly two `skip_seed_account` rows: one with `detail->'blocked_by' = '["id"]'::jsonb` and `detail->'seed'->>'account_id' = '7'`, one with `'["name"]'::jsonb` and seed id 8. No other `skip`/`insert` rows (so the nine untouched carried seed accounts logged nothing).
- `to_regnamespace('cimmeria_seed') IS NULL`.
- `pp17_probe.account` holds exactly 14 rows: seed ids 2 to 6 and 9 to 15 (12 rows, id 6 restored), plus the id-7 sentinel-a row and the id-2147000020 row.

End with `ROLLBACK;`.

### Step 4: `crates/base/src/base/smoke_tests.rs`

Add consts `PERSISTENT_SEED_ACCOUNTS_STAGE_SQL`, `PERSISTENT_SEED_ACCOUNTS_SQL`, `PERSISTENT_SEED_ACCOUNTS_SMOKE_SQL` (same `include_str!` style), add the two marker arms (`"--#include seed-accounts-stage.sql"` and `"--#include seed-accounts.sql"`) to `expand_persistent_includes`, and one test:

```rust
#[tokio::test]
async fn live_db_persistent_seed_accounts_smoke_passes_against_seed_data() {
    let pool = require_db_or_skip!();
    let sql = strip_psql_metacommands(&expand_persistent_includes(PERSISTENT_SEED_ACCOUNTS_SMOKE_SQL));
    sqlx::raw_sql(AssertSqlSafe(sql))
        .execute(&pool)
        .await
        .expect("persistent_seed_accounts_smoke.sql must pass: see the RAISE message");
}
```

If `smoke_tests.rs` would pass 250 lines, keep it in one file; it is far under the cap.

### Lane checks

```powershell
Set-Location <worktree>; pwsh tools/build-lane/lane.ps1 cargo fmt -p cimmeria-base
Set-Location <worktree>; pwsh tools/build-lane/lane.ps1 cargo clippy -p cimmeria-base --all-targets -- -D warnings
Set-Location <worktree>; pwsh tools/build-lane/live-db-test.ps1 persistent_
```

(`persistent_` also runs PP-01's two tests, which must still pass.)

### Revert proof

(a) Empty `seed-accounts.sql`: the "account 6 is back" assert fails. (b) Replace the insert branch by an upsert (`INSERT ... ON CONFLICT`, or an `UPDATE`) in a scratch edit: the `accesslevel = 99` assert fails. (c) Empty the stage file: the stage assert fails. Restore after each.

### Acceptance

The three behaviors in the contract (stage, top-up, drop) are proven by the smoke; `grep`-style check in the report: the words `UPDATE` and `DELETE` do not occur in `seed-accounts.sql` outside comments.

### Docs owed

None (PP-09, PP-10, PP-16 own them). Note `tools/persistent_seed_accounts_smoke.sql` for the tools catalog in the report.

---

## PP-02 Table classification lists and lint

- **Agent:** packet-coder. Worktree `pp-02`. Test DB not needed.
- **Commit subject:** `feat(db): classify public tables as persistent or reseed, with lint`
- **Crate:** `cimmeria-content-engine` (`crates/content-engine/tests/it/` already parses `db/` files; the `interact_tag_linter.rs` precedent).

### Files

1. Create `db/persistent/reseed-public-tables.txt`.
2. Create `db/persistent/persistent-public-tables.txt`.
3. Create `crates/content-engine/tests/it/table_classification_linter.rs`.
4. Edit `crates/content-engine/tests/it/main.rs`: add `mod table_classification_linter;` in alphabetical position (after `mod onitemuse_remove_item_pairing;`).

Do not add the lint into `interact_tag_linter.rs` (it is already large).

### Step 1: the lists

Format for both: one lowercase table name per line, `#` starts a comment (whole-line or trailing), blank lines allowed. The `.txt` files in `db/persistent/` are not docs, so use LF endings (as `.sql`).

`reseed-public-tables.txt`: header comment (what the file is; that these tables are refilled from the seed on every rebuild), then the line `shards`.

`persistent-public-tables.txt`: header comment, then these 24 names in this order (the order is by name):

```
account
cell_event_outbox
login_audit
sgw_auction
sgw_auction_bid
sgw_contact_list
sgw_contact_list_member
sgw_gate_mail
sgw_gate_mail_item
sgw_inventory
sgw_inventory_base
sgw_mission
sgw_organization_cash_log
sgw_organization_events
sgw_organization_members
sgw_organization_ranks
sgw_organization_vault_items
sgw_organization_vault_log
sgw_organizations
sgw_player
sgw_player_ability_grants
sgw_player_content_cooldown
sgw_player_discipline_expertise
sgw_player_tutorials
```

Before writing, run `Glob db/sgw/*/Tables/*.sql` and compare: there must be exactly 25 files whose names are those 24 plus `shards`. If the set differs, stop with `blocked` and list the difference (do not guess the classification).

### Step 2: the lint

Read `crates/content-engine/tests/it/interact_tag_linter.rs` lines 22 to 40 for the `workspace_root()` helper and copy its shape (a private `fn workspace_root() -> PathBuf` in the new file). Implement:

- `fn parse_list(text: &str) -> Vec<String>`: per line, strip from the first `#`, trim, skip empty, return names.
- `fn create_table_names(sql: &str) -> Vec<String>`: for each line that, after trimming leading whitespace, starts with `CREATE TABLE `: take the remainder, drop an optional leading `IF NOT EXISTS `, take the first whitespace/`(`-delimited token, strip a leading `public.` and any double quotes, lowercase it.
- `fn table_files() -> Vec<PathBuf>`: every `*.sql` under `db/sgw/*/Tables/` (read the directory two levels deep with `std::fs::read_dir`; no new crates).
- `fn lint(tables: &BTreeSet<String>, reseed: &[String], persistent: &[String]) -> Vec<String>`: returns violation strings: `"<t> is in neither list"`, `"<t> is in both lists"`, `"<t> is listed but has no CREATE TABLE under db/sgw/**/Tables"`, and `"<t> is listed twice"` for duplicates inside one list.
- `fn forbidden_mentions() -> Vec<String>`: scan every `*.sql` under `db/sgw/` and `db/resources/` (recursive) plus `db/database.sql`; report `"<relative path> mentions <word>"` for `cimmeria_meta` or `cimmeria_seed` (case-insensitive).

Tests (all type 1, no DB):

```rust
#[test]
fn real_tables_are_classified_exactly_once() { /* read the real files, assert lint(...) is empty */ }

#[test]
fn real_lists_have_the_expected_sizes() { /* 24 persistent incl. account, 1 reseed (shards) */ }

#[test]
fn db_source_never_mentions_the_persistence_schemas() { /* forbidden_mentions() is empty */ }

#[test]
fn lint_catches_an_unclassified_a_doubled_and_a_stale_table() {
    // inline fixture: tables {a, b, c}; reseed [a]; persistent [a, c, ghost]
    // expect: "b is in neither list", "a is in both lists", "ghost is listed but ..."
}
```

Assert the fixture test with `assert!(v.iter().any(|m| m.contains("b is in neither list")))` and so on for each of the three messages.

### Lane checks

```powershell
Set-Location <worktree>; pwsh tools/build-lane/lane.ps1 cargo fmt -p cimmeria-content-engine
Set-Location <worktree>; pwsh tools/build-lane/lane.ps1 cargo clippy -p cimmeria-content-engine --all-targets -- -D warnings
Set-Location <worktree>; pwsh tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-content-engine table_classification
```

### Revert proof

Delete the line `sgw_player` from `persistent-public-tables.txt` and rerun: `real_tables_are_classified_exactly_once` fails with "sgw_player is in neither list". Restore. Append `shards` to the persistent list and rerun: the same test fails with "in both lists". Restore. Put both in the report.

### Acceptance

The lint is green on the real files, fails on both edits above, and the fixture test proves the three failure messages. File under 500 lines.

### Docs owed

None (PP-16 and PP-10). Mention the two lists in the report for the doc-update map.

---

## PP-03 Dangling non-FK id tolerance audit

- **Agent:** rust-gameserver-dev. Worktree `pp-03`. Test DB `sgw_pp_03`.
- **Commit subject:** `test(db): guard server load of removed resource ids`
- **Guard crate:** `cimmeria-base-methods` (owns the mapLoaded player load and the saved-mission load).
- **Nothing is fixed in this packet.** If a case panics or blocks login, record it as a follow-up and mark that guard `#[ignore = "..."]` with a one-line reason; do not change production code. A one-line guard (for example `.get(..)` for an index) is allowed only when the code line is unambiguous; say so in the report.

### Part A: the audit (read-only)

The report must contain this table, one row per id kind, filled from what you read: `id kind | load site (file:line) | behaviour today (skipped with a log, silently ignored, error, panic, client-visible break) | severity (none / cosmetic / blocks feature / blocks login)`.

Id kinds and the load sites already known (start there, then Grep `crates/` for every other reader of the same column or field name; list each site you find):

| Id kind | Source | Known load sites to read |
|---|---|---|
| known stargates | `sgw_player.known_stargates` | `crates/base-methods/src/base/world_entry/methods/player_load/core/player_data.rs` (query at lines 72 to 78, field at 225), `crates/base-world-entry/src/base/world_entry_appearance/client_ready/mod.rs`, `crates/cell-interactions/src/cell/gate_travel/dial_hub.rs`, `crates/cell-console/src/cell/console/gm/travel/dhd.rs` |
| abilities | `sgw_player.abilities` | `player_data.rs`, `crates/base-methods/src/base/world_entry/methods/player_load/meta.rs` (`player_ability_tree`, line 161), `crates/cell/src/cell/service/base_messages/mod.rs` |
| trained abilities | `sgw_player.trained_abilities` | Grep `trained_abilities` in `crates/` |
| disciplines | `sgw_player.discipline_ids`, `sgw_player_discipline_expertise` | Grep `discipline_ids` |
| blueprints | `sgw_player.blueprint_ids` | `player_data.rs`, then Grep `blueprint_ids` |
| respawners | `sgw_player.known_respawners` | Grep `known_respawners` |
| mission and step ids | `sgw_mission.mission_id`, `current_step_id`, `completed_step_ids` | `crates/base-methods/src/base/world_entry/methods/missions/mod.rs` (`try_query_saved_missions`, line 39) and its consumers (`crates/cell/.../player_init/`) |
| tutorials | `sgw_player_tutorials` | Grep `sgw_player_tutorials`; find which resource table `tutorial_id` resolves against |
| content cooldowns | `sgw_player_content_cooldown` | `crates/content-engine/src/actions.rs` and Grep `content_cooldown` |
| ability grants | `sgw_player_ability_grants` | Grep `ability_grants` |

For each, answer concretely: after the id is read, is it used as an index (`[i]`, `unwrap()`, `expect(...)`), looked up in a map (what happens on `None`), or sent to the client as is (what does the client do with an unknown id: cite `docs/` or a RE note if one exists, otherwise say "unknown")? Record panics and login-blocking cases separately as follow-ups with a one-line recommendation each.

### Part B: the guard tests

1. Create `crates/base-methods/src/base/world_entry/methods/player_load/core/removed_ids_tests.rs`.
2. Edit `crates/base-methods/src/base/world_entry/methods/player_load/core/mod.rs`: after line 11 (`mod bank_load_tests;`) add `#[cfg(test)]` and `mod removed_ids_tests;`.

Sentinel base: choose `TEST_BASE: i32 = 0x7000_2A00`, but first run `Grep "0x7000_2A" crates/` and pick the next free `0x7000_xx00` block if it is used. Name the constant in the file doc comment.

Test 1, `live_db_player_load_keeps_a_player_with_removed_resource_ids`:

```rust
use std::sync::Arc;
use crate::base::world_entry::methods::missions::query_saved_missions;
use crate::test_support::require_db_or_skip;
use super::*;
```

- `pool = require_db_or_skip!()`; `account_id = TEST_BASE`, `player_id = TEST_BASE + 1`; removed id `REMOVED: i32 = 2_147_000_000 - 1` is fine as a sentinel for resource ids (they are not row keys; any value absent from the resources works, and an `i32` fits the arrays).
- Cleanup first and last by exact sentinel: `DELETE FROM sgw_mission WHERE player_id = $1`, `DELETE FROM sgw_player WHERE player_id = $1`, `DELETE FROM account WHERE account_id = $1`.
- Insert the account and the player exactly like `insert_account_and_player` in `player_load/meta.rs` (it inserts `account (account_id, account_name, password)` with name `format!("removed-ids-{account_id}")` and a `sgw_player` row with world `'CombatSim'`; copy that statement), then `UPDATE sgw_player SET known_stargates = ARRAY[$2], abilities = ARRAY[$2], trained_abilities = ARRAY[$2], discipline_ids = ARRAY[$2], blueprint_ids = ARRAY[$2], known_respawners = ARRAY[$2] WHERE player_id = $1` bound to `player_id` and `REMOVED`.
- Insert one `sgw_mission` row: `(player_id, mission_id = REMOVED, current_step_id = REMOVED, status = 1, completed_step_ids = '{}', completed_objective_ids = '{}', active_objective_ids = '{}', failed_objective_ids = '{}')`.
- Call `query_player_load_data(&Some(Arc::new(pool.clone())), account_id as u32, player_id).await` and `query_saved_missions(&Some(Arc::new(pool.clone())), player_id).await`.
- Assert: `data.player_id == player_id` (not the default sentinel 0); `data.known_stargates == vec![REMOVED]`; `data.blueprint_ids == vec![REMOVED]`; the missions vec has length 1 with `mission_id == REMOVED` (the loaders carry the ids unchanged; tolerance is "the load succeeds", not "the id is dropped").

Test 2, `live_db_player_load_logs_no_degraded_event_for_removed_ids`: same setup, install `LogCapture::install()` before the load (see `degraded_tests` in `player_data.rs`, lines 270 to 337) and assert `capture.find_event(Level::ERROR, "", "player_row_missing")` is `None` and no row has `event = player_load_degraded` at ERROR. Share the setup through a small helper `async fn seed_removed_id_player(pool: &PgPool, base: i32)` in the same file.

If you find during Part A that another load path panics or blocks login on a removed id, add one more guard for the most important path (a plain unit test on the pure function if it needs no DB), marked `#[ignore = "PP-03 follow-up: <one line>"]` when it currently fails. At most 2 extra tests. If a path cannot be exercised without a large setup, document it in the report instead.

### Lane checks

```powershell
Set-Location <worktree>; pwsh tools/build-lane/lane.ps1 cargo fmt -p cimmeria-base-methods
Set-Location <worktree>; pwsh tools/build-lane/lane.ps1 cargo clippy -p cimmeria-base-methods --all-targets -- -D warnings
Set-Location <worktree>; pwsh tools/build-lane/live-db-test.ps1 removed_ids
```

### Revert proof

The guards assert the loader's behaviour today. To prove they can fail, temporarily change `WHERE player_id = $1 AND account_id = $2` in the `player_data.rs` query to a wrong column and rerun: test 1 fails on `data.player_id`. Restore (this is the only edit to production code allowed, and it is reverted). State in the report that, because no production fix is made, the tests are regression guards for the current behaviour, not revert tests of a change.

### Acceptance

- The report has the audit table with every row filled, file:line for each load site, and a follow-up list.
- Both guards pass (or are `#[ignore]`d with the reason, if a real defect surfaced).
- No production code is changed.

### Docs owed

None. The findings feed PP-10's reference doc.

---

## PP-15 Throwaway-DB scripts never target `sgw`

- **Agent:** packet-coder. Worktree `pp-15`. No Postgres is needed to test.
- **Commit subject:** `fix(build-lane): keep reload-db and live-db-test off the play database`
- **Decision:** owner D8. The main checkout's test DB is `sgwtest`. `reload-db` refuses the name `sgw` for any reason, including `CIMMERIA_TEST_DB`. Worktree databases (`sgw_<name>`) are unchanged.

### Files

1. Edit `tools/build-lane/reload-db.ps1`.
2. Edit `tools/build-lane/reload-db.sh`.
3. Create `tools/build-lane/test-reload-db.ps1`.
4. Edit `docs/agents/development-workflow.md` line 254 (CRLF).
5. Comment edits only, if the consumer review below says so.

### Step 1: `reload-db.ps1`

- Update the header comment (lines 1 to 10): say that the main checkout uses `sgwtest` (not `sgw`, which is the play database `setup.ps1` owns), that the script refuses `sgw` even via `CIMMERIA_TEST_DB`, and that `-DryRun` prints the resolved name and exits.
- Add a `param([switch]$DryRun)` block as the first statement of the file (before `$ErrorActionPreference`; a comment-only header may stay above it).
- Replace lines 20 and 21:

```powershell
$Db = if ($same) { 'sgwtest' } else { "sgw_$Name" }
if ($env:CIMMERIA_TEST_DB) { $Db = $env:CIMMERIA_TEST_DB }
if ($Db -ieq 'sgw') {
    [Console]::Error.WriteLine("reload-db: refusing to reload 'sgw': it is the play database. Use `setup.ps1 -ForceDatabase` (resources only) or `-WipeDatabase` (full wipe, prints counts and backs up first).")
    exit 2
}
if ($DryRun) { Write-Output "reload-db: target database $Db"; exit 0 }
```

The refusal and the dry run come before any `psql` call. The `db/database.sql` existence check (lines 12 to 14) and the git lookups stay above them.

### Step 2: `reload-db.sh`

Edit the header comment (lines 2 to 8) the same way (no dry-run flag in the shell twin), replace lines 16 and 17:

```bash
if [ "$TOP" = "$MAIN" ]; then DB="sgwtest"; else DB="sgw_${NAME}"; fi
DB="${CIMMERIA_TEST_DB:-$DB}"
if [ "${DB,,}" = "sgw" ]; then
  echo "reload-db: refusing to reload 'sgw': it is the play database. Use setup.ps1 -ForceDatabase (resources only) or -WipeDatabase (full wipe)." >&2
  exit 2
fi
```

Say in the report that the `.sh` twin was not run locally (no bash on the workstation) and is proven by review and by the identical logic of the `.ps1`.

### Step 3: consumer review

Read each file below and write one line each in the report with the verdict (`no change`, or the change you made). Known so far from the review the writer did: only `reload-db.*` derives the main-checkout name.

- `tools/build-lane/live-db-test.ps1` and `.sh`: they call `reload-db`; confirm they pass the exit code through (a refusal must fail the run) and that nothing else names `sgw`.
- `tools/test-live-db.ps1` and `.sh`: slot-clone naming and the stale-DB drop. Confirm no code path can drop a database named `sgw` or `sgwtest` that is not a slot clone. If a drop is built from a pattern, confirm the pattern cannot match `sgw`.
- `tools/build-lane/rm-worktree.ps1` (`Get-DbName`, line 75; slot regex, line 97) and `rm-worktree.sh` (`db_name`, line 76): they derive `sgw_<name>` only for worktrees. Confirm that retiring the main checkout can never drop `sgw` or `sgwtest` (add a guard that skips with a message if the computed name is `sgw` or `sgwtest`, only if one is missing).
- Grep `tools/` and `crates/lab/` and `docker/` for `localhost:5433/sgw` and `/sgw"` and list the ones that read the main-checkout default; a consumer that assumed the main checkout's test DB was `sgw` needs `sgwtest` or `CIMMERIA_TEST_DB`. Do not change `setup.ps1`, `bootstrap/` or the lab server's play DB target: those use `sgw` on purpose.

### Step 4: `tools/build-lane/test-reload-db.ps1`

Style precedent: `tools/build-lane/test-lane-slots.ps1` (the `Check([bool],[string])` helper, `$script:failed`, scratch dir under the temp folder, non-zero exit on failure, no Pester). The test builds a scratch git repo so that no Postgres is needed:

1. Create `$tmp` under `[System.IO.Path]::GetTempPath()`; `git init` there; create `db/database.sql` (empty); copy `reload-db.ps1` is not needed, call the real script by absolute path with the scratch repo as the working directory.
2. `git -c user.name=t -c user.email=t@t.invalid add -A` then `git -c user.name=t -c user.email=t@t.invalid commit -m init` (use a `.invalid` address; no real identity).
3. Create a worktree: `git worktree add ../wt-<guid> -b pp15test`.
4. A helper `function Run([string]$dir, [hashtable]$env)` that `Push-Location $dir`, sets/clears `CIMMERIA_TEST_DB`, runs `pwsh -NoProfile -File <abs reload-db.ps1> -DryRun 2>&1`, captures output and `$LASTEXITCODE`, and restores the location and env in `finally`.
5. Checks (each with `Check`):
   - main checkout, no override: exit 0 and output contains `target database sgwtest`;
   - main checkout, `CIMMERIA_TEST_DB=sgw`: exit non-zero and output contains `refusing`;
   - main checkout, `CIMMERIA_TEST_DB=SGW` (case): exit non-zero;
   - worktree: exit 0 and output matches `target database sgw_wt_`;
   - worktree with `CIMMERIA_TEST_DB=sgw`: exit non-zero;
   - override to a normal name (`sgw_custom`): exit 0 and output contains it.
6. `finally`: remove the scratch dirs (`git worktree remove --force` is fine inside the scratch repo only), and `exit 1` if `$script:failed -gt 0`.

`.github/workflows/build-lane-scripts.yml` already runs every `tools/build-lane/test-*.ps1`, so no workflow edit is needed (confirm by reading its glob and say so in the report).

Run it: `pwsh -NoProfile -File <worktree>\tools\build-lane\test-reload-db.ps1` (no lane, no cargo).

### Step 5: docs

In `docs/agents/development-workflow.md`, line 254, replace `the main checkout keeps `sgw`.` with `the main checkout uses `sgwtest` (never `sgw`, the play database; the script refuses it, also through `CIMMERIA_TEST_DB`).` Edit with exact-string replacement and verify afterwards that the file is still CRLF: `(Get-Content -Raw <file>) -match "`r`n"` and that no bare LF lines were introduced (compare `([regex]::Matches($t, "(?<!`r)`n")).Count` is 0). The skills and TESTING.md belong to PP-16, not here.

### Revert proof

Put the old line 20 back (`'sgw'` instead of `'sgwtest'`) and rerun the pwsh test: the "main checkout, no override" and refusal checks fail. Restore. Also temporarily delete the refusal block: the three refusal checks fail. Restore.

### Acceptance

The printed `DATABASE_URL` for the main checkout names `sgwtest` (the dry run prints the name; the URL line is unchanged otherwise), `CIMMERIA_TEST_DB=sgw` exits non-zero before any DROP, a worktree still gets `sgw_<wt>`, both script headers explain why, the pwsh test passes and fails on the reverts, and the report lists each consumer's verdict.

### Docs owed

Row touched in the doc-update map: development workflow (line 254). Nothing else.

---

## Design issues

1. **TRUNCATE in `seed-accounts-stage.sql` versus "no DELETE" (PP-17).** The contract says the stage "empties `public.account`" while the PP-17 acceptance says no UPDATE or DELETE on `public.account` anywhere in either file. The packet resolves it with `TRUNCATE` in the stage file only. If the architect wants a stricter reading, the stage file could instead `CREATE TABLE ... AS` and leave `account` to be emptied by the pg_restore step; say so before dispatch.
2. **Unqualified table names in the persistent SQL (PP-01, PP-17).** The contract calls the tables `public.account`, `public.sgw_player` and so on. The packets use unqualified names so the smokes can run on scratch copies without taking ACCESS EXCLUSIVE locks (dropping FKs or truncating `account` inside a smoke would block parallel `live_db_*` tests until the rollback). It relies on `search_path = public` in the container and local refresh. If PP-06 or PP-13 connect with a different `search_path`, this breaks; both should keep the default or `SET search_path = public` first.
3. **`reconcile.sql` always RAISEs without the fallback world,** even when no player is in a removed world. This follows D-PP9 ("if the fallback world is gone too, the rebuild raises") but means removing `Castle_CellBlock` from the seeds blocks every deploy, not only deploys with stranded players. Confirm that is intended.
4. **Sequence fix-up needs integer owner columns.** `max(col)::bigint` fails loudly on a non-integer owned column. No such column exists in `public` today (verified only through the table list; PP-01 should note any it finds).
5. **`count_dangling_ref` list is hard-coded** in `reconcile.sql` (eight columns). A new array column of resource ids in `sgw_player` will not be counted until someone adds it. PP-11's snapshot check does not cover this; PP-16 should add it to the checklist for new persistent columns.
6. **Contract wording, `-p` crate names.** The work-packets lanes say `-p base`; the crate's package name is `cimmeria-base`. The packets use the package names.
7. **PP-03 cannot prove a revert.** The task is an audit with guards of current behaviour; the "fails when reverted" rule is met only by a temporary break of the loader query. If the audit finds a defect, the fix belongs in a follow-up packet.
8. **PP-15 `-DryRun` is a new switch on `reload-db.ps1`.** It exists only so the pwsh test can run without Postgres. The `.sh` twin has no equivalent and is proven by review only.
9. **Sentinel-range collisions.** The `0x7000_2A00` base in PP-03 and the `2147000xxx` values in the smokes were not checked against every live-DB test in the workspace (a full-tree Grep of `0x7000_` timed out). The PP-03 packet tells the coder to Grep before using the block; the smokes only touch scratch tables, so they cannot collide.
