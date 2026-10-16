# DW-25A-E: slash commands 1–27

The headings are the first 27 `[[command]]` names in `data/client-patches/012-gm-slash-commands/commands.toml`, not an inventory of stock client commands. `params` are required; `optional` may be omitted. Types and labels below reproduce that TOML, even where they disagree with a server method. The patch's `Event_SlashCmd_*` label is a **client event name**, not proof that the client sends a particular cell method. `crates/lab/src/workbench/slash_catalog.rs::method_candidate` calls its matches candidates and sets `binding_verified: false`. `docs/protocol/cell-method-dispatch-table.md` and `entities/defs/SGWGmPlayer.def` establish server method signatures/indices independently. Do not turn a candidate into an executable binding based on spelling. The stock files and native submission/quoting proof are absent (DW-24A worknote); every command in this slice is **Unsupported for palette execution pending DW-24B**, even when the server handler works. Additional mismatches below block individual forms until reconciled.

The patch's `access = "p"` is a client parser role filter; `data/client-patches/README.md` documents its unusual mask calculation. It does **not** grant authority. `crates/cell/src/cell/dispatch/router.rs::dispatch_cell_method` calls `crates/cell-world/src/cell/dispatch/gm_gate.rs::enforce_gm_gate` before method routing: every SGWGmPlayer index 109+ requires the server-sourced `CellEntity.access_level >= GameMaster`, with missing entities refused. The palette must recheck a live actor/instance; a browser's role, target or successful text submission is not acceptance. For implemented methods, `crates/cell-console/src/cell/console/gm/mod.rs::dispatch` is the handler roster. A fall-through index is authorized but **unimplemented**, not success.

For mission readback, capture the authenticated caller's `player_id` and inspect that exact player's `sgw_mission` rows via read-only `server_db_query`, alongside the current mission X-ray if a trusted session binding exists. `crates/base-methods/src/base/world_entry/methods/missions/mod.rs::handle_mission_update` UPSERTs current mission state, but an abandoned non-repeat mission deletes its row; `MissionUpdate` is asynchronous and can fail or lack a DB pool. Feedback and client UI update alone do not prove persistence. `crates/cell-console/src/cell/console/gm/tests/missions.rs` covers assign, advance, abandon and their persistence paths. For grants, read exact `sgw_player` columns or `sgw_inventory` rows for the captured character after base completion, never infer a commit from the cell's queue send. `server_db_query` is read-only and row-capped (`crates/lab-mcp/src/tools/db.rs`). Compensating a persistent grant requires a separate approved operation; no general undo is claimed.

## /gmmissionassign

- **Sources:** TOML `Event_SlashCmd_MissionAssign`; method candidate 109 `gmMissionAssign` in the protocol table; `gm/missions.rs::handle_mission_assign`, `missions::accept_mission` and `send_mission_update`.
- **Arguments:** Required `String MissionDesignId`, `Boolean DisplayAcceptWindow` (TOML order). Server method declares WSTRING positive numeric mission ID and UINT8 popup; the handler reads the ID and does not inspect the popup byte.
- **Target/scope:** Calling GM's own player mission tracker; no selected-player target.
- **Preconditions and authority:** Server GM gate; live player and loaded mission definition with first step/objectives; already-held or non-offerable mission refuses.
- **Effects and impact:** Persistent mission accept is queued for DB, mission update sent, `mission_accepted` and first-step region events fired; can trigger content chains. Popup is a client UI hint.
- **Verification:** Compare caller's `sgw_mission` row (`status`, `current_step_id`, objective IDs) before/after; cell feedback and client mission UI are separate observations. `tests/missions.rs::gm_mission_assign_persists_the_mission` and `gm_mission_assign_fires_mission_accepted` cover the server path.
- **Compensation:** Manual mission clear only if currently active and still safe; fired chains are not reversed.
- **Availability:** Unsupported until native event-to-method, boolean encoding and submission are proved; server handler exists.

## /gmmissionclear

- **Sources:** TOML `Event_SlashCmd_MissionClear`; candidate 110 `gmMissionClear`; `gm/missions.rs::handle_mission_clear`, `missions::abandon_mission`, `tests/missions.rs::gm_mission_clear_saves_the_abandon`.
- **Arguments:** Required `String DesignID`; server parses a positive numeric WSTRING mission ID.
- **Target/scope:** Calling GM's own **active** mission; not a selected player or arbitrary history row.
- **Preconditions and authority:** Server GM gate, live player and active held mission; inactive/completed mission is a no-op with feedback.
- **Effects and impact:** Persistent abandon and client removal, followed by `mission_abandoned` content event; `sgw_mission` row is deleted when status becomes not-active with zero repeats, otherwise updated.
- **Verification:** Compare the exact caller/mission `sgw_mission` row and active mission state after base processing; do not equate absence of a row with a DB failure without feedback/log context.
- **Compensation:** Reassign manually if offerable; prior progress, repeats and content side effects are not restored.
- **Availability:** Unsupported until native binding/submission proof; server handler exists.

