# Colo playtest 2026-09-18 — screenshot evidence

Part of the [2026-09-18 colo playtest report](README.md). Section numbers continue from that document so existing references (code comments, PR descriptions) still resolve.

## 8. Screenshot evidence (supplied by the owner after the first draft)

Four Discord screenshots were checked against telemetry for the same seconds. Images are not committed; filenames
are the owner's local names.

### 8.1 `moonwalk.png` — 7:03 PM CDT, Cellblock Guard

What it shows: a hostile Cellblock Guard in a **full run animation with his back to the player**, feet clearly above
his drop shadow, on or beside a ramp. The player is still in the orange prison jumpsuit, which dates it before the
crate at 7:17 PM.

Telemetry for 00:03:16–00:03:18 UTC:

| Entity | Position (x, y, z) | Notes |
|---|---|---|
| Guard `npc_id 100131` | (-287.8, 68.6, -160.6) → (-287.5, 68.6, -162.1) | `waypoint_reached`, 2 then 1 waypoints remaining; Y is the quantized 68.6 on both |
| Player (entity 2) | (-314.2, 73.47, -190.3) → (-307.8, 71.09, -180.4) | velocity (+3.26, 0, +5.04); Y falling 2.4 u in 2 s — **the player is walking down a ramp toward the guard** |

Guard → player is (Δx, Δz) ≈ (-24, -26). A chase heading toward the player is `atan2(-24, -26) ≈ -2.4 rad` —
negative, so `pack_angle` saturates it to byte 0 = facing +Z. The player is on the guard's -Z side, so **+Z is
directly away from the player**: the guard runs at the player showing his back. That is the screenshot.

Two conclusions:

- **This incident is H1, not the animation decoupling.** The guard is playing a run cycle, so the client *did* have
  a moving animation state; what is wrong is the body yaw. §4.5 remains a real gap but is not what "moonwalk" meant
  here. This also answers the open question in §4.4 for this case: translation correct, facing reversed.
- The first logged corner leg had `dx = +0.3` (yaw positive, transmitted correctly). The facing broke on the
  *following* legs, when the bearing swung to `dx < 0` — the "first leg fine, next leg backwards" pattern,
  produced by bearing rather than by leg number, exactly as the movement appendix argues.

The float: the guard's path Y is pinned at the quantized 68.6 while the scene is a ramp (the player's own Y moves
2.4 u across it). Lerping between quantized corners across a ramp and sending the result as FullPos (H2) puts him
in the air. The shadow-to-feet gap in the image is on the order of half a metre — far more than the 0.15–0.18 u
flat-floor sawtooth, as expected on a slope.

### 8.2 `levitate.png` — ~7:27–7:29 PM CDT, Dr. Zuritska following

What it shows: Zuritska upright, in an idle pose, well above the floor (his drop shadow appears on the floor tiles
left of the player, far below his feet), quest marker overhead, body in profile rather than facing the player.

Telemetry for 00:29:43–00:29:49 UTC:

| UTC | Follower waypoint Y (`npc_id 100112`) | Player Y (entity 2) | Player `vy` |
|---|---|---|---|
| 00:29:43.19 | 69.921 | — | — |
| 00:29:45.15 | — | 71.735 | -0.5 |
| 00:29:45.19 | **70.899** | — | — |
| 00:29:45.82 | — | 70.109 | 0 (grounded) |
| 00:29:46.46 | — | 70.357 | **+7.37** |
| 00:29:47.10 | — | 71.032 | **-5.28** |
| 00:29:47.19 | 70.222 | — | — |
| 00:29:48.99 | 70.127 | — | — |

