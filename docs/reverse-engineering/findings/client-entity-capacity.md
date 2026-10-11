# Client entity capacity, actor spawn and avatar-filter timing (CL-03)

> **Date**: 2026-10-11
> **Method**: headless Ghidra (`SGW.exe` QA build, read-only) decompile of the `EntityManager` map code, `UWorld::SpawnActor`, the filter classes in `BWFilter.cpp`, and a dump of the filter's constants from `.data`/`.rdata`. Nothing here ran in a live client.
> **Status**: statically verified unless a line says **inferred**. Packet CL-03 of [client-limits](../../analysis/client-limits/README.md).
> **Related**: [client-entity-lifecycle.md](client-entity-lifecycle.md) (the maps and handlers), [position-updates.md](../../protocol/position-updates.md) (filter overview), [npc-movement-pathfinding.md](npc-movement-pathfinding.md).

## Summary

The client has **no fixed-size entity table and no entity-count check** on the create path, and **no actor-count limit** in `SpawnActor`. Every table on the path is a growable `std::map`. The practical ceiling is memory and the appearance scheduler, not a count. The client also has **no AoI radius of its own**: it creates whatever the server introduces and removes whatever the server withdraws. The avatar filter renders remote entities **0.1 to 1.0 s behind the newest received position** (adaptive), from an **8-frame** ring buffer.

## Q1. Entity table

| Fact | Evidence |
|---|---|
| The world map (`EntityManager+0x18`) is a `std::map<int, Entity*>`; `enterWorld` (`0x00dd1d00`) inserts with the find-or-insert helper `0x00c6c500` (tree descent, then `FUN_00c6c1a0` to allocate a node). No size compare anywhere in it. | decompile of `0x00dd1d00`, `0x00c6c500` |
| The create handler allocates the `Entity` through the factory `FUN_00e9b8d0(FUN_00e9ba60())` (`GameEntityFactory.cpp` line `0x27`), whose registry `DAT_01ef27c0` is a map keyed by **type id**, not by instance. The only failure is an unknown type id (assert `FALSE`, returns null); `BW_client_entity_manager_7` (`0x00dd09e0`) then asserts `NULL != entity` (`entity_manager.cpp` line `0xa3b`). | decompile of `0x00e9b8d0`, `0x00dd09e0` |
| `Entity` init (`0x00e685e0`) allocates a fixed 0x1c-byte block with `scalable_malloc` and throws `bad_alloc` on null. | decompile of `0x00e685e0` |
| The cache map (`+0x24`), pending map (`+0x30`) and queue map (`+0x3c`) are the same kind of tree. | [client-entity-lifecycle.md](client-entity-lifecycle.md) |

So past any count nothing is refused; the client runs out of address space or time first. Limits that do exist are on **identity**, not count: a second create for an id already in the world map asserts (`NewEntity->getEnterCount() > 0`, line `0x1e0`), and a create with enter count below 1 parks the entity ([client-entity-lifecycle.md](client-entity-lifecycle.md)).

## Q2. Actors

`FUN_00e994b0` (called from `FUN_00e69070`, `0x00e6909b`) calls `UWorld::SpawnActor` (`0x00876970`) with the class cached in `DAT_01ef26d4`, no template and no owner. This is the per-entity spawn wrapper; **which entity event reaches `FUN_00e69070` is not traced** (the appearance job runner behind `0x00e998e0` is the likely route, **inferred**). `SpawnActor` itself has these refusals only: null class, class flags `0x2000000` or `1` set, a non-actor class, the `CurrentLevel` and `GWorld` assertions (`UnLevAct.cpp` lines `0x84` to `0x86`), and a collision-test failure when a placement check applies (`FUN_00876230`, skipped when `param_7 != 0`; the wrapper passes `1`). **No actor-count comparison exists in it.** Strings `too many actors`, `max actors`, `maxactors` were searched in `.rdata`: not found. The only "too long" strings are the MSVC container guards (`list<T> too long`, `map/set<T> too long`), reachable only at 2^32 elements.

## Q3. Interpolation