## /gmmissionclearactive

- **Sources:** TOML `Event_SlashCmd_MissionClearActive`; protocol candidate 111 `gmMissionClearActive` is `ADAPT`; no arm for 111 in `gm/mod.rs::dispatch`.
- **Arguments:** None.
- **Target/scope:** Client event implies caller's active missions; server effect is absent.
- **Preconditions and authority:** Server GM gate would still apply to method 111, but no implementation follows it.
- **Effects and impact:** Unknown original-client intent; Cimmeria performs no clear through this index.
- **Verification:** No accepted-effect readback; inventory active `sgw_mission` rows only to establish baseline, not success.
- **Compensation:** None; no implemented mutation.
- **Availability:** Unsupported: server handler absent, as well as native binding proof. Do not substitute repeated `/gmmissionclear` calls.

## /gmmissionclearhistory

- **Sources:** TOML `Event_SlashCmd_MissionClearHistory`; protocol candidate 112 `NEW`; no arm in `gm/mod.rs::dispatch`.
- **Arguments:** None.
- **Target/scope:** History scope unimplemented; no selected target.
- **Preconditions and authority:** Server GM gate would apply, but no history-clear primitive is present.
- **Effects and impact:** No implemented mutation; original-client semantics unknown.
- **Verification:** No effect recipe; `sgw_mission` history read is diagnostic only.
- **Compensation:** None.
- **Availability:** Unsupported: no handler or native binding; do not equate with abandon or delete DB rows.

## /gmmissionlist

- **Sources:** TOML `Event_SlashCmd_MissionList`; candidate 113; `gm/missions.rs::handle_mission_list`.
- **Arguments:** None.
- **Target/scope:** Calling GM's active missions only.
- **Preconditions and authority:** Server GM gate and live caller entity.
- **Effects and impact:** Read-only cell tracker report through GM feedback, not a database scan.
- **Verification:** Capture `gmMissionList` feedback and compare active mission IDs/status in mission X-ray for the same caller; no DB write expected.
- **Compensation:** None.
- **Availability:** Unsupported until native binding/submission proof; server read handler exists.

## /gmmissionlistfull

- **Sources:** TOML `Event_SlashCmd_MissionListFull`; candidate 114; `gm/missions.rs::handle_mission_list_full`.
- **Arguments:** None.
- **Target/scope:** Calling GM's loaded missions, including completed/hidden.
- **Preconditions and authority:** Server GM gate and live caller entity.
- **Effects and impact:** Read-only feedback from cell tracker; not all missions in the resource catalog.
- **Verification:** Compare feedback mission IDs/status to mission X-ray or caller's `sgw_mission` rows, allowing explicit in-memory versus DB freshness differences.
- **Compensation:** None.
- **Availability:** Unsupported until native binding/submission proof; server read handler exists.

## /gmmissiondetails

- **Sources:** TOML `Event_SlashCmd_MissionDetails`; candidate 115; `gm/missions.rs::handle_mission_details`.
- **Arguments:** Required `String MissionDesignId`; server requires positive numeric WSTRING ID.
- **Target/scope:** One mission on the calling GM, not a catalog definition or selected player.
- **Preconditions and authority:** Server GM gate, live caller and mission in their tracker; otherwise “not found on you.”
- **Effects and impact:** Read-only feedback with mission ID, status, current step and completed/total objective counts (`gm/missions.rs::mission_line`); it does not print repeat count.
- **Verification:** Capture feedback for the requested numeric ID and compare the same mission in mission X-ray/`sgw_mission`. Read `sgw_mission.repeats` separately if needed; it is not in this command's feedback. A missing live mission yields an explicit not-found result.
- **Compensation:** None.
- **Availability:** Unsupported until native binding/submission proof; server read handler exists.

## /gmmissionadvance

