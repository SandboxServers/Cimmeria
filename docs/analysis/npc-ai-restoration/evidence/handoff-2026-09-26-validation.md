# External AI Handoff Validation (2026-09-26)

> Type: reference. Audience: the owner and the next coordinator session.
> Updated: 2026-09-26 (NA44). Companions: [campaign README](../README.md), [work packets](../work-packets.md#phase-6-external-handoff-follow-ups-2026-09-26), [session resume](../handoffs/session-resume.md), [telemetry plan](../telemetry.md).

## What this is

On 2026-09-26 an external document, "Cimmeria / Stargate Worlds — Server-Side AI Reconstruction Handoff", proposed a 32-section plan for NPC AI. It was written before the NA00-NA38 campaign. Five read-only auditors checked every claim against `origin/main` at 27f4e565. Their areas were aggro, locomotion, follow and leash, attack, and observability. This page condenses their reports into one verdict per claim.

Verdicts:

- **ALREADY-FIXED:** `main` already does what the handoff asks, usually because of an NA packet.
- **WRONG-CLAIM:** the handoff's premise is false. Do not implement it.
- **OPEN:** a real gap. The **Packet** column says who takes it: NA41-NA44 run in parallel from this validation, and "owner" means the item needs an owner decision first (see [Owner decisions](#owner-decisions)).
- **NEEDS-UAT:** implemented, but only a real client can confirm it.

Three premises run through the whole handoff, and all three are out of date:

- **"Castle has no navmesh."** `data/spaces/castle.nav` shipped in #709. NA26 (#794) and NA28 (#796) gave every world a mesh, and NA27 (#797) gave every world an occluder.
- **"The client picks NPC locomotion from a server-sent movement type."** No server-to-client movement-type message exists. `setMovementType` is a client-to-server cell method. The client animates NPC gait from the `EntityMoved` velocity, and `0x00deb660` is the GM `onShowPath` visualiser (NA10 #779, [findings §11](../../../reverse-engineering/findings/npc-movement-pathfinding.md#11-correction-2026-09-25-the-client-has-no-movement-type-receiver)). Two agent-memory notes still said otherwise and were probably the source; NA44 annotated both.
- **"No proximity aggro; the leash measures the player."** NA13 (#787) added faction-derived proximity aggro, and NA12 (#785) measures the leash on the NPC.

A side finding: the shared `:5433` `sgw` database is older than `main` (no `aggro_radius`, no ability sets 4 or 5). The auditors read seed facts from the `db/resources` files instead.

## Section verdicts

| § | Claim | Verdict | Evidence | Packet |
|---|---|---|---|---|
| 0 | Evidence and source policy | Holds | Matches the project's evidence classes and [rules-and-gotchas](../../../agents/rules-and-gotchas.md). | none |
| 1 | Keep the server-authoritative state machine; no behaviour trees or ML; keep the 12 states | ALREADY-FIXED | `AiState` has 12 variants; `npc_ai/dispatch.rs` matches each state explicitly. | none |
| 2 | A reproducible real-client CellBlock encounter | OPEN (MED) | Unit guards run on the real Cellblock mesh (`aggro_castle.rs`, `path_robustness.rs`, `step_back.rs`); the owner checklist is in the session resume. NA37 (#816) drives two wire clients for visibility only; no wireclient AI encounter exists. | owner |
| 2 | Per-NPC identity: ids, name, tag, faction, aggression, AI state, position, home, threat, LoS, navmesh, path, facing | ALREADY-FIXED | `.bug` → `playtest.bookmark.entity` (`console/bookmark.rs`) and `npc_ai.tick` (`npc_ai/dispatch.rs`) carry all of them. | none |
| 2 | Ability set, equipped weapon, current target | OPEN → fixed | Absent from the spawn row and the bookmark row. NPCs never set `weapon_visual`: their weapon is a template `components` entry. | NA44 |
| 2 | Current movement type | WRONG-CLAIM | No such wire message (NA10). `last_movement_type` is already on the bookmark row as server bookkeeping. | none |
| 2 | Sequence emitted per attack | OPEN | `abilities.sequence` rows carry `sequence_id` and `event_set_id`, but no witness count, and a missing `Ability_End` logs at untargeted DEBUG. | NA43 |
| 2 | A GM command that dumps one NPC's AI state | OPEN (MED) | `.info` has no AI state, threat, path or target; `.bug` writes to SigNoz, not chat. `.aiinfo` could reuse `bookmark::capture`. | owner |
| 3a | Ordinary NPCs spawn at aggression 0, so nothing aggroes on sight | ALREADY-FIXED | NA13 (#787): effective level is the override, else `FACTION_REACTION_TABLE[3][faction]`; faction 10 is HOSTILE. | none |
| 3b | "aggression > 0" reads NEUTRAL/FRIENDLY as aggressive | ALREADY-FIXED | `MobAggression::is_hostile()` is HOSTILE only. | none |
| 3c | SUSPICIOUS must be source-backed or fenced | ALREADY-FIXED | Treated as not aggressive ([npc-ai.md](../../../gameplay/npc-ai.md)). Python used `< NEUTRAL` only for the player's attack check. | none |
| 3d | A DEFAULT (5) override should fall through to the faction default | OPEN (LOW) | Rust treats 5 as not hostile, matching Python, which stored the override verbatim. No seed or chain uses 5. | owner |
| 3e | Baseline aggression data-driven; `set_aggression` keeps working | ALREADY-FIXED | Faction column, `spawnlist.aggression_override`, `entity_templates.aggro_radius`; spawns 10 and 20 are chain-armed. | none |
| 4 | Perception from the spatial set, with a sight radius smaller than AoI | ALREADY-FIXED | `idle_aggro.rs` scans `get_witnesses_of(npc)`; gates in `aggro_gates.rs` (18 u radius, 4 u band, fail-closed LoS, GM switch). Hearing is not built, which the handoff allows. | none |
| 5 | Bounded, non-chaining assist aggro | ALREADY-FIXED | NA14 (#789, D-NA04): same faction, HOSTILE, Idle/Patrol/Wander, same room; assist does not recruit. Every seeded `set_name` is NULL, so faction plus radius is the only grouping. | none |
| 6 | Translation, locomotion and attack animation are separate channels | ALREADY-FIXED | NA02 detectors (`stale_velocity`); `wire.out.avatar_update` carries `npc_moved_since_last`. Locomotion is velocity. | none |
| 7a | Send `setMovementType` at leg start | WRONG-CLAIM | See the premises above. | none |
| 7b | `broadcast_movement_type(None)` leaves a stale animation | WRONG-CLAIM | Stopping is zero velocity (`stop_npc_movement`, final-waypoint arrival, attack in place); guarded by `stop_hygiene` tests. Stale code comments remained: the entity crate (NA44) and `follow.rs` / `investigate.rs` (NA41's files). | NA44, NA41 |
| 8a | Only FullPos (`0x10`) is sent | Intended | D-NA07 keeps `0x10`. | none |
| 8b | OnGround (`0x18`) makes the client ray-cast the ground | WRONG-CLAIM | `FUN_00ddb830` writes Y = -13000.0 and the client substitutes the actor's current height; no ray-cast. `0x18` would pin an NPC at its creation height. | none |
| 8c | Keep server Y on the floor | ALREADY-FIXED, NEEDS-UAT | NA11 (#783) storey-aware per-step clamp. | UAT |
| 9a | Chase: no path must not walk through walls | ALREADY-FIXED | NA15 (#788): `None` enqueues nothing; partial route holds then goes home. | none |
| 9b | Follow, patrol, wander, investigate: no raw straight-line fallback | OPEN (HIGH) | Each pushes the raw destination (`PathFallback::DirectWaypoint`) on any routing failure, meshed world or not, and the movement tick has no horizontal containment. | NA41 |
| 9c | Leash no path | ALREADY-FIXED | NA12/NA15: route only; snap on no route, partial route or 20 s. | none |
| 9d | No path: stop and face the target | OPEN (HIGH) | The chase `None` and degenerate branches never call `face_target`. | NA41 |
| 9e | Step-back fallback uses the raw point off-mesh | OPEN (MED) | `move_along_navmesh(...).unwrap_or(raw)` in the step-back. | NA41 |
| 10 | Cellblock's mesh has disconnected components; Marsh's area and the combat area differ | ALREADY-FIXED | 17 components, documented in `data/spaces/README.md`; `gc1_escort.rs` checks the ring-3 reposition. | none |
| 10 | Connectivity to the later areas is checked | OPEN (test only) | Tests prove Ring 3 → MessHall only; Hallway05 (where mission 686 completes) is never reached, and `chain_1174` asserts only `len() > 1`. | NA42 |
| 10 | Trace how the original mission moved Marsh | WRONG-CLAIM | Legacy scripts only change Marsh's interaction flags; the escort is new authored content (D-CB13, GC1). | none |
| 10 | Move him through the ring hand-off | ALREADY-FIXED | Chain 1173 `move_waypoint`, then 1174 `set_follow_target`. | none |
| 10 | Relog mid-escort | OPEN (MED) | The instance respawns Marsh in the Preparation room with no follow, and no `player_loaded` restore chain exists. Seed-only fix. | owner |
| 11a | Range-aware chase goal from weapon min/max | OPEN (MED) | `stop_distance = max(min_range, 1.0)`: a ranged NPC routes to 1 u and stops only when the 2 s tick sees it in range. NA15 chose this on purpose. | owner |
| 11b | Evaluate LoS, stationary flag, cover | ALREADY-FIXED, NEEDS-UAT | NA16/NA23/NA27 line of sight; stationary hold; NA22 cover firing position. | UAT |
| 11c | Melee closes to melee range | ALREADY-FIXED | `NPC_MELEE_RANGE` 3.0; `melee_reach` tests. | none |
| 12a | Facing independent of the nav path | ALREADY-FIXED except 9d | `fight::face_target` in stationary hold, attack in place, step-back hold, cover-blind hold and `hold_unreachable`. | NA41 (9d) |
| 12b | `pack_angle` wrap | ALREADY-FIXED | #677: `rem_euclid`, round; test `pack_angle_wraps_negative_yaw_like_the_cpp_cast`. | none |
| 12c | Believable facing cadence while strafing | NEEDS-UAT | Yaw updates on the 2 s AI tick; legacy `lookAt` ran once per action. A 100 ms refresh is MED and only if UAT shows lag. | UAT |
| 13 | Template 24 → set 3 → 559 → event set 15 → `Ability_End` sequence 15 | ALREADY-FIXED | The whole chain is intact in the seed and the code path. | none |
| 13 | Negative logs on the attack path | OPEN (HIGH) | NULL event set logs nothing; missing `Ability_End` logs at untargeted DEBUG; no witness count. | NA43 |
| 13 | New: Castle hostiles fire Pistol Shot 592 while holding an SMG | OPEN (HIGH) | Templates 146, 148, 169, 170, 171 (SMG) and 145 (drone) have no ability set, so they fall back to 592. Same defect class as Harset H-B8. | NA43 |
| 13 | New: SGC Jaffa 34/35 fire 592 while holding a staff | OPEN (MED) | Set 4 brings in 710, which deals 0 damage. | owner |
| 13 | New: a player's attack `onSequence` reaches only the player | OPEN (HIGH) | `send_entity_method` routes a player to self; Python `playSequence` also sent to witnesses. | NA43 |
| 13 | New: effect hit and pulse sequences are never sent | OPEN (MED) | Python played Pulse_Begin, Hit and Pulse_End per effect pulse; Rust sends none of events 2000-2008. | owner |
| 13 | New: an NPC's cooldown `onTimerUpdate` goes to every witness | OPEN (LOW) | Python sent it only when the entity had a client. Extra traffic, decodes correctly. | deferred |
| 14 | Audit NPC ability sets | ALREADY-FIXED for mappings, OPEN for coverage | Every spawned NPC ability has an event set and an `Ability_End`. The only linter covers Harset sets 4 and 5. A global seed linter with a weapon-binding check is the guard for the Castle fix. 710/711/712 deal 0 damage. | NA43; owner (damage) |
| 15 | Damage vs animation timing | ALREADY-FIXED for NPCs | Every NPC ability has warmup 0. | none |
| 15 | Warmup deferral | OPEN (MED) | Rust sends Begin and End back to back and resolves at once; its cooldown excludes warmup. Affects player abilities only. | owner |
| 15 | `ability-system.md` says warmup, interruption and facing are DONE | WRONG (doc) → fixed | That described the Python. Now marked not implemented in Rust. | NA44 |
| 16 | Leash measures target to home | WRONG-CLAIM (stale) | NA12 measures NPC to spawn, 5 u band, 20 u vertical cap. | none |
| 16 | Stop attacking, clear threat, release cover, walk home, heal, go Idle, allow re-acquire | ALREADY-FIXED | `leash/begin.rs`, `leash/mod.rs::arrive`, 5 s re-aggro suppression. | none |
| 16 | Play leash locomotion | NEEDS-UAT | No movement-type wire; the gait comes from velocity. | UAT |
| 16 | Never snap a witnessed NPC | PARTIAL | Snaps only on no route, partial route or 20 s timeout, under owner-approved D-NA03. | none |
| 16 | Side finding: `npc-ai.md` speed row claims a per-state movement-type broadcast and a hardcoded speed | WRONG (doc) → fixed | `entity_templates.move_speed` exists (Marsh 0.9). | NA44 |
| 17 | `follow_target_id` set and cleared only by the mission | PARTIAL, acceptable | Also cleared when the target entity is gone (WARN `follow_target_lost`); the GM console can set it. | none |
| 17 | A path failure keeps the target | ALREADY-FIXED | `follow.rs` never clears it. | none |
| 17 | A path failure must not walk through geometry | OPEN (HIGH) | Same as 9b. | NA41 |
| 17 | Combat preemption keeps the target | ALREADY-FIXED | `follow_preempted_by_threat_clears_nav_keeps_target`. | none |
| 17 | The escort returns to Follow after combat | OPEN (HIGH) | The leash resets a follower in place to Idle, and Idle never promotes to Follow. Chain 1302 re-arms Zuritska; Marsh has no equivalent. | NA42 |
| 17 | New: a `being` pulled into Fighting freezes | OPEN (HIGH) | `ai_driven_npc_entity_ids` never admits a being in Fighting or Leashing. No being has an ability set. | NA42 |
| 18 | Patrol/Wander resume after combat | ALREADY-FIXED | Leash → Idle → the next Idle tick promotes to Patrol or Wander; route state persists. | none |
| 18 | Follow resumes after combat | OPEN (HIGH) | Same as §17. | NA42 |
| 19 | Encounter cooperation without player groups | ALREADY-FIXED | Same as §5. | none |
| 20 | Source and target alive, cooldown, max range, LoS gates | ALREADY-FIXED | `handle.rs` and `fight.rs` gates; `attack_line_of_sight`. | none |
| 20 | Attack LoS must fail safe | OPEN | Unknown fails open for attacks by decision D-NA08 (aggro fails closed). Every combat world ships a `.nav` and an `.occ`, so it is rare. | owner |
| 20 | Min range inside `handle_use_ability` | OPEN (LOW) | AI-only today; no seeded NPC ability has `min_range > 0`. | deferred |
| 21 | Cover after the basic combat loop | ALREADY-FIXED, NEEDS-UAT | NA20-NA23 and NA32: world-space nodes, hold and seek, peek-point sight, Cover Stance, damage reduction. Whether the client crouches is the open Q4 experiment. Not separately audited. | UAT |
| 22 | Castle has no working navmesh | WRONG-CLAIM | `castle.nav` and `castle.occ` on `main` since #709; seeded `advisory`, which relaxes only player containment. | none |
| 22 | No autonomous chase in Castle | WRONG-CLAIM for chase, OPEN for the other movers | Chase is routed with no raw fallback. Follow, patrol, wander and investigate are §9b. | NA41 |
| 23 | Harset mesh fragmented; validate projection, components, path | ALREADY-FIXED (mostly) | NA26/NA28 rebuilt and tiled; NA29 made five world-57 spawns mobile; NA36 decoded mesh actors. Open data work: five points 8-10 m up and spawn 308. | deferred (data) |
| 23 | Stale seed comments (`worlds.sql` Harset advisory; `spawnlist.sql` "world 68 has no navmesh") | WRONG (comment) → fixed | 23 rows are advisory; `harset_cmdcenter.nav` exists. | NA44 |
| 23 | Static and talk NPCs stay static | ALREADY-FIXED | All world-68 rows are stationary and non-hostile. | none |
| 24 | AI cadence and staggering | OPEN (MED) | Every decision runs on the 2 s tick with no stagger. Staggering by `npc_id % 20` keeps the period; a 1 s cadence changes feel. | owner |
| 24 | Stale retry-sweep comment | WRONG (comment) → fixed | The sweep is O(pending), not an `all_npc_entity_ids()` walk (`message_loop.rs`). | NA44 |
| 25 | Spawn seam | OPEN → fixed | `spawner.npc_behaviour` lacked `world`, `space_id`, ability set, event sets and weapon. | NA44 |
| 25 | Perception, movement and leash seams | ALREADY-FIXED | `npc_ai.aggro_scan` rejection reasons; `movement.npc` steps; `npc_ai.leash` events. | none |
| 25 | Threat seam: per-add and removal rows | OPEN (MED) | `generate_threat` has only a TRACE span; no add or removal-reason row. | owner |
| 25 | State seam: "previous behaviour state" | PARTIAL | Nothing to log until Follow resumes (§17). | NA42 |
| 25 | Navigation seam: component and leg ids | OPEN (LOW) | `npc_ai.path` has status and snap deltas but no mesh component or leg id. | deferred |
| 25 | Ability seam | OPEN (HIGH) | Witness count, a stable rejection `reason`, WARN on a missing event set or `Ability_End`. | NA43 |
| 25 | Follow seam: retry and restoration rows | OPEN | Restoration does not exist yet. | NA42 |
| 25 | Per-NPC trace (`CIMMERIA_TRACE_NPC_ID`) | OPEN (MED) | No such switch. The runbook's "Timeline for one NPC" view is the partial equivalent, but the throttled seams still hide rows. | owner |
| 26 | Automated regression tests | See [§26 tests](#26-tests) | | |
| 27 | Real-client acceptance, Cellblock | NEEDS-UAT | Lines added to the session resume's owner checklist. | UAT |
| 28 | Real-client acceptance, Marsh | NEEDS-UAT, blocked | "Survives one failed path query" needs NA41; "re-follows after combat" needs NA42; relog is an owner decision. Scripted removal already works (chains 1161, 1175, 1162). | NA41, NA42, owner |
| 29 | Real-client acceptance, Castle | NEEDS-UAT | The "no navmesh" premise is wrong, but the lines belong in the checklist. | UAT |
| 30 | Non-goals | Holds | Nothing in the campaign conflicts. | none |
| 31 | Implementation order AI-01 to AI-10 | Superseded | NA00-NA38 already delivered AI-01 to AI-09 in a different order; AI-10 is the UAT pass. | none |

## §26 tests

| # | Test | Status | Packet |
|---|---|---|---|
| 1 | Hostile idle NPC acquires without damage | Covered (`aggression.rs::faction_10_npc_aggroes_without_any_override`, `aggro_castle.rs`) | none |
| 2-3 | Neutral and friendly do not aggro | Covered (`friendly_and_neutral_factions_stay_idle`, `levels_two_to_five_do_not_aggro`) | none |
| 4 | Damage generates threat | Covered (`generate_threat_transitions_idle_to_fighting`) | none |
| 5 | Nearby ally assists | Covered (`assist.rs`, `assist_castle.rs`) | none |
| 6 | Dead target removed from threat | Covered (`dead_player_drop.rs`) | none |
| 7 | Dead NPC gets no AI turn | Covered (`zero_health_guard.rs`) | none |
| 8 | Ranged NPC stops inside its useful range | Missing; depends on the §11a decision | owner |
| 9 | Melee closes to melee range | Covered (`melee_reach.rs`) | none |
| 10 | No path, no through-wall translation | Chase and leash covered; follow, patrol, wander, investigate missing | NA41 |
| 11 | Stationary NPC never paths | Covered | none |
| 12 | Stationary fighter faces a moving target | Single-tick only; no moving-target or chase no-path case | NA41 |
| 13 | Leash measures NPC to home | Covered (`leash_is_measured_on_the_npc_not_the_player`) | none |
| 14 | Leash returns home and restores Idle | Covered (`leash_walk.rs`) | none |
| 15 | Follow path failure keeps the follow target | Missing (the log is asserted, the target is not) | NA41 |
| 16 | Follow resumes after combat | Missing; existing tests pin the opposite | NA42 |
| 17 | Path start sends the correct movement type | WRONG-CLAIM; replaced by the `stop_hygiene` zero-velocity guards | none |
| 18 | A NULL-event-set ability cannot reach production | Harset sets only | NA43 |
| 19 | Ability 559 resolves `Ability_End` | Missing | NA43 |
| 20 | NPC attack `onSequence` reaches witnesses | Missing | NA43 |
| 21-22 | No damage after death; no attacking at zero health | Covered (`zero_health_guard.rs`, `bleed_death_tests.rs`) | none |

## §32 definition of done

| Item | Status |
|---|---|
| Auto-aggro; neutral and friendly passive | Implemented (NA13, NA14, NA23). NEEDS-UAT. |
| Locomotion without sliding | Implemented (NA10). NEEDS-UAT. |
| Grounded | Implemented (NA11). NEEDS-UAT. |
| Faces what it attacks | Implemented, except the chase no-path branches (NA41). NEEDS-UAT. |
| Visibly fires with damage | Code sends the sequences; the fan-out guard and negative logs are NA43; Castle hostiles fire the wrong ability (NA43). NEEDS-UAT. |
| Ranged stops at range, melee closes | Melee done. Ranged is an owner decision (§11a). |
| No walking through geometry on path failure | Chase and leash done; the other movers are NA41. |
| Leash | Implemented (NA12). NEEDS-UAT. |
| Death stops AI | Implemented and guarded. NEEDS-UAT. |
| Marsh follows reliably | NA24 ticks the follow; NA41 and NA42 close the fallback and the resume; relog is an owner decision. NEEDS-UAT. |
| Castle has no invalid navigation | The premise is wrong (Castle has a mesh); the other movers are NA41. NEEDS-UAT. |
| Telemetry explains every decision | Mostly there. NA44 adds identity and weapon; NA43 adds the ability seam; threat rows and a per-NPC trace are owner decisions. |

## Owner decisions

None of these is implemented. Each needs an owner answer before a packet can take it.

| Item | Why it needs a decision | Confidence |
|---|---|---|
| Range-aware ranged chase goal (§11a) | Reverses NA15's `stop_distance` choice. A ranged NPC would stop near `max_range - 2` with a clear line instead of closing to 1 u. | MED |
| AI tick stagger, or a 1 s cadence (§24) | Staggering by `npc_id % 20` is behaviour-neutral apart from phase, but the tests expect a full pass per call. A 1 s cadence changes how combat feels. | MED |
| DEFAULT-override fall-through (§3d) | Python kept a level-5 override verbatim; falling through to the faction reaction would be a reconstruction. No live seed uses 5. | LOW |
| Marsh relog restore chains (§10) | Seed-only: two `player_loaded Castle_CellBlock` chains that move Marsh to the pad and re-arm the follow. The seed comment above chain 1171 already asks for a coordinator call. | MED |
| Jaffa 34/35 ability sets and 710 damage (§13, §14) | Set 4 brings in 710's swing, which deals 0 damage; 711 and 712 have the same shape. Any damage value is a reconstruction. | MED |
| Effect hit sequences (§13) | Server-only and no new opcode, but a new client-visible behaviour (hit reactions) that needs UAT. | MED |
| Warmup deferral (§15) | Changes player ability timing and cooldowns to match Python. | MED |
| D-NA08 fail-open attack LoS (§20) | Reverses an adopted decision: attacks would fail closed on an Unknown line. | owner call |
| `CIMMERIA_TRACE_NPC_IDS` / `.tracenpc` (§25) | A new env var and console command that bypass the per-NPC throttles. | MED |
| `.aiinfo` (§2) | A new GM command surface the owner may want to name. | HIGH to build |
| Threat add and remove rows (§25) | New DEBUG rows on a hot path; `threat` is exported at INFO today, so it also needs a filter change. | MED-HIGH |
| Wireclient Cellblock AI encounter (§2) | A new end-to-end scenario (approach, aggro, attack, leash) on the wireclient harness. | MED |

Also noted for the owner, not in the handoff: the being-class NPC admission in Fighting is taken by NA42 as "beings never enter combat", which is the safe reading of the code's own TODO.

## Where the audit itself was corrected

NA44 re-verified the claims it implemented before changing them:

- **Weapon on the spawn and bookmark rows.** The observability auditor said the NPC entity "has `weapon_visual`". The field exists, but only players set it; for an NPC it is always `None`. The new rows fall back to the first `WP` template component (`WP-Human.WP_SMG_1A`), which is where an NPC's weapon lives.
- **The observability ADR.** Its `movement.movement_type` row still said the client picks mob animation from the movement-type byte and listed an outcome `sent`. The code logs `suppressed`. The auditors did not flag it; NA44 corrected it with the other stale claims.
- **Ability definitions at startup.** The startup spaces (Castle among them) spawned their NPCs before `CellService::start` loaded the ability definitions, so the new `event_set_ids` field would have read 0 for every startup NPC. NA44 moved the ability-definition load ahead of the startup spawn. Nothing in the spawn path read those definitions before, so no behaviour changes.
- **`current_target_id` on NPCs.** It is the player-side selected target and reads `0` on NPCs, which aim at `threat_top_id`. The bookmark row carries both.