Constants read from the binary (the draft's `DAT_` names resolved):

| Constant | Address | Value | Role |
|---|---|---|---|
| Frame count | `DAT_01e69c8c` | **8** | Ring buffer size. The `FrameCount > 4` assertion (`0x00e823b0`, `0x00e82a30`) is a floor, not the value. Frame stride `0x38`; the filter constructor seeds frame times at `i - 2000.0` (`DAT_01848ac4`). |
| Offset convergence rate | `DAT_01e69c80` | **0.3** | `BWAvatarFilter::Output` (`0x00e824f0`) moves the render delay (`+0x204`) toward its target (`+0x208`) by at most `0.3 * dt` per call. |
| Target delay floor / ceiling | `DAT_01e69c84` / `DAT_01e69c88` | **0.1 s / 1.0 s** | The target is `now - newest frame time` at each arrival, clamped to this range (`0x00e824f0`, lines setting `+0x208`). |
| Teleport threshold | `DAT_01e69c90` | **2500.0** | `USGWAvatarFilter::Input` (`0x00e81970`) hard-snaps all frames when `distance / dt` exceeds it. Units are the filter's position units, **inferred** to be UE3 units (cm), which would be 25 m/s; not confirmed. |
| Velocity damping | `DAT_01816a84` / `DAT_01903cd4` | **2.0 / 0.3333** | SGW velocity smoothing in `Input`. |
| Angle wrap | literal `0x4001` | 16385 units | Yaw/pitch delta wrap in `Input`. |
| Idle-velocity cut | `DAT_0181418c` | 100.0 | In `Output`, a mean frame speed at or below 100 uses 1 sample instead of the multi-sample blend. |
| Ring "stale" test | `DAT_018fde24` | 1e-4 s | `Output` zeroes the entity velocity at `+0xf4` only if the gap since the last output exceeds this. |

Behaviour:

- **Render lag.** A remote avatar is drawn at `now - delay`, `delay` in 0.1 to 1.0 s, tracking observed arrival latency at 0.3 s of change per second. The server tick is 100 ms, so the client sits at least one tick behind.
- **Interpolation window.** The newest of 8 frames; a position older than the oldest frame is not reachable. The sampler (`0x00e81430`) lerps between the two frames bracketing the requested time and, past the newest frame, **holds the last position** (velocity term multiplied by 0.0).
- **Extrapolation.** `USGWAvatarFilter::Output` (`0x00e81dc0`) extrapolates `last + velocity * (now - lastTime)` for positions and angles with **no upper bound on the time term** in the decompile. Which filter class a given entity type uses is not recorded here.
- **Teleport.** A step faster than the threshold, or the reset flag (`+0x21c` bit 1), snaps every frame to the new position with no blend.
- **Buffer overrun.** When a frame arrives with `param_6 >= -50000.0` (`DAT_019e204c`) while the reset flag is set, `0x00e81030` shifts the whole ring by one slot.

## Q4. AoI radius

Searched `.rdata` for `radiusofinterest`, `areaofinterest`, `maxentit`, `aoi`. Only logging strings match: `Entity Enter AoI` (`0x019cded0`, from `enterAoI` `0x00dd24f0`), `Entity Leave AoI` (`0x019ce02c`), `[Entity not in client AOI]` (`0x019b9d68`, used by `EntityAppearanceLog` `0x00cfea00` when the id is absent from the world map) and `Entity not in AOI, killing job` (`0x019eb9d0`, `0x00eb7890`, an appearance job whose entity left the map). **No radius, no distance test and no cull.** "In AoI" means "in the world map". The only distance constant on the path is the filter's teleport threshold above.

## Open

- The path from an entity event to `FUN_00e69070` / `SpawnActor` (the appearance job runner) is untraced.
- Whether `ABigWorldEntity` actors are ever pooled or reused; the wrapper spawns without a template.
- The SGW filter constructor `0x00e82a30` was not decompiled, so the sample count `+0x228` and spacing `+0x22c` defaults are unknown.
- The unit of the teleport threshold, and which filter class each entity type gets, need a live probe or the entity defs.
- BigWorld-side claims (that an AoI lives only on the cell, and 100 ms update spacing) are not from this binary and should be reviewed by the BigWorld engine advisor.
