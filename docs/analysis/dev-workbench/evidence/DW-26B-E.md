# DW-26B-E: slash commands 82–108

These are exactly entries 82–108 of `data/client-patches/012-gm-slash-commands/commands.toml`. The TOML supplies a typeable word, client event and ordered mandatory parser parameters, not a proven cell-method call. `docs/analysis/dev-workbench/stock-slash-audit.md` and `crates/lab/src/workbench/slash_catalog.rs::method_candidate` keep event-to-method bindings unverified; stock slash files and native submission/quoting proof are absent. **All 27 palette forms remain disabled pending DW-24B**, including the three with implemented server paths. Do not submit a dot command as a substitute. No parameters in this slice are optional or have TOML bounds/defaults. String quoting and Boolean text syntax remain unproved. Numeric widths below are *server method* widths, not extra TOML parser claims.

The `access = "p"` parser filter does not confer server authority. `crates/cell/src/cell/dispatch/router.rs::dispatch_cell_method` invokes `gm_gate::enforce_gm_gate` before the SGWGmPlayer tail; it checks server-sourced `CellEntity.access_level >= GameMaster` and refuses a missing entity. An unhandled tail method falls through `crates/cell-console/src/cell/console/gm/mod.rs::dispatch` to a warning; neither a parsed line nor a GM-gate pass proves an effect. The numbered signatures below come from `docs/protocol/cell-method-dispatch-table.md` and `entities/defs/SGWGmPlayer.def`, independently of patch 012. The many server reload primitives in the table are startup/full-load functions, not per-ID GM handlers. `server_db_query` can read persistent state but cannot prove an absent handler ran; use handler feedback and an actor-bound readback only where a handler exists.

Only `/gmsendgmshout` is an announcement in this slice. Its accepted text is intentionally displayed to the selected audience and logged by the server, so the companion must redact `Text` in its own history, audit export, traces and screenshots by default and show a scope/recipient preview before submission. The two minigame-contact and two stargate-address words *name* other-player targets but have no implemented method; the palette must not imply their target IDs were changed. `/gmtestlos` reads two entities in the caller's own space without mutation. `/gmonphysics` changes only the calling GM's movement validation state if its method is actually reached. Other listed reload, flag, AI, loot and vision effects are unavailable or unknown, not inferred from the words.

## /gmgiveminigamecontact

- **Sources:** TOML `Event_SlashCmd_GiveMinigameContact`; protocol method 192 `gmGiveMinigameContact(WSTRING contactId, INT64 target)`; `crates/cell-console/src/cell/console/gm/mod.rs::dispatch` has no 192 arm. The protocol table's minigame stub note is not an implemented GM grant.
- **Arguments:** Required `Integer ContactId`, `Integer TargetId` in TOML order; neither is optional. This conflicts with the method's WSTRING/INT64 shape, so the encoding and target identity cannot be assumed.
- **Target/scope:** A proposed contact on one proposed target; no implemented target resolver or proven client binding.
- **Preconditions and authority:** GM gate would apply to method 192; contact existence, minigame session and target authorization are unimplemented.
- **Effects and impact:** No Cimmeria contact grant through this method; a UI submission must not claim target/session mutation.
- **Verification:** No accepted-effect readback. A before/after contact snapshot would be diagnostic only until a handler and source are identified.
- **Compensation:** None; no proven grant or inverse.
- **Availability:** Disabled: handler absent, TOML/method types disagree and native binding unproved.

## /gmremoveminigamecontact

- **Sources:** TOML `Event_SlashCmd_RemoveMinigameContact`; protocol method 193 `gmRemoveMinigameContact(WSTRING contactId, INT64 target)`; no 193 arm in `gm/mod.rs::dispatch`.
- **Arguments:** Required `Integer ContactId`, `Integer TargetId`; no TOML optional/bounds. Server signature instead uses WSTRING/INT64.
- **Target/scope:** Proposed one target/contact pair; no resolved player or session.
- **Preconditions and authority:** GM gate would apply to 193; ownership/contact removal rules have no handler.
- **Effects and impact:** No Cimmeria contact removal; never report the target as changed.
- **Verification:** No accepted-effect readback or handler feedback.
- **Compensation:** None; no proven inverse.
- **Availability:** Disabled: missing handler, type mismatch and native binding proof.

## /gmgivestargateaddress