- **Sources:** TOML `Event_SlashCmd_MissionAdvance`; candidate 116; `gm/missions.rs::handle_mission_advance`, `missions::advance_step`, `tests/missions.rs::gm_mission_advance_persists_the_new_step`.
- **Arguments:** Required `String MissionDesignId`, `Integer StepToAdvanceTo`; server requires positive numeric ID and positive step belonging to that mission.
- **Target/scope:** Calling GM's held mission.
- **Preconditions and authority:** Server GM gate, loaded mission/step and active state; a step of another mission is refused by `step_missions` ownership check.
- **Effects and impact:** Persistent step/objective transition, client update and step-region content replay; `send_mission_update` queues DB state.
- **Verification:** Compare `sgw_mission.current_step_id` and objective ID/status fields for caller/mission after base processing; feedback text alone can be emitted even when the transition returned false, so require readback.
- **Compensation:** No automatic reset; another advance is a new transition, not rollback of fired chains.
- **Availability:** Unsupported until native binding/submission proof; server handler exists.

## /gmmissionreset

- **Sources:** TOML `Event_SlashCmd_MissionReset`; protocol candidate 117 `NEW`; no `gm/mod.rs::dispatch` arm.
- **Arguments:** Required `String MissionDesignId`, `Integer StepToAdvanceTo`.
- **Target/scope:** No implemented server target/effect; apparent mission and step labels do not establish reset semantics.
- **Preconditions and authority:** GM gate would apply to 117; no reset primitive is wired.
- **Effects and impact:** Unknown original-client effect; Cimmeria does not revert mission state.
- **Verification:** No accepted-effect recipe; baseline `sgw_mission` read cannot prove reset.
- **Compensation:** None.
- **Availability:** Unsupported: handler absent and native binding unproved; never use `/gmmissionadvance` as a reset.

## /gmmissioncomplete

- **Sources:** TOML `Event_SlashCmd_MissionComplete`; protocol candidate 118 `ADAPT`; no 118 arm in `gm/mod.rs::dispatch`; protocol table notes `complete_mission_direct` does not fire rewards.
- **Arguments:** Required `String MissionDesignId`, `Boolean TurnIn`.
- **Target/scope:** No implemented slash target/effect; do not assume `TurnIn` pays rewards.
- **Preconditions and authority:** GM gate would apply; completion/turn-in handler not wired.
- **Effects and impact:** No current mutation through this index; reward semantics unproved.
- **Verification:** No accepted-effect recipe; `sgw_mission` state and reward balances would be separate future reads.
- **Compensation:** None.
- **Availability:** Unsupported: handler absent and native binding unproved.

## /gmmissionsetavailable

- **Sources:** TOML `Event_SlashCmd_MissionSetAvailable`; protocol candidate 119 `NEW`; no 119 dispatch arm; protocol table says availability is not entity state.
- **Arguments:** Required `String MissionDesignId`.
- **Target/scope:** No implemented server target/effect.
- **Preconditions and authority:** GM gate would apply; no availability writer.
- **Effects and impact:** Unknown original-client semantics; no Cimmeria mutation.
- **Verification:** No accepted-effect recipe; mission offer visibility alone is not proof of this command.
- **Compensation:** None.
- **Availability:** Unsupported: handler absent and native binding unproved.

## /gmmissionabandon

- **Sources:** TOML `Event_SlashCmd_MissionAbandon`; candidate 120 `gmMissionAbandon`; `gm/mod.rs::dispatch` routes it to `gm/missions.rs::handle_mission_clear` and `gm/mission_abandoned_tests.rs` covers the alias.
- **Arguments:** Required `String MissionDesignId`; server requires positive numeric WSTRING mission ID.
- **Target/scope:** Calling GM's active mission only.
- **Preconditions and authority:** Server GM gate and active held mission; inactive/completed row is not cleared.
- **Effects and impact:** Same persistent abandon/removal and `mission_abandoned` event as `/gmmissionclear`.
- **Verification:** Compare exact caller/mission `sgw_mission` before/after, with deletion versus repeat-preserving update distinguished; observe client removal separately.
- **Compensation:** Manual reassign if offerable; progress and fired content effects are not undone.
- **Availability:** Unsupported until native event binding/submission is proved; server alias exists.

## /gmshowtargetlocation

- **Sources:** TOML `Event_SlashCmd_ShowTargetLocation`; candidate 121; `gm/query.rs::handle_show_target_location` and `subject_or_self`.
- **Arguments:** None.
- **Target/scope:** Current target only if in caller's space and view; otherwise caller.
- **Preconditions and authority:** Server GM gate and live caller; no cross-space target read.
- **Effects and impact:** Read-only cell position feedback.
- **Verification:** Compare feedback subject ID/coordinates to `server_entity_get` of the same entity and instance at nearby time; moving subjects can change between reads.
- **Compensation:** None.
- **Availability:** Unsupported until native binding/submission proof; server read handler exists.

