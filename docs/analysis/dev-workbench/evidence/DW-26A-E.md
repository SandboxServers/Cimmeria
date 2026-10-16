# DW-26A-E: slash commands 55–81

The headings below are exactly entries 55–81 of `data/client-patches/012-gm-slash-commands/commands.toml`. Its `Event_SlashCmd_*` identifiers and mandatory `params` describe the project-written client patch, **not** a verified event-to-cell-method binding. `docs/protocol/cell-method-dispatch-table.md` supplies separate method candidates and signatures. The stock slash XML is absent (`docs/analysis/dev-workbench/stock-slash-audit.md`); native submission and quoting remain unproved (DW-24B). Thus every palette entry is **disabled**, including those with working server handlers. No slash-to-dot fallback is safe.

All 27 TOML entries inherit `access = "p"`: the patched client role mask is `0x2FF`, admitting a nonzero client access level, not granting GameMaster. `data/client-patches/README.md` explains the parser defect. `crates/cell/src/cell/dispatch/router.rs::dispatch_cell_method` calls `crates/cell-world/src/cell/dispatch/gm_gate.rs::enforce_gm_gate` before plugins and handlers; the whole `SGWGmPlayer` method tail (109+) requires server-sourced `access_level >= GameMaster`, including the **non-`/gm`** index 191 `debugMinigameComplete`. A missing/low-level caller is refused. A method with no `gm/mod.rs::dispatch` arm then reaches the router's unhandled warning, with no claimed effect. `crates/lab/src/workbench/slash_catalog.rs::method_candidate` marks all mappings `binding_verified: false`. None of the TOML fields specify numeric bounds or string quoting; below, “bounds/quoting unproved” means neither may be invented by the palette. No command in this slice declares optional parameters.

## /gmreloadinventory

- **Sources:** TOML `Event_SlashCmd_ReloadInventory`; dispatch table index 165 `gmReloadInventory()` NEW; no `gm/mod.rs::dispatch` arm.
- **Arguments:** None.
- **Target/scope:** Intended inventory definitions or caller inventory is unknown; neither is implemented by this method.
- **Preconditions and authority:** Client patch role filter plus server GM gate if index 165 is emitted; no hot-reload primitive follows.
- **Effects and impact:** No implemented inventory reload; never treat existing inventory snapshot or vendor refresh as this command's effect.
- **Verification:** `server_db_query`/inventory snapshot may establish baseline only; no accepted reload readback exists.
- **Compensation:** None; no implemented mutation.
- **Availability:** Disabled: native binding/submission and handler absent; target semantics unknown.

## /gmusers

- **Sources:** TOML `Event_SlashCmd_Users`; table index 166 DONE; `crates/cell-console/src/cell/console/gm/query.rs::handle_users`.
- **Arguments:** None.
- **Target/scope:** Players in the caller's current **space**, not all shard users.
- **Preconditions and authority:** Server GM gate and live caller space; handler reports “not in a space” otherwise.
- **Effects and impact:** Read-only; sorts entity IDs and optional character IDs for feedback to caller.
- **Verification:** Compare GM feedback `gmUsers (N in space)` with a contemporaneous `server_entity_query` for the caller's space, counting player entities and checking `capped`; `server_sessions` has no space ID and cannot establish this count alone. No persistent data changes.
- **Compensation:** None needed.
- **Availability:** Disabled until native event binding and submission are proved; server handler exists.

## /gmsethidegm

- **Sources:** TOML `Event_SlashCmd_SetHideGM`; table index 167 NEW; no `gm/mod.rs::dispatch` arm.
- **Arguments:** Required `Boolean on/off`; TOML gives no encoding beyond Boolean, bounds or quoting.
- **Target/scope:** Presumed caller visibility, but actual server target and wire behavior are unknown.
- **Preconditions and authority:** Client role filter and server GM gate; no `bHideGM` server state is implemented.
- **Effects and impact:** No implemented hide/show; login-sourced `access_level` is not altered.
- **Verification:** No authoritative hide-state readback exists; client appearance alone would not prove a server mutation.
- **Compensation:** None; no implemented mutation.
- **Availability:** Disabled: native binding/submission and handler absent.