- **Sources:** TOML `Event_SlashCmd_GiveStargateAddress`; protocol method 194 `gmGiveStargateAddress(WSTRING addr, INT64 target, UINT8 hidden)`; no 194 arm in `gm/mod.rs::dispatch`; `spawner/stargates.rs` loads startup data only.
- **Arguments:** Required `String AddressId`, `Boolean Hidden`; method also requires an INT64 target absent from TOML. Quoting/Boolean encoding unproved.
- **Target/scope:** Unresolved target player and one address; do not silently substitute the caller for the omitted target.
- **Preconditions and authority:** GM gate would apply; per-player address grant/hidden rules absent.
- **Effects and impact:** No implemented discovery grant, target mutation or broadcast.
- **Verification:** No handler feedback/readback; startup stargate catalog is not a player's known-address state.
- **Compensation:** None; no grant or inverse.
- **Availability:** Disabled: missing target parameter/handler and unproved native binding.

## /gmremovestargateaddress

- **Sources:** TOML `Event_SlashCmd_RemoveStargateAddress`; protocol method 195 `gmRemoveStargateAddress(WSTRING addr, INT64 target)`; no 195 arm in `gm/mod.rs::dispatch`.
- **Arguments:** Required `String AddressId`; method requires an additional INT64 target missing from TOML.
- **Target/scope:** No resolved target; one proposed address.
- **Preconditions and authority:** GM gate would apply; target and address ownership rules absent.
- **Effects and impact:** No implemented per-player address removal or broadcast.
- **Verification:** No handler feedback/readback; a catalog row cannot prove target state.
- **Compensation:** None.
- **Availability:** Disabled: missing target parameter/handler and native binding proof.

## /gmloadconstants

- **Sources:** TOML `Event_SlashCmd_LoadConstants`; protocol method 196 `loadConstants()` is NEW; no 196 arm in `gm/mod.rs::dispatch`.
- **Arguments:** None.
- **Target/scope:** Potential server constants, scope undefined; no player target.
- **Preconditions and authority:** GM gate would apply; no constants reload entrypoint.
- **Effects and impact:** No implemented reload; do not claim safe or reversible behavior.
- **Verification:** No effect/readback or handler feedback.
- **Compensation:** None.
- **Availability:** Disabled: no handler and native binding unproved.

## /gmloadability

- **Sources:** TOML `Event_SlashCmd_LoadAbility`; protocol method 197 `loadAbility(INT32 id)` is ADAPT; `spawner/abilities.rs::load_ability_defs` is full-load only; no 197 arm in `gm/mod.rs::dispatch`.
- **Arguments:** Required `Integer aAbilityId`; no optional/bounds.
- **Target/scope:** Proposed one ability definition, not a player's granted ability.
- **Preconditions and authority:** GM gate would apply; no per-ID hot-reload handler.
- **Effects and impact:** No implemented per-ID reload; full startup loader is not proof of one.
- **Verification:** No handler receipt or version/readback.
- **Compensation:** None; reload rollback is unproved.
- **Availability:** Disabled: no handler and native binding unproved.

## /gmreloadscripts

- **Sources:** TOML `Event_SlashCmd_LoadNACSI`; protocol method 198 `loadNACSI(INT32 id)` is NEW, with no NACSI table; no 198 arm in `gm/mod.rs::dispatch`.
- **Arguments:** Required `Integer nacsiId`; no optional/bounds.
- **Target/scope:** Proposed one NACSI script ID; the word “reloadscripts” does not prove broader scope.
- **Preconditions and authority:** GM gate would apply; no script loader.
- **Effects and impact:** No implemented reload or session mutation.
- **Verification:** No handler feedback/readback; do not treat unrelated content reload logs as success.
- **Compensation:** None.
- **Availability:** Disabled: no handler/storage and native binding unproved.

## /gmloadabilityset

- **Sources:** TOML `Event_SlashCmd_LoadAbilitySet`; protocol method 199 `loadAbilitySet(INT32 id)` is ADAPT; `spawner/abilities.rs::load_archetype_ability_trees` loads fully; no 199 arm in `gm/mod.rs::dispatch`.
- **Arguments:** Required `Integer aAbilitySetId`; no optional/bounds.
- **Target/scope:** Proposed one ability-set definition, not a selected mob or player.
- **Preconditions and authority:** GM gate would apply; no per-ID reload path.
- **Effects and impact:** No implemented per-ID reload; possible runtime impact cannot be inferred from startup load.
- **Verification:** No handler receipt or post-reload definition version.
- **Compensation:** None; no proven inverse.
- **Availability:** Disabled: no handler and native binding unproved.

## /gmloadbehavior

