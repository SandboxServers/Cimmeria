# DW-27B-E: slash commands 136–162

These headings follow entries 136–162 of `data/client-patches/012-gm-slash-commands/commands.toml` exactly. `params` are required and retain their TOML order and spelling; there are no optional parameters in this slice. The `Event_SlashCmd_*` strings identify client events, **not** cell methods or proof of a server receipt. Patch 012's `access = "p"` is a client parser filter, not authority. `docs/protocol/cell-method-dispatch-table.md` and the entity definitions establish separate method signatures. A matching name alone does not bind an event to a method. DW-24B's native submission and binding proof is absent, so **all 27 palette actions remain disabled** even where a server method works. `docs/commands.md` sometimes describes stock or dot behavior and uses parameters that differ from this TOML; the typed patch table wins for this inventory.

The SGWGmPlayer tail at cell indices 109–225 is server gated by `crates/cell/src/cell/dispatch/router.rs::dispatch_cell_method` and `crates/cell-world/src/cell/dispatch/gm_gate.rs::enforce_gm_gate`, then handled only if `crates/cell-console/src/cell/console/gm/mod.rs::dispatch` has an arm. Ordinary SGWPlayer methods, local client render switches, and base methods have different authority paths; a `/gm` prefix does not place them behind the GM tail. `crates/lab/src/workbench/slash_catalog.rs::method_candidate` returns a candidate with `binding_verified: false`, not a runnable mapping. A future form must recheck a fresh actor/target/instance at submission and distinguish client submission, server receipt, handler acceptance and effect readback.

## /gmsetarchetype

- **Sources:** TOML `Event_SlashCmd_SetArchetype`; no `gmSetArchetype` row in `docs/protocol/cell-method-dispatch-table.md` or arm in `gm/mod.rs::dispatch`; `deprecated/python/cell/SGWBeing.py::setArchetype` is a server-internal setter, not this slash handler; `crates/base-methods/src/base/world_entry/methods/player_load/core/player_data.rs` loads `sgw_player.archetype`.
- **Arguments:** None in TOML. `docs/commands.md` claims `<archetypeId>`, a direct schema conflict; no typed archetype can be inferred.
- **Target/scope:** Unknown character; no selected-player or caller-only binding is proven.
- **Preconditions and authority:** No Cimmeria slash method or validated archetype target. A future mutation would need server GM authority, valid archetype, character ownership and coordinated ability/stat updates.
- **Effects and impact:** No implemented effect. A real archetype change would be persistent character mutation, unlike a client-only appearance update; the legacy setter broadcasts `onArchetypeUpdate` but does not prove a DB write.
- **Verification:** No current effect to read back. For any future handler compare exact `sgw_player.archetype` for the captured `player_id`, then the reloaded cell archetype and client `onArchetypeUpdate`; a client update alone cannot prove persistence.
- **Compensation:** None; no safe inverse for a changed ability tree or related character state is established.
- **Availability:** Disabled: TOML has no argument and no server slash handler or event binding exists.

## /gmtimeofday

- **Sources:** TOML `Event_SlashCmd_TimeofDay`; `docs/protocol/client-method-dispatch-table.md` `onTimeofDay` 102; `deprecated/python/cell/commands/Net.py::timeOfDay` is a legacy console path that sends `player.client.onTimeofDay` only; no GM-tail method or Cimmeria slash handler.
- **Arguments:** Required `Float Time`, `Float Wind`, `Integer Weather`; preserve all three rather than `docs/commands.md`'s one-hour sketch.
- **Target/scope:** Legacy primitive addresses the calling player's client, not a world clock; patch event target is unproved.
- **Preconditions and authority:** No server slash handler. A local client display change does not authorize or persist a world change.
- **Effects and impact:** Unproved for this event; legacy command changes presentation on one client, with no `sgw_player` or world seed mutation.
- **Verification:** If independently proven, compare the recipient's `onTimeOfDay` packet and client rendered time/weather; no durable server readback exists for this slash word.
- **Compensation:** None; restoring previous presentation requires a separate known value and proven send path.
- **Availability:** Disabled: event-to-handler binding and Cimmeria slash effect absent.