## /gmprintstats

- **Sources:** TOML `Event_SlashCmd_PrintStats`; table index 168 ADAPT; no `gm/mod.rs::dispatch` arm.
- **Arguments:** Required `String Stat`; allowed values, bounds and quoting unknown.
- **Target/scope:** Stat selection is named; whether the client targets self, a selection or server-wide counters is unverified.
- **Preconditions and authority:** Client role filter and server GM gate; no index-168 handler.
- **Effects and impact:** No implemented print; `stat_list.rs` is only a possible server primitive, not a routed effect.
- **Verification:** No method-specific feedback; stat snapshots can establish baseline only.
- **Compensation:** None; no implemented mutation.
- **Availability:** Disabled: no binding/submission proof or handler; target contract unknown.

## /gmdebugability

- **Sources:** TOML `Event_SlashCmd_AbilityDebug`; table index 169 DONE; `gm/combat_debug.rs::handle_ability_toggle`, `cell/combat_debug/commands.rs::toggle_ability`, `cell/combat_debug/tests.rs`.
- **Arguments:** Required `Integer AbilityId`; client parser range unknown. Server reads signed INT32 `aAbilityId`; `0` clears, other IDs must exist in cell ability definitions.
- **Target/scope:** Caller debug state; casts by/on caller, not a grant of the ability.
- **Preconditions and authority:** GM gate; live caller and known ability for nonzero ID; malformed/unknown IDs get refusal feedback.
- **Effects and impact:** Transient toggle in `debugAbilityList`, or clear list on `0`; later debug lines expose combat detail to caller.
- **Verification:** First-press `gmDebugAbility` feedback on/off/clear and subsequent combat-debug lines; no persistent DB readback or stable query for the toggle.
- **Compensation:** Repeat toggle or send `0` to clear, once native submission is proven; no general combat-log deletion.
- **Availability:** Disabled pending native binding/submission and integer formatting proof; server handler exists.

## /gmdebugcombat

- **Sources:** TOML `Event_SlashCmd_CombatDebug`; table index 170 DONE; `gm/combat_debug.rs::handle_toggle`, `cell/combat_debug/commands.rs::toggle`.
- **Arguments:** None.
- **Target/scope:** Caller combat-debug stream.
- **Preconditions and authority:** GM gate and live caller; no selected target.
- **Effects and impact:** Transient on/off toggle; hit roll and damage lines may later print to caller.
- **Verification:** `gmDebugCombat` first-press on/off feedback; later combat lines corroborate active state, not persistence.
- **Compensation:** Repeat toggle after reading current feedback; no DB rollback.
- **Availability:** Disabled pending native binding/submission; handler exists.

## /gmdebugcombatverbose

- **Sources:** TOML `Event_SlashCmd_CombatDebugVerbose`; table index 171 DONE; `gm/combat_debug.rs::handle_toggle`, `cell/combat_debug/commands.rs::toggle`.
- **Arguments:** None.
- **Target/scope:** Caller verbose combat-debug stream.
- **Preconditions and authority:** GM gate and live caller.
- **Effects and impact:** Transient toggle; adds effect plans, NVP damage, ledger and pulse detail to later hit lines.
- **Verification:** `gmDebugCombatVerbose` on/off feedback, then verbose combat lines when a hit occurs; no DB readback.
- **Compensation:** Repeat toggle after checking feedback.
- **Availability:** Disabled pending native binding/submission; handler exists.

## /gmdebugheal

