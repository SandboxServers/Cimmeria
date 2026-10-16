# DW-27A-E: slash commands 109–135

This is the fourth consecutive 27-entry slice of `data/client-patches/012-gm-slash-commands/commands.toml`. Each heading and argument type follows that file, including its `String` and `Boolean` spellings; the `Event_SlashCmd_*` value is a client event label, **not** proof of a cell-method call. Patch `access = "p"` is only a client parser filter (`data/client-patches/README.md`, “What `Access` does”). A received SGWGmPlayer method is independently gated by `crates/cell-world/src/cell/dispatch/gm_gate.rs::enforce_gm_gate` before `crates/cell-console/src/cell/console/gm/mod.rs::dispatch`. That dispatcher implements only named arms; an authorized fall-through is not acceptance. `docs/protocol/cell-method-dispatch-table.md` and `entities/defs/SGWGmPlayer.def` establish method shapes, while `crates/lab/src/workbench/slash_catalog.rs::method_candidate` deliberately marks event bindings unverified. Native stock-client submission and quoting are still unproved (DW-24B), so **all 27 palette actions are disabled** regardless of a related server primitive. Never submit a dot alias from a slash form.

In each **Sources** field, `TOML` means `data/client-patches/012-gm-slash-commands/commands.toml`, `protocol` means `docs/protocol/cell-method-dispatch-table.md`, `def` means `entities/defs/SGWGmPlayer.def`, and `gm/mod.rs` means `crates/cell-console/src/cell/console/gm/mod.rs`. Other abbreviated source paths are relative to `crates/` or `docs/` as indicated by their prefixes. These aliases keep the 27 per-command citations readable without making a client event into a server binding.

The cover commands describe potentially world-wide live AI tuning, not a seed edit. Their documented cell methods 223–225 have no Cimmeria dispatch arms. `crates/cell-cover/src/cell/cover/loader.rs` loads nodes; `scoring.rs` has fixed weights. The reverse-engineering finding `docs/reverse-engineering/findings/cover-system.md` describes original-client NetOut events, but does not establish an accepted Cimmeria mutation or a current graph read API. `docs/security-audit/2026-05-31-server-authority/findings/CAT-N-gm-commands.md` CAT-N-32 calls out unbounded `maxLinks`; a future handler must validate finite floats and bounded work. No inverse can reconstruct an overwritten graph or prior weights.

For a proven server handler, collect an authenticated actor/session and receipt first. `server_entity_get` returns a cell snapshot, not arbitrary physics, ammo, PvP, cover or client UI flags (`crates/lab-mcp/src/tools/entities.rs`); `server_db_query` is read-only and cannot prove an in-memory toggle. A named feedback line, cell log, and the specific state source named below are separate observations. Missing readback stays `unknown`, never `verified`. All Boolean inputs need a client-parser round trip before quoting/omission or wire-byte polarity is promised; the patch documents no numeric bounds or default for optional values.

## /gmregeneratecoverlinks

- **Sources:** TOML `Event_SlashCmd_RegenerateCoverLinks`; protocol method 223 `regenerateCoverLinks`; `SGWGmPlayer.def`; `cell-cover/cover/loader.rs`; cover-system finding and CAT-N-32; no 223 arm in `gm/mod.rs::dispatch`.
- **Arguments:** Required `Float normalLimit`, `Integer maxLinks`, `Float maxDistance`, in order. The def expects FLOAT, UINT32, FLOAT; negative/out-of-range `maxLinks`, NaN/infinite floats and safe limits are not specified by TOML, so cannot be accepted by inference.
- **Target/scope:** Intended cover-link graph in the current world or space; exact original-client scope and any cross-instance propagation are unknown. There is no selected entity.
- **Preconditions and authority:** Client parser role filter is insufficient; a method 223 call would require the server GM gate, valid parameters and a bounded graph implementation that does not exist.
- **Effects and impact:** Original-client NetOut event suggests graph rebuilding; Cimmeria has no regeneration handler. A future implementation would be disruptive world authoring, potentially expensive, never a read-only operation.
- **Verification:** No current graph generation/version/read tool. A future run needs before/after graph metrics per exact space plus handler receipt; `server_entity_get` cannot verify links.
- **Compensation:** None. Recomputing again does not restore an old graph.
- **Availability:** Disabled: handler, bounded-parameter contract, native binding and readback absent. Unknown original-client side effects remain explicit.

