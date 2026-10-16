# DW-25B-E: slash entries 28–54

This is the source contract for DW-25B, not live acceptance. The inventory is the 27 `[[command]]` entries from `/gmgiverespawner` through `/gmreloadorganizations` in `data/client-patches/012-gm-slash-commands/commands.toml`, in source order. Each `Arguments` field transcribes the TOML spelling and type; the cell wire width can be narrower. “Disabled” describes the proposed palette action, not whether the native slash word is present in a patched client.

## Shared boundary and readback

All 27 patch entries declare `access = "p"` and an `Event_SlashCmd_*` client event. The client consumes `/` locally. The patch and event spelling do **not** prove that its handler emitted the corresponding `SGWGmPlayer` cell method. `docs/analysis/dev-workbench/stock-slash-audit.md` records missing stock XML and `binding_verified=false`; DW-24B still owes native submission, quoting and outgoing-method proof. Accordingly every action below is disabled even when the candidate server method is implemented. There is no dot-command fallback. `docs/architecture/gm-cell-method-gating.md` explains that the patch role mask is cosmetic; `crates/cell-world/src/cell/dispatch/gm_gate.rs::{requires_gm,enforce_gm_gate}` reads the server's `CellEntity::access_level` and refuses every `SGWGmPlayer` tail method (109+) below `GameMaster`. `crates/cell-console/src/cell/console/gm/mod.rs::dispatch` routes only named implemented methods; its `false` result falls through to the authorized unimplemented arm. These are candidate-method facts, not proven event-to-method bindings.

Readbacks below use `server_entity_get {entity_id}` (`crates/lab-mcp/src/tools/entities.rs`; `crates/wire/src/cell/messages/lab.rs::LabEntitySnapshot`) for live position, target and stats, or read-only `server_db_query` (`crates/lab-mcp/src/tools/db.rs`) for the named persisted character fields. A submitted line or cell-to-base enqueue is not acceptance, and feedback is not a substitute for state readback. The live entity snapshot does not expose crafting balances, every combat flag, ability-set ID or organization-window application. No command was invoked in a live client or lab. No inverse is offered without exact prior state, target and downstream outcome. Source for the method candidates and NEW/ADAPT status is `docs/protocol/cell-method-dispatch-table.md` rows 138–164; the corresponding user guide is `docs/commands.md` GM table. Where those summaries differ from code, the handler below controls the contract.

The refused-grant spot check is `gm/tests/give.rs::gm_give_expertise_rejects_nonpositive_fields`: zero amount, zero discipline and truncated payload each produce feedback and **no** `GmGrantExpertise` plugin message. `gm/give.rs::handle_give_expertise` enforces those branches before enqueue; `base-crafting/src/base/crafting/handlers.rs::grant_expertise_in_db` commits the actual grant later. The reload spot check is `gm/tests/organizations.rs`: a GM call forwards `OrgCellToBase::GmReload`, a caller without a character ID does not, and the non-GM gate blocks it. `base-session/src/base/organization/handlers/gm_inspect.rs::gm_reload` reads memberships and calls `push_org_state` for each; partial push failure is reported. The test proves routing and refusal, not client application.

### /gmgiverespawner

- **Sources:** TOML `Event_SlashCmd_GiveRespawner`; dispatch table `gmGiveRespawner` 138 (`NEW`); no arm in `gm/mod.rs::dispatch`.
- **Arguments:** Required `Integer RespawnerMobId`; bounds not declared; whitespace-only numeric token, no quoted text.
- **Target/scope:** Presumed caller's respawner, but no implemented target/persistence path proves this.
- **Preconditions and authority:** Client `p` filter and server GM tail gate; respawner ID validity and ownership are unknown.
- **Effects and impact:** No implemented method effect. A grant or respawn-point write must not be inferred from the word.
- **Verification:** No implemented readback or DB field mapping for this candidate; unknown.
- **Compensation:** None proven.
- **Availability:** Disabled: server handler and native event binding unproved.

### /gmgiveexpertise

