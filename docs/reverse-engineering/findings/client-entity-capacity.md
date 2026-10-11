# Client entity capacity, actor spawn and avatar-filter timing (CL-03)

> **Date**: 2026-10-11
> **Method**: headless Ghidra (`SGW.exe` QA build, read-only) decompile of the `EntityManager` map code, `UWorld::SpawnActor`, the filter classes in `BWFilter.cpp`, and a dump of the filter's constants from `.data`/`.rdata`. Nothing here ran in a live client. The filter section was re-read against a BigWorld engine review on 2026-10-11; where the two disagree the text says so.
> **Status**: statically verified unless a line says **inferred**. Packet CL-03 of [client-limits](../../analysis/client-limits/README.md).
> **Related**: [client-entity-lifecycle.md](client-entity-lifecycle.md) (the maps and handlers), [position-updates.md](../../protocol/position-updates.md) (filter overview), [npc-movement-pathfinding.md](npc-movement-pathfinding.md). The bounds table rows are in [aoi.md](../../client/limits/aoi.md) and [timing.md](../../client/limits/timing.md).

## Summary

The client has **no fixed-size entity table and no entity-count check** on the create path, and **no actor-count limit** in `SpawnActor`. Every table on the path is a growable `std::map`. The practical ceiling is address space, memory and the appearance scheduler, not a count. The client also has **no entity-lifecycle AoI cull of its own**: it creates whatever the server introduces and removes whatever the server withdraws (UE3 render culling was not examined). There are **two avatar filter classes** with different timing. The BigWorld-style one renders remote entities **0.1 to 1.0 s behind** the stream (adaptive) from an **8-frame** ring. The SGW one has no render delay: it extrapolates from the newest sample and snaps on a speed above **2500** units/s. Which class a given entity type uses is not recorded.

## Q1. Entity table

| Fact | Evidence |
|---|---|
| The world map (`EntityManager+0x18`) is a `std::map<int, Entity*>`; `enterWorld` (`0x00dd1d00`) inserts with the find-or-insert helper `0x00c6c500` (tree descent, then `FUN_00c6c1a0` to allocate a node). No size compare anywhere in it. | decompile of `0x00dd1d00`, `0x00c6c500` |
| The create handler allocates the `Entity` through the factory `FUN_00e9b8d0(FUN_00e9ba60())` (`GameEntityFactory.cpp` line `0x27`), whose registry `DAT_01ef27c0` is a map keyed by **type id**, not by instance. The only failure is an unknown type id (assert `FALSE`, returns null); `BW_client_entity_manager_7` (`0x00dd09e0`) then asserts `NULL != entity` (`entity_manager.cpp` line `0xa3b`). | decompile of `0x00e9b8d0`, `0x00dd09e0` |
| `Entity` init (`0x00e685e0`) allocates a fixed 0x1c-byte block with `scalable_malloc` and throws `bad_alloc` on null. | decompile of `0x00e685e0` |
| The cache map (`+0x24`), pending map (`+0x30`) and queue map (`+0x3c`) are the same kind of tree. | [client-entity-lifecycle.md](client-entity-lifecycle.md) |

So past any count nothing is refused; the client runs out of address space or time first. Limits that do exist are on **identity**, not count: a second create for an id already in the world map asserts (`NewEntity->getEnterCount() > 0`, line `0x1e0`), and a create with enter count below 1 parks the entity ([client-entity-lifecycle.md](client-entity-lifecycle.md)).

The MSVC "too long" guards (`list<T> too long`, `map/set<T> too long`) fire at the container's `max_size()`, which is `SIZE_MAX / sizeof(node)` for a 32-bit process, far above what 4 GB of address space (Large Address Aware, see [process.md](../../client/limits/process.md)) can hold. Address space runs out first.

**Server-side lever (BigWorld knowledge, not from this binary).** In stock BigWorld the effective cap on what a client holds was the server's per-client bandwidth budget (`bitsPerSecondToClient`, with witness updates priority-queued into it), not anything on the client. That is the lever to look for on the server side.

## Q2. Actors

`FUN_00e994b0` (called from `FUN_00e69070`, `0x00e6909b`) calls `UWorld::SpawnActor` (`0x00876970`) with the class cached in `DAT_01ef26d4`, no template and no owner. This is the per-entity spawn wrapper; **which entity event reaches `FUN_00e69070` is not traced** (the appearance job runner behind `0x00e998e0` is the likely route, **inferred**). `SpawnActor` itself has these refusals only: null class, class flags `0x2000000` or `1` set, a non-actor class, the `CurrentLevel` and `GWorld` assertions (`UnLevAct.cpp` lines `0x84` to `0x86`), and a collision-test failure when a placement check applies (`FUN_00876230`, skipped when `param_7 != 0`; the wrapper passes `1`). **No actor-count comparison exists in it.** Strings `too many actors`, `max actors`, `maxactors` were searched in `.rdata`: not found.

