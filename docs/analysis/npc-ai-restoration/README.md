# NPC AI Restoration

> Type: how-to. Audience: Claude Code coordinator and implementing engineers.
> Updated: 2026-09-24. Companions: [audit](audit.md), [work packets](work-packets.md), [telemetry plan](telemetry.md), [evidence passes](evidence/), [2026-09-18 colo playtest](../playtests/2026-09-18-colo-castle/README.md), [Castle rebuild](../castle-rebuild/README.md), [documentation index](../../readme.md).

## Purpose

This campaign restores NPC combat AI to the behaviour the owner expects from Stargate Worlds, starting in Castle_CellBlock (world 12) and Castle (world 8). It covers four reported problems:

- later mobs never aggro;
- NPCs never use cover;
- NPCs float and climb into the air;
- NPCs get stuck partway, frozen-attacking or running in place.

Telemetry comes first. The owner wants these problems debugged from live sessions rather than from static analysis and the debugger alone. Phase 0 therefore makes every symptom visible in SigNoz before any behaviour changes, and the same signals then prove each fix at UAT.

## What was found

The [audit](audit.md) holds the evidence and confidence for every row. In short:

| Symptom | Root cause | Packets |
|---|---|---|
| Later mobs don't aggro | Idle NPCs with `aggression = 0` are never ticked, and nothing seeds aggression. Only the first guard (chain 1008) and the PRU (chain 1032) are armed, both by content chains. Once armed, the proximity scan has no radius, line-of-sight or floor gate: it pulled at 60-201 m. | NA13, NA14 |
| Running in place, frozen but attacking | Stopping mid-path clears the path but not the velocity. The AoI tick then tells clients "moving at 6 u/s" every 100 ms. (The `CombatAdvance` movement type never reached the client as a type; NA10 found it went out as a truncated `onSequence` and removed it.) | NA10 |
| Stuck partway | The leash measures spawn-to-*player* at 50 u, which is right at the tutorial staging spot. The leash is a teleport that keeps the old path. Aggro and leash form a 6 s loop (120 leashes in 12 min for one guard). A fight that ends leaves the NPC Idle in place and never ticked again. Player combat state is never drained. | NA12, NA15 |
| Floating | Detour paths are 2D and we lerp Y along each leg, so the NPC climbs over flat floor toward a ramp and is then frozen at that height when it stops to shoot. Our ground query returns the storey nearest Y = 0, so the telemetry could not see it. The OnGround wire variant is **not** a fix: the client keeps the current height and does not ray-cast. | NA01, NA11 |
| No cover | The cover AI is built and wired, but its 9,346 nodes are prefab-local offsets that were never transformed to world space. Cover is also only considered when the target is out of range. | NA20, NA21, NA22 |
| (found on the way) Drone doesn't fire | The stationary PRU fails line of sight at 12-16 m every time. | NA16 |
| (found on the way) Blind telemetry | The OTLP filter drops the debug seams we already have, logs carry no host/env/SHA, and there are no stuck, float or stale-velocity detectors. | NA00, NA02, NA03 |

## Decisions

The owner answered the design questions on 2026-09-24. Rows marked PROPOSED are the coordinator's defaults and need the owner's answer before their packet starts.

