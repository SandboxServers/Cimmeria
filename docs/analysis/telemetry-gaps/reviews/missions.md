# Missions and content telemetry review

Reviewer: mission-systems-advisor, 2026-10-10. Data: colo SigNoz, last 7 days, aggregate queries only.
Every mission transition and chain firing ships, with mission, step and chain names, and shares a trace with its chain. But none of the cell lifecycle rows carries `player_id` or `account_id`, only a recyclable `entity_id`. `content.resolve` logs ids as Debug strings (`"Some(EntityId(2))"`).
The step-stall detector does see #1341: 17 of 40 `step_stalled` rows are 622 "Search the nearby corpses". But a stall can't tell "the client never had the object" from "the player didn't look", and nothing joins the stall to the quest object the step needs.

## 1. Inventory

| Target / scope | Level | In OTEL_FILTER | 7-day rows |
|---|---|---|---|
| `cimmeria_cell_content::cell::missions::{lifecycle,progression}` | INFO/WARN | yes (crate row) | accepted 184, step advanced 107, completed directly 55, objective 5 (DEBUG), hidden-frame suppress 68 (DEBUG). **0 WARN** |
| `…::content::executor::mission` (`Content: accepting/advancing/completing…`) | INFO | yes | 184 / 107 / 55 / 5 |
| `…::content::event_dispatch::*` (`fire_*: matched` / `no chains matched`) | INFO / DEBUG | yes | matched ~400; no-match ~4,400, of which **cover is 3,348** |
| `content.resolve` (condition failed, filtered out, once spent) | DEBUG | yes | 1,393 / 20 / 4 |
| `mission.step_context` | DEBUG | yes | 107 |
| `dialog.display`, `…::interactions::dialog` (`Sending onDialogDisplay`) | DEBUG | yes | 90 / 213 |
| `playtest.friction` | WARN | via `info` | step_stalled 40, objective_never_completed 1, dialog_displaced 1, repeat_interact 1, region_dwell 1 |
| `content`, `content.deferred` | INFO | yes | 122 / 2 |
| `cimmeria_base_methods::…::missions` (`Mission state persisted`, `Loaded saved missions`) | DEBUG / INFO | yes | 351 / 141. 0 ERROR |
| `…::player_init::mission_restore` | DEBUG/WARN | yes | 35 DEBUG, 0 WARN |
| `cimmeria_content_engine::*` | DEBUG | **no** (`crate_rows.rs:41`, never exported below INFO) | invisible |
| `mission` (custom target) | INFO | yes (`filters.rs:356`) | **dead: no emitter in the tree** |

Never fired in 7 days: any offer-guard refusal, `complete_objective` WARN, `Mission completed!` (the auto-complete path), `Mission abandoned`, every `MissionUpdate` send-failure row and every loader WARN. The persist count (351) equals accept + advance + complete + objective (351), so the save path is countable end to end.

## 2. Positive gaps

- **Rule 5 missing on every lifecycle row.** Each of these has `entity_id` + `entity_name` and nothing else (keys checked in ClickHouse): `Mission accepted` (`missions/lifecycle.rs:130`), the offer-refused WARN (`:88`), `Mission abandoned` (`:233`), `Mission step advanced` (`progression.rs:190`), `Objective completed` (`:331`), `Mission completed!` (`:449`), `Mission completed directly` (`:531`). The executor rows (`content/executor/mission.rs:25,57,129,244,297,336,393`) are the same, although `player_id` is a parameter there. The spans (`mission.accept`, `mission.abandon`) record `player_id`, but span fields don't reach log rows. A `player_id = N` log filter finds only the base's `Mission state persisted`.
- **No `event=` on any transition.** Only `mission_abandon_refused` has one (`lifecycle.rs:300`). Accept, advance, objective, complete and abandon are distinguishable only by message text.
- **Debug-formatted ids (NT rule):** `?old_step_id` (`progression.rs:195`) ships as `"Some(2113)"`, and `source_entity = ?ctx.source_entity_id` (`content-engine/src/chain/mod.rs:461,484`) ships as `"Some(EntityId(2))"`. Also `current_step_id = ?…` (`progression.rs:324,406`), `?prior_status` (`executor/mission.rs:190`) and `?current_step_id` on the `mission.persist` span (`base-methods/…/missions/mod.rs:97`).
- **`content.resolve` has no player.** A condition failure is the main ordering diagnostic ("the step wasn't active yet"), but it names neither the player nor the world.
- **No snapshot of the missions at login.** `InitPlayerState` logs only `saved_count` (`cell/…/player_init/mod.rs:135`), and the base logs only `count` (`missions/mod.rs:68`). Nothing says which step a player was on when they logged in. The two restore WARNs (`mission_restore.rs:75,115`) carry no player identity at all.
- **The stall row is thin.** `step_stalled` (`playtest_friction_watch/mod.rs:245`) has `entity_id`, mission, step and age. It lacks `player_id`/`account_id` (T6 needs these too), `world`, position, the open objectives, and any sign of whether the player tried anything.
- **`Mission state persisted` (DEBUG, `missions/mod.rs:203`)** omits `current_step_id`, so the step that was saved can't be read off the row.