- **Sources:** TOML `Event_SlashCmd_HealDebug`; table index 172 DONE; `gm/combat_debug.rs::handle_toggle`, `cell/combat_debug/commands.rs::toggle`.
- **Arguments:** None.
- **Target/scope:** Caller heal/buff debug stream.
- **Preconditions and authority:** GM gate and live caller.
- **Effects and impact:** Transient toggle; later heal/buff details print to caller, no health mutation by the toggle.
- **Verification:** `gmDebugHeal` on/off feedback and later heal-debug lines; no persistent readback.
- **Compensation:** Repeat toggle after checking feedback.
- **Availability:** Disabled pending native binding/submission; handler exists.

## /gmdebugstartminigame

- **Sources:** TOML `Event_SlashCmd_DebugStartMinigame`; table index 173 ADAPT; no tail handler in `gm/mod.rs::dispatch`. `crates/cell-methods/src/cell/cell_methods/minigame.rs::dispatch` has a **different**, inherited index-20 `debugStartMinigame` logging stub.
- **Arguments:** Required `Integer gameId`; TOML has no ID range or quoting rule. Candidate method takes INT32.
- **Target/scope:** Caller minigame session intent; no implemented tail effect.
- **Preconditions and authority:** GM gate covers index 173 and inherited debug index 20; neither starts a game through this slash word.
- **Effects and impact:** Unknown client event binding; no implemented start. `base/world_entry/cell_dispatch/minigame.rs::start_minigame` is reached by a separate content-triggered `StartMinigame` message and ticket registration, not by index 173.
- **Verification:** `server_minigame_state` and `server_minigame_tap_read` can show an independently started session only; neither certifies this unhandled method.
- **Compensation:** None; no implemented slash mutation. A real session has a separate cancel path.
- **Availability:** Disabled: binding/submission and handler absent; never substitute the index-20 stub or a dot command.

## /gmdebugspectateminigame

- **Sources:** TOML `Event_SlashCmd_DebugSpectateMinigame`; table index 174 ADAPT; no tail handler; `cell_methods/minigame.rs::dispatch` index-21 debug spectator is a separate logging stub.
- **Arguments:** None.
- **Target/scope:** Spectator/session target unknown; no selected player parameter in TOML.
- **Preconditions and authority:** Both debug indices are GM gated; no working spectator routing is shown.
- **Effects and impact:** No implemented spectate transition for index 174.
- **Verification:** No method-specific readback; a minigame session snapshot cannot attribute success to this call.
- **Compensation:** None; no implemented mutation.
- **Availability:** Disabled: binding/submission and handler absent.

## /gmdebugjoinminigame

- **Sources:** TOML `Event_SlashCmd_DebugJoinMinigame`; table index 175 ADAPT; no tail handler; `cell_methods/minigame.rs::dispatch` inherited index-22 debug join is a separate logging stub.
- **Arguments:** None.
- **Target/scope:** Join target/session unknown; no argument in patch.
- **Preconditions and authority:** GM gate covers both debug indices; no working join handler.
- **Effects and impact:** No implemented join transition for index 175.
- **Verification:** No accepted-effect readback; minigame presence alone cannot prove this path.
- **Compensation:** None; no implemented mutation.
- **Availability:** Disabled: binding/submission and handler absent.

## /gmdebugabilityonmob

- **Sources:** TOML `Event_SlashCmd_DebugAbilityOnMob`; table index 176 DONE; `gm/combat_debug.rs::handle_ability_toggle`, `cell/combat_debug/commands.rs::toggle_mob`, its tests.
- **Arguments:** Required `Integer AbilityID`; candidate reads INT32. `0` means all selected mob abilities; nonzero ID must exist. Client numeric bounds unknown.
- **Target/scope:** **Selected NPC** in caller's space; handler rejects no target, player target and other-space target.
- **Preconditions and authority:** GM gate; live caller, valid selected NPC, known nonzero ability.
- **Effects and impact:** Transient debug subscription for the selected mob's casts; no ability grant or mob mutation.
- **Verification:** First-press `gmDebugAbilityOnMob` feedback plus later lines tied to selected mob/ability; tests cover invalid selection/unknown ability. No DB state.
- **Compensation:** Repeat on the same mob/ability to remove; `gmDebugAbility 0` clears caller debug lists.
- **Availability:** Disabled pending native binding/submission and selection/argument encoding proof; handler exists.