## /gmchangecoverweight

- **Sources:** TOML `Event_SlashCmd_ChangeCoverWeight`; protocol method 224; def; `cell-cover/cover/scoring.rs::CoverWeights`; cover-system finding and CAT-N-32; no 224 dispatch arm.
- **Arguments:** Required `Float distance`, `Float defCover`, `Float offCover`, `Float move`, `Float crossPath`, `Float cover`, in order. The six def fields are FLOAT; finite/range rules are unknown.
- **Target/scope:** Intended cover scoring in a world/space, with exact propagation and lifespan unknown; no selected entity.
- **Preconditions and authority:** A received 224 method would require GM gate; Cimmeria's fixed scoring constants have no runtime setter.
- **Effects and impact:** Original-client NetOut weight event is evidenced, but no Cimmeria weight change occurs through this method. If implemented it would change NPC behavior across affected users, not just the caller.
- **Verification:** No live cover-weight readback/version exists. Inspecting `scoring.rs` constants only establishes the baseline implementation, not an accepted change.
- **Compensation:** None; prior weights are not captured or restorable by this form.
- **Availability:** Disabled: no handler/readback or proven native event binding.

## /gmchangecoverstanceweight

- **Sources:** TOML `Event_SlashCmd_ChangeCoverStanceWeight`; protocol method 225; def; cover-system finding; no 225 dispatch arm or stance-weight system.
- **Arguments:** Required `String stance`, then `Float distance`, `Float defCover`, `Float offCover`, `Float move`, `Float crossPath`, `Float cover`. **Mismatch:** def declares WSTRING for all seven arguments, not FLOAT for the six weights; the reverse-engineering finding discusses float fields in a NetOut event, which is a different surface. Quoting and conversion are unproved.
- **Target/scope:** Intended stance-specific cover scoring; stance identity and world/instance scope unknown.
- **Preconditions and authority:** Client role filter only; any method 225 requires GM gate, but Cimmeria has no stance-weight handler or validated stance vocabulary.
- **Effects and impact:** No implemented Cimmeria effect; original-client event may alter world AI tuning, with unknown side effects.
- **Verification:** No stance-weight state or read API; no safe accepted-effect recipe.
- **Compensation:** None; prior stance settings cannot be reconstructed.
- **Availability:** Disabled: handler and type/binding reconciliation absent.

## /gmhelpfull

- **Sources:** TOML `Event_SlashCmd_HelpFull`; `docs/technical/slash-commands.md` help inventory; no matching SGWGmPlayer method or GM dispatch arm in the protocol table/`gm/mod.rs`.
- **Arguments:** None.
- **Target/scope:** Calling client's command help, not a selected player or cell world.
- **Preconditions and authority:** Patch parser `access = "p"`; no Cimmeria server authority path is established for this event.
- **Effects and impact:** Client-only display is plausible but unverified for the patched build; no evidenced server mutation.
- **Verification:** Capture actual client help output after separately proven submission. No server readback would establish it.
- **Compensation:** None; no proven mutable state.
- **Availability:** Disabled: original-client event handler effect and native submission not proved.

## /gmtestsequence

- **Sources:** TOML `Event_SlashCmd_TestSequence`; `docs/technical/slash-commands.md` misc-debug inventory; no matching GM method/dispatch arm in the protocol table/`gm/mod.rs`.
- **Arguments:** None.
- **Target/scope:** Unknown client sequence/test scope; no target specified in TOML.
- **Preconditions and authority:** Client role filter only; server method, authorization path and conditions are unproved.
- **Effects and impact:** Unknown; could trigger a local sequence or outbound action, so do not label read-only.
- **Verification:** No existing state/readback is tied to this client event.
- **Compensation:** None; no inverse is evidenced.
- **Availability:** Disabled: handler/effect/binding unknown.

