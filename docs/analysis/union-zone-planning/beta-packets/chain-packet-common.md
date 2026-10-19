# Beta E1 chain packets: common contract

> **Type:** Reference (shared by every Beta chain packet) · **Index:** [beta-work-packets.md](../beta-work-packets.md)

Every chain packet (UB-01b, UB-10 to UB-16, UB-20 to UB-22, UB-30 to UB-36, UB-45a, UB-45b) inlines its own rules. This page holds the shared shapes, patterns, test helpers and lane commands. If a packet and this page disagree, the packet wins. Report the disagreement.

## Rules (binding; packet-coder skips CLAUDE.md)

1. PowerShell only. Start every command with `Set-Location C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\<wt>;`. Use absolute paths. No bash.
2. Never run `cargo` directly. Use `pwsh tools/build-lane/lane.ps1 cargo <cmd> -p <crate>`. Live-DB tests run through `pwsh tools/build-lane/live-db-test.ps1 <filter>`, which first reloads your own `sgw_<wt>`. Run everything in the foreground. Check that the lane log lands under your worktree's `logs/`, and that each test count is above zero (a filter that matches nothing passes).
3. Edit only the files in your packet's allowlist.
4. `docs/**/*.md` is CRLF; a seed `.sql` matches its neighbouring seed files in the working tree (CRLF here; execution-contract.md § Shared definitions, Line endings), never mixed. Write the file, then normalise it: `[IO.File]::WriteAllText('<abs>', ([IO.File]::ReadAllText('<abs>') -replace "\r?\n", "\r\n"))`. For `db/database.sql` and `.github/instructions/*.md`, read the current line endings and keep them.
5. Seeds live only in `db/resources/`. No `db/scripts/*.sql` and no migrations.
6. Files stay under 500 lines (hard cap 700). If a test file would pass 500, split it along a natural seam (offer and accept / body / restore), as `mission_701/` does.
7. Never touch the `sgw` database. No lab, no client, no Ghidra.
8. No `git stash`, no `git worktree prune`, no push. One local commit with the packet's subject.
9. Never edit Omega, SGC, Dakara or Castle files, or `docs/analysis/four-zone-planning/id-allocation.md`.
10. Each SQL block gets a comment with its provenance label (RESTORED_SOURCE, CIMMERIA_RECONSTRUCTION, CIMMERIA_AUTHORING) and its evidence.

## Seed shapes

Header for every Beta chain file (`db/resources/Content/Seed/beta_e1_<slug>_chains.sql`):

```sql
-- Beta Site E1 (world 23, Beta_Site_Evo_1) - <mission> <name> (<packet>).
-- Plan: docs/analysis/union-zone-planning/beta-work-packets.md. Block 23000-23499.
-- Chains use only per-player binds (D-UB10) and no phasing (D-UB11).
SET search_path = resources, pg_catalog;
```

Row shapes (copied from `castle_701_chains.sql` and `dakara_e1_space_chains.sql`):

```sql
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (23020, '<text> (<packet>)', 'mission', 1423, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (23020, 'player_loaded', 'Beta_Site_Evo_1', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23020, 'step_status', 1423, '4223', 'eq', 'active', 0);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (23020, 'advance_step', 1423, '4224', '{}', 0, 0);
```

`scope_type` is `'mission'` with `scope_id` = the mission, except travel and world-life chains (`'space'`, 23). Every `delay_ms` is 0 (`assert_no_deferred_actions`).

