---
title: "Gap Analysis: NPC Systems (§16-§17)"
type: explanation
audience: engineers
last_updated: 2026-10-03
companion_docs:
  - ../gap-analysis.md
  - ../project-status.md
---

# Gap Analysis: NPC Systems (§16-§17)

> Part of the [Gap Analysis](../gap-analysis.md), split out of it on 2026-10-03 with no change to any row. The status taxonomy, the evidence bar and the Summary Completion Matrix are in the main file; each matrix row counts the feature rows of its section here, so change both together.

## NPC Systems

### 16. NPC AI and Behavior --- IM

- **Confidence**: HIGH for the state machine, aggro, leash and cover code (re-read 2026-09-25 after the NPC AI restoration campaign). MEDIUM for tuning: the D-NA09 radii (aggro 18 u, assist 10 u, leash 50 u, band 4 u) are starting guesses. One in-client session (UAT-1, 2026-09-25) covers the build before the NA23/NA24/NA27 fixes.
- **Documentation**:
  - [gameplay/npc-ai.md](../gameplay/npc-ai.md);
  - [architecture/cover-system.md](../architecture/cover-system.md);
  - [operations/npc-ai-telemetry-runbook.md](../operations/npc-ai-telemetry-runbook.md);
  - campaign ledger [analysis/npc-ai-restoration/](../analysis/npc-ai-restoration/README.md), with the UAT-1 worknote [worknotes/uat-1.md](../analysis/npc-ai-restoration/worknotes/uat-1.md);
  - the [2026-09-18 colo playtest](../analysis/playtests/2026-09-18-colo-castle/README.md).
- **Rust code**:
  - [`crates/cell-combat/src/cell/service/npc_ai/`](../../crates/cell-combat/src/cell/service/npc_ai/): **51 files, 8,130 lines excluding tests** (11,500 with them). It holds dispatch, the `transition.rs` state-change helper, `idle_aggro.rs`, `aggro_gates.rs`, `assist.rs`, `fight.rs`, `fight_cover.rs`, `fight_target.rs`, `chase/`, `leash/`, `path_failure/`, `movement_stop/`, patrol, wander, investigate, follow, `lifecycle/` and the NA02 `detectors/`.
  - [`crates/cell-cover/src/cell/cover/`](../../crates/cell-cover/src/cell/cover/), the `cimmeria-cell-cover` crate: **2,827 lines excluding tests** (4,345 with them). Includes `peek.rs`. `stance.rs` (222 lines) stays in [`crates/services/src/cell/cover/`](../../crates/cell-cover/src/cell/cover/), which re-exports the crate at its old path.
  - [`crates/cell-world/src/cell/space_manager/cover_sight.rs`](../../crates/cell-world/src/cell/space_manager/cover_sight.rs): LoS policy.
  - [`crates/cell-world/src/cell/combat/aggression.rs`](../../crates/cell-world/src/cell/combat/aggression.rs) and [`faction_reaction.rs`](../../crates/cell-world/src/cell/combat/faction_reaction.rs).
  - [`crates/cell/src/cell/service/ticks/npc_ground.rs`](../../crates/cell/src/cell/service/ticks/npc_ground.rs) and `npc_movement.rs`.
  - [`crates/occluder/`](../../crates/occluder/): 4,048 lines, with a `data/spaces/<world>.occ` for all 23 worlds.
  - [`crates/cell-console/src/cell/console/aggro.rs`](../../crates/cell-console/src/cell/console/aggro.rs): the `.aggro` GM toggle.
- **Recent PRs**:
  - Earlier: #368 (ability buckets and auto-aggro), #428 (movement states), #429 (cover).
  - #677 (wire facing, attacker re-face, follower height).
  - Telemetry: #776 (NA00, transition helper and OTLP identity), #781 (NA02, detectors), #782 (NA03, dashboard and runbook).
  - #779 (NA10, zero velocity on stop; the malformed movement-type `onSequence` removed).
  - #783 (NA11, per-step ground clamp).
  - **#785 (NA12, leash on NPC-to-spawn, walk home, evade, reset, player combat drain).**
  - **#787 (NA13, faction-derived proximity aggro with radius, floor band, LoS and the GM `.aggro` toggle).**
  - **#789 (NA14, same-room assist).**
  - #788 (NA15, path robustness).
  - #786 (NA16, stationary LoS relaxation).
  - #780 (NA21, world-space cover seeds).
  - **#790 (NA22, cover as firing positions and Cover Stance).**
  - #791 (NA24, UAT-1 findings 4-8).
  - #793 (NA23, cover peek point and flank hysteresis).
  - **#797 (NA27, collision-geometry occluder line of sight, closes #784).**