- **Sources:** TOML `Event_SlashCmd_GiveExpertise`; dispatch table 139; `gm/give.rs::handle_give_expertise`; `crates/base-crafting/src/base/crafting/handlers.rs::{handle_grant_expertise,grant_expertise_in_db}`; `gm/tests/give.rs::gm_give_expertise_rejects_nonpositive_fields`.
- **Arguments:** Required `Integer disciplineId`, `Integer expertise`, in that order; cell decodes signed `i32`; both must be positive, no upper input bound; no quoting.
- **Target/scope:** Calling GM's `player_id`, one crafting discipline; no selected/other-player target.
- **Preconditions and authority:** Shared GM gate; caller must have player ID. Cell refuses truncated or nonpositive values before forwarding; base needs a DB row and pool.
- **Effects and impact:** Persistent, additive expertise capped at 100; transaction registers a new discipline, saves and commits. Cell forwarding alone is not a grant.
- **Verification:** Wait for base success line `discipline <id> now <value>` and `onUpdateDiscipline`; `server_db_query` `sgw_player.discipline_ids` and `sgw_player_discipline_expertise.expertise` for the caller and discipline before/after. A missing pool drops the grant.
- **Compensation:** No proven inverse; a manual correction needs the exact old expertise and discipline membership, since clamping loses the original delta.
- **Availability:** Disabled until native event-to-method binding and submission/receipt proof; server candidate implemented.

### /gmgiveappliedsciencepoints

- **Sources:** TOML `Event_SlashCmd_GiveAppliedSciencePoints`; dispatch table 140; `gm/give.rs::handle_give_applied_science`; `base-crafting/src/base/crafting/handlers.rs::handle_grant_applied_science`; `gm/tests/give.rs::gm_give_applied_science_emits_grant_and_rejects_nonpositive`.
- **Arguments:** Required `Integer points`; signed `i32` at cell, strictly positive, no upper input bound or quoting.
- **Target/scope:** Calling GM's character; no selected target.
- **Preconditions and authority:** Shared GM gate, player ID and DB row/pool; nonpositive/truncated args refused before enqueue.
- **Effects and impact:** Persistent additive `sgw_player.applied_science_points`, saturated at `i32::MAX` by a locked SQL update; not read-only. Base then pushes the total as a client property.
- **Verification:** Wait for base `+<amount> (total <value>)` or refusal; compare `sgw_player.applied_science_points` through `server_db_query`. Cell enqueue is not success.
- **Compensation:** No proven inverse: points may be spent concurrently and saturation loses the requested delta; manual reconciliation only.
- **Availability:** Disabled until native binding and receipt proof; server candidate implemented.

### /gmgiveracialparadigmlevels

- **Sources:** TOML `Event_SlashCmd_GiveRacialParadigmLevels`; dispatch table 141 (`ADAPT` column only); no arm in `gm/mod.rs::dispatch`.
- **Arguments:** Required `Integer racialParadigmId`, `Integer levels`; no TOML bounds; no quoting.
- **Target/scope:** Candidate character racial-paradigm array, but no handler proves caller versus selection.
- **Preconditions and authority:** Shared GM gate; valid paradigm ID, level range and storage policy unknown.
- **Effects and impact:** No implemented grant; table's array-column hint is not a write path.
- **Verification:** No supported before/after recipe until a handler identifies the player and exact array element.
- **Compensation:** None proven.
- **Availability:** Disabled: server effect and native binding unproved.

### /gmsetgodmode

- **Sources:** TOML `Event_SlashCmd_SetGodMode`; dispatch table 142; `gm/god_mode.rs::handle_set_god_mode`; `crates/cell-combat/src/cell/combat/god_mode.rs`; `gm/tests/abilities.rs::gm_set_god_mode_toggles_the_callers_flag_with_feedback`.
- **Arguments:** Required `Boolean on/off`; client boolean spelling/serialization still needs DW-24B proof. Cell wire is `UINT8`: zero off, any nonzero on; no quoting.
- **Target/scope:** Calling GM only.
- **Preconditions and authority:** Shared GM gate; caller entity must exist; missing byte refused.
- **Effects and impact:** Transient `CellEntity::god_mode`; absorbs Health/Focus loss at hit and pulse seams, leaves heals/buffs/debuffs. Relog starts off; no DB save.
- **Verification:** Feedback says on/off; a controlled damage observation may confirm protection, but `server_entity_get` does not expose `god_mode`. Do not equate feedback with measured combat behavior.
- **Compensation:** Explicit same method with `false` can switch it off while the same entity/session exists; relog also clears it. Neither undoes missed combat damage.
- **Availability:** Disabled until native binding and Boolean serialization proof; server candidate implemented.

### /gmsetnoxp