## /gmtoggleautocycleability

- **Sources:** TOML `Event_SlashCmd_toggleAutoCycleAbility`; `docs/protocol/auto-cycle-button.md` traces that event, from the stock unprefixed `/toggleAutoCycleAbility` word, to cell `setAutoCycle` 83; `crates/cell-methods/src/cell/cell_methods/player/world/{mod,auto_cycle}.rs::handle_set_auto_cycle`, its `tests/auto_cycle.rs::set_auto_cycle_disable_clears_stash_and_bsf` and `crates/cell/src/cell/service/base_messages/player_init/tests/auto_cycle_starts_off.rs`.
- **Arguments:** None in TOML or the stock slash word. Its client event constructs the `INT8 enabled` of method 83; `docs/commands.md`'s typed `<enabled>` is not the patch schema.
- **Target/scope:** The authenticated caller's auto-cycle mode and selected target; no other player's mode.
- **Preconditions and authority:** Ordinary player method 83, not the GM tail. The event-to-method path is source-proven for the stock word, but registration/submission of the prefixed patch word remains unproved. Server decides target/ability validity.
- **Effects and impact:** If routed to 83, transient `auto_cycle`, ability stash and `BSF_AUTO_CYCLING` change; enable may immediately fire a weapon/last ability. Login test starts with mode off, so do not label it persistent.
- **Verification:** For a proven invocation, compare caller's cell `auto_cycle`, `auto_cycle_ability_id`, `state_field` bit and emitted state/ability packets; client button light or text submission alone is insufficient.
- **Compensation:** Proven ordinary method 83 with `enabled=0` can turn a running loop off; it cannot reverse a shot already fired. No slash inverse is proven.
- **Availability:** Disabled: native submission and prefixed parser registration remain unproved, despite the event-to-method trace and working method 83.

## /gmshowcover

- **Sources:** TOML `Event_SlashCmd_ShowCover`; `docs/reverse-engineering/findings/cover-system.md` identifies the legacy cover-node render toggle; no matching GM-tail method/arm.
- **Arguments:** Required `Boolean on/off`.
- **Target/scope:** Local client's cover visualization; no selected entity or persistent cover edit.
- **Preconditions and authority:** Client debug rendering status for this prefixed event is unverified; no server-side GM authorization/handler is shown.
- **Effects and impact:** Possible transient overlay only; no cover-link regeneration or seed write follows from this event name.
- **Verification:** If bound, inspect actual local cover overlay on/off and client event; no server effect or DB readback.
- **Compensation:** Same proven Boolean toggle with the prior value; current patch binding unproved.
- **Availability:** Disabled: client event handler/transport proof is absent.

## /gmshowcommandwaypoints

- **Sources:** TOML `Event_SlashCmd_ShowCommandWaypoints`; `docs/reverse-engineering/findings/npc-movement-pathfinding.md` identifies `GameProxyPlayer`'s **inbound** `onShowCommandWaypoints` callback; no outbound GM-tail method/arm.
- **Arguments:** Required `Boolean on/off`.
- **Target/scope:** Local command waypoint visualization, if implemented by the client event.
- **Preconditions and authority:** Inbound display callback is not an outbound slash handler; no server authorization path for this event is proven.
- **Effects and impact:** Possible transient local overlay. It does not author or change NPC paths.
- **Verification:** Observe overlay and client callback after a proven event; an inbound method by itself is not acceptance evidence.
- **Compensation:** Restore prior display state through a proven client toggle only.
- **Availability:** Disabled: slash-to-client callback binding is unproved.

## /gmshowspawnset