## /gmrepairitem

- **Sources:** TOML `Event_SlashCmd_RepairItem`; protocol inventory method 40 `repairItemRequest`; `crates/cell-methods/src/cell/cell_methods/inventory/item_ops.rs::handle_repair_item_request` logs `UNIMPLEMENTED`; `crates/base-methods/src/base/world_entry/methods/vendor/repair.rs` is a separate vendor flow.
- **Arguments:** Required `Integer ItemId`, `Float RepairRatio`; method 40 also takes INT32/FLOAT, but event-to-method binding and valid ratio range are unproved.
- **Target/scope:** Item instance ID, likely in caller inventory; owner/vendor context is not proved for this slash event. Do not treat `ItemId` as a design ID.
- **Preconditions and authority:** Patch client filter; method 40 is outside the SGWGmPlayer 109+ gate. No slash-specific server authorization is evidenced and method 40 is only a stub.
- **Effects and impact:** No Cimmeria repair through method 40. The working vendor repair path must not be substituted for an unproved slash call.
- **Verification:** No slash-effect readback. If a future handler exists, compare exact inventory instance durability in `sgw_inventory` after commit, not just a vendor UI flash.
- **Compensation:** None; durability/currency changes have no proven inverse.
- **Availability:** Disabled: method is unimplemented and native binding/authorization unproved.

## /gmgiveammo

- **Sources:** TOML `Event_SlashCmd_GiveAmmo`; `docs/reverse-engineering/findings/ammo-system.md`; `crates/cell-console/src/cell/console/gm/give_ammo.rs` and `docs/commands.md` distinguish the working dot command.
- **Arguments:** None in TOML; do not borrow dot `.giveammo`'s ammo type/count inputs.
- **Target/scope:** Client event's target and ammo type are unknown; dot command's selected-player behavior is unrelated.
- **Preconditions and authority:** Patch client filter only; no native slash server call/GM method is established.
- **Effects and impact:** Native slash is documented as not reaching the server. The dot command can change ammo stacks, but does not define this form's effect.
- **Verification:** No slash-effect recipe. A dot inventory read would verify a different action.
- **Compensation:** None for the slash form.
- **Availability:** Disabled: no server effect or native binding; never route to dot.

## /gmgivefaction

- **Sources:** TOML `Event_SlashCmd_GiveFaction`; protocol SGWGmPlayer inventory and `gm/mod.rs::dispatch` contain no matching method/handler.
- **Arguments:** None in TOML; faction ID, amount and target cannot be invented from the name.
- **Target/scope:** Unknown faction/character scope; no selected-player rule proved.
- **Preconditions and authority:** Patch client filter only; no Cimmeria call or authority gate for this event identified.
- **Effects and impact:** Unknown original-client grant; no implemented Cimmeria effect evidenced.
- **Verification:** No specific faction field or receipt is bound to this form.
- **Compensation:** None; no inverse evidenced.
- **Availability:** Disabled: target, arguments, handler and binding unknown.

## /gmsetfaction

- **Sources:** TOML `Event_SlashCmd_SetFaction`; protocol SGWGmPlayer inventory and `gm/mod.rs::dispatch` have no matching method/handler.
- **Arguments:** None; no faction value or target is declared.
- **Target/scope:** Unknown; cannot assume caller, selected mob or reputation row.
- **Preconditions and authority:** Client role filter only; no proven server setter or gate.
- **Effects and impact:** Potentially persistent faction mutation in the original client, but Cimmeria behavior is unknown.
- **Verification:** No bound field/query; a generic faction read would not prove this event ran.
- **Compensation:** None; no original value snapshot/inverse.
- **Availability:** Disabled: handler, scope and effect unknown.

## /gminvokeability

