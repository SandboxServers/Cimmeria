# Starter-world UAT, shops: work packets

> **Type:** runbook. **Audience:** the orchestrator, the packet agents, the lab drivers and the verifiers.
> **Companions:** [README.md](README.md) (scope, state, decisions, status), [RESUME.md](RESUME.md). Every drive packet follows the [shared UAT contract](../starter-uat-praxis/work-packets.md#shared-uat-contract) (§ C1 to § C8) in the Praxis campaign; this file does not repeat it.

## Dispatch rules

1. **Wave 1.** `ops` creates `ss-integration` (`pwsh tools/build-lane/mk-worktree.ps1 starter-uat-shops/integration ss-integration`) and commits these three campaign files into it. Then `ops` creates one worktree per packet **branched at the `ss-integration` head** (branch at SHA): `ss-01`, `ss-02`, `ss-04` first, in one message, in the background. When SS-01 is reviewed and merged into `ss-integration`, branch `ss-03a` and `ss-05` at the new head; when SS-03a is merged, branch `ss-03b`. Merge each packet back `--no-ff` (`merge-noff.ps1`) after its packet review passes.
2. **Agents as defined, no model override:** SS-01 `items-systems-advisor`; SS-02, SS-03a, SS-03b `packet-coder`; SS-04, SS-05, SS-98, SS-99 `documentation-writer`; SS-20 to SS-23 `database-persistence`.
3. **Lab waves (2 and 3b) are ASK-FIRST** (contract § C1). Ask the user before each wave and wait for a yes.
4. **Lanes.** SS-D1 runs on instance `default` (Debug Area plaza). SS-D2 then SS-D3 run one after the other on `p2`: both use the stasis room, and two lanes never share a room.
5. **Per drive packet:** as Praxis dispatch rule 5 (Haiku `lab-driver`, then the `general-purpose` verifier with the verify file, three attempts, then one Sonnet takeover). SS-D1 is a spec run, not a step brief: the contract § C8 sequence (one `plan_only` call, then one execute call with the ready rows, resumed with the same `run_dir` if it times out; see its section).
6. **Results and issues:** as Praxis dispatch rules 6 and 7, with paths under `docs/analysis/starter-uat-shops/results/`.
7. **Review loops** (campaign-packet skill § 4): the packet loop on every wave-1 and wave-3a packet, then the wave loop before each PR. Lab results are reviewed by their verifier.
8. **Before the wave-1 PR**, from `ss-integration`, in the foreground: `Set-Location <ss-integration>; pwsh tools/build-lane/live-db-test.ps1 live_db_starter_vendor` and `Set-Location <ss-integration>; pwsh tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-lab committed_specs_parse_and_validate every_committed_spec_tag_is_seeded`.

## Rules every packet-coder and database-persistence packet carries

These agents skip CLAUDE.md; this list is their rule set.

1. Edit only the files the packet names. No `git stash`, no push, no `git worktree prune`. Commit locally with the packet's subject, then stop.
2. Every `cargo` call goes through the lane, in the foreground, from your own worktree: `Set-Location <your worktree>; pwsh tools/build-lane/lane.ps1 cargo ...`. Never call cargo directly. Live-DB tests run only through `pwsh tools/build-lane/live-db-test.ps1 <filter>`.
3. A live-DB test is gated with `require_db_or_skip!` and has `live_db` in its module or fn name, or the meta-tests fail. Get the URL from `test_support::database_url()`, never `DATABASE_URL`.
4. Files: soft cap 500 lines, hard cap 700.
5. Files under `docs/` ending `.md` are **CRLF**: after writing, run `Get-ChildItem <absolute paths> | ForEach-Object { $t=[IO.File]::ReadAllText($_.FullName); $t=$t -replace "`r?`n","`r`n"; [IO.File]::WriteAllText($_.FullName,$t) }`. TOML specs under `docs/guides/uat-specs/` stay **LF** like the existing ones; Rust and SQL files keep their current endings.
6. Seeds are the source of truth: edit `db/resources/`; never add `db/scripts/*.sql` migrations.
7. A guard must fail when the thing it guards is broken: break it on purpose once (the packet names how), see the failure, restore it, and record both runs in the commit body.
8. Every id and coordinate is traced from `db/resources/` and cited (`file:line`); never invent one.
9. At most 5 fix rounds on a failing check; then stop and report `blocked` with the last error.

## Wave 1 (one PR from `ss-integration`)

### SS-01: store-open failure and the client paths for sell, buyback, repair and recharge

- **Agent:** items-systems-advisor. **Host:** windows. **Worktree:** `ss-01`, branch `starter-uat-shops/SS-01-store-paths`. **Commit:** `docs(starter-uat-shops): SS-01 store paths`. Read-only research: writes one file, runs no cargo, uses no lab tool.
- **Questions to answer, each with file:line evidence:**
  1. Why did the stasis-room Quartermaster (template 300, spawn 400) open nothing on 2026-10-12? Inputs: [BANK-R10](../haiku-lab-uat/briefs/bank/BANK-R10.md), [haiku-lab-uat README](../haiku-lab-uat/README.md) bank step 10, the right-click popup (`interaction_type` 65536 on `entity_templates.sql:1011`), the vendor open path in `crates/base-methods/src/base/world_entry/methods/vendor/`, and `store_opened` in its `telemetry.rs`. Say which server log line would prove the store opened, and whether the failure was the click (popup item), the server or the window. If you need the client's Lua or a binary fact, follow the `re-lookup` skill (docs first).
  2. For sell, buyback, repair and recharge: the client Lua call or window control each button sends, the server method it reaches, and the lab tool that can press it (`client_window_click` on a named control, `client_drag_drop`, or a stock Lua call through `client_lua_eval`, which grades N3). See `docs/guides/live-research-lab.md` (tools table) and `.claude/agent-memory/` notes on `client_vendor_buy`.
  3. Can any in-game action or GM command lower an item's durability or drain its charge today (`docs/commands.md`, the item code)? If not, write "none" (D-SS3 then applies).
  4. For the trainer (template 301): the client call behind "Train", and the server events (`train_requested` in `crates/cell-methods/src/cell/cell_methods/player/vendor/train.rs`, `train_persisted` in `crates/base-methods/src/base/world_entry/methods/progression/train_ability.rs`).
- **Write** `docs/analysis/starter-uat-shops/evidence/store-paths.md` (CRLF): one section per question, then a `## Recipes` table `| Action | Tool call (exact args) | Tier | Server event that proves it |` that SS-03a, SS-03b and SS-05 copy.
- **Rules:** contract § C7.

### SS-02: live-DB guard: no starter-world vendor or trainer has an empty or dangling list

- **Agent:** packet-coder. **Host:** windows. **Worktree:** `ss-02`, branch `starter-uat-shops/SS-02-vendor-guard`. **Commit:** `test(cell-catalog): starter-world vendors and trainers have stocked lists`.
- **Files:** new `crates/cell-catalog/src/cell/spawner/tests/live_db_starter_vendors.rs`; register it in `crates/cell-catalog/src/cell/spawner/tests/mod.rs` (one `mod live_db_starter_vendors;` line, kept in the existing alphabetical block, and one `//! - [`live_db_starter_vendors`]: ...` line in the module doc list). Nothing else.
- **Model:** copy the shape of `live_db_vendor_arbitrage.rs` in the same folder (`mod live_db { use crate::test_support::require_db_or_skip; #[tokio::test] async fn ... { let pool = require_db_or_skip!(); ... } }`).
- **Worlds:** `const STARTER_WORLDS: [i32; 14] = [12, 8, 57, 68, 69, 70, 19, 58, 61, 62, 18, 80, 23, 1300];` with a comment naming each world.
- **Test 1, `live_db_starter_vendor_lists_are_stocked`:** for every template spawned in those worlds (`resources.spawnlist.world_id`, `template_id`), each non-null `buy_item_list`, `sell_item_list`, `repair_item_list` and `recharge_item_list` has at least one `resources.item_list_items` row, and every row's `design_id` exists as `resources.items.item_id`. Collect every miss as `(template_id, column, list_id, problem)` and assert the list is empty, printing it.
- **Test 2, `live_db_starter_trainer_lists_are_stocked`:** for every such template with a non-null `trainer_ability_list_id`, the list has at least one `resources.trainer_abilities` row and every `ability_id` exists in `resources.abilities`.
- **Controls (each test):** assert the vendor set includes templates 300, 314, 415 and 1302, and the trainer set includes 301, so an empty join cannot pass.
- **Revert proof (rule 7):** in your worktree only, change template 415's `buy_item_list` in `db/resources/Entities/Seed/entity_templates.sql:1099` from 1300 to 999999, run the check, see test 1 fail naming 415, restore the line, run again green. Do not commit the broken line.
- **Check:** `Set-Location <ss-02>; pwsh tools/build-lane/live-db-test.ps1 live_db_starter_` then `Set-Location <ss-02>; pwsh tools/build-lane/lane.ps1 cargo clippy -p cimmeria-cell-catalog --tests -- -D warnings`.
- **If a test finds a real miss in the seed:** the guard stays fail-closed: no allow-list, no skip, no `#[ignore]`. Do not fix the seed and do not commit a failing guard. Commit nothing, and report `blocked` with the printed miss list `(template_id, column, list_id, problem)`. The orchestrator files an issue and asks the architect for a ruling; the expected ruling is a `seed-change` packet that fixes the miss before SS-02 is re-run, all before the wave-1 PR.
- **Rules:** § Rules above.

### SS-04: vendor coverage map and the D-SS1 evidence

- **Agent:** documentation-writer. **Host:** windows. **Worktree:** `ss-04`, branch `starter-uat-shops/SS-04-coverage`. **Commit:** `docs(starter-uat-shops): SS-04 vendor coverage map`.
- **Write** `docs/analysis/starter-uat-shops/vendor-map.md` (CRLF):
  - Table 1, every vendor or trainer spawn in worlds 12, 8, 57, 68, 69, 70, 19, 58, 61, 62, 18, 80, 23, 1300: `| World | Spawn | Tag | Template | Display name | Buy / sell / repair / recharge / trainer list | Buy rows | Covered by (campaign and row id) |`. Find them with `Select-String` over `db/resources/Worlds/Seed/*.sql` and `db/resources/Entities/Seed/*.sql` (a template is a vendor if any list column is non-null, a trainer if `trainer_ability_list_id` is non-null). "Covered by" uses README D-SS4: Castle hub S1-S5 to SP-DCA6; Omega to starter-uat-union UO-R5 (cite the row ids from `docs/analysis/union-zone-planning/omega-packets/UO-U1.md`); stasis hub and Debug Area to this campaign's SS-D1/SS-D2.
  - Table 2, the D-SS1 evidence: for Tollana, SGC_W1, Dakara_E1, Beta Site E1 and Harset, `| World | Vendors today | Nearest vendor a player of that faction can reach (world, how) | Ledger mention of vendors (file:line, or "none") |`. Read the zone ledgers: [tollana-continuation](../tollana-continuation/README.md), [union-sgc-continuation](../union-sgc-continuation/README.md), [dakara-e1-rebuild](../dakara-e1-rebuild/README.md), [union-beta-e1-continuation](../union-beta-e1-continuation/README.md), [harset-continuation](../harset-continuation/README.md) (HA-15b/c, PU-08 draft).
  - A one-paragraph note on OD-5 (D-SS2) with the citation.
- **Checks:** CRLF; every spawn id in table 1 appears in the seed (`Select-String` each).
- **Rules:** contract § C7.

### SS-03a: `vendors.toml` rows V1 to V7

- **Agent:** packet-coder. **Host:** windows. **Worktree:** `ss-03a`, branched at `ss-integration` after SS-01 merged; branch `starter-uat-shops/SS-03a-spec`. **Commit:** `docs(uat-specs): vendor rows V1-V7 from starter-uat-shops`.
- **Read first:** `docs/analysis/starter-uat-shops/evidence/store-paths.md` (the recipes), `docs/guides/uat-specs/debug-area.toml` rows DA-U6 and DA-U8 (lines 255-330: setup, `@world_click`, SigNoz clauses), [automated-uat.md § Write a row spec](../../guides/automated-uat.md#write-a-row-spec) and § Capture values.
- **Write** new `docs/guides/uat-specs/vendors.toml` (LF): `[section] id = "vendors"`, `account = "gm"`, `character = "fresh"`, a header comment citing this campaign. Every row starts with the DA-U6 setup (`.gotolocation DebugArea 259 7.2 -925`, then the existing wait) and right-clicks the vendor named in the row. Rows:

  | Row | Vendor | Does | Expect clauses (source `signoz` unless noted) |
  |---|---|---|---|
  | V1 | Basic Equipment Quartermaster (300) | open | `store_opened`, `vendor_template_id = 300`, `buy_count` ≥ 1, `sell_count` ≥ 1 |
  | V2 | Consumables (1302) | buy one special-ammo stack with `client_vendor_buy` | `transaction`, `vendor_template_id = 1302`, `price = 1`; a `delta` capture on cash of -1 |
  | V3 | Consumables (1302) | buy the SGHC 6 SMG (design 21, price 1000) with less than 1000 cash | `refused` with `reason = 'insufficient_cash'`; cash unchanged. If a fresh character starts with 1000 or more, record that in a `blocked` line naming the starting cash and stop the row |
  | V4 | Quartermaster (300) | sell the pistol bought in setup (`client_vendor_buy` SI 3 pistol at 1302, 300 cash, given by `/gmgivecash 300`) | `transaction` with the sell action and `price = 300` |
  | V5 | Quartermaster (300) | buy back the pistol sold in V4 | `transaction` with the buyback action; cash back down by 300 |
  | V6 | Quartermaster (300) | sell an item list 2 does not buy (a Health Slappack) | `refused`, `reason = 'not_sellable'` |
  | V7 | Common Materials Components (314, at the plaza spawn 13019) | sell anything | `refused`, `reason = 'vendor_buys_nothing'` |

  **Seed citations** (rule 8; copy each into the row's comment, and re-check the line before you copy it):

  | Value | Source |
  |---|---|
  | Setup `.gotolocation DebugArea 259 7.2 -925` (world 1300, the plaza beside the vendors) | `docs/guides/uat-specs/debug-area.toml:260` (DA-U6); nearest spawns below |
  | Template 300, Basic Equipment Quartermaster, buy list 1, sell/repair/recharge list 2 | `db/resources/Entities/Seed/entity_templates.sql:1011`; plaza spawn 13001 at `db/resources/Worlds/Seed/spawnlist_debug_area_plaza.sql:45` |
  | Template 301, Archetype Skills Trainer (SS-03b) | `entity_templates.sql:1019`; plaza spawn 13002 at `spawnlist_debug_area_plaza.sql:43` |
  | Template 314, Common Materials Components, buy list 310, sell list NULL | `entity_templates.sql:1125`; plaza spawn 13019 at `spawnlist_debug_area_plaza.sql:59` |
  | Template 1302, Consumables (munitions vendor), buy list 1300, sell/repair/recharge list 2 | `db/resources/Entities/Seed/entity_templates_debug_area_plaza.sql:43`; plaza spawn 13006 at `spawnlist_debug_area_plaza.sql:46` |
  | Buy list 1300 | `db/resources/Items/Seed/item_lists_debug_area_plaza.sql:25` |
  | Special-ammo stacks, designs 9000-9014, quantity 100, price 1 (V2) | `item_lists_debug_area_plaza.sql:27-41` (rows 13001-13015) |
  | Health Slappack TC1, design 2893, stack of 5, price 1 (V6; not on list 2) | `item_lists_debug_area_plaza.sql:42`; list 2 has no 2893 row in `db/resources/Items/Seed/item_list_items.sql` |
  | SI 3 9mm Pistol, design 55, price 300 (V4, V5) | `item_lists_debug_area_plaza.sql:43`; list 2 buys it back at 300, `item_list_items.sql:15` |
  | SGHC 6 SMG, design 21, price 1000 (V3) | `item_lists_debug_area_plaza.sql:44`; list 2 row at `item_list_items.sql:11` |

  The comment above template 1302 says "1 naquadah each"; the list rows (lines 43-44) and the list file's header (lines 2-5) price the pistol at 300 and the SMG at 1000. The rows win.

  Use the exact sell and buyback tool calls from `store-paths.md`; if it says a step is only reachable by a stock Lua call, use it and set `tier = "N3"`. If `store-paths.md` says an action cannot be pressed at all, write the row with a `blocked = "<reason from store-paths.md>"` line instead of steps. Read the action names for the `transaction` filter from `crates/base-methods/src/base/world_entry/methods/vendor/` (the `action` field); do not guess them.
- **Check (the only cargo):** `Set-Location <ss-03a>; pwsh tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-lab committed_specs_parse_and_validate every_committed_spec_tag_is_seeded`. Must pass.
- **Rules:** § Rules above.

### SS-03b: `vendors.toml` rows V8 to V12 and the DA-U7 re-spec

- **Agent:** packet-coder. **Host:** windows. **Worktree:** `ss-03b`, branched after SS-03a merged; branch `starter-uat-shops/SS-03b-spec`. **Commit:** `docs(uat-specs): vendor rows V8-V12 and DA-U7 from starter-uat-shops`.
- **Read first:** as SS-03a, plus the trainer row in `debug-area.toml` (the `trainer_open` clause near line 245).
- **Append to** `docs/guides/uat-specs/vendors.toml`:

  | Row | Vendor | Does | Expect |
  |---|---|---|---|
  | V8 | Quartermaster (300) | repair a worn item | `transaction` with the repair action. If `store-paths.md` question 3 is "none", `blocked = "no in-game way to wear an item (D-SS3)"` |
  | V9 | Quartermaster (300) | recharge a drained item | `transaction` with the recharge action; same D-SS3 rule |
  | V10 | Archetype Skills Trainer (301) | open, then train one ability the character lacks | `trainer_open` with `trainer_template_id = 301`, then `train_requested` and `train_persisted` |
  | V11 | Archetype Skills Trainer (301) | train with 0 training points | the refusal event and line `store-paths.md` names; if none exists, `blocked` naming the gap |
  | V12 | Quartermaster (300) | open the store, `client_move_to` 30 m away, buy | `refused` with the distance reason and the line "You are too far from the vendor." (cite the reason string from the vendor code) |

- **Also** in `docs/guides/uat-specs/debug-area.toml` row DA-U7 (line 280-290): if V2, V4, V8 and V9 are not all `blocked`, replace its `blocked = ...` line with a `note` pointing at `vendors.toml` rows V2, V4, V8, V9; change nothing else in that file.
- **Check:** as SS-03a, from `ss-03b`.
- **Rules:** § Rules above.

### SS-05: briefs for SS-D2 and SS-D3

- **Agent:** documentation-writer. **Host:** windows. **Worktree:** `ss-05`, branched after SS-01 merged; branch `starter-uat-shops/SS-05-briefs`. **Commit:** `docs(starter-uat-shops): SS-05 briefs`.
- **Write** `briefs/SS-D2.md`, `verify/SS-D2.md`, `briefs/SS-D3.md`, `verify/SS-D3.md` under `docs/analysis/starter-uat-shops/`, in the contract § C2 and § C3 shapes. Lease owner `ss-<packet>`.
- **SS-D2 rows** (stasis room, Castle_CellBlock world 12, fresh Praxis Commando; positions from `db/resources/Worlds/Seed/spawnlist.sql` lines 1107, 1109, 1128, 1197):
  - D2-1: right-click the Basic Equipment Quartermaster (spawn 400) and use the popup item the way `store-paths.md` says; expect `store_opened` for template 300. This is the 2026-10-12 failure repro: if the store does not open, the driver captures `lab_screenshot` and `client_window_read` of the popup, reports them raw and stops the row. The driver never grades: the verify file grades D2-1 FAIL when no `store_opened` for 300 is in the server evidence, and the verifier writes the issue body under `results/issues/`.
  - D2-2: at the supplies vendor (spawn 414), buy one item with `client_vendor_buy`; expect `transaction` for template 314.
  - D2-3: sell that item back to the Quartermaster; expect `transaction` (sell) or `refused` `not_sellable`, whichever list 2 says (check `db/resources/Items/Seed/item_lists.sql` for list 2 and write the expected result into the row).
  - D2-4: right-click the Archetype Skills Trainer (spawn 401); expect `trainer_open` for 301.
  - D2-5: right-click Goa'uld Advanced Skills (spawn 450); expect `trainer_open` for 360.
- **SS-D3 rows:** [unified-uat Bank step 10](../../guides/unified-uat.md) ("Sell an item; try to drag it out of the vendor's buyback tab without paying": refused). Start from [BANK-R10](../haiku-lab-uat/briefs/bank/BANK-R10.md) and replace its vendor-open steps with the `store-paths.md` recipe. The verify file grades on `move_rejected` with `reason = 'source_container_not_player_movable'` (as [BANK-C3](../haiku-lab-uat/briefs/bank/BANK-C3.md) line 107) and an unchanged cash value.
- **Front-loaded gotchas:** the contract's list, plus: a fresh client logging straight into Castle hangs (#756), which does not apply in the stasis room; kill the screensaver first; the stasis-room NPCs are visible to every player, so never stage a second lane there.
- **Checks:** CRLF; one `### ROW` block per row and one grading section per row (`Select-String` counts equal); each brief ≤ 500 lines.
- **Rules:** contract § C7.

## Wave 2: lab (ASK-FIRST)

Every packet: **LAB: ASK-FIRST** (contract § C1). Host: windows (the lab machine). Server from D-SP2. Writes only `results/` through its verifier.

### SS-D1: run `vendors.toml`

- **Instance** `default`. Lease taken and released by the run itself.
- **Sequence:** contract § C8, exactly two calls unless the run times out: one `plan_only` call, one execute call with the ready rows, and a resume with the same `run_dir` only after a timeout.
- **Driver brief (the whole prompt for `lab-driver`):** "Run spec rows with no lease_id (the run takes and releases its own lease). Call 1: `lab_uat_run` with `sections ["vendors"]`, `plan_only: true`, `specs_dir "<ss-integration>\docs\guides\uat-specs"`, `server_version "<sha>"`; report each row's planned state as returned. If no row is ready, stop. Call 2: the same call without `plan_only`, with `rows` set to the rows call 1 did not return BLOCKED. If call 2 times out, call it once more with the same `rows` and the `run_dir` it returned. Report, raw: each row's state as the tool returned it, every `run_dir`, and nothing else. Do not grade." ([automated-uat.md § Hand a run to a cheap agent](../../guides/automated-uat.md#hand-a-run-to-a-cheap-agent).)
- **Verifier:** `general-purpose`, prompt: read the run's evidence bundle (not the driver's summary), check each SigNoz clause against ClickHouse, answer each `source = "human"` clause from the screenshots with a Haiku looker, write `results/SS-D1.md` (contract § C6) with the § C8 `## Runs` table first; plan-BLOCKED rows are graded BLOCKED with the missing tool named. Issue bodies under `results/issues/`.

### SS-D2 and SS-D3

`lab-driver` with `briefs/SS-D2.md` then `briefs/SS-D3.md`, both on instance `p2`, one after the other. Verifier per contract § C3 with `verify/SS-D2.md` and `verify/SS-D3.md`.

The wave's results ship as one docs PR from `ss-integration`.

## Wave 3a: vendors for the worlds that have none (BlockedDecision D-SS1)

Runs only if the owner answers D-SS1 with (b) or (c). With (a), mark SS-20 to SS-23 and SS-D4 `Done (not needed)` and go to wave 4.

Each of SS-20 (Tollana, world 19), SS-21 (SGC_W1, 58), SS-22 (Dakara_E1, 61), SS-23 (Beta Site E1, 23):

- **Agent:** database-persistence, with the `seed-change` skill. **Host:** windows. **Worktree:** `ss-2N`, branched at the `ss-integration` head after the previous one merged (they share no files, but one at a time keeps the id audit simple). **Commit:** `feat(seed): <world> vendor and trainer (starter-uat-shops SS-2N)`.
- **Files:** `db/resources/Items/Seed/item_lists_starter_<world>.sql`, `db/resources/Entities/Seed/entity_templates_starter_<world>.sql`, `db/resources/Worlds/Seed/spawnlist_starter_<world>.sql`, the three `\ir` lines in `db/database.sql` next to the existing `*_union_omega.sql` lines, and one new test file `crates/cell-catalog/src/cell/spawner/tests/live_db_starter_<world>_vendor.rs` registered in that folder's `mod.rs`.
- **Ids:** before writing, list used ids with `Select-String` over the seed folders and take the first free block of 10 templates, 10 spawns, 5 item lists and 100 list rows; write the chosen blocks in the commit body.
- **Content (D-SS1 (b)):** one general-goods vendor (sell, repair and recharge list 2; a buy list of tier-1 and tier-2 consumables and that faction's starter weapons and armor) and one archetype trainer (`trainer_ability_list_id` 1, as template 301). With (c), the trainer only. Price every buy row with the D-UO29 rule in `live_db_union_omega_vendor_common.rs` (`expected_price`); never stock designs 21, 55 or 3437; no tier 3 or higher.
- **Placement:** in the world's hub, next to an existing friendly NPC the zone ledger names; check the point on the world's navmesh with `Set-Location <worktree>; pwsh tools/build-lane/lane.ps1 cargo run -p cimmeria-navmesh-extractor --bin nav_inspect -- data/spaces/<world nav file> --probes <probe file>` (see `data/spaces/README.md`). Keep 2.5 m from every other NPC.
- **Guard:** the new test asserts the template ids, their lists and prices with the Omega helpers (`assert_vendor_template`, `assert_list`), and SS-02's guard covers the world automatically. Revert proof: point the buy list at 999999 once, see it fail, restore.
- **Check:** `Set-Location <worktree>; pwsh tools/build-lane/live-db-test.ps1 live_db_starter_`; `Set-Location <worktree>; pwsh tools/build-lane/lane.ps1 cargo clippy -p cimmeria-cell-catalog --tests -- -D warnings`.
- **Rules:** § Rules above.

### SS-D4: lab check of the new vendors and trainers (wave 3b, ASK-FIRST)

The orchestrator dispatches `documentation-writer` to append row blocks to `vendors.toml` for what wave 3a seeded, each with a `.gotolocation <world> <x> <y> <z>` setup at the spawn (coordinates cited from the new `spawnlist_starter_<world>.sql`):

- **Under D-SS1 (b)** (vendor and trainer): per world, `V-<world>-1` open the vendor, `V-<world>-2` buy one row, and `V-<world>-T1` open the trainer and train one ability (`trainer_open` for the new template, then `train_requested` and `train_persisted`, as V10).
- **Under D-SS1 (c)** (trainer only): per world, `V-<world>-T1` only.

Then it runs them exactly like SS-D1 (contract § C8) with `rows` limited to those ids.

## Wave 4: roll-up and close-out

### SS-98: results summary

- **Agent:** documentation-writer. Worktree `ss-integration`. Commit `docs(starter-uat-shops): SS-98 results summary`.
- **Write:** a `## Results summary` section at the top of README.md (grade counts, a per-vendor sign-off table, issues filed, lessons), and `signoff.md` listing every NEEDS_HUMAN row with its screenshot path.

### SS-99: close-out

- **Agent:** documentation-writer. **Depends on:** SS-98. Worktree `ss-integration`. Commit `docs(starter-uat-shops): SS-99 close-out`.
- **Write:** in `docs/guides/unified-uat.md`, the stasis-room debug hub table statuses (Basic Equipment Quartermaster, Archetype Skills Trainer, Goa'uld Advanced Skills), Bank step 10's notes, and the Debug Area DA-U7 row, each citing `results/<packet>.md` and the date. In `docs/content/debug-hub.md`, the vendor lines the results change. Then the status docs once: the vendor line in `docs/project-status.md` and the matching `docs/gap-analysis*` row (walk [doc-update-map.md](../../agents/doc-update-map.md); list the rows touched in the PR body). Rewrite RESUME.md to "closed". Rules § C7.