- **Sources:** TOML `Event_SlashCmd_SetNoXP`; dispatch table 143 (`NEW`); no arm in `gm/mod.rs::dispatch`.
- **Arguments:** No mandatory or optional TOML parameters; `docs/commands.md` notes a typed argument is ignored.
- **Target/scope:** Unknown; no XP-immunity state or target handler exists.
- **Preconditions and authority:** Shared GM gate only; toggle semantics unknown.
- **Effects and impact:** No implemented XP-suppression effect; no read-only classification from zero arguments.
- **Verification:** No supported readback for a nonexistent toggle.
- **Compensation:** None proven.
- **Availability:** Disabled: server effect and native binding unproved.

### /gmsetnodamage

- **Sources:** TOML `Event_SlashCmd_SetNoDamageTimedMode`; dispatch table 144 (`NEW`); no arm in `gm/mod.rs::dispatch`.
- **Arguments:** No TOML parameters, despite guide prose suggesting on/off; do not add a Boolean form from that prose.
- **Target/scope:** Unknown; no timed-damage immunity handler.
- **Preconditions and authority:** Shared GM gate only; duration and toggle semantics unknown.
- **Effects and impact:** No implemented effect, not a confirmed read or reversible toggle.
- **Verification:** No supported timed-mode state readback.
- **Compensation:** None proven.
- **Availability:** Disabled: server effect, argument semantics and native binding unproved.

### /gmsetnoaggro

- **Sources:** TOML `Event_SlashCmd_SetNoAggro`; dispatch table 145 (`NEW`); no arm in `gm/mod.rs::dispatch`.
- **Arguments:** Required `Boolean on/off`; no TOML bound; Boolean native serialization unproved.
- **Target/scope:** Unknown; NPC threat seeding has no gate for this command.
- **Preconditions and authority:** Shared GM gate only; target and lifetime unknown.
- **Effects and impact:** No implemented aggro suppression; cannot promise peaceful NPCs.
- **Verification:** No relevant flag or threat readback is wired to this candidate.
- **Compensation:** None proven.
- **Availability:** Disabled: server effect and native binding unproved.

### /gmsetspeed

- **Sources:** TOML `Event_SlashCmd_SetSpeed`; dispatch table 146 (`ADAPT`, stat primitive only); no arm in `gm/mod.rs::dispatch`.
- **Arguments:** Required `Float Multiplier`; no TOML finite/range constraint, no quoting.
- **Target/scope:** Caller is suggested by the name but not established by a handler.
- **Preconditions and authority:** Shared GM gate; safe multiplier range and target policy unknown.
- **Effects and impact:** No implemented command; `Stat::set_current` is a possible primitive, not a mapped effect.
- **Verification:** A future handler could compare movement-speed stat in `server_entity_get`; no current command receipt exists.
- **Compensation:** None proven.
- **Availability:** Disabled: server effect and native binding unproved.

### /gmsethealth

- **Sources:** TOML `Event_SlashCmd_SetHealth`; dispatch table 147; `gm/stats.rs::{handle_set_health,set_stat}`; `gm/tests/stats.rs`.
- **Arguments:** Required `Integer Amount`, `Integer TargetId`; TOML uses `Integer` for both, while cell wire is `INT32` then `INT64`; amount must be `>= 0`, target `0` means self. No quoting.
- **Target/scope:** Caller or existing entity in caller's space selected by explicit ID, not UI selection.
- **Preconditions and authority:** Shared GM gate; valid same-space target with Health stat; negative amount, missing target/stat or truncated wire refused.
- **Effects and impact:** Transient cell Health current value via `Stat::set_current`, clamped by stat bounds; `onStatUpdate` sent to owner/witnesses. No DB save in this handler.
- **Verification:** Compare `server_entity_get` target `health_cur` before/after; feedback's requested amount need not equal clamped value.
- **Compensation:** Manual re-set from captured prior value, subject to intervening combat; no safe automatic inverse.
- **Availability:** Disabled until native binding and `Integer TargetId` to `INT64` encoding proof; server candidate implemented.

### /gmsethealthmax

