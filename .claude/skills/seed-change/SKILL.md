---
name: seed-change
description: Change game data (abilities, items, NPC spawns, content chains, dialogs, missions, loot, worlds) by editing the seed under db/resources/, reloading the worktree's test database, guarding it with a live-DB test, and seeing it on a running server. Use when a fix or feature needs a new or changed DB row, when someone suggests a SQL migration or a one-off UPDATE, when you write a live_db test, or when content authored with the in-game .-console (savespawn, path_*) needs committing.
---

# Seed change

The seed under `db/resources/` is the source of truth. Every database
(per-worktree test DBs, CI, and the colo, which reloads from the seed on
every deploy) is built from it via `db/database.sql`.

## Rules

- **Edit the seed file, never add a `db/scripts/*.sql` migration**, and
  never ask anyone to run a one-off `UPDATE`. Ask the user before you add a
  migration for any reason.
- **The seed is Project Giza's reconstruction from the client's cooked
  packages, not the 2009 CME database.** A NULL or missing link means Giza's
  extraction didn't recover it, not that retail lacked it. Never call seed
  rows "original" or "retail". Reconstruct missing wiring from client
  evidence (see the `re-lookup` skill).
- Find the right file by grepping `db/resources/` for a neighbouring row or
  the table name. It's split by area (`Abilities/`, `AI/`, `Content/`,
  `Dialogs/`, `Items/`, `Loot/`, ...); schemas are `db/resources/_schema.sql`
  and `db/sgw/`.
- Match the file's existing statement style and ordering, and keep its line
  endings. Read the file header first: some seed files (for example
  `Effects/Seed/effect_nvps.sql`, `Abilities/Seed/abilities.sql`) say they
  are generated. Change their generator under `tools/` and regenerate
  rather than hand-editing.

## Steps

1. **Edit** the seed file(s).
2. **Reload this worktree's database** from the worktree root:

   ```powershell
   pwsh tools/build-lane/reload-db.ps1
   ```

   It rebuilds `sgw_<worktree>` (the main checkout uses `sgw`) on the bundled
   Postgres at `localhost:5433` and prints the `DATABASE_URL`. A load error
   here is a broken seed; fix it before anything else.
3. **Guard it with a test**, following [TESTING.md](../../../TESTING.md)
   (the live-DB and content-chain replay sections). Live-DB rules:
   - Gate with `let pool = require_db_or_skip!();` and get the URL from
     `test_support::database_url()`, never `DATABASE_URL`.
   - Put `live_db` in the test fn or module name, or the `ci-live-db` profile
     never runs it.
   - Test ids use a positive `0x7000_xxxx` sentinel that fits in `i32`;
     cleanup deletes by exact sentinel (`= $id` or `IN (...)`), never by
     range.
   - Don't trust seed values: read the baseline inside the test or assert by
     relationship.
   - The guard must fail when the change is reverted.
   - A content chain gets a replay test, including verbs with zero seed rows.
4. **Run it in a lane slot** (reloads the DB, then runs the CI live-DB
   profile):

   ```powershell
   pwsh tools/build-lane/live-db-test.ps1 <test-name filter>
   ```

5. **See it live (optional).** A running server reads the database it was
   started against, not the seed files. Reload that database (or restart the
   server on a fresh one) first. Content chains then hot-reload with the lab
   server's `server_content_reload` tool, which re-reads every chain from the
   DB; other tables (abilities, items, spawns) need a server restart. Ask the
   user before touching the lab server.

## Content authored in-game

The GM `.`-console authoring commands (`savespawn`, `delspawn`, `path_add`,
`path_assign`, ...) apply in memory and to the live DB, then emit canonical
seed SQL per seed file on `.seedconfirm` (to the server log today; see
`crates/cell-console/src/cell/console/seed.rs`). The live write is lost on
the next rebuild. **Copy the emitted SQL into the named `db/resources/` file
and commit it**; that's the durable change.

## Shipping it

- Say "takes effect on the colo at the next deploy" in the PR. Nothing has
  to run there by hand.
- A developer's local DB that isn't reloaded stays stale until they rerun
  `setup.ps1` or `reload-db.ps1`. Mention it once.
- Server-side data files under `data/spaces/` (such as `.nav`) ship in the
  image the same way. The client needs nothing.