- **Sources:** TOML `Event_SlashCmd_LoadBehavior`; protocol method 200 `loadBehavior(INT32 id)` is NEW; no 200 arm in `gm/mod.rs::dispatch` or loadable behavior definition path.
- **Arguments:** Required `Integer behaviorEventSetId`; no optional/bounds.
- **Target/scope:** Proposed behavior event set ID; no target NPC selected by the TOML.
- **Preconditions and authority:** GM gate would apply; no behavior reload handler.
- **Effects and impact:** No implemented behavior reload or NPC/session change.
- **Verification:** No handler feedback/readback.
- **Compensation:** None.
- **Availability:** Disabled: missing handler and native binding proof.

## /gmloadmob

- **Sources:** TOML `Event_SlashCmd_LoadMOB`; protocol method 201 `loadMOB(INT32 id)` is NEW; no 201 arm in `gm/mod.rs::dispatch` or per-mob reload.
- **Arguments:** Required `Integer mobId`; no optional/bounds.
- **Target/scope:** Proposed mob definition ID, not a live entity ID.
- **Preconditions and authority:** GM gate would apply; no per-definition reload handler.
- **Effects and impact:** No implemented reload or live NPC mutation.
- **Verification:** No handler feedback/readback; a spawn listing would not prove reload.
- **Compensation:** None.
- **Availability:** Disabled: handler absent and native binding unproved.

## /gmloaddialogset

- **Sources:** TOML `Event_SlashCmd_ShowDialog`; protocol method 202 `loadDialogSet(INT32 id)` is ADAPT; `spawner/dialogs.rs::load_dialog_set_maps` loads fully; no 202 arm in `gm/mod.rs::dispatch`.
- **Arguments:** Required `Integer interactionSetId`; no optional/bounds. Event `ShowDialog` does not prove binding to `loadDialogSet`.
- **Target/scope:** Proposed interaction set, with no TOML target player/NPC.
- **Preconditions and authority:** GM gate would apply to 202; no per-ID handler.
- **Effects and impact:** No implemented per-ID reload or dialog presentation via this route.
- **Verification:** No handler feedback/readback; visible dialog alone would not establish a reload.
- **Compensation:** None.
- **Availability:** Disabled: event/method ambiguity, no handler, native binding unproved.

## /gmloaditem

- **Sources:** TOML `Event_SlashCmd_LoadItem`; protocol method 203 `loadItem(INT32 id)` is ADAPT; `spawner/loot.rs::load_item_defs` loads weapon definitions fully; no 203 arm in `gm/mod.rs::dispatch`.
- **Arguments:** Required `Integer itemId`; no optional/bounds.
- **Target/scope:** Proposed item definition, not an inventory stack or recipient.
- **Preconditions and authority:** GM gate would apply; no per-item hot-reload handler.
- **Effects and impact:** No implemented per-ID reload or inventory mutation.
- **Verification:** No handler receipt/readback; item catalog presence alone is baseline data.
- **Compensation:** None; reload rollback unproved.
- **Availability:** Disabled: handler absent and native binding unproved.

## /gmloadmission

- **Sources:** TOML `Event_SlashCmd_LoadMission`; protocol method 204 `loadMission(INT32 id)` is ADAPT; `spawner/missions.rs::load_mission_defs` loads fully; no 204 arm in `gm/mod.rs::dispatch`.
- **Arguments:** Required `Integer missionId`; no optional/bounds.
- **Target/scope:** Proposed mission definition, not a player's mission progress.
- **Preconditions and authority:** GM gate would apply; no per-mission reload handler.
- **Effects and impact:** No implemented per-ID reload or player tracker mutation.
- **Verification:** No handler receipt/version readback; `sgw_mission` rows are not definition reload proof.
- **Compensation:** None.
- **Availability:** Disabled: handler absent and native binding unproved.

## /gmsetmobvariable

- **Sources:** TOML `Event_SlashCmd_SetMobVariable`; protocol method 205 `setMobVariable(INT32 var, INT32 val)` is NEW; no generic mob KV store or 205 arm in `gm/mod.rs::dispatch`.
- **Arguments:** Required `Integer variable`, `Integer value`; no optional/bounds.
- **Target/scope:** No target mob parameter in TOML; selected-mob semantics unproved.
- **Preconditions and authority:** GM gate would apply; no variable writer or target resolver.
- **Effects and impact:** No implemented mob-state mutation.
- **Verification:** No handler feedback/readback; NPC state snapshot cannot assign causality.
- **Compensation:** None.
- **Availability:** Disabled: no handler/store, target unresolved, binding unproved.