## 3. Negative gaps

- **Silent client-frame drops (Pattern A).** `let _ = tx.send(EntityMethodCall{ON_MISSION/STEP/OBJECTIVE_UPDATE})` at `lifecycle.rs:153,165,180,288` and `progression.rs:218,231,367,421,440,562,576,590`. Only `progression.rs:154` (the old-objective frame) WARNs. A dropped frame leaves the journal out of step with the server until relog, and nothing logs it.
- **Silent early returns:** `lifecycle.rs:118-121` and `:223-226`, `progression.rs:94-97` and `:482-485` (`complete_mission_direct` with the entity gone: the persist then WARNs from `persist.rs:106`, but the cause is lost).
- **Deferred chain actions dropped silently.** `space_manager/entities.rs:144` (destroy) and `:451` (disconnect) do `pending_content_actions.remove(&entity_id)`. A delayed `advance_step`, `grant_item` or `display_dialog` scheduled before a relog or teleport just disappears.
- **`player_id` 0 fallbacks.** `cell-methods/…/interaction/dialog.rs:106-109` and `interact.rs:170-173,211-214` use `.unwrap_or(0)` and fire chains anyway. A chain that persists then sends `MissionUpdate{player_id: 0}`, and the base logs a bare DB error.
- **Short-args drops with no row:** `dialog.rs:19-21` (`dialogButtonChoice`) and `interact.rs:18-20`.
- **Fail-closed conditions are invisible.** `conditions/mod.rs:384,400,425` ("the firing dispatcher did not populate world_id/live_tags/shown_tutorials") are DEBUG in `cimmeria_content_engine`, which has no OTEL row. A dispatcher wiring bug leaves the chain dead with nothing in SigNoz.
- **The base read path** returns an empty list with no row when there is no pool (`missions/mod.rs:13-16`). On a query error (`:76-83`) the ERROR has no `reason` and no `account_id`, and the player then loads as if they had no missions (see §6.4).

## 4. Noise