## /gmdebugbehaviorsonmob

- **Sources:** TOML `Event_SlashCmd_DebugBehaviorsOnMob`; table index 177 ADAPT; no `gm/mod.rs::dispatch` arm.
- **Arguments:** None.
- **Target/scope:** Name suggests selected mob, but selection and output route are unverified.
- **Preconditions and authority:** GM gate; no handler-level target check exists.
- **Effects and impact:** No behavior-debug stream is implemented by this method; `ai_state`/threat reads in table are proposals.
- **Verification:** `server_entity_get` may show AI baseline, not command acceptance or a debug stream.
- **Compensation:** None; no implemented mutation.
- **Availability:** Disabled: native binding/submission and handler absent; target policy unknown.

## /gmdebugpathsonmob

- **Sources:** TOML `Event_SlashCmd_DebugPathsOnMob`; table index 178 ADAPT; no `gm/mod.rs::dispatch` arm.
- **Arguments:** None.
- **Target/scope:** Intended selected mob path, but no verified selection contract.
- **Preconditions and authority:** GM gate; no path-debug handler.
- **Effects and impact:** No implemented path stream or `onShowPath` callback from this index.
- **Verification:** `server_entity_get` position/path data is diagnostic baseline only.
- **Compensation:** None; no implemented mutation.
- **Availability:** Disabled: binding/submission and handler absent.

## /gmdebugevents

- **Sources:** TOML `Event_SlashCmd_DebugEvents`; table index 179 NEW; no `gm/mod.rs::dispatch` arm.
- **Arguments:** Required `Integer TargetId`, `Integer InformLevel` in that order; TOML has no bounds, level enum or quoting rule.
- **Target/scope:** Numeric target ID named, but whether it is entity/player/event source is unknown.
- **Preconditions and authority:** GM gate; no implemented target or level validation.
- **Effects and impact:** Unknown; no debug event subscription is routed.
- **Verification:** No accepted-effect readback; event logs alone would not prove subscription.
- **Compensation:** None; no implemented mutation.
- **Availability:** Disabled: native binding/submission, handler and level semantics absent.

## /gmmobdata

- **Sources:** TOML `Event_SlashCmd_MobData`; table index 180 `gmDebugMobData` DONE; `gm/query.rs::handle_debug_mob_data`.
- **Arguments:** Required `Integer SpaceId`, `Integer TargetId` in TOML order; server reads two INT32 values, uses second as target and does not use first as authority. Client bounds unknown.
- **Target/scope:** Explicit target entity in caller's current space; method does **not** require it to be a mob despite the word.
- **Preconditions and authority:** GM gate; valid in-range target and same-space check; no such entity/other space returns feedback.
- **Effects and impact:** Read-only feedback: ID, name/template, AI, faction, level, health, threat count, tag.
- **Verification:** Compare feedback with `server_entity_get` for the exact entity/space at capture time; ephemeral fields can age between reads.
- **Compensation:** None needed.
- **Availability:** Disabled pending native binding/submission and two-integer formatting; server handler exists.

## /gmdebuginteract

- **Sources:** TOML `Event_SlashCmd_DebugInteract`; table index 181 NEW; no `gm/mod.rs::dispatch` arm.
- **Arguments:** None.
- **Target/scope:** Interaction subject unknown; patch gives no target.
- **Preconditions and authority:** GM gate; no handler-side selection check.
- **Effects and impact:** Unknown intended debug effect; no implemented interaction call.
- **Verification:** No method-specific readback exists.
- **Compensation:** None; no implemented mutation.
- **Availability:** Disabled: binding/submission and handler absent.

## /gmemitbehavioreventonmob