## /gmentererroraistate

- **Sources:** TOML `Event_SlashCmd_EnterErrorAIState`; protocol method 206 `enterErrorAIState()` is ADAPT; `service/npc_ai.rs` has `AiState::Error`, but no 206 GM arm in `gm/mod.rs::dispatch`.
- **Arguments:** None.
- **Target/scope:** Intended NPC target unspecified by TOML; cannot assume current selection.
- **Preconditions and authority:** GM gate would apply; no target/transition handler.
- **Effects and impact:** No implemented GM transition; an existing AI state enum is not an entrypoint.
- **Verification:** No handler feedback/readback for this word.
- **Compensation:** None; `/gmexiterroraistate` also lacks a handler.
- **Availability:** Disabled: no handler/target proof or native binding.

## /gmexiterroraistate

- **Sources:** TOML `Event_SlashCmd_ExitErrorAIState`; protocol method 207 `exitErrorAIState()` is ADAPT; no 207 arm in `gm/mod.rs::dispatch`.
- **Arguments:** None.
- **Target/scope:** Intended NPC target unspecified.
- **Preconditions and authority:** GM gate would apply; no error-state clearance entrypoint.
- **Effects and impact:** No implemented GM state transition.
- **Verification:** No handler feedback/readback; observed AI recovery would need another cause.
- **Compensation:** None.
- **Availability:** Disabled: handler/target binding absent.

## /gmperfstatsbychannel

- **Sources:** TOML `Event_SlashCmd_PerfStatsByChannel`; protocol method 208 `gmPerfStatsByChannel(INT8 onOff)` is NEW; no per-channel counters or 208 arm in `gm/mod.rs::dispatch`.
- **Arguments:** Required `Boolean aOnOff`; no optional/default. Server signature is INT8, parser Boolean encoding unproved.
- **Target/scope:** Proposed diagnostic toggle, scope unspecified; no other-player audience established.
- **Preconditions and authority:** GM gate would apply; no metrics-toggle handler.
- **Effects and impact:** No implemented toggle or telemetry subscription.
- **Verification:** No handler feedback/counter readback.
- **Compensation:** None; turning it off would not be a proven inverse.
- **Availability:** Disabled: no handler or native binding.

## /gmshowinstanceflag

- **Sources:** TOML `Event_SlashCmd_ShowInstanceFlag`; protocol method 209 `gmShowInstanceFlag(INT32 flag)` is NEW; no `instance_flags` on `SpaceInstance`, no 209 arm in `gm/mod.rs::dispatch`.
- **Arguments:** Required `Integer aFlagNumber`; no optional/bounds.
- **Target/scope:** Presumed current instance flag, but no implemented state or cross-instance scope.
- **Preconditions and authority:** GM gate would apply; no flag reader.
- **Effects and impact:** No implemented read response or mutation.
- **Verification:** No handler feedback/readback.
- **Compensation:** None; read only if ever implemented.
- **Availability:** Disabled: no state/handler and native binding unproved.

## /gmsetinstanceflag

- **Sources:** TOML `Event_SlashCmd_SetInstanceFlag`; protocol method 210 `gmSetInstanceFlag(INT32 flag, INT8 val)` is NEW; no `instance_flags` state or 210 arm in `gm/mod.rs::dispatch`.
- **Arguments:** Required `Integer aFlagNumber`, `Integer aFlagValue`; no optional/bounds. Second server argument is INT8, so range/encoding needs reconciliation.
- **Target/scope:** Proposed current-instance flag; no actual instance writer.
- **Preconditions and authority:** GM gate would apply; no valid range or mutation handler.
- **Effects and impact:** No implemented instance-state change; effect on co-located players unknown.
- **Verification:** No handler receipt/flag readback.
- **Compensation:** None; prior value cannot be restored through this route.
- **Availability:** Disabled: no state/handler, argument mismatch and binding unproved.

## /gmspawnentityloot

- **Sources:** TOML `Event_SlashCmd_ShowSpawns`; protocol method 212 `spawnEntityLoot(INT32 entity, LootTableID)` is ADAPT; `abilities/loot_drop.rs::generate_loot_on_death` is an internal primitive, with no 212 arm in `gm/mod.rs::dispatch`.
- **Arguments:** Required `Integer EntityId`, `Integer LootTableID`; no optional/bounds. Event `ShowSpawns` is not proof of `spawnEntityLoot` binding.
- **Target/scope:** Proposed one entity and loot table; no implemented entity/loot resolver.
- **Preconditions and authority:** GM gate would apply to 212; no GM spawn-loot handler.
- **Effects and impact:** No implemented loot creation; do not promise a generated item or a world broadcast.
- **Verification:** No handler feedback or loot-instance readback; internal drop logic does not prove this command.
- **Compensation:** None; a future spawned item may be consumed before reversal.
- **Availability:** Disabled: event/method ambiguity, no handler and binding unproved.

