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

## Where confidence is low

- How the client renders a stationary NPC that still has non-zero velocity. The code path is confirmed; the rendering is inferred. The `wire.out.avatar_update` export in NA00 settles it.
- Which movement type or stance the client wants for "stationary in combat" (NA10), and the Leash movement type value: 5 per `enumerations.xml` vs 2 per one RE note.
- Whether `SGWCoverNodeComponent` in the prefab packages carries the node array or points at the pak template (NA20), and what drives the crouch and peek pose.
- The original aggro, assist and leash radii (D-NA09 values are starting guesses).
- Floating is shown in telemetry only indirectly until NA01 fixes the ground query. The mechanism is confirmed in code and in the path data.
