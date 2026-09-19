# Legacy Command Static Audit

> Type: reference. Audience: implementing engineers and reviewers.
> Updated: 2026-09-16. Companions: [coordinator handoff](README.md), [work packets](work-packets.md).
> Source revision: `6279fcfb53a7325ec9f00ff705e027b6e0d39ce0`. Evidence: static emulator source inspection only. Runtime tests and in-client UAT: not run.

## Reading The Matrix

The denominator is the exact 116 registrations in [ConsoleCommands.py](../../../deprecated/python/cell/ConsoleCommands.py), including imported bodies. The second cell is exact-name presence in the active [Rust registry](../../../crates/services/src/cell/console/registry.rs), not native method availability. Routing is [chat](../../../crates/services/src/cell/chat.rs) -> [dispatch](../../../crates/services/src/cell/console/dispatch.rs). The generic commands crate is not this roster.

`source-implemented` means the relevant operation exists in inspected source; it does not mean runtime parity was tested. `partial` means known behavior gaps or untraced downstream effects; `no-op` means feedback without the requested operation; `absent` means no matching dot registration. Work kind: **W** thin typed wrapper over working operations, **E** extend/correct existing behavior, **N** new operation/state/orchestration, **V** verify existing source behavior. These labels can combine; N does not imply the entire surrounding subsystem is missing.

Signatures omit the dot name, caller and target. Brackets denote optional arguments; `()` means none. Target codes reproduce registration: **P** selected SGWPlayer, **B** SGWBeing, **S** SGWSpawnableEntity, **M** SGWMob, **O** no required target (a resolvable selection may still be passed). Native references below are method names/indices; their typed slash bindings were not independently checked. All rows have a packet, including existing implementations.

| Family | Legacy | Dot present | Dot absent |
|---|---:|---:|---:|
| Console | 3 | 3 | 0 |
| Player | 24 | 6 | 18 |
| Entity | 37 | 22 | 15 |
| Mission | 14 | 2 | 12 |
| Crafting | 5 | 3 | 2 |
| Resource | 11 | 9 | 2 |
| Net | 11 | 8 | 3 |
| Misc | 11 | 4 | 7 |
| Total | 116 | 57 | 59 |

Rust has 71 dots, of which 14 are outside this baseline: `seedconfirm`, `seedpending`, `seedcancel`, `path_add`, `path_show`, `path_clear`, `path_assign`, `path_unassign`, `path_set_seq`, `path_clear_seq`, `path_set_tp`, `path_clear_tp`, `path_set_tp_seq`, `path_set_tp_delay`. Preserve them. The overlap is about 49% registration coverage, not 49% full implementation.

## Console

Legacy: [ConsoleCommands.py](../../../deprecated/python/cell/ConsoleCommands.py). Active: [query.rs](../../../crates/services/src/cell/console/query.rs), [server.rs](../../../crates/services/src/cell/console/server.rs). Modern GM-only access is intentional, including help; do not lower the threshold to legacy access levels 0/1.