| Kind | Name | Columns |
|---|---|---|
| trigger | `player_loaded` | `event_key` = `'Beta_Site_Evo_1'` (the `worlds.sql` name) |
| trigger | `interact_tag` | `event_key` = the spawn tag (e.g. `'BetaE1_Carter'`) |
| trigger | `dialog_choice` | `event_key` = the dialog id as text (e.g. `'4641'`) |
| trigger | `item_use` | `event_key` = the item id as text |
| trigger | `enter_region` | `event_key` = the `point_sets.name` of the region |
| trigger | `mission_completed` / `mission_accepted` | `event_key` = the mission id as text |
| condition | `mission_status` | `target_id` mission, `value` `not_active`/`active`/`completed`. A missing key reads `not_active` |
| condition | `step_status` | `target_id` mission, `target_key` step id text, `value` `active`/`completed`/`not_active` |
| condition | `objective_status` | `target_id` mission, `target_key` objective id text, `value` `completed`/`active` (missing reads `not_active`) |
| condition | `archetype` | `value` text; Human `gte 1` **and** `lte 4` (the Union Human gate, execution-contract.md § Shared definitions); Asgard `eq 5`; Free Jaffa `eq 7` |
| condition | `counter` | `target_key` counter name, `value` integer text |
| action | `accept_mission`, `complete_mission` | `target_id` mission, `'{}'` |
| action | `advance_step` | `target_id` mission, `target_key` the **next** step id |
| action | `complete_objective` | `target_id` mission, `target_key` objective id. **Read the two-objective pattern first** |
| action | `display_dialog` | `target_id` dialog id |
| action | `add_dialog_set` | `target_id` dialog_set_map id, `params` `{"slot": <template id>, "mission_id": <m>}` |
| action | `remove_dialog_set` | `target_id` dialog_set_map id, `params` `{"slot": <template id>}` |
| action | `add_item` / `remove_item` | `target_id` item, `params` `{"container": 0, "qty": n}` / `{"qty": n}` |
| action | `increment_counter` | `target_key` counter name, `params` `{"amount": 1}` |
| action | `grant_stargate_address` | `target_id` stargate id |
| action | `move_entity` | `target_key` NULL, `params` `{"destination": "x,y,z", "world": "Beta_Site_Evo_1", "use_player": true}` |

## Patterns

**P1 Receiver (UB-10, UB-20, UB-30).** On `player_loaded`, gated on race and the arrival step being `active`: `advance_step` to the talk step and `add_dialog_set` for the talk dialog. It is idempotent because the arrival step leaves `active`. You also need a **restore** chain: on `player_loaded`, while the talk step is `active`, re-add the bind. Binds are not persisted (castle 1240 pattern).

**P2 Offer and accept.**
- **Offer bind:** `player_loaded` **and** `mission_completed '<previous>'` (two trigger rows on one chain). Gate on race, previous `completed` and this mission `not_active`. Action: `add_dialog_set <blurb dsm> {"slot": T, "mission_id": M}`.
- **Offer show:** `interact_tag`, with the same gates. Action: `display_dialog <offer dialog>`.
- **Accept:** `dialog_choice '<offer dialog>'`, gated on race and this mission `not_active`. Actions in order: `accept_mission M`, `remove_dialog_set <blurb dsm> {"slot": T}`, then the first step's binds.

`player_loaded` is not replayed after a mission completes in the same session, so the offer bind needs the second trigger.

**P3 Turn-in on interact (castle chain 1322 precedent).** `interact_tag` on the NPC while the last step is `active`: `display_dialog <turn-in>`, `remove_dialog_set <dsm> {"slot": T}`, `complete_mission M`. Use `complete_mission`, not the last `complete_objective` (castle 706 note: `complete_mission_direct` sends the right status byte).

**P4 Mid-step talk on interact.** `interact_tag` while step S is `active`: `display_dialog <d>`, `remove_dialog_set` if bound, `advance_step M <next>`, then the next step's bind if it is a talk step.

**P5 Object click.** `interact_tag` on an object tag while step S is `active`: optional `display_dialog <object dialog>`, `add_item` if the step names an item, then `advance_step M <next>`. Objects carry the static `interaction_type 1073741824` (INT_MissionWorldObject) from their template, so the chain needs no `set_interaction_type`.

**P6 Two-objective step (F19).** `complete_objective` completes the **whole mission** once every required objective of the current step is complete (`crates/cell-content/src/cell/missions/progression.rs:463-506`). Never complete the last open objective of a mid-mission step. For a step with objectives A and B, use four chains:

| Chain | Trigger | Extra conditions (plus step `active`) | Actions |
|---|---|---|---|
| A-mark | click A | obj A `neq completed`, obj B `neq completed` | `complete_objective M A` |
| A-last | click A | obj A `neq completed`, obj B `eq completed` | `advance_step M <next>` |
| B-mark | click B | obj B `neq completed`, obj A `neq completed` | `complete_objective M B` |
| B-last | click B | obj B `neq completed`, obj A `eq completed` | `advance_step M <next>` |

The four are pairwise disjoint, and a repeat click on a done objective matches nothing. For N objectives, partition "not all others done" by the lowest open other objective (477 uses N = 4).