- **Sources:** TOML `Event_SlashCmd_ShowSpawnSet`; protocol table `activateSpawnSet` 214/`deactivateSpawnSet` 215 are distinct mutating server methods; `gm/spawn_sets.rs::handle_switch` handles those only, not this no-argument word.
- **Arguments:** None in TOML; `docs/commands.md`'s `<setId>` is a conflicting description.
- **Target/scope:** Unknown visualization/selection; no spawn-set ID can be carried by this schema.
- **Preconditions and authority:** No handler for this event; never substitute indices 214/215, which change live spawn-set state.
- **Effects and impact:** Unknown client-only effect; no verified mutation. The similarly named server methods can spawn/despawn NPCs but are not this command.
- **Verification:** No accepted-effect readback. Existing spawn-set snapshot may establish baseline only.
- **Compensation:** None for this unbound event; no inverse inferred from activate/deactivate.
- **Availability:** Disabled: missing parameter, event binding and corresponding handler.

## /gmshowarea

- **Sources:** TOML `Event_SlashCmd_ShowArea`; no matching method in the cell dispatch table or `gm/mod.rs::dispatch`; `docs/commands.md` attributes a local game display.
- **Arguments:** None.
- **Target/scope:** Local area overlay, if the client implements it; no selected server target.
- **Preconditions and authority:** Client event handler and role behavior unverified; no server handler.
- **Effects and impact:** Potential transient visualization only, not area/region authoring.
- **Verification:** Local visual inspection after proven event; no server readback or DB change.
- **Compensation:** No proven toggle polarity; closing/reloading an overlay is not a documented inverse.
- **Availability:** Disabled: effect and submission path are not source-proven.

## /gmshowregion

- **Sources:** TOML `Event_SlashCmd_ShowRegion`; no matching cell method/GM dispatch arm; `docs/commands.md` describes game-rendered regions without this typed argument.
- **Arguments:** Required `String Type`; quoting/accepted choices unknown.
- **Target/scope:** Unknown region type filter on local display; not a server region edit.
- **Preconditions and authority:** No validated `Type` choices or event handler; never turn an arbitrary string into a server query.
- **Effects and impact:** Possible transient overlay only.
- **Verification:** If proven, match the visible region type to the requested `Type`; no server receipt or durable readback is available.
- **Compensation:** No proven inverse/toggle syntax.
- **Availability:** Disabled: string semantics and client event behavior are unknown.

## /gmchat

- **Sources:** TOML `Event_SlashCmd_GMChat`; `docs/protocol/cell-method-dispatch-table.md` has distinct `sendGMShout` 222 and `crates/cell-console/src/cell/console/gm/shout.rs` backs that method; no `gmChat` method/arm.
- **Arguments:** Required `String Text`; quote/space serialization is unproved and text is sensitive in logs/exports.
- **Target/scope:** Audience unknown. Do not infer space/global broadcast or substitute `/gmsendgmshout`/`.announce`.
- **Preconditions and authority:** No actual GMChat server authorization or delivery handler evidenced.
- **Effects and impact:** Unverified social transmission; could disclose operator text if sent, so redact it in evidence.
- **Verification:** No known recipient or feedback channel; a local chat echo is not delivery proof.
- **Compensation:** None; a delivered message cannot be recalled.
- **Availability:** Disabled: no handler, audience proof or safe quoting path.

## /gmloadinteractionset

- **Sources:** TOML `Event_SlashCmd_LoadInteractionSet`; `docs/engine/cooked-data-pipeline.md` describes CookedInteractionSet data; `deprecated/python/cell/SGWPlayer.py::initialResponse` is an interaction selection, not a reload; no matching GM-tail method/arm.
- **Arguments:** None; no ID or scope to validate.
- **Target/scope:** Unknown resource/client cache; cannot claim one selected NPC, all clients or server seed.
- **Preconditions and authority:** No Cimmeria reload handler for this event.
- **Effects and impact:** Unknown. A cooked-data load would not itself edit committed seed, but even that load is unproved here.
- **Verification:** No handler receipt or cache generation readback; existing interaction choices are baseline only.
- **Compensation:** None.
- **Availability:** Disabled: absent handler and unspecified load scope.

## /gmpetinvokeability