- `fire_cover_*: no chains matched`: 3,348 DEBUG rows (45% of this system's volume), emitted even when no chain listens for that trigger type.
- `add_dialog_set` writes three INFO rows per action (`executor/dialog/mod.rs:191,204,226`): 321 rows for 107 actions.
- Nothing fires per tick at INFO+. `step_stalled` fires once per (mission, step). Most of its rows are lab runs, so it can't alert until T6 lands.

## 5. Seams (two hops)

| Hand-off | Sides | Can SigNoz say which side dropped it? |
|---|---|---|
| Click → content: `interact` (cell-methods) → `fire_interact_tag/template` → `handle_interact` → `fire_dialog_open` | INFO `interact`, `fire_*: matched/no chains`, `interact: target resolved` | Yes for a click that arrives. **No click arriving** (#1341) leaves nothing. The `interact` row lacks `player_id` |
| Dialog-set bind → AoI → client: `add_dialog_set` pushes per-player `InteractionType` only if the NPC is witnessed, otherwise defers to AoI create (`dialog/mod.rs:483-489`) | DEBUG `Sending per-player InteractionType` / `deferring…to AoI create` (2 rows) | The server side is complete. Whether the client applied it is T2's job (`unknown_entity`) |
| AoI hold flush → client bundle (two hops: missions → AoI → Mercury) | `Cinematic AoI hold: released flushed=N`, `aoi.introduce` DEBUG; client `client.mercury.unpack_fault` | Only by a hand join across services. T1/T2 close the AoI half. The mission half (which introduced entity a step needs) is missing (TG-MIS-03) |
| Dialog display → base → client → `dialogButtonChoice` | `Sending onDialogDisplay`, send-failure WARN, #479 gate WARN, `dialog_displaced` friction | Good. The server can't see a dialog the client failed to render (client side) |
| Cell → base persist: `MissionUpdate` | Cell ERROR on send failure (`persist.rs:122`); base DEBUG success, ERROR failure | Yes, joined on `player_id` + `mission_id`. The base error lacks `reason` and the step |
| Login → cell: `query_saved_missions` → `InitPlayerState` → `build_restored_missions` | INFO count; restore WARNs without identity | Partly. No per-player active-step snapshot (TG-MIS-05) |
| Content → items: `GrantItem` → base `grant_refused`/`grant_outcome_unknown` | Cell `Content: granting item`; base INFO/WARN | Only by time and player join. The base row has no chain or mission provenance, so a lost mission reward reads like any full bag |
| Content → abilities (`GrantAbility`); regions → content (hint, H52 replay) | refusal WARN/ERRORs; `region_dwell_no_hint`, `filtered_out` | Yes |
| Movement → friction: `player_tick` runs only on accepted movement packets (`cell/…/base_messages/movement.rs:109`) | none | A player standing still never gets a stall row. That's intended ("while the player keeps playing"), but a player stuck in a dialog goes unseen |

## 6. Adversarial

1. **#1341, Frost's corpse never created (players hit this).** *Today:* `fire_player_loaded: matched` → `Mission accepted` 622 / step 2113 (no player id) → three `add_dialog_set` rows binding template 14 → `aoi.introduce` DEBUG for Frost inside the hold flush → silence. After at least 300 s of movement comes `step_stalled` 622/2113 with only `entity_id`. The cause is a `client.mercury.unpack_fault` in `cimmeria-client`, tied by hand on time. The stall reads the same as a player who never looked. *Should:* at step activation, one `mission.step_anchor` row saying "anchor `ArmYourself_FrostBody` = entity N, in witness set, 6 m away". Then an `anchor_unused` WARN saying "player within 10 m of entity N for 30 s, 0 interacts on it" (the signature of an uncreated object, since nobody stands next to a quest corpse without clicking it). Then a stall row with identity, world and `interacts_since_step`. Plus T1/T2 on the AoI side.
2. **Frost's Letter step 4037 has no advancer** (13 of the 40 stalls; no chain advances 4037→4038). *Today:* a stall per player, indistinguishable from a grind step. *Should:* the anchor row reports `advancing_chains = 0, anchors = 0` at activation. That makes "unreachable by construction" a single query, before anyone stalls.
3. **Deferred action lost on relog.** A chain schedules `display_dialog +3000ms`, then the player relogs or a chain teleports them. *Today:* nothing; `entities.rs:144/451` drop it. *Should:* WARN `deferred_actions_dropped` with the chain ids and action kinds.
4. **Saved-missions query fails at login.** *Today:* one ERROR without a `reason`. The cell then hydrates zero missions, `fire_player_loaded` re-accepts the starter mission, and the offer guard has no prior row to refuse. The UPSERT writes `status=1` over a completed row, which is the #411 corruption by another route. The next rows look like a normal fresh start. *Should:* a `missions_restored` row with `saved_count=0`, plus a load-failure flag on `InitPlayerState`. The behaviour fix (don't persist missions for a session whose load failed) needs a domain decision and is not a telemetry packet.
5. **Chain skipped because its step wasn't active yet** (the ordering trap). *Today:* `content.resolve condition_failed` names the chain and the failed condition, but `source_entity="Some(EntityId(2))"` defeats an `entity_id = 2` filter and no player id is present. *Should:* numeric `entity_id`, plus `player_id` and `world_id`.

## Candidate packets

| ID | Title | Sev | Files |
|---|---|---|---|
| TG-MIS-01 | Identity and `event=` on mission lifecycle rows | high | 2 |
| TG-MIS-02 | Identity and context on `playtest.friction` rows | high | 2 |
| TG-MIS-03 | `mission.step_anchor` row at step activation | high | 3 |
| TG-MIS-04 | `anchor_unused` friction signal | med | 3 |
| TG-MIS-05 | `missions_restored` login snapshot | med | 1 |
| TG-MIS-06 | Executor mission rows carry `player_id` | med | 1 |
| TG-MIS-07 | Log dropped deferred content actions | med | 1 |
| TG-MIS-08 | WARN on dropped mission client frames | med | 3 |
| TG-MIS-09 | Numeric ids on `content.resolve` | med | 1 |
| TG-MIS-10 | Malformed-args and `player_id` 0 rows on interact and dialog choice | low | 2 |
| TG-MIS-11 | Base persist rows: step, `reason`, error | low | 1 |
| TG-MIS-12 | Demote no-listener cover rows; collapse `add_dialog_set` | low | 2 |
| TG-MIS-13 | OTEL row for content-engine conditions; drop dead `mission` target | low | 2 |

**TG-MIS-01: Identity and `event=` on mission lifecycle rows.** High. `cell-content/src/cell/missions/lifecycle.rs` (`accept_mission`, `abandon_mission`) and `progression.rs` (`advance_step`, `complete_objective`, `complete_mission_direct`). Resolve `space_mgr.player_identity(entity_id)` (or the identity from the already-borrowed entity) and add `account_id`, `player_id`, `player_name` and `world` to every INFO/WARN row. Add `event` = `mission_accepted`, `mission_accept_refused`, `mission_abandoned`, `mission_step_advanced`, `objective_completed`, `mission_completed` (with `path = auto|direct`). Fix `?old_step_id` → `old_step_id = old_step_id` and `current_step_id = ?` → the bare `Option<i32>`. Turn the silent `None => return` at `lifecycle.rs:118`, `progression.rs:94` and `:482` into WARN `reason = "entity_missing"`. Test: a LogCapture unit test in each file's `tests` module asserting `event` and a numeric `player_id` on accept, advance and complete. It fails when the fields are reverted. No OTEL change.

**TG-MIS-02: Identity and context on `playtest.friction` rows.** High. `cell-world/src/cell/playtest_friction_watch/mod.rs` (`PlayerWatch`, `player_tick`, `emit`, `objectives_never_completed`) and `cell-world/src/cell/playtest_friction.rs` (the four `emit` sites). Cache the `PlayerIdentity` and world name on the watch in `player_tick`, as `entity_name` already is. Emit `account_id`, `player_id`, `player_name` and `world` on every friction row. On `step_stalled` also emit `x`, `z` and `open_objectives` (comma-joined ids of the current step's incomplete objectives). Test: extend `playtest_friction_watch/tests.rs` with a LogCapture test that drives `player_tick` past `STEP_STALL_AFTER` using a test clock or the existing `evaluate` seam, then asserts `player_id` and `world` on the WARN.

**TG-MIS-03: `mission.step_anchor` row at step activation.** High. `content-engine/src/chain/mod.rs`: add `ChainEngine::step_anchors(mission_id, step_id) -> StepAnchors { advancers: usize, tags: Vec<String>, templates: Vec<String>, dialog_ids: Vec<i32> }`. It covers enabled chains that have a `StepStatus{mission,step,Eq,Active}` condition, keyed by `OnInteractTag`, `OnInteractTemplate` and `DialogOpen` triggers. `advancers` counts those chains whose actions include `AdvanceStep`/`CompleteMission`/`CompleteObjective` for that mission. `cell-content/src/cell/content/event_dispatch/step_activation/mod.rs`: after activation, resolve tags and templates to entity ids in the player's space. Resolve `dialog_ids` through the player's `available_interactions` (template → dialog), then to entities. Emit one INFO on target `mission.step_anchor` with `event = "step_anchor"`, identity, mission and step ids and names, `advancers`, `anchors` (count), and for up to 5 anchors `anchor_ids`, `anchor_tags`, `anchor_in_witness_set`, `anchor_distance`. Call it from both the accept and the `advance_step` paths (they already call `fire_step_activation_regions`). Test: unit-test `step_anchors` in `chain/tests.rs` (fails without the method), and add a LogCapture test in step_activation's tests asserting `advancers = 0` for a step no chain gates. Add `mission.step_anchor=info` to `filters.rs` (the target-scan test requires it).

**TG-MIS-04: `anchor_unused` friction signal.** Medium; depends on 03. `playtest_friction_watch/mod.rs`: store the anchor entity ids per (mission, step) on the watch, through a new `set_step_anchors(entity_id, mission_id, step_id, ids)` called from step_activation (03). In `evaluate`, accumulate time within 10 m of any anchor. `cell-methods/…/interaction/interact.rs`: call a new `friction_watch::interacted(entity_id, target_id)` at the top of `handle_interact`. Emit WARN `signal = "anchor_unused"`, `reason = "anchor_near_never_interacted"` once per step when the dwell is at least 30 s and there were 0 interacts on any anchor. Fields: identity, mission, step, `anchor_id`, `anchor_tag`, `dwell_secs`, `interacts_since_step`. Test: a pure `evaluate` unit test (dwell, no interact → one row; one interact → none). A second test checks independence from another entity's watch.

**TG-MIS-05: `missions_restored` login snapshot.** Medium. `cell/src/cell/service/base_messages/player_init/mod.rs` (after `build_restored_missions`): one INFO `event = "missions_restored"` with identity, `world`, `saved_count`, `active` (`"622:2113,1360:4037"`, hidden ones marked `h`), `completed_count` and `failed_count`. Test: LogCapture on the existing `InitPlayerState` handler test fixture asserting `active`.

**TG-MIS-06: Executor mission rows carry `player_id`.** Medium. `cell-content/src/cell/content/executor/mission.rs`, all nine rows: add `player_id` (the parameter), `account_id` and `player_name` from `player_identity`. Add `world` to the `No mission_defs entry` WARN (`:105`), which today lacks even `entity_id`. Test: LogCapture in `offer_guard_tests` asserting `player_id` on `Content: accepting mission`.

**TG-MIS-07: Log dropped deferred content actions.** Medium. `cell-world/src/cell/space_manager/entities.rs:144,451`: when `remove` returns a non-empty queue, WARN `event = "deferred_actions_dropped"`, `reason = "destroy_entity" | "disconnect"`, identity, `dropped`, `chain_ids`, `action_kinds` (via `player_journal::action_kind`). Test: add LogCapture to `deferred_content_actions.rs::destroy_entity_drops_pending_content_actions` and `disconnect_entity_drops_pending_content_actions`.

**TG-MIS-08: WARN on dropped mission client frames.** Medium. `cell-content/src/cell/missions/mod.rs`: add `send_mission_frame(tx, entity_id, method_index, args, frame: &'static str, mission_id)`, which WARNs `reason = "mission_frame_send_failed"` with `frame` and identity. Replace the `let _ =` sends in `lifecycle.rs` and `progression.rs` (sites in §3). Test: LogCapture with a closed `mpsc` receiver on `accept_mission`, asserting the WARN with `frame = "onMissionUpdate"`.

**TG-MIS-09: Numeric ids on `content.resolve`.** Medium. `content-engine/src/chain/mod.rs:456-490`: replace `source_entity = ?ctx.source_entity_id` with `entity_id = ctx.source_entity_id.map(|e| e.0)` and add `world_id = ctx.world_id`. Test: LogCapture in `chain/tests.rs` that resolves a chain with a failing condition and asserts `entity_id` is a numeric field, not a string.

**TG-MIS-10: Malformed-args and `player_id` 0 rows.** Low. `cell-methods/…/interaction/dialog.rs:19,106` and `interact.rs:18,170,211`: a short payload → WARN `reason = "malformed_args"` with `args_len`. A missing `player_id` → WARN `reason = "no_player_id"` before the chain fires (log only; no behaviour change). Test: LogCapture in `dialog_choice_gate_tests.rs` with a 4-byte payload.

**TG-MIS-11: Base persist rows.** Low. `base-methods/…/missions/mod.rs`: add `current_step_id` and its name to `Mission state persisted`. Make the failure rows structured: `reason = "upsert_failed" | "delete_failed" | "query_failed"`, `error = %e`, `status`, `current_step_id`. Give the no-pool read path a DEBUG `reason = "no_database"`. Test: an existing live-DB test in `missions/tests.rs` plus a LogCapture assertion on `current_step_id`, or a unit test with `db_pool = None` asserting the read's DEBUG row.

**TG-MIS-12: Demote no-listener cover rows; collapse `add_dialog_set`.** Low. `event_dispatch/cover.rs`: emit the `no chains matched` DEBUG only when the engine has a chain for that trigger type, otherwise TRACE. `executor/dialog/mod.rs:191-247`: merge the three INFO rows into one `Content: adding dialog set`, keeping all fields. Test: LogCapture asserts exactly one INFO per `add_dialog_set`, and no DEBUG for a cover event with an empty engine.

**TG-MIS-13: OTEL row for content-engine conditions; drop the dead `mission` target.** Low. `needs-domain-agent` (an observability decision for the coordinator). `server/src/logging/filters.rs`: add `cimmeria_content_engine::conditions=debug` and remove the dead `mission=info`. In `parity_tests/crate_rows.rs`, move content-engine out of `NO_OWN_ROW`. Test: the existing parity and crate-rows guards, plus one fail-closed condition event reaching the server index.