- **Sources:** TOML `Event_SlashCmd_InvokeAbility`; protocol SGWGmPlayer inventory; `gm/mod.rs::dispatch` lacks an invoke-ability arm; ordinary ability invocation is a different player method.
- **Arguments:** None; ability ID, rank, target and source are not declared.
- **Target/scope:** Unknown ability and recipient; never infer current selection.
- **Preconditions and authority:** Client filter only; no slash-to-server mapping or GM authorization proved.
- **Effects and impact:** Unknown client event; could cause combat, costs or threat if implemented, so impact is not read-only.
- **Verification:** No receipt or effect field tied to this command.
- **Compensation:** None; combat effects cannot be undone generically.
- **Availability:** Disabled: missing target/ability encoding and handler proof.

## /gmsettechskill

- **Sources:** TOML `Event_SlashCmd_SetTechSkill`; protocol GM inventory; no `gm/mod.rs` setter arm.
- **Arguments:** None; skill ID/level absent.
- **Target/scope:** Unknown character/crafting scope.
- **Preconditions and authority:** Client role filter only; no proven server setter.
- **Effects and impact:** Potential progression mutation, but no Cimmeria implementation is established.
- **Verification:** No exact persisted skill field or server receipt bound to this form.
- **Compensation:** None; no previous-value capture.
- **Availability:** Disabled: missing typed contract, handler and binding.

## /gmgiveblueprint

- **Sources:** TOML `Event_SlashCmd_GiveBlueprint`; protocol GM inventory; no matching `gm/mod.rs` arm.
- **Arguments:** None; blueprint ID and recipient are absent.
- **Target/scope:** Unknown crafting owner; do not assume caller or selected player.
- **Preconditions and authority:** Client filter only; no proven Cimmeria grant authorization.
- **Effects and impact:** Potential persistent recipe grant in the original client, not an evidenced Cimmeria grant.
- **Verification:** No exact recipe row/readback attached to this event.
- **Compensation:** None; no grant reversal proved.
- **Availability:** Disabled: source event has no implementable target/argument/handler contract.

## /gmgivegearset

- **Sources:** TOML `Event_SlashCmd_GiveGearset`; protocol GM inventory; no matching `gm/mod.rs` arm.
- **Arguments:** None; gearset ID and recipient absent.
- **Target/scope:** Unknown inventory/character scope.
- **Preconditions and authority:** Client filter only; no server grant path proved.
- **Effects and impact:** Possible multi-item persistent grant, but no Cimmeria effect evidenced.
- **Verification:** No exact set/instance list or receipt tied to the event; inventory totals alone would be ambiguous.
- **Compensation:** None; a mass removal would not restore item identities or modifiers.
- **Availability:** Disabled: grant contents, target and binding unknown.

## /gmgiveinventory

- **Sources:** TOML `Event_SlashCmd_GiveInventory`; protocol GM inventory; `gm/mod.rs` has only `gmGiveItem` as a distinct two-argument method.
- **Arguments:** None; do not synthesize `gmGiveItem`'s design ID/quantity.
- **Target/scope:** Unknown owner and inventory contents.
- **Preconditions and authority:** Client filter only; no corresponding server method or Cimmeria grant.
- **Effects and impact:** Original-client intent unknown; could be a bulk inventory mutation, with no proved Cimmeria behavior.
- **Verification:** No exact inventory-instance readback recipe can be formed without item identities.
- **Compensation:** None; bulk item creation has no generic inverse.
- **Availability:** Disabled: no handler or typed contents/binding.

## /gmdebugminigameinstance

- **Sources:** TOML `Event_SlashCmd_DebugMinigameInstance`; protocol MinigamePlayer index 23 `debugMinigameInstance`; `crates/cell-methods/src/cell/cell_methods/minigame.rs::dispatch` logs `UNIMPLEMENTED`; `gm_gate.rs` explicitly gates index 23.
- **Arguments:** Required `Integer instanceId`, `Integer seed`, `Integer techcomp` in TOML order, matching three INT32 method fields; no accepted range is defined here.
- **Target/scope:** Calling GM's minigame debug session, with instance identity in the argument; no selected-player target proved.
- **Preconditions and authority:** If index 23 is received, server GM gate applies; event binding is still unproved. The handler logs the values but starts no instance.
- **Effects and impact:** No implemented session mutation in Cimmeria; original-client side effects before/after the method are unknown.
- **Verification:** `server_log_tail` can establish a received stub call and decoded IDs, **not** an opened minigame. No accepted-effect readback exists.
- **Compensation:** None; no implemented state change.
- **Availability:** Disabled: server stub and unproved client binding/submission.