- **Sources:** TOML `Event_SlashCmd_PetInvokeAbility`; ordinary SGWPlayer cell method 88 has `INT32 entityId, abilityId, targetId` in protocol table; `crates/cell-pets/src/cell/cell_methods/player/pet/invoke.rs::handle` and `tests/invoke.rs` implement owner/target/ability refusal.
- **Arguments:** Required `Integer entityId`, `Integer abilityId`; **TOML lacks method 88's `targetId`**. No implicit selected-target encoding is proven.
- **Target/scope:** If method 88 is reached, only caller-owned pet; explicit target must be valid/hostile/in-space.
- **Preconditions and authority:** Ordinary pet method uses owner checks, not GM-tail access. Missing third argument and event binding prevent this slash form from claiming that handler.
- **Effects and impact:** Proven method 88 can cast/engage transiently and apply combat damage/effects; slash event's effect remains unknown.
- **Verification:** For a separately proven method 88 call, correlate `pets.command` receipt, owner feedback/refusal, pet cast/effects and target state; no verification for this two-argument slash form.
- **Compensation:** None for damage/effects; a pet order is not safely undoable.
- **Availability:** Disabled: method signature mismatch and event binding unproved.

## /gmpetinvokecommand

- **Sources:** TOML `Event_SlashCmd_PetInvokeCommand`; protocol table lists pet ability/toggle/stance methods 88–90, no `petInvokeCommand`; `crates/cell-pets/src/cell/cell_methods/player/pet/mod.rs` registers only those methods.
- **Arguments:** Required `Integer entityId`, `Integer commandId`; permitted command IDs are not defined here.
- **Target/scope:** Unknown pet command; do not assume `entityId` is a caller-owned pet without a bound handler.
- **Preconditions and authority:** No server method or ownership gate for this event.
- **Effects and impact:** Unknown; no confirmed pet AI state mutation.
- **Verification:** No handler receipt, response or reliable state field for this slash word.
- **Compensation:** None.
- **Availability:** Disabled: no mapped handler or command-ID contract.

## /gmpetabilitytoggle

- **Sources:** TOML `Event_SlashCmd_PetAbilityToggle`; ordinary cell method 89 is `INT32 entityId, abilityId, INT8 toggle`; `crates/cell-pets/src/cell/cell_methods/player/pet/toggle.rs::handle` and `tests/toggle.rs` cover owner gate and list resend.
- **Arguments:** Required `Integer entityId`, `Integer abilityId`, `Integer toggle`; method 89 narrows `toggle` to `INT8` and only accepts 0/1. Event-to-method encoding is unproved.
- **Target/scope:** If method 89 is reached, caller-owned pet and one of its abilities.
- **Preconditions and authority:** Ordinary pet ownership check, not GM-tail authorization; missing/unowned pet, unknown ability and invalid toggle refuse.
- **Effects and impact:** Method 89 transiently changes pet `toggled_off` ability set and re-sends owner pet bar. No DB persistence proved; the slash event's effect is unverified.
- **Verification:** For proven method 89, compare pet `toggled_off` and owner `onPetAbilityList`, including refusal resend. Packet submission alone is not a state change.
- **Compensation:** A later 0/1 toggle can restore previous transient state if pet/ability still exists; no slash inverse established.
- **Availability:** Disabled: client event binding and Integer-to-INT8 serialization unproved.

## /gmchangeammo

- **Sources:** TOML `Event_SlashCmd_ChangeAmmo`; `crates/cell-methods/src/cell/cell_methods/inventory/dispatch.rs` routes a separate inventory `requestAmmoChange`, and `crates/cell-combat/src/cell/cell_methods/inventory/bandolier/ammo_change.rs` validates weapon compatibility; no `gmChangeAmmo` GM-tail method.
- **Arguments:** Required `Integer ammoType`; `docs/commands.md`'s two-argument example does not match the patch.
- **Target/scope:** No weapon instance is supplied by this slash schema. The ordinary inventory handler selects by a separately supplied item instance.
- **Preconditions and authority:** No proven binding to inventory request; do not fill missing weapon from current selection without evidence.
- **Effects and impact:** Unknown for slash. The separate inventory method changes a live bandolier slot and can alter later damage; it does not grant ammunition.
- **Verification:** No readback for this event. For the ordinary handler only, compare slot `cur_ammo_type` and owner ammo property after server acceptance.
- **Compensation:** None for this slash word; previous ammo type could be requested separately only if still compatible.
- **Availability:** Disabled: no corresponding GM method and missing weapon binding.

