# NPC Cover System

**Status:** Accepted (NA22, 2026-09-25). Implements D-NA05 of the [NPC AI restoration](../analysis/npc-ai-restoration/README.md) within the limit D-NA10 sets.

**Code:** [`crates/services/src/cell/cover/`](../../crates/services/src/cell/cover/) (decision, reservation, scoring, stance), [`npc_ai/fight_cover.rs`](../../crates/services/src/cell/service/npc_ai/fight_cover.rs) (the fight tick's cover step), [`effects/cover_stance.rs`](../../crates/services/src/cell/effects/cover_stance.rs) (the Cover Stance scripts).

**Evidence:** [findings/cover-world-placement.md](../reverse-engineering/findings/cover-world-placement.md) (where the markers are, Q4 pose, Q5 Cover Stance), [findings/cover-system.md](../reverse-engineering/findings/cover-system.md) (wire surface, scorer weights), [engine/cover-extraction.md](../engine/cover-extraction.md) (how the seed is produced), [audit rows C1-C7](../analysis/npc-ai-restoration/audit.md).

## Context

Castle and Castle_CellBlock designers placed guards at cover markers: 9 of Cellblock's 12 NID Guard spawns sit within 1.5 u of an extracted `SGWSpecCoverNode` marker, and `MessHall_Guard1` is 0.63 u from one. Before NA22 the server ignored this:

- cover was only considered when the target was out of range, so an NPC that already had a shot never took or held cover (C3);
- nothing was reserved at spawn, so a guard authored in cover did not own its slot (C4);
- `use_cover` was hard-coded `true` for every spawned NPC, props and sentries included (C5);
- the client pose contract is unknown: `USGWAnim_BlendByCover`'s one native function is a `return 1;` stub, so the pose logic is UnrealScript (C6);
- the no-cover branch logged nothing (C7, fixed by NA02).

NA21 made the positions real: the seed now holds each map's markers in world space, one index partition per `resources.worlds.world_id`.

## Decisions

### 1. Cover is a firing position

A slot is only worth holding or taking if the NPC can shoot its target from it.

- **Pick:** a candidate must be within the chosen ability's `max_range` of the target, less `PICK_RANGE_MARGIN` (2 u). The existing six-weight scorer ranks what passes.
- **Hold:** the slot is released when the target leaves the attack range (`ReleaseReason::OutOfRange`). Once the NPC stands at the slot this uses the NPC's own distance, the same one the attack uses, so an NPC in cover always has the target in range.
- The 2 u margin between pick and release is the range hysteresis: a strafing target cannot flip an NPC between taking a slot and leaving it.

**Why:** the previous rule (seek cover out of range, ignore it in range) produced NPCs that ran to cover they could not fire from, or never used cover at all. D-NA05 asks for the faithful behaviour: prefer a covered firing position inside attack range.

### 2. The seek runs in range too, with hysteresis

The cover step runs on every fight tick for a mobile NPC with `use_cover`, whether or not the target is in range (C3).

- An NPC that already has a shot only takes a short walk: `IN_RANGE_MAX_MOVE` (10 u).
- When an in-range seek finds nothing, the NPC does not look again for `SEEK_RETRY` (4 s). The deferral lives with the reservation table and is cleared on release.
- A held slot is kept until it is flanked or out of range. There is no "better slot" re-evaluation, so an NPC in cover never hops.
- The flank test keeps its 5 degree hysteresis (`FLANK_HYSTERESIS_DOT`), and the scorer now skips a candidate the threat already flanks, which would otherwise be picked and released on alternate ticks.

### 3. Spawned in cover holds it

`cover::hold_spawn_cover` reserves the nearest free node of the NPC's world within `COVER_ARRIVE_RADIUS` (1.5 u) horizontally and 2 u vertically. It runs:

- at spawn (`spawn_npc_from_record_into`);
- once for the startup population when cover finishes loading (`cover_loaded` → `hold_spawn_cover_all`), because startup spawns happen before the cover index and the world ids exist;
- at respawn, after the snap to spawn;
- when a leash walk gets home.

The held slot goes through the normal hold test when combat starts: kept while the target is in front and in range, released when flanked or out of range, then re-picked.

### 4. Arrival: stop, stance, fire from the slot

An NPC has arrived when it is within 0.5 u of the slot, or within `COVER_ARRIVE_RADIUS` with no route left. On arrival the fight tick:

- stops it (`StopReason::InCover`: empty path, zero velocity, which every witness receives on the next AoI tick);
- grants Cover Stance (decision 5);
- skips the chase block and the min-range backup, and fires.

The navmesh line of sight does not gate a shot from a slot. A cover prop is usually a hole in the navmesh, so a ray from behind it reads as blocked by construction; an NPC at a Low or Mid marker fires over the cover. This is a rule of NA16's attack line-of-sight policy: `SpaceManager::attack_los_policy` returns `AttackLosPolicy::InCoverSlot` for a mobile NPC that holds Cover Stance, and the `npc_ai.tick` row logs it as `los_policy=in_cover_slot`. The fight tick computes line of sight after the cover step, so the arrival tick already sees the stance. NA16's stationary rules are unchanged.

The walk to a slot is `chase::walk_to_cover_slot` (`npc_ai/chase/cover_slot.rs`), not NA15's target chase: a slot is a place to stand, so there is no stop-distance offset, no hold-then-walk-home at a dead end and no target snap. NA15's rules still apply to every target chase. A route that fails on a meshed world releases the slot (`ReleaseReason::Unreachable`) and defers the next seek. A space with no navmesh gets the slot as a direct waypoint.

### 5. Cover Stance through the effect-script layer

Seeded ability 1451 "Cover Stance" owns two single-shot effects. NA22 names a script on each row (`db/resources/Effects/Seed/effects.sql`):

| Effect | Seed text | `script_name` | Does |
|---|---|---|---|
| 4565 | "Single Target +100 CoverDefense" | `CoverStance` | `COVER_DEFENSE` (stat 67) cur and max += 100 |
| 1742 | "Remove Stance Moniker" | `RemoveCoverStance` | the same -= 100, floored at the stat minimum |

`cover::grant_cover_stance` runs 4565 on arrival; `cover::revoke_cover_stance` runs 1742 when the NPC leaves the slot (flanked, out of range, unreachable) and from `cover::release_npc_cover` on leash, death and surrender. Grant and revoke are idempotent through a per-NPC stance set kept with the reservations, so the buff is applied once per arrival and removed exactly once.

The magnitude is 100, from the effect row. The ability's "+200" description is not used; the difference is unexplained. A `CoverDefense` NVP on the effect row overrides it.

**Security:** the effect ids are server constants, never a client field, so this entry point does not widen the reach that [abilities-and-effects-system.md §16](abilities-and-effects-system.md#16-content-initiated-effects-use-a-separate-entry-point-not-handle_use_ability) guards.

**Limit:** `COVER_DEFENSE` is not read by hit/defence resolution yet, so the stance is visible (GM `.stats`) but does not change a fight. Wiring cover defence into the QR roll belongs to the combat pipeline.

### 6. No pose message

D-NA10: there is no server-to-client movement-type message, so the pose cannot come from movement type 0. NA22 invents no wire. The client learns about cover only from the NPC's position and zero velocity.

Whether the client crouches an NPC that stands at a marker is decided by the owner experiment in [cover-world-placement.md Q4](../reverse-engineering/findings/cover-world-placement.md#q4--what-drives-the-clients-crouchpeekfire-pose): spawn a `use_cover` NPC at the desk marker and watch the model. If it does not crouch, the pose needs a client-side claim this server does not set, and that is a separate decision.

### 7. `use_cover` from the template

`entity_templates.use_cover` (nullable boolean) is the client's `SGWMob.def` `useCover`.

| Column | Result |
|---|---|
| `true` / `false` | used as given |
| NULL | the default rule: a hostile NPC (`faction = 10`, the only faction a player can fight) takes cover |

Whatever the column says, a stationary NPC or a prop (`static_mesh`) never takes cover, and an NPC whose every known ability is melee (`is_ranged = false`) is skipped at fight time, when the ability defs are known.

Seeded values: `true` for the ranged guards (Cellblock 15 and 24; Castle 146, 148, 169, 170, 171), `false` for the two prisoner retrieval unit drones (4, stationary in Cellblock, and 145). Everything else is NULL.

### 8. Squad affinity by distance, not by set

NA21 groups markers transitively, which makes some sets very large (105 nodes across a 17 x 14 m Castle courtyard). A per-set ally count penalised the whole courtyard for one ally. The penalty now counts other NPCs holding a slot within `SQUAD_AFFINITY_RADIUS` (2 u) of the candidate. Reservation was already per slot.

## Telemetry

| Row | When |
|---|---|
| `npc_ai decision_outcome=move_to_cover` (INFO) | a slot was picked; carries `in_range`, `arrived` |
| `npc_ai decision_outcome=stay_in_cover` (DEBUG) | holding a slot; the terminal outcome while walking to it |
| `npc_ai decision_outcome=cover_released_flanked` / `_out_of_range` / `_unreachable` / `_stale` (INFO) | a slot was given up |
| `npc_ai decision_outcome=no_cover` (DEBUG, sampled) | `reason` adds `seek_cooldown`; `out_of_reach` counts candidates rejected by range, walk or flank |
| `npc_ai decision_outcome=attack_in_place` | carries `in_cover` |
| `cover.hold event=spawn_reserved` (INFO) / `event=startup_summary` / `event=released` (DEBUG) | the spawn hold and every release through `release_npc_cover` |
| `cover.stance event=granted` / `revoked` (DEBUG); `event=effect_missing` (WARN) | Cover Stance; the WARN means the seed lost the effect row or its `script_name` |
| `movement.npc event=stop reason=in_cover` | the arrival stop |
| `npc_ai.tick los_policy=in_cover_slot` | an NPC at its slot, whose shot the navmesh verdict does not gate |
| `npc_ai.path_fail reason=partial decision_outcome=cover_partial` | a partial route to a slot |

## Consequences

- Guards authored in cover stay there and shoot; guards in the open walk up to 10 u to cover that reaches their target.
- An NPC holding cover does not chase. A target that retreats past attack range releases the slot and the NPC advances normally.
- The stance has no combat effect until cover defence is wired into the hit roll.
- Worlds without extracted cover rows (everything except 12 and 8 today) behave as before: no candidates, `no_cover`.

## Open questions

1. Does standing at a marker make the client crouch (Q4)? Owner experiment.
2. Should `COVER_DEFENSE` feed the QR defence roll, and with which of the two magnitudes (+100 effect, +200 ability text)? Combat pipeline.
3. `CoverNode.width` is loaded and unused. A wide marker could host more than one NPC.
