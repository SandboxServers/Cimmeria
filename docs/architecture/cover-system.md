# NPC Cover System

**Status:** Accepted (NA22, 2026-09-25; amended by NA23, 2026-09-25, decision D-NA12). Implements D-NA05 of the [NPC AI restoration](../analysis/npc-ai-restoration/README.md) within the limit D-NA10 sets.

**Code:** [`crates/services/src/cell/cover/`](../../crates/services/src/cell/cover/) (decision, reservation, scoring, stance, peek point), [`space_manager/cover_sight.rs`](../../crates/services/src/cell/space_manager/cover_sight.rs) (an NPC's line of sight from its slot), [`npc_ai/fight_cover.rs`](../../crates/services/src/cell/service/npc_ai/fight_cover.rs) (the fight tick's cover step), [`effects/cover_stance.rs`](../../crates/services/src/cell/effects/cover_stance.rs) (the Cover Stance scripts).

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
- The flank test is a hysteresis band (NA23). A held slot is released as flanked only once the threat is 20 degrees past side-on (`FLANK_RELEASE_DOT`, normalised dot below -0.342). A free slot is picked only with the threat in front of side-on (`FLANK_PICK_DOT`, dot at least 0). NA22's 5 degree band flipped on a strafe in the tight Cellblock mess hall: UAT-1's three `cover_released_flanked` rows were all 10-12 degrees past side-on.
- A slot given up as flanked, blind (decision 4) or unreachable cannot be picked again by the same NPC for `COVER_REPICK_COOLDOWN` (6 s). UAT-1's NPC 100160 re-picked the slot it had been flanked out of 6 s later.
- A pick must give the NPC a shot at its target from the slot (`SpaceManager::slot_has_shot`, decision 9); candidates that fail are counted as `no_shot` on the `no_cover` row.

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

A shot from a slot is checked from the slot's peek point (decision 9), strictly: `SpaceManager::attack_los_policy` returns `AttackLosPolicy::CoverPeek`, logged as `los_policy=cover_peek`. The fight tick computes line of sight after the cover step, so the arrival tick already looks from the slot. NA16's stationary rules are unchanged.

With no line from its slot the NPC holds fire (`decision_outcome=cover_no_shot`), faces its target and keeps the slot for `COVER_BLIND_GRACE` (3 s), then gives it up (`cover_released_no_shot`) and fights as a mobile NPC out of cover.

*Superseded (NA22):* the navmesh line of sight did not gate a shot from a slot at all (`AttackLosPolicy::InCoverSlot`, `los_policy=in_cover_slot`), on the reasoning that the prop is a navmesh hole. UAT-1 found every `in_cover_slot` row read `los=blocked`, and `Hallway02_Guard` shot the player through two walls at 23.5 u.

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

### 9. The peek point (NA23, D-NA12)

An NPC at a cover slot looks from the slot's **peek point**: the first navmesh point where its shot clears the prop. `cover::find_peek` searches from the node:

1. **Over the prop**, along the node's facing (toward the defended side): 0.5 to 3.5 u out in 0.25 u steps. A sample counts when a polygon lies within 0.3 u of it and a navmesh ray along the facing runs at least 1 u from it before hitting anything. Every seeded height (`Low`, `Mid`, `High`) is fired over; a `Los` marker (none are seeded) only peeks round.
2. **Round either end** of the marker: 0.6 u past `width / 2`, level with the node and then 0.5 u forward. The same on-mesh and clearance tests apply, and the NPC must be able to walk there in at most 6 u.

An NPC standing within `COVER_ARRIVE_RADIUS` of the slot it holds sees a target when the ray from the peek point or its own ray is clear (`cover::sight_from_slot`, `SpaceManager::npc_line_of_sight`). Neither adds a shot through a wall: the peek point is past the prop, and a clear ray from the NPC crosses nothing. A slot with no peek point sees only what the NPC's own ray sees.