## /gmlistabilities

- **Sources:** TOML `Event_SlashCmd_ListAbilities`; protocol candidate 123 is **`listAbilities`**, not `gmListAbilities`; `gm/mod.rs::dispatch` routes 123 to `gm/query.rs::handle_list_abilities`.
- **Arguments:** None.
- **Target/scope:** In-view current target in caller's space, otherwise caller.
- **Preconditions and authority:** Server GM gate still applies to index 123, despite method name lacking `gm`.
- **Effects and impact:** Read-only known-ability-ID feedback from cell entity.
- **Verification:** Capture feedback; for a player, compare persisted `sgw_player.abilities` after state settles, noting live/DB freshness. `server_ability_state` reports active casts/effects but has no known-ability list, so an NPC's IDs lack an independent readback here and remain unverified.
- **Compensation:** None.
- **Availability:** Unsupported: event-to-`listAbilities` binding is not established by its similar name; native path unproved. Server handler exists.

## /gmshowflag

- **Sources:** TOML `Event_SlashCmd_ShowFlag`; candidate 125; `gm/query.rs::handle_show_flag`.
- **Arguments:** Optional `Integer flagId` in TOML; server requires INT32 bit index 0–31 and refuses missing bytes.
- **Target/scope:** In-view target in caller's space, otherwise caller.
- **Preconditions and authority:** Server GM gate; valid flag ID required. TOML's omitted form does not satisfy the handler.
- **Effects and impact:** Read-only state-field bit feedback.
- **Verification:** Compare feedback bit and raw `state_field` with a fresh same-entity server snapshot if field exposed; otherwise mark independent verification unavailable.
- **Compensation:** None.
- **Availability:** Unsupported: native binding/optional omission unproved; absent-argument form must be disabled even after transport proof unless reconciled.

## /gmlistinteractions

- **Sources:** TOML `Event_SlashCmd_ListInteractions`; protocol candidate 126 `ADAPT`; no 126 arm in `gm/mod.rs::dispatch`.
- **Arguments:** None.
- **Target/scope:** Interaction target/scope not implemented for this method.
- **Preconditions and authority:** GM gate would apply; no feedback handler.
- **Effects and impact:** No implemented effect; do not infer a list from current target's separate interaction API.
- **Verification:** No accepted-effect recipe.
- **Compensation:** None.
- **Availability:** Unsupported: server handler absent and native binding unproved.

## /gmgetmobattribute

- **Sources:** TOML `Event_SlashCmd_GetMobAttribute`; protocol method 127 `gmGetMobAttribute(INT32 target, WSTRING attr)`; `gm/query.rs::handle_get_mob_attribute` reads both.
- **Arguments:** Required `String attribute` **only** in TOML. Server wire method requires an additional leading INT32 target ID; this is a signature mismatch.
- **Target/scope:** Server handler reads explicit target ID in caller's space and a fixed attribute name; the patch declares no target argument.
- **Preconditions and authority:** Server GM gate, valid in-space target and supported attribute; cannot synthesize the missing wire argument from a UI selection without client-binding evidence.
- **Effects and impact:** Read-only server attribute feedback if correctly invoked by some proven path; patch event's actual method binding unknown.
- **Verification:** No safe patch-form execution recipe; for a separately proven method call, compare named attribute feedback with `server_entity_get` same target.
- **Compensation:** None.
- **Availability:** Unsupported due TOML/wire arity mismatch plus native binding gap.

## /gmshowmobcount

- **Sources:** TOML `Event_SlashCmd_ShowMobCount`; candidate 128; `gm/query.rs::handle_show_mob_count`.
- **Arguments:** Required `Integer SpaceID`; server INT32 `0` means caller's current space, positive ID selects that space.
- **Target/scope:** Counts NPC entities in the requested loaded space, not all shard or DB spawns.
- **Preconditions and authority:** Server GM gate; valid space ID or caller space.
- **Effects and impact:** Read-only cell count feedback.
- **Verification:** Compare to `server_entity_query` in the exact space, counting `is_player=false` in a complete, uncapped result. The query has no non-player-only filter, so a capped result cannot independently verify this count; `total_matched` includes players.
- **Compensation:** None.
- **Availability:** Unsupported until native binding/submission proof; server read handler exists.

## /gmshowip