## /gmtestlos

- **Sources:** TOML `Event_SlashCmd_TestLOS`; protocol method 216 `testLOS(INT32 source, INT32 target)` is DONE; `crates/cell-console/src/cell/console/gm/query.rs::handle_test_los`; `gm/tests/query.rs::test_los_reports_via_feedback` and `test_los_truncated_and_missing_target_feed_back`.
- **Arguments:** Required `Integer sourceID`, `Integer targetID`; no optional/bounds. Server reads two signed INT32 IDs.
- **Target/scope:** Two existing entities in the GM caller's exact space; read-only diagnostic, feedback to the caller only.
- **Preconditions and authority:** Server GM gate; both IDs must convert to u32 and resolve in the caller's space. Missing, cross-space, negative or truncated IDs refuse with feedback before the navmesh query.
- **Effects and impact:** No entity mutation or player broadcast. Navmesh result is `CLEAR`, `BLOCKED` or `UNKNOWN`; `UNKNOWN` is not clear visibility.
- **Verification:** Capture caller-only `testLOS source → target: verdict` GM feedback; a refused target yields `testLOS: source/target not found in your space.`. The tests prove both branches; compare IDs and caller space, not merely line submission.
- **Compensation:** None; read only.
- **Availability:** Disabled for palette execution pending DW-24B event-to-method and native submission proof; implemented method 216 is separately source-checked. Do not infer this event binds 216 solely from its name.

## /gmtogglecombatlos

- **Sources:** TOML `Event_SlashCmd_ToggleCombatLOS`; protocol method 217 `toggleCombatLOS()` is NEW; `bCombatLOS` is def-only, no 217 arm in `gm/mod.rs::dispatch`.
- **Arguments:** None.
- **Target/scope:** Potential combat LOS toggle scope unknown; no current actor/world setting.
- **Preconditions and authority:** GM gate would apply; no enforcement toggle handler.
- **Effects and impact:** No implemented combat rule change or audience fan-out.
- **Verification:** No handler feedback/readback; a combat outcome is not proof of a toggle.
- **Compensation:** None.
- **Availability:** Disabled: no handler and native binding unproved.

## /gmtrackmob

- **Sources:** TOML `Event_SlashCmd_ShowMobPaths`; protocol method 218 `trackMob()` is ADAPT; AI state is readable but no debug stream/218 arm in `gm/mod.rs::dispatch`.
- **Arguments:** None.
- **Target/scope:** Proposed mob tracking target unspecified by TOML; selected entity cannot be presumed.
- **Preconditions and authority:** GM gate would apply; no tracking subscription.
- **Effects and impact:** No implemented path stream or state mutation; AI state read is a separate operation.
- **Verification:** No handler receipt/track output readback.
- **Compensation:** None.
- **Availability:** Disabled: event/method ambiguity, absent handler and native binding proof.

## /gmonxrayeyes

- **Sources:** TOML `Event_SlashCmd_XRayEyes`; protocol method 219 `onXRayEyes(UINT8 on)` is NEW and client-presentation only; no 219 arm in `gm/mod.rs::dispatch`.
- **Arguments:** Required `Boolean on/off`; no optional/default. Exact client Boolean text/byte is unproved.
- **Target/scope:** Calling GM's client view, not another player or server-wide visibility.
- **Preconditions and authority:** GM gate would apply to method 219; client-local presentation behavior is not established from this TOML.
- **Effects and impact:** No Cimmeria state change; any client-only effect remains unverified.
- **Verification:** No server readback; a future client-side visual check is required.
- **Compensation:** None proven.
- **Availability:** Disabled: no server handler or native client-effect/submission proof.

## /gmoninvisible

