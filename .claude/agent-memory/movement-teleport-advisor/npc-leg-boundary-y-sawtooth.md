---
name: npc-leg-boundary-y-sawtooth
description: Detour straight-path corners are grid-quantized in Y while the final corner is the true surface — NPCs bob ~0.15-0.18u at every path-leg boundary, on flat floors
metadata:
  type: reference
---

> **Status 2026-09-19 — mitigation landed, not re-measured.** The claim below that
> `get_navmesh_height` has zero production callers is no longer true: #680 calls it from
> `ticks/npc_movement.rs`. Whether that removes the measured 0.15–0.18u per-leg bob has not been
> re-measured against telemetry. The poly-mesh vs detail-mesh explanation stands.

**Measured 2026-09-18 from colo `movement.npc` telemetry.** Across every NPC in the session, Detour
`findStraightPath` returns **intermediate** corners on a 0.1-grid (X, Z **and Y**) while the **final**
corner is the caller's destination projected onto the detail mesh, hence irregular:

| NPC | intermediate `wp_y` | final `wp_y` | Δ |
|---|---|---|---|
| 100138 | 34.6 ×4 | **34.779** | +0.179 |
| 100137 | 34.6 | **34.751** | +0.151 |

Intermediate X/Z are likewise round (`-93.7`, `-130`, `-132.1`); final X/Z are not (`-96.827`,
`-125.964`, `-132.664`).

`npc_movement_tick` snaps to each waypoint exactly
(`cell/service/ticks/npc_movement.rs:79`) and lerps linearly between them (`:152`), so an NPC spends
each leg at the **quantized** Y — up to ~0.18 *below* the true floor — then snaps **up** to the true
surface at the last waypoint, then **drops back down** when the next leg's quantized corners arrive.

**A visible per-leg bob on a perfectly flat floor, with no stairs involved.** This is the mechanism
behind "NPCs don't stay stuck to the ground" reports that survive every stairs/ramp fix, and it is
why the defect reads as *second-leg-onward*: leg 1 from a standstill looks fine, the bob appears at
the first boundary. On slopes it is worse, because the lerp also cuts the chord over intervening
polys.

**How to apply:**
- Do not accept "waypoints come from findStraightPath so they're on the surface" as justification for
  skipping a ground query — the code comment at `npc_movement.rs:149-151` says exactly that and it is
  wrong. Poly-mesh ≠ detail-mesh.
- `SpaceManager::get_navmesh_height` (`cell/space_manager/spatial.rs:71-76`) is implemented and has
  **zero production callers**. It is the fix for the server's own truth.
- The cheaper fix is on the wire: see [[npc-broadcast-facing-and-grounding]] — sending the OnGround
  variant makes the client terrain-ray-cast and ignore our Y entirely.
- **Latent risk:** `START_EXTENTS = [0.5, 0.5, 0.5]` (`crates/entity/src/navigation/mod.rs:41`) is the
  vertical search box for the *start* poly on re-path. The measured drift is 0.15–0.18 per leg, under
  3× the margin. It did **not** fail this session (zero `no start poly` warnings), but a slope, a
  leash snap or accumulated drift could exceed it — and the failure mode is a silent fall-through to
  the unvalidated straight-line fallback ([[castle-has-no-navmesh]]).
- When triaging a grounding report, ask **"inches or body-heights?"** — this sawtooth is ~0.15 u
  (barely-unplanted); the straight-line fallback is metres (clearly airborne). The answer picks the
  root cause without instrumentation.