- **Sources:** TOML `Event_SlashCmd_ShowIP`; protocol candidate 129 `ADAPT` expects INT32 target; no 129 dispatch arm.
- **Arguments:** None in TOML, while documented method takes a target ID.
- **Target/scope:** Unknown; no entity-to-address handler implemented.
- **Preconditions and authority:** GM gate would apply; any future address display must be authorized and redacted in exported evidence.
- **Effects and impact:** No implemented read; network address would be sensitive if a later handler supplied one.
- **Verification:** No accepted-effect recipe and no IP export.
- **Compensation:** None.
- **Availability:** Unsupported: absent handler, argument mismatch and native binding gap.

## /gmshowinventory

- **Sources:** TOML `Event_SlashCmd_ShowInventory`; protocol candidate 130 `NEW` expects INT32 target; no 130 dispatch arm; inventory is base-owned.
- **Arguments:** None in TOML; method signature includes target ID.
- **Target/scope:** Unknown for patch event; no server read handler.
- **Preconditions and authority:** GM gate would apply; no base round-trip or target authorization is wired.
- **Effects and impact:** No implemented read.
- **Verification:** No accepted-effect recipe; read-only `sgw_inventory` query for an explicitly authorized player is a separate lab operation, not command success.
- **Compensation:** None.
- **Availability:** Unsupported: handler absent, argument mismatch and native binding gap.

## /gmshowplayer

- **Sources:** TOML `Event_SlashCmd_ShowPlayer`; protocol candidate 131 expects INT32 target; `gm/query.rs::handle_show_player` accepts a missing/zero value as target-or-self fallback.
- **Arguments:** None in TOML; server signature contains INT32 target ID, although handler has a missing-byte fallback. Do not assume the client emits zero or omits bytes.
- **Target/scope:** Server handler's explicit ID may inspect an entity in caller's space; fallback uses in-view target or caller.
- **Preconditions and authority:** Server GM gate; target space check. Patch event's wire encoding remains unknown.
- **Effects and impact:** Read-only entity summary feedback if handler is actually reached.
- **Verification:** Compare feedback subject ID/level/health/position to `server_entity_get` for that exact entity and instance; timing differences remain possible.
- **Compensation:** None.
- **Availability:** Unsupported: client event-to-method and omitted-target encoding unproved, despite server fallback.

## /gmgivexp

- **Sources:** TOML `Event_SlashCmd_GiveXp`; candidate 132; `gm/give.rs::handle_give_xp` → `CellToBaseMsg::GrantXP` → `base-methods/.../progression/mod.rs::handle_grant_xp`; `gm/tests/give.rs` covers positive/refused input.
- **Arguments:** Required `Integer XpAmount`; server INT32 must be positive.
- **Target/scope:** Calling GM's own character, not selection.
- **Preconditions and authority:** Server GM gate and caller player ID; base requires current connected character and DB row.
- **Effects and impact:** Persistent XP and possible level/ASP changes; base sends definitive GM feedback after its DB update and client progression update.
- **Verification:** Compare exact caller's `sgw_player.exp`, `level`, `training_points` and `applied_science_points` before/after, plus base feedback; use level transition rules, not assume XP delta alone proves all secondary effects.
- **Compensation:** No automatic inverse; removing XP/levels would require separate approved operation and effects may already have fired.
- **Availability:** Unsupported until native binding/submission proof; server grant handler exists.

## /gmgiveitem

- **Sources:** TOML `Event_SlashCmd_GiveItem`; candidate 133 (pcap-anchored server index); `gm/give.rs::handle_give_item` → `GrantItem` base inventory sink; `gm/tests/give.rs` covers clamp/refusals.
- **Arguments:** Required `String DesignId`, `Integer Quantity`; server resolves **positive numeric** design IDs only, requires quantity ≥1 and clamps to 1000.
- **Target/scope:** Calling GM's own character inventory; selected entity ignored.
- **Preconditions and authority:** Server GM gate, live player ID, valid item template and base inventory capacity/container rules; internal item name is refused.
- **Effects and impact:** Persistent inventory grant with possible placement in a non-main container; base sends definitive feedback after write and client item update.
- **Verification:** Compare exact character's `sgw_inventory` instance/count/container rows before/after plus base feedback; do not assume the requested main bag is final placement.
- **Compensation:** No safe automatic inverse: remove requires a resulting **instance ID** and current quantity; manual removal only after fresh read and approval.
- **Availability:** Unsupported until native binding/submission proof; server handler exists.

## /gmgivecash