- **Path forward**:
  - Owner re-UAT on a build that has #791/#793/#797. The [session-resume checklist](../analysis/npc-ai-restoration/handoffs/session-resume.md) covers leash walk-home, ramps, running-in-place, cover hold/seek and `.aggro off`.
  - Tune the radii from UAT.
  - Send `onAggressionOverrideUpdate` to the client (#330; the SGWMob method index is unverified).
  - Give Cover Stance a combat effect: `COVER_DEFENSE` is not read by hit resolution, and the magnitude is undecided.
  - Add a player fire-time LoS check (`NoLOS = 40`, still open after #797).
  - Decide whether SGC_W1 Ba'al Jaffa should stay passive (D-NA01 made them hostile).
  - Hearing radius, mob groups and kill-credit tapping remain unimplemented.

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| AI state machine | NT | -- | cell/service/npc_ai/dispatch.rs, transition.rs | **Re-verified 2026-09-25.** All 11 states are dispatched (dispatch.rs:171-180). Every change now goes through the NA00 `transition.rs` helper, which logs the reason and cause (#776). Hostile Idle NPCs are ticked for the aggro scan (NA13), and a finished fight no longer parks the NPC Idle forever (NA12, #785). UAT-1 exercised Idle→Fighting→Dead in-client, but leash, return-home and several fixes landed after it |
| Spawning state | CW | -- | spawner/npcs.rs | Loads ammo, transitions to Idle |
| Idle state | CW | -- | spawner/npcs.rs | Waits for threat. Chain-armed spawns 20 and 10 waited for their chains at UAT-1 ([uat-1.md](../analysis/npc-ai-restoration/worknotes/uat-1.md)) |
| Fighting state | NT | -- | cell/service/npc_ai/fight.rs, fight_cover.rs, fight_target.rs, chase/ | **Re-verified 2026-09-25.** The chase moved into `chase/` (NA15, #788). The attack check uses `attack_line_of_sight` (fight.rs:160). Attackers re-face every tick (#677). At UAT-1 the guards fought and killed the tester, but guards in cover shot through walls and flank-churned. Both are fixed on main by #793, and then #797 (occluder LoS), which a client has not seen yet |
| Threat accumulation | IM | -- | cell/combat/threat/ | -healthChange*2 - focusChange. **Re-verified 2026-09-25:** a dying player now leaves every threat list (NA24, #791) |
| Top-threat targeting | IM | -- | cell/combat/threat/, npc_ai/fight_target.rs | Linear scan with dead pruning. A `BSF_DEAD` target counts as dead whatever its HEALTH (UAT-1 finding 4, fixed in #791). Multi-attacker targeting is not client-tested |
| Ability bucket selection | CW | -- | spawner/abilities.rs, npc_ai/ability_select.rs | PR #368 three-bucket model (usable/cooling/needs-ammo) |
| Auto-reload | CW | -- | cell/abilities/ | PR #394 |
| Loot on death | CW | -- | cell/abilities/loot_drop.rs | Generates loot, sets interaction |
| Aggression override | IM | -- | spawner/npcs.rs, cell/combat/aggression.rs, console/aggro.rs | **Re-verified 2026-09-25.** The override comes from `spawnlist.aggression_override`, a chain's `set_aggression` or the console, else the faction table (NA13, #787). The server side held in-client at UAT-1: spawns 20 and 10 waited for chains 1032 and 1008. **Still IM:** the client is never told the level, because `onAggressionOverrideUpdate` is unsent (open #330, SGWMob method index unverified) |
| Auto-aggro | CW | -- | PR #368, chains 1008/1032 | Chain-armed aggro. The Castle drone encounter was verified end to end, and at UAT-1 the PRU fired across the med-station desk once armed ([uat-1.md](../analysis/npc-ai-restoration/worknotes/uat-1.md) "Confirmed working"). Proximity aggro is its own row below |
| Investigating state | IM | -- | cell/service/npc_ai/investigate.rs | 197 lines; POI set by content action `Action::SetNpcPoi` (PR #428). NA11 snaps the endpoints onto the mesh |
| Leashing state | NT | -- | cell/service/npc_ai/leash/ (mod.rs, begin.rs, policy.rs) | **Re-verified 2026-09-25.** Rewritten by NA12 (#785), replacing the snap-home teleport that the 2026-09-18 playtest (H6) found never fired and would desync. Leash is now measured on the NPC's own distance from spawn (`entity_templates.leash_distance`, default 50 u, 5 u band). The NPC walks home on the navmesh, evades, heals on arrival, drains player combat and holds off re-aggro for 5 s. It snaps only with no route or after 20 s. UAT-1 did not record a leash |
| Patrol state | IM | -- | cell/service/npc_ai/patrol.rs | 262 lines; Idle auto-promotes to Patrol when `has_patrol`. NA11 snaps the endpoints onto the mesh |
| Wander state | IM | -- | cell/service/npc_ai/wander.rs | 241 lines; off-mesh candidates rejected; Idle auto-promotes on `has_wander` |
| Follow state | IM | -- | cell/service/npc_ai/follow.rs | 407 lines. **Re-verified 2026-09-25.** At UAT-1 Col Marsh's follow never ticked, because being-class NPCs were excluded from the AI tick (finding 6, fixed in #791, `ai_driven_npc_entity_ids`). The playtest's H7 silent early-returns were addressed only partly: Coppleman still has no follow chain. Not re-tested |
| Despawning state | IM | -- | cell/service/npc_ai/lifecycle/ | Terminal states (Despawning / Submit / Error) are dispatched at npc_ai/dispatch.rs:177-179 |
| Cover system | IM | -- | cell/cover/, npc_ai/fight_cover.rs, cover/peek.rs, cover/stance.rs | **Re-verified 2026-09-25.** The cover seeds are now real world-space nodes extracted from the maps: 236 for Castle_CellBlock and 3,788 for Castle (NA21, #780). The 9,346 prefab-local `.pak` rows were dropped. NPCs hold authored cover from spawn, seek a covered firing position and get Cover Stance (NA22, #790). **In-client at UAT-1:** guards held their slots, and Stance was granted and removed in balance. **Defects found:** guards in cover never aggroed, shot through walls and flank-churned; fixed by #793 and #797, not re-tested. **Still IM:** Cover Stance has no combat effect (`COVER_DEFENSE` is unread). The crouch pose has no server wire (D-NA10); an owner experiment decides it |
| Hearing system | KM | -- | -- | hearingRadius defined; still no runtime consumer (grep 2026-09-25) |
| Mob groups | KM | -- | -- | mobGroup is defined but unused. Assist aggro (below) recruits by faction and radius, not mobGroup |
| Tapping / kill credit | KM | -- | -- | tappedEntity is defined but unused. `handle_use_ability_with_kill_credit` credits the killing blow, not a tap |
| XP on kill | CW | -- | cell/abilities/damage_apply/ | kill_xp(), 10×mob_level Cell→Base pipeline |
| Faction proximity aggro | NT | -- | npc_ai/idle_aggro.rs, aggro_gates.rs, cell/combat/aggression.rs:49, console/aggro.rs | **New 2026-09-25.** NA13 (#787), D-NA01/02. A HOSTILE NPC, by override or else the 2009 `FACTION_REACTION_TABLE`, engages the nearest player within `aggro_radius` (18 u), inside a 4 u floor band, with LoS. An `Unknown` LoS verdict fails closed. GMs are aggroed unless they set `.aggro off`. **At UAT-1** there was no cross-floor aggro and one proximity aggro at 4.6 u. Guards spawned in cover never aggroed (finding 1); the fix is on main (#793, then the occluder in #797) and has not been re-tested. Before this, proximity aggro was structurally dead (playtest H5) |
| Assist aggro | CW | -- | npc_ai/assist.rs:61 | **New 2026-09-25.** NA14 (#789), D-NA04, a marked deviation from legacy. Same-faction hostile Idle/Patrol/Wander NPCs within `assist_radius` (10 u) of the victim join, with no chaining. **In-client:** at UAT-1 "the MessHall guards assisted each other, both ways" ([uat-1.md](../analysis/npc-ai-restoration/worknotes/uat-1.md)). Caveat: #797 has since moved the assist LoS source to the occluder |
| NPC line of sight | NT | -- | space_manager/cover_sight.rs:113, crates/occluder/, data/spaces/*.occ | **New 2026-09-25.** NA27 (#797), D-NA13, closes #784. Aggro, assist, the attack check (`los_policy=occluder`) and cover sight all use a paged collision-geometry occluder at 1.5 m eyes; all 23 worlds ship one, 131 MB. This replaces the navmesh ray, which was wrong on 38-49% of its "blocked" answers. The navmesh stopgaps, the stationary relaxation (D-NA11, #786) and the cover peek point (D-NA12, #793), now apply only where there is no `.occ`. **At UAT-1** (pre-occluder) the drone fired across the desk under `stationary_relaxed`. The occluder itself has not been seen in a client |
| NPC grounding and stop hygiene | NT | -- | ticks/npc_ground.rs:45, ticks/npc_movement.rs, npc_ai/movement_stop/, npc_ai/chase/, npc_ai/path_failure/ | **New 2026-09-25.** These fix the 2026-09-18 playtest symptoms: floating (H2/H4), running in place, and stuck NPCs. **Ground and stop:** NA01 (#774) makes the height query storey-aware. NA10 (#779) sends zero velocity on stop. NA11 (#783) clamps every step and arrival to the ground, backs up with `moveAlongSurface`, and grounds the spawn Y. **Paths:** NA15 (#788) holds at mesh-island edges, snaps off-mesh starts, stops 1 u short of the target and routes to off-mesh targets. **Navmesh coverage:** every world now has a navmesh (#794), with tiled meshes for the large exteriors (#796). **Wire facing:** fixed in #677. None of this is recorded as seen in a client |

### 17. Spawn System --- IM

- **Confidence**: HIGH (re-read 2026-09-25). The **direct cell-side spawn from `resources.spawnlist`** covers the full spawn, death and respawn lifecycle, and the 2026-09-18 playtest measured 19 of 19 respawns at 120 s. The original `SGWSpawnRegion` / `SGWSpawnSet` population-control layer was **never built**: the Python server had empty stubs, and Rust spawns straight from the cell. Rows describing that layer were previously credited to `spawner/regions.rs`, but that file loads GenericRegion (client-hinted trigger regions). Issue #62 (triaged 2026-09-25) records the misattribution.
- **Documentation**: [gameplay/spawn-system.md](../gameplay/spawn-system.md). Its "Original Design (not implemented as such)" section is accurate.
- **Rust code**: [`crates/cell-catalog/src/cell/spawner/`](../../crates/cell-catalog/src/cell/spawner/) (split out of `cimmeria-services` in 2026-09) is **2,849 lines excluding tests, about 96 tests** (7,660 lines with the tests, now in `spawner/tests/` there and `cell/spawner_tests/` in `cimmeria-cell-world`, `cimmeria-cell-combat` (the Harset guards) and `cimmeria-services` (the GM-spawn parity guard)). It holds:
  - `npcs.rs`: the spawnlist + template load (`SpawnRecord` itself is `cimmeria_wire::cell::spawn_record`), `aggression_override`, `leash_distance`, `use_cover`;
  - `templates.rs`: prototype records for content-engine `spawn_entity`;
  - `regions.rs`: GenericRegion, not spawn regions;
  - `respawners.rs`: player defeat-window respawn points;
  - `stargates.rs`, `dialogs.rs`, `loot.rs`, `missions.rs`, `abilities.rs`, `worlds.rs`.

  Populating spaces from the records is [`crates/cell-world/src/cell/space_manager/npc_population.rs`](../../crates/cell-world/src/cell/space_manager/npc_population.rs). NPC respawn is [`crates/cell/src/cell/service/ticks/npc_respawn/`](../../crates/cell/src/cell/service/ticks/npc_respawn/). Mission-scoped spawns are [`crates/cell-world/src/cell/space_manager/spawn.rs`](../../crates/cell-world/src/cell/space_manager/spawn.rs) and `cell/content/executor/spawn/`. The standalone `crates/game/src/world/spawning.rs` `SpawnSet` model has no caller in `cimmeria-services`.
- **Recent PRs**:
  - Castle and Harset population: #667 (CA05, Castle World 8 story actors and respawn timers), #662 and #682 (Harset spawn/despawn actions, templates and spawns), #717 (Harset placements).
  - #640 (GM `.spawn` / `.despawn`).
  - #783 (NA11, spawn Y grounded on the navmesh).
  - #785, #787, #789 and #790 add the `leash_distance`, `aggression_override`, `assist_radius` and `use_cover` spawn/template columns.
  - #791 (spawn 244 moved onto the mesh).
  - #795 (NA29, five world-57 spawns made mobile).
- **Path forward**:
  - Decide whether region/set population control (MaxActiveSets, population caps, set cooldowns, weighted spawn tables) is needed at all. The shipped content spawns fixed rows per spawnlist entry, and `spawn_sets.sql` / `spawn_points.sql` are empty in the seed.
  - If it is needed, port it cell-side against `resources.spawnlist`.
  - Time-of-day spawns and linked sets remain unknown in semantics.

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| SpawnRegion entity | KM | -- | -- | **Downgraded 2026-09-25** (was IM). No `SGWSpawnRegion` in `crates/services`. `spawner/regions.rs` is GenericRegion loading. See issue #62 and spawn-system.md "not implemented as such" |
| SpawnSet entity | KM | -- | -- | **Downgraded 2026-09-25** (was IM). No `SGWSpawnSet` in `crates/services`. `crates/game/src/world/spawning.rs` has an unused model; `spawn_sets.sql` is empty. Issue #62 |
| Region activation | KM | -- | -- | **Downgraded 2026-09-25** (was CW). No spawn-region Activated/Deactivated lifecycle exists; the old citation was GenericRegion. Issue #62 |
| Set activation | KM | -- | -- | **Downgraded 2026-09-25** (was CW). No set Activate/Deactivate hooks (grep 2026-09-25). Issue #62 |
| Mob spawning | CW | -- | spawner/npcs.rs, space_manager/spawn.rs | Every `spawnlist` row is spawned at cell startup. Seen in-client in Castle_CellBlock and Castle (2026-09-18 playtest; UAT-1) |
| Mob registration | KM | -- | -- | **Downgraded 2026-09-25** (was IM). There is no `RegisterMobBase` analog; NPCs are registered only in the cell's SpaceManager. Likely obsolete under the direct-spawn design |
| Population tracking | KM | -- | -- | **Downgraded 2026-09-25** (was IM). No CurrentPopulation or reportPopulation (grep 2026-09-25). Issue #62 |
| Mob death notification | CW | -- | ticks/npc_respawn/mod.rs, abilities/death/ | `mark_npc_dead` stamps `respawn_at`. At the 2026-09-18 playtest "every death that reached `mark_npc_dead` also armed a respawn" ([appendix-npc-ai.md](../analysis/playtests/2026-09-18-colo-castle/appendix-npc-ai.md)). Code pointer corrected 2026-09-25 (respawners.rs is player respawn points) |
| Respawn timers | CW | -- | ticks/npc_respawn/mod.rs, spawner/npcs.rs:176 | `COALESCE(spawnlist.respawn_secs, template.respawn_secs)`. The 2026-09-18 playtest had 19 deaths and 19 respawns at 120.2-120.9 s ([playtest README](../analysis/playtests/2026-09-18-colo-castle/README.md) "CONFIRMED — respawn path is healthy"). Seeded: Castle 120 s (#667), Harset 30 s. Code pointer corrected 2026-09-25 |
| Set cooldowns | KM | -- | -- | **Downgraded 2026-09-25** (was IM). No min/maxCooldownSeconds consumer. Issue #62 |
| Max active sets | KM | -- | -- | **Downgraded 2026-09-25** (was IM). `grep MaxActiveSets crates/` finds nothing. Issue #62 |
| Spawn tables (weighted) | KM | -- | -- | **Downgraded 2026-09-25** (was IM). No (id, weight) spawn-table roll; each spawnlist row names one template |
| Spawn point randomization | KM | -- | -- | **Downgraded 2026-09-25** (was IM). No bRandomizeSpawnPoints consumer; spawns use the fixed spawnlist position |
| Level range filtering | KM | -- | -- | **Downgraded 2026-09-25** (was IM). No minMOBLevel/maxMOBLevel. Level comes from `entity_templates.level` (npcs.rs:162) |
| Player detection radius | KM | -- | -- | detectionRadius is defined but not wired. NA13's `entity_templates.aggro_radius` is a per-NPC aggro gate, not this region property |
| Time-of-day spawns | KM | -- | -- | onTimeOfDayTick. Only the client `onTimeOfDay` push exists |
| Mission integration | IM | -- | space_manager/spawn.rs, spawner/templates.rs, content/executor/spawn/ | Content-engine `spawn_entity` / despawn actions create mission-scoped NPCs from template prototypes with `respawn_secs` forced to None (#662). Chain-armed spawns keep a seeded passive override (D-NA01a). Code pointer corrected 2026-09-25 (spawner/missions.rs is the mission-definition cache) |
| Linked sets | KM | -- | -- | bLinked flag, semantics unknown |
| Population scaling | NU | -- | -- | timerReduction suggests dynamic spawn rates |
| Stargate spawning | CW | -- | spawner/stargates.rs | Castle ↔ neighbor verified |
| Loot-drop integration | CW | -- | spawner/loot.rs | Castle smoke covers loot bag drop |
| Dialog NPC spawning | CW | -- | spawner/dialogs.rs | Castle Cellblock NPCs |
| Ability NPC spawning | CW | -- | spawner/abilities.rs | PR #368 three-bucket |
