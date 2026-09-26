# Client generic-region hit test

**Status:** HIGH confidence (decompiled from SGW.exe, three functions, cross-checked against 2026-09-26 colo telemetry).
**Consumers:** `crates/services/src/cell/spawner/regions.rs` (`client_would_hint_region`), the `region_dwell_no_hint` friction signal in `crates/services/src/cell/playtest_friction_watch.rs`.

## Summary

The client decides on its own when to send `triggerClientHintedGenericRegion`. It keeps the volumes it was given by `addClientHintedGenericRegion` and hit-tests its pawn against them every tick. Its test is **not** the server's gate (`is_point_in_region`, a port of `GenericRegion.py:isPointInRegion`). The two differ on the vertical axis:

| Bound | Client (`FUN_00eae960`) | Server gate (`is_point_in_region`) |
|---|---|---|
| Floor | `min.y - 100.0` | `min.y - 1.5` |
| Ceiling | `max.y`, exact | `max.y + 1.5` |
| X / Z | AABB exact, then XZ polygon | XZ polygon, else AABB plus 1.5 |

So the client reports a room from anywhere up to 100 units below it, and the server refuses those reports. The client never reports a room from above its ceiling.

## Functions

### `FUN_00eaed70`: `addClientHintedGenericRegion` handler

It reads `regionId`, `height`, `radius`, `flags` and `points` from the event property tree into a 0x38-byte record: `+0x00` id, `+0x04` height, `+0x08` radius, `+0x0c` flags, `+0x10` point vector, `+0x20..+0x28` AABB min, `+0x2c..+0x34` AABB max.

The AABB is built as follows:

- The **first** point sets both min and max verbatim. Its Y does not get `height`.
- Every later point widens min with `p`, and widens max with `(p.x, p.y + height, p.z)`.

### `FUN_00eae960`: point-in-region

The position passed in is the pawn location divided by 100 (`FUN_00eaf310` scales by `1 / [0x018cad90]`, which is `100.0f`) and swizzled into BigWorld axes (`bw = (ue.y, ue.z, ue.x) / 100`).

1. The point is rejected if `x < min.x`, `y < min.y - [0x019eab10]`, `z < min.z`, `x > max.x`, `y > max.y` or `z > max.z`. `[0x019eab10]` is `100.0f` (`00 00 c8 42`). It is applied to a position that is already in world units, so the floor reaches 100 world units down.
2. A region with exactly one point is an XZ circle: it is inside when `dx² + dz² <= radius²`. The AABB of a single point is that point, though, so step 1 already rejects nearly everything. This is why the server's cylinder workaround expands one-point regions to four corners before sending them.
3. Anything else uses an XZ ray-cast polygon. It toggles when `(z_i > z) != (z_j > z)` and `x < x_i + (z - z_i)(x_j - x_i)/(z_j - z_i)`. That is the same crossing rule as `region_contains_xz`.

### `FUN_00eaf310`: per-tick diff and emit

This function collects the regions that contain the pawn (`FUN_00eaeb90`) and diffs them against the previous set. It then emits one `Event_NetOut_TriggerClientHintedGenericRegion` (`id`, `bEntering`, `position`) per region that was entered or left.

## Evidence from telemetry (2026-09-26 colo, build 623ada98)

- **Floor reach.** Castle_Cellblock.Region3, the Mess Hall, spans y 34.52 to 45.90. Two players each sent an ENTER hint at about (-101.5, 24.67, -133.1), the corridor one floor below. The server refused both with `region_containment_failed`, and an EXIT followed 4 to 5 seconds later. The client's floor reach explains the hint. The server's 1.5-unit floor explains the refusal.
- **Exact ceiling.** Region6 has its ceiling at 29.96 and Region12 at 31.90. Players at y 39.55, in the room above them, sent no hints. The old XZ-only `region_dwell_no_hint` detector flagged both regions as "client sent no hint".

## Implications

- A refusal of the "room from the floor below" kind is expected and correct: the player is not in the room. Do not widen the server floor to match the client.
- When the refused ENTER is followed by the player climbing into the room, the client does not send a new ENTER, because it already counts the player as inside. The server then never sees that entry. No Castle_Cellblock chain hit this in the 2026-09-26 session. However, an `enter_region` chain on a room with a reachable space underneath can be starved this way.
- Any server-side "the client should have hinted" heuristic must use the client's test (`client_would_hint_region`), not an XZ footprint.