- **Sources:** TOML `Event_SlashCmd_SetHealthMax`; dispatch table 148; `gm/stats.rs::{handle_set_health,set_stat}`; `gm/tests/stats.rs`.
- **Arguments:** Required `Integer Amount`, `Integer TargetId`; cell wire `INT32` then `INT64`; amount `>= 0`, target `0` self; no quoting.
- **Target/scope:** Explicit same-space entity ID or caller.
- **Preconditions and authority:** Shared GM gate; target must exist and have Health stat; negative/truncated request refused.
- **Effects and impact:** Transient cell maximum Health via `Stat::set_max`; may clamp current Health, broadcasts dirty stat; no DB save in handler.
- **Verification:** Compare both `health_max` and `health_cur` in `server_entity_get` for target; feedback reports requested amount only.
- **Compensation:** Manual restoration of both prior max/current after checking intervening damage; no automatic inverse.
- **Availability:** Disabled until native binding and `INT64 TargetId` encoding proof; server candidate implemented.

### /gmsetfocus

- **Sources:** TOML `Event_SlashCmd_SetFocus`; dispatch table 149; `gm/stats.rs::{handle_set_focus,set_stat}`; `gm/tests/stats.rs`.
- **Arguments:** Required `Integer Amount`, `Integer TargetId`; cell wire `INT32` then `INT64`; amount `>= 0`, target `0` self; no quoting.
- **Target/scope:** Explicit same-space entity ID or caller.
- **Preconditions and authority:** Shared GM gate; target exists with Focus stat; negative/truncated request refused.
- **Effects and impact:** Transient current Focus via `Stat::set_current`, clamped and sent through `onStatUpdate`; no DB save in handler.
- **Verification:** Compare `server_entity_get` target `focus_cur` before/after; feedback is not a clamped-value read.
- **Compensation:** Manual prior-value restoration subject to intervening spend/regen; no automatic inverse.
- **Availability:** Disabled until native binding and `INT64 TargetId` encoding proof; server candidate implemented.

### /gmsetfocusmax

- **Sources:** TOML `Event_SlashCmd_SetFocusMax`; dispatch table 150; `gm/stats.rs::{handle_set_focus,set_stat}`; `gm/tests/stats.rs`.
- **Arguments:** Required `Integer Amount`, `Integer TargetId`; cell wire `INT32` then `INT64`; amount `>= 0`, target `0` self; no quoting.
- **Target/scope:** Explicit same-space entity ID or caller.
- **Preconditions and authority:** Shared GM gate; target exists with Focus stat; negative/truncated request refused.
- **Effects and impact:** Transient maximum Focus via `Stat::set_max`, potentially clamping current Focus; `onStatUpdate` broadcast; no DB save in handler.
- **Verification:** Compare `focus_max` and `focus_cur` in `server_entity_get` target snapshot.
- **Compensation:** Manual restoration of captured max/current, with intervening effects accounted for; no automatic inverse.
- **Availability:** Disabled until native binding and `INT64 TargetId` encoding proof; server candidate implemented.

### /gmsetflag

- **Sources:** TOML `Event_SlashCmd_SetFlag`; dispatch table 151 (`ADAPT`, `state_flags.rs::set_state_flag` only); no arm in `gm/mod.rs::dispatch`.
- **Arguments:** Required `Integer aFlagId`, `Integer aForceVal`; no TOML enum/range and no quoting; wire would require flag `INT32`, force `UINT8` if method binding is proven.
- **Target/scope:** No handler establishes caller, selection or other target.
- **Preconditions and authority:** Shared GM gate; valid flag IDs, ref-count policy and force values unknown.
- **Effects and impact:** No implemented force-set. A primitive exists but counted flags make a raw write potentially destructive.
- **Verification:** No command receipt; `server_entity_get.state_field` is only a partial bitfield, not refcount proof.
- **Compensation:** None proven; restoring one bit would not reconstruct refcounts.
- **Availability:** Disabled: no server handler, native binding or safe flag contract.

### /gmsetlevel

- **Sources:** TOML `Event_SlashCmd_SetLevel`; dispatch table 152 (`ADAPT`, several stat primitives only); no arm in `gm/mod.rs::dispatch`.
- **Arguments:** Required `Integer Level`; no TOML bounds, no quoting.
- **Target/scope:** Candidate caller character; no implemented target path proves it.
- **Preconditions and authority:** Shared GM gate; allowed level range, stat rescale and persistence transaction absent.
- **Effects and impact:** No implemented level mutation; a `scale_for_level` primitive is not a full level grant.
- **Verification:** No supported result/readback for this word; a future handler must compare character level and derived stats.
- **Compensation:** None proven; leveling can alter multiple derived values.
- **Availability:** Disabled: server effect and native binding unproved.

### /gmresetabilities