## /gmforceclientcrash

- **Sources:** TOML `Event_SlashCmd_ForceClientCrash`; `docs/commands.md` labels it game-handled; no outbound cell method/GM arm in protocol table.
- **Arguments:** None.
- **Target/scope:** Potentially the local client process, not another player/server.
- **Preconditions and authority:** Actual client event handler/fingerprint behavior unverified; not a safe debug action for the palette.
- **Effects and impact:** Potential local process termination, loss of unsaved in-memory context and capture interruption.
- **Verification:** A process exit would be observable, but no lab execution is permitted for this evidence packet; server silence cannot prove safe failure.
- **Compensation:** None; restart is recovery, not undo.
- **Availability:** Hidden/disabled: deliberately destructive and no bound handler proof.

## /gmforcerendercrash

- **Sources:** TOML `Event_SlashCmd_ForceRenderThreadCrash`; `docs/commands.md` labels it game-handled; no corresponding GM-tail server method/arm.
- **Arguments:** None.
- **Target/scope:** Potentially the local render thread/client only.
- **Preconditions and authority:** Actual client handler unverified; command must not be exposed as a runnable palette control.
- **Effects and impact:** Potential local render crash or process failure; no server character mutation evidenced.
- **Verification:** Crash/hang observation is intentionally not exercised; no positive server readback.
- **Compensation:** None.
- **Availability:** Hidden/disabled: destructive local action and unproved binding.

## /gmsetringdestination

- **Sources:** TOML `Event_SlashCmd_SetRingTransporterDestination`; ordinary SGWPlayer cell method 91 has two `INT32`s in protocol table; `crates/cell-methods/src/cell/cell_methods/player/world/mod.rs::dispatch` calls `ring_transport::handle_select_destination`; `crates/cell-content/src/cell/ring_transport/tests/mod.rs` covers success/refusal.
- **Arguments:** Required `Integer regionId`, `Integer destinationId`, matching method 91's width/order, but no event-to-method proof.
- **Target/scope:** If method 91 is reached, calling player at an active ring transport interaction; destination may be cross-world, not an arbitrary target.
- **Preconditions and authority:** Ordinary player method; ring state machine checks source/destination/occupancy and can refuse self/busy destinations. Slash prefix does not supply GM authority.
- **Effects and impact:** Proven method 91 starts transient ring transfer, potentially a cross-world relocation; no durable character rewrite is implied by the slash word itself.
- **Verification:** For proven method 91 correlate `ring_transport.select_destination`/arrival state, player world/position and any transfer packet, not just outgoing call or feedback. This event is not yet verified.
- **Compensation:** Manual return travel only; no automatic inverse for concurrent ring state or triggered content.
- **Availability:** Disabled: native slash event binding unproved despite an implemented ordinary method.

## /gmdumpobjects

- **Sources:** TOML `Event_SlashCmd_DumpObjects`; no matching GM-tail method/arm; `docs/commands.md` says object dump to log but does not show handler.
- **Arguments:** None.
- **Target/scope:** Unknown local object registry or server objects; no audience/scope evidence.
- **Preconditions and authority:** No bound handler, output destination or data classification proven.
- **Effects and impact:** Potential read/debug output; object names/addresses might be sensitive, so do not export an unreviewed dump.
- **Verification:** No defined output/readback; a generic log line is not proof of complete inventory.
- **Compensation:** None.
- **Availability:** Disabled: object scope, handler and redaction unresolved.

## /gmspacequeuestatus