- **Sources:** TOML `Event_SlashCmd_EmitBehaviorEventOnMob`; table index 182 NEW; no `gm/mod.rs::dispatch` arm.
- **Arguments:** Required `Integer EventID`; TOML has no event catalog, range or quoting rule.
- **Target/scope:** Mob selection implied by word but unverified and unchecked.
- **Preconditions and authority:** GM gate only; no validated NPC/event path.
- **Effects and impact:** No implemented event emission; hypothetical AI/content effects must not be claimed.
- **Verification:** No accepted-effect readback; AI state/log baseline cannot prove emission.
- **Compensation:** None; future event effects may be irreversible, so no inverse is assumed.
- **Availability:** Disabled: binding/submission, handler, event catalog and target contract absent.

## /gmaddbehavioreventset

- **Sources:** TOML `Event_SlashCmd_AddBehaviorEventSet`; table index 183 NEW; no `gm/mod.rs::dispatch` arm. Patch 012 deliberately keeps the later QA duplicate with no parameters (`data/client-patches/README.md`).
- **Arguments:** None in canonical TOML; do not borrow the earlier duplicate's parameter shape.
- **Target/scope:** Event set and mob target unknown.
- **Preconditions and authority:** GM gate; no handler-side event-set or target validation.
- **Effects and impact:** No implemented set attachment; content mutation intent cannot be inferred as persistence.
- **Verification:** No accepted-effect readback exists.
- **Compensation:** None; no implemented mutation and no proven inverse.
- **Availability:** Disabled: native binding/submission and handler absent; intended target/set unknown.

## /gmremovebehavioreventset

- **Sources:** TOML `Event_SlashCmd_RemoveBehaviorEventSet`; table index 184 NEW; no `gm/mod.rs::dispatch` arm. Canonical patch entry is the later no-parameter QA duplicate.
- **Arguments:** None; no ID field may be invented from the word.
- **Target/scope:** Event set and mob target unknown.
- **Preconditions and authority:** GM gate; no handler-side validation.
- **Effects and impact:** No implemented set removal; effects of a future implementation unknown.
- **Verification:** No accepted-effect readback exists.
- **Compensation:** None; no implemented mutation or proven inverse.
- **Availability:** Disabled: native binding/submission and handler absent.

## /gmspawnbycmd

- **Sources:** TOML `Event_SlashCmd_Spawn`; table index 185 `gmSpawnByCmd` DONE is a **semantic candidate**, not a proven binding (`slash_catalog.rs::method_candidate`). `gm/spawn.rs::handle_spawn_by_cmd`, `gm/tests/spawn.rs`.
- **Arguments:** Required `String DesignId`, `Float X-Offset`, `Float Z-Offset` in order; no TOML bounds/quoting. Server accepts only a positive numeric template ID encoded as WSTRING and finite FLOAT offsets.
- **Target/scope:** New NPC at caller's X/Z offsets, same Y/current world/space; no selected target.
- **Preconditions and authority:** GM gate; live caller/world, valid numeric template and finite position; base checks template before Cell spawns.
- **Effects and impact:** Transient world NPC spawn; handler queues `GmSpawnNpc`, base resolves template, Cell confirms spawn. Queue acceptance is not spawn success.
- **Verification:** Wait for base/cell completion feedback, then `server_entity_get`/space roster for returned entity ID, template and position; `gm/tests/spawn.rs` pins queued message shape.
- **Compensation:** Manual NPC despawn by verified ID only; side effects of any triggered behavior are not rolled back.
- **Availability:** Disabled until client `Event_SlashCmd_Spawn` → index-185 binding and native submission/quoting are proved; server handler exists.

## /gmdespawn