## /gmdebugflash

- **Sources:** TOML `Event_SlashCmd_DebugFlash`; `docs/technical/slash-commands.md` misc-debug list; protocol GM inventory and `gm/mod.rs` have no corresponding method.
- **Arguments:** None.
- **Target/scope:** Likely client Flash UI, but exact widgets/session effect are unknown.
- **Preconditions and authority:** Client parser filter only; no server gate is associated with the event.
- **Effects and impact:** Potential local debug display; client-only side effect is **unknown**, not assumed to be harmless or server accepted.
- **Verification:** Requires a client UI observation after proven submission; server entity/DB reads cannot verify it.
- **Compensation:** None; no proven toggle or reset semantics.
- **Availability:** Disabled: no traced client handler effect/submission.

## /gmshowlog

- **Sources:** TOML `Event_SlashCmd_ShowLog`; `docs/commands.md` says the game handles log display; protocol GM inventory/`gm/mod.rs` have no matching method.
- **Arguments:** Optional `Boolean on/off`, exactly as TOML. Omitted value's default/toggle behavior and accepted text format are unknown.
- **Target/scope:** Calling client's log window/output, not a world target.
- **Preconditions and authority:** Client parser role filter; no Cimmeria server gate/path established.
- **Effects and impact:** Client-local display is documented, but the optional parameter's exact effect and any client-only side effect remain unverified for palette execution.
- **Verification:** Observe client log visibility and actual parsed value; no server readback exists.
- **Compensation:** Manual client action only after toggle semantics are proven; do not advertise an inverse now.
- **Availability:** Disabled: native submission/Boolean omission and client result unproved.

## /gmsetinfiniteammo

- **Sources:** TOML `Event_SlashCmd_SetInfiniteAmmo`; `docs/reverse-engineering/findings/ammo-system.md`; `docs/gameplay/weapon-ammo-reload.md`; `gm/set_infinite_ammo.rs` implements only dot `.infiniteammo`/`.gmsetinfiniteammo`.
- **Arguments:** Optional `Boolean on/off`; no TOML default and no verified native on/off serialization.
- **Target/scope:** Native slash client's own debug state is unknown. Dot command changes the calling GM's character reserve-ammo reload rule; this is a **separate** executor.
- **Preconditions and authority:** Patch client filter; no server method for native slash. Dot command has its own cell-console GM authority checks, which cannot be borrowed.
- **Effects and impact:** Native slash is documented as not reaching the server; no Cimmeria ammo-state mutation through it. Dot's in-memory per-character toggle is not slash evidence.
- **Verification:** No slash readback. A dot state/log or bag count proves only dot execution, not this event.
- **Compensation:** None for the slash form; do not present `.infiniteammo off` as its inverse.
- **Availability:** Disabled: no native server effect and no proven local client readback/submission.

## /gmsetinvulnerable

- **Sources:** TOML `Event_SlashCmd_SetInvulnerable`; protocol GM inventory; `gm/god_mode.rs::handle_set_god_mode` implements **different** index 142 `gmSetGodMode`.
- **Arguments:** Optional `Boolean on/off`; omission/default and byte encoding unknown.
- **Target/scope:** Presumed caller toggle from spelling only; actual client target/state is unproved.
- **Preconditions and authority:** Client filter only; no invulnerability-specific server method or Cimmeria handler. Do not alias to god mode.
- **Effects and impact:** Unknown local/server behavior; cannot claim no damage or persistent protection.
- **Verification:** No invulnerable flag/readback is associated with this slash event; god-mode feedback would be a different action.
- **Compensation:** None; no proven prior state/inverse.
- **Availability:** Disabled: no effect/binding proof.

## /gmsetomnipotent