The floor here is Y ≈ 70.1 (the player's grounded sample). **The tester was jumping**, and the follower's
destination Y tracks the player's *airborne* Y: it was sent to 70.899 — 0.8 u above the floor — because the player
was mid-jump when `follow.rs:88-98` sampled `target_pos.y`. Earlier in the same corridor (z ≈ 1036) the follower's
Y swings 67.10 → 68.51 → 67.24 → 68.25 on consecutive legs, the same signature.

This **replaces** the "authored spawn Y" explanation for "levitating, then he came down" (movement appendix A6):
`other z dude.png` shows Zuritska standing correctly on the floor of his cell, so his authored Y is fine. The
mechanism is: no navmesh in Castle (H3) → straight-line fallback → destination Y copied from a jumping player →
rendered literally (H2) → he "comes down" on the next leg when the player happens to be grounded at sample time.
With a navmesh `find_path` would project the destination onto the mesh and hide this; the robust fix is to never
take a follower's Y from the target — clamp to the NPC's own Y when unrouted, and to the mesh when routed.

Facing: he is in profile to the player with an idle pose, consistent with H4b (yaw frozen at the end of the last
leg) — not provable from one still.

### 8.3 `one z dude.png` and `other z dude.png` — the two Zuritskas

- `one z dude.png`: the Comms twin (`Castle_Zuritska_Comms`, 100113) at the Communications Terminal, grounded,
  **with a mission marker overhead**.
- `other z dude.png`: Zuritska in a cell, grounded, no marker, prison boot visible.
- `levitate.png`: the escort (`Castle_Zuritska_Cell`, 100112) **also carries a mission marker**.

So both rows advertise the same mission interaction at the same time — beyond the visual duplicate, a player could
plausibly turn in at either. The suppress/despawn action proposed in §5 should cover the marker as well as the model.
Both show the prison boot, matching the tester's "so he has the boot too".

### 8.4 What the screenshots changed

| Item | Before | After |
|---|---|---|
| H1 as the cause of "walks at me backwards" | Verified in code, unproven per-incident | Positions + image agree for the 7:03 PM guard |
| Open question: translation wrong, or only facing? | Open | Facing only, for this incident |
| Animation decoupling as the 7:03 PM cause | Inferred | Ruled out for this incident (run cycle is playing) |
| "Levitating then came down" | Authored spawn Y | Follower copies the jumping player's Y; authored Y is fine |
| New seam | — | Log `target_y`, `target_vy` / `target_on_ground` and `dest_y` on every follow `move_order`; a follower whose `dest_y` differs from its own Y with no routed path is the air-climb signature |

### 8.5 The Romney screenshots — `romney.png`, `wall-shouild-be-door.png`, `wall-no-texture.png`, `floor-with-hole.png`

What they show:

- `romney.png`: NID Interrogator Romney inside a cell behind an open doorway numbered **01**, in a cell identical to
  Zuritska's — standing **with his back to the doorway**.
- `wall-shouild-be-door.png`: the corridor outside that cell ends in a solid panelled wall between two pillars where
  the connection to the rest of the block should be. The chat box reads `.gotoxyz 320.64 66.97 1042.95` →
  `gotoxyz: moved entity 2 to (...)` → **`Ghost enabled.`**
- `wall-no-texture.png`: the other end of that corridor is closed by a flat **untextured** surface.
- `floor-with-hole.png`: a gap in the floor beside a grate, open to the void.

An untextured blocking surface plus a hole in the floor is unfinished developer geometry that was walled off, not a
door that failed to open.

Why Romney is in there (seed, `db/resources/Worlds/Seed/spawnlist.sql:353-416`): CA05 recovered 36
`CA-Cell_Doorway01_Pf0` prefab instances across two map tiles and anchored both NPCs on them —

| Row | Tile | Position | Heading |
|---|---|---|---|
| 238 `Castle_Zuritska_Cell` | `Castle-000a0002` | (268.00, 66.79, 1042.59) | 0 |
| 240 `Castle_Romney` | `Castle-000a0003` — "second half of the corridor (interpreted as Interrogation Room 02)" | (320.64, 66.79, 1042.59) | 0 |

Same Y, same Z, X offset by exactly 52.64 — the same cell in the neighbouring tile. The raw map read cannot see
that tile `000a0003` is the sealed, unfinished mirror of `000a0002`; the comment rates the placement HIGH
confidence. **Fix: move spawn 240 to a doorway prefab in tile `Castle-000a0002`** (the accessible wing — the escort's
logged path runs x 240 → 277 along z ≈ 1036), matching Lomiada's "one door after the Dr." Treat every other
reconstructed Castle row anchored in `000a0003`, or in any tile not yet walked in-client, as suspect until a
`.location` walk confirms it; the seed comments already ask for that walk for the Comms room.

How the Jaffa reached him in 24 seconds with no `.gotoxyz` — `movement.player`, entity 3:

| UTC | Position (x, y, z) | Velocity (vx, vy, vz) |
|---|---|---|
| 01:19:56.8 | (297.9, 67.18, 1038.0) | (0, 0, 3.9) — at the sealed wall |
| 01:20:08.1 | (299.3, 66.84, 1035.8) | (**17.5, -4.2**, 0.9) |
| 01:20:08.8 | (308.3, 67.29, 1036.3) | (**17.6, -3.6**, 1.0) |
| 01:20:09.9 | (314.2, 66.99, 1039.1) | (0, 0, 0) |
| 01:20:11.5 | (318.8, 66.99, 1037.1) | (0, 0, 0) — outside Romney's cell; Romney dies 01:20:14 |

A straight 20-unit run in +X through where the wall stands, with a sustained vertical velocity of about -4 while Y
stays level (every on-foot sample in the session has `vy = 0` on flat floor) — that is the UE3 `ghost` fly/noclip
cheat, the same one the first character's chat box shows being enabled. The room **is** sealed; the second run
simply went through the wall. The server saw nothing unusual: Castle has no navmesh, so `is_position_valid` fails
open for everyone there (H3), GM or not.

Two side findings:

- **Every reconstructed Castle row has `heading = 0`**, i.e. facing +Z. The cells sit on the +Z side of the corridor,
  so both Romney and Zuritska spawn facing the back wall — which is exactly `romney.png`. This is the owner's
  "wrong facing while idle at spawn", and it is authoring, not `pack_angle`: seed headings are stored in [0, 2π),
  which the saturating cast handles correctly (it only breaks *negative* yaws from `atan2`). Cell occupants want
  `heading ≈ 3.14159`. The image also independently confirms the axis convention used in §8.1 (yaw 0 = +Z).
- Client-side `ghost` leaves no server trace. With the speed and navmesh validators both warn-only / fail-open, a
  flying player is only inferable from `vy ≠ 0` on level ground. A cheap seam: log `vy` sign-vs-ΔY disagreement, or
  at minimum tag `movement.player` samples with `access_level` so GM noclip can be excluded when reading NPC
  chase/follow data (an NPC pathing to a ghosting target is not a fair test of the pathing).

Correction to §8.2: the tester's airborne Y at 7:29 PM reads as jumping (velocity +7.4 then -5.3 with Y rising then
falling — ballistic), and ghost mode is only evidenced from 7:40 PM onward. Either way the defect is the same: the
follower adopts the target's airborne Y.