- **Sources:** TOML `Event_SlashCmd_ResetAbilities`; dispatch table 153; `gm/abilities.rs::handle_reset_abilities`; `base-methods/src/base/world_entry/methods/progression/gm_ability_bulk.rs::persist_bulk`; its `gm_ability_bulk_tests.rs`.
- **Arguments:** No TOML parameters; no quoting.
- **Target/scope:** Calling GM's persisted character, not selected player.
- **Preconditions and authority:** Shared GM gate, player ID, DB row and resolvable start profile. Cell forwards; base may refuse missing row/profile.
- **Effects and impact:** Persistent reset: removes GM grants and old uncredited abilities, keeps start-profile abilities and credited non-GM grants, refunds spent tree points and clears trained abilities in one transaction; not a no-op or ordinary trainer respec.
- **Verification:** Await `GmAbilitiesChanged`/definitive base feedback, compare `server_db_query` `sgw_player.abilities`, `training_points`, `trained_abilities`, `tree_points_spent` and grant-provenance rows; client ability window burst is presentation.
- **Compensation:** No proven inverse; exact removed abilities, provenance and spent-tree state must be captured for manual reconstruction.
- **Availability:** Disabled until native binding and receipt proof; server candidate implemented.

### /gmgiveallabilities

- **Sources:** TOML `Event_SlashCmd_GiveAllAbilities`; dispatch table 154; `gm/abilities.rs::{handle_give_all_abilities,plan_give_all}`; `progression/gm_ability_bulk.rs::persist_bulk`; `gm_ability_bulk_tests.rs`.
- **Arguments:** No TOML parameters; no quoting.
- **Target/scope:** Calling GM, all missing abilities in their archetype tree.
- **Preconditions and authority:** Shared GM gate, player ID/archetype/tree; cell refuses missing data or nothing to grant; base requires persisted row.
- **Effects and impact:** Persistent one-transaction ability append plus GM provenance; no training points spent. Later `/gmresetabilities` removes GM grants, including any preexisting ones, so it is not a precise inverse.
- **Verification:** Compare `sgw_player.abilities` and GM grant-provenance rows by `player_id`; `GmAbilitiesChanged` and window refresh are additional presentation evidence.
- **Compensation:** Manual restore from prior abilities/provenance; no safe automatic inverse.
- **Availability:** Disabled until native binding and receipt proof; server candidate implemented.

### /gmrespec

- **Sources:** TOML `Event_SlashCmd_Respec`; dispatch table 155 (`NEW`); no arm in `gm/mod.rs::dispatch`. Player `resetMyAbilities` is a different method with trainer conditions (`gm_gate.rs`).
- **Arguments:** No TOML parameters; no quoting.
- **Target/scope:** Unknown; no GM respec handler.
- **Preconditions and authority:** Shared GM gate; no proven target or cost policy.
- **Effects and impact:** No implemented GM reset; do not substitute `/gmresetabilities` or player trainer respec.
- **Verification:** No command-specific readback until a handler is defined.
- **Compensation:** None proven.
- **Availability:** Disabled: server effect and native binding unproved.

### /gmsettarget

- **Sources:** TOML `Event_SlashCmd_SetTarget`; dispatch table 156; `gm/world.rs::handle_set_target`; `gm/tests/world.rs`.
- **Arguments:** Required `String NameOrID`; no quoting rule proven; server parses nonnegative numeric text only, `0` clears; names are refused despite field name.
- **Target/scope:** Calling GM's `current_target_id`; a positive ID is stored even if that entity's existence is not checked by this handler.
- **Preconditions and authority:** Shared GM gate and caller entity; malformed WSTRING or nonnumeric/negative text refused.
- **Effects and impact:** Transient selection change; sends `onTargetUpdate` to owner and AoI witnesses. No DB write.
- **Verification:** Compare caller `server_entity_get.current_target_id`; client target highlight is separate. Stored ID alone does not prove the entity still exists.
- **Compensation:** Set the captured prior ID or `0` in same session, provided that target still makes sense; no other state is undone.
- **Availability:** Disabled until native string quoting/binding proof; server candidate implemented for numeric ID only.

### /gmsetmobstance

- **Sources:** TOML `Event_SlashCmd_SetMobStance`; dispatch table 157 (`NEW`); no arm in `gm/mod.rs::dispatch`.
- **Arguments:** Required `Integer StanceID`; no TOML bounds, no quoting.
- **Target/scope:** Presumed mob but selection semantics unknown; no distinct stance field is wired.
- **Preconditions and authority:** Shared GM gate; target class and stance enum unknown.
- **Effects and impact:** No implemented stance mutation. `AiState` is not proven equivalent.
- **Verification:** No supported stance readback.
- **Compensation:** None proven.
- **Availability:** Disabled: handler/effect and native binding unproved.