- **Sources:** TOML `Event_SlashCmd_Despawn`; table index 186 `gmDespawnByCmd` DONE; `gm/world.rs::handle_despawn`, `gm/tests/world.rs`.
- **Arguments:** None in TOML. Candidate server method takes INT32 TargetID; observed stock slash sends `0`, which handler interprets as current selection (`gm/world.rs`). The patch event's encoding is unproved.
- **Target/scope:** Selected NPC in caller's space if wire `0`; explicit positive ID only if separately supplied by a verified native binding.
- **Preconditions and authority:** GM gate; server rejects absent, player or other-space target and rechecks despawn primitive.
- **Effects and impact:** Transient removal from spatial grid with immediate witness `LeftAoI` and combat release; may disrupt players fighting that NPC.
- **Verification:** Success feedback plus absent entity in `server_entity_get`/space roster and witness disappearance; refusal feedback is not success.
- **Compensation:** Manual respawn from known template/position, not exact state restoration; no automatic inverse.
- **Availability:** Disabled: patch event binding and argument/sentinel behavior remain unproved despite implemented method.

## /gmrechargeitem

- **Sources:** TOML `Event_SlashCmd_RechargeItem`; table index 187 ADAPT; no `gm/mod.rs::dispatch` arm. Separate player vendor path: `crates/base-methods/src/base/world_entry/methods/vendor/recharge.rs::handle_recharge_inventory_items` → `paid_recharge::handle_paid_recharge_inventory_items`, with live-DB refusal/success tests in those modules.
- **Arguments:** Required `Integer ItemId`; TOML has no range/quoting. Candidate method takes INT32 `itemId`; inventory row ID versus design ID is unverified for this slash event.
- **Target/scope:** Intended item target unknown. Do **not** equate the separate vendor handler's `sgw_inventory.item_id` with this command without a GM cell-to-base route.
- **Preconditions and authority:** GM gate at index 187, then unhandled warning. The existing `rechargeItems` player path independently requires an open server-sourced vendor template, carried item, recharge list, positive price and sufficient balance.
- **Effects and impact:** **No free GM recharge implemented.** Paid vendor recharge, if separately requested, transactionally restores charges and debits cash; its rules are not the slash command's effects.
- **Verification:** For vendor path only, compare exact `sgw_inventory.charges` and `sgw_player.cash` plus vendor telemetry/feedback after DB commit. No index-187 effect/readback exists.
- **Compensation:** None for unimplemented GM call; paid recharge has no proven automatic refund/charge inverse.
- **Availability:** Disabled: native binding/submission and GM handler absent; do not expose a free recharge or substitute vendor flow.

## /gmsetmobattribute

- **Sources:** TOML `Event_SlashCmd_SetMobAttribute`; table index 188 ADAPT; no `gm/mod.rs::dispatch` arm. `gm/query.rs::handle_get_mob_attribute` is read-only and is not a setter.
- **Arguments:** Required `String attribute`, `String attributeType`, `Integer value`; TOML order. Candidate method instead declares `INT32 target, WSTRING attr, WSTRING type, INT32 val`: missing target and string-vs-integer position mismatch. Bounds/quoting unknown.
- **Target/scope:** Mob selection/target unverified; patch contains no explicit target.
- **Preconditions and authority:** GM gate, but no handler validates target or attribute whitelist for index 188.
- **Effects and impact:** No implemented attribute mutation; do not infer a reflection setter from `get_entity_mut` availability.
- **Verification:** `server_entity_get`/stat query can establish baseline, not success for this unhandled method.
- **Compensation:** None; no implemented mutation or proven inverse.
- **Availability:** Disabled: native binding/argument mapping and handler absent.

## /gmrespawn

- **Sources:** TOML `Event_SlashCmd_Respawn`; table index 189 DONE; `gm/world.rs::handle_respawn_cmd`, `gm/tests/world.rs`.
- **Arguments:** None.
- **Target/scope:** Calling GM player only, not selected target.
- **Preconditions and authority:** GM gate; caller must have `player_id`, otherwise handler refuses. Canonical respawn resolves current-world respawner (fallback documented in handler).
- **Effects and impact:** Transient, potentially disruptive position/life reset: health/focus refill, death flags clear, pawn re-anchor and related broadcasts.
- **Verification:** Handler feedback plus `server_entity_get` position/life/stat fields and client position after authoritative snap; test covers player and nonplayer paths.
- **Compensation:** No exact inverse for health/position/combat state; manual restore only with known prior state.
- **Availability:** Disabled pending native binding/submission; server handler exists.

