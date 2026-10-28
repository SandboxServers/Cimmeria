# Debug Area armory: work packets

> **Type:** runbook. **Audience:** the orchestrator, the packet agents, the lab drivers and the verifiers.
> **Companions:** [README.md](README.md) (scope, state, decisions, status), [RESUME.md](RESUME.md). Every lab packet follows the [shared UAT contract](../starter-uat-praxis/work-packets.md#shared-uat-contract) (§ C1 to § C8) in the Praxis campaign; this file does not repeat it.

## Dispatch rules

1. **Integration.** `ops` creates `dr-integration` (`pwsh tools/build-lane/mk-worktree.ps1 debug-armory/integration dr-integration`) and commits these three campaign files into it. Every packet worktree is **branched at the current `dr-integration` head** (the ops "branch at SHA" op; `mk-worktree.ps1` always bases on `origin/main`). Merge each packet back `--no-ff` (`merge-noff.ps1`) after its packet review passes.
2. **Order.** DR-01 and DR-02 at once (one message, background). DR-03a after both merge. DR-03b after DR-03a. DR-04 and DR-05 at once after DR-03b. Wave-2 loop, then PR 1. After PR 1 merges, `ops` rebases `dr-integration` onto `origin/main`; DR-06 and DR-07 at once, then DR-08; wave-3 loop, then PR 2.
3. **Agents as defined, no model override:** DR-01, DR-03a, DR-04, DR-06, DR-07 `packet-coder`; DR-02 `game-archaeology-specialist` (with the `re-lookup` skill); DR-03b `database-persistence` (with the `seed-change` skill); DR-05, DR-08, DR-99 `documentation-writer`.
4. **Lab waves 4 and 5 are ASK-FIRST** (contract § C1). Ask the user before each wave and wait for a yes. Server per D-SP2 and D-DR7.
5. **Lab packets are spec runs** (except DR-L1): one `lab-driver` follows contract [§ C8](../starter-uat-praxis/work-packets.md#c8-spec-runs-lab_uat_run): a `plan_only: true` call, then one execute call with the ready rows and no `lease_id`, resumed with the same `run_dir` after a timeout or past 90 minutes; a split file gets its own plan, execute and `run_dir` ([automated-uat.md § Hand a run to a cheap agent](../../guides/automated-uat.md#hand-a-run-to-a-cheap-agent)). The driver reports raw states and every `run_dir`, and never grades. The verifier (`general-purpose`) reads the run's evidence bundle, not the driver's summary, and writes `results/<packet>.md` (contract § C6, with the § C8 `## Runs` table first). Three attempts, then one Sonnet takeover.
6. **One lane per run.** Every spec stands at the plaza (z -872 to -930). Run one Armory spec at a time on instance `default`; never beside a starter-uat-shops SS-D1 run.
7. **Before each PR**, from `dr-integration`, in the foreground: `Set-Location <dr-integration>; python -m unittest discover -s tools/ability_mechanics -p "test_*.py"`; `Set-Location <dr-integration>; pwsh tools/build-lane/live-db-test.ps1 live_db_debug_area_armory`; `Set-Location <dr-integration>; pwsh tools/build-lane/live-db-test.ps1 live_db_vendor_arbitrage`; and for PR 2 also `Set-Location <dr-integration>; pwsh tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-lab committed_specs_parse_and_validate every_committed_spec_tag_is_seeded`.

## Rules every packet-coder and database-persistence packet carries

These agents skip CLAUDE.md; this list is their rule set.

1. Edit only the files the packet names. No `git stash`, no push, no `git worktree prune`. Commit locally with the packet's subject, then stop.
2. Every `cargo` call goes through the lane, in the foreground, from your own worktree: `Set-Location <your worktree>; pwsh tools/build-lane/lane.ps1 cargo ...`. Never call cargo directly. Live-DB tests run only through `pwsh tools/build-lane/live-db-test.ps1 <filter>`.
3. A live-DB test is gated with `require_db_or_skip!` and has `live_db` in its module or fn name. Get the URL from `test_support::database_url()`, never `DATABASE_URL`.
4. Files: soft cap 500 lines, hard cap 700. A generated seed file may pass 500 lines; a hand-written file may not.
5. Files under `docs/` ending `.md` are **CRLF**: after writing, run `Get-ChildItem <absolute paths> | ForEach-Object { $t=[IO.File]::ReadAllText($_.FullName); $t=$t -replace "`r?`n","`r`n"; [IO.File]::WriteAllText($_.FullName,$t) }`. TOML specs under `docs/guides/uat-specs/` stay **LF**. Python, Rust and SQL keep the endings their neighbours use (`seed_sql.write_text` already does this).
6. Seeds are the source of truth: edit `db/resources/`; never add `db/scripts/*.sql` migrations. Generated seed files carry a header naming the command that wrote them and are never hand-edited.
7. A guard must fail when the thing it guards is broken: break it once (the packet names how), see the failure, restore it, and record both runs in the commit body.
8. Every id and coordinate is traced from `db/resources/` or a tool's output and cited; never invent one. Use only the id blocks in README F4, and re-prove them free before writing.
9. Use PowerShell. Use absolute paths with `[IO.File]`. At most 5 fix rounds on a failing check; then stop and report `blocked` with the last error.

## Wave 1: catalog, client limits, generator

### DR-01: the representative catalog and the id-block audit

- **Agent:** packet-coder. **Host:** windows. **Worktree:** `dr-01`, branch `debug-armory/DR-01-survey`. **Commit:** `feat(tools): debug-armory survey and id audit (DR-01)`.
- **Files:** new `tools/ability_mechanics/debug_armory.py`, new `tools/ability_mechanics/test_debug_armory.py`, new `docs/analysis/debug-armory/catalog.md` (written by the tool), a `## Debug armory` section in `tools/ability_mechanics/README.md` (usage lines like the weapon-requirement section at lines 150-158). Do not edit `tools/README.md`: its catalog block (`<!-- gen:tools-catalog -->`) is regenerated by `tools/docs-gen/regen.py`; make the module docstring's first line a one-sentence summary, which that tool reads.
- **Reuse, do not re-parse:** `sys.path.insert(0, <this folder>)`, then `from seed_sql import REPO_ROOT, read_text, sql_rows, write_text` and `from weapon_requirement_audit import load, rows, int_array`. `load()` returns `monikers, abilities, items, sources, item_events, starter_items`; extend with your own reads of `items.sql` for `tier`, `quality_id` and `container_sets`, and of `archetype_ability_tree.sql`.
- **`survey` subcommand** writes `catalog.md` (`--check` exits 1 if it drifted, `--stdout` prints):
  - **Weapons:** items whose `container_sets` contains 3. Family = the item's `moniker_ids` that appear in any `abilities.item_monikers`, keyed by the sorted tuple of moniker names. Representative per (family, `tier`, `quality_id`): the lowest `item_id` that has a RANGED row (`items_event_sets` event 7), else the lowest `item_id`; mark the second kind `unbound`. Table: `| Family | Tier | Quality | Item | Name | Ranged ability | Unbound |`.
  - **Equipment:** items whose `container_sets` contains any of 4-14 (`INV_Head` ... `INV_Artifact2`, [inventory-system.md:104](../../gameplay/inventory-system.md)). Slot = the lowest such container. Representative per (slot, `tier`, `quality_id`): the lowest `item_id`. Table `| Slot | Tier | Quality | Item | Name |`.
  - **Ability coverage:** for each of the 7 archetypes in `archetype_ability_tree`: tree rows, rows with `item_monikers`, and the representative weapon chosen for each such ability (the first representative, in item-id order, carrying one of its monikers; `none` if no weapon does, which should be exactly the audit's FAIL abilities that are tree rows).
  - **Totals:** weapon and equipment representative counts, and the number of vendors needed at a cap of `--cap N` rows per vendor (default 19, the largest buy list proven to open, list 1300).
  - **Capacity:** at most **39** vendors fit (buy lists 1310-1348; list 1349 is the shared sell list, README F4). If weapons + equipment need more than 39 vendors at the cap, also print the counts with `quality_id` dropped from both keys (keep the highest quality), and say so at the top of `catalog.md`; DR-03a then drops quality on its own (below).
- **`audit` subcommand:** prints, for each README F4 block (templates 1610-1659, spawns 14110-14299, item lists 1310-1349, `item_list_items` ids 131000-134999), every id already used anywhere under `db/resources/` (scan every `*.sql` for that table's INSERTs with `sql_rows`), and exits 1 if any is used.
- **Tests (`python -m unittest`):** representative choice on a small in-memory fixture (bound beats unbound, lowest id wins, quality kept apart); family keying; slot choice; the audit exits 1 on an injected collision; `catalog.md` has CRLF endings (copy the line-ending test in `test_weapon_requirement_audit.py`).
- **Checks:** `Set-Location <dr-01>; python -m unittest discover -s tools/ability_mechanics -p "test_*.py"`; `Set-Location <dr-01>; python tools/ability_mechanics/debug_armory.py audit` exits 0; `survey --check` exits 0 after a `survey` run. Revert proof (rule 7): change one expected count in a test, see it fail, restore.

### DR-02: client limits for the Armory

- **Agent:** game-archaeology-specialist, with the `re-lookup` skill (docs, then the Ghidra MCP, then headless Ghidra; **no x64dbg, no lab**). **Host:** windows. **Worktree:** `dr-02`, branch `debug-armory/DR-02-client-limits`. **Commit:** `docs(debug-armory): DR-02 client limits`.
- **Questions, each answered with a file:line, an address or "not found":**
  1. The store window's row limit: how many buy rows the client shows, whether it pages or scrolls, and what happens past the limit (the store's Lua in the client scripts; the `onStoreOpen` / vendor list method in `docs/protocol/*-dispatch-table.md`).
  2. Client-side equip checks: does the client refuse to equip by level, archetype, faction or discipline before it sends the move? (Equip is a move into containers 3-14; the server checks only `container_sets`, [inventory-system.md:132](../../gameplay/inventory-system.md).)
  3. The Lua call or control behind equipping an armor piece from the backpack, so a spec can use `@item_action { action = "equip", name }` (see `first-session.toml:492`) for armor as well as weapons; and whether the client unequips the old piece itself.
  4. Does the client refuse an ability press by archetype (an ability not of the character's archetype)? (Feeds D-DR4.)
- **Write** `docs/analysis/debug-armory/evidence/client-limits.md` (CRLF), one section per question, then a `## For DR-03a` line giving the row cap to use (default 19 if not found).
- **Rules:** contract § C7.

### DR-03a: the `seed` subcommand (generator only)

- **Agent:** packet-coder. **Host:** windows. **Worktree:** `dr-03a`, branched after DR-01 and DR-02 merged; branch `debug-armory/DR-03a-seed-gen`. **Commit:** `feat(tools): debug-armory seed generator (DR-03a)`.
- **Files:** `tools/ability_mechanics/debug_armory.py`, `tools/ability_mechanics/test_debug_armory.py`, the README section. No seed file is written in this packet.
- **Read first:** `catalog.md`, `evidence/client-limits.md` (row cap), `db/resources/Entities/Seed/entity_templates_debug_area_plaza.sql` (template 1302's row is the vendor shape), `db/resources/Worlds/Seed/spawnlist_debug_area_plaza.sql` (spawn 13006's row is the spawn shape), `db/resources/Items/Seed/item_lists_debug_area_plaza.sql` (list 1300 rows: `item_id` = list × 10 + n, `quantity`, `naquadah`), `crates/cell-catalog/src/cell/spawner/tests/live_db_vendor_arbitrage.rs` (how a sell price is computed), `crates/navmesh-extractor/src/bin/nav_inspect.rs` (probe file format `NAME X Y Z`, its output and exit codes).
- **`seed probes --out <file>`:** lays out one vendor per group of `cap` representatives, weapons first (by family, tier, quality), then equipment (by slot, tier, quality). Vendor k stands at x = 266.0, z = -915.0 - 3.0 k, heading -1.5708; writes a probe file with y = 10.0 and names `DR<k>`. Refuse (exit 1) if any point is within 2.5 m of an existing world-1300 spawn (read every `spawnlist*` seed for world 1300).
- **Vendor count:** if the representatives need more than 39 vendors at the cap, `seed probes` and `seed emit` drop `quality_id` from both keys (keep the highest quality) and print that they did; if that still needs more than 39, exit 1 naming the count. Never write a buy list past 1348.
- **`seed emit --heights <nav_inspect output> --out-dir <dir>`:** reads the grounded y per probe. A probe nav_inspect could not ground (exit 2) is moved +2.0 in x and must be re-probed: print the new probe file and exit 3. Each `DR<k>` gets at most 3 re-probes (the generator counts them in a `.retries` file beside the probe file). When a probe still fails after its third re-probe, exit **4** (`ungroundable: DR<k> at x, z`), write no SQL file, and leave the output directory untouched. Otherwise it writes three files with a header naming the command:
  - `item_lists_debug_area_armory.sql`: one buy list per vendor (1310 up, at most 1348) and the shared sell list, always **1349**, with every stocked design except those D-DR5 leaves off. Buy row price `naquadah` = max(1, the highest unit price any template's `sell_item_list` pays for the design), `quantity` 1. Sell row price: 0 if `evidence/client-limits.md` or the vendor sell code (`crates/base-methods/src/base/world_entry/methods/vendor/`) accepts a 0 price, else 1; leave off the sell list any design some other buy list sells for less than the sell price (D-DR5). Print every design left off (`skipped from sell list: <design> <name>, cheaper on list <id> at <price>`); DR-03b copies those lines into its commit body.
  - `entity_templates_debug_area_armory.sql`: one template per vendor from 1610, copying 1302's columns, name `Armory <k>: <first label> to <last label>` (labels from `catalog.md`), its buy list, the shared sell list, no repair or recharge list.
  - `spawnlist_debug_area_armory.sql`: one spawn per vendor from 14110, world 1300, `is_stationary` as 13006.
  - Exit 1 if any id falls outside the README F4 blocks.
- **Tests:** layout spacing; the 2.5 m refusal; the +2.0 x retry; exit 4 and no SQL file after a probe fails its third re-probe; the sell list is 1349 and buy lists stop at 1348; quality is dropped above 39 vendors, and exit 1 when 39 is still exceeded; pricing (a design on a sell list at 7 is bought at 7; one on none at 1); the sell list skips a cheaper-elsewhere design; ids stay in the blocks; emitted SQL parses back with `sql_rows`.
- **Checks:** as DR-01. Revert proof: drop the `max(1, ...)` and see the pricing test fail.

## Wave 2: the seed, its guard and its docs (PR 1 with wave 1)

### DR-03b: write the Armory seed

- **Agent:** database-persistence, with the `seed-change` skill. **Host:** windows. **Worktree:** `dr-03b`, branched after DR-03a merged; branch `debug-armory/DR-03b-seed`. **Commit:** `feat(seed): Debug Area armory vendors (DR-03b)`.
- **Steps:**
  1. `Set-Location <dr-03b>; python tools/ability_mechanics/debug_armory.py audit` (must exit 0).
  2. `python tools/ability_mechanics/debug_armory.py seed probes --out <scratch>\armory.probes`.
  3. Find the Debug Area navmesh: `data/spaces/ihpet_crater_light.nav` ([debug-area.md:1008](../../content/debug-area.md)); if it is not in the worktree, follow `data/spaces/README.md` to get it, or stop `blocked` naming the missing file.
  4. `Set-Location <dr-03b>; pwsh tools/build-lane/lane.ps1 cargo run -p cimmeria-navmesh-extractor --bin nav_inspect -- data/spaces/ihpet_crater_light.nav --probes <scratch>\armory.probes > <scratch>\armory.heights` (foreground).
  5. `python tools/ability_mechanics/debug_armory.py seed emit --heights <scratch>\armory.heights --out-dir db/resources` (exit 3: re-run step 4 on the new probe file; exit 4: stop, commit nothing, report `blocked` with the printed probe, and the orchestrator asks the architect for a new vendor line).
  6. Move the three files into `db/resources/Items/Seed/`, `db/resources/Entities/Seed/`, `db/resources/Worlds/Seed/`, and add three `\ir` lines to `db/database.sql` right after the `*_debug_area_plaza.sql` lines (325-412 today).
  7. Reload the worktree's test database per the `seed-change` skill, then `Set-Location <dr-03b>; pwsh tools/build-lane/live-db-test.ps1 live_db_vendor_arbitrage` and `live_db_debug_area_plaza` (both must stay green).
- **Commit body:** the vendor count (at most 39), the rows per vendor, whether quality was dropped, the probe results (y per vendor), the audit output, and the generator's "skipped from sell list" lines (or "none").
- **Rules:** § Rules above.

### DR-04: live-DB guard for the Armory

- **Agent:** packet-coder. **Host:** windows. **Worktree:** `dr-04`, branched after DR-03b merged; branch `debug-armory/DR-04-guard`. **Commit:** `test(cell-catalog): Debug Area armory stock, prices and ability coverage (DR-04)`.
- **Files:** new `crates/cell-catalog/src/cell/spawner/tests/live_db_debug_area_armory.rs`, registered in that folder's `mod.rs` (one `mod` line in alphabetical order, one `//!` doc-list line). Model: `live_db_debug_area_plaza.rs` and `live_db_vendor_arbitrage.rs`.
- **Tests (each `require_db_or_skip!`):**
  1. `live_db_armory_vendors_spawned`: every template in 1610-1659 that the seed defines is spawned exactly once in world 1300 within spawns 14110-14299; at least one exists (control).
  2. `live_db_armory_lists_stocked_and_priced`: each Armory buy list has 1 to `CAP` rows (the cap from `evidence/client-limits.md`, as a const with a comment), every `design_id` exists in `resources.items`, and every price equals max(1, the highest sell-list unit price for that design) computed in SQL.
  3. `live_db_armory_covers_tree_weapons`: for every `archetype_ability_tree` ability with a non-empty `item_monikers`, some Armory buy row's design carries one of those monikers; the expected misses are a `KNOWN_UNCOVERED` const holding exactly the tree abilities the weapon-requirement audit lists as FAIL ([weapon-requirement-audit.md:327-339](../class-start-v6/weapon-requirement-audit.md): 997 Full Mag: Dart Pistol, 1246 Stealthed Strike, 1250 Escape, 1355 Lethal Strike), each with a comment naming its monikers. The test asserts the misses equal the const exactly. 1250 needs `ITEM_Stealth_*` armor monikers, not a weapon: if an Armory armor row carries one, drop 1250 from the const and say so in the commit body.
  4. `live_db_armory_covers_equipment_slots`: for every (slot 4-14, tier) present in `resources.items`, some Armory buy row's design has that slot and tier.
- **Revert proofs (two, deterministic; the seed edits stay local):**
  1. Test 2: set the `naquadah` of the lowest `item_id` row of list 1310 in `item_lists_debug_area_armory.sql` to 0, reload, see test 2 fail naming that row; restore and reload.
  2. Test 3: with test 3's join, find the lowest `ability_id` that exactly one Armory buy row covers, and delete that row; see test 3 fail naming the ability. If no ability has exactly one carrier, take the lowest covered ability and delete all its carrier rows. Restore and reload.
  Record both runs, the row and the ability in the commit body.
- **Check:** `Set-Location <dr-04>; pwsh tools/build-lane/live-db-test.ps1 live_db_debug_area_armory`; `Set-Location <dr-04>; pwsh tools/build-lane/lane.ps1 cargo clippy -p cimmeria-cell-catalog --tests -- -D warnings`.
- **Rules:** § Rules above.

### DR-05: Armory docs

- **Agent:** documentation-writer. **Host:** windows. **Worktree:** `dr-05`, branched after DR-03b merged; branch `debug-armory/DR-05-docs`. **Commit:** `docs(debug-area): the Armory (DR-05)`.
- **Write:** in `docs/content/debug-area.md`, add the new blocks to its id-block list (near lines 40-69) and a `## Armory` section: what it is, the vendor table (template, spawn, position, stock label; from the seed), how to use it (`.gotolocation DebugArea 266 <y> -915`, `/gmgivecash`, buy, equip, fire at the dummies at z -872, sell back; and a "Not sold back" list of the designs DR-03b's commit body says were left off the sell list: they cannot be sold back here and stay in the bag until a `.removeitem` exists, D-DR5), what it does not do (every item: use `/gmgiveitem`; other archetypes' abilities: D-DR4), and the regenerate command. In `docs/guides/unified-uat.md`, under the Debug Area section, rows DR-U1 (Armory walk, DR-L1), DR-U2 (ability specs, DR-L2 to DR-L8), DR-U3 (gear specs, DR-L9/L10), status "never run".
- **Checks:** CRLF; every template and spawn id in the table appears in the seed (`Select-String`); no `<!-- gen:` block touched.
- **Rules:** contract § C7.

## Wave 3: generated lab specs (PR 2)

### DR-06: ability specs per archetype

- **Agent:** packet-coder. **Host:** windows. **Worktree:** `dr-06`, branch `debug-armory/DR-06-ability-specs`. **Commit:** `feat(uat-specs): Debug Area armory ability specs (DR-06)`.
- **Read first:** `docs/guides/uat-specs/abilities.toml` lines 1-120 (the row shape: setup gives and places, step presses the key, `client.ability.sent` / `client.ability.applied` clauses, teardown), [automated-uat.md § Write a row spec](../../guides/automated-uat.md#write-a-row-spec), the `@dummy`, `@cooldowns_reset`, `@clear_effects` and `@item_action` macros there, and `crates/lab/src/supervisor/flows/widgets.rs:177-196` (archetype names per alignment).
- **`specs abilities --out-dir docs/guides/uat-specs`** writes `armory-abilities-<archetype>.toml` (LF) for Soldier, Commando, Scientist, Archeologist, Asgard and Sholva (alignment `sgu`) and Goauld (alignment `praxis`); Jaffa gets none (D-AT04). Section id `armory-abilities-<archetype>`, `account = "gm"`, `character = "fresh"`.
  - Section setup row `<A>-00`: `.gotolocation DebugArea 246 6.6 -878`, `/gmgiveallabilities`, `@dummy` hostile.
  - One row per tree ability, in `tree_index`, `ability_index` order, id `<A>-<ability_id>`: if the ability has `item_monikers`, setup gives and equips its representative weapon from `catalog.md` (`/gmgiveitem <id> 1`, then `@item_action { action = "equip", name = "<item name>" }`); then `@use_ability { ability_id, place = true }`, wait, `@cooldowns_reset`; step presses the key; expect `client.ability.sent` with `match_fields = { ability_id }` and `client.ability.applied` (`required = false` for Self and Ground target types, as `abilities.toml` does). Teardown `@clear_effects` and `@cooldowns_reset`.
  - An ability in `KNOWN_UNCOVERED` (DR-04: 997, 1246, 1250 and 1355 unless DR-04 dropped one) gets `blocked = "no shipped weapon carries <monikers> (weapon-requirement audit FAIL)"`; the verifier grades those rows BLOCKED and writes **one** issue body for all of them. If DR-04 found 1250 covered by an armor row, its row equips that armor piece (`@item_action` equip) instead of a weapon.
  - More than 60 rows: split into one file per `tree_index` (`armory-abilities-<archetype>-<n>.toml`).
- **Tests:** the generator writes a row per tree ability, equips before pressing, splits at 60, and emits LF.
- **Check:** unit tests, then `Set-Location <dr-06>; pwsh tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-lab committed_specs_parse_and_validate every_committed_spec_tag_is_seeded`. Revert proof: emit a row with an unknown macro, see the validator fail, restore.

### DR-07: gear specs

- **Agent:** packet-coder. **Host:** windows. **Worktree:** `dr-07`, branch `debug-armory/DR-07-gear-specs`. **Commit:** `feat(uat-specs): Debug Area armory gear specs (DR-07)`.
- **Read first:** as DR-06, plus `crates/cell-methods/src/cell/cell_methods/player/interaction/hostile_attack.rs:130-180` (the `weapon_unbound` line) and the inventory move telemetry in `crates/base-methods/src/base/world_entry/methods/inventory/move_/` (find the accepted-move event and its fields; do not guess the name).
- **`specs gear --out-dir docs/guides/uat-specs`** writes (LF, Soldier `sgu` fresh):
  - `armory-gear-weapons.toml`: one row per weapon representative: give, equip, `@target` the L1 dummy (template 1310), right-click it; a bound weapon expects its ranged ability's `client.ability.sent`; an `unbound` one expects `weapon_unbound` with `reason = 'no_ranged_attack'` in SigNoz and is graded PASS on that line (it measures the gap, F8).
  - `armory-gear-armor.toml`: one row per equipment representative: give, equip, and expect the accepted move into its slot (the event you found) plus a `client_window_read` of the character sheet slot (`source = "human"` clause, screenshot).
  - Split either file at 60 rows (`-1`, `-2` ...).
- **Tests and check:** as DR-06.

### DR-08: brief for DR-L1

- **Agent:** documentation-writer. **Host:** windows. **Worktree:** `dr-08`, branch `debug-armory/DR-08-brief`. **Commit:** `docs(debug-armory): DR-L1 brief (DR-08)`.
- **Write** `docs/analysis/debug-armory/briefs/DR-L1.md` and `verify/DR-L1.md` in the contract § C2 / § C3 shapes, lease owner `dr-DR-L1`. Rows: L1-1 stand at the first Armory vendor and screenshot the row of vendors (human: "a line of vendors east of the plaza vendors, none inside a wall"); L1-2 open each vendor in turn (`store_opened` per template 1610 up, `buy_count` equal to its list size); L1-3 `/gmgivecash 100000`, buy one weapon and one armor piece that are both on sell list 1349 (pick them from the seed, cite the lines; never one DR-03b's commit lists as skipped) (`transaction`); L1-4 equip both (`@item_action`), screenshot the character; L1-5 right-click the L1 dummy with the weapon; L1-6 sell both back (`transaction` sell at the list 1349 price). Front-load: the screensaver, stand 4-6 m off a vendor, close dialogs first, never `client_wait_for` a world load (#1677).
- **Checks:** CRLF; one grading section per row.

## Wave 4 and wave 5: lab (ASK-FIRST)

| Packet | Wave | Run | Expected length |
|---|---|---|---|
| DR-L1 | 4 | `lab-driver` with `briefs/DR-L1.md`, verifier `verify/DR-L1.md` | 28 calls |
| DR-L2 to DR-L5 | 4 | `lab_uat_run` of `armory-abilities-soldier`, `-commando`, `-scientist`, `-archeologist` (each file or split) | about 60-80 min per archetype |
| DR-L6 to DR-L8 | 5 | `armory-abilities-asgard`, `-goauld`, `-sholva`. Asgard: if character creation fails, grade the whole section BLOCKED citing F42 / HELD-B16 | as above |
| DR-L9, DR-L10 | 5 | `armory-gear-weapons`, `armory-gear-armor` | about 60 min each |

Each runs as dispatch rule 5 (contract § C8: plan, execute, resume with the same `run_dir`), one at a time on `default` (rule 6). If a run passes 90 minutes, resume it with the same `run_dir`; a section with split files runs each file as its own plan and execute, with its own `run_dir`. Each wave's results ship as one docs PR from `dr-integration`.

## Wave 6: issues and close-out

### DR-30: issues

The orchestrator has `ops` file each body under `results/issues/` (`gh issue create --body-file`), labels per [docs/agents/](../../agents/), and records the numbers in the results files.

### DR-99: close-out

- **Agent:** documentation-writer. Worktree `dr-integration`. Commit `docs(debug-armory): DR-99 close-out`.
- **Write:** a `## Results summary` at the top of README.md (per archetype: rows, PASS, FAIL, BLOCKED; weapons bound vs unbound; armor slots seen); the statuses of DR-U1 to DR-U3 in `docs/guides/unified-uat.md`; the Armory section's "tested" line in `debug-area.md`; then the status docs once (`docs/project-status.md` and the matching `docs/gap-analysis*` row; walk [doc-update-map.md](../../agents/doc-update-map.md)). Rewrite RESUME.md to "closed". Rules § C7.

### DR-40: cross-archetype grant (stub, BlockedDecision D-DR4)

Written as a full packet only if the owner wants it. Shape: a GM line `/gmgiveabilitytree <archetype>` under the `/gmgiveallabilities` handler, recorded as GM grants that `/gmresetabilities` removes; a live-DB test; a `commands.md` row. Needs `rust-gameserver-dev` and a server-authority review.
