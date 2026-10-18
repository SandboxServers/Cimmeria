# Cellblock continuation: wave 5 packets (CB-N06)

> **Type:** Work packets (canonical, on `content/cellblock-campaign`)
> **Audience:** The Cellblock orchestrator, and the agents it dispatches for CB-N06s, CB-N06m, CB-N06-1 to CB-N06-7, W5a-INT and W5-INT, and their reviewers
> **Last updated:** 2026-10-17
> **Companions:** [w5-frozen.md](w5-frozen.md) (every literal these packets copy), [work-packets.md](work-packets.md) (dispatch rules, waves), [README.md](README.md) (status; D-CC18, D-CC19), [owner-lab-checklist.md § CB-U12](owner-lab-checklist.md#cb-u12--before-the-patrol-returns-in-the-native-client-after-w5-int), [w3-packets.md](w3-packets.md) (the CB-N05 packets this wave mirrors), [CB-N06 contract](../four-zone-planning/cellblock-work-packets.md), [execution contract](../four-zone-planning/execution-contract.md)

Wave 5 builds CB-N06, *Before the Patrol Returns* (mission 62060): one optional, one-time choice after stock 638. Both Op-CORE ambient prisoners offer it; the player either helps them hide (a second click on a prisoner) or jams the cell-door panel beside Prisoner 329. Either branch completes the mission and gives two Health Slappacks; the other branch then closes.

- **Owner decision D-CC18 (2026-10-17):** CB-N06 is included as clearly labelled new Cimmeria content (`CIMMERIA_AUTHORING`; never presented as 2009 content; execution contract lines 18 and 83; O14-10 allows lore-safe premises). The owner delegated the quest and prop choice. The W5 architect pre-chose them from static evidence ([w5-frozen.md](w5-frozen.md)); the seal packet CB-N06s verifies them before any build packet runs.
- **Owner decision D-CC19 (2026-10-17):** CB-N07b (Tier 1 difficulty) is omitted. The CB-N07a Tier 0 bark stays. Nothing in this wave touches the barracks or chain 11020.
- **No packet drives the lab** (owner directive, 2026-10-17). The owner runs CB-U12 after W5-INT.
- **Nine packets, sized under 40k each.** The contract's single CB-N06 packet is split the same way as CB-N05 (D-CC14): a static seal and sign-off, three data packets (W5a), then four runtime proofs on the integrated data (W5b).

## Dispatch plan

| Step | Packet | Agent | Worktree / branch | Base | Runs after |
|---|---|---|---|---|---|
| E3 | [CB-N06s](#cb-n06s--choice-seal) | `mission-systems-advisor` | `cellblock-cb-n06s` / `cellblock/cb-n06s-choice-seal` | W5 architect commit | dispatch; may run beside W3 and W4 |
| E4 | [CB-N06m](#cb-n06m--items-sign-off) | `items-systems-advisor` | `cellblock-cb-n06s` (same worktree) | CB-N06s tip | CB-N06s verdict `seal` |
| 1 | [CB-N06-1](#cb-n06-1--mission-62060-catalog-entry-and-seeds) | `packet-coder` | `cellblock-cb-n06-1` / `cellblock/cb-n06-1-mission` | **W4-INT tip** | W4-INT `done` and CB-N06m `approve` |
| 1 | [CB-N06-2](#cb-n06-2--dialog-60151-override-and-seeds) | `packet-coder` | `cellblock-cb-n06-2` / `cellblock/cb-n06-2-dialog` | W4-INT tip | same |
| 1 | [CB-N06-3](#cb-n06-3--chains-11030-11041-rows-and-linters) | `packet-coder` | `cellblock-cb-n06-3` / `cellblock/cb-n06-3-chains` | W4-INT tip | same |
| 2 | W5a review, fixes | one read-only `packet-reviewer` for the seal, sign-off and three packets; fixes by `packet-coder` in the owning worktree; fresh re-review; at most 3 rounds | - | - | step 1 returned |
| 3 | [W5a-INT](#w5a-int--integrate-the-data) | `packet-coder` | `zone-cellblock` / `content/cellblock-campaign` | branch tip | every W5a packet `ship` |
| 4 | [CB-N06-4](#cb-n06-4--resolve-matrix) | `packet-coder` | `cellblock-cb-n06-4` / `cellblock/cb-n06-4-resolve` | W5a-INT tip | W5a-INT `done` |
| 4 | [CB-N06-5](#cb-n06-5--executor-replay) | `rust-gameserver-dev` | `cellblock-cb-n06-5` / `cellblock/cb-n06-5-replay` | W5a-INT tip | W5a-INT `done` |
| 4 | [CB-N06-6](#cb-n06-6--cooked-sync-transfer-proof) | `packet-coder` | `cellblock-cb-n06-6` / `cellblock/cb-n06-6-transfer` | W5a-INT tip | W5a-INT `done` |
| 4 | [CB-N06-7](#cb-n06-7--services-round-trip) | `rust-gameserver-dev` | `cellblock-cb-n06-7` / `cellblock/cb-n06-7-round-trip` | W5a-INT tip | W5a-INT `done` |
| 5 | W5b review, fixes | as step 2 | - | - | step 4 returned |
| 6 | [W5-INT](#w5-int--integrate-wave-5-and-bookkeeping) | `packet-coder` | `zone-cellblock` | branch tip | every W5b packet `ship` |
| - | CB-U12 | **the owner** | lab | - | after W5-INT; never dispatched by an agent |

E3 and E4 write one new note only, so they may run as soon as this commit lands. The build packets wait for W4-INT because they append to the same seed files as CB-N07a. Packets inside one step touch disjoint files and run in parallel, each in its own worktree with its own `sgw_<wt>` database. CB-N06-5 and CB-N06-7 go to `rust-gameserver-dev` because their rigs stage a whole space and dispatch path ("L runner packets stall Haiku").

**Stop rules (every packet).**

- CB-N06s or CB-N06m returns `reject`: no build packet is dispatched. CB-N06 closes `Done / omitted` at CB-N08b with the seal as evidence; the ID claims are released at W5-INT's place in CB-N08b.
- CB-N06s or CB-N06m returns `change`: the orchestrator sends the seal to the architect, who edits [w5-frozen.md](w5-frozen.md) (and this file if needed) before W5a starts. Coders never apply a seal's change themselves.
- A frozen literal that a loader, validator or emitter refuses, or turns into different bytes: stop, report `blocked` with both strings. Never edit the literal or weaken validation.
- The full engine runs an aid or sabotage chain (11036-11038) on the same click that accepts (11033/11034), or runs two of 11036-11038 on one click: stop, report `blocked` (the exclusive-choice rule is broken; reseal).
- A stock chain that shares a trigger (1016, 1017, 1028 on `329_CellDoorButton`; 1152 on `mission_accepted 641`) changes its resolved result: stop, report `blocked`.
- Any needed file outside the packet's **Writes** list: stop, report `blocked`.

## Contract

| Item | Value |
|---|---|
| Feature branch | `content/cellblock-campaign` (worktree `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\zone-cellblock`) |
| Base SHA | E3: **the W5 architect commit** (the newest commit that adds this file; the overlay `RESUME.md` names it). W5a: **the W4-INT tip**. W5b: **the W5a-INT tip**. Each return line names its tip. |
| Worktree recipe | 1. Ops: `pwsh tools/build-lane/mk-worktree.ps1 <branch> <wt-name>` (branches from `main`). 2. Ops: `git -C <wt> rebase --onto <base> HEAD <branch>`. 3. Verify `git -C <wt> rev-parse HEAD` = `<base>` and `git -C <wt> status --porcelain` is empty. Stop on any mismatch; never reset. |
| Report path | `<wt>\.claude\reports\cellblock-continuation\<packet>-<role>.md` (role `coder`, `reviewer`, or the agent name) |
| Return line | Exactly one line: `<packet> <done\|blocked\|failed> <commit-sha\|-> <report-path>` |
| Measurement | Every report ends with `## Measurement`: working tokens (or `not exposed`), wall-clock start and end (UTC, `Get-Date -AsUTC`), lane runs, review rounds. |
| Claimed IDs | Mission **62060**; step **62061**; objective **62062**; task **62063**; dialog **60151**; screen **96111**; chains **11030-11041**. Reused, not claimed: dialog-set-maps **3862** (slot 176) and **7509** (slot 22), item **2893**. [id-allocation.md § CB-N06](../four-zone-planning/id-allocation.md#cb-n06-claims--2026-10-17). Never pick another ID. |
| Frozen literals | [w5-frozen.md](w5-frozen.md): F1 mission XML (1369 bytes), F2 mission seed rows, F3 dialog (392 bytes) and its seeds, F4 chain SQL and expected actions, F5 spawn and binding facts. |
| Gameplay contract (CB-N06, cellblock-work-packets.md) | "Author a bounded choice to aid a captive or sabotage a jailor device using existing interaction vocabulary. Tie any item grant/removal to server authority, with first-press feedback. Accept: each branch resolves once, declined/skipped content never blocks the escape, repeated use cannot earn both branches or duplicate items, death/relog keeps the chosen outcome, and the route finishes 688. Add a replay that asserts mutually exclusive conditions and executed actions plus a live-DB inventory/mission guard; prove both fail when the protective condition is removed." |
| Design (architect, seal verifies) | Offer only in world 12 while 638 = `completed`, 641 = `not_active`, 62060 = `not_active`. Accept only by a validated click on spawn 520 or 521. On accept: dsm 7509 binds on the panel (slot 22) and 60151 shows. During step 62061 exactly one of: prisoner click (aid) or panel click (sabotage); each grants item 2893 x2, unbinds both choices and completes objective 62062, which completes the mission. Abandon unbinds the panel and keeps the offer; 641 accepted before 62060 withdraws the offer. No XP, no naquadah, no `mission_rewards` row. 62060 is not required by 638-688. |
| Telemetry (owner rule, 2026-10-13) | Existing only: the executor's INFO lines (`Content: accepting mission`, `Content: adding dialog set`, `Content: displaying dialog`, `Content: granting item`, `Content: removing dialog set`, `Content: complete objective`), the mission spans, and `cooked_data.mission_catalog_inserted`. No new telemetry code. |
| Review (D-CC6) | One read-only `packet-reviewer` per wave; fixes by `packet-coder` (or the packet's own agent) in the owning worktree; a fresh `packet-reviewer` re-reviews; at most 3 rounds. **No `packet-review-fixer`.** Haiku agents stay under 100k tokens each. |

### Repo rules every packet carries

1. **PowerShell only**, no bash. Begin every command with `Set-Location <wt>;` and use absolute paths. `[IO.File]` calls take absolute paths only.
2. **Never run `cargo` directly.** Every fmt, compile, test or clippy goes through `pwsh tools/build-lane/lane.ps1 cargo ...` (format new Rust with `pwsh tools/build-lane/lane.ps1 cargo fmt -p <crate>`, then check with `pwsh tools/build-lane/lane.ps1 cargo fmt --all -- --check`). Live-DB tests go through `pwsh tools/build-lane/live-db-test.ps1 <filter>`, which reloads this worktree's own `sgw_<wt>` database from seeds first. Run lanes in the foreground; read their logs under `logs/<worktree-name>/`.
3. **No `git stash`, no `git worktree prune`, no `git reset`, no push.** Commit locally once the checks pass.
4. **Seeds are the source of truth.** Edit `db/resources/` only. Never add `db/scripts/*.sql`.
5. **File caps**: soft 500 lines, hard 700, for Rust and new docs. Check `(Get-Content <file>).Count` before and after. Seed SQL and `docs/content/mission-chains.md` are exempt (append only). Two exceptions, each named in its packet: `dialog_overrides/mod.rs` (641 lines) takes exactly 5 more lines in CB-N06-2; `interact_tag_linter.rs` (754 lines) takes exactly 1 more line in CB-N06-3.
6. **Line endings.** Docs under `docs/` are CRLF: after writing one, run `$p='<abs path>'; $t=[IO.File]::ReadAllText($p) -replace "`r?`n","`r`n"; [IO.File]::WriteAllText($p,$t)`; after `git add`, `git -C <wt> ls-files --eol <path>` must show `w/crlf`. Stored `i/crlf`: `castle_cellblock_chains.sql`, `docs/content/mission-chains.md`. Stored `i/lf w/crlf`: `missions.sql`, `mission_steps.sql`, `mission_objectives.sql`, `mission_tasks.sql`, `dialogs.sql`, `dialog_screens.sql`. For every seed file, `git -C <wt> diff --stat <path>` must count only your added lines; a whole-file count means the endings broke. Rust files are `i/lf`; the Edit tool keeps them. Literals copied from `w5-frozen.md` (a CRLF doc) go into Rust with LF only: the F1 test asserts exactly one `\n` and no `\r`.
7. **Edit only the files your packet lists.** Never edit the chain seed's top-of-file range header or `id-allocation.md` (D-CC8: W5-INT owns them). Never hand-edit a `<!-- gen:NAME -->` block.
8. **IDs come only from the contract's claimed list**; literals only from [w5-frozen.md](w5-frozen.md), copied byte for byte.
9. **Every test must fail when its change is reverted.** Each packet names its revert proof; do it once (revert, run, see the failure, restore, rerun green) and record the failing assertion in the report. A negative test must sit next to a passing positive in the same rig, or it is theatre.
10. Run `git -C <wt> diff --check` before committing. If a lane check fails twice for the same reason, stop and report `failed` with the log path. Do not widen the change to make it pass.

---

## CB-N06s — Choice seal

| Field | Value |
|---|---|
| Agent | `mission-systems-advisor` (static evidence only; no lab, no live server) |
| Size | About 40k working tokens |
| Branch / worktree | `cellblock/cb-n06s-choice-seal` / `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\cellblock-cb-n06s` |
| Writes | `docs/analysis/cellblock-continuation/worknotes/cb-n06-seal.md` (new, under 250 lines) only |
| Commit | `docs(cellblock): seal Before the Patrol Returns` |
| Report | `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\cellblock-cb-n06s\.claude\reports\cellblock-continuation\CB-N06s-mission-systems-advisor.md` |

**Why.** The owner delegated the quest and prop choice. The architect chose them from static evidence; this seal checks every claim in [w5-frozen.md](w5-frozen.md) against code and seeds before a coder copies anything. Read w5-frozen.md fully first, then the [CB-N05 seal](worknotes/cb-n05-seal.md) as the format precedent.

**Checks (cite file and line for each; the verdict needs every one):**

- **G1 Click path.** Spawns 520/521 (template 176) and 22 (template 22) have `interaction_type` 0 and no static sets. Trace `INTERACT` (cell-methods `player/interaction/interact.rs`) for an entity whose only interaction is a per-player `available_interactions` bind of a dialog-set-map with `dialog_id` NULL and flags 1073741824 (3862, 7509): the click must reach `fire_interact_tag` with the entity's tag, exactly as Prisoner 329's 2795 bind does for 11003. State what the client is told (topic text `Mend Slave Cages` / `Remove Safeguards to Prevent a Power Overload`: shown or not) and that the click gives visible feedback on the first press (accept: journal entry plus 60151; branch: item toast plus mission complete).
- **G2 Template side effects.** Template 176's `event_set_id` (570) and template 22's: no item, combat or mission side effect on click.
- **G3 Panel after 638.** With 638 completed, a click on `329_CellDoorButton` starts no Livewire and changes no bit (chains 1016, 1017, 1028 are gated on 638 step 2115). Quote their conditions.
- **G4 Single snapshot.** One click resolves every chain against the pre-click context (`event_dispatch/interaction.rs` line 55, then `execute_actions`), so 11033 and 11036 cannot both run on one click. Confirm the nested `mission_accepted` dispatch runs 11035 after `AcceptMission` with step 62061 active, as 11004 does for 62050.
- **G5 Completion.** Completing objective 62062 (the only objective of the only step) completes 62060 with the seeded defs. Name the code path.
- **G6 World change.** The per-player bind lives on the cell entity, which is rebuilt on a world change and on relog; template 176 also spawns in world 8 (spawns 193, 199-203). Confirm no bind reaches world 8.
- **G7 Restores.** 11031/11032 (`once` on `player_loaded`) restore the right binds after relog in each state: offered, step 62061 active, completed (none), 641 accepted (none).
- **G8 Reachability and budget.** After 640's ring-out, the player can still walk to (-271, -135), (-254, -135) and the panel, or the offer is at least reachable between 638 completion and 641 acceptance. Estimate the beat (spawn 520 to the panel is about 29 m) against the 10-minute optional budget combined with 62050 (O14-14).
- **G9 Archetypes.** Archetype 1 (Human) and 8 (Loyalist Jaffa) both reach 638 completion with the same tags; no class branch hides the cast.
- **G10 Copy and lore.** The F1/F3 copy is lore-safe for Op-CORE prisoners in a Goa'uld cell block, uses no 2009 claim, and fits the client fields (title, step, objective, monologue).
- **G11 Item.** Item 2893 grants into a valid container with `{"container": 0}` (the executor's fallback to `item_containers`); a full bag does not lose the grant silently (name the base path). CB-N06m checks this in depth.

**Seal file sections:** `## Verdict` (`seal`, `change` or `reject`, one line why); `## Checks` (G1-G11, result and evidence); `## Values` (the F5 table, corrected if wrong); `## Changes` (on `change`: the exact literal edits for the architect); `## Risks` (anything the owner must watch in CB-U12). CRLF (rule 6). Commit, then return.

Repo rules 1, 3, 6, 7, 10.

## CB-N06m — Items sign-off

| Field | Value |
|---|---|
| Agent | `items-systems-advisor` (static) |
| Size | About 20k working tokens |
| Worktree | `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\cellblock-cb-n06s` (CB-N06s tip) |
| Writes | `docs/analysis/cellblock-continuation/worknotes/cb-n06-seal.md` only: append `## Items sign-off` |
| Commit | `docs(cellblock): items sign-off for Before the Patrol Returns` |
| Report | `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\cellblock-cb-n06s\.claude\reports\cellblock-continuation\CB-N06m-items-systems-advisor.md` |

Check and record, with file and line: (1) item 2893 stacks (max 10) and a grant of 2 merges into an existing stack; (2) the content grant path end to end (`executor/inventory.rs` `grant`, `CellToBaseMsg::GrantItem` with `notify_gm: false`, base `handle_content_grant_item` in `base-methods/.../grant/grant_item.rs` line 82), including the full-bag behaviour (mail fallback or refusal, and what the player sees); (3) first-press feedback: the client sees the item arrive on the same click; (4) the reward is modest for a level-1 optional beat (O14-10) and cannot be farmed: no repeat (`num_repeats` 0), no second branch, abandon gives nothing. Verdict line: `approve`, `change` (exact edits for the architect) or `reject`.

Repo rules 1, 3, 6, 7, 10.

---

## CB-N06-1 — Mission 62060 catalog entry and seeds

| Field | Value |
|---|---|
| Agent | `packet-coder` |
| Size | About 35k working tokens |
| Branch / worktree | `cellblock/cb-n06-1-mission` / `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\cellblock-cb-n06-1` |
| Writes | `crates/resources/src/base/mission_additions.rs`; `crates/resources/src/base/resources/tests/mission_additions.rs` (one edit); `crates/resources/src/base/resources/tests/mission_additions_cellblock.rs` (two edits); `crates/resources/src/base/resources/tests/mission_additions_cellblock_62060.rs` (new, under 150 lines); `crates/resources/src/base/resources/tests/mod.rs` (one line); `db/resources/Missions/Seed/{missions,mission_steps,mission_objectives,mission_tasks}.sql` (append) |
| Commit | `feat(cellblock): add Before the Patrol Returns catalog entry` |
| Report | `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\cellblock-cb-n06-1\.claude\reports\cellblock-continuation\CB-N06-1-coder.md` |

**Why.** The client shows a mission's title, step and objective only from its cooked category 3. Mission 62060 is not in the stock pak, so the server inserts it into the served catalog (`MISSION_ADDITIONS`) and seeds the same IDs and text for the cell's defs.

**Steps.**

1. `mission_additions.rs`: apply [w5-frozen.md § F1](w5-frozen.md#f1-mission-62060-category-3-xml-cb-n06-1) exactly (new `PRISONER_CHOICE_XML` after `REMAINING_PODS_XML`, the two-entry `MISSION_ADDITIONS`, the line-2 doc comment). Leave `MAX_MISSION_XML_BYTES` and the `TEST_*` items alone.
2. Seeds: append the [§ F2](w5-frozen.md#f2-mission-seed-rows-cb-n06-1) blocks (each after the CB-N05 62050 row, before the `setval` tail). Check rule 6's `diff --stat`: 3 insertions per file (blank, comment, row).
3. `tests/mission_additions.rs` (line 125-132): replace the comment `// CB-N05 added the first production entry.` with `// CB-N05 and CB-N06 added the production entries.` and `vec![62050]` with `vec![62050, 62060]`. Nothing else in this file.
4. `tests/mission_additions_cellblock.rs` (the CB-N05 tests now share the registry):
   - line 4: `use crate::base::mission_additions::{MISSION_ADDITIONS, REMAINING_PODS_XML};`
   - in `production_cache_serves_remaining_pods`: `raw.elements.len() + 1` becomes `raw.elements.len() + MISSION_ADDITIONS.len()`.
   - in `production_load_logs_the_insert`: replace the two lines `assert_eq!(inserted.len(), 1);` and `assert!(inserted[0].has_field("mission_id", "62050"));` with a count of the events whose `mission_id` is `62050`, asserted equal to 1 (`inserted.iter().filter(|e| e.has_field("mission_id", "62050")).count()`). Nothing else.
5. New `tests/mission_additions_cellblock_62060.rs`; register `mod mission_additions_cellblock_62060;` after `mod mission_additions_cellblock;` in `tests/mod.rs`. Copy the CB-N05 file's header shape (`use super::super::*;`, `DIR`, `SEED_DIR`, an independent `EXPECTED` raw string copied again from F1, a `SEED_ROWS` table of the four F2 rows). Plain `#[test]`s:
   - `prisoner_choice_xml_is_the_frozen_bytes`: `PRISONER_CHOICE_XML == EXPECTED`, `len() == 1369`, exactly one `\n`, no `\r`.
   - `prisoner_choice_ids_are_absent_from_the_stock_pak`: no raw key 62060; for each of `62060..=62063`, no stock element's XML contains `ID="<id>"`.
   - `production_cache_serves_prisoner_choice`: `load_all(DIR)` serves 62060 as `PRISONER_CHOICE_XML.as_bytes()` and still serves 62050 as `REMAINING_PODS_XML.as_bytes()`; the category-3 element count is the raw count + 2.
   - `production_load_logs_the_62060_insert`: inside `LogCapture`, exactly one `cooked_data.mission_catalog_inserted` event has `mission_id` 62060.
   - `prisoner_choice_seed_rows_match_the_catalog`: each F2 `INSERT` line is present exactly once (normalize `\r\n`); the XML contains `MissionDefn="Before the Patrol Returns"`, the history text, the step text and the objective text verbatim.
6. Lanes, in order: `cargo fmt --all -- --check`; `cargo clippy -p cimmeria-resources -p cimmeria-base-session --all-targets -- -D warnings`; `cargo nextest run -p cimmeria-resources -p cimmeria-base-session` (the base-session cooked-sync tests use the production cache, which now carries 62060; the CB-N05-4 `resync_cellblock` tests must still pass unchanged; if one fails, stop and report `blocked` with the assertion).
7. Revert proof: remove the 62060 element from `MISSION_ADDITIONS`; `production_cache_serves_prisoner_choice` and `fixture_is_exact_stock_transform_and_ids_are_test_only` must fail. Restore.

Repo rules 1-10.

## CB-N06-2 — Dialog 60151 override and seeds

| Field | Value |
|---|---|
| Agent | `packet-coder` |
| Size | About 25k working tokens |
| Branch / worktree | `cellblock/cb-n06-2-dialog` / `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\cellblock-cb-n06-2` |
| Writes | `crates/resources/src/base/dialog_overrides/overrides_cellblock.rs` (append one const); `crates/resources/src/base/dialog_overrides/mod.rs` (exactly 5 added lines); `crates/resources/src/base/dialog_overrides/override_seed_agreement_cellblock_62060.rs` (new, under 150 lines); `crates/resources/src/base/resources/tests/dialog_overrides.rs` (lines 85-89 only); `db/resources/Dialogs/Seed/dialogs.sql`, `db/resources/Dialogs/Seed/dialog_screens.sql` (append) |
| Commit | `feat(cellblock): serve the patrol choice dialog` |
| Report | `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\cellblock-cb-n06-2\.claude\reports\cellblock-continuation\CB-N06-2-coder.md` |

**Why.** Dialog 60151 is new, so the server must push it through `DIALOG_OVERRIDES`. Authored dialogs crashed the client on 2026-09-27; 60151 uses only the served shape of 3995, 3996 and 60150: one screen, speaker 0, zero buttons, UI type 2.

**Rule exception (this packet only).** `dialog_overrides/mod.rs` is 641 lines. Add exactly the 5 lines below; `(Get-Content <abs path>).Count` afterwards = 646.

**Steps.**

1. `overrides_cellblock.rs` and `mod.rs`: apply [w5-frozen.md § F3](w5-frozen.md#f3-dialog-60151-cb-n06-2) exactly (the `PRISONER_CHOICE_DIALOG` const; the `#[cfg(test)]` / `mod override_seed_agreement_cellblock_62060;` pair after the 62050 pair at lines 59-60; the three-line array element after `overrides_cellblock::REMAINING_PODS_DIALOG,`). Do not reorder `DIALOG_OVERRIDES` and do not touch the `QUARANTINED_*` lists.
2. Seeds: append the two F3 blocks after the 60150 rows (before the `setval` tail; no setval changes: 60151 < 100011 and 96111 < 200170). `diff --stat`: dialogs.sql 4 insertions, dialog_screens.sql 3.
3. New `override_seed_agreement_cellblock_62060.rs` (template: `override_seed_agreement_cellblock_62050.rs`, its imports, `workspace_root`, `read_seed`, `sql_text`). `#[test]`s:
   - `prisoner_choice_dialog_emits_the_frozen_bytes`: `generate_dialog_xml(&PRISONER_CHOICE_DIALOG)` equals an independent `const EXPECTED: &[u8]` copied from F3; length 392; no `\n`.
   - `prisoner_choice_dialog_keeps_the_served_shape`: exactly one `DIALOG_OVERRIDES` entry has id 60151; one screen, `speaker_id` 0, no buttons, `ui_screen_type` 2, `dialog_flags` 0; 60151 is in no quarantine list.
   - `prisoner_choice_dialog_matches_its_seed`: the two F3 seed rows are present exactly once, and the seed's screen text equals the override's text.
   - `prisoner_choice_dialog_is_new_to_the_client`: the committed stock dialog pak has no `_60151` member (copy the 62050 file's approach).
4. `tests/dialog_overrides.rs` lines 85-89: the expected slice becomes `&[3995u32, 3996u32, 60150u32, 60151u32]` and the message becomes `"overridden_elements must name every regenerated dialog id (the two mission-622 corpses and the CB-N05/CB-N06 dialogs) in ascending order, and no quarantined one"`. This also clears the stale message W3a-INT recorded.
5. Existing tests that must still pass unchanged: `shipped_overrides_carry_no_buttons`, `quarantined_dialogs_are_not_served`, `all_overrides_emit_their_ids_and_text`, `every_cooked_override_element_id_fits_in_16_bits`, and all of `override_seed_agreement_cellblock_62050`. If any fails, report `blocked`.
6. Lanes: `cargo fmt --all -- --check`; `cargo clippy -p cimmeria-resources --all-targets -- -D warnings`; `cargo nextest run -p cimmeria-resources`.
7. Revert proof: remove the array element; `prisoner_choice_dialog_keeps_the_served_shape` must fail. Restore.

Repo rules 1-10, with the cap exception above.

## CB-N06-3 — Chains 11030-11041, rows and linters

| Field | Value |
|---|---|
| Agent | `packet-coder` |
| Size | About 35k working tokens (the SQL and the row literals are copies) |
| Branch / worktree | `cellblock/cb-n06-3-chains` / `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\cellblock-cb-n06-3` |
| Writes | `db/resources/Content/Seed/castle_cellblock_chains.sql` (append); `crates/content-engine/tests/it/interact_tag_linter.rs` (one line); `crates/cell-content/src/cell/content/chain_replay_tests/na13_chain_armed_aggro.rs` (the once set and its doc comment); `docs/content/mission-chains.md` (one section); `crates/cell-content/src/cell/content/chain_replay_tests/mod.rs` (one line); `.../chain_replay_tests/mission_62060/{mod.rs,rows.rs,resolve.rs,executor.rs}` (new) |
| Commit | `feat(cellblock): add Before the Patrol Returns chains` |
| Report | `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\cellblock-cb-n06-3\.claude\reports\cellblock-continuation\CB-N06-3-coder.md` |

**Rule exception (this packet only).** `interact_tag_linter.rs` is 754 lines, over the hard cap from before this campaign. Add exactly one line; afterwards the count is 755.

**Steps.**

1. Append the [w5-frozen.md § F4](w5-frozen.md#f4-chains-11030-11041-cb-n06-3) SQL block at the end of `castle_cellblock_chains.sql` (CRLF; one blank line before the banner). Do not touch the header (rule 7).
2. `interact_tag_linter.rs`: directly after the line containing `("castle_cellblock_chains.sql", 11003)`, add (8 spaces of indentation):
   ```rust
           | ("castle_cellblock_chains.sql", 11033 | 11034 | 11036 | 11037) // Cellblock_Ambient_Prisoner1/2 (CB-N06): per-player bind, dsm 3862 slot 176 (flags 1073741824) by chains 11030/11031/11032; template 176 interaction_type 0
   ```
   Chain 11038 needs no entry: `329_CellDoorButton` already has `set_interaction_type` rows. No `set_interaction_type` for the prisoners: it would light them for every player in the world.
3. `na13_chain_armed_aggro.rs`, test `live_db_once_chains_match_the_seeded_set`: the list reads `vec![1008, 1026, 1027, 1044, 11001, 11010, 11020]` after W4-INT (CB-N07a step 3b); make it `vec![1008, 1026, 1027, 1044, 11001, 11010, 11020, 11031, 11032]`. In its doc comment change `exactly the seven` to `exactly the nine` and add `11031/11032 (CB-N06 login restores)` to the once-by-design list. Nothing else in the file. If the list on your base differs from the one quoted, stop and report `blocked`.
4. `docs/content/mission-chains.md`: insert a section `#### Mission 62060: "Before the Patrol Returns" (CB-N06, CIMMERIA_AUTHORING)` directly before the line `### Castle (Persistent Zone)`, then one blank line. Content: two sentences (optional one-time choice after 638, new Cimmeria content; reward two Health Slappacks), the F4 "Expected resolved actions" table (columns Chain, Trigger, Gate, Actions), then: "Stock chains 1016/1017/1028 on the panel are gated on 638 step 2115 and stay disjoint; 1152 co-fires with 11040. Seal: `docs/analysis/cellblock-continuation/worknotes/cb-n06-seal.md`." CRLF; `ls-files --eol` shows `i/crlf w/crlf`.
5. `chain_replay_tests/mod.rs`: add `mod mission_62060;` directly after `mod mission_62050;`.
6. `mission_62060/mod.rs` (under 80 lines; copy the shape of `mission_62050/mod.rs`): module doc (what the directory proves, F4 and seal paths); `mod executor;`, `mod resolve;`, `mod rows;`; `pub(super) const CHAINS: [i32; 12] = [11030, 11031, ..., 11041];`; `pub(super) const CELLBLOCK_WORLD_ID: i32 = 12;`; the same `pub(super) fn ctx(world, params, archetype)` helper. `resolve.rs` and `executor.rs`: one line each, a module doc `//! CB-N06-4 fills this module.` / `//! CB-N06-5 fills this module.` (they compile empty; the later packets replace them, so no packet edits `mod.rs` again).
7. `mission_62060/rows.rs` (under 450 lines), `#[tokio::test]`s each opening `let pool = require_db_or_skip!();`:
   - `live_db_mission_62060_rows_are_exact`: `sqlx::query_as` over `content_triggers`, `content_conditions` and `content_actions` (`params::text`) for `chain_id BETWEEN 11030 AND 11041 ORDER BY chain_id, sort_order`; each equals a literal `Vec` copied from F4 (12 triggers, 31 conditions, 23 actions). Template: `mission_62050/rows.rs`.
   - `live_db_mission_62060_rows_load_as_sealed`: each chain loads (`load_single_chain_for_test`); `engine.get_chain_actions(id).len()` is 1, 1, 2, 1, 1, 2, 4, 4, 4, 1, 1, 1 for 11030-11041; `engine.is_once` true only for 11031 and 11032; `assert_no_deferred_actions` for each chain.
8. Lanes: `cargo fmt --all -- --check`; `cargo clippy -p cimmeria-cell-content -p cimmeria-content-engine --all-targets -- -D warnings`; `cargo nextest run -p cimmeria-content-engine --filter-expr 'not test(dialog_button_linter)'` (the interact-tag and region-key linters pass); `cargo nextest run -p cimmeria-content-engine --no-fail-fast --filter-expr 'test(dialog_button_linter)'`: **record, do not fix**: it may fail because chain 11035 displays 60151 and the `dialogs.sql` row is on CB-N06-2's branch (clears at W5a-INT); any failure that does not name 60151: stop, report `failed`; `cargo nextest run -p cimmeria-cell-content` (non-DB); `pwsh tools/build-lane/live-db-test.ps1 mission_62060` (both tests run and pass); `pwsh tools/build-lane/live-db-test.ps1 once_chains_match_the_seeded_set`.
9. Revert proofs: (a) delete the linter line; `every_interact_tag_chain_has_set_interaction_type` fails naming chain 11033; restore. (b) change 11032's trigger `once` to `false` in the seed; `rows_load_as_sealed` and `once_chains_match_the_seeded_set` fail; restore with `git -C <wt> checkout -- <file>` after the commit, or Edit before it.

Repo rules 1-10, with the cap exception above.

## W5a-INT — Integrate the data

| Field | Value |
|---|---|
| Agent | `packet-coder` |
| Size | About 15k working tokens |
| Worktree | `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\zone-cellblock` (`content/cellblock-campaign`) |
| Writes | nothing new: cherry-picks only |
| Report | `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\zone-cellblock\.claude\reports\cellblock-continuation\W5a-INT-coder.md` |

1. `git -C <zone-cellblock> status --porcelain` shows only the untracked `lab-server*.log` files (leave them; never add them).
2. `git -C <zone-cellblock> cherry-pick -x`, in order: CB-N06s, CB-N06m (both from `cellblock-cb-n06s`), CB-N06-2, CB-N06-1, CB-N06-3, each followed by its fix commits. CB-N06-2 lands before CB-N06-3 (the dialog-button linter needs the 60151 row). On a conflict: `git cherry-pick --abort`, report `blocked` with the paths.
3. Lanes: `cargo fmt --all -- --check`; `cargo clippy -p cimmeria-resources -p cimmeria-base-session -p cimmeria-cell-content -p cimmeria-content-engine --all-targets -- -D warnings`; `cargo nextest run` for the same four crates (`dialog_button_linter` included, now green); `pwsh tools/build-lane/live-db-test.ps1 mission_62060`; `pwsh tools/build-lane/live-db-test.ps1 chain_replay_tests`.
4. Return the tip SHA (the W5b base). No bookkeeping commit. Carry any review notes to the report's `## Notes for W5-INT`.

Repo rules 1, 2, 3, 10.

---

## CB-N06-4 — Resolve matrix

| Field | Value |
|---|---|
| Agent | `packet-coder` |
| Size | About 35k working tokens (table-driven) |
| Branch / worktree | `cellblock/cb-n06-4-resolve` / `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\cellblock-cb-n06-4` |
| Writes | `crates/cell-content/src/cell/content/chain_replay_tests/mission_62060/resolve.rs` (replace the stub; under 500 lines) only |
| Commit | `test(cellblock): pin Before the Patrol Returns gates` |
| Report | `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\cellblock-cb-n06-4\.claude\reports\cellblock-continuation\CB-N06-4-coder.md` |

**Why.** This is the contract's "mutually exclusive conditions" half: every gate and every refusal, with the stock chains present.

Template: `mission_62050/resolve.rs` (engine `build_engine(Some(&pool)).await`, the `TriggerEvent` shape, the `super::{CHAINS, CELLBLOCK_WORLD_ID, ctx}` helpers). All `#[tokio::test]`, names starting `live_db_mission_62060_`, each opening `let pool = require_db_or_skip!();`. Keep only `(chain_id, action)` pairs whose chain is in `CHAINS` unless a test says "unfiltered". `OFFER` = world 12, `mission_638_status` `completed` (641, 62060 unset). `STEP` = world 12, `mission_62060_status` `active`, `mission_62060_step_62061_status` `active`, 638 `completed`.

- `..._offer_gates`: 11030 (`MissionCompleted` 638) and 11031 (`PlayerLoaded` `Castle_CellBlock`). Positive in `OFFER`: exactly the F4 `AddDialogSet { 3862, 176, None }` from that chain. Refusals (zero CB-N06 actions each): world `None`; world 13; 638 `active`; 641 `active`; 641 `completed`; 62060 `active`; 62060 `completed`; wrong key (`mission_id` 637; `world_name` `Castle`).
- `..._accept_is_exclusive`: `InteractTag` `Cellblock_Ambient_Prisoner1` and `...2` in `OFFER`: exactly `[(11033 or 11034, AcceptMission { mission_id: 62060 })]`, and **no** 11036/11037 action (the accept click cannot complete). Same refusals as the offer gates. `329_CellDoorButton` in `OFFER`: zero CB-N06 actions. `Prisoner_329` in `OFFER`: zero CB-N06 actions.
- `..._follow_up`: `MissionAccepted` 62060 with world 12 and step 62061 `active`: exactly 11035's two actions in order. Refusals: world `None`; step unset; step `completed`; `mission_id` 62050 (and, unfiltered, 11004 still resolves for 62050).
- `..._one_branch_per_click`: in `STEP`, each of the three tags gives exactly one CB-N06 chain (11036, 11037, 11038 respectively) with its four F4 actions in order, and no `AcceptMission`. With step 62061 `completed` and 62060 `completed`: zero CB-N06 actions for all three tags.
- `..._panel_stock_chains_disjoint`: `329_CellDoorButton`, **unfiltered**, in `STEP` and in `OFFER`: no action from chains 1016, 1017 or 1028. Then with 638 `active` and `mission_638_step_2115_status` `active`: 1016 still resolves as before (copy its expected action from `mission_638` tests) and no CB-N06 action.
- `..._restores_and_withdrawals`: 11032 positive in `STEP` (both binds, in order), refusals step unset and world 13; 11039 `MissionAbandoned` 62060 with world 12: `RemoveDialogSet { 7509, 22 }`; 11040 `MissionAccepted` 641 with 62060 unset: `RemoveDialogSet { 3862, 176 }`, refused with 62060 `active` or `completed`, and unfiltered 1152's `DisplayDialog { 4000 }` still resolves; 11041 `MissionAbandoned` 62060 with 641 `active`: `RemoveDialogSet { 3862, 176 }`, refused with 641 unset.
- `..._no_dialog_path_moves_the_mission`: `DialogChoice` (`dialog_id` 60151, `button_id` -1 then 0) and `DialogOpen` (`dialog_id` 60151), in `OFFER` and in `STEP`: zero CB-N06 actions.

Lanes: `cargo fmt --all -- --check`; `cargo clippy -p cimmeria-cell-content --all-targets -- -D warnings`; `pwsh tools/build-lane/live-db-test.ps1 mission_62060` (every test runs and passes). Revert proof: delete the seed condition row `(11033, 'mission_status', 62060, ...)`; the 62060 `active` refusal in `accept_is_exclusive` fails. Restore with `git -C <wt> checkout -- db/resources/Content/Seed/castle_cellblock_chains.sql`.

Repo rules 1-10.

## CB-N06-5 — Executor replay

| Field | Value |
|---|---|
| Agent | `rust-gameserver-dev` |
| Size | About 40k working tokens |
| Branch / worktree | `cellblock/cb-n06-5-replay` / `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\cellblock-cb-n06-5` |
| Writes | `crates/cell-content/src/cell/content/chain_replay_tests/mission_62060/executor.rs` (replace the stub; under 500 lines) only |
| Commit | `test(cellblock): replay Before the Patrol Returns through the executor` |
| Report | `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\cellblock-cb-n06-5\.claude\reports\cellblock-continuation\CB-N06-5-rust-gameserver-dev.md` |

**Why.** The contract's "executed actions" half: the nested accept, the per-player binds, the monologue, one grant per player, and the replays (abandon, 641, relog, repeat clicks).

**Rig:** copy `mission_62050/executor.rs` (CB-N05-7) and change: spawn tags `Cellblock_Ambient_Prisoner1`, `Cellblock_Ambient_Prisoner2`, `329_CellDoorButton`; stamp the world rows; `mgr.dialog_set_maps` and monologue ids as there; archetype per test (1 or 8). Events: `fire_interact_tag(PLAYER, PLAYER_ID, tag, npc_id, &engine, &tx, &mut mgr)`, `fire_player_loaded(.., "Castle_CellBlock", ..)`, `execute_actions` with `Action::CompleteMission { mission_id: 638 }` (the 11030 offer) or `Action::AbandonMission { mission_id: 62060 }`. Observables: binds per [F5](w5-frozen.md#f5-spawn-and-binding-facts-the-tests-use); `onDialogDisplay` (method 105, args 4..8 = 60151); grants = drained `CellToBaseMsg::GrantItem { item_id: 2893, count: 2, .. }` to the player; mission state from the player's mission list.

**Tests** (`#[tokio::test]`, names `live_db_mission_62060_replay_*`):

1. `..._offer_accept_shows_the_choice` (archetype 1 and 8): 638 active at its last step, `CompleteMission 638`: slot 176 holds 3862. Prisoner 1 click: 62060 active at 62061, slot 22 holds 7509, slot 176 still holds 3862, exactly one `onDialogDisplay` 60151, **zero** `GrantItem`.
2. `..._aid_grants_once`: from test 1's state, a Prisoner 2 click: exactly one `GrantItem` (2893 x2), 62060 completed, slots 22 and 176 empty. Then clicks on Prisoner 1, Prisoner 2 and the panel: zero `GrantItem`, zero `AcceptMission` effect, 62060 still completed.
3. `..._sabotage_grants_once`: after the accept, a panel click: exactly one `GrantItem`, 62060 completed, both slots empty. Then a prisoner click and a second panel click: nothing.
4. `..._abandon_and_641`: accept, `AbandonMission 62060`: slot 22 empty, slot 176 holds 3862 (11039 only); re-accept works. Separately: a player who never takes 62060 accepts 641 (`Rig::accept(641)` or the rig's equivalent): slot 176 empty and 4000 shows (1152); a prisoner click then accepts nothing. Then: a player who took 62060, accepts 641, and abandons 62060: slot 176 empty (11041).
5. `..._relog_keeps_the_outcome`: fresh rigs, `fire_player_loaded`: 638 completed binds 176 once (a second load adds no duplicate); 62060 at 62061 binds 176 and 22; 62060 completed binds nothing and a prisoner click accepts nothing; unstamped world rows bind nothing.

Lanes: `cargo fmt --all -- --check`; `cargo clippy -p cimmeria-cell-content --all-targets -- -D warnings`; `pwsh tools/build-lane/live-db-test.ps1 mission_62060`. Revert proof (the contract's "protective condition removed"): in the worktree only, delete chain 11038's `step_status` condition row; test 3's second panel click then grants again and the test fails. Restore with `git -C <wt> checkout -- db/resources/Content/Seed/castle_cellblock_chains.sql`.

Repo rules 1-10.

## CB-N06-6 — Cooked-sync transfer proof

| Field | Value |
|---|---|
| Agent | `packet-coder` |
| Size | About 25k working tokens |
| Branch / worktree | `cellblock/cb-n06-6-transfer` / `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\cellblock-cb-n06-6` |
| Writes | `crates/base-session/src/base/cooked_sync/tests/resync_cellblock_62060.rs` (new, under 200 lines); `crates/base-session/src/base/cooked_sync/tests/mod.rs` (one line) |
| Commit | `test(cellblock): prove the patrol choice cooked-sync transfer` |
| Report | `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\cellblock-cb-n06-6\.claude\reports\cellblock-continuation\CB-N06-6-coder.md` |

Register `mod resync_cellblock_62060;` directly after `mod resync_cellblock;`. Template: `resync_cellblock.rs` (CB-N05-4) and its shared helpers; the production `committed_cache()` already carries 62060 and 60151. Grep `47_110` and `47_111` first (both must be unused). Two `#[tokio::test]`s:

- `prisoner_choice_mission_reaches_a_stock_client` (port 47_110): client category 3 from `load_raw_mission_category_for_test`; request as the template does; exactly one fragment head `(3, 62060)` with flags `FRAG_FIRST_AND_LAST` (1369 bytes fit one fragment); its body equals an independent `const EXPECTED: &str` copy of F1; after `apply_all` the client holds 62060 with those bytes and the server's category-3 version.
- `prisoner_choice_dialog_reaches_a_stock_client` (port 47_111): client category 5 = `server_entries(5)` minus 60151, stale version; exactly one head `(5, 60151)`, `FRAG_FIRST_AND_LAST`, body equal to an independent copy of F3 (392 bytes); the client then holds it.

Lanes: `cargo fmt --all -- --check`; `cargo clippy -p cimmeria-base-session --all-targets -- -D warnings`; `cargo nextest run -p cimmeria-base-session`. Revert proof: change one byte of the first test's `EXPECTED`; it fails on the body comparison. Restore. Record that `resync.rs` and `resync_cellblock.rs` still pass unchanged.

Repo rules 1-10.

## CB-N06-7 — Services round trip

| Field | Value |
|---|---|
| Agent | `rust-gameserver-dev` |
| Size | About 40k working tokens |
| Branch / worktree | `cellblock/cb-n06-7-round-trip` / `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\cellblock-cb-n06-7` |
| Writes | `crates/services/src/mission_62060_round_trip_tests.rs` (new, under 450 lines); `crates/services/src/lib.rs` (two lines: `#[cfg(test)]` and `mod mission_62060_round_trip_tests;` after the `mission_62050_round_trip_tests` pair) |
| Commit | `test(cellblock): round-trip the patrol choice grant and persistence` |
| Report | `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\cellblock-cb-n06-7\.claude\reports\cellblock-continuation\CB-N06-7-rust-gameserver-dev.md` |

**Why.** The contract's "live-DB inventory/mission guard": a real click, through the wire dispatcher, reaches the base, which writes exactly two slappacks once, and the outcome persists per character.

**Rig.** Copy `bank_org_round_trip_tests.rs` (real `dispatch(PLAYER, INTERACT, ..)`, the base's real `handle_cell_message`) and `consumable_round_trip_tests.rs` (`sgw_inventory` setup, `teardown`, the `Base` forwarder), plus `mission_62050_round_trip_tests.rs` (CB-N05-6) for `sgw_mission` persistence and the relog rebuild. Stage `Castle_CellBlock` with the spawn records tagged `Cellblock_Ambient_Prisoner1`, `Cellblock_Ambient_Prisoner2` and `329_CellDoorButton`, the real mission defs, stamped world rows, dialog-set maps, and the full engine. Sentinels (grep first; all must be unused): account **0x7000_8F10**, players **0x7000_8F11** and **0x7000_8F12**, entity **0x7000_8F20**; port **40940**. Each test cleans its own rows first and last. Give the player 638 completed, then `fire_player_loaded` (chain 11031 binds the offer), then place the player 1 m from the clicked entity before each click.

**Tests** (`#[tokio::test]`, names `live_db_mission_62060_round_trip_*`):

1. `..._aid_then_panel_grants_two_once`: INTERACT Prisoner 1 (accept), forward to the base; INTERACT Prisoner 2 (aid), forward: `sgw_inventory` holds exactly one row of item 2893 for the character with stack 2, and the saved mission row is 62060 completed. Move to the panel; INTERACT it: still exactly 2 slappacks, no new row.
2. `..._far_click_refused`: the player 6 m from Prisoner 1 (beyond `MAX_INTERACT_DISTANCE` 5.0): 62060 absent, no 2893 row. Then 1 m: accepted (the positive beside the negative).
3. `..._relog_keeps_the_outcome_per_character`: after test 1's flow for player A, rebuild a fresh manager from A's saved rows and `fire_player_loaded`: 62060 completed, no binds, an INTERACT on each prisoner and the panel changes nothing, still 2 slappacks. Player B on the same account has no 62060 row and no 2893 row.

Lanes: `cargo fmt --all -- --check`; `cargo clippy -p cimmeria-services --all-targets -- -D warnings`; `pwsh tools/build-lane/live-db-test.ps1 mission_62060_round_trip`. Revert proof: in the worktree only, delete chain 11037's `step_status` condition row and rerun; test 1 must fail (a second grant or a second row). Restore with `git -C <wt> checkout -- db/resources/Content/Seed/castle_cellblock_chains.sql`.

Repo rules 1-10.

## W5-INT — Integrate wave 5 and bookkeeping

| Field | Value |
|---|---|
| Agent | `packet-coder` |
| Size | About 30k working tokens |
| Worktree | `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\zone-cellblock` |
| Writes | cherry-picks, then one commit touching `db/resources/Content/Seed/castle_cellblock_chains.sql` (header line only), `docs/analysis/four-zone-planning/id-allocation.md`, `docs/analysis/cellblock-continuation/README.md`, `docs/analysis/cellblock-continuation/worknotes/cb-n06.md` (new) |
| Commit | `docs(cellblock): W5 integrate CB-N06` |
| Report | `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\zone-cellblock\.claude\reports\cellblock-continuation\W5-INT-coder.md` |

1. Cherry-pick (`-x`) CB-N06-4, -5, -6, -7 and their fix commits, as in W5a-INT step 2.
2. Sibling recheck (record the output): `git -C <zone-cellblock> grep -n "62060\|60151\|96111" origin/main -- crates db/resources` and the same for the local branches `content/castle-campaign`, `content/harset-campaign`, `content/tollana-campaign` if they exist; and `git grep -n "VALUES (110[34][0-9]," <branch> -- db/resources/Content`. Any reuse of 62060-62063, 60151, 96111 or 11030-11041: stop, report `blocked`.
3. Bookkeeping commit:
   - Chain seed header: one line directly before the line starting `--   (next free inside 1001-1111:`: `--   Cellblock optional choice CB-N06 (2026-10-17): chains 11030-11041, mission 62060 "Before the Patrol Returns" (id-allocation.md, CB-N06 claims).` (`diff --stat`: 1 line.)
   - `id-allocation.md` § CB-N06 claims: every `Status` cell becomes `In use (W5-INT)`; in "Chain IDs in use today", the `castle_cellblock_chains.sql` row: count +12, append `, 11030–11041` in the same style. **Last updated** becomes the commit date.
   - Branch `README.md`: CB-N06s, CB-N06m, CB-N06-1 to -7 -> `Integrated` with SHAs; CB-N06 -> `Integrated, UATPending (CB-U12)`; nothing else.
   - `worknotes/cb-n06.md` (new, under 80 lines): commits integrated, test names per packet, revert proofs, what is proven statically versus what only CB-U12 proves (client renders the journal and 60151, first-press feedback on both branches, the slappacks arrive, relog in the real client), and `## Follow-ups` from W5a-INT's notes.
   - CRLF for all docs (rule 6).
4. Lanes: `cargo fmt --all -- --check`; `cargo clippy -p cimmeria-resources -p cimmeria-base-session -p cimmeria-cell-content -p cimmeria-services -p cimmeria-content-engine --all-targets -- -D warnings`; `cargo nextest run` for the same five crates; `pwsh tools/build-lane/live-db-test.ps1 mission_62060`; `pwsh tools/build-lane/live-db-test.ps1 chain_replay_tests`.
5. Return the tip SHA. CB-U12 and CB-N08a run on it.

Repo rules 1, 2, 3, 6, 10.

## Owner lab row

CB-U12 is the owner's, after W5-INT ([owner-lab-checklist.md § CB-U12](owner-lab-checklist.md#cb-u12--before-the-patrol-returns-in-the-native-client-after-w5-int)). A blank dialog, a client crash, a missing first-press response, a lost or doubled reward, or a broken core route blocks the campaign PR. If the owner dislikes the copy or the reward, the rollback is a fix packet: `enabled = false` on chains 11030-11041, 60151 into the dialog quarantine, and 62060 out of `MISSION_ADDITIONS`; CB-N06 then closes `Done / omitted`.