- **Sources:** TOML `Event_SlashCmd_SetOmnipotent`; protocol GM inventory and `gm/mod.rs` have no matching handler; `gm/god_mode.rs` is a distinct method.
- **Arguments:** None; even an `off` value is not represented.
- **Target/scope:** Unknown caller/client debug state.
- **Preconditions and authority:** Client role filter only; no corresponding server authority path.
- **Effects and impact:** Unknown potentially broad life/combat modification; do not equate with god mode.
- **Verification:** No specific state field or reply associated with the event.
- **Compensation:** None; no reset/previous-value protocol.
- **Availability:** Disabled: effect and handler unknown.

## /gmsetignorefocus

- **Sources:** TOML `Event_SlashCmd_SetIgnoreFocus`; protocol GM inventory/`gm/mod.rs` lack a corresponding method; `docs/commands.md` marks it not yet implemented.
- **Arguments:** Optional `Boolean on/off`; omitted default unknown.
- **Target/scope:** Unknown caller focus-cost/debug state; no selected-player semantics proved.
- **Preconditions and authority:** Client filter only; no Cimmeria server setter/gate for this event.
- **Effects and impact:** Unknown client-only or server effect. No claim that focus costs are bypassed.
- **Verification:** No event-specific flag/receipt; focus before/after alone cannot attribute a toggle.
- **Compensation:** None; no proven inverse.
- **Availability:** Disabled: handler and local side effect unproved.

## /gmsetignorehealth

- **Sources:** TOML `Event_SlashCmd_SetIgnoreHealth`; protocol GM inventory/`gm/mod.rs` lack a corresponding method; `docs/commands.md` marks it not yet implemented.
- **Arguments:** Optional `Boolean on/off`; omitted default unknown.
- **Target/scope:** Unknown caller health/debug state.
- **Preconditions and authority:** Client filter only; no corresponding server method. Distinct from `gmSetGodMode`.
- **Effects and impact:** Unknown client-only or server effect; no established server damage bypass.
- **Verification:** No event-specific state/readback; health remaining stable in one hit is insufficient.
- **Compensation:** None; no proven inverse.
- **Availability:** Disabled: effect and method binding unproved.

## /gmsetfly

- **Sources:** TOML `Event_SlashCmd_SetFly`; protocol method 221 `onPhysics(UINT8)`; `gm/physics.rs::handle_physics`; `docs/architecture/movement-validation.md` documents client-local switch and server validator bypass.
- **Arguments:** Optional `Boolean on/off` in TOML. Omitted behavior and mapping from parser Boolean to `onPhysics` byte need native proof. The **method** wire polarity is inverted: `onPhysics(0)` means physics off and unrestricted server movement; `(1)` restores validation.
- **Target/scope:** Calling GM's pawn only, in current cell; no selected entity. Client changes pawn physics immediately, before any server reply.
- **Preconditions and authority:** If method 221 is sent, server GM gate and live caller apply. Cimmeria stores `CellEntity::movement_unrestricted` in memory, reset on entity lifecycle.
- **Effects and impact:** Transient client fly physics plus server movement-validator bypass if method received; no cross-world teleport or persisted capability. Server receives the same method for fly and ghost.
- **Verification:** Changed-state `onPhysics` feedback and cell log; inspect `movement_unrestricted` through a trusted targeted server state/debug observation if exposed. `server_entity_get` does not currently include this field. Client physics must be separately observed; a successful slash text parse proves neither method receipt nor bypass.
- **Compensation:** A proven opposite client command/method could restore validation, but current slash form cannot promise this, and movement already made is not reversed.
- **Availability:** Disabled for palette: native event-to-method/optional Boolean submission unproved despite server handler; do not claim `verified` from client-local motion alone.

## /gmsetghost