**P7 Per-player binds only (D-UB10).** `add_dialog_set`'s `slot` is the NPC's template id, and the bind is per player (`executor/dialog/mod.rs:166-176`). Never `set_interaction_type` on a shared NPC tag. Each `interact_tag` chain needs a row in `crates/content-engine/tests/it/interact_tag_linter/beta_e1.rs` (see Registration).

**P8 Disjointness.** Sibling chains on one trigger key all fire together. On a shared NPC (Carter 2300, Felger 2301, Mimir 2305), every chain that displays a dialog must be exclusive against every other chain on that tag: by race, by mission status or by step status. Each packet's test proves its own chains. UB-45a and UB-45b prove the whole cast.

**P9 Quota on N objects (381 cages, 420 scanners).** No condition reads an item count, so the quota is a counter.
- One chain per object tag, `once = true`: `add_item <item> {"container": 0, "qty": 1}`, then `increment_counter <name> {"amount": 1}`.
- The turn-in on the NPC is gated on `counter <name> gte N`. A "not yet" sibling gated on `counter <name> lt N` re-displays the step's instruction dialog, so the first press always answers.
- Bind the turn-in dsm when the collect step starts. There is no event for "the Nth object was clicked".
- **Login resync:** counters live in memory and reset on relog, and `once` re-arms (UB-00a Q5). A `player_loaded` chain, gated on the collect step being `active`, runs `remove_item <item> {"qty": N}`, so the items and the counter restart together. UB-00a1 Q5 answered this: `remove_item` warns on a missing or short stack and never fails the chain, so the resync ships. Caveat (accepted, UB-U7b): it removes the first stack only.

**P10 Region with an interact fallback.** A "go to X" step advances on `enter_region '<name>'`. A player who already stands inside the region never fires the event, so the first object click in that place, while the go-to step is `active`, also advances it. Both chains `advance_step` to the same next step and are disjoint from the object's own step chain by step status.

## Registration (each chain packet)

| File | Edit |
|---|---|
| `db/database.sql` | `\ir resources/Content/Seed/beta_e1_<slug>_chains.sql` in the Beta block that UB-01b opens (after `\ir resources/Content/Seed/beta_e1_travel_chains.sql`, in packet order) |
| `crates/cell-content/src/cell/content/chain_replay_tests/beta_e1/mod.rs` | `mod m<mission>;` (alphabetical) |
| `crates/content-engine/tests/it/dialog_button_linter/seed_model.rs` | append `"beta_e1_<slug>_chains.sql",` to `CHAIN_FILES` (UB-01b turns it into a slice) |
| `crates/content-engine/tests/it/interact_tag_linter/beta_e1.rs` | one tuple per `interact_tag` chain in `BETA_E1` (UB-01b splits the linter): `("beta_e1_<slug>_chains.sql", <chain>, "<tag>: per-player bind dsm <id> (chain <binder>)"),` for an NPC, `"<tag>: template <id> static INT_MissionWorldObject"` for an object, or `"<tag>: template 3 static INT_RingNetwork (32)"` for the beam pads |
| `crates/content-engine/tests/it/onitemuse_remove_item_pairing.rs` | only if the packet adds an `item_use` trigger: list the item in `KNOWN_CONSUMABLES` or `KNOWN_REUSABLES`, with a reason |
| `.github/instructions/content-chains.instructions.md` | one row in the Beta table that UB-01b creates |

Shared-file lines conflict between packets in one wave. The review-fixer keeps both sides.

## Test module helpers (`chain_replay_tests/beta_e1/mod.rs`, created by UB-01b)

Copied from `chain_replay_tests/mission_701/mod.rs` (whose `dialog_choice_ctx` comment is stale: CS-01b sets `archetype` on dialog events, `event_dispatch/dialog.rs:39-43`):