- **Sources:** TOML `Event_SlashCmd_SpaceQueueStatus`; SGWPlayer **base** method `onSpaceQueueStatus` index 26 in `docs/protocol/sgwplayer-base-method-dispatch-table.md`; `deprecated/python/base/SGWPlayer.py::onSpaceQueueStatus` is a trace stub; no GM-tail method.
- **Arguments:** None.
- **Target/scope:** Calling player's space queue, if that base method is the intended binding; no global queue query proven.
- **Preconditions and authority:** Base method requires authenticated player/session; no evidence this slash event sends it or that Cimmeria implements a queue status response.
- **Effects and impact:** Unknown query; do not call it read-only simply from its name because queue status may affect protocol state.
- **Verification:** No reliable queue snapshot or feedback for this event; base method receipt would need to be tied to the exact caller.
- **Compensation:** None.
- **Availability:** Disabled: event-to-base binding and effective handler missing.

## /gmspacequeuedresponse

- **Sources:** TOML `Event_SlashCmd_SpaceQueuedResponse`; SGWPlayer base method 28 `onSpaceQueuedResponse(INT8 accept)` in protocol table; legacy `deprecated/python/base/SGWPlayer.py` method is a trace stub; no GM-tail method.
- **Arguments:** None in TOML, versus base method's required `INT8 accept`; `docs/commands.md` supplies an unproven response argument.
- **Target/scope:** Caller queue prompt only if bound; no selected player.
- **Preconditions and authority:** Missing acceptance value and binding; never synthesize acceptance from UI defaults.
- **Effects and impact:** Potential queue decision or travel, but no implemented slash effect established.
- **Verification:** No current readback; if implemented require exact queue state/transfer outcome and base receipt for same session.
- **Compensation:** None; accepting/declining a queue position need not be reversible.
- **Availability:** Disabled: argument mismatch and unproved base handler.

## /gmspacequeuereadyresponse

- **Sources:** TOML `Event_SlashCmd_SpaceQueueReadyResponse`; SGWPlayer base method 27 `onSpaceQueueReadyResponse(INT8 accept)` in protocol table; legacy base implementation traces only.
- **Arguments:** None in TOML; base method requires `INT8 accept`.
- **Target/scope:** Calling player's ready prompt if mapped; no other target.
- **Preconditions and authority:** No response bit, binding or server queue state machine proof.
- **Effects and impact:** Potential queue readiness/transfer decision, unverified for this slash event.
- **Verification:** No accepted-effect readback; future implementation would compare queue state and actual destination after base receipt.
- **Compensation:** None.
- **Availability:** Disabled: missing parameter and unproved base routing.

## /gmconfirmeffect

- **Sources:** TOML `Event_SlashCmd_ConfirmEffect`; ordinary `SGWAbilityManager.confirmationResponse` cell index 4 takes `INT32 aEffectId, UINT8 aAccepted` in protocol table; `crates/cell-methods/src/cell/cell_methods/ability_manager.rs::dispatch` currently **logs only**; `docs/protocol/message-catalog.md` names `Event_NetOut_ConfirmEffect` separately.
- **Arguments:** Required `Integer EffectId`, `Boolean Response`; type/order resemble method 4 but slash-event-to-NetOut binding is unproved.
- **Target/scope:** Caller and one pending effect confirmation; whether `EffectId` is definition or instance ID is explicitly unproved in handler comment.
- **Preconditions and authority:** Ordinary method 4, no GM-tail gate. No server effect application or pending-confirmation check in current handler.
- **Effects and impact:** Current Cimmeria logs the response and returns; it does not commit/deny the effect. Any original-client behavior beyond that is unknown.
- **Verification:** Correlate `confirmationResponse` log of exact caller/effect/accepted byte with unchanged effect state; a log receipt is not accepted effect state.
- **Compensation:** None; no implemented mutation to reverse.
- **Availability:** Disabled: event binding unknown and server handler is receipt-only.

## /gmshowtriggers

- **Sources:** TOML `Event_SlashCmd_ShowTriggers`; no matching GM-tail cell method/arm; `docs/commands.md` calls it a game overlay without a handler trace.
- **Arguments:** None.
- **Target/scope:** Local trigger visualization, if present; not a content trigger mutation.
- **Preconditions and authority:** Actual client event handler and exposure unverified; no server action.
- **Effects and impact:** Potential transient overlay only; no trigger firing is evidenced.
- **Verification:** Observe client overlay after proven event; no durable server readback.
- **Compensation:** No proven toggle/restore API.
- **Availability:** Disabled: event effect and submission path unproved.