One rule serves every check an NPC makes: the Idle aggro scan and the assist check (`npc_ai::aggro_gates::same_room`), the attack check (decision 4), the pick's shot check (decision 2) and the `npc_ai.tick` row. The slot is reserved from spawn (decision 3), so an Idle guard authored in cover looks from its peek point too; before NA23 its own prop blocked every ray and it never aggroed (UAT-1: `Hallway01_Guard` rejected the player `no_los` at 8.1 u, its ray stopping 0.34-0.42 u out).

**Measured** on `castle_cellblock.nav` and the world-12 seed:

| Guard | Peek point |
|---|---|
| `Hallway01_Guard` | 1.08 u over its counter (1200037/3; the walk round the counter is 9.4 u, which is why an over-the-prop peek is not walked) |
| `Hallway02_Guard` | 1.30 u over its marker (1200034/0) |
| `MessHall_Guard2` | 3.36 u over its mess table (1200053/0) |
| `MessHall_Guard1` | 3.46 u over its mess table (1200046/0) |

Of the 236 markers, 150 peek over their prop (median 1.6 u), 17 round it and 69 not at all. Every over-the-prop peek has a full navmesh route from behind its marker, so none lands on another mesh island.

**Why not the stationary rule?** D-NA11 lets a stationary NPC fire across any same-floor `Blocked`, which would have let the hallway guard keep shooting through walls. The peek point removes only the one obstacle the NPC is known to be behind.

**Known cost.** The mess-hall tables are navmesh holes too, so from its slot a mess-hall guard sees little of the room. It holds fire from cover for 3 s, then leaves the slot and closes in. The collision-geometry occluder is the fix for furniture; see decision 10.

### 10. The occluder replaces the peek point (NA27, D-NA13)

In a world that ships `data/spaces/<world>.occ`, an NPC at a cover slot
looks from its own eyes (1.5 m) over the prop, and the peek point is not
used. `SpaceManager::npc_line_of_sight`, `npc_sight_origin` and
`slot_has_shot` all take the occluder first. The cover prop is solid
geometry below eye height, so it no longer blocks the NPC hiding behind
it, and every wall past it still blocks. The shot is `los_policy=occluder`,
not `cover_peek`. Decision 9 is the rule only for a world with no
occluder. On Castle_CellBlock, `Hallway01_Guard` sees Lomiada at 13.6 u
over its counter, a spot the peek point still read as blocked.
`Hallway02_Guard` stays blind through the hallway walls. The mess-hall
tables are geometry too, so a guard there sees the room from its slot.

## Telemetry

| Row | When |
|---|---|
| `npc_ai decision_outcome=move_to_cover` (INFO) | a slot was picked; carries `in_range`, `arrived` |
| `npc_ai decision_outcome=stay_in_cover` (DEBUG) | holding a slot; the terminal outcome while walking to it |
| `npc_ai decision_outcome=cover_released_flanked` / `_out_of_range` / `_unreachable` / `_no_shot` / `_stale` (INFO) | a slot was given up |
| `npc_ai decision_outcome=no_cover` (DEBUG, sampled) | `reason` adds `seek_cooldown`; `out_of_reach` counts candidates rejected by range, walk or flank |
| `npc_ai decision_outcome=attack_in_place` | carries `in_cover` |
| `cover.hold event=spawn_reserved` (INFO) / `event=startup_summary` / `event=released` (DEBUG) | the spawn hold and every release through `release_npc_cover` |
| `cover.stance event=granted` / `revoked` (DEBUG); `event=effect_missing` (WARN) | Cover Stance; the WARN means the seed lost the effect row or its `script_name` |
| `movement.npc event=stop reason=in_cover` | the arrival stop |
| `npc_ai.tick los_policy=cover_peek` | an NPC at its slot, whose line was checked from the slot's peek point (NA23); `in_cover_slot` on builds before NA23 |
| `npc_ai decision_outcome=cover_no_shot` (DEBUG) | at its slot with no line from the peek point, holding fire; carries `blind_ms` |
| `npc_ai.los origin=cover_peek` / `cover_no_peek` | a non-clear ray from an NPC at a slot, with the peek point as `from_xyz` (worlds with no occluder) |
| `npc_ai.tick los_policy=occluder`, `npc_ai.los source=occluder` | the verdict came from the world's occluder, eye to eye (NA27), cover slot or not |
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