| Command | Registered | Arguments; target | Status | Reuse and gap | Source | Packet |
|---|---|---|---|---|---|---|
| `.help` | Yes | `[command]`; O | partial | E: sorted/filter summary exists; restore per-argument help without weakening GM gate. | [query](../../../crates/services/src/cell/console/query.rs) | [P01](work-packets.md#p01) |
| `.loglevel` | Yes | `level [category]`; O | no-op | N/E: needs runtime tracing reload and legacy-category mapping; not inherently impossible in Rust. | [server](../../../crates/services/src/cell/console/server.rs) | [G10](work-packets.md#g10) |
| `.logclient` | Yes | `()`; O | no-op | N: bounded per-GM log subscription, forwarding and cleanup need design; OTLP is not the requested toggle. | [server](../../../crates/services/src/cell/console/server.rs) | [G10](work-packets.md#g10) |

## Player

Legacy: [Player.py](../../../deprecated/python/cell/commands/Player.py). Reuse: [native give](../../../crates/services/src/cell/cell_methods/gm/give.rs), [native travel](../../../crates/services/src/cell/cell_methods/gm/travel.rs), [base progression](../../../crates/services/src/base/world_entry/methods/progression/mod.rs). Native grant wrappers are caller-oriented, not selected-player adapters.

| Command | Registered | Arguments; target | Status | Reuse and gap | Source | Packet |
|---|---|---|---|---|---|---|
| `.kill` | No | `()`; B | absent | W/E: native 190 kills NPCs only; selected-player death needs canonical lifecycle. | [death](../../../crates/services/src/cell/abilities/death.rs) | [P27](work-packets.md#p27) |
| `.revive` | No | `()`; B | absent | N/E: respawn moves/reloads; extract safe in-place revival, not a health-only edit. | [respawn](../../../crates/services/src/cell/cell_methods/player/combat/respawn.rs) | [P28](work-packets.md#p28) |
| `.clearabilities` | No | `()`; P | absent | N/E: enumeration/removal exists; persist revocation and reconcile weapon-granted tracking and client list. | [abilities](../../../crates/entity/src/abilities/manager.rs) | [P29](work-packets.md#p29) |
| `.giveaddress` | No | `stargateId [hidden=False]`; P | absent | N: validated grant, mutually exclusive known/hidden lists, persistence and live update; hidden login list currently empty. | [map-loaded](../../../crates/services/src/mercury/world_data/map_loaded.rs) | [G07](work-packets.md#g07) |
| `.giveability` | No | `abilityId`; P | absent | N/W: free grant, not TP-debiting training; reuse post-commit AbilityGranted mirror. | [ability-granted](../../../crates/services/src/cell/service/base_messages/ability_granted.rs) | [P29](work-packets.md#p29) |
| `.givecash` | No | `amount`; P | absent | W: GrantCash persists authoritative total; separate caller feedback from selected recipient, retain amount bounds. | [progression](../../../crates/services/src/base/world_entry/methods/progression/mod.rs) | [P05](work-packets.md#p05) |
| `.giveitem` | No | `designId quantity`; P | absent | W: GrantItem transaction/outbox exists; route selected player and correct container, propagate inventory-full/DB errors. | [grant-item](../../../crates/services/src/base/world_entry/methods/inventory/grant/grant_item.rs) | [P06](work-packets.md#p06) |
| `.giverespawner` | No | `respawnerId`; P | absent | N: schema has known_respawners; defeat choices currently use global world-filtered catalog, so append alone is insufficient. | [defeat choices](../../../crates/services/src/cell/abilities/damage_apply/mod.rs) | [G08](work-packets.md#g08) |
| `.givetp` | No | `amount`; P | absent | N/E: direct TP receiver must synchronize DB, connection cache and client; XP level-up is not an equivalent grant. | [progression](../../../crates/services/src/base/world_entry/methods/progression/mod.rs) | [P30](work-packets.md#p30) |
| `.givexp` | No | `amount`; P | absent | W: GrantXP has level/TP persistence and UI; route selected player and reject unsafe signed-to-u64 conversion. | [progression](../../../crates/services/src/base/world_entry/methods/progression/mod.rs) | [P05](work-packets.md#p05) |
| `.god` | No | `[enabled=True]`; P | absent | N: no invulnerability guard across all damage paths; main pipeline and direct scripts both matter. | [damage pipeline](../../../crates/services/src/cell/combat/damage/pipeline.rs), [scripts](../../../crates/services/src/cell/effects/scripts.rs) | [G06](work-packets.md#g06) |
| `.listabilities` | No | `()`; P | absent | W: enumerate IDs, resolve definition names, retain unknown-ID fallback; feedback to caller. | [ability manager](../../../crates/entity/src/abilities/manager.rs) | [P04](work-packets.md#p04) |
| `.dynamicupdate` | Yes | `()`; S | partial | E: RefreshAppearance for players only; no NPC generic dynamic refresh. Trace final receiver before crediting appearance parity. | [entity](../../../crates/services/src/cell/console/entity.rs) | [P15](work-packets.md#p15) |
| `.adddialog` | Yes | `templateId setMapId`; P | partial | E: map validation/mutation exists; Rust permits Spawnable instead of Player. Verify runtime interaction cache and UI use. | [entity](../../../crates/services/src/cell/console/entity.rs) | [G02](work-packets.md#g02) |
| `.removeaddress` | No | `stargateId`; P | absent | N: revoke from hidden/known state, persist and notify live client; login serialization is not a revoke operation. | [legacy player](../../../deprecated/python/cell/SGWPlayer.py) | [G07](work-packets.md#g07) |
| `.removedialog` | Yes | `templateId setMapId`; P | partial | E: removes mapping in memory; same target-type and runtime/cache obligations as add. | [entity](../../../crates/services/src/cell/console/entity.rs) | [G02](work-packets.md#g02) |
| `.removeitem` | No | `designId quantity`; P | absent | E: remove-by-type only drains first stack; D07 requires exact aggregate removal or transactional no change. Native gmRemoveItem uses instance ID. | [remove-by-type](../../../crates/services/src/base/world_entry/methods/inventory/core/remove_by_type.rs) | [P07](work-packets.md#p07) |
| `.removerespawner` | Yes | `respawnerId`; P | no-op | N: no unlock receiver; share grant/revoke/hydration/filter design, not success-only feedback. | [server](../../../crates/services/src/cell/console/server.rs) | [G08](work-packets.md#g08) |
| `.reloadmap` | Yes | `()`; P | no-op | E/N: selected-player map reload; current registry requires no target. State preservation and failure behavior need approval. | [server](../../../crates/services/src/cell/console/server.rs) | [G09](work-packets.md#g09) |
| `.save` | Yes | `()`; O, caller | no-op | E/N: define durable flush/ack; incremental persistence does not establish that every mutation is saved. | [server](../../../crates/services/src/cell/console/server.rs) | [G09](work-packets.md#g09) |
| `.goto` | No | `name`; O, selected-or-caller mover | absent | N/E: native 160 is numeric/same-space/caller. Need name lookup and destination player's actual loaded instance. | [travel](../../../crates/services/src/cell/cell_methods/gm/travel.rs) | [G05](work-packets.md#g05) |
| `.summon` | No | `name`; O, selected-or-caller destination | absent | N/E: native 161 is numeric/same-space. Named player moves to destination entity's existing instance. | [travel](../../../crates/services/src/cell/cell_methods/gm/travel.rs) | [G05](work-packets.md#g05) |
| `.gotolocation` | No | `worldName x y z`; O, selected-or-caller | absent | E: native 162 GateTravel is reuse, not proof of world validation or safe teardown/instance selection. | [gate-travel](../../../crates/services/src/base/world_entry/gate_travel/mod.rs) | [G05](work-packets.md#g05) |
| `.gotoxyz` | No | `x y z`; O, selected-or-caller | absent | W/E: native 163 and TeleportPlayer provide snap/DB plumbing; adapt subject safely, keep NPC movement cell-side. | [teleport](../../../crates/services/src/base/world_entry/teleport.rs) | [P26](work-packets.md#p26) |

## Entity

Legacy: [Entity.py](../../../deprecated/python/cell/commands/Entity.py). Active: [entity.rs](../../../crates/services/src/cell/console/entity.rs), [stats.rs](../../../crates/services/src/cell/console/stats.rs), [net.rs](../../../crates/services/src/cell/console/net.rs). Native query methods 121/122/123/131 and stat methods 147-150 are reuse candidates, not dot registrations or proof of setter semantics.

| Command | Registered | Arguments; target | Status | Reuse and gap | Source | Packet |
|---|---|---|---|---|---|---|
| `.info` | No | `[entityId]`; O, selection wins | absent | W/E: native query summaries and CellEntity fields exist; explicit ID fallback only without selection, add detailed flags/template fields. | [native query](../../../crates/services/src/cell/cell_methods/gm/query.rs) | [P02](work-packets.md#p02) |
| `.location` | No | `[x y z]`; S | absent | W/E: read AND setter; accept zero or full tuple, reject incomplete/nonfinite values, update spatial/client state. | [native travel](../../../crates/services/src/cell/cell_methods/gm/travel.rs) | [P18](work-packets.md#p18) |
| `.rotation` | No | `[x y z]`; S | absent | E: read AND setter; preserve full orientation contract, do not expose only a yaw readout. | [legacy entity](../../../deprecated/python/cell/commands/Entity.py) | [P18](work-packets.md#p18) |
| `.facing` | No | `()`; S | absent | W: geometry query returns angle radians/degrees, facing class and distance to selected entity. | [legacy entity](../../../deprecated/python/cell/commands/Entity.py) | [P02](work-packets.md#p02) |
| `.lookat` | Yes | `()`; S | partial | E: direction mutates but no authoritative snap/fan-out; synchronize selected entity orientation. | [entity](../../../crates/services/src/cell/console/entity.rs) | [P18](work-packets.md#p18) |
| `.visible` | Yes | `visible [entityId]`; O, selection wins | partial | N/E: false emits EntityInvisible without authoritative hide state; true only feedback. Restore explicit-ID fallback and re-entry. | [entity](../../../crates/services/src/cell/console/entity.rs) | [P16](work-packets.md#p16) |
| `.staticmesh` | Yes | `staticMesh`; S | partial | E: field assignment lacks immediate client refresh. Do not claim savespawn persists appearance fields. | [entity](../../../crates/services/src/cell/console/entity.rs) | [P15](work-packets.md#p15) |
| `.bodyset` | Yes | `bodySet`; S | partial | E: same field-versus-live-appearance gap; reconcile final appearance assembly. | [entity](../../../crates/services/src/cell/console/entity.rs) | [P15](work-packets.md#p15) |
| `.nameid` | Yes | `nameId`; S | partial | E: field changes, missing immediate update; savespawn persistence feedback is misleading. | [entity](../../../crates/services/src/cell/console/entity.rs) | [P13](work-packets.md#p13) |
| `.eventset` | Yes | `eventSetId`; S | partial | E: field changes; immediate client update and truthful persistence feedback needed. | [entity](../../../crates/services/src/cell/console/entity.rs) | [P14](work-packets.md#p14) |
| `.interactiontype` | Yes | `interactionType`; S | partial | E: flags change locally; publish update so the client can actually interact. | [entity](../../../crates/services/src/cell/console/entity.rs) | [P14](work-packets.md#p14) |
| `.interact` | No | `()`; S | absent | N/W: normal interaction paths exist, but Python bypasses aggression checks/listeners; approve privileged policy before adapting. | [interact](../../../crates/services/src/cell/cell_methods/player/interaction/interact.rs) | [G02](work-packets.md#g02) |
| `.initialresponse` | No | `setMapId`; S | absent | E/W: normal dialog path depends on last_interaction_target; use explicit selected target without forging caller context. | [dialog](../../../crates/services/src/cell/cell_methods/player/interaction/dialog.rs) | [G02](work-packets.md#g02) |
| `.tag` | Yes | `tag`, `none` clears; S | source-implemented | V/E: runtime tag mutation exists; check clear sentinel and saved tag roundtrip, without inventing wider persistence. | [entity](../../../crates/services/src/cell/console/entity.rs) | [P14](work-packets.md#p14) |
| `.level` | No | `level`; B | absent | E: CellEntity already has level; player progression/cache/derived state must stay consistent, not just assign u32. | [cell entity](../../../crates/entity/src/cell_entity/mod.rs) | [P31](work-packets.md#p31) |
| `.name` | Yes | `name`; B | partial | E: local name mutation lacks immediate client update; distinguish player and NPC representations. | [entity](../../../crates/services/src/cell/console/entity.rs) | [P13](work-packets.md#p13) |
| `.alignment` | Yes | `alignment` named string; B | partial | E: field assignment exists; restore legacy named mapping and live propagation, not numeric-only parity. | [entity](../../../crates/services/src/cell/console/entity.rs) | [P13](work-packets.md#p13) |
| `.faction` | No | `faction` named string; B | absent | E: faction u8 exists; map legacy names to valid values and update dependent behavior/client state. | [legacy mappings](../../../deprecated/python/cell/commands/Entity.py) | [P13](work-packets.md#p13) |
| `.speed` | No | `speed` int, 100 normal; B | absent | W/E: set current movementSpeedMod AND rotationSpeedMod and send dirty stats; reuse checked stat/movement machinery. | [legacy entity](../../../deprecated/python/cell/commands/Entity.py) | [P35](work-packets.md#p35) |
| `.addcomponent` | Yes | `component`; B | partial | E: mutation plus player RefreshAppearance; receiver may recomposite from other state, NPC refresh incomplete. | [entity](../../../crates/services/src/cell/console/entity.rs) | [P15](work-packets.md#p15) |
| `.delcomponent` | Yes | `component`; B | partial | E: same authoritative appearance and player/NPC fan-out verification needed. | [entity](../../../crates/services/src/cell/console/entity.rs) | [P15](work-packets.md#p15) |
| `.setstate` | No | `flag` BSF suffix; B | absent | E: named state-field operation must preserve state_flag_counts/refcount invariants. | [legacy flags](../../../deprecated/python/cell/commands/Entity.py) | [P17](work-packets.md#p17) |
| `.unsetstate` | No | `flag` BSF suffix; B | absent | E: clear through correct ownership/refcount model, with client updates. | [legacy flags](../../../deprecated/python/cell/commands/Entity.py) | [P17](work-packets.md#p17) |
| `.setcombatant` | Yes | `flag` PLAYER_STATE suffix; B | partial | E: Rust takes numeric bit index and mutates state_field u32; legacy uses distinct combatant mask domain. | [entity](../../../crates/services/src/cell/console/entity.rs) | [P17](work-packets.md#p17) |
| `.unsetcombatant` | Yes | `flag` PLAYER_STATE suffix; B | partial | E: same wrong-domain/input mismatch; test against state-field collateral changes. | [entity](../../../crates/services/src/cell/console/entity.rs) | [P17](work-packets.md#p17) |
| `.health` | No | `health`; B | absent | W/E: native stat operations exist; preserve bounds, death transition and self/witness update semantics. | [native stats](../../../crates/services/src/cell/cell_methods/gm/stats.rs) | [P35](work-packets.md#p35) |
| `.focus` | No | `focus`; B | absent | W: shared checked stat mutation and client synchronization; selected target, caller feedback. | [native stats](../../../crates/services/src/cell/cell_methods/gm/stats.rs) | [P35](work-packets.md#p35) |
| `.stats` | No | `()`; B | absent | W: add basic health/focus/healthRegen/focusRegen set to existing readout machinery. | [stats](../../../crates/services/src/cell/console/stats.rs) | [P03](work-packets.md#p03) |
| `.primarystats` | Yes | `()`; B | source-implemented | V: existing primary-stat set and actual values; pin exact legacy set and recipients. | [stats](../../../crates/services/src/cell/console/stats.rs) | [P03](work-packets.md#p03) |
| `.speedstats` | Yes | `()`; B | source-implemented | V: existing speed-stat set; compare exact fields, not line count alone. | [stats](../../../crates/services/src/cell/console/stats.rs) | [P03](work-packets.md#p03) |
| `.armorstats` | Yes | `()`; B | source-implemented | V: existing armor/resistance set; exact values and missing-stat behavior. | [stats](../../../crates/services/src/cell/console/stats.rs) | [P03](work-packets.md#p03) |
| `.qrstats` | Yes | `()`; B | source-implemented | V: existing QR set; pin all 19 legacy stat names. | [stats](../../../crates/services/src/cell/console/stats.rs) | [P03](work-packets.md#p03) |
| `.absorbstats` | Yes | `()`; B | source-implemented | V: existing absorption set; pin all 15 legacy fields. | [stats](../../../crates/services/src/cell/console/stats.rs) | [P03](work-packets.md#p03) |
| `.stealthstats` | Yes | `()`; B | source-implemented | V: existing five-stat set; selected entity/caller isolation. | [stats](../../../crates/services/src/cell/console/stats.rs) | [P03](work-packets.md#p03) |
| `.aggression` | Yes | `level`; M | partial | E: aggression field mutation exists; validate enum/range and downstream AI transition. | [net](../../../crates/services/src/cell/console/net.rs) | [P34](work-packets.md#p34) |
| `.threaten` | Yes | `threat` int; M | partial | E: Rust accepts float and directly adds to threat_list; restore integer contract and threatGenerated-style transition/bookkeeping. | [net](../../../crates/services/src/cell/console/net.rs) | [P34](work-packets.md#p34) |
| `.combatinfo` | No | `()`; M | absent | W/E: restore template/weapon/ability-set diagnostics and ability-type counts from actual NPC data, not a generic threat summary. | [legacy entity](../../../deprecated/python/cell/commands/Entity.py) | [P02](work-packets.md#p02) |

## Mission

Legacy: [Mission.py](../../../deprecated/python/cell/commands/Mission.py), [MissionManager.py](../../../deprecated/python/cell/MissionManager.py). Active: [console mission](../../../crates/services/src/cell/console/mission.rs), [native missions](../../../crates/services/src/cell/cell_methods/gm/missions.rs), [content mission](../../../crates/services/src/cell/content/executor/mission.rs). Native methods 109/110/113/114/115/116/120 provide related operations, not complete selected-player orchestration.

| Command | Registered | Arguments; target | Status | Reuse and gap | Source | Packet |
|---|---|---|---|---|---|---|
| `.missionaccept` | No | `missionId`; P | absent | W/E: accept and first-step resolution exist; native assign omits persistence/accept chains, use checked shared orchestration. | [lifecycle](../../../crates/services/src/cell/missions/lifecycle.rs) | [P20](work-packets.md#p20) |
| `.missionabandon` | No | `missionId`; P | absent | E: intended checked fail-and-retain-history, not Rust abandon's remove-record behavior; Python wrapper mistakenly accepts. | [legacy manager](../../../deprecated/python/cell/MissionManager.py) | [P22](work-packets.md#p22) |
| `.missionadvance` | No | `missionId stepId`; P | absent | E: active/owned-step/forward validation, accurate objective snapshots and events; not merely native advance. | [progression](../../../crates/services/src/cell/missions/progression.rs) | [P21](work-packets.md#p21) |
| `.missionclear` | No | `missionId`; P | absent | N/E: forget record durably and clean mission-bound state; no production delete receiver found. | [mission persistence](../../../crates/services/src/base/world_entry/methods/missions.rs) | [P23](work-packets.md#p23) |
| `.missionclearactive` | No | `()`; P | absent | E: clear active records including hidden; fix Python wrapper that clears all tracked records. | [mission model](../../../crates/entity/src/missions.rs) | [P24](work-packets.md#p24) |
| `.missionclearhistory` | No | `()`; P | absent | E: clear non-active records only; preserve visible and hidden active missions. | [mission model](../../../crates/entity/src/missions.rs) | [P24](work-packets.md#p24) |
| `.missioncomplete` | No | `missionId`; P | absent | E: guard before mutation/DB; repeated, failed or absent records must not fabricate completion or increment repeats. No catalog payout. | [content complete](../../../crates/services/src/cell/content/executor/mission.rs) | [P25](work-packets.md#p25) |
| `.missionfail` | Yes | `missionId`; P | partial | E: active guard, mutation/RPC/feedback/Discord exist; missing durable snapshot, failed objectives and cleanup. | [console mission](../../../crates/services/src/cell/console/mission.rs) | [P22](work-packets.md#p22) |
| `.missionlist` | No | `()`; P | absent | W/E: native list hides hidden missions; legacy active filter includes them. Read selected player, reply to GM. | [native missions](../../../crates/services/src/cell/cell_methods/gm/missions.rs) | [P19](work-packets.md#p19) |
| `.missionlistfull` | No | `()`; P | absent | W/E: all_missions reuse; distinguish failed/not-active labels rather than generic other. | [native missions](../../../crates/services/src/cell/cell_methods/gm/missions.rs) | [P19](work-packets.md#p19) |
| `.missiondetails` | No | `missionId`; P | absent | E: summary exists; expand names/objectives/history/runtime-chain diagnostics, label unavailable metadata truthfully. | [native missions](../../../crates/services/src/cell/cell_methods/gm/missions.rs) | [P19](work-packets.md#p19) |
| `.missionreload` | No | `missionId`; P | absent | N/E: per-instance script-variable/rebind/restore equivalent needs design; global engine swap is not equivalent. | [legacy manager](../../../deprecated/python/cell/MissionManager.py) | [G09](work-packets.md#g09) |
| `.missionreset` | No | `missionId stepId`; P | absent | E: force ordering only; still require active mission and owned step. Not clear/reaccept or history resurrection. | [legacy manager](../../../deprecated/python/cell/MissionManager.py) | [P21](work-packets.md#p21) |
| `.missionrewards` | Yes | `missionId`; P | partial | N/E: state text only; catalog and generic grants exist, actionable authoritative offer/claim does not. | [console mission](../../../crates/services/src/cell/console/mission.rs) | [G04](work-packets.md#g04) |

## Crafting

Legacy: [Crafting.py](../../../deprecated/python/cell/commands/Crafting.py), [Crafter.py](../../../deprecated/python/cell/Crafter.py). Active: [console crafting](../../../crates/services/src/cell/console/crafting.rs), [base handlers](../../../crates/services/src/base/crafting/handlers.rs), [persistence](../../../crates/services/src/base/crafting/persistence.rs). Native expertise 139 and ASP 140 are related methods, not proof of matching dots.

| Command | Registered | Arguments; target | Status | Reuse and gap | Source | Packet |
|---|---|---|---|---|---|---|
| `.appliedscience` | No | `amount`; P | absent | W/E: GrantAppliedSciencePoints saves and replies; add live ASP property synchronization. | [handlers](../../../crates/services/src/base/crafting/handlers.rs) | [P36](work-packets.md#p36) |
| `.racialparadigm` | No | `racialParadigmId level`; P | absent | N/E: storage/load/save exists; add validated set-level receiver and client callback. | [crafting state](../../../crates/entity/src/crafting.rs) | [P36](work-packets.md#p36) |
| `.learndiscipline` | Yes | `disciplineId [expertise=1]`; P | partial | E: additive GrantExpertise clamps 0-100, accepts unknown positive IDs; validate catalog and initial versus existing behavior. | [handlers](../../../crates/services/src/base/crafting/handlers.rs) | [P37](work-packets.md#p37) |
| `.forgetdiscipline` | Yes | `disciplineId`; P | partial | E: sends -100, a supported delta, but leaves/creates known membership at zero; real forgetting removes membership and expertise. | [persistence](../../../crates/services/src/base/crafting/persistence.rs) | [P37](work-packets.md#p37) |
| `.allcraft` | Yes | `()`; P | no-op | N/E: needs option groups, all blueprints, paradigms 7 and discipline initialization 50; approve bounded catalog batch semantics. | [console crafting](../../../crates/services/src/cell/console/crafting.rs) | [G13](work-packets.md#g13) |

## Resource

Legacy: [Resource.py](../../../deprecated/python/cell/commands/Resource.py). Active: [query](../../../crates/services/src/cell/console/query.rs), [spawn](../../../crates/services/src/cell/console/spawn.rs), [seed](../../../crates/services/src/cell/console/seed.rs), [authoring SQL sink](../../../crates/services/src/base/console_authoring.rs). Native spawn 185/despawn 186 exist; dot restoration still needs target, heading and lifecycle handling.

| Command | Registered | Arguments; target | Status | Reuse and gap | Source | Packet |
|---|---|---|---|---|---|---|
| `.searchitem` | Yes | `name [name2='']`; O | partial | E/V: real search; silent 25-result cap and SQL wildcards differ from literal legacy substring search. Retain bounds, disclose truncation. | [search sink](../../../crates/services/src/base/console_authoring.rs#L130) | [P01](work-packets.md#p01) |
| `.searchmission` | Yes | `name [name2='']`; O | partial | E/V: same cap/wildcard gaps; verify two-token join, case behavior, real results and errors. | [search sink](../../../crates/services/src/base/console_authoring.rs#L130) | [P01](work-packets.md#p01) |
| `.searchtemplate` | Yes | `name [name2='']`; O | partial | E/V: same cap/wildcard gaps; literal matching and truthful bounded-result feedback need regression coverage. | [search sink](../../../crates/services/src/base/console_authoring.rs#L130) | [P01](work-packets.md#p01) |
| `.reloadres` | Yes | `[category]`; O | no-op | N/E: resource-cache reload needs scope/atomicity policy; global chain reload does not reload every resource category. | [server](../../../crates/services/src/cell/console/server.rs) | [G09](work-packets.md#g09) |
| `.respawnall` | Yes | `()`; O, caller space | partial | N/E: resets existing NPC fields only; legacy reloads templates and removes/loads spawns, requiring cleanup and fan-out design. | [spawn](../../../crates/services/src/cell/console/spawn.rs) | [G01](work-packets.md#g01) |
| `.autosavespawn` | Yes | `autosave` bool; O | partial | E: preference toggles but no spawn consumer; save the newly created entity, not stale selection from Python wrapper. | [spawn](../../../crates/services/src/cell/console/spawn.rs) | [P11](work-packets.md#p11) |
| `.spawn` | No | `templateId`; O | absent | W/E: reuse GmSpawnNpc roundtrip with catalog validation, caller placement/heading and actual creation result. | [native GM](../../../crates/services/src/cell/cell_methods/gm/mod.rs) | [P08](work-packets.md#p08) |
| `.spawnrandom` | Yes | `templateId xRange zRange [count=1]`; O | partial | E: deterministic ring differs from independent uniform XZ interior offsets; retain 1-50 safety bound, test seeded RNG and heading. | [spawn](../../../crates/services/src/cell/console/spawn.rs) | [P12](work-packets.md#p12) |
| `.despawn` | No | `()`; S | absent | W/E: native destruction exists; use safe selected-entity cleanup/witness removal, not persistence deletion. | [native GM](../../../crates/services/src/cell/cell_methods/gm/mod.rs) | [P08](work-packets.md#p08) |
| `.savespawn` | Yes | `()`; S | partial | N/E: INSERT lacks returned spawn ID/callback; repeated saves reinsert. UPDATE omits world/template; use correlated typed result. | [spawn](../../../crates/services/src/cell/console/spawn.rs) | [P09](work-packets.md#p09) |
| `.delspawn` | Yes | `()`; S | partial | E: Rust destroys runtime entity immediately after queued DELETE, even on DB failure; legacy clears spawnId and keeps entity alive. | [spawn](../../../crates/services/src/cell/console/spawn.rs) | [P10](work-packets.md#p10) |

## Net

Legacy: [Net.py](../../../deprecated/python/cell/commands/Net.py). Active: [net.rs](../../../crates/services/src/cell/console/net.rs). Wire evidence comes from [SGWBeing.def](../../../entities/defs/interfaces/SGWBeing.def), [SGWPlayer.def](../../../entities/defs/SGWPlayer.def), and the [dispatch table](../../protocol/client-method-dispatch-table.md), not source comments alone. Net debug recipients sometimes intentionally differ from the selected subject.

| Command | Registered | Arguments; target | Status | Reuse and gap | Source | Packet |
|---|---|---|---|---|---|---|
| `.net_dhd` | No | `[origin]`; O | absent | W/E: display DHD, optional world-gate origin; Python dereferences optional target unsafely. Native gmDHD 159 dials, not displays. | [legacy net](../../../deprecated/python/cell/commands/Net.py) | [P43](work-packets.md#p43) |
| `.net_seq` | Yes | `sequenceId [viewType=EventInvoker]`; S | source-implemented | V: selected source/target and self+witness send exist; confirm default enum, exact bytes and recipients. | [net](../../../crates/services/src/cell/console/net.rs) | [P40](work-packets.md#p40) |
| `.net_seqto` | Yes | `sequenceId [viewType=EventInvoker]`; O | source-implemented | V: caller -> selection, falling back to caller, with caller broadcaster; verify bytes/default. | [net](../../../crates/services/src/cell/console/net.rs) | [P40](work-packets.md#p40) |
| `.net_seqfrom` | Yes | `sequenceId [viewType=EventInvoker]`; S | source-implemented | V: selection -> caller, selected broadcaster; verify direction/recipients rather than assuming a slash alias. | [net](../../../crates/services/src/cell/console/net.rs) | [P40](work-packets.md#p40) |
| `.net_timer` | Yes | `id type [totalTime=1] [secondaryId=0]`; S | partial | E: 21-byte block with secondaryId is fixed (#719) and pinned by `legacy_p38_`; still open: caller used as source, completion equals duration (legacy sends `getGameTime() + totalTime`). Def Type is INT8. | [net](../../../crates/services/src/cell/console/net.rs) | [P38](work-packets.md#p38) |
| `.net_timeofday` | No | `time wind weather`; P, caller receives | absent | W: legacy is not a stub; reuse client 102 serialization with float time/wind, integer weather. | [map-loaded](../../../crates/services/src/mercury/world_data/map_loaded.rs) | [P43](work-packets.md#p43) |
| `.net_mapinfo` | Yes | `sysId keyId lifetime [delete=False] [sysTypeId=0]`; P | partial | E: real packet exists, but WorldID uses caller space ID; restore resource world ID, caller position, target recipient, strict optional parsing. | [net](../../../crates/services/src/cell/console/net.rs) | [P39](work-packets.md#p39) |
| `.net_speak` | Yes | `message [channel='say']`; S, caller receives | partial | E: five names versus twelve, witness broadcast versus caller-only debug. Follow named body contract despite erroneous Python int annotation. | [net](../../../crates/services/src/cell/console/net.rs) | [P41](work-packets.md#p41) |
| `.net_minigame` | No | `gameId [difficulty=1] [techCompetency=1]`; P | absent | N/W: StartMinigame/session plumbing exists; approve debug parameter/host/result adapter, not whole subsystem replacement. | [minigame dispatch](../../../crates/services/src/base/world_entry/cell_dispatch/minigame.rs) | [G14](work-packets.md#g14) |
| `.net_dialog` | Yes | `dialogId`; O, caller receives | partial | E/V: body requires target despite optional registration; trace displayDialog(None) before choosing safe equivalent. | [net](../../../crates/services/src/cell/console/net.rs), [legacy player](../../../deprecated/python/cell/SGWPlayer.py) | [P42](work-packets.md#p42) |
| `.net_challenge` | Yes | `challenge type object id1 id2`; O | source-implemented | V: four INT32 fields around a WSTRING, not five integers; exact client 142 bytes/recipient still need tests. | [net](../../../crates/services/src/cell/console/net.rs) | [P40](work-packets.md#p40) |

## Misc

Legacy: [Misc.py](../../../deprecated/python/cell/commands/Misc.py). Active: [net](../../../crates/services/src/cell/console/net.rs), [query](../../../crates/services/src/cell/console/query.rs), [follow tick](../../../crates/services/src/cell/service/npc_ai/follow.rs), [path query](../../../crates/services/src/cell/space_manager/spatial.rs). Different engine models need approved equivalents, not a blanket not-applicable label.

| Command | Registered | Arguments; target | Status | Reuse and gap | Source | Packet |
|---|---|---|---|---|---|---|
| `.debug_velocity` | Yes | `velocityX velocityY velocityZ`; S | partial | E/V: velocity array changes; tick consumption and resulting movement were not traced to runtime effect. | [net](../../../crates/services/src/cell/console/net.rs) | [P33](work-packets.md#p33) |
| `.debug_controller` | Yes | `()`; S | no-op | N: debug movement controller equivalent needs bounded ownership/tick/cleanup design. | [net](../../../crates/services/src/cell/console/net.rs) | [G12](work-packets.md#g12) |
| `.debug_follow` | Yes | `()`; S | partial | E: real Follow state/tick; repeat cancels same caller only, whereas legacy cancels any existing action. | [follow tick](../../../crates/services/src/cell/service/npc_ai/follow.rs) | [P33](work-packets.md#p33) |
| `.debug_paths` | No | `()`; S, per-GM toggle | absent | N: per-GM path subscription, cleanup and supported onShowPath wire contract must be established. | [legacy misc](../../../deprecated/python/cell/commands/Misc.py) | [G12](work-packets.md#g12) |
| `.debug_nav` | No | `()`; S, caller-position markers | absent | N/W: find_path exists; new per-GM rolling start marker and result display, unreachable handling. | [spatial](../../../crates/services/src/cell/space_manager/spatial.rs) | [G12](work-packets.md#g12) |
| `.debug_events` | No | `()`; S, caller subscriptions in Python | absent | N: Rust chain/event inspection policy needed; Python subscriptionsByEvent has no direct model equivalent. | [legacy misc](../../../deprecated/python/cell/commands/Misc.py) | [G12](work-packets.md#g12) |
| `.debug_ai` | No | `()`; M | absent | N: target AI diagnostics with per-GM subscription, instrumentation and teardown; not merely a loglevel change. | [legacy misc](../../../deprecated/python/cell/commands/Misc.py) | [G12](work-packets.md#g12) |
| `.debug_inven` | No | `()`; P | absent | W/E: inspect actual inventory ownership/slots/orphans; Python deletion queues are not Rust queues. No mutations. | [inventory model](../../../deprecated/python/cell/commands/Misc.py) | [P32](work-packets.md#p32) |
| `.debug_invreload` | No | `()`; P | absent | N/E: safe live rehydration must reconcile locks, equipment, bandoliers and outbox before replacing state. | [inventory methods](../../../crates/services/src/base/world_entry/methods/inventory/) | [G11](work-packets.md#g11) |
| `.reloadscripts` | No | `()`; O | absent | N/E: Python module reload is inapplicable as a mechanism; modern content reload exists and needs an approved equivalent scope. | [engine loop](../../../crates/services/src/cell/service/message_loop.rs) | [G09](work-packets.md#g09) |
| `.players` | Yes | `()`; O | partial | E: Rust lists current-space numeric IDs; legacy lists names/worlds across CellApp, including in-transition players. | [query](../../../crates/services/src/cell/console/query.rs) | [P04](work-packets.md#p04) |

## Evidence Qualifications

### Persistence And Authoring

[seed::record](../../../crates/services/src/cell/console/seed.rs) sends live authoring SQL and writes/buffers it before confirmation. Confirm emits grouped SQL; cancel does not roll back. The [base authoring sink](../../../crates/services/src/base/console_authoring.rs) returns only GM rowcount feedback, not an entity-correlated spawn ID. Repeated new-spawn saves therefore remain INSERTs. Preserve the current authoring model, but design a typed result where identity matters. Spawn UPDATE covers XYZ/heading/tag, not appearance, event set or name ID; it also omits the legacy world/template fields.

The six [server handlers](../../../crates/services/src/cell/console/server.rs) for save, reloadmap, reloadres, removerespawner, loglevel and logclient are feedback-only. Incremental persistence is not a proof of full save coverage: mission fail is a concrete counterexample. Runtime tracing reload needs plumbing, not a blanket impossible/unsafe designation.

### Missions And Rewards

The [mission UPSERT](../../../crates/services/src/base/world_entry/methods/missions.rs) inserts missing records and updates repeats; older contrary notes are stale. The problems are upstream: native/dot transitions can omit MissionUpdate; content snapshots use step IDs as objective IDs; completion discards history arrays; [hydration](../../../crates/services/src/cell/service/base_messages/player_init/mod.rs) loses metadata, treats missing steps as Some(0) and ignores failed objectives. [MissionInstance](../../../crates/entity/src/missions.rs) lacks failed-objective storage. Design accurate snapshot/hydration and durable deletion before restoring lifecycle adapters.

[Content completion](../../../crates/services/src/cell/content/executor/mission.rs) guards the follow-up event too late: mutation and completed-state persistence happen first. An absent/failed/already-completed mission must not mutate state, repeats, DB or events. Distinguish fail/history-retaining abandon from total clear; include mission-bound dialog cleanup and the approved failure-flag policy.

Reward data exists in [missions](../../../db/resources/Missions/Tables/missions.sql), [mission_reward_groups](../../../db/resources/Missions/Tables/mission_reward_groups.sql) and [mission_rewards](../../../db/resources/Missions/Tables/mission_rewards.sql). [MissionReward::grant_to](../../../crates/game/src/missions/rewards.rs) is a todo and [chosenRewards dispatch](../../../crates/services/src/cell/cell_methods/player/world/mod.rs) is unimplemented. Separate grant transactions do not supply an atomic exactly-once claim.

Client 127 is `onMissionRewardsDisplay(Rewards, missionId)`; [alias.xml](../../../entities/defs/alias.xml) defines groups, choice counts and indexed items. The [SGWPlayer definition](../../../entities/defs/SGWPlayer.def) gives incoming method 87 arguments as RewardChoices then missionId, despite a reversed order in an older RE summary. Python's display path is actionable: it sets pending state, can claim immediately for no item groups, and claim can complete. D08 deliberately separates plain completion from catalog payout while leaving offer/claim ordering for approval.

The [admin content route](../../../crates/admin-api/src/routes/content.rs) queues ReloadContentEngine and the [cell loop](../../../crates/services/src/cell/service/message_loop.rs) swaps build_engine output. This is a global chain reload, not arbitrary resource-cache reload or selected-mission restoration. The current load-failure path can replace working chains with an empty engine; an approved reload implementation must retain working state on failure.

### Player, Crafting And Client Effects

The Python [removeItem wrapper](../../../deprecated/python/cell/commands/Player.py) advertises designId but calls the [instance-ID inventory method](../../../deprecated/python/cell/Inventory.py); removeItemByDesign is the aggregate variant. D07 records intended design-ID semantics explicitly. Existing Rust remove-by-type locks/drains only the first matching stack. Do not change native instance-ID removal by accident.

[Crafting persistence](../../../crates/services/src/base/crafting/persistence.rs) writes zero expertise rows and known discipline IDs. A -100 grant does not forget membership. ASP currently saves and replies without a live property update; an explanatory comment about a future refresh is not traced behavior. Legacy allcraft leaves already-known disciplines unchanged rather than necessarily raising them to 50; settle idempotence under G13.

[find_or_create_space](../../../crates/services/src/cell/space_manager/lifecycle.rs) creates a fresh instance for instanced worlds. A world-name GateTravel call does not prove that `.goto name` joins that player's current instance. Selected-subject IDs also cannot be substituted into caller-scoped handlers as a shortcut for permission/feedback separation.

The timer body in [net.rs](../../../crates/services/src/cell/console/net.rs) conflicts with [SGWBeing.def](../../../entities/defs/interfaces/SGWBeing.def): six fields total 21 bytes, including signed INT8 Type, selected source, secondary ID and absolute game-time completion. Map info is implemented but confuses WorldID with space ID. Net speech's named channels are `say`, `emote`, `yell`, `team`, `squad`, `command`, `officer`, `server`, `feedback`, `tell`, `splash`, `chat`; the Python type annotation is the bug, not grounds to drop named channels.

The [StartMinigame receiver](../../../crates/services/src/base/world_entry/cell_dispatch/minigame.rs) and [content executor](../../../crates/services/src/cell/content/executor/mod.rs) show working session plumbing distinct from stub native minigame handlers. Limit G14 to debug adapter gaps. Path visualization support, velocity tick effect, appearance recomposition and optional dialog handling remain explicit verification obligations, not assumed runtime successes.