### /gmsetmobabilityset

- **Sources:** TOML `Event_SlashCmd_SetMobAbilitySet`; dispatch table 158; `gm/mob_ability_set.rs::{handle_set_mob_ability_set,apply}`; `gm/tests/abilities.rs::gm_set_mob_ability_set_refusals_leave_the_mob_alone`.
- **Arguments:** Optional `Integer abilitySetId`; no mandatory params or declared default. The server's candidate method requires an `INT32` byte argument and refuses missing bytes; optional-TOML serialization is unresolved. No quoting.
- **Target/scope:** Caller-selected mob in same space, not a player; set loaded from `resources.ability_set_abilities`.
- **Preconditions and authority:** Shared GM gate; selected live non-player entity and loaded known set. Refuses no target, other space, player, no set data or unknown set.
- **Effects and impact:** Transient replacement of mob known abilities, interrupts a warmup cast whose ability was removed; next spawn restores template set. No DB or seed edit.
- **Verification:** Feedback lists chosen set and abilities, but `server_entity_get` exposes neither current known abilities nor ability-set ID. The existing read-only tools cannot independently prove this exact swap; mark readback unverified until an exact live ability-set/known-ability read exists.
- **Compensation:** Manual restore of captured prior ability list if a supported path exists; no exact inverse here. Respawn uses template, which need not equal prior debug state.
- **Availability:** Disabled until optional argument behavior and native binding are proven; server candidate implemented only with explicit ID.

### /gmdhd

- **Sources:** TOML `Event_SlashCmd_DHD`; dispatch table 159; `gm/travel/dhd.rs::handle_dhd`; `gm/tests/travel.rs`; `crate::cell::gate_travel::handle_dial_gate`.
- **Arguments:** Required `Integer DestinationId`; no TOML bound; cell method decodes signed `INT8`, only positive values accepted (0 list request unsupported). No quoting.
- **Target/scope:** Calling GM's current world and target gate address; may grant address to caller's **live** address book for this session, except outbound-only hub.
- **Preconditions and authority:** Shared GM gate; known dialable address, different world and standable arrival; hub and failed dial refused.
- **Effects and impact:** Cross-world `GateTravel` if accepted, with possible session-only address-book grant; disruptive movement, not a read or harmless query.
- **Verification:** Feedback distinguishes dialing/refusal, then compare caller world/space and position after destination world entry via `server_entity_get`/`server_sessions`; feedback alone precedes settled travel.
- **Compensation:** Manual return travel if available; no automatic restoration of prior world, combat/duel state or address-book side effect.
- **Availability:** Disabled until native binding and `Integer` to signed `INT8` range/encoding proof; server candidate implemented.

### /gmgoto

- **Sources:** TOML `Event_SlashCmd_Goto`; dispatch table 160; `gm/travel/goto_summon.rs::handle_goto`; `gm/tests/travel.rs`.
- **Arguments:** Required `String TargetName`; no quoting rule proven. Server accepts positive numeric entity ID only; names are not resolved.
- **Target/scope:** Caller moves to a live target in the same space; target itself stays put.
- **Preconditions and authority:** Shared GM gate; positive numeric ID, same-space target and caller must exist; cross-space or absent target refused.
- **Effects and impact:** Transient authoritative position snap via spatial-grid update and `TeleportPlayer`; travel hook may end a duel and pet follows. No seed write.
- **Verification:** Compare caller `space_id` and `position` against target with `server_entity_get` before/after; feedback does not prove client snap arrived.
- **Compensation:** Manual return to captured position, if still safe; duel/other travel side effects have no inverse.
- **Availability:** Disabled until native string quoting/binding proof; numeric-ID server candidate implemented.

### /gmsummon