- **Sources:** TOML `Event_SlashCmd_GiveNaqahdah` (event name differs from slash word); protocol candidate 134 `gmGiveCash`, mapped explicitly in `slash_catalog.rs`; `gm/give.rs::handle_give_cash` → `GrantCash` → `base-methods/.../progression/mod.rs::handle_grant_cash`.
- **Arguments:** Required `Integer Amount`; server requires positive INT32.
- **Target/scope:** Calling GM's own character.
- **Preconditions and authority:** Server GM gate, player ID and matching persistent DB row; no selected target.
- **Effects and impact:** Persistent additive `sgw_player.naquadah` update; base feedback follows commit and cash client update.
- **Verification:** Compare exact caller's `sgw_player.naquadah` before/after and base feedback; cell queue send is not acceptance.
- **Compensation:** No inverse slash command; any manual debit is a separately approved action.
- **Availability:** Unsupported: documented semantic alias is a candidate, not a proven client event binding/submission.

## /gmremoveitem

- **Sources:** TOML `Event_SlashCmd_GMDeleteItem`; protocol index 135 `gmRemoveItem(ItemID, INT16 qty)`; `gm/give.rs::handle_remove_item` → `RemoveInventoryItem`.
- **Arguments:** Required `String ItemName`, `Integer Quantity` in TOML. Server expects **INT32 inventory instance ID**, then **INT16 quantity**; this is a type/width and identity mismatch, not an item-design-name remove.
- **Target/scope:** Server method removes from calling GM's own inventory, not another player; patch event's real target encoding unknown.
- **Preconditions and authority:** Server GM gate, valid positive instance ID/quantity and base inventory ownership/vault rules.
- **Effects and impact:** Potential persistent item count/removal if a separately proven correctly encoded method is invoked; patch form cannot safely imply that effect.
- **Verification:** No safe patch-form effect recipe; for a proven direct method, compare exact `sgw_inventory` instance/count/container before/after and base feedback.
- **Compensation:** No generic inverse: grant by design ID may create a different instance and lose instance state.
- **Availability:** Unsupported due patch/server signature mismatch and native binding gap. Never coerce `ItemName` into an instance ID silently.

## /gmgiveability

- **Sources:** TOML `Event_SlashCmd_GiveAbility`; candidate 136; `gm/abilities.rs::handle_give_ability` → `give_ability::plan_grant`/`GmGrantAbility` → `base-methods/.../progression/grant_ability.rs`.
- **Arguments:** Required `Integer AbilityId`; server positive/known INT32 ability ID, with plan-time refusal for unknown/already-known cases.
- **Target/scope:** Calling GM's own character; no training-point debit and no selected target.
- **Preconditions and authority:** Server GM gate, live player, valid ability grant; base row must still match. `gm/tests/abilities.rs` checks caller-only and refusal paths.
- **Effects and impact:** Persistent append to `sgw_player.abilities`, cell mirror and client ability burst; grant provenance/skill state may change.
- **Verification:** Compare exact caller's persisted `sgw_player.abilities` before/after and base result feedback; the client ability burst is a separate observation. `server_ability_state` does not expose the known-ID set.
- **Compensation:** No automatic inverse; respec/reset has wider effects and is not a true undo.
- **Availability:** Unsupported until native binding/submission proof; server handler exists.

## /gmgivetrainingpoints

- **Sources:** TOML `Event_SlashCmd_GiveTrainingPoints`; candidate 137; `gm/give_training_points.rs::handle_give_training_points` → `GrantTrainingPoints` → `base-methods/.../progression/grant_training_points.rs::handle_grant_training_points`; `grant_training_points_tests.rs` covers guarded SQL.
- **Arguments:** Required `Integer numPoints`; server requires positive INT32, base refuses overflow beyond `i32::MAX`.
- **Target/scope:** Calling GM's own character, no selected player.
- **Preconditions and authority:** Server GM gate, current matching player session, DB pool/row and sufficient integer headroom.
- **Effects and impact:** Persistent additive `sgw_player.training_points` `UPDATE ... RETURNING`; base sends `TrainingPointsGranted`, cell mirrors points and client property update after commit.
- **Verification:** Compare exact caller's `sgw_player.training_points` before/after and base feedback; `server_entity_get` cell mirror and client counter are separate freshness checks. `cell/dispatch/gm_dispatch_tests.rs` verifies non-GM rejection for 137.
- **Compensation:** No remove-points inverse; a later spend is a different action.
- **Availability:** Unsupported until native binding/submission proof; server handler exists.