- **Sources:** TOML `Event_SlashCmd_Invisible`; protocol method 220 `onInvisible(UINT8 on)` is NEW; `mercury/aoi/leave.rs::build_entity_invisible` is a wire primitive, not a state toggle; no 220 arm in `gm/mod.rs::dispatch`.
- **Arguments:** Required `Boolean on/off`; no optional/default or verified parser encoding.
- **Target/scope:** Presumed caller visibility to observers; exact AoI audience and persistence unknown.
- **Preconditions and authority:** GM gate would apply; no visibility state/observer refresh handler.
- **Effects and impact:** No implemented invisibility transition or witness fan-out through this method.
- **Verification:** No handler feedback/readback; a lone invisible packet primitive is not an observer-state proof.
- **Compensation:** None; no proven restore route.
- **Availability:** Disabled: no handler or native binding; cannot promise stealth.

## /gmonphysics

- **Sources:** TOML `Event_SlashCmd_Physics`; protocol method 221 `onPhysics(UINT8 on)` is DONE, but explicit stock aliases are `/gmsetfly` and `/gmsetghost`; `crates/cell-console/src/cell/console/gm/physics.rs::handle_physics` and its tests. The patch event's binding is unproved.
- **Arguments:** Required `Boolean on/off`; no optional/default. Server reads one UINT8; polarity is `0` → `movement_unrestricted=true`, nonzero → false, the opposite of “fly on”.
- **Target/scope:** Calling GM entity's server movement validator only; no other player audience.
- **Preconditions and authority:** GM gate and extant caller entity; missing/truncated byte gets GM feedback and no state change.
- **Effects and impact:** If 221 is reached, toggles transient validator bypass. Stock fly/ghost also toggle client pawn physics immediately; this patch word's client-local effect is unverified. No persistence or world broadcast.
- **Verification:** On actual state change, caller GM feedback says movement validation disabled/restored and an info log records `unrestricted`; no feedback for a no-op. Read the same entity's `movement_unrestricted` in a trusted entity view; no DB row.
- **Compensation:** A verified method 221 call restoring physics (`on=1`) clears bypass, but cannot retroactively validate moves.
- **Availability:** Disabled pending proof that this patch event invokes 221 and native submission/Boolean formatting; method handler alone is insufficient.

## /gmsendgmshout

- **Sources:** TOML `Event_SlashCmd_GMShout`; explicit protocol mapping to method 222 `sendGMShout(UINT8 global, WSTRING text)`; `crates/cell-console/src/cell/console/gm/shout.rs::{handle_send_gm_shout,broadcast}`, `crates/base-session/src/base/gm_broadcast.rs::broadcast_to_online_players`, `crates/base-world-entry/src/base/world_entry/cell_dispatch/chat_dispatch.rs::route`; `gm/tests/shout.rs` covers space/global/refusals; `docs/gameplay/chat-system.md` documents channel 8.
- **Arguments:** Required `Boolean isGlobal`, `String Text` in TOML order; no optional/default. Server reads one UINT8 then WSTRING; exact slash text split/quoting remains unrecovered. Blank, controls/format characters and text over 255 UTF-16 units are refused, not truncated.
- **Target/scope:** `isGlobal=0` sends to every player entity in the GM's **space instance**, including GM; any nonzero byte sends through base to every online in-world player across instances. Not a private target message. Preview exact chosen audience before submit.
- **Preconditions and authority:** Server GM gate, live caller and valid chat text. An accepted cell event is not final delivery; base channel and each online transport may fail.
- **Effects and impact:** Sends `onPlayerCommunication` on server channel 8 with GM speaker/name; clients show a red line and modal Server Message. Accepted text is logged in `chat.gm_broadcast` and shown to recipients. It must never be treated as secret text; redact Text in companion history/export/telemetry and require deliberate reveal.
- **Verification:** `chat.gm_broadcast` (`source=native`, scope, actor IDs, `text_units`) and nearby `chat.gm_broadcast_delivered` (`delivered`, `failed`, plus `not_in_world` globally) or `chat.gm_broadcast_send_failed` show aggregate fan-out evidence. They lack a per-shout correlation ID, so delivery of a particular submission remains unverified when broadcasts overlap; check actual recipient UI if authorized. Refusal logs `chat.gm_broadcast_rejected` and sends caller-only feedback, with no fan-out. `gm_shout_space_scope_stays_in_space`, `gm_shout_global_forwards_one_broadcast_to_base` and `gm_shout_refusals_send_feedback_and_nothing_else` pin the paths. Do not equate accepted or forwarded with delivered.
- **Compensation:** None: a sent announcement and modal cannot be unsent. A correction is a new broadcast.
- **Availability:** Disabled for palette execution pending DW-24B native submission/quoting proof; method 222 and its scope are implemented and source-backed. No slash-to-dot substitution.