## Q3. Interpolation

### Two filter classes

The filter objects are UObjects with an input and an output function in a vtable pair. Both classes read and write the same frame layout (stride `0x38`, time at `+0x40`, position at `+0x4c`, velocity at `+0x58`).

| Class | Input | Output | Vtable slots |
|---|---|---|---|
| BigWorld avatar filter (`BWAvatarFilter`) | `0x00e81030` | `0x00e824f0` | `0x019e23b0` / `0x019e23b4` |
| SGW avatar filter (`USGWAvatarFilter`) | `0x00e81970` | `0x00e81dc0` | `0x019e24d0` / `0x019e24d4` |

The ring, the render delay, its target and convergence belong to the BigWorld class. The velocity smoothing, the teleport snap and the angle fold belong to the SGW class. An earlier draft of this document attributed all of them to one filter.

### BigWorld class constants

| Constant | Address | Value | Role |
|---|---|---|---|
| Frame count | `DAT_01e69c8c` | **8** | Ring buffer size. The `FrameCount > 4` assertion (`0x00e823b0`, `0x00e82a30`) is a floor, not the value. The filter constructor seeds frame times at `i - 2000.0` (`DAT_01848ac4`). |
| Offset convergence rate | `DAT_01e69c80` | **0.3** | `Output` moves the render delay (`+0x204`) toward its target (`+0x208`) by at most `0.3 * dt` per call, linearly and in both directions (0.1 to 1.0 takes 3 s). |
| Target delay floor / ceiling | `DAT_01e69c84` / `DAT_01e69c88` | **0.1 s / 1.0 s** | Clamp on the target. |
| Idle-velocity cut | `DAT_0181418c` | 100.0 | In `Output`, a mean frame speed at or below 100 uses 1 sample instead of the multi-sample blend (count at `+0x228`, spacing `+0x22c`). The mean is `+0x220 / +0x224`: `Input` keeps a running sum of the frames' speed magnitudes and a frame count, dropping the oldest when the count reaches `+0x228` (static). |
| Stale test | `DAT_018fde24` | 1e-4 s | `Output` writes the entity velocity at `+0xf4` only if the gap since the last output exceeds this: zero when the mean speed is at or below the idle cut, else the sampled velocity. |

### SGW class constants

| Constant | Address | Value | Role |
|---|---|---|---|
| Teleport threshold | `DAT_01e69c90` | **2500.0** | `Input` (`0x00e81970`) snaps all frames when `distance(newPos, previousFramePos) / dt` exceeds it, or reset bit 1 (`+0x21c`) is set. |
| Velocity damping | `DAT_01816a84` / `DAT_01903cd4` | **2.0 / 0.3333** | `v = (v_prev + 2 * v_observed) / 3` per axis, with `v_observed = delta / dt`. |
| Angle fold | literal `0x4001` | 16385 units | The yaw and pitch deltas subtract or add `0x4001` outside +-`0x4001`. This is a fold, not a modulo wrap, so a jump above roughly 90 degrees between updates yields a wrong angular velocity. |

Units: positions are floats in the filter's units; angles are integers (UE3 style rotators). The teleport unit is **inferred** to be UE3 units per second, so 2500 is 25 m/s and the idle cut of 100 is 1 m/s. The reasoning: the filters are UObjects with `execInput` / `execOutput`, integer rotators are the UE3 convention, the repo uses 100 UU per metre ([ability-system.md](../../gameplay/ability-system.md)), and 2500 m/s with a 100 m/s idle cut would both be absurd. Not confirmed on a live client.

### Behaviour