## /gmshowwaypoints

- **Sources:** TOML `Event_SlashCmd_ShowWaypoints`; `docs/protocol/client-method-dispatch-table.md` lists **inbound** `SGWGmPlayer.onShowWaypoints` 158 with waypoint payload; no matching outbound GM-tail handler.
- **Arguments:** None.
- **Target/scope:** Potential local waypoint display; no point-set name or radius in slash schema.
- **Preconditions and authority:** Inbound server-to-client method is not evidence that this slash word requests data.
- **Effects and impact:** Unknown/transient visualization, no waypoint authoring.
- **Verification:** If proven, compare visible points and inbound `onShowWaypoints` payload; no write readback.
- **Compensation:** No verified disable semantics.
- **Availability:** Disabled: request/binding and response source unknown.

## /gmshownavmesh

- **Sources:** TOML `Event_SlashCmd_ShowNavMesh`; protocol client method `SGWGmPlayer.onShowNavigation` 162 is inbound with polygons; server `gmShowNavigation` 211 is only **ADAPT**, with no `gm/mod.rs::dispatch` arm.
- **Arguments:** None; candidate server method 211 expects `INT8 onOff`, so there is a schema mismatch.
- **Target/scope:** Potential local navmesh overlay for current chunk, not a navmesh edit.
- **Preconditions and authority:** No implemented request/response bridge; do not treat inbound 162 as an outbound action.
- **Effects and impact:** No Cimmeria effect; a future overlay is transient and could disclose navigation data.
- **Verification:** No current polygon readback; future proof requires exact request, server callback and rendered chunk match.
- **Compensation:** None until toggle semantics are known.
- **Availability:** Disabled: server method unimplemented and Boolean/request mapping absent.

## /gmshowmemory

- **Sources:** TOML `Event_SlashCmd_ShowMemory`; no matching GM-tail method/arm; `docs/commands.md` says game-handled but gives no callback/metric contract.
- **Arguments:** None.
- **Target/scope:** Potential local client memory display; no server-process memory query is proven.
- **Preconditions and authority:** Client handler and scope unverified. Never export raw addresses, heap content or account data by default.
- **Effects and impact:** Possible transient diagnostic display, not a server mutation.
- **Verification:** If proven, inspect client-local metric source and timestamp; there is no server readback for this event.
- **Compensation:** No proven toggle syntax.
- **Availability:** Disabled: metric source, client binding and output sensitivity unresolved.

## /gmdebugtarget

- **Sources:** TOML `Event_SlashCmd_DebugTarget`; no `gmDebugTarget` row/arm in the GM cell table; `docs/commands.md` calls it a target dump but supplies a `<target>` argument absent from TOML. Separate `gmShowPlayer` 131 and `gmDebugMobData` 180 have implemented query handlers in `crates/cell-console/src/cell/console/gm/query.rs::{handle_show_player,handle_debug_mob_data}` with refusal tests in `gm/tests/query.rs`; they are not this event.
- **Arguments:** None in TOML; no explicit target ID or quoting.
- **Target/scope:** Unknown local selected target, if any; selection must be recaptured at submit time. Do not substitute 131/180 or trust a stale UI target.
- **Preconditions and authority:** No bound server handler or proven client-side debug callback; a future server inspection must pass GM gate and same-space target checks.
- **Effects and impact:** Intended read/debug query, but actual output and side effects unknown. Sensitive names/positions in any future dump require redaction in exported evidence.
- **Verification:** No current reply. For a future target query, compare response entity ID and fields with a fresh `server_entity_get` snapshot for that same entity/session; a chat echo alone is not proof.
- **Compensation:** None for a read, provided no debug mode is toggled; that precondition is not yet verified.
- **Availability:** Disabled: no handler, no target argument and no client event binding.
