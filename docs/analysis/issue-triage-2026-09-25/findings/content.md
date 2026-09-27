# Triage findings — batch `content`

Code of record: `main-ro` detached at origin/main 059d6038 (2026-09-25). All file:line
citations are against that tree unless marked otherwise. Research only — nothing was
posted, labelled, or committed.

## #269 — Content engine small hookups: RemoveItem wire, StartMinigame difficulty, clip-sizes from DB

- Verdict: CLOSE (completed)
- Priority: P3
- Labels: no change (closing)
- Summary: The issue lists three executor stubs. All three have landed. `Action::RemoveItem` sends a real base-side removal: by inventory instance when the chain was fired by `useItem`, otherwise by item type. `StartMinigame` has a `difficulty` field that is range-checked 1–5 at load time and forwarded to base (#652). Weapon clip sizes come from the `resources.items` cache (`space_mgr.item_defs`, loaded by `spawner::load_item_defs`). The hardcoded 55/21 `match` no longer exists.
- Evidence:
  - `crates/services/src/cell/content/executor/mod.rs:264-266` → `inventory::remove`; `crates/services/src/cell/content/executor/inventory.rs:149-215` (`CellToBaseMsg::RemoveInventoryItem` by instance / `RemoveInventoryItemByType` fallback).
  - `crates/services/src/cell/content/executor/mod.rs:299-327` (`StartMinigame { minigame_type, difficulty, on_victory_chains }` → `CellToBaseMsg::StartMinigame { difficulty, .. }`); guard `crates/services/src/cell/content/chain_replay_tests/start_minigame_difficulty.rs`; PR #652 (merged 2026-09-18).
  - `crates/services/src/cell/content/executor/inventory.rs:20-37` (`weapon_stats` reads `space_mgr.item_defs`, "loaded at startup from `resources.items`"); `crates/services/src/cell/spawner/loot.rs:173` (`load_item_defs`). No per-item-id `match` remains under `cell/content/executor/`.
  - RemoveItem/OnItemUse pairing linter: PR #731 (`crates/content-engine/tests/onitemuse_remove_item_pairing.rs`).
- Related/duplicates: #265 (umbrella), #332

### Action text

Closing as completed. All three hookups are on main:

1. **`RemoveItem`**: `executor/inventory.rs::remove` sends `RemoveInventoryItem` with the clicked instance id when `useItem` fired the chain. Otherwise it falls back to `RemoveInventoryItemByType` (see #113 and #371). PR #731 added the OnItemUse/RemoveItem pairing linter.
2. **`StartMinigame` difficulty**: the action carries `difficulty`, the loader range-checks it to 1–5, and it is forwarded on `CellToBaseMsg::StartMinigame` (#652). `chain_replay_tests/start_minigame_difficulty.rs` guards it.
3. **Clip sizes**: `weapon_stats` reads `space_mgr.item_defs`, which is loaded from `resources.items` (`clip_size > 0`). The hardcoded 55/21 match is gone.

## #242 — Seed cleanup: chains 5020 and 5021 are byte-identical duplicates

- Verdict: CLOSE (completed)
- Priority: P3
- Labels: no change (closing)
- Summary: Castle rebuild packet C01 (#646) deleted the whole auto-exported `space_castle_cellblock_chains.sql`, which held chains 5000–5029 and so both duplicate chains 5020 and 5021. The one behaviour worth keeping was re-authored as curated chain 1008. No `content_chains.chain_id` 5020 or 5021 exists anywhere in `db/resources/Content/Seed/`. The numbers 5020 and 5021 still appear there, but only as dialog ids. The duplicate-chain linter the issue proposed was never added. Nothing in the issue's reported symptom remains.
- Evidence:
  - `db/resources/Content/Seed/space_castle_cellblock_chains.sql` no longer exists. Commit 73a99e6b (PR #646, 2026-09-17): "C01: delete the auto-exported space_castle_cellblock_chains.sql (chains 5000-5029), which duplicated curated chains…".
  - `grep '\b502[01]\b' db/resources/Content/Seed/` matches only dialog ids (for example `castle_cellblock_chains.sql:577,634,640`), never a chain id.
  - `crates/content-engine/tests/` has `interact_tag_linter.rs`, `dialog_button_linter.rs` and `onitemuse_remove_item_pairing.rs`, but no duplicate-chain linter.
- Related/duplicates: #268 (system_message is still a stub, so duplicate messages would never have been visible anyway)

### Action text

Closing as completed. Castle rebuild packet C01 (PR #646, commit 73a99e6b) deleted the auto-exported `space_castle_cellblock_chains.sql`, including both duplicate chains 5020 and 5021. The one unique behaviour in that file, the Region8 aggro trap, was re-authored as curated chain 1008. No chain 5020 or 5021 exists in `db/resources/Content/Seed/` any more; those numbers only appear there as dialog ids. The proposed duplicate-chain linter was not added. If we want it as a general seed guard, it should be a new hygiene ticket rather than a reason to keep this one open.

## #334 — fix(seed): add items_event_sets entry for Radio (item 5168) — unblocks SGC_W1 mission 1561

- Verdict: REWRITE
- Priority: P2
- Labels: add `needs-triage`. Keep `bug`.
- Summary: The issue's central claim is that "the SGW client treats `items_event_sets` as the authoritative can-this-item-be-used gate". That claim is wrong. The client never sees our DB table. It reads `<ItemEventSet>` from its own `CookedDataItems.pak`, and the server's `useItem` path never consults `items_event_sets`. The client's cooked `_5168` entry is `<ItemEventSet AbilityID="0" EventID="5">`, while working quest items such as 1937 "Banged-up Radio", 2819 and 1893 carry `AbilityID="597"`. Our seed faithfully mirrors the PAK, which has no ability for 5168. Open PR #605, which only adds the DB row, will therefore not change what the client shows. Any real fix needs a cooked-item override that rewrites `_5168`'s `ItemEventSet AbilityID` to 597 through the existing per-key invalidation path. `base/item_overrides.rs` handles only icon and stack size today, so it would need extending. The DB row would ship alongside for consistency. Whether `AbilityID=0` actually hides "Use" still needs a UAT or client-side check: 507 cooked items have `EventID=5` with `AbilityID=0`.
- Evidence:
  - Client PAK (local QA client, `Stargate Worlds-QA/Working/SGWGame/SourceCache.en-us/CookedDataItems.pak`, entry `_5168`): `<ItemEventSet AbilityID="0" EventID="5">`. Entries `_1937`, `_1893`, `_2819` and `_2133` all have `AbilityID="597" EventID="5"`. Across the whole PAK: 507 EventID=5 sets with AbilityID 0, and 311 non-zero.
  - `crates/services/src/cell/cell_methods/inventory/item_ops.rs:140-176`: `handle_use_item` forwards `UseInventoryItem` to base with no `items_event_sets` check. `items_event_sets` is only read for weapon ability resolution (`cell/spawner/abilities.rs` `load_item_event_set_abilities`, `cell/abilities/resolve.rs`).
  - `crates/services/src/base/item_overrides.rs:1-60`: the cooked-item override mechanism patches `IconLocation` and `MaxStackSize` only. `docs/engine/cooked-data-pak-format.md:140-170` shows `ItemEventSet` living in the cooked XML.
  - `db/resources/Items/Seed/items_event_sets.sql` has no row for 5168. Chains 3020, 3021 and 3025 are in `db/resources/Content/Seed/sgc_w1_chains.sql:302-376`.
  - PR #605 (open, MERGEABLE, last updated 2026-07-06) adds only the DB row and a loader test. Its own body concedes "this row is specifically the client-side usability gate", which the PAK evidence contradicts.
- Related/duplicates: PR #605, #335 (1561 is the entry gate for the SGC_W1 chain port)

### Action text

Comment:

> Re-verified against main (059d6038) and the client's cooked data. The premise needs correcting: the client never reads `resources.items_event_sets`. Its Use affordance comes from the `<ItemEventSet>` in `CookedDataItems.pak`, and the client's `_5168` entry is `AbilityID="0" EventID="5"`. Working quest items (1937 Banged-up Radio, 1893, 2819, 2133) are `AbilityID="597"`. Our seed already matches the PAK, so adding the DB row alone (PR #605) changes nothing the client sees. The server-authoritative fix is a cooked-item override for `_5168`, using the same per-key invalidation path as `base/item_overrides.rs`, extended to rewrite `ItemEventSet AbilityID`. The DB row ships with it for consistency. The body is rewritten below. Suggest closing or reworking #605 once this is agreed.

#### New body

**Problem.** SGC_W1 mission 1561 (bomb defusal) needs the player to *use* the Radio (item 5168) twice. Chain 3021 fires `item_use 5168` at step 4621 to advance to bomb defusal. Chain 3025 fires `item_use 5168` at step 4623 to complete the mission (`db/resources/Content/Seed/sgc_w1_chains.sql:318-380`). If the client never offers Use on the Radio, the mission softlocks after chain 3020 grants it.

**Evidence.**

- The client's Use affordance comes from its cooked item catalogue, not from any server table. The client's `CookedDataItems.pak` entry `_5168` is `<ItemEventSet AbilityID="0" EventID="5">`. Working quest-use items carry `AbilityID="597" EventID="5"`: `_1937` "Banged-up Radio", `_1893`, `_2819` Jaffa Disguise, `_2133` Med Kit. 507 cooked items have `EventID=5` with ability 0.
- The server does not gate `useItem` on `items_event_sets` (`crates/services/src/cell/cell_methods/inventory/item_ops.rs:140-176`). That table only drives weapon ability resolution.
- `db/resources/Items/Seed/items_event_sets.sql` has no 5168 row, which matches the PAK.
- `crates/services/src/base/item_overrides.rs` already patches cooked item XML at startup and pushes it through `versionInfoRequest` → `onVersionInfo(InvalidKeys)` → `resourceFragment`. Today it only rewrites `IconLocation` and `MaxStackSize`.

**Unknown.** Whether the client actually hides Use, or suppresses the `useItem` send, when `AbilityID=0`. Settle this first, either with an in-game check (pick up the radio in SGC_W1 and right-click it) or with a client-side finding.

**Acceptance criteria.**

- If the radio is not usable in-game: extend `ItemOverride` with an optional `ItemEventSet` ability rewrite. Add a `_5168` override setting `AbilityID="597"` for EventID 5. Add the matching `items_event_sets` seed row (597, 5), and keep `setval` in step.
- If the radio is already usable (ability 0 does not gate Use): close this with that finding. No seed change is needed.
- Chains 3021 and 3025 fire in-game and mission 1561 completes.

**Test type.** Unit test on the override patcher: the rewritten `_5168` XML carries `AbilityID="597"` and the guard fails when the override entry is removed. Add a live-DB loader guard for the seed row if one is added. Chain-replay tests for 3021/3025 cannot guard this, because those chains exist either way.

**Docs to update.** `docs/content/consumable-via-onitemuse-pattern.md` (the radio section), and a note in `docs/engine/cooked-data-pak-format.md` that `ItemEventSet` can be overridden.

**Client impact.** Free: a server-pushed cooked-data override, no client patch.

**Domain advisor.** `items-systems-advisor`.

**Needs a human for.** The in-game check of whether ability-0 items show Use.

## #335 — epic(content): port SGC_W1 missions 1562-1568 + space script to content-engine chains

- Verdict: KEEP
- Priority: P2
- Labels: no change
- Summary: Nothing has moved since the epic was filed. `sgc_w1_chains.sql` still holds exactly chains 3001–3028: mission 1559 fully chained, 1561 fully chained, and 1562 covering only accept, relog-restore and the elevator. No chains exist for 1563–1568, and no SGC_W1 PR has merged since. Two details in the body are stale. The chain range is 3001–3028, not "3001-3050". The "#334 Radio bridge" blocker has a corrected premise: see #334, where the fix is a cooked-item override, not a DB row. Two facts the body omits: the `Hack` minigame that 1563/1564 depend on is still `PlaceholderGame`, which wins instantly on a `victory` message, and the in-repo handoff pack now carries an SGU/SGC starter design doc that sub-PRs can use as a secondary reference. Owner decision D6 keeps SGU characters starting in `SGC_W1`, so this remains that faction's entire starter experience.
- Evidence:
  - `db/resources/Content/Seed/sgc_w1_chains.sql`: `INSERT INTO content_chains` rows are 3001–3028 only. Chains 3026–3028 are the 1562 accept (dialog 5365), the 4624 relog restore, and the ElevatorButton2 hop.
  - `db/resources/Missions/Seed/missions.sql:621-633`: missions 1562–1568 are seeded.
  - `crates/services/src/minigame/games/mod.rs`: `"Hack" | "Activate" | …` → `placeholder::PlaceholderGame` (auto-victory; `games/placeholder.rs`).
  - `docs/content/mission-chains.md:1141-1145` still says "Total scripted missions: 3 (1559, 1561, 1562)".
  - `docs/analysis/sgw-handoff-pack-v1.2/phase0-gap-report.md:150,191` (SGU → `SGC_W1` 58; decision D6 "Keep"); `docs/analysis/sgw-handoff-pack-v1.2/pack/references/world_content/SGW_Earth_SGC_SGU_Human_Starter_Dev_Master_v1.md`.
  - `deprecated/python/cell/missions/SGC_W1/` contains only `SecurityOffice.py` (1561/1562). The 1563–1568 references are external (fanmmorpg).
- Related/duplicates: #334 (1561 radio usability), #66 (minigame framework: a real `Hack` implementation is out of scope here)

### Action text

Status comment:

> Triage 2026-09-25 against main 059d6038: still open and still accurate in substance. `sgc_w1_chains.sql` has chains 3001–3028: 1559 and 1561 are complete, and 1562 has only its accept, relog-restore and elevator chains. There is nothing for 1563–1568 or the space script. Corrections and additions:
>
> 1. The chain range is 3001–3028, not 3001–3050.
> 2. The 1561 blocker (#334) has a corrected premise. The Radio's Use affordance comes from the client's cooked `ItemEventSet` (`_5168` has `AbilityID=0`), not from `items_event_sets`, so the fix is a cooked-item override. See #334.
> 3. 1563 and 1564 use the `Hack` minigame. That game is still `PlaceholderGame`, which wins instantly on `victory` (`minigame/games/mod.rs`). Those sub-PRs can chain `start_minigame` → `on_victory_chains` today. A real Hack game is #66 territory.
> 4. `docs/analysis/sgw-handoff-pack-v1.2/pack/references/world_content/SGW_Earth_SGC_SGU_Human_Starter_Dev_Master_v1.md` is an extra in-repo reference for each sub-PR, with the usual caveat that the pack is a guide, not authority.
> Owner decision D6 keeps SGU characters starting in `SGC_W1`, so this epic is the SGU starter experience.

## #310 — T1-12: Implement mission reward dispatch (xp/cash/items)

- Verdict: REWRITE
- Priority: P2
- Labels: add `needs-info`. The XP formula is an owner/design decision; see the question below.
- Summary: The core claim, that mission rewards never dispatch, is still true. `chosenRewards` (cell method 87) is still `tracing::info!("UNIMPLEMENTED: chosenRewards")`. No code sends `onMissionRewardsDisplay` (client method 127), and `crates/game/src/missions/rewards.rs:52` is still a dead `todo!()`. Two assumptions in the body and in the long maintainer comment are wrong.
  - (1) The comment says ~99% of missions would be served by an "empty-rewards short-circuit" granting `missions.reward_xp`/`reward_naq`. In fact every one of the 1,041 seeded missions has `reward_xp = 0` and `reward_naq = 0`, while 1,018 have `award_xp = true`. So the original server computed XP from a formula, and that formula is unknown. Castle rebuild decision D-CB10 and packet GC3 already record this as BlockedDesign ("observed 52 XP… exclude a hardcoded 52").
  - (2) The comment calls the `Rewards`/`RewardChoices` payload "a Python-serialized blob, the load-bearing unknown". It is not. Both are plain FIXED_DICTs in `entities/defs/alias.xml:461-496` (`Rewards{XP:UINT32, Naquadah:UINT32, ItemGroups:ARRAY<ItemGroup{GroupId,NumChoices,Items:ARRAY<RewardItem{ItemId,Index}>}>}`, `RewardChoices{GroupChoices:ARRAY<GroupChoice{GroupId,Choices:ARRAY<UINT32>}>}`), and `RewardChoices` is already documented in `docs/reverse-engineering/findings/mission-wire-formats.md:50-68`.
  - The `CellToBaseMsg` grant plumbing exists, and `Action::GrantXP` now has an executor arm.
  - Only missions 40 and 1001 have item reward groups (3 groups, 8 items).
- Evidence:
  - `crates/services/src/cell/cell_methods/player/world/mod.rs:136-139` (the `CHOSEN_REWARDS` stub); `crates/services/src/cell/client_methods/player.rs:62` (`ON_MISSION_REWARDS_DISPLAY = 127`, no sender).
  - `crates/game/src/missions/rewards.rs:52`, `manager.rs:102,107`: `todo!()`s with no callers outside `crates/game/src/missions/`.
  - `db/resources/Missions/Seed/missions.sql`: 1,037 rows end `, 0, 0, NULL);` and the other 4 end `, 0, 0, '<script_spaces>');`. No non-zero `reward_naq`/`reward_xp`. 1,018 rows have `award_xp = true`.
  - `db/resources/Missions/Seed/mission_reward_groups.sql`: groups (1001,1,2), (1001,2,1), (40,6,6). `mission_rewards.sql`: 8 rows.
  - `entities/defs/alias.xml:461-496`; `docs/reverse-engineering/findings/mission-wire-formats.md:50-68`; `docs/protocol/client-method-dispatch-table.md:274`; `docs/protocol/cell-method-dispatch-table.md:318`.
  - `docs/analysis/castle-cellblock-rebuild/README.md:44` (D-CB10), `work-packets.md:176-178` (GC3 BlockedDesign), `uat-guide.md:989`.
  - `crates/services/src/cell/content/executor/mod.rs:512` (`Action::GrantXP` arm); `db/resources/Content/Seed/*.sql` has 9 `grant_xp` rows.
- Related/duplicates: #304 (parent), GC3 (Castle Cellblock design group), #265 C4 (transactional grants)

### Action text

Comment:

> Re-verified 2026-09-25 against main 059d6038. Rewards still never dispatch: `chosenRewards` is a stub, nothing sends `onMissionRewardsDisplay`, and `crates/game/src/missions/` is still dead `todo!()` code. Two premises from the earlier plan do not hold up. **(1)** The "empty-rewards short-circuit" would grant nothing, because every seeded mission has `reward_xp = 0` and `reward_naq = 0` while 1,018 have `award_xp = true`. The original server computed XP, and the Castle rebuild already parks this as GC3 / D-CB10, BlockedDesign on the formula. **(2)** The `Rewards` and `RewardChoices` payloads are not Python blobs. They are ordinary FIXED_DICTs in `entities/defs/alias.xml:461-496`, so a byte-exact wire test is possible today. The body is rewritten to split the work into a formula-independent reward-dialog round trip and an XP formula blocked on an owner decision.

#### New body

**Problem.** Completing a mission grants nothing. Three facts on main:

- `chosenRewards` (cell method 87) logs `UNIMPLEMENTED` (`crates/services/src/cell/cell_methods/player/world/mod.rs:136-139`).
- No code sends `onMissionRewardsDisplay` (client method 127, `crates/services/src/cell/client_methods/player.rs:62`).
- Mission completion (`crates/services/src/cell/missions/progression.rs::complete_mission_direct`) flips state only.

**Evidence.**

- Seed: every one of the 1,041 missions has `reward_xp = 0` and `reward_naq = 0`, and 1,018 have `award_xp = true` (`db/resources/Missions/Seed/missions.sql`). Item rewards exist only for missions 40 and 1001 (`mission_reward_groups.sql`: 3 groups; `mission_rewards.sql`: 8 items).
- Wire shapes are known: `Rewards` and `RewardChoices` are FIXED_DICTs in `entities/defs/alias.xml:461-496`. `docs/reverse-engineering/findings/mission-wire-formats.md:50-68` documents `chosenRewards`.
- Grant sinks already exist: `CellToBaseMsg::GrantXP`, `GrantCash` and `GrantItem`, with handlers in `base/.../progression` and `inventory/grant`. There is also an `Action::GrantXP` executor arm.
- Reference flow (intent, not client truth): `deprecated/python/cell/SGWPlayer.py:1091-1552`. `displayMissionRewards` sends method 127 or short-circuits. `chosenRewards` validates `lastRewardsMissionId`, that the mission is Active, the exact per-group choice count, and index bounds, then grants XP → cash → items and only then completes the mission.
- `crates/game/src/missions/` (`rewards.rs:52`, `manager.rs:102,107`) is an abandoned prototype with `todo!()`s and no external callers.

**Owner decision needed (blocks part B only).** What is the mission-completion XP formula? Every `reward_xp` is 0, the Castle spec observed 52 XP for missions 680/681/686, and Castle decision D-CB10 / GC3 excludes a hardcoded constant. Options: (a) RE the formula from the client or spreadsheets, (b) adopt a documented level×difficulty table as a Cimmeria decision, or (c) author per-mission `grant_xp` chain actions.

**Acceptance criteria.**

- A. Reward dialog round trip (formula-independent):
  - On completion, missions with reward groups send `onMissionRewardsDisplay` and stay Active. Missions without groups grant and complete in the same tick.
  - `chosenRewards` parses `RewardChoices` and validates last-offered mission, Active status, exact choice count per group, and index bounds. Each validation failure logs one `warn!`/`error!` with `mission_id`, `player_id` and `failure_kind`.
  - On success, grant XP, then naquadah, then items (count 1 per chosen index), then complete the mission last.
  - Delete `crates/game/src/missions/`.
- B. XP amount: implement whatever the decision above settles. Until then the XP field is 0, which matches today's seed.
- Transactionality: either one base-side aggregate transaction (grants + completion) or explicitly best-effort. State which, and test it.

**Test type.**

- Wire-format: byte-exact `Rewards` encode and `RewardChoices` decode.
- Unit tests for each validation rejection. Mission 1001 choice arithmetic: `[0,1]` → 5620+5621, `[1,1]` → a duplicate grant, `[0]` and `[0,1,2]` rejected, `[3]` out of bounds.
- A live-DB guard for the chosen atomicity rule that fails when the fix is reverted.

**Docs to update.** `docs/gameplay/mission-system.md`, `docs/reverse-engineering/findings/mission-wire-formats.md` (add `Rewards`), and `docs/protocol/client-method-dispatch-table.md` (mark 127 as implemented).

**Client impact.** Free: both messages are already in the client's def.

**Domain advisor.** `mission-systems-advisor` and `items-systems-advisor`.

## #715 — Mission frames are not suppressed for hidden missions (reference gates all of them on isHidden)

- Verdict: KEEP
- Priority: P2
- Labels: add `ready-for-human` (the body already says it is ready-for-human; the label is missing)
- Summary: The divergence is still exactly as described. `accept_mission` copies `is_hidden` from the def, then sends `onMissionUpdate`, `onStepUpdate` and one `onObjectiveUpdate` per objective with no hidden check. `is_hidden` is only consulted by `MissionManager::active_missions()`. New evidence settles half of the ticket's first "unknown", and against the "harmless" branch. The client's own cooked mission data does not mark these missions hidden: `CookedDataMissions.pak` entries `_682` and `_686` carry `IsHidden="false"`, and `_689` has no cooked entry at all. No `mission_overrides.rs` entry patches any of them. The seed's `is_hidden = true` for 682–686/689 is server-side only. The client therefore has no data of its own that would suppress these frames, which leans toward gating them, or toward adding cooked-mission overrides that set `IsHidden`. The UAT guide already asks the tester to confirm 689 stays out of the quest log.
- Evidence:
  - `crates/services/src/cell/missions/lifecycle.rs:105-165`: `mission.is_hidden = is_hidden;` then unconditional sends of `ON_MISSION_UPDATE`, `ON_STEP_UPDATE` and `ON_OBJECTIVE_UPDATE`.
  - `crates/entity/src/missions.rs:173`: the only read (`!m.is_hidden` filter in `active_missions`).
  - Seed: `db/resources/Missions/Seed/missions.sql` has 682–686 and 689 with `is_hidden = true`.
  - Local QA client `SourceCache.en-us/CookedDataMissions.pak`: `_682` and `_686` `IsHidden="false"`; `_689` missing.
  - `crates/services/src/base/mission_overrides.rs`: overrides exist only for 688 and others; none for 682–686/689.
  - `docs/analysis/castle-cellblock-rebuild/uat-guide.md:109-136` (the "689 … never shows in the quest log" check).
- Related/duplicates: #714

### Action text

Status comment:

> Triage 2026-09-25 (main 059d6038): the divergence is unchanged. New data point from the client's cooked data: `CookedDataMissions.pak` has `IsHidden="false"` for `_682` and `_686`, and `_689` has no cooked entry at all. Nothing in `base/mission_overrides.rs` patches them. The server-side seed flag `is_hidden = true` is the only place these missions are hidden, so the client has no data of its own that would drop these frames. That rules out the "client hides them itself" branch for 682–686 unless the UI filters on something else. The Cellblock UAT (`uat-guide.md` 689 check, plus the Hallway controllers) is still the cheapest way to see whether the frames surface. If they do, gate them with one predicate, or add cooked-mission `IsHidden` overrides. Adding `ready-for-human` for that check.

## #612 — RE + content: fail_objective is blocked — no failed-objective status exists and the client wire value is unknown

- Verdict: KEEP
- Priority: P3
- Labels: no change (`documentation`, `enhancement`). Optionally add `needs-info` for the wire value.
- Summary: Still accurate. `Action::FailObjective` loads (`loader/action.rs:363`) but has no executor arm; it falls to the `other =>` debug arm. `MissionInstance` has no `fail_objective` and no failed-objective list. The objective status constants are still only `STATUS_ACTIVE = 0` and `STATUS_COMPLETED = 1`, while the mission-level constants include `MISSION_FAILED = 3`. `MissionUpdate.failed_objective_ids` is plumbed through base and is always empty. Three new data points:
  - (a) The reference emulator did not know the wire value either. `deprecated/python/cell/MissionManager.py:769-773` has `# TODO: How to fail an objective? Status 2?` and sends status 1, the same value as completion. Its entity-level `failObjective` (`:437-449`) only moves the objective into `failedObjectives` and fires `mission_objective.failed::<id>`.
  - (b) The objective status enum itself is inconsistent across sources. `docs/reverse-engineering/findings/mission-state-machine.md:190-205` reads `onObjectiveUpdate` status 0 as "Removed" and 1 as "Unlocked", with 2 = complete only inferred. The Rust and Python servers instead send 0 for active and 1 for completed. Any "failed" byte has to be settled together with that enum.
  - (c) The only seeded use is still demonstration chain 2025 (TestEffect, `effect_pulse_end`), which is inert until #610 settles effect-chain binding.
  
  Line citations in the body have drifted: `progression.rs` `complete_objective` is now at :232.
- Evidence:
  - `crates/entity/src/missions.rs:13-21` (`MISSION_FAILED = 3`; objective `STATUS_ACTIVE = 0`, `STATUS_COMPLETED = 1`), `:91` (mission-level `fail()`).
  - `crates/services/src/cell/content/executor/mod.rs`: no `Action::FailObjective` arm (falls to `other =>`, around line 672).
  - `crates/services/src/base/world_entry/methods/missions/mod.rs:24-92`, `cell_dispatch/progression_dispatch.rs:57-68,311-323` (`failed_objective_ids` plumbing).
  - `db/resources/Content/Seed/effects_chains.sql:107-115` (chain 2025).
  - `deprecated/python/cell/MissionManager.py:437-449,754-774`; `deprecated/python/common/Constants.py:19-21`.
- Related/duplicates: #610 (effect-chain binding; the only seed use is behind it), #266 (task-count wire width), #265 A3/B6

### Action text

Status comment:

> Triage 2026-09-25 (main 059d6038): unchanged and still blocked. Additional evidence: the previous Python emulator did not know the failed-objective wire value either. `MissionManager.py:769-773` has `# TODO: How to fail an objective? Status 2?` and sends 1, the same byte as completion. Its entity-level `failObjective` only tracked the objective server-side and fired `mission_objective.failed::<id>`. Separately, the objective status enum is itself unsettled. The RE finding (`mission-state-machine.md` §Objective) reads 0 as removed and 1 as unlocked, while both server implementations send 0 as active and 1 as completed. Resolve the "failed" value together with that enum. A server-side-only step would be safe to land ahead of the RE: track `failed_objectives` and persist `failed_objective_ids`, with no client frame. The only seeded use (chain 2025) is still inert behind #610, so this stays P3.

## #610 — content-engine: no effect_* trigger is ever dispatched — effects_chains.sql is inert

- Verdict: NEEDS-OWNER
- Priority: P3
- Labels: keep `needs-info`
- Summary: Still true on main. There are no `EffectInit`, `EffectPulseBegin`, `EffectPulseEnd` or `EffectRemoved` references anywhere under `crates/services/src/`. PR #745, which adds `OnEffectInit` dispatch with a depth guard for the content `ApplyEffect`/`LaunchAbility` paths only, is open with green CI. The owner has explicitly held it: "Leaving this open for an owner decision; not merging". The blocker is the binding question from the 2026-09-19 premise review. Effect chains are seeded with `scope_id = NULL` and `event_key = NULL`, and the matcher accepts any effect event. Emitting PulseEnd blindly would therefore fire every TestEffect demo chain (2021–2025) for unrelated effects, including abandoning mission 1627. All the seeded chains are demonstration rows, so nothing player-facing is lost while this waits.
- Evidence:
  - `grep -rn "EffectInit|EffectPulseBegin|EffectPulseEnd|EffectRemoved" crates/services/src/` → 0 hits on main-ro.
  - `crates/content-engine/src/triggers/mod.rs:107-116` (the four variants); `triggers/matching.rs` (effect arms match on discriminant only).
  - `crates/services/src/cell/content/executor/mod.rs:644-662`: the `ApplyEffect` arm comment still reads "currently unreachable".
  - `db/resources/Content/Seed/effects_chains.sql` (chains 2011, 2021–2025, `scope_id NULL`).
  - PR #745 (open, updated 2026-09-25). Owner comment on the PR: held for the binding decision.
- Related/duplicates: #612 (chain 2025 `fail_objective`), #265 B3 (damage/effect reactive triggers), PR #745

### Action text

Owner question (post as a comment):

> Triage 2026-09-25: still no effect trigger dispatch on main. PR #745 (OnEffectInit plus a depth guard) is ready but held for this decision. **How should effect-scoped chains bind?** (a) By stable `effect_id`, carried in `event_key` or `scope_id`. (b) By script class (Reload / RangedEnergyDamage / TestEffect). (c) Disable the demonstration chains 2011 and 2021–2025 and land typed lifecycle dispatch with no seeded consumers. Once this is decided, #745 can be finished (with the matcher filtering on the chosen key), followed by PulseBegin/PulseEnd/Removed and the ability-path dispatch. Until then this is P3: every seeded effect chain is a demonstration row.

## #616 — move_waypoint emits no position broadcast — chain-repositioned NPCs may not visibly move

- Verdict: CLOSE (completed)
- Priority: P3
- Labels: no change (closing)
- Summary: Fixed by PR #707, merged 2026-09-19, "broadcast move_waypoint snap to witnesses immediately (#616)". `move_waypoint` now calls `update_position_preserving_facing`, tags the move as `MoveSource::Content` for the NPC-AI detectors, reseeds the validator clock, and sends `CellToBaseMsg::EntityMoved` to every current witness. The misleading `MoveWaypoint` doc comment was also corrected: it now says "an instant server-side position write, not a path or a walk animation… `speed` is parsed … but the executor does not use it". The issue's question 2 (interpolation versus snap) was resolved in favour of an honest snap with an immediate broadcast. PR #707 did not change the SGC_W1 rows (3005, 3009, 3011) themselves.
- Evidence:
  - `crates/services/src/cell/content/executor/world/mod.rs:601-650` (witness fanout of `EntityMoved`).
  - `crates/content-engine/src/actions.rs:399-409` (corrected doc).
  - Guard: `crates/services/src/cell/content/executor/world/tests.rs:669` (`move_waypoint_updates_target_position_to_destination`, plus the witness-message assertions).
  - Commit e5c881bd (PR #707).
- Related/duplicates: #613, #335 (the SGC_W1 Hammond/Airman rows get their in-game check under that epic)

### Action text

Closing as completed by PR #707 (e5c881bd). `move_waypoint` now sends `EntityMoved` to every current witness right after the grid update, instead of waiting for the next AoI tick. The `MoveWaypoint` doc comment now describes the actual behaviour: an instant snap, with `speed` unused. Interpolated movement was not adopted. If a scripted walk is ever needed, it should get its own ticket (NPC-AI movement primitives exist since the NA campaign). The visual check of the SGC_W1 Hammond/Airman repositions belongs with #335.

## #268 — Content engine SystemMessage action: client wire format unknown

- Verdict: REWRITE
- Priority: P3
- Labels: remove `documentation`, add `needs-triage` (so a maintainer can re-slot it as a small implementation ticket)
- Summary: The premise, "wire format unknown, needs RE", is stale. The target method is already documented. Client method 27 is `onSystemCommunication(INT32 TextType, INT32 StringId, WSTRING Speaker, ARRAY<StringToken> tokenList)` (`docs/protocol/client-method-dispatch-table.md:124`), and `StringToken` is a FIXED_DICT `{stringID: INT32, literal: WSTRING}` (`entities/defs/alias.xml:538-543`). The Rust constant `ON_SYSTEM_COMMUNICATION = 27` exists but has no sender. The reference emulator's space scripts send exactly this for region text, `client.onSystemCommunication(11, <stringId>, '', [])`, nine times across `Castle.py` (5188/5189/5190) and `Castle_CellBlock.py` (5180, 5040). TextType 11 is `TEXT_TYPE_DiscoveryText` (`enumerations.xml:1527`). The executor arm is still a log-only stub. Seed usage has shrunk to one row: chain 1013, `system_message 5040`, Cellblock Region2. The other 10 went with the C01 purge. `docs/content/content-engine.md:451,683` still says "11 seeded rows". The Castle audit lists the three Castle region hints as authorable once the arm works.
- Evidence:
  - `crates/services/src/cell/content/executor/mod.rs:420-432` (stub arm, TODO text).
  - `crates/services/src/cell/client_methods/communicator.rs:4` (`ON_SYSTEM_COMMUNICATION: u16 = 27`, no callers).
  - `docs/protocol/client-method-dispatch-table.md:124-125`; `entities/defs/alias.xml:538-543`; `entities/defs/enumerations.xml:1527`.
  - `deprecated/python/cell/spaces/Castle.py:240,249,298`; `deprecated/python/cell/spaces/Castle_CellBlock.py:384,394`.
  - `db/resources/Content/Seed/castle_cellblock_chains.sql:570` (the only row); `docs/analysis/castle-rebuild/audit.md:70`; `docs/analysis/dialog-ui-redesign/worknotes/du07.md:442-444`.
- Related/duplicates: #242 (closed-candidate duplicate chains), #265 (listed as a stub action)

### Action text

Comment:

> Triage 2026-09-25 (main 059d6038): the "wire format unknown" premise no longer holds. `onSystemCommunication` is client method 27 with a documented signature (`client-method-dispatch-table.md:124`; `StringToken` in `alias.xml:538`). The previous Python emulator drove region/discovery text with exactly `onSystemCommunication(11, stringId, '', [])` (TextType 11 = `TEXT_TYPE_DiscoveryText`) in `Castle.py` and `Castle_CellBlock.py`. What remains is implementation plus an in-game check, not RE. Only one seeded row is left (chain 1013 → 5040); the other 10 were dropped with the C01 purge. Body rewritten accordingly.

#### New body

**Problem.** `Action::SystemMessage { message_id }` is a log-only stub (`crates/services/src/cell/content/executor/mod.rs:420-432`), so region and discovery text never reaches the client. An earlier attempt routed it through `onPlayerCommunication` (method 28) and garbled chat.

**Evidence.**

- Target method: client method 27 `onSystemCommunication(INT32 TextType, INT32 StringId, WSTRING Speaker, ARRAY<StringToken> tokenList)` (`docs/protocol/client-method-dispatch-table.md:124`). `StringToken = {stringID: INT32, literal: WSTRING}` (`entities/defs/alias.xml:538-543`). The constant exists as `crates/services/src/cell/client_methods/communicator.rs:4` with no sender.
- Reference usage (intent): `deprecated/python/cell/spaces/Castle.py:240,249,298` and `Castle_CellBlock.py:384,394` call `onSystemCommunication(11, <stringId>, '', [])`. TextType 11 = `TEXT_TYPE_DiscoveryText` (`entities/defs/enumerations.xml:1527`).
- Seed consumers: chain 1013 (`castle_cellblock_chains.sql:570`, string 5040). The Castle region hints 5188/5189/5190 (`docs/analysis/castle-rebuild/audit.md:70`) become authorable once this works.

**Acceptance criteria.**

- The `SystemMessage` arm sends `onSystemCommunication(TextType, message_id, "", [])` to the triggering player only. TextType defaults to 11. An optional `text_type` param on the action/loader is allowed if a seed needs another type.
- Chain 1013 (Cellblock Region2) shows string 5040 in-game, with no chat garbling and no client freeze.
- `docs/content/content-engine.md` (the §10 gap and the "11 seeded rows" statements at :451/:683) and `docs/protocol/client-method-dispatch-table.md` are updated.

**Test type.** Wire-format: byte-exact args for `(11, 5040, "", [])`. Executor unit test: exactly one `EntityMethodCall` with method 27 to the triggering entity. The guard must fail if the arm reverts to logging.

**Client impact.** Free: an existing client method.

**Domain advisor.** `mission-systems-advisor` (content), `aoi-witness-broadcast` (owner-only send).

**Needs a human for.** An in-game check that the Region2 text renders in the expected UI surface.

## #233 — Gate respawn-point availability by player progression (per-player unlocked-respawners set)

- Verdict: REWRITE
- Priority: P2
- Labels: add `needs-triage` (the unlock mechanism still needs a maintainer pick: A proximity or B chain action)
- Summary: The bug is still real. The Defeat Window lists every respawner in the player's world. The code has moved: it is now `send_begin_aid_wait` in `crates/services/src/cell/abilities/death/side_effects.rs:118-133`, not `damage_apply/mod.rs:362`, and it logs `filter = "world_name_only"`. The seed now has 12 respawners (Castle 1–4, Cellblock 5 and 8, world 23 ×2, Harset 20–23), not 8. The body's implementation plan has a wrong persistence premise. It proposes a new `sgw_player_respawners` table plus a migration script. Migrations are disallowed by project rule, and the storage already exists: `sgw_player.known_respawners integer[] DEFAULT '{}'` (`db/sgw/Players/Tables/sgw_player.sql:38`). The reference emulator used that exact column: `SGWPlayer.knownRespawnerIds` is loaded from it, `getActiveRespawners()` filters by known and same-world, and `addRespawner()` adds to it. No Rust code reads or writes the column today. In the reference, the only caller of `addRespawner` is a GM command (`commands/Player.py:108`), so the original in-game unlock trigger is not evidenced either way. Proximity (option A) remains a Cimmeria design choice, not restoration.
- Evidence:
  - `crates/services/src/cell/abilities/death/side_effects.rs:118-133,164-175,191` (world-only filter; the `respawnerID = 0` "Respawn Point" fallback when empty).
  - `db/resources/Worlds/Seed/respawners.sql`: 12 rows.
  - `db/sgw/Players/Tables/sgw_player.sql:38` (`known_respawners integer[]`); `grep known_respawners crates/` → 0 hits.
  - `deprecated/python/cell/SGWPlayer.py:82,168,241,641-660,1253-1264`; `deprecated/python/cell/commands/Player.py:108`.
  - Castle/Harset respawner work: PRs #651, #667, #717. None of them gate by player.
- Related/duplicates: #265 C7/D (counter-based alternative), Castle CA00

### Action text

Comment:

> Re-verified 2026-09-25 (main 059d6038). The Defeat Window still lists every same-world respawner (`cell/abilities/death/side_effects.rs::send_begin_aid_wait`, log field `filter = "world_name_only"`), so the bug stands. The plan needs one correction: we do not need a new `sgw_player_respawners` table or a migration. `sgw_player.known_respawners integer[]` already exists in the schema, and the previous emulator used it (`knownRespawnerIds`, `getActiveRespawners`, `addRespawner`). The reference only ever filled it from a GM command, so the unlock trigger (proximity vs chain action) is a Cimmeria design choice. Body rewritten with current paths, 12 respawners, and the existing column.

#### New body

**Problem.** On death, the Defeat Window (`onBeginAidWait`) offers every respawner in the player's current world, including ones the player has never reached. In Castle_CellBlock, "Level 7: Ring Transporters" (respawner 5) appears before the ring puzzle is done, so dying once skips the progression.

**Evidence.**

- `crates/services/src/cell/abilities/death/side_effects.rs:118-133`: the filter is `r.world_name == *wn` only. An empty list falls back to a single `respawnerID = 0` "Respawn Point".
- Seed: 12 respawners (Castle 1–4, Cellblock 5 and 8, world 23: 6 and 7, Harset 20–23).
- Storage already exists: `db/sgw/Players/Tables/sgw_player.sql:38` `known_respawners integer[] DEFAULT '{}'`. No Rust reads or writes it.
- Reference intent: `deprecated/python/cell/SGWPlayer.py` loads `knownRespawnerIds` from that column (:168), persists it (:241), and `getActiveRespawners()` (:1253-1264) returns known ∩ same-world. The only `addRespawner` caller is a GM command (`cell/commands/Player.py:108`).

**Design pick (maintainer).**

- A. Proximity auto-unlock: 1 Hz tick; unlock within a radius of about 6 m of the respawner position; the chardef start respawner unlocks on first load.
- B. Chain action `unlock_respawner { respawner_id }` authored on region or step chains.

A needs no content authoring. B gives explicit control. Either way, add a GM `.`-console command to add or remove a known respawner, mirroring the reference.

**Acceptance criteria.**

- Load `known_respawners` into cell state on player load. Persist additions through a cell→base message: `UPDATE sgw_player SET known_respawners = array_append(...)`, idempotent. No new table and no `db/scripts` migration.
- `send_begin_aid_wait` lists only respawners that are known and in the same world. Keep the existing fallback when the list is empty.
- A fresh character in Castle_CellBlock sees only "Stasis Chamber" until reaching level 7.
- Every button press still gets feedback: a respawn choice from the filtered list works on the first press.

**Test type.**

- Unit: the filter excludes unknown respawners, and the guard fails if the filter is removed.
- Unit: the unlock path, whether tick or action, is idempotent.
- Live-DB: `known_respawners` round-trip (append, then reload), cleaned up by exact sentinel.

**Docs to update.** `docs/gameplay/death-respawn-system.md`. Also `docs/content/content-engine.md` if option B adds an action.

**Client impact.** Free.

**Domain advisor.** `npc-ai-spawn-advisor` (respawners), `database-persistence`.

## #277 — Verify: onUpdateRacialParadigmLevel emit on racial-paradigm level change

- Verdict: CLOSE (completed)
- Priority: P3
- Labels: no change (closing)
- Summary: This was an audit ticket, and the audit is done. PR #728 (merged 2026-09-25, "docs(crafting): audit racial paradigm schema and runtime gaps") records the def-verified schema in `crafting-wire-formats.md` and `crafting-state-machine.md`: client method 138, `INT32 aRacialParadigmId` + `INT8 aLevel`, a five-byte payload. It also documents that the emit is missing, that nothing mutates `racial_paradigm_levels`, and that `onPlayerDataLoaded` carries no levels, which corrects the ticket's assumption about relog. The missing implementation was filed as #723, which is exactly what the ticket's criterion "If misimplemented: file a real-bug issue" asks for. The "live test" criterion cannot run until #723 lands and now belongs there.
- Evidence:
  - PR #728 merged 2026-09-25T05:56Z.
  - `docs/reverse-engineering/findings/crafting-state-machine.md:15,52,190-214` ("Verified Schema… method index 138… five bytes"; "The implementation gap is tracked in #723").
  - `crates/services/src/cell/client_methods/player.rs:83-84` (`ON_UPDATE_RACIAL_PARADIGM_LEVEL = 138`, no sender).
- Related/duplicates: #723 (implementation), #567 (crafting epic)

### Action text

Closing as completed. The audit landed in PR #728. The wire schema is verified from `SGWPlayer.def` (method 138: `INT32 aRacialParadigmId`, `INT8 aLevel`, five bytes) and recorded in `crafting-wire-formats.md` and `crafting-state-machine.md`. The runtime gap is documented: no emit, no production mutation of `racial_paradigm_levels`, and `onPlayerDataLoaded` carries no paradigm levels, so a relog does not restore them either. The implementation and the live crafting-UI check are tracked in #723.

## #723 — Crafting: wire racial-paradigm progression and client synchronization

- Verdict: KEEP
- Priority: P3
- Labels: remove `needs-triage`. Add `needs-info` until the progression rule is specified (see below).
- Summary: Accurate against main. `base/crafting/persistence.rs` loads and saves `racial_paradigm_levels`. `ON_UPDATE_RACIAL_PARADIGM_LEVEL = 138` has no sender. Nothing in production mutates the levels. The one open question the body flags, the progression rule, is confirmed as a gap in the reference too. The Python `Crafter` initialises every paradigm to level 1 (`Crafter.py:44-47`). `updateRacialParadigmLevel` → `onRacialParadigmUpdated` → `onUpdateRacialParadigmLevel` is reachable only from the GM command `commands/Crafting.py:20 setRacialParadigmLevel`. So there is no evidenced gameplay award rule. A scoped first cut is: initial sync on login, a GM `.`-console setter, and the emit on change. Automatic awards wait on a design decision. This also makes #567 Phase 6's "wire format unknown, blocked on x64dbg D.2" stale.
- Evidence:
  - `crates/services/src/base/crafting/persistence.rs:87-152,239-271`.
  - `crates/services/src/cell/client_methods/player.rs:83-84`.
  - `deprecated/python/cell/Crafter.py:44-47,66-73,169`; `deprecated/python/cell/SGWPlayer.py:842-848`; `deprecated/python/cell/commands/Crafting.py:20`.
  - `docs/reverse-engineering/findings/crafting-state-machine.md:190-214` (PR #728).
- Related/duplicates: #277 (audit, close), #567 (parent epic, Phase 6)

### Action text

Status comment:

> Triage 2026-09-25: body confirmed against main 059d6038. On progression policy: the previous emulator had no gameplay award rule either. `Crafter` starts every paradigm at 1, and the only mutation path is the GM `setRacialParadigmLevel` command → `onRacialParadigmUpdated` → `onUpdateRacialParadigmLevel`. Suggested scope for an agent: (1) push each paradigm level via method 138 on world entry (owner-only), (2) a GM `.`-console setter that persists the new level and emits it, (3) a byte-exact 5-byte wire test plus a live-DB persist guard. Automatic level awards stay out of scope until a maintainer specifies the rule. Moving from `needs-triage` to `needs-info` for that rule only.

## #567 — Implement crafting activity handlers (craft/research/reverse-engineer/alloy/spendASP)

- Verdict: KEEP
- Priority: P2
- Labels: no change
- Summary: Still accurate. All six cell handlers in `crates/services/src/cell/cell_methods/player/crafting.rs` still log `UNIMPLEMENTED`: spendAppliedSciencePoints (:33), craft (:48), research (:60), reverseEngineer (:65), alloying (:72), respecCrafting (:84). No crafting PR has merged since Phase 1 (#427) and the GM grants (#521); the only later crafting PRs are docs (#576, #728). `ALLOYING_ELEMENTARY_COUNTS` still exists only in `deprecated/python/common/Constants.py:83`. `ON_UPDATE_CRAFTING_OPTIONS = 140` is defined with no sender. One stale item: Phase 6 says `onUpdateRacialParadigmLevel` is "wire format unknown, blocked on x64dbg D.2". PR #728 verified the format from the def (method 138, `INT32` + `INT8`), and the work is split out as #723. D.1 (respec confirm) and D.3 (`craftingEntityFlags`) remain open x64dbg items.
- Evidence:
  - `crates/services/src/cell/cell_methods/player/crafting.rs:33-84` (the stubs), `:102` (`#[allow(dead_code)]` Phase-2 helper).
  - `crates/services/src/cell/client_methods/player.rs:84,88`.
  - `docs/reverse-engineering/findings/crafting-restoration.md:104-119` (D.1–D.3), `crafting-state-machine.md:190-214` (D.2 superseded by the def-verified schema).
  - Merged PRs #427, #521, #576, #728.
- Related/duplicates: #723 (Phase 6 paradigm sync), #53 (superseded)

### Action text

Status comment:

> Triage 2026-09-25 (main 059d6038): unchanged. The six activity handlers are still parse-and-log stubs, and `onUpdateCraftingOptions` (140) has no sender. One correction to Phase 6: `onUpdateRacialParadigmLevel` is no longer blocked on x64dbg D.2. PR #728 verified the schema from `SGWPlayer.def` (method 138, `INT32 aRacialParadigmId` + `INT8 aLevel`), and that slice is now #723. D.1 (respec confirm) and D.3 (`craftingEntityFlags`) are still open. Phase 1 (`spendAppliedSciencePoints`) remains the unblocked starting point.

## #265 — Content engine: 25 gaps against client/binary expectations (V5-grounded analysis)

- Verdict: REWRITE
- Priority: P3
- Labels: remove `documentation`; add `needs-triage`. Maintainer call: keep as a tracking checklist, or close as not planned and let gaps be filed on demand as content needs them. The analysis below supports either.
- Summary: This is a speculative wishlist written before the Castle/Harset/NPC-AI campaigns, plus a V5 correction comment. Checked item by item against main, only a few gaps have closed, and those were closed by campaign needs rather than by this plan:
  - C5 async/wait: per-action `delay_ms` scheduling, C08a/#646.
  - B4 partially: the `OnEntityHealthBelow` trigger.
  - D1 partially: `content.resolve`, `content.deferred` and `content.execute_actions` tracing, and cover counters.
  - A4 was dropped by V5.
  
  The engine surface is now 54 actions, 35 triggers and 14 conditions (the body says 48/26/12). Almost all proposed Phase A/B variants are still absent: no state-flag, posture/archetype-animation, task-level, faction-set, cooldown, damage/level/objective/login triggers, proximity/LoS/random/level conditions. No shipped content has been blocked on them; the campaigns worked around them with existing actions. Two guardrail facts the body does not know:
  - `content_triggers.once` is loaded but never honoured, so "one-shot" is the author's job (C3).
  - `TriggerChain` is still log-only with zero recursion (C2). `StartTimer`/`CancelTimer`, `PlayAnimation`, `PlaySound`, `ModifyProperty`, `RollLootTable`, `SpawnLootBag` and `ExecuteCustom` also have no executor arms (they fall through to `other =>`).
- Evidence (per item, main-ro):
  - Surface: `crates/content-engine/src/actions.rs:36` (54 variants), `triggers/mod.rs:28` (35 `On*`), `conditions/mod.rs:15` (14: PropertyEquals, PropertyInRange, HasItem, HasAbility, InRegion, FactionCheck, CustomExpression, MissionStatus, StepStatus, Archetype, ObjectiveStatus, Counter, StatBelowMax, World).
  - Absent (a grep of `crates/content-engine/src` for StateFlag, PlayArchetypeAnimation/Recomposite, CompleteTask/TaskCount, SetFaction/SetAlignment/IsHostile, SetCooldown/ResetCooldown/InterruptChannel, OnDamage, OnLevelUp, OnObjective*/MissionFailed, OnLogin/Logout, Proximity/LineOfSight, RandomChance, PlayerLevel/StatComparison, rate_limit, dry_run, schema_version) → 0 hits each.
  - B2: `crates/services/src/cell/content/event_dispatch/lifecycle/mod.rs:92-130` fires `EntityDeath` for the killer only.
  - C2: `crates/services/src/cell/content/executor/mod.rs:663-671` (`TriggerChain` → debug log "caller must re-dispatch").
  - C3: `content_triggers.once` is loaded (`crates/services/src/cell/content/engine_loader.rs:68-79`) and never consulted in `chain/mod.rs`. `chain_replay_tests/entity_health_below.rs:91,329` says "`content_triggers.once` is dead".
  - C5: `delay_ms` scheduling (`executor/deferred.rs`, PR #646 C08a).
  - C6: `ApplyEffect` still targets the source (`executor/mod.rs:644-662`). Tag-targeted actions exist for LaunchAbility, SetAggression, GenerateThreat, SetNpcAiState, MoveEntity and others.
  - C7: counters are unnamespaced per-entity strings (`executor/counter.rs`).
  - D1: `docs/architecture/observability.md:231,246` (`content.deferred`, `content.resolve`); `event_dispatch/cover.rs:81` counters.
  - Linked issues: #266 OPEN, #267 CLOSED, #270 CLOSED, #271 OPEN (PR #718), #272 CLOSED, #278 OPEN, #279 OPEN, #268 (rewrite, above), #269 (close, above).
- Related/duplicates: #264 (Bible), #266, #268, #269, #278, #279, #610, #612

### Action text

Comment:

> Triage 2026-09-25 against main 059d6038. I checked each gap from the body and the V5 revision comment against the current engine (now 54 actions / 35 triggers / 14 conditions). Three closed, all through campaign work rather than this plan: C5 (per-action `delay_ms`, #646), part of B4 (`OnEntityHealthBelow`), and part of D1 (`content.resolve`, `content.deferred` and the execute spans). Almost every proposed variant is still absent, and none has blocked shipped content, because the Castle, Harset and NPC-AI campaigns authored around them. Two facts the inventory missed: `content_triggers.once` is loaded but never enforced, and eight authorable actions (StartTimer, CancelTimer, PlayAnimation, PlaySound, ModifyProperty, RollLootTable, SpawnLootBag, ExecuteCustom) have no executor arm. The checklist is rewritten below. Suggest a maintainer either keep this as the tracking list or close it as not planned and file individual gaps when content needs them.

#### New body

**Status.** A tracking checklist of content-engine capability gaps. It was re-verified on 2026-09-25 against main 059d6038. The original V5-era analysis is kept in the issue history. Surface today: 54 `Action` variants, 35 triggers, and 14 conditions (`crates/content-engine/src/{actions.rs,triggers/mod.rs,conditions/mod.rs}`). File a focused issue for any item before implementing it. Each needs its own evidence, tests, and docs row.

**A. Spec-facing primitives**

- [ ] A1 state-flag set/clear + `Condition::StateFlag`. Absent. The mechanism belongs to the witness-fanout helper (#278).
- [ ] A2 `PlayArchetypeAnimation { entity_tag, event_id }` and the BeingAppearance recomposite action. Absent (#279). Also note that `PlayAnimation` exists but has no executor arm.
- [ ] A3 task-level actions and `Condition::TaskCount`. Absent. Blocked on #266 (`MissionTaskStatus.count` width).
- [x] A4 Damage/Heal actions. Dropped by V5 (`ChangeStat` + `GenerateThreat`).
- [ ] A5 `SetFaction` / `SetAlignment` / `IsHostile` / `FactionEquals`. Absent. `FactionCheck` still uses the three-way enum.

**B. Gameplay triggers and conditions**

- [ ] B1 cooldown manipulation (set/reset/warmup/interrupt channel). Absent.
- [ ] B2 party propagation of `EntityDeath`. Absent; it fires for the killer only (`event_dispatch/lifecycle/mod.rs:92`).
- [ ] B3 `OnDamageDealt` / `OnDamageReceived`. Absent.
- [~] B4 stat-threshold trigger. Partial: `OnEntityHealthBelow { entity_tag, pct }` exists. There is no generic `OnStatChange`.
- [ ] B5 `OnLevelUp` / `OnAbilityUnlocked`. Absent.
- [ ] B6 `OnObjectiveCompleted` / `OnMissionFailed`. Absent. `OnMissionAccepted`, `OnMissionCompleted` and `OnMissionAbandoned` exist.
- [ ] B7 session login/logout triggers. Absent.
- [ ] B8 `EntityProximity` / `LineOfSight` conditions. Absent. `Condition::World` exists.
- [ ] B9 `RandomChance`, `PlayerLevel`, `ItemEquipped` condition, `StatComparison`. Absent. `StatBelowMax` exists.

**C. Guardrails (each needs a maintainer decision first)**

- [ ] C1 per-chain rate limit. Absent.
- [ ] C2 `TriggerChain` recursion. Still log-only, zero depth (`executor/mod.rs:663`). The only depth guard in flight is effect-init (PR #745).
- [ ] C3 idempotency / one-shot. `content_triggers.once` is loaded but **never enforced**. Either implement it or drop the column.
- [ ] C4 transactional rollback across actions. Absent (see #310 for reward grants).
- [x] C5 wait/delay. Per-action `delay_ms` scheduling landed (#646, `executor/deferred.rs`). `StartTimer`/`CancelTimer`/`OnTimer` have no executor arm.
- [~] C6 recipient targeting. Partial: many actions take `entity_tag`. `ApplyEffect` still targets the source only.
- [ ] C7 counter namespacing. Absent; counters are per-entity free-form strings.

**D. Observability and schema**

- [~] D1 execution telemetry. Partial: `content.execute_actions` span, `content.resolve` (condition-failed) and `content.deferred` events, cover counters. There are no per-chain fire counters.
- [ ] D2 dry-run. Absent (chain-replay tests are the offline substitute).
- [ ] D3 edit audit log. Not needed while seeds live in git; propose dropping.
- [ ] D4 `schema_version` on action/condition/trigger encodings. Absent.

**E. Executor arms missing for authorable actions (new)**

- [ ] `StartTimer`, `CancelTimer`, `PlayAnimation`, `PlaySound`, `ModifyProperty`, `RollLootTable`, `SpawnLootBag`, `ExecuteCustom` fall through to `other =>` in `crates/services/src/cell/content/executor/mod.rs`. Either implement them or reject them at load time.
- [ ] `SystemMessage`: see #268. `FailObjective`: see #612. Effect triggers: see #610.

**Docs.** Keep `docs/content/content-engine.md` in step whenever an item closes.

**Domain advisor.** `mission-systems-advisor`.

## Batch summary

| # | verdict | priority | one-line reason |
|---|---|---|---|
| 715 | KEEP (+`ready-for-human`) | P2 | Hidden-mission frames still ungated. The client PAK has `IsHidden="false"` for 682/686 and no entry for 689, so the client cannot be hiding them itself |
| 612 | KEEP | P3 | Still no objective-failed state or arm. The Python reference also guessed ("Status 2?"), and the objective status enum itself is unsettled |
| 610 | NEEDS-OWNER | P3 | No effect triggers dispatched. PR #745 (OnEffectInit) is held by the owner on the effect-chain binding decision (effect id vs script class vs disable demo chains) |
| 335 | KEEP | P2 | SGC_W1 chains still 3001–3028 only. Hack minigame is a placeholder. Range/#334 details corrected |
| 334 | REWRITE | P2 | Premise wrong: client Use gate is cooked `ItemEventSet` (`_5168` AbilityID=0), not `items_event_sets`. PR #605's DB row alone does nothing; needs a cooked-item override |
| 310 | REWRITE (+`needs-info`) | P2 | Rewards still dark. Every seeded `reward_xp`/`reward_naq` is 0, so XP needs a formula decision (GC3). `Rewards`/`RewardChoices` are plain FIXED_DICTs, not blobs |
| 269 | CLOSE (completed) | P3 | RemoveItem (#113/#371), StartMinigame difficulty (#652), clip sizes from `resources.items` all landed |
| 268 | REWRITE | P3 | Not an RE gap: `onSystemCommunication` (method 27) is documented and the reference used `(11, stringId, '', [])`. Only 1 seed row left (chain 1013) |
| 265 | REWRITE (or close not planned) | P3 | Of 25 gaps, only C5 is done and B4/D1 are partial. Found `content_triggers.once` is never enforced and 8 actions have no executor arm. Updated checklist provided |
| 242 | CLOSE (completed) | P3 | Chains 5020/5021 deleted with the whole auto-exported file in #646 (C01) |
| 233 | REWRITE | P2 | Bug stands (world-only filter). The plan's new table plus migration is wrong: `sgw_player.known_respawners` already exists (reference used it) |
| 616 | CLOSE (completed) | P3 | PR #707 broadcasts `EntityMoved` to witnesses and fixed the `MoveWaypoint` doc |
| 277 | CLOSE (completed) | P3 | Audit done in PR #728. Implementation split to #723 |
| 723 | KEEP (`needs-triage`→`needs-info`) | P3 | Accurate. Reference has no gameplay award rule (GM-only), so scope as login sync + GM setter + emit |
| 567 | KEEP | P2 | All six handlers still stubs. Phase 6 "D.2 wire unknown" is stale (#728, split to #723) |