- **Render delay target (BigWorld class).** `Output` recomputes the target only on the first call after reset bit 0 (`+0x21c`) was set, as `clamp(now - T, 0.1, 1.0)` where `T` is the time of frame `(idx + 7) % 8`, the sample **before** the newest (the newest is `idx`; `Input` advances `idx` before writing). The BigWorld `Input` sets bit 0 on every accepted frame, so the target is refreshed on each arrival. That reads as a gap-since-previous-sample measure. Whether it tracks network latency depends on what time `Input` is given: **inferred**, if that is client-local receipt time (as in stock BigWorld) the target sits near the 0.1 floor except after hitches or gaps. What time the callers pass is not traced. The SGW `Input` never sets bit 0 and the SGW class has no render delay at all.
- **Oldest-frame clamp.** After sampling, if `now - delay` is earlier than the oldest frame (`(idx + 1) % 8`) and entity flag `+0x1b0` bit 0 is set, `Output` pulls the delay to `now - oldest` (lines setting `+0x204`). At 10 Hz the 8 frames span about 0.7 s (7 intervals), so for those entities the real ceiling is the buffer span, not 1.0 s.
- **Sampling.** The sampler (`0x00e81430`) lerps between the two frames bracketing the requested time and, past the newest frame, **holds the last position** (velocity term multiplied by 0.0).
- **Extrapolation (SGW class).** `Output` (`0x00e81dc0`) computes `previous.pos + velocity * (now - newest.time)` for positions and angles with **no upper bound on the time term**, only a `> 0` test. This is the entity that keeps moving when the server goes quiet.
- **Equal timestamp discard.** The whole body of the SGW `Input` sits inside `if (t - newestFrameTime != 0.0)`: an input with the same timestamp as the newest frame is dropped entirely (static). An older timestamp is not dropped; it gives a negative `dt`. The BigWorld `Input` drops any input not strictly newer. Consequence (**inferred**, if the time is per-packet receipt time): two position updates for one entity in one bundle keep the stale one, and a tiny non-zero `dt` between packets within a frame inflates the speed and can trigger a spurious teleport snap. Server advice: coalesce to the latest position per entity per bundle.
- **Teleport.** A step above the threshold, or reset bit 1, snaps every frame to the new position with no blend. Stock BigWorld `AvatarFilter` has no velocity snap (a teleport goes through `reset()`), and its latency model is different (from the engine reviewer's memory, not this binary: `latencyMinimum` 0.1, `latencyFrames` 2.0, a power-curve approach, no 1.0 ceiling). The velocity snap and the 0.1 / 1.0 / 0.3 target formula are therefore CME changes; do not "correct" them to stock.
- **Reset path.** `BWAvatarFilter::Input` (`0x00e81030`) with reset bit 1 set and a valid Z fills the ring with the new frame: it writes the last slot, copies it down through the older slots subtracting 1.0 s per step from the time, sets `idx` to the last slot, sets bit 0 and clears bit 1. This replaces an earlier note calling it a buffer overrun.
- **Invalid Z sentinel.** In the same function, `param_6 < -50000.0` (`DAT_019e204c`) means "no Z": outside reset the previous frame's Z is reused; under reset the whole input is ignored. The value `-50000` is not an overrun threshold.

## Q4. AoI radius

Searched `.rdata` for `radiusofinterest`, `areaofinterest`, `maxentit`, `aoi`. Only logging strings match: `Entity Enter AoI` (`0x019cded0`, from `enterAoI` `0x00dd24f0`), `Entity Leave AoI` (`0x019ce02c`), `[Entity not in client AOI]` (`0x019b9d68`, used by `EntityAppearanceLog` `0x00cfea00` when the id is absent from the world map) and `Entity not in AOI, killing job` (`0x019eb9d0`, `0x00eb7890`, an appearance job whose entity left the map). **No entity-lifecycle radius, distance test or cull.** "In AoI" means "in the world map". UE3 render culling (cull distance volumes, maximum draw distance, LOD) was not examined. The only distance constant on the entity path is the SGW filter's teleport threshold above.

## Open

- The path from an entity event to `FUN_00e69070` / `SpawnActor` (the appearance job runner) is untraced.
- Whether `ABigWorldEntity` actors are ever pooled or reused; the wrapper spawns without a template.
- The SGW filter constructor `0x00e82a30` was not decompiled, so the sample count `+0x228` and spacing `+0x22c` defaults are unknown.
- Which filter class each entity type gets, and what time the callers pass to `Input` (receipt time or a server timestamp), need a live probe or the entity defs.
- Who sets reset bit 1 (`+0x21c`): both `Input` functions only test or clear it.
- The teleport threshold unit needs a live probe.
- UE3 render-side culling (cull distance volumes, maximum draw distance, LOD) is unexamined.
- BigWorld-side claims (the bandwidth-budget lever, stock `AvatarFilter` parameters, 100 ms update spacing) are not from this binary. The 100 ms spacing is the server's: see `crates/cell/src/cell/service/message_loop.rs` and `tick_rate` 100 in `crates/wire/src/mercury/game_clock/mod.rs`.