- **Sources:** TOML `Event_SlashCmd_SetGhost`; protocol method 221; `gm/physics.rs::handle_physics`; movement-validation architecture.
- **Arguments:** Optional `Boolean on/off`; omitted default and client-to-wire encoding unproved. Method 221 byte polarity remains `0` unrestricted, `1` restored.
- **Target/scope:** Calling GM's pawn; no selected entity. Client changes collision/physics locally and immediately.
- **Preconditions and authority:** Method 221 requires server GM gate and live caller; same in-memory flag as fly.
- **Effects and impact:** Transient client ghosting plus possible server movement-validation bypass. Fly and ghost are indistinguishable to the server handler; do not claim separate server flags.
- **Verification:** Changed-state `onPhysics` feedback/cell log plus a trusted flag observation if available; `server_entity_get` lacks this flag. Check client collision separately; no claim of acceptance from local effect.
- **Compensation:** Opposite command may restore client physics and method flag if proven, but cannot undo prior movement/collision effects.
- **Availability:** Disabled for palette pending native path and optional-value proof, even though method 221 is implemented.

## /gmsetspectator

- **Sources:** TOML `Event_SlashCmd_SetSpectator`; protocol GM inventory/`gm/mod.rs` lack a matching server method; `docs/commands.md` marks it not yet implemented.
- **Arguments:** Optional `Boolean on/off`; omission/default unknown.
- **Target/scope:** Likely calling client's spectator camera/state, but actual scope is untraced.
- **Preconditions and authority:** Client filter only; no proven server gate for this event.
- **Effects and impact:** Client-only and possible server side effects unknown. Do not conflate with minigame spectate or world movement.
- **Verification:** No server field or reply; would require actual client camera/state observation after proven submit.
- **Compensation:** None; restore behavior unproved.
- **Availability:** Disabled: client handler/effect and submission unproved.

## /gmsetnotarget

- **Sources:** TOML `Event_SlashCmd_SetNoTarget`; protocol GM inventory/`gm/mod.rs` lack a matching method; `docs/commands.md` marks it not yet implemented.
- **Arguments:** Optional `Boolean on/off`; omission/default unknown.
- **Target/scope:** Caller/no-target status is suggested by name only; whether other players or NPC AI see it is unproved.
- **Preconditions and authority:** Client filter only; no server state/gate for this event.
- **Effects and impact:** Unknown. Do not promise AI aggro suppression, AoI removal or untargetability.
- **Verification:** No event-specific server property/readback; an NPC failing to attack is not sufficient.
- **Compensation:** None; opposite input has no proven semantics.
- **Availability:** Disabled: unknown handler and cross-observer effect.

## /gmsetpvp

- **Sources:** TOML `Event_SlashCmd_SetPVP`; `docs/reverse-engineering/findings/duel-restoration.md` identifies `Event_SlashCmd_SetPVP` one-byte client emitter; `entities/defs/SGWPlayer.def` internal `setPvPFlag` is **not exposed**; protocol table lists it without a flattened client-call index; `crates/cell-world/src/cell/combat/aggression.rs::player_may_attack` checks duel registry.
- **Arguments:** Required `Boolean on/off` in TOML. RE finding observes an INT8 emitter for `/pvp`; exact patched `/gmsetpvp` event-to-emitter binding and Boolean encoding are unproved.
- **Target/scope:** One player's requested PvP flag, if the original client emits it; not a grant of attack rights against all players. No selected-player argument.
- **Preconditions and authority:** Patch client role filter only. `setPvPFlag` is an internal server method, not a GM-callable index. Cimmeria has no slash setter or acceptance path; duel combat authorization still requires an engaged pair in `DuelRegistry`.
- **Effects and impact:** Original client may emit an INT8 request; Cimmeria has no proved receiving handler for this slash event. Current duel PvP flag fan-out is presentation, and does **not** replace the server combat gate (`cell/duel/outbound.rs`, `combat/aggression.rs`). Client-only side effects remain unknown.
- **Verification:** No accepted slash readback. For an established duel, `server_entity_get` is not a standalone PvP setter receipt; compare exact duel registry state, recipient/witness property messages and combat gate separately if those are under test.
- **Compensation:** None. Sending `off` is not a proven inverse for an unaccepted request and cannot undo prior combat.
- **Availability:** Disabled: client event binding, server handler and authoritative PvP scope are unproved; never equate a local flag with combat permission.
