# Cellblock continuation: wave 3 packets

> **Type:** Work packets (canonical, on `content/cellblock-campaign`)
> **Audience:** The Cellblock orchestrator, and the agents it dispatches for CB-N05-1 to CB-N05-7, W3a-INT and W3-INT, and their reviewers
> **Last updated:** 2026-10-17
> **Companions:** [w3-frozen.md](w3-frozen.md) (every literal these packets copy), [CB-N05 seal](worknotes/cb-n05-seal.md), [work-packets.md](work-packets.md) (dispatch rules, waves), [README.md](README.md) (status, D-CC decisions), [owner-lab-checklist.md](owner-lab-checklist.md), [CB-N05 contract](../four-zone-planning/cellblock-work-packets.md#cb-n05--the-remaining-pods-first-authored-catalog-proof)

Wave 3 builds CB-N05, *The Remaining Pods* (mission 62050): Cellblock's first authored mission and the repo's first production entry in `MISSION_ADDITIONS`. It is optional, one-time and has no reward. Prisoner 329 offers it after stock 638; accepting it shows one monologue (dialog 60150); telling Col. Marsh completes it.

- **No lab in this wave (owner directive, 2026-10-17).** The owner runs every lab row at the end ([owner-lab-checklist.md](owner-lab-checklist.md)). D-CC13 builds CB-N05 before CB-U09 (pacing) and CB-U10 (native client proof). Both stay UATPending for the owner, and the campaign PR waits for them.
- **Seven build packets in two parallel waves.** The contract's single CB-N05 packet is far over 40k working tokens, so D-CC14 splits it along file seams. W3a writes the data (catalog, dialog, chains) and its own unit and resolve tests. W3b proves the runtime paths on the integrated data.
- **CB-N05b gets no packet.** The Castle evidence hand-in is BlockedDependency on Harset HA-1360's possession and consume primitive (not on `main`). **CB-N06 gets no packet.** It is BlockedEvidence (CB-U09, CB-U10) and BlockedDecision (D-CC15). See [§ Held](#held-cb-n05b-and-cb-n06).

## Dispatch plan

| Step | Packet | Agent | Worktree / branch | Base | Runs after |
|---|---|---|---|---|---|
| 1 | [CB-N05-1](#cb-n05-1--mission-62050-catalog-entry-and-seeds) | `packet-coder` | `cellblock-cb-n05-1` / `cellblock/cb-n05-1-mission` | W3 architect commit | dispatch |
| 1 | [CB-N05-2](#cb-n05-2--dialog-60150-override-and-seeds) | `packet-coder` | `cellblock-cb-n05-2` / `cellblock/cb-n05-2-dialog` | W3 architect commit | dispatch |
| 1 | [CB-N05-3](#cb-n05-3--chains-11000-11005-and-resolve-matrix) | `packet-coder` | `cellblock-cb-n05-3` / `cellblock/cb-n05-3-chains` | W3 architect commit | dispatch |
| 2 | W3a review | `packet-reviewer` (read-only), one for all three | the three worktrees | - | all of step 1 `done` |
| 3 | Fixes, then re-review | `packet-coder` in the owning worktree, then a fresh `packet-reviewer` | owning worktree | - | only on `fix`; at most 3 rounds (D-CC6) |
| 4 | [W3a-INT](#w3a-int--integrate-the-data) | `packet-coder` | `zone-cellblock` / `content/cellblock-campaign` | branch tip | every W3a packet `ship` |
| 5 | [CB-N05-4](#cb-n05-4--cooked-sync-transfer-proof) | `packet-coder` | `cellblock-cb-n05-4` / `cellblock/cb-n05-4-transfer` | W3a-INT commit | W3a-INT `done` |
| 5 | [CB-N05-5](#cb-n05-5--interaction-dispatch-negatives) | `rust-gameserver-dev` | `cellblock-cb-n05-5` / `cellblock/cb-n05-5-dispatch` | W3a-INT commit | W3a-INT `done` |
| 5 | [CB-N05-6](#cb-n05-6--persistence-round-trip) | `packet-coder` | `cellblock-cb-n05-6` / `cellblock/cb-n05-6-persist` | W3a-INT commit | W3a-INT `done` |
| 5 | [CB-N05-7](#cb-n05-7--executor-replay) | `rust-gameserver-dev` | `cellblock-cb-n05-7` / `cellblock/cb-n05-7-replay` | W3a-INT commit | W3a-INT `done` |
| 6 | W3b review, fixes | as steps 2 and 3 | the four worktrees | - | all of step 5 `done` |
| 7 | [W3-INT](#w3-int--integrate-wave-3-and-bookkeeping) | `packet-coder` | `zone-cellblock` | branch tip | every W3b packet `ship` |
| - | CB-U09, CB-U10 | **the owner** | lab | - | after W3-INT; never dispatched by an agent |

Packets inside one step touch disjoint files and run in parallel, each in its own worktree with its own `sgw_<wt>` database. CB-N05-5 and CB-N05-7 go to `rust-gameserver-dev` because their rigs stage a whole space and dispatch path; the memory note "L runner packets stall Haiku" applies.

**Stop rules (every packet).**

- A frozen literal ([w3-frozen.md](w3-frozen.md)) that a loader, validator or emitter refuses, or turns into different bytes: stop, report `blocked` with both strings. Never edit the literal or weaken validation.
- The full engine does not run the matching core chain and the optional chain together at Marsh (1051 or 1052 beside 11005): stop, report `blocked` (seal rule: reseal).
- The last objective of the last step does not complete the mission with the seeded defs: stop, report `blocked`. Never complete it by hand in a test.
- Any needed file outside the packet's **Writes** list: stop, report `blocked`.

## Contract

| Item | Value |
|---|---|
| Feature branch | `content/cellblock-campaign` (worktree `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\zone-cellblock`) |
| Base SHA | W3a packets: **the W3 architect commit** (the newest commit that adds this file; the main-checkout overlay `RESUME.md` names it). W3b packets: **the W3a-INT commit** (W3a-INT's return line names it). |
| Worktree recipe | 1. Ops: `pwsh tools/build-lane/mk-worktree.ps1 <branch> <wt-name>` (branches from `main`). 2. Ops: `git -C <wt> rebase --onto <base> HEAD <branch>`. 3. Verify `git -C <wt> rev-parse HEAD` = `<base>` and `git -C <wt> status --porcelain` is empty. Stop on any mismatch; never reset. |
| Report path | `<wt>\.claude\reports\cellblock-continuation\<packet>-<role>.md` (role `coder`, `reviewer`, `rust-gameserver-dev`) |
| Return line | Exactly one line: `<packet> <done\|blocked\|failed> <commit-sha\|-> <report-path>` |
| Measurement | Every report ends with `## Measurement`: working tokens (or `not exposed`), wall-clock start and end (UTC, `Get-Date -AsUTC`), lane runs, review rounds. |
| Claimed IDs | Mission **62050**; steps **62051** (A), **62054** (B); objectives **62052**, **62055**; tasks **62053**, **62056**; dialog **60150**; screen **96110**; chains **11000-11005**. Reused, not claimed: dialog-set-map **2795** (slot 17). [id-allocation.md § CB-N05](../four-zone-planning/id-allocation.md#cb-n05-individual-claims--2026-10-14). Never pick another ID. |
| Frozen literals | [w3-frozen.md](w3-frozen.md): F1 mission XML (1678 bytes), F2 mission seed rows, F3 dialog (305 bytes) and its seeds, F4 chain SQL and expected actions, F5 spawn and binding facts. |
| Gameplay contract (seal) | Offer only in world 12 while 638 = `completed`, 641 = `not_active`, 62050 = `not_active`. Accept only by a validated Prisoner 329 interaction. On accept: A -> B, the offer leaves, 60150 shows. Marsh completes B only while 640 = `completed` and step 62054 is active. No `dialog_choice`, `dialog_open` or `initialResponse` path changes 62050. Abandon restores the offer; completion never does (one-time). No reward. |
| Telemetry (owner rule, 2026-10-13) | Existing only: the executor's INFO lines (`Content: accepting mission`, `Content: advancing step`, `Content: removing dialog set`, `Content: displaying dialog`, `Content: adding dialog set`, `Content: complete objective`), the mission spans, and the resource loader's `cooked_data.mission_catalog_inserted` event. No new telemetry code. |
| Review (D-CC6) | One read-only `packet-reviewer` per wave; fixes by `packet-coder` (or the packet's own agent) in the owning worktree; a fresh `packet-reviewer` re-reviews; at most 3 rounds. **No `packet-review-fixer`.** Haiku agents stay under 100k tokens each. |

### Repo rules every packet carries

1. **PowerShell only**, no bash. Begin every command with `Set-Location <wt>;` and use absolute paths. `[IO.File]` calls take absolute paths only.
2. **Never run `cargo` directly.** Every fmt, compile, test or clippy goes through `pwsh tools/build-lane/lane.ps1 cargo ...` (fmt too: `pwsh tools/build-lane/lane.ps1 cargo fmt --all -- --check`). Live-DB tests go through `pwsh tools/build-lane/live-db-test.ps1 <filter>`, which reloads this worktree's own `sgw_<wt>` database from seeds first. Run lanes in the foreground; read their logs under `logs/<worktree-name>/`.
3. **No `git stash`, no `git worktree prune`, no `git reset`, no push.** Commit locally once the checks pass.
4. **Seeds are the source of truth.** Edit `db/resources/` only. Never add `db/scripts/*.sql`.
5. **File caps**: soft 500 lines, hard 700, for Rust and new docs. Check `(Get-Content <file>).Count` before and after. Seed SQL and `docs/content/mission-chains.md` are exempt (append only). One exception (D-CC14): `dialog_overrides/mod.rs` is already 635 lines; CB-N05-2 may add at most 6 lines to it and nothing else.
6. **Line endings.** Docs under `docs/` are CRLF: after writing one, run `$p='<abs path>'; $t=[IO.File]::ReadAllText($p) -replace "`r?`n","`r`n"; [IO.File]::WriteAllText($p,$t)`; after `git add`, `git -C <wt> ls-files --eol <path>` must show `w/crlf`. Stored `i/crlf`: `castle_cellblock_chains.sql`, `docs/content/mission-chains.md`. Stored `i/lf w/crlf`: `missions.sql`, `mission_steps.sql`, `mission_objectives.sql`, `mission_tasks.sql`, `dialogs.sql`, `dialog_screens.sql`. For every seed file, `git -C <wt> diff --stat <path>` must count only your added lines; a whole-file count means the endings broke. Rust files are `i/lf`; the Edit tool keeps them.
7. **Edit only the files your packet lists.** Never edit the chain seed's top-of-file range header or `id-allocation.md` (D-CC8: W3-INT owns them).
8. **IDs come only from the contract's claimed list**; literals only from [w3-frozen.md](w3-frozen.md), copied byte for byte.
9. **Every test must fail when its change is reverted.** Each packet names its revert proof; do it once (revert, run, see the failure, restore) and record the failing assertion in the report.
10. Run `git -C <wt> diff --check` before committing. If a lane check fails twice for the same reason, stop and report `failed` with the log path. Do not widen the change to make it pass.

---

## CB-N05-1 — Mission 62050 catalog entry and seeds

| Field | Value |
|---|---|
| Agent | `packet-coder` |
| Size | About 35k working tokens |
| Branch / worktree | `cellblock/cb-n05-1-mission` / `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\cellblock-cb-n05-1` |
| Writes | `crates/resources/src/base/mission_additions.rs`; `crates/resources/src/base/resources/tests/mission_additions.rs` (two edits only); `crates/resources/src/base/resources/tests/mission_additions_cellblock.rs` (new); `crates/resources/src/base/resources/tests/mod.rs` (one line); `db/resources/Missions/Seed/{missions,mission_steps,mission_objectives,mission_tasks}.sql` (append) |
| Commit | `feat(cellblock): add Remaining Pods catalog entry` |
| Report | `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\cellblock-cb-n05-1\.claude\reports\cellblock-continuation\CB-N05-1-coder.md` |

**Why.** The client shows a mission's title, steps and objectives only from its cooked category 3 (`CookedDataMissions.pak`). Mission 62050 is not in the stock pak, so the server must insert it into the served catalog (`MISSION_ADDITIONS`) and seed the same IDs and text so the cell's mission defs agree.

**Steps.**

1. `mission_additions.rs`: apply [w3-frozen.md § F1](w3-frozen.md#f1-mission-62050-category-3-xml-cb-n05-1) exactly (new `REMAINING_PODS_XML`, one-entry `MISSION_ADDITIONS`, line-2 doc comment). Leave `MAX_MISSION_XML_BYTES` and the `TEST_*` items alone.
2. Seeds: append the [§ F2](w3-frozen.md#f2-mission-seed-rows-cb-n05-1) blocks to the four mission seed files (after the last `INSERT`, before the `--` TOC / `setval` tail). Check rule 6's `diff --stat`.
3. `tests/mission_additions.rs`, two edits only (the production registry is no longer empty):
   - At the end of `fixture_is_exact_stock_transform_and_ids_are_test_only` (line 125), replace `assert!(MISSION_ADDITIONS.is_empty());` with `assert_eq!(MISSION_ADDITIONS.iter().map(|a| a.mission_id).collect::<Vec<_>>(), vec![62050]);`. Do not rename the test (renames churn the reviewer); add the comment `// CB-N05 added the first production entry.` above it.
   - In `insertion_preserves_served_baseline_and_bumps_once` (lines 130-138): today it compares `load_all(DIR)` against `load_all_with_mission_additions(DIR, &[])`. Change it so `served = ResourceCache::load_all(DIR).unwrap()` is compared, per `CATEGORY_PAKS`, against `ResourceCache::load_all_with_mission_additions(DIR, MISSION_ADDITIONS).unwrap()`, and bind `let baseline = ResourceCache::load_all_with_mission_additions(DIR, &[]).unwrap();` so every later use of `baseline` in the test still means "no additions" (so line 139's "baseline lacks 61000" and the loop at 164-169 keep their meaning). Do not touch anything else in this file (610 lines).
4. New `tests/mission_additions_cellblock.rs` (register `mod mission_additions_cellblock;` after `mod mission_additions;` in `tests/mod.rs`). Reuse that file's style: `const DIR` as in `tests/mission_additions.rs` line 17, and `crate::test_support::LogCapture` for the log test (pattern at lines 581-608). Tests, all plain `#[test]` unit tests:
   - `remaining_pods_xml_is_the_frozen_bytes`: an independent `const EXPECTED: &str` (concat!-free, one raw string copied again from F1, not a reference to `REMAINING_PODS_XML`); assert `REMAINING_PODS_XML == EXPECTED`, `REMAINING_PODS_XML.len() == 1678`, exactly one `\n`, no `\r`.
   - `remaining_pods_ids_are_absent_from_the_stock_pak`: `ResourceCache::load_raw_mission_category_for_test(DIR)`; for 62050 assert `raw.elements` has no key; for each of `62050..=62056` assert no stock element's XML contains `ID="<id>"` (the pak has 1040 entries; a byte search is fine).
   - `production_cache_serves_remaining_pods`: `served = ResourceCache::load_all(DIR)`; `served.get(3, 62050)` equals `REMAINING_PODS_XML.as_bytes()`; the category-3 element count is the raw count + 1; `served.category(3).unwrap().metadata` differs from `ResourceCache::load_all_with_mission_additions(DIR, &[])`'s category-3 metadata (the bump happened); every other `CATEGORY_PAKS` category is byte-identical between the two caches.
   - `production_load_logs_the_insert`: inside a `LogCapture`, call `load_all(DIR)`; exactly one captured event has `event = cooked_data.mission_catalog_inserted` with `mission_id` 62050.
   - `mission_seed_rows_match_the_catalog`: read the four seed files (`concat!(env!("CARGO_MANIFEST_DIR"), "/../../db/resources/Missions/Seed/<file>")`) with `std::fs::read_to_string`, normalize `\r\n` to `\n`, and assert each F2 `INSERT` line is present exactly once. Then assert the seed strings agree with the XML: `REMAINING_PODS_XML` contains `MissionDefn="The Remaining Pods"`, each step text, each objective text and the history text verbatim.
5. Lanes, in order: `pwsh tools/build-lane/lane.ps1 cargo fmt --all -- --check`; `pwsh tools/build-lane/lane.ps1 cargo clippy -p cimmeria-resources -p cimmeria-base-session --all-targets -- -D warnings`; `pwsh tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-resources -p cimmeria-base-session`. The base-session run matters: its cooked-sync tests use the production cache, which now carries 62050.
6. Revert proof: set `MISSION_ADDITIONS` back to `&[]`; `production_cache_serves_remaining_pods` and `fixture_is_exact_stock_transform_and_ids_are_test_only` must fail. Restore.

**Docs owed.** None in this packet (W3-INT and CB-N05-3 own the docs).

## CB-N05-2 — Dialog 60150 override and seeds

| Field | Value |
|---|---|
| Agent | `packet-coder` |
| Size | About 25k working tokens |
| Branch / worktree | `cellblock/cb-n05-2-dialog` / `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\cellblock-cb-n05-2` |
| Writes | `crates/resources/src/base/dialog_overrides/overrides_cellblock.rs` (new); `crates/resources/src/base/dialog_overrides/mod.rs` (at most 6 added lines); `crates/resources/src/base/dialog_overrides/override_seed_agreement_cellblock_62050.rs` (new); `db/resources/Dialogs/Seed/dialogs.sql`, `db/resources/Dialogs/Seed/dialog_screens.sql` (append) |
| Commit | `feat(cellblock): serve the Remaining Pods report dialog` |
| Report | `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\cellblock-cb-n05-2\.claude\reports\cellblock-continuation\CB-N05-2-coder.md` |

**Why.** Dialog 60150 is new, so the client has no cooked entry for it; the server must push one through `DIALOG_OVERRIDES` (category 5). Authored dialogs crashed the client on 2026-09-27 and are quarantined; 60150 uses only the shape 3995 and 3996 already serve without trouble: one screen, speaker 0, zero buttons, UI type 2.

**Steps.**

1. Create `overrides_cellblock.rs` and edit `mod.rs` exactly as [w3-frozen.md § F3](w3-frozen.md#f3-dialog-60150-cb-n05-2) says (module line, test-module pair, the appended array element). Do not reorder `DIALOG_OVERRIDES` and do not touch `QUARANTINED_*` lists.
2. Append the two F3 seed blocks (after the 60160 rows). Check rule 6's `diff --stat`.
3. New test module `override_seed_agreement_cellblock_62050.rs` (template: `override_seed_agreement_debug_hub.rs`, its `read_seed` helper and imports). `#[test]`s:
   - `remaining_pods_dialog_emits_the_frozen_bytes`: `generate_dialog_xml(&overrides_cellblock::REMAINING_PODS_DIALOG)` (or find it in `DIALOG_OVERRIDES` by id) equals an independent `const EXPECTED: &[u8]` copied from F3; length 305.
   - `remaining_pods_dialog_keeps_the_served_shape`: exactly one `DIALOG_OVERRIDES` entry has id 60150; it has one screen, `speaker_id` 0, no buttons, `ui_screen_type` 2, `dialog_flags` 0; 60150 is not in any quarantine list.
   - `remaining_pods_dialog_matches_its_seed`: the `dialogs.sql` and `dialog_screens.sql` F3 rows are present exactly once (normalize `\r\n`), and the screen text in the seed equals the override's text.
   - `remaining_pods_dialog_is_new_to_the_client`: the committed stock dialog pak has no `_60150` member (copy `cooked_entry` from `patch_seed_agreement_cellblock.rs` line 170 and assert it returns `None`).
4. Existing tests that must still pass unchanged: `shipped_overrides_carry_no_buttons`, `quarantined_dialogs_are_not_served`, `all_overrides_emit_their_ids_and_text`, `every_cooked_override_element_id_fits_in_16_bits` (60150 < 65535). If any fails, report `blocked`; do not edit them.
5. Lanes: `cargo fmt --all -- --check`; `cargo clippy -p cimmeria-resources --all-targets -- -D warnings`; `cargo nextest run -p cimmeria-resources` (all through the lane).
6. Revert proof: remove the array element; `remaining_pods_dialog_keeps_the_served_shape` must fail. Restore.

## CB-N05-3 — Chains 11000-11005 and resolve matrix

| Field | Value |
|---|---|
| Agent | `packet-coder` |
| Size | About 40k working tokens (table-driven tests keep it there) |
| Branch / worktree | `cellblock/cb-n05-3-chains` / `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\cellblock-cb-n05-3` |
| Writes | `db/resources/Content/Seed/castle_cellblock_chains.sql` (append); `docs/content/mission-chains.md` (one new section); `crates/cell-content/src/cell/content/chain_replay_tests/mod.rs` (one line); `crates/cell-content/src/cell/content/chain_replay_tests/mission_62050/mod.rs` (new); `.../mission_62050/resolve.rs` (new) |
| Commit | `feat(cellblock): add Remaining Pods chains` |
| Report | `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\cellblock-cb-n05-3\.claude\reports\cellblock-continuation\CB-N05-3-coder.md` |

**Steps.**

1. Append the [w3-frozen.md § F4](w3-frozen.md#f4-chains-11000-11005-cb-n05-3) SQL block to the end of `castle_cellblock_chains.sql` (CRLF file; one blank line before the banner). Do not touch the header (rule 7).
2. `docs/content/mission-chains.md`: insert a section `#### Mission 62050: "The Remaining Pods" (CB-N05, CIMMERIA_AUTHORING)` directly before `### Castle (Persistent Zone)` (after the `#### Cellblock ambient cast (CB-N04a)` section). Content: two sentences of purpose (optional one-time story after 638, no reward), then the F4 "Expected resolved actions" table with a Conditions column (copy the gates from the SQL), then one line: "Marsh's core chains 1051/1052 still fire beside 11005; 1152 is unchanged. Seal: `docs/analysis/cellblock-continuation/worknotes/cb-n05-seal.md`." CRLF (rule 6).
3. `chain_replay_tests/mod.rs`: add `mod mission_62050;` in alphabetical order. `mission_62050/mod.rs`: a module doc comment (what the directory proves), `mod resolve;` and shared helpers used by `resolve.rs` (and later by CB-N05-7's `executor.rs`): `pub(super) const CHAINS: [i32; 6] = [11000, ..., 11005];`, `pub(super) const CELLBLOCK_WORLD_ID: i32 = 12;`, and `pub(super) fn ctx(world: Option<i32>, params: &[(&str, &str)], archetype: Option<i64>) -> ExecutionContext` that sets `ctx.world_id = world`, each string param, and `archetype` as an i64 param. Keep `mod.rs` under 120 lines.
4. `resolve.rs` (under 500 lines), all `#[tokio::test]`, names starting `live_db_mission_62050_`, each opening with `let pool = require_db_or_skip!();`. Resolve with the full engine `build_engine(Some(&pool)).await` (so stock chains are present), event shape as `cellblock_ambience.rs` lines 95-108 (`TriggerEvent { trigger_type, source_entity: None, target_entity: None, params: ctx.params.clone() }`, then `engine.resolve_event(&event, &ctx)`), and keep only the `(chain_id, action)` pairs whose chain is in `CHAINS` unless a test says otherwise. Tests:
   - `..._rows_are_exact`: `sqlx::query_as` over `content_triggers` (`chain_id, event_type, event_key, scope, once`), `content_conditions` (`chain_id, condition_type, target_id, target_key, operator, value, sort_order`) and `content_actions` (`chain_id, action_type, target_id, target_key, params::text, delay_ms, sort_order`) for `chain_id BETWEEN 11000 AND 11005 ORDER BY chain_id, sort_order`; each equals a literal `Vec` copied from F4 (6 triggers, 21 conditions, 8 actions). This pins that no chain has a death, dialog or region trigger.
   - `..._rows_load_as_sealed`: for each chain, `load_single_chain_for_test(&pool, id)` is `Some`; `engine.get_chain_actions(id).len()` is 1, 1, 1, 1, 3, 1; `engine.is_once(11001)` true and false for the other five. Use `assert_no_deferred_actions` from `chain_replay_tests/mod.rs` for each chain (no delays).
   - `..._offer_gates`: table-driven over the three offer triggers (11000 `MissionCompleted` `mission_id` 638; 11001 `PlayerLoaded` `world_name` `Castle_CellBlock`; 11002 `MissionAbandoned` `mission_id` 62050). Positive: world 12, `mission_638_status` = `completed`, 641 and 62050 unset; exactly the F4 `AddDialogSet { dialog_set_id: 2795, slot: 17, mission_id: None }` from that chain. Refusals (each: zero 62050-chain actions): world `None`; world 13; 638 `active`; 641 `active`; 641 `completed`; 62050 `active`; 62050 `completed`; wrong key (`mission_id` 637 for 11000 and 11002, `world_name` `Castle` for 11001).
   - `..._accept_gate`: `InteractTag`, `entity_tag` `Prisoner_329`, same positive state: exactly `[(11003, AcceptMission { mission_id: 62050 })]` among the CB-N05 chains. Same refusal list as the offer gates, plus `entity_tag` `329_CellDoorButton`.
   - `..._accepted_follow_up`: `MissionAccepted` `mission_id` 62050 with world 12 and `mission_62050_step_62051_status` = `active`: exactly the three F4 actions of 11004 in order. Refusals: world `None`; step 62051 `completed`; step 62051 unset; `mission_id` 641 (and assert chain 1152's `DisplayDialog { 4000 }` still resolves for 641, unfiltered).
   - `..._marsh_completes_b`: `InteractTag` `Preparation_ColMarsh`, world 12, 640 `completed`, 641 `not_active`, step 62054 `active`, **unfiltered**: archetype 1 gives 11005's `CompleteObjective { 62050, 62055 }` **and** 1051's `DisplayDialog { 4001 }`; archetype 8 gives 11005's and 1052's `DisplayDialog { 5022 }`. Refusals for 11005 only: 640 `active`; step 62054 unset; step 62054 `completed`; world 13.
   - `..._no_dialog_path_moves_the_mission`: `DialogChoice` with params `dialog_id` 60150 and `button_id` -1 (then 0), and `DialogOpen` with `dialog_id` 60150 (param names as `event_dispatch/dialog.rs` lines 34 and 104-105 set them), each in the positive offer state and in the step-B state: zero actions from any `CHAINS` chain.
5. Lanes: `cargo fmt --all -- --check`; `cargo clippy -p cimmeria-cell-content --all-targets -- -D warnings`; `cargo nextest run -p cimmeria-cell-content` (non-DB part); `pwsh tools/build-lane/live-db-test.ps1 mission_62050` (all seven must run and pass, not skip; the log shows the count).
6. Revert proof: delete the chain 11005 condition row `(11005, 'mission_status', 640, ...)`, rerun `live-db-test.ps1 mission_62050`; the 640 `active` refusal must fail. Restore.

## W3a-INT — Integrate the data

| Field | Value |
|---|---|
| Agent | `packet-coder` |
| Size | About 15k working tokens |
| Worktree | `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\zone-cellblock` (`content/cellblock-campaign`) |
| Writes | nothing new: cherry-picks only |
| Report | `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\zone-cellblock\.claude\reports\cellblock-continuation\W3a-INT-coder.md` |

1. Confirm `git -C <zone-cellblock> status --porcelain` shows only the untracked `lab-server*.log` files (leave them; never add them).
2. `git -C <zone-cellblock> cherry-pick -x` each W3a commit in order CB-N05-1, CB-N05-2, CB-N05-3, then each fix commit after its packet. On a conflict: `git cherry-pick --abort` and report `blocked`.
3. Lanes from zone-cellblock: `cargo fmt --all -- --check`; `cargo clippy -p cimmeria-resources -p cimmeria-base-session -p cimmeria-cell-content --all-targets -- -D warnings`; `cargo nextest run -p cimmeria-resources -p cimmeria-base-session -p cimmeria-cell-content`; `pwsh tools/build-lane/live-db-test.ps1 mission_62050`; `pwsh tools/build-lane/live-db-test.ps1 chain_replay_tests`.
4. Return the tip SHA. It is the W3b base. No bookkeeping commit here.

## CB-N05-4 — Cooked-sync transfer proof

| Field | Value |
|---|---|
| Agent | `packet-coder` |
| Size | About 30k working tokens |
| Branch / worktree | `cellblock/cb-n05-4-transfer` / `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\cellblock-cb-n05-4` |
| Writes | `crates/base-session/src/base/cooked_sync/tests/resync_cellblock.rs` (new); `crates/base-session/src/base/cooked_sync/tests/mod.rs` (one line) |
| Commit | `test(cellblock): prove the Remaining Pods cooked-sync transfer` |
| Report | `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\cellblock-cb-n05-4\.claude\reports\cellblock-continuation\CB-N05-4-coder.md` |

**Why.** CB-N05-1/2 prove the server's cache holds the entries. This packet proves a stock client that asks for them receives exactly those bytes over the real cooked-sync path, mission 62050 in two fragments.

**Steps.** Register `mod resync_cellblock;` in `tests/mod.rs` (alphabetical, after `resync`). Template: `authored_mission_catalog_reaches_stock_client` in `tests/resync.rs` lines 122-213; reuse the shared helpers in `tests/mod.rs` (`committed_cache`, `server_version`, `rig(port)`, `decode`, `server_entries`, `ClientModel::holding`, `apply_all`) and `crate::mercury::{FRAG_FIRST, FRAG_FIRST_AND_LAST}`. Do **not** replace the rig's cache: the production `committed_cache()` already carries 62050 and 60150 after W3a-INT. Two `#[tokio::test]`s, ports **47_108** and **47_109** (unused elsewhere; grep `47_10` to confirm):

- `remaining_pods_mission_reaches_a_stock_client`: client category 3 = `ClientModel::holding(3, raw.metadata, raw.elements)` from `load_raw_mission_category_for_test`; the client requests (3, 7538) as in the template; pump until idle; exactly one fragment head `(3, 62050)`, with flags `FRAG_FIRST` (not first-and-last); reassemble every fragment carrying that data id; the bytes equal an independent copy of F1 (a `const EXPECTED: &str` in this file); after `apply_all`, the client holds element 62050 equal to the same bytes and its version equals the server's category-3 version.
- `remaining_pods_dialog_reaches_a_stock_client`: client category 5 = `server_entries(5)` minus 60150, at version `served + 7` (stale); exactly one head `(5, 60150)` with flags `FRAG_FIRST_AND_LAST`; body equals an independent copy of F3 (305 bytes); the client then holds it.

Lanes: `cargo fmt --all -- --check`; `cargo clippy -p cimmeria-base-session --all-targets -- -D warnings`; `cargo nextest run -p cimmeria-base-session`. Revert proof: point the first test's `EXPECTED` at a one-byte-changed literal; it must fail on the body comparison (proves the comparison bites). Restore. Record that the existing `resync.rs` tests still pass unchanged.

## CB-N05-5 — Interaction dispatch negatives

| Field | Value |
|---|---|
| Agent | `rust-gameserver-dev` |
| Size | About 40k working tokens |
| Branch / worktree | `cellblock/cb-n05-5-dispatch` / `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\cellblock-cb-n05-5` |
| Writes | `crates/cell-methods/src/cell/cell_methods/player/interaction/interact/mission_62050_tests.rs` (new); `crates/cell-methods/src/cell/cell_methods/player/interaction/interact.rs` (two lines: `#[cfg(test)]` and `mod mission_62050_tests;`) |
| Commit | `test(cellblock): pin Remaining Pods interaction gates` |
| Report | `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\cellblock-cb-n05-5\.claude\reports\cellblock-continuation\CB-N05-5-rust-gameserver-dev.md` |

**Why.** The accept must come only from a validated Prisoner 329 click: in range, same space, the real entity. A lying client must not accept or complete 62050 by forged clicks, dialog choices or a stale `initialResponse`.

**Rig** (copy the shape of `debug_hub_dispatch_tests.rs` lines 39-159; this crate has no `sqlx`, so load with a macro): `spawner::load_spawns_from_db`, `spawner::load_mission_defs`, `spawner::load_world_rows` through `crate::cell::spawner`; engine `crate::cell::content::build_engine(Some(&pool)).await`. Stage `Castle_CellBlock` (MinX -800..800), spawn only records tagged `Prisoner_329` and `Preparation_ColMarsh` with `allocate_npc_id` + `spawn_npc_from_record`, set `mgr.mission_defs`, **stamp the world rows** with `mgr.stamp_world_rows(&rows)` (without it `ctx.world_id` is `None` and every chain fails closed; precedent `cell-content/src/cell/content/mod.rs` lines 123-143), create the player (`is_player`, `player_id` 4242, `archetype_id` 1), `connect_entity`. Give the player 638 completed: `MissionInstance::new(638, <any 638 step id>, vec![])` then `.complete()`, added with `missions.add_mission(..)`. The observable is the player's mission list: `get_mission(62050)`.

**Tests** (`#[tokio::test]`, names `live_db_mission_62050_dispatch_*`):

1. `..._prisoner_click_in_range_accepts`: player 1 m from Prisoner 329, `dispatch(PLAYER, INTERACT, &(npc as i32).to_le_bytes(), ..)`; 62050 is active at step 62054 (the nested follow-up ran).
2. `..._far_click_refused`: player 6 m away (beyond `MAX_INTERACT_DISTANCE` 5.0): 62050 absent.
3. `..._wrong_and_missing_entity_refused`: click Marsh's id (no 62050 effect: absent), and click an unallocated id: absent.
4. `..._cross_space_refused`: a second space (another world name in the XML) holding a copy of the Prisoner record; the player in Castle_CellBlock clicks that id: absent.
5. `..._forged_dialog_choice_refused`: `DIALOG_BUTTON_CHOICE` with args (60150, -1) and (60150, 0) via `dialog_choice_args` from `dialog_choice_gate_tests.rs` line 45 (make a local copy if it is not reachable): 62050 absent; then after a real accept, the same choices leave 62050 at step 62054, objective 62055 not complete.
6. `..._stale_initial_response_refused`: `INITIAL_RESPONSE` with args `2795i32.to_le_bytes()` with and without the 2795 binding present in `available_interactions` slot 17: 62050 absent.
7. `..._marsh_click_far_refused`: with 62050 at step 62054 and 640 completed, a Marsh click 6 m away leaves the mission active; a click 1 m away completes it (`MISSION_COMPLETED`).

Lanes: `cargo fmt --all -- --check`; `cargo clippy -p cimmeria-cell-methods --all-targets -- -D warnings`; `cargo nextest run -p cimmeria-cell-methods` (non-DB); `pwsh tools/build-lane/live-db-test.ps1 mission_62050_dispatch`. Revert proof: drop the `stamp_world_rows` call; test 1 must fail (proves the world gate is live in the dispatch path). Restore. If a negative passes only because the positive also fails, the test is theatre: each negative must sit next to a passing positive in the same rig.

## CB-N05-6 — Persistence round trip

| Field | Value |
|---|---|
| Agent | `packet-coder` |
| Size | About 35k working tokens |
| Branch / worktree | `cellblock/cb-n05-6-persist` / `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\cellblock-cb-n05-6` |
| Writes | `crates/services/src/mission_62050_round_trip_tests.rs` (new, under 400 lines); `crates/services/src/lib.rs` (two lines after `mod mission_abandon_round_trip_tests;`) |
| Commit | `test(cellblock): persist Remaining Pods per character` |
| Report | `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\cellblock-cb-n05-6\.claude\reports\cellblock-continuation\CB-N05-6-coder.md` |

**Why.** A one-time mission must survive relog at each step, must not leak between two characters on one account, and must not be offered again after completion.

**Template:** `mission_abandon_round_trip_tests.rs` (read it fully; copy `cleanup`, `insert_account_and_player`, `forward`, the imports). Differences: use the real defs, `mgr.mission_defs = crate::cell::spawner::load_mission_defs(&pool).await.unwrap()`, instead of the hand-built `MissionDefEntry`; world `Castle_CellBlock` instead of `Agnos`. Sentinel IDs (grep shows none in use): test 1 account **0x7000_8F00** with players **0x7000_8F01** and **0x7000_8F02**; test 2 account and player **0x7000_8F03** (account) / **0x7000_8F04** (player). Each test cleans up its own rows first and last.

Drive the mission with the cell functions (`crate::cell::missions::{accept_mission, abandon_mission, send_mission_update}` and `advance_step`, `complete_objective` from the same module): accept 62050 at step 62051 with objective 62052 active; `advance_step(.., 62050, 62054, ..)`; `complete_objective(.., 62050, 62055, ..)`. After each step call `send_mission_update` and `forward` so the base writes `sgw_mission`, then read `query_saved_missions`.

1. `live_db_mission_62050_progress_survives_relog_per_character`: player A: after accept, saved row = 62050 active at 62051; after advance, at 62054; after completion, status completed, `repeats` 1. Player B on the same account has **no** 62050 row at every point. A fresh manager rebuilt from player A's saved rows (as the template's relog does) shows 62050 completed, and `accept_mission(.., 62050, ..)` on it returns `false` (no repeat: `num_repeats` 0).
2. `live_db_mission_62050_abandon_then_reaccept`: accept, advance to 62054, `abandon_mission`; the saved row is not active (as the template asserts for abandon); re-accept succeeds and starts at 62051 again.

Lanes: `cargo fmt --all -- --check`; `cargo clippy -p cimmeria-services --all-targets -- -D warnings`; `pwsh tools/build-lane/live-db-test.ps1 mission_62050_`. Revert proof: remove the 62050 rows from `mission_steps.sql` in the worktree (do not commit), `live-db-test.ps1` must fail on the defs load or the advance; restore with `git -C <wt> checkout -- db/resources/Missions/Seed/mission_steps.sql`.

## CB-N05-7 — Executor replay

| Field | Value |
|---|---|
| Agent | `rust-gameserver-dev` |
| Size | About 40k working tokens |
| Branch / worktree | `cellblock/cb-n05-7-replay` / `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\cellblock-cb-n05-7` |
| Writes | `crates/cell-content/src/cell/content/chain_replay_tests/mission_62050/executor.rs` (new, under 500 lines); `.../mission_62050/mod.rs` (one line: `mod executor;`) |
| Commit | `test(cellblock): replay Remaining Pods through the executor` |
| Report | `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\cellblock-cb-n05-7\.claude\reports\cellblock-continuation\CB-N05-7-rust-gameserver-dev.md` |

**Why.** CB-N05-3 proves each chain resolves; this proves the executed story: the nested accept follow-up, the per-player binding, the monologue wire bytes, the Marsh completion beside the core chains, and the replays (abandon, relog, duplicate, skip).

**Rig:** copy `mission_681_686_backstop.rs` lines 20-121 (imports, `Rig`, `rig()`, `accept()`), with these changes: spawn tags `Prisoner_329` and `Preparation_ColMarsh`; `mgr.stamp_world_rows(&crate::cell::spawner::load_world_rows(&pool).await.expect("world rows"))`; `mgr.dialog_set_maps` from `load_dialog_set_maps` and the monologue ids from `load_monologue_dialog_ids` (cell-catalog `dialogs.rs` lines 48 and 108; follow how the cell's startup assigns them); player `archetype_id` set per test (1 or 8). Fire events with the real dispatchers: `fire_interact_tag(PLAYER, PLAYER_ID, tag, npc_id, &engine, &tx, &mut mgr)`, `fire_player_loaded(PLAYER, PLAYER_ID, "Castle_CellBlock", ..)`, and `execute_actions` with `Action::CompleteMission { mission_id: 638 }` (the 11000 offer) or `Action::AbandonMission { mission_id: 62050 }` (the 11002 restore). Binding observable: `mgr.get_entity(PLAYER).unwrap().available_interactions.get(&17)` equals `Some(&vec![(2795, None, 1_073_741_824i64)])` (precedent `mission_701/body.rs` lines 537-547). Dialog observable: drain `rx` for `CellToBaseMsg::EntityMethodCall { method_index: 105, .. }` to the player; args bytes 0..4 = the player's entity id (speaker-0 monologue), 4..8 = 60150.

**Tests** (`#[tokio::test]`, names `live_db_mission_62050_replay_*`):

1. `..._offer_accept_and_report`: 638 active at its last step, then `CompleteMission 638`: slot 17 holds the 2795 binding. Prisoner click: 62050 active at **62054** (not 62051), slot 17 no longer holds 2795, exactly one `onDialogDisplay` for 60150 with the player as speaker.
2. `..._marsh_completes_beside_core` (run for archetype 1 and 8): from step 62054 with 640 completed and 641 not active, a Marsh click completes 62050 and sends exactly one `onDialogDisplay` 4001 (archetype 1) or 5022 (archetype 8). Then accepting 641 through `Rig::accept(641)` still shows 4000 (chain 1152 intact).
3. `..._duplicate_and_skip`: a second Prisoner click while 62050 is active changes nothing (no second accept, no second 60150). A Marsh click while 62050 is at 62051 is impossible by construction; instead stage 62050 at 62051 by hand and click Marsh: no completion. A player who never takes 62050 and accepts 641: no 2795 binding remains after the next `player_loaded` (641 is no longer `not_active`).
4. `..._abandon_restores_completion_does_not`: after accept, `AbandonMission 62050`: the binding returns (11002). Re-accept, complete via Marsh, then fire `player_loaded`: no binding, and a Prisoner click does not re-accept.
5. `..._relog_restores_offer_once`: a fresh rig with 638 completed: `fire_player_loaded` binds 2795; a second `fire_player_loaded` on the same rig adds no duplicate (`once`). A fresh rig whose world rows are **not** stamped binds nothing (world gate fails closed).

Lanes: `cargo fmt --all -- --check`; `cargo clippy -p cimmeria-cell-content --all-targets -- -D warnings`; `pwsh tools/build-lane/live-db-test.ps1 mission_62050`. Revert proof: in the worktree only, delete chain 11004's `remove_dialog_set` row, rerun; test 1 must fail on the slot-17 assertion. Restore with `git -C <wt> checkout -- db/resources/Content/Seed/castle_cellblock_chains.sql`.

## W3-INT — Integrate wave 3 and bookkeeping

| Field | Value |
|---|---|
| Agent | `packet-coder` |
| Size | About 30k working tokens |
| Worktree | `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\zone-cellblock` |
| Writes | cherry-picks, then one commit touching `db/resources/Content/Seed/castle_cellblock_chains.sql` (header line only), `docs/analysis/four-zone-planning/id-allocation.md`, `docs/analysis/cellblock-continuation/README.md`, `docs/analysis/cellblock-continuation/worknotes/cb-n05.md` (new) |
| Commit | `docs(cellblock): W3 integrate CB-N05` |
| Report | `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\zone-cellblock\.claude\reports\cellblock-continuation\W3-INT-coder.md` |

1. Cherry-pick (`-x`) CB-N05-4, -5, -6, -7 and their fix commits, as in W3a-INT step 2.
2. Sibling recheck (record the output in the report): `git -C <zone-cellblock> grep -n "mission_id: " origin/main -- crates/resources/src/base/mission_additions.rs` and the same for the local branches `content/castle-campaign`, `content/harset-campaign`, `content/tollana-campaign` if they exist (`git branch --list`). Any production `MissionAddition` other than 62050, or any reuse of 62050-62056, 60150, 96110 or 11000-11005: stop, report `blocked`.
3. Bookkeeping commit:
   - Chain seed header: add one line directly before the line starting `--   (next free inside 1001-1111:`: `--   Cellblock authored story CB-N05 (2026-10-17): chains 11000-11005, mission 62050 "The Remaining Pods" (id-allocation.md, CB-N05 individual claims).` (CRLF file; `diff --stat` must show 1 line.)
   - `id-allocation.md` § CB-N05 individual claims: add a `Status` column to the table, every row `In use (W3-INT)`; replace "The mission seal is still under review; no coder may use the claims until that review passes." with "The seal passed review; W3-INT (2026-10-17) moved every claim to in use." Add `, and 11000–11005` to the `castle_cellblock_chains.sql` row of the "Chain IDs in use today" table (after `11010–11010`, keeping its style). Update **Last updated**.
   - Branch `README.md`: row CB-N05 -> `Integrated (W3-INT)`, CB-U10 -> `UATPending (owner)`; nothing else.
   - `worknotes/cb-n05.md` (new, under 80 lines): commits integrated, test names per packet, revert proofs recorded by each packet, what is proven statically versus what only CB-U10 can prove (client renders the journal and 60150, no crash, first-press feedback, relog in the real client).
   - CRLF for all docs (rule 6).
4. Lanes from zone-cellblock: `cargo fmt --all -- --check`; `cargo clippy -p cimmeria-resources -p cimmeria-base-session -p cimmeria-cell-content -p cimmeria-cell-methods -p cimmeria-services --all-targets -- -D warnings`; `cargo nextest run` for the same five crates; `pwsh tools/build-lane/live-db-test.ps1 mission_62050`; `pwsh tools/build-lane/live-db-test.ps1 chain_replay_tests`.

## Held: CB-N05b and CB-N06

| Packet | Status | Blocked on | Unblock |
|---|---|---|---|
| CB-N05b (Castle evidence hand-in, item-bearing; not step B of 62050) | BlockedDependency | Harset HA-1360's atomic possession, consume and mission-completion primitive is not on `main` and not on this branch; the Castle recipient owner and the mission, items and authority reviewers must then reseal item, recipient, copy, reward and IDs | A later architect pass after HA-1360 merges |
| CB-N06 (second authored quest) | BlockedEvidence + BlockedDecision | CB-U09 (pacing headroom) and CB-U10 (62050 renders safely in the real client); owner decision D-CC15 (whether a second quest exists, which one, which prop) | Owner runs CB-U09/U10, then answers D-CC15; then an architect pass seals CB-N06s |

## Owner lab rows for this wave

CB-U09 and CB-U10 are the owner's, after W3-INT, with steps in [owner-lab-checklist.md](owner-lab-checklist.md). An observed blank dialog, client crash, missing first-press feedback, lost progress or broken core briefing in CB-U10 blocks the campaign PR (seal rule); it is not "UATPending". Rollback if 60150 crashes the client: move 60150 into the dialog quarantine list in a fix packet and delete `Cache.en-US\CookedDataDialogs.pak` on the client (D-CC13).