| ID | Status | Decision | Reason |
|---|---|---|---|
| D-NA01 | **APPROVED** | **Proximity aggro is derived from faction hostility.** Effective aggression is a runtime or seed override, or else the faction reaction (`EMobAggressionLevel`); only HOSTILE aggroes on sight. It is gated by an aggro radius (template column, about 18 u default), line of sight, and a same-floor vertical band. | This matches the original design's faction reaction table, and every future zone gets correct defaults without per-row seeding. |
| D-NA01a | APPROVED (follows from D-NA01) | **Chain-armed mobs keep a seeded passive override** (spawns 20 and 10, plus any Castle/Harset equivalents found in the NA13 audit), so chains 1008 and 1032 still decide when they engage. | Without it the PRU would shoot the player before the vial pickup, and the first guard would engage before Region8. |
| D-NA02 | **APPROVED** | **Mobs aggro onto GMs**, except while a server-side GM toggle (`.aggro off`) is set. | The owner tests on a GM account. Client `ghost` is invisible to the server, so it cannot be the switch. |
| D-NA03 | **APPROVED** | **Leash and reset:** the leash is measured on the NPC's own distance from spawn (with hysteresis). The NPC walks home on the navmesh with the Leash movement type and evades while returning, then heals, restores its facing and clears cooldowns on arrival. A lost target also sends it home. It snaps home only if there is no path. Player combat state is drained, and re-aggro is suppressed for about 5 s after arrival. | The 2026-09-18 RE notes and the client's Leash movement type say walk-home is the client-side intent. The current snap and park is what produces the stuck and loop symptoms. |
| D-NA04 | **APPROVED** | **Same-room assist aggro:** same-faction Idle NPCs within about 10 u with line of sight to the victim join the fight, with no chaining. | A marked deviation from legacy, which had no assist. |
| D-NA05 | **APPROVED** | **Cover is faithful to the original:** real cover nodes per map, NPCs spawned in cover hold it, NPCs under fire seek it, and the client's crouch/peek pose is driven the way the client expects. | Owner choice. It needs the per-map extractor (NA21) rather than hand-authored rows. |
| D-NA06 | PROPOSED | **Telemetry lands before behaviour.** NA00-NA02 ship with no behaviour change, and one "before" UAT session is recorded. | Every fix then has a before and after in SigNoz. Cost: about one extra deploy cycle. |
| D-NA07 | PROPOSED | **Ground NPCs server-side** (per-tick storey-aware height). Keep avatar-update variant 0x10. Do not use 0x18. | Binary evidence M6. Needs no client patch. |
| D-NA08 | PROPOSED | **Aggro line of sight fails closed on `Unknown`**, while attack line of sight keeps failing open. | An off-mesh ray should not pull a mob through a wall, but a mob already fighting should not stop shooting because of a mesh hole. |
| D-NA09 | PROPOSED | Default radii: aggro 18 u, assist 10 u, leash 50 u NPC-to-spawn with 5 u of hysteresis, vertical band 4 u. Each becomes an `entity_templates` column with these defaults, tuned at UAT. | These are starting values; the originals are unrecovered. |
| D-NA10 | APPROVED (coordinator, 2026-09-25, under the owner's autonomous-run authorization) | **Correction to D-NA03 and D-NA05: there is no server-to-client movement-type message.** The walk home is shown by position and velocity alone, and the cover pose cannot come from movement type 0. The old `broadcast_movement_type` sent a truncated `onSequence` (witness method 1) and is now suppressed (NA10). 0x00deb660 is the GM `onShowPath` visualiser. | Ghidra evidence in NA10, [findings/npc-movement-pathfinding.md §11](../../reverse-engineering/findings/npc-movement-pathfinding.md). |
| D-NA11 | APPROVED (coordinator, 2026-09-25, autonomous-run authorization) | **Stationary NPCs ignore a navmesh-LoS `Blocked` within the 4 u same-floor band while Fighting.** The rule is `LineOfSight::permits_stationary_attack`, applied through `SpaceManager::attack_line_of_sight`. Mobile NPCs and aggro keep the strict navmesh verdict. The `npc_ai.tick` row reports it as `los_policy=stationary_relaxed`. **Known cost:** a stationary NPC that is already fighting can shoot through a real same-floor wall. About 40 seeded spawns are stationary, including the Harset hub mobs, the world-68 rows and the Find Ambernol drone. Retired when the collision-geometry occluder lands ([#784](https://github.com/SandboxServers/Cimmeria/issues/784)). | NA16 measured 4,000 same-floor Cellblock pairs against the extracted collision geometry at 1.5 m eyes. 45% of navmesh `Blocked` verdicts were false (269 of 601), because Recast cuts furniture out as holes with no height. The drone's S11 silence was the med-station desk. Navmesh-only heuristics were rejected on numbers ([audit S11/S15](audit.md)). |
| D-NA12 | APPROVED (owner, 2026-09-25: "do the follow up work") | **An NPC at a cover slot looks from the slot's peek point.** The peek point is the first navmesh point past the cover prop: over it along the node's facing (0.5-3.5 u out, with 1 u of clear floor ahead), else round either end of the marker (a walk of at most 6 u). An NPC standing within 1.5 u of the slot it holds sees a target when the ray from the peek point, or its own ray, is clear. One rule serves the Idle aggro scan, the assist check, the attack check (`los_policy=cover_peek`, replacing NA22's unchecked `in_cover_slot`) and the `npc_ai.tick` row. With no line from its slot the NPC holds fire and gives the slot up after 3 s, and a new pick needs a shot from the slot. The flank release moves to 20 degrees past side-on, the pick needs the threat in front of side-on, and a slot given up as flanked, blind or unreachable cools for 6 s for that NPC. D-NA11 is unchanged. | UAT-1 ([worknotes/uat-1.md](worknotes/uat-1.md)). A cover prop is a navmesh hole: the ray from behind it stopped within 0.42 u, so guards in cover never aggroed, and the exemption let `Hallway02_Guard` kill the player through two walls at 23.5 u. The mess-hall flanks were all 10-12 degrees past side-on. **Known cost:** mess-hall tables are holes too, so a guard there sees little of the room from its slot and leaves cover after 3 s; the occluder (#784) is the real fix. |
| D-NA13 | APPROVED (owner, 2026-09-25: "We have tons of disk and ram on the server I'm not worried about it. Distance packing and unpack what's near players.") | **Where a world ships `data/spaces/<world>.occ`, NPC line of sight comes from the collision geometry, eye to eye at 1.5 m, instead of the navmesh ray.** This covers the Idle aggro scan, the assist check, the attack check (`los_policy=occluder`) and the cover sight. D-NA11 (the stationary relaxation) and D-NA12 (the cover peek point) apply only in worlds without one. An endpoint outside the occluder's trimmed explorable area reads `Unknown`: aggro fails closed (D-NA08), and the attack check takes the navmesh rules. The files are paged (64 m pages, compressed), and only the pages within 132 m of a player are unpacked. All 23 client worlds ship one, 131.4 MB in total. | NA27 ([worknote](worknotes/na27-occluder-phase1.md)). Against an exact ray through the collision triangles at 0.5 m cells: 0 false clears on the Castle_CellBlock (4,000 pairs) and Castle (2,000 pairs) sweeps, and 1.24% / 1.05% of truly clear pairs called blocked, most of them rays grazing within 0.1 m of a wall edge. The navmesh ray was wrong on 38% / 49% of its `Blocked` answers, and on Castle it saw through walls on 23% of the truly blocked pairs. Phase 1 was no-go under the first budget (Agnos, 57 MB / 288 MB unpaged); trimming and paging brought Agnos to 30.5 MB on disk (on NA28's whole-map tiled mesh) and 4.0 MB of RAM with one player. |
| D-NA16 | **APPROVED (owner, 2026-09-25)** | **Broadcast the NPC aggression level to clients** with `onAggressionOverrideUpdate`/`onAggressionOverrideCleared` (SGWMob flat method indices 27/28, Ghidra-verified), on every runtime override change (content `set_aggression`, the `.aggression` console command, the surrender disarm) and replayed on AoI entry when an override is active — mirroring python `SGWMob.createOnClient`'s conditional send. A faction-derived (no override) mob still sends nothing, matching legacy exactly. | NA33 ([findings/npc-aggression-broadcast.md](../../reverse-engineering/findings/npc-aggression-broadcast.md)). Legacy `setAggression`'s literal wire call (`onEntityProperty(GENERICPROPERTY_MobAggression)`) has no client consumer (NA13); this uses the ClientMethod the client's `GameMob` handler actually reads, finishing `createOnClient`'s intent for the runtime-change path python never wired it for. No client patch. |

## Coordinator launch prompt

You are the Claude Code coordinator for the NPC AI restoration. Implement the packets in [work-packets.md](work-packets.md) as small, reviewed changes. Do not broaden into navmesh generation (the pipeline is done: #683, #694, #709), content chains beyond the aggression overrides, or zones other than 12 and 8 unless a packet says so.

1. Record `git rev-parse HEAD` and `git status --short --branch`. Check that PR #726 (navmesh logging review) has landed. If it has not, land or rebase it before NA00.
2. Before creating worktrees, check for a live peer session on this campaign (`~/.claude/sessions/*.json`, recent commits on `npcai/*` branches). If one exists, message it and stand down.
3. Read [AGENTS.md](../../../AGENTS.md), [CLAUDE.md](../../../CLAUDE.md), [TESTING.md](../../../TESTING.md), the [audit](audit.md) and the [telemetry plan](telemetry.md). Give workers only the audit rows their packet cites.
4. Get the owner's answers on the PROPOSED rows. Record each answer as a new row; do not edit existing rows.
5. Dispatch NA00, NA01 and NA20 in parallel worktrees with disjoint ownership. Integrate NA00 first; it defines the state-transition helper everyone else uses.
6. After NA02 is integrated and deployed, ask the owner for **UAT-0**: one colo session through Cellblock's first two rooms, with a `.bug` bookmark at each problem. Save the SigNoz picture in `worknotes/uat-0.md`.
7. Serialise the contended files (`fight.rs`, `npc_movement.rs`, `navigation/mod.rs`) through the coordinator, one packet integration at a time, in the shared cargo lane.
8. After each milestone (UAT-1 movement, UAT-2 aggro, UAT-3 cover), compare the same SigNoz queries against UAT-0 and record the result in the packet's status line.
9. When blocked, leave `handoffs/<packet>.md` with the exact next action.

## UAT milestones

The owner plays on the colo, as GM with `.aggro off` unset once NA13 lands, using `.bug <note>` at every oddity. The coordinator reads SigNoz afterwards with the queries in [telemetry.md §3](telemetry.md#3-live-session-runbook).

| Milestone | After | Owner checks | Telemetry must show |
|---|---|---|---|
| UAT-0 | NA00-NA02 | Play normally through the stasis room, first guard, PRU and MessHall. | The before-picture: `stale_velocity` > 0, leash loops, `cover.coverage` WARN, `no_cover` reasons, and `ground_deviation` on ramps. |
| UAT-1 | NA10, NA11, NA12 | Guards stop without running in place and stay on ramps. When you run or die they walk home and re-engage when you return. | `stale_velocity`, `ground_deviation`, `leash loop`, `idle_parked` and `cleared_without_exit` all about 0. |
| UAT-2 | NA13, NA14 | Each room's guards engage when you enter, and not through walls or floors. Both MessHall guards join. The PRU waits for the vial. | `npc_ai.aggro cause=proximity/assist` per room tag; `aggro_scan` rejects named. |
| UAT-3 | NA21, NA22 (+NA15, NA16) | Guards use room cover, and ones placed in cover stay there. The drone fires at range. | `cover.coverage` > 0 for worlds 12 and 8; `stay_in_cover` and `move_to_cover` decisions; `stuck` about 0. |

## Implementation session record (2026-09-24 to 2026-09-25)

The owner authorized an autonomous run on 2026-09-24: "autonomously work on these work packets and merge PRs when they are ready until you are done with the work packets from this plan", then post `/release` on the last PR. The PROPOSED rows D-NA06 to D-NA09 were adopted at their recommended defaults. D-NA10 and D-NA11 were added from evidence found during the run.

- **Merged:** every packet, via PRs #774 to #789 plus the close-out PR. PR #726 (navmesh logging review) was fixed and merged first.
- **Process:**
  - One isolated worktree per worker (Agent `isolation: "worktree"`), with per-worktree test databases and the shared cargo lane.
  - Squash-merge after green CI.
  - An independent `testing-validation-engineer` review on the two foundation PRs, #776 and #781.
  - A coordinator trial-merge plus test run whenever a PR's CI predated the latest `main`. This caught one semantic conflict: NA12's `SpawnRecord.leash_distance` against NA11's test fixture.
- **Not done:** UAT-0, the separate before-picture session (see D-NA06). Every in-client check is listed in [handoffs/session-resume.md](handoffs/session-resume.md).

## Where confidence is low

- How the client renders a stationary NPC that still has non-zero velocity. The code path is confirmed; the rendering is inferred. The `wire.out.avatar_update` export in NA00 settles it.
- Which movement type or stance the client wants for "stationary in combat" (NA10), and the Leash movement type value: 5 per `enumerations.xml` vs 2 per one RE note.
- Whether `SGWCoverNodeComponent` in the prefab packages carries the node array or points at the pak template (NA20), and what drives the crouch and peek pose.
- The original aggro, assist and leash radii (D-NA09 values are starting guesses).
- Floating is shown in telemetry only indirectly until NA01 fixes the ground query. The mechanism is confirmed in code and in the path data.