- **Sources:** TOML `Event_SlashCmd_Summon`; dispatch table 161; `gm/travel/goto_summon.rs::handle_summon`; `gm/tests/travel.rs`.
- **Arguments:** Required `String TargetName`; no quoting rule proven. Server accepts positive numeric entity ID only, not names.
- **Target/scope:** Explicit other entity in caller's same space; player target receives `TeleportPlayer`, NPC target moves in cell spatial grid.
- **Preconditions and authority:** Shared GM gate; cannot target self; absent or other-space target refused.
- **Effects and impact:** Transient move of another entity to caller; travel hook may end target duel, and target's pet can follow. A player client needs authoritative snap; no DB/seed write here.
- **Verification:** Compare selected entity's `space_id` and `position` with caller through `server_entity_get`; for a player verify client snap separately.
- **Compensation:** Manual return to captured position; no inverse for duel and other intervening state.
- **Availability:** Disabled until native string quoting/binding proof; numeric-ID server candidate implemented.

### /gmgotolocation

- **Sources:** TOML `Event_SlashCmd_GotoLocation`; dispatch table 162; `gm/travel/goto_location.rs::handle_goto_location`; `gm/tests/travel.rs`.
- **Arguments:** Required `String WorldName`, `Float X-Coordinate`, `Float Y-Coordinate`, `Float Z-Coordinate` in order. No quoting rule proven for spaces in world names; coordinates must be finite. `0,0,0` requests authored world entry point rather than literal origin.
- **Target/scope:** Calling GM to named destination world; even same-world name uses full reload, not an in-place snap.
- **Preconditions and authority:** Shared GM gate; caller exists, nonempty world, finite coordinates; absent entry point for zero sentinel refuses. Base handles unknown world after enqueue.
- **Effects and impact:** Disruptive `GateTravel` and local teardown after queue confirmation; world entry and duel/pet effects possible. No direct seed mutation.
- **Verification:** Compare world/space/position only after re-entry through `server_sessions` plus `server_entity_get`; the immediate `teleporting` feedback proves request path, not settled arrival.
- **Compensation:** Manual return to captured world and position; no safe automatic rollback of transfer or surrounding state.
- **Availability:** Disabled until native string quoting/binding proof and accepted travel receipt; server candidate implemented.

### /gmgotoxyz

- **Sources:** TOML `Event_SlashCmd_GotoXYZ`; dispatch table 163; `gm/travel/goto_xyz.rs::handle_goto_xyz`; `gm/tests/travel.rs`.
- **Arguments:** Required `Float X-Coordinate`, `Float Y-Coordinate`, `Float Z-Coordinate`; finite values only; no quoting.
- **Target/scope:** Calling GM within current space; facing preserved.
- **Preconditions and authority:** Shared GM gate, caller entity and finite coordinates; missing bytes/nonfinite refused.
- **Effects and impact:** Transient authoritative same-space move: spatial grid update plus `TeleportPlayer`/forced-position snap; travel hook may affect duel and pet. No seed write.
- **Verification:** Compare caller `space_id` and `position` with `server_entity_get` before/after; client snap is a separate observation.
- **Compensation:** Manual return to captured coordinates; no undo for intervening duel/combat effects.
- **Availability:** Disabled until native binding/submission proof; server candidate implemented.

### /gmreloadorganizations

- **Sources:** TOML `Event_SlashCmd_ReloadOrganizations`; dispatch table 164 (DONE, despite older `docs/commands.md` row saying not yet); `gm/organizations.rs::handle_reload_organizations`; `crates/base-session/src/base/organization/handlers/gm_inspect.rs::gm_reload`; `gm/tests/organizations.rs::{gm_reload_organizations_forwards_to_the_base,gm_reload_organizations_without_a_character_stays_on_the_cell}`.
- **Arguments:** No TOML mandatory or optional params; no quoting.
- **Target/scope:** Caller character's own Team and Command memberships; other members are not pushed to.
- **Preconditions and authority:** Shared cell GM gate, caller player ID; base re-authorizes GameMaster and requires DB pool. Cell refuses no character/closed base channel; base can refuse DB/auth failures.
- **Effects and impact:** **Transient client-state re-push**, not read-only and not a schema/seed reload. Base reloads memberships and calls `push_org_state` for each; a subset can fail. It does not change organization DB rows.
- **Verification:** Feedback states `re-sent <pushed> of <total>` and `org.gm_action` logs count; compare caller client organization view or captured outgoing state pushes with `server_db_query` membership rows. A feedback success does not prove the client applied every push.
- **Compensation:** No automatic inverse; restore the client's prior view by another authoritative push/relog if needed. Since no DB mutation occurs, no DB rollback is applicable.
- **Availability:** Disabled until native binding and outgoing/received method proof; server candidate implemented. Do not present as safely reversible or read-only.