- `pub(super) const BETA: &str = "Beta_Site_Evo_1";` `HUMANS: [i64; 4] = [1, 2, 3, 4]`, `ASGARD: i64 = 5`, `GOAULD: i64 = 6`, `FREE_JAFFA: i64 = 7`, `JAFFA: i64 = 8`, `NON_HUMANS: [i64; 4] = [5, 6, 7, 8]`.
- `engine_for(pool, chain_id) -> ChainEngine` (single chain, panics if missing) and `whole_seed(pool) -> ChainEngine` (`super::super::engine_loader::build_engine(Some(pool))`).
- `fire(engine, TriggerType, &ctx) -> ResolvedActions`, `summarize(&Action) -> String`, `summarized(&resolved, chain) -> Vec<String>`, `chains_matched(&resolved) -> BTreeSet<i64>`.
- Context builders: `login_ctx(world, archetype: Option<i64>)`, `interact_ctx(tag, archetype)`, `choice_ctx(dialog_id, archetype)`, `item_use_ctx(item_id, archetype)`, `completed_ctx(mission_id, archetype)`, plus `with_mission`, `with_step`, `with_obj`, `with_counter`.

`summarize` arms: `accept_mission(m)`, `complete_mission(m)`, `display_dialog(d)`, `add_dialog_set(dsm, slot=t)`, `remove_dialog_set(dsm, slot=t)`, `advance_step(m, s)`, `complete_objective(m, o)`, `add_item(i, n)`, `remove_item(i, n)`, `increment_counter(name)`, `grant_stargate_address(g)`, `move_entity(x,y,z)`, and `UNEXPECTED(..)` for anything else.

## Test recipe (each chain packet)

Module `chain_replay_tests/beta_e1/m<mission>.rs`. Every test is `#[tokio::test]` and starts `let pool = require_db_or_skip!();`. Each gets a `///` comment that names the revert it catches.

1. **Exact list per chain:** `engine_for(chain)`, fire the positive context, and `assert_eq!(summarized(..), vec![..])` in the exact order of the packet's table. Call `super::super::assert_no_deferred_actions`.
2. **Gate negatives per chain:** wrong race (every race outside the gate, plus `None`), wrong mission or step status, another world for `player_loaded`. Assert empty.
3. **Whole-seed exclusivity:** `whole_seed`, fire each positive context, and assert `chains_matched` equals exactly the packet's expected set. This catches a sibling chain (another packet's or this one's) that fires on the same event.
4. **Revert proof:** comment out one chain's INSERTs, rerun the live filter (the tests must fail), restore, and rerun (they must pass). Record both in the report.

## Lane checks (in order)

```powershell
Set-Location <wt>; pwsh tools/build-lane/lane.ps1 cargo fmt -p cimmeria-cell-content
Set-Location <wt>; pwsh tools/build-lane/lane.ps1 cargo clippy -p cimmeria-cell-content --all-targets -- -D warnings
Set-Location <wt>; pwsh tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-content-engine --test it dialog_button_linter interact_tag_linter onitemuse
Set-Location <wt>; pwsh tools/build-lane/live-db-test.ps1 chain_replay_tests::beta_e1
Set-Location <wt>; pwsh tools/build-lane/live-db-test.ps1 gate_beta_e1
```

`<wt>` is `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\<packet worktree>`. Run the packet's own filter first (`chain_replay_tests::beta_e1::m1423`), then the whole `beta_e1` module.

## Telemetry (SigNoz, service `cimmeria-cell`)

Each packet's acceptance names the lines its chains produce. The source strings are:

| Line | Source |
|---|---|
| `resolve_event: chain matched` (field `chain_id`) | content-engine `chain` |
| `Content: accepting mission` / `Content: advancing step` / `Content: completing mission` / `Content: complete objective` | `executor` mission arms |
| `Content: displaying dialog` | executor dialog arm |
| span `dialog.add_set` | `executor/dialog/mod.rs:166` |
| `Content: granting item` | executor item arm |
| `Content: granting stargate address` | `executor/stargate.rs:170-182` |
| `Content: teleporting entity` (target `movement.teleport`) | executor move arm |
| `fire_dialog_choice: matched` / `fire_dialog_choice: no chains matched` | `event_dispatch/dialog.rs` |
| `event="chain_refused"` | executor once/refusal gate |

Before quoting a line, the coder greps its exact text and field set, and names them in the report.

## Docs rows (doc-update-map)

- Row 41, "Game systems, content chains, or content-engine actions": one row in `.github/instructions/content-chains.instructions.md`. `docs/content/mission-chains.md` waits for UB-99.
- Row 40 (observability catalog) only if a packet adds a new log line. None should.

## Return

One line: `<UB-xx> <done|blocked|failed> <sha|-> <report path>`. The report path is `<wt>\.claude\reports\union-beta-e1\<UB-xx>-coder.md`.