## /gmkilltarget

- **Sources:** TOML `Event_SlashCmd_Kill`; table index 190 DONE; `gm/world.rs::handle_kill_target`, `gm/tests/world.rs`; protocol table records stock sentinel `-1` behavior.
- **Arguments:** Required `Integer TargetId` in TOML, with no numeric bound. Candidate method takes **INT64**; observed stock slash sends `-1` and handler uses current selection. Patch event's argument conversion is unproved.
- **Target/scope:** NPC in caller's space; explicit valid ID wins if truly sent directly. Server rejects players, absent/other-space targets and invalid IDs.
- **Preconditions and authority:** GM gate; live NPC target, not already dead. Selection must be rechecked server-side at execution.
- **Effects and impact:** Irreversible encounter mutation: zeros NPC health, then canonical death sequence, loot/threat effects and witness broadcasts; no XP grant by this GM kill.
- **Verification:** Success feedback plus exact NPC `health=0`, dead state and event/loot evidence; `gm/tests/world.rs` covers kill and player refusal. No success claim from a submitted slash line.
- **Compensation:** No inverse for death/loot/content side effects; manual fresh spawn is a different entity.
- **Availability:** Disabled pending patch binding, INT32-to-INT64/sentinel behavior and native submission proof; server handler exists.

## /debugminigamecomplete

- **Sources:** TOML `Event_SlashCmd_MinigameComplete`; table index **191 `debugMinigameComplete(INT32 resultCode)` ADAPT**, with no `gm/mod.rs::dispatch` arm. `docs/analysis/dev-workbench/stock-slash-audit.md` records guide drift: `docs/commands.md` lists three arguments, TOML lists one. Actual game-result path is `crates/minigame/src/minigame/server/{handshake.rs,session_loop.rs,result_dispatch.rs}` → `CellToBaseMsg::MinigameResult` → `crates/base-world-entry/src/base/world_entry/cell_dispatch/minigame.rs::minigame_result` → `crates/cell/src/cell/service/base_messages/minigame.rs::handle_minigame_result`.
- **Arguments:** Required **one** `Integer ResultCode` per TOML; no `gameId`, `winnerId` or `loserId`. Method candidate expects INT32; client range/encoding unverified. SmartFox result codes 0 cancel, 1 victory, 2 defeat are separate server-side output values, not proof that this slash integer is accepted.
- **Target/scope:** Non-`/gm` debug word in the project patch, but its candidate method is in the SGWGmPlayer tail and affects only a caller's minigame if ever implemented; exact native binding unknown.
- **Preconditions and authority:** Patched client `access="p"` filter plus **server GM gate for index 191**. No method-191 handler exists. Actual minigame results require a registered ticket and authenticated/claimed SmartFox connection; `handshake.rs` rejects wrong/reused tickets. `session_loop.rs` passes extension requests to `game.message`. `crates/minigame/src/minigame/games/mod.rs::validation` says **Livewire is server validated; placeholder games accept a client-declared victory**. Ticket authentication alone is not outcome validation.
- **Effects and impact:** No implemented result completion from this slash word. The separate minigame server sends ticketed game outcomes; base checks replacement ticket before closing UI, then Cell fires victory chains only on result `1` (see `base_messages/minigame.rs` and tests). Never route a client-provided slash ResultCode directly to `MinigameResult` or victory chains.
- **Verification:** `server_minigame_tap_read` retained outcome, `minigame` result logs, `onEndMinigame` and victory-chain evidence can verify the **ticketed game path**, not this unhandled slash index. No command-specific readback exists.
- **Compensation:** None for unimplemented slash call; a genuine victory chain may have irreversible content effects.
- **Availability:** Disabled: native binding/one-argument parsing and server handler unproved/absent. Do not map to `endCurrentMinigame` (cancel), a dot command or a fabricated victory.
