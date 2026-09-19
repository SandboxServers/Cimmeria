# NPC Movement Kinematics & Wire Forensics — 2026-09-18 colo playtest

**Revision 2** — incorporates the owner interview (second-leg framing, flat-floor grounding errors,
facing-while-chasing, attack-stops-when-facing-away, leash return defects, logging-design request).

Scope: NPC movement kinematics and what goes on the wire. AI decision layer (aggro/leash/cover/target
selection) is a sibling agent's slice. Read-only investigation; no repo files edited, no builds run.

Time base: Discord CDT (UTC-5). Session 23:55 UTC → 01:35 UTC (2026-09-18/19).

---

> [!NOTE]
> **Superseded in two places by the screenshot review** (main report §8). (1) A6 "spawn float": the
> escort's authored Y is fine — `other z dude.png` shows him grounded in his cell. "Levitating, then he came
> down" is the follower copying the *jumping player's* Y through the unrouted follow fallback. (2) A9
> "moonwalk": the 7:03 PM guard was playing a run cycle with his back to the player, so that incident is A1
> (`pack_angle`), not the animation decoupling. Both mechanisms remain real; only their attribution to those
> two incidents changed.

## Executive summary

Six independent defects. They compose, which is why no single fix has made the symptom go away.

| # | Defect | Symptom it produces | Confirmed? |
|---|---|---|---|
| 1 | `pack_angle` saturates negative yaw → byte 0 (due north) | faces wrong way while chasing | **Yes, by inspection** |
| 2 | NPC yaw is written **only** inside the movement tick, which skips path-less NPCs | can never re-face; attack-stops-when-facing-away | **Yes, by inspection** |
| 3 | Detour corner Y is grid-quantized; final corner is true surface → **per-leg Y sawtooth** | not stuck to ground on flat floors; bobs at each leg boundary | **Yes, measured: 0.15–0.18 u** |
| 4 | We only ever send the `FullPos` wire variant → client never floor-snaps | everything in #3 and #5 is rendered literally | **Yes, by inspection + docs** |
| 5 | Castle has **no navmesh** → unvalidated 3D straight-line fallback, Y taken from the target | through walls/floors; climbs air toward the player | **Yes, in telemetry** |
| 6 | Leash is a bare `npc.position = spawn_pos` field write | drifts home off-geometry facing wrong | **Yes, by inspection** |

**The owner's "second leg" observation is the key that ties #1–#4 together — see A0.**

---

## A0. Why the *second* leg is where it breaks

The owner's sharpest observation: leg 1 looks fine, leg 2 goes up and/or walks backwards facing
backwards. Three separate mechanisms are all leg-boundary-triggered, and I now have the measurement
for the main one.

### The Y sawtooth — measured, and it is a leg-boundary artifact

From `waypoint_reached` telemetry, a pattern holds across every NPC in the session: **intermediate
waypoints have grid-quantized coordinates, the final waypoint has true projected surface
coordinates.**

| NPC | intermediate `wp_y` | final `wp_y` (`path_complete=true`) | Δ |
|---|---|---|---|
| 100138 (00:13:31–32Z) | 34.6, 34.6, 34.6, 34.6 | **34.779** | +0.179 |
| 100137 (00:12:57Z) | 34.6 | **34.751** | +0.151 |
| 100134 (00:19:30–31Z) | 24.8 | 24.8 (x/z irregular: -125.964/-114.171) | — |
| 100135 (00:19:08–09Z) | 24.8, 24.8 | 24.8 (x/z -132.664/-111.595) | — |
| 100147 (00:14:54Z) | 39.6, 39.6 | 39.6 (x/z -116.482/-61.809) | — |

Intermediate X/Z are likewise on a 0.1 grid (`-93.7`, `-130`, `-132.1`, `-102.1`) while final X/Z are
irregular (`-96.827`, `-96.298`, `-125.964`). That is the Detour signature: `findStraightPath`
corners sit on **poly-mesh** vertices (quantized by Recast's `cs`/`ch` cell size), while the end point
is the caller's destination **projected onto the detail mesh**, hence irregular.

**Consequence.** `npc_movement_tick` snaps to each waypoint exactly
(`crates/services/src/cell/service/ticks/npc_movement.rs:79` — `let snap_y = next_wp.y;`) and lerps
linearly between them (`:152`). So the NPC spends each leg at the **quantized** Y — up to ~0.18 below
the true floor — then at the last waypoint **snaps up to the true surface**. The next leg's corners
are quantized again, so it **drops back down**. A visible bob at every leg boundary, **on a perfectly
flat floor**, with no stairs involved.

This is exactly the owner's point 2 ("grounding errors happen on stairs/ramps AND on flat floors —
expect a constant offset"). It is a **constant offset of ~0.15–0.18 u that flips sign at each leg
boundary.** On stairs the same mechanism is worse, because linear interpolation across a multi-poly
span also cuts the chord over the steps.

And because we send `FullPos` (A2), the client renders every bit of that literally.

### Why facing looks leg-dependent

Within a leg, yaw is recomputed every tick toward the current waypoint
(`npc_movement.rs:156`) and is correct — *modulo* `pack_angle` (A1). `pack_angle` fails as a function
of **bearing**, not of leg: every heading with `atan2(dx, dz) < 0` transmits as due north.

Leg 1 is the approach from the NPC's spawn/patrol position — one fixed bearing that is either
correct or not, consistently. Leg 2 onward re-paths toward the player's *new* position, so the
bearing changes and scatters across the circle. The moment it crosses into the negative half, the
NPC snaps to north mid-chase. To the player that reads as "it was fine, then on the next move it
turned backwards." **No separate second-leg facing bug is needed to explain this** — and I looked
for one specifically (path-replacement, stale segment index, yaw from the previous segment) and did
not find it; see A7 for what I ruled out.

### Velocity goes to zero at every leg boundary

The final-waypoint branch sets `velocity = [0,0,0]` (`npc_movement.rs:106`, `:110`). The AI tick then
takes one tick (100 ms) to notice the empty path and issue a new one. During that window the server
broadcasts zero velocity, and `USGWAvatarFilter::Output` extrapolates
`position = lastPosition + velocity * dt`, so the client **halts** the NPC, then jerks it into the
new leg. Stutter at every leg boundary, stacking visually with the Y bob.

---

## A1. Backwards facing — `pack_angle` saturating cast. **CONFIRMED**

`crates/services/src/mercury/aoi/mod.rs:47-50`

```rust
pub(super) fn pack_angle(radians: f32) -> u8 {
    const SCALE: f32 = 0.024543693;
    (radians / SCALE) as u8
}
```

The doc comment claims this "Matches C++ `(uint8_t)(angle / 0.024543693f)`". **It does not.** Rust's
float→int `as` cast is **saturating** (since 1.45); a negative float becomes `0u8`. C++ truncates
modularly on x86 and yields the correct wrapped byte.

NPC yaw is `dx.atan2(dz)` (`npc_movement.rs:103`, `:110`, `:156`), range `(-π, π]`:

| yaw | quotient | `as u8` | rendered |
|---|---|---|---|
| `(0, π]` | `0..128` | correct | correct |
| `(-π, 0)` | `-128..0` | **0** | **due north** |

Half the compass collapses to north. Matches "wrong facing is mostly while walking/chasing, ~180°
off" — a north-facing NPC walking south reads as exactly backwards, and the owner's sample is
dominated by the chase bearings that happen to be negative.

**Confirming experiment (no client needed):** `assert_eq!(pack_angle(-FRAC_PI_2), 192)` — returns `0`
today. **Fix:** `(radians.rem_euclid(std::f32::consts::TAU) / SCALE) as u8`. Reproduces C++ for all
inputs; no change to the currently-correct `(0, π]` half. **Any regression guard must use a negative
angle** — a `[0, π]` fixture passes with the bug present.

**Same function, second defect (players).** The inbound path stores packed angle *bytes cast to f32*
rather than radians: `crates/services/src/base/connect_loop/encrypted/mod.rs:278` reads
`payload[32..34] as i8`; `crates/services/src/cell/space_manager/entities.rs:404-408` casts each
`as f32` into `Vector3`. There is no `unpack_angle` in the tree. `pack_angle` then re-divides that
byte by 2π/256. Every moving **player** also has a wrong facing for witnesses. Tracked as P49
(`docs/analysis/legacy-command-parity/work-packets.md:62`). Fix both together — but note the units
mismatch trap in A8.

---

## A2. NPCs can never turn in place. **CONFIRMED — answers the owner's point 3**

The owner asked specifically: *does the server ever send a turn-in-place / yaw-only update?*

**No. There is no re-face path in the codebase at all.**

`npc_movement_tick` selects its candidate set with `!e.nav_path.is_empty()`
(`npc_movement.rs:41-47`), and it is the **only** production writer of NPC `direction` apart from
spawn. Exhaustive grep of `.direction = ` in `crates/services/src/cell/`:

- `npc_movement.rs:121`, `:187` — the movement tick (path-holders only)
- `npc_respawn/mod.rs:296` — `spawn_dir` at respawn
- `console/entity.rs:241`, `console/placement.rs:272` — GM console only

When an NPC closes to range and attacks, `fight.rs:473` does `npc.nav_path.clear()`. From that
instant the NPC is **excluded from the movement tick**, so its yaw is frozen at whatever value it
held when the path emptied — forever, no matter how far the target strafes.

This explains the owner's observation precisely: *"little wrong-facing while attacking in place —
BUT when the NPC ends up facing far enough away from the target it stops attacking while apparently
still holding threat."* It faces correctly at the moment it stops (that's the last yaw the tick
wrote), the player then circles, and the NPC cannot follow. `attack_in_place` was the dominant
decision this session — **123 events vs 25 `chase`** — so this is the common case, not an edge case.

I found **no server-side facing-arc gate** (grep across `cell/abilities/` and `cell/combat/` for
facing/arc/angle checks returned nothing). So if attacks genuinely stop when the NPC faces away, that
gate is **client-side** — which makes the frozen yaw fatal rather than cosmetic.

**Fix (my half):** add a yaw-only update path. Either (a) let the movement tick process path-less
NPCs that have a combat target, writing yaw toward the target without touching position, or (b) a
small `face_target` helper called from the attack-in-place branch. Either way the wire message is the
same `0x10` we already send — direction bytes are always present on this variant, so **a turn-in-place
costs nothing new on the wire**. **Whether an arc gate should exist, and its width, is
npc-ai-spawn-advisor's call**; I'm asserting only that the server must be *able* to re-face.

---

## A3. Air-walking — we only ever send `FullPos`. **CONFIRMED**

`crates/services/src/mercury/aoi/mod.rs:38` hardcodes
`BASEMSG_UPDATE_AVATAR_NO_ALIAS_FULL_POS_YPR = 0x10`; built at
`crates/services/src/mercury/aoi/update.rs:34-47` and `create.rs:101-104`.

Per `docs/drafts/spec/position-updates.md:119-127` and
`docs/reverse-engineering/findings/position-movement-wire-formats.md:117-126`, the variant index
bits[3:2] select the position type, and the three types are **byte-identical on the wire** — only the
client handler differs:

- `FullPos` `FUN_00ddb0c0` — reads wire Y as-is
- `OnChunk` `FUN_00ddb220` — **discards wire Y** (FLT_MAX sentinel), uses the chunk height map
- `OnGround` `FUN_00ddb830` — **discards wire Y**, uses a terrain ray-cast

We send FullPos, so the client renders our Y exactly and cannot correct us. Reinforcing:
`ABigWorldEntity` disables UE3 collision (`CollisionResponseFlags = 0xFFFFC004`,
`docs/reverse-engineering/findings/entity-creation-wire-formats.md:640`), and neither
`BWAvatarFilter::Output (0x00e824f0)` nor `USGWAvatarFilter::Output (0x00e81dc0)` does a ground query
(`docs/protocol/position-updates.md:381-425`).

**Switching the NPC broadcast to the OnGround variant (`0x18`) is a one-byte change** — identical
layout — that hands grounding to the client's own terrain ray-cast. It fixes the A0 sawtooth, the A5
straight-line climb, and the A6 spawn float **simultaneously and in every world, including the
meshless ones**. This is by far the best impact-per-risk fix available.

Caveats: needs one in-game UAT to confirm it grounds rather than sinks (the sentinel path is
documented but we have never exercised it); apply to NPCs only, leave players on FullPos; do not
apply to any flying/swimming NPC. The `physics` byte is separately hardcoded `0x01`
(`update.rs:43`, pinned by `aoi/tests.rs:207`) and the PHYS_* value table is **undocumented** — see C4.

**Server-side ground truth is also unwired.** `SpaceManager::get_navmesh_height`
(`crates/services/src/cell/space_manager/spatial.rs:71-76`) exists with **zero production callers**
(only `crates/entity/src/navigation/tests.rs:480`). Wiring it into the tick is the belt to A3's
braces, and the only fix that corrects the server's *own* notion of where the NPC is (which matters
for LoS, range and AoI).

Vertical axis note: **Y is vertical** in SGW/BigWorld (`docs/engine/cme-framework.md:491`: BW Y-up,
UE3 Z-up, `UE3_Z = BW_Y * 100`). Generic "validate Z" guidance maps to **Y** here;
`docs/architecture/movement-validation.md:41` calls it "Z", which contradicts the navmesh code.

---

## A4. Through walls and floors — Castle has no navmesh. **CONFIRMED in telemetry**

- `2026-09-18T23:49:37.821407416Z` — `"No navmesh for space (optional)"`, `world = "Castle"`,
  `path = "data/spaces/castle.nav"`, `error = "No such file or directory"`, `space_id = 65537`,
  severity **DEBUG**
- `2026-09-18T23:59:12.048101063Z` — `"NavMesh loaded for space"`, `world = "Castle_CellBlock"`,
  `polys = 1479`, `space_id = 65552`, INFO
- Also loaded: `Harset` (19345), `Agnos` (4062). Also missing: SandBox, Lucia, Omega_Site, Tollana,
  Agnos_Library, Sewer_Falls, Harset_CmdCenter, Dakara_E1, Ihpet_Crater_*, Menfa_*, Beta_Site_Evo_1.
- All shipped meshes carry `agent_height = 0.6`, `agent_radius = 0.6`.

`data/spaces/` holds only `agnos.nav`, `castle_cellblock.nav`, `harset.nav`, `harset_storagerm.nav`,
`sgc_w1.nav`. **No `castle.nav`.**

Castle_CellBlock (~23:55–00:23 UTC) is meshed; Castle (~00:23–01:03 UTC) is not — and Castle is
exactly where Dr. Zerutska "came thru walls and floors" (00:26–00:29 UTC).

`SpaceManager::find_path` (`spatial.rs:41-51`) returns `None` at `space.navmesh.as_ref()?`. Callers
then push a **raw, unvalidated waypoint**:

- `cell/service/npc_ai/follow.rs:99-111` — `find_path(...).unwrap_or_default()`; when `len() <= 1`
  it pushes `dest`, computed at `:88-98` by linear interpolation along the NPC→target vector **in all
  three axes**. `dest.y = npc_pos.y + (target.y - npc.y) * scale` — **the follower's Y is pulled
  toward the *player's* Y every leg.** Since a player's reported Y is their own capsule origin (and
  the tester was a GM with 26 off-navmesh bypasses), the follower converges upward leg by leg. Leg 1
  moves a fraction; leg 2 more. **This is the "second leg goes up" in Castle.**
- `cell/service/npc_ai/fight.rs:451-452` — `min_range_backup` pushes a raw `compute_backup_waypoint`
- `cell/service/npc_ai/investigate.rs:132` — same shape

**Important negative:** `NavMesh::find_path`'s failure logs (`navigation/mod.rs:647`, `:664`, `:684`)
fired **zero times** in the session. So the fallback did **not** fire in the meshed Cellblock. In
Castle it cannot even log — `spatial.rs:49` returns `None` before reaching `NavMesh::find_path`, so
**the Castle fallback is live and completely invisible by construction.**

**Fixes:** (1) promote the missing-mesh log (`space_manager/lifecycle.rs:36-39`) from DEBUG to WARN;
(2) log the straight-line fallback with its vertical delta; (3) generate `castle.nav`
(`crates/navmesh-extractor/` exists — see C1); (4) clamp the fallback waypoint's Y to the NPC's
current Y so a follower can never climb.

---

## A5. Leash return — a bare field write. **CONFIRMED — answers the owner's point 4**

`crates/services/src/cell/service/npc_ai/leash.rs:48-50`:

```rust
if let (None, Some(spawn_pos)) = (npc.follow_target_id, npc.spawn_position) {
    npc.position = spawn_pos;
}
```

This is a **direct field write that bypasses `write_position` entirely**. Four consequences, all
matching "on the way home they show the same defects":

1. **The AoI spatial grid is not updated.** `write_position` calls
   `space.grid.update_position(...)` (`space_manager/entities.rs:462-466`); this doesn't. The grid
   keeps indexing the NPC in its old cell until something else moves it. **This is a position-write
   authority violation in my domain** — every position mutation must go through the writer — and an
   AoI correctness bug for **aoi-witness-broadcast**.
2. **`velocity` is never zeroed.** The NPC keeps its last chase velocity, and the AoI tick keeps
   broadcasting it. `USGWAvatarFilter::Output` does `position = lastPosition + velocity * dt`, so the
   client **extrapolates the leashed NPC along its old chase velocity** — it drifts away from spawn,
   off the geometry. That is the "not stuck to geometry on the way home."
3. **`direction` is never touched.** No re-face to the spawn heading; per A2 nothing else can re-face
   it either. It keeps its (often north, per A1) chase yaw. That is the "facing wrong way on the way
   home."
4. **It is a snap, not a walk** — the doc comment at `:10-11` says so ("In a full implementation this
   would pathfind the NPC back to spawn"). What the owner perceives as "walking home" is the client's
   AvatarFilter lerping across the snap gap (a short leash won't exceed the 2500 u/s teleport
   threshold) while extrapolating on stale velocity. **The walk home is entirely client-side
   interpolation of a teleport, which is why it ignores geometry.**

**Minimum fix:** route it through `update_position_preserving_facing` (or a variant that also sets
the spawn heading) with `velocity = [0,0,0]`. That fixes the grid desync, the drift, and gives a
clean stop — without building the full walk-back.

**Why there were zero leash logs:** `leash.rs:67` is a bare `tracing::info!` with **no `target:`**, so
it lands on the module path `cimmeria_services::cell::service::npc_ai::leash`, not on `npc_ai` or
`movement.npc`. It is caught only by the blanket `cimmeria_services=debug` in the OTLP filter
(`crates/server/src/logging.rs:307`) and carries **no position, no spawn position, no reason, no
distance** — so even when it fires it is unqueryable. See B7.

---

## A6. Spawn float — "levitating, then he came down". **CONFIRMED mechanism**

An NPC with an empty `nav_path` is never touched by the movement tick (`npc_movement.rs:41-47`), so
it sits at whatever the spawner wrote — the **authored row Y** — and the AoI tick broadcasts that Y
at FullPos every 100 ms forever. Nothing grounds a stationary NPC at any point in its life.

The instant a follow path is computed, `new_y` begins lerping toward the first waypoint's Y
(`:152`) and it descends over several ticks. Precisely "levitating but following" → "he came down"
(00:26–00:29 UTC).

Same class as my existing memory note `arrival-coordinate-offnavmesh`: **authored coordinates are
model origins, not standing positions** — true of NPC spawn rows as much as gate rows. A3's OnGround
switch fixes the visual immediately; snapping spawn Y via `get_navmesh_height` fixes the server's
truth, but only in meshed worlds.

---

## A7. What I looked for and ruled out

The owner's second-leg framing pointed at path-replacement bugs. I checked each and found them clean,
which is worth recording so nobody re-investigates:

- **Stale start position on re-path** — `fight.rs:390` reads `npc_pos` fresh each AI tick. Clean.
- **Stale segment index** — there is no index; `nav_path` is a `VecDeque` consumed by `pop_front()`
  (`npc_movement.rs:120`). No index to go stale.
- **Yaw/velocity derived from the previous segment** — the waypoint-reached branch reads
  `nav_path.get(1)` *before* `pop_front()` (`:82-88`), so the look-ahead targets the correct next
  corner. Clean.
- **Mid-path replacement while a path is active** — `follow.rs:80-84` returns early unless
  `nav_path` is empty, so follow legs are strictly sequential. `fight.rs:381-392` *does* replace
  mid-path, but only when the target has moved >5 u from the path's last waypoint, and it rebuilds
  from the current position. Clean.
- **First leg on a different code path from later legs** — no. Both go through the same
  `needs_repath` → `find_path` → `skip(1)` sequence (`fight.rs:381-403`). The `skip(1)` correctly
  drops Detour's corner 0, which is the start position (`navigation/mod.rs:688-694`).
- **Start-poly lookup failing after Y drift** — plausible on paper (`START_EXTENTS = [0.5, 0.5, 0.5]`
  at `navigation/mod.rs:41` is a thin ±0.5 vertical box, and A0's drift is 0.15–0.18 per leg), but
  **zero `no start poly` warnings fired this session**, so it did not happen. Flagging as a *latent*
  risk: the margin is under 3× the measured drift, and a slope or a leash snap could exceed it.

**Two hypotheses I raised earlier and am now downgrading:**

- `min_range_backup` (`fight.rs:441-470`) walks the NPC *away* from the target via
  `compute_backup_waypoint` (`ability_select.rs:92-110`), facing it along its direction of travel —
  i.e. away from the player. That would be a textbook "walks directly backwards facing backwards."
  **But it logged zero times this session** (decision outcomes were `attack_in_place` 123, `chase` 25,
  `stationary_holds` 17 — no `min_range_backup`), so it is not tonight's cause; most templates
  presumably have `min_range = 0`. **It remains a latent landmine** — it pushes an unvalidated single
  waypoint whose Y is taken from `target_pos.y` (`ability_select.rs:107`), so the first NPC given a
  real `min_range` will snap its Y onto the player's. Worth fixing pre-emptively.
- Cover-exit as the moonwalk trigger — `cover_released_flanked` also logged zero times. The
  animation-decoupling mechanism (A9) is real and confirmed by inspection, but the 00:03 UTC sighting
  cannot be pinned to a cover release from the logs.

---

## A8. Ordering hazard and a units mismatch

`npc_movement.rs:113-118` and `:184` call `update_entity_position(npc_id, pos, [0,0,0], velocity)`,
which per its own doc (`space_manager/entities.rs:391-396`) writes `direction` **unconditionally** —
zeroing facing to north — and the tick then overwrites it at `:121` / `:186-188`. Benign today only
because `write_position` doesn't broadcast; it is a re-entrancy trap for any future change that
broadcasts inside the write, or any early return between the two statements. Pass the real angle in
one call.

**Units mismatch to preserve when fixing P49:** NPCs store `direction.y` as true **radians**
(`:121`, `:187`); players store it as **packed-byte-cast-to-f32** (A1). One field, two unit systems,
one `pack_angle` consumer. Fixing the player path must not regress the NPC path.

---

## A9. Moonwalk — animation and translation are separate channels

The client picks mob animation from an explicit `setMovementType` byte, **not** from velocity and not
from position deltas: `crates/entity/src/cell_entity/mod.rs:180-190` — "The client uses this purely
for animation selection … Confirmed by Ghidra: `FUN_00deb660` switches on this byte." Enum at
`:197-205` (`Cover=0, CombatAdvance=1, Patrol=2, Follow=3, Wander=4, Leash=5, Avoid=6`); client FSM
jump table at `0x00dec018` (`docs/reverse-engineering/findings/npc-movement-pathfinding.md:60-72`).

Nothing couples the two channels:

- `npc_movement_tick` never calls `broadcast_movement_type` — only AI-state handlers do.
- `broadcast_movement_type` (`crates/services/src/cell/abilities/messaging.rs:201-249`) dedups on
  `last_movement_type` (`:226-231`) and, for `kind = None`, clears the cache and **sends nothing**
  (`:235-239`) — the client keeps its previous animation.
- The AoI tick emits `EntityMoved` for **every** entity every tick, ungated on a position delta
  (`crates/services/src/cell/space_manager/aoi.rs:217-230`).

An NPC that translates while the client holds a stationary pose glides with planted feet. Already a
documented gap: `npc-movement-pathfinding.md:150-158`. **Fix:** broadcast movement type on path
start/stop from the movement tick (one byte, only on transitions — the dedup makes it cheap).
**Which enum value per AI state → npc-ai-spawn-advisor.**

---

## A10. Speed warnings — the window is wrong; don't calibrate yet

756 `movement.speed_warning` + 26 `navmesh_gm_bypass` in 100 minutes, both warn-only. Sample rows
(entity_id 2, player_id 71, space_id 65552 = Castle_CellBlock, ~00:04 UTC):

| time (UTC) | distance | dt_secs | implied_speed | top_speed | ratio |
|---|---|---|---|---|---|
| 00:04:03.066 | 0.582 | 0.045 | 12.956 | 8.125 | 1.595 |
| 00:04:03.636 | 0.491 | 0.035 | 14.002 | 8.125 | 1.723 |
| 00:04:38.861 | 0.965 | 0.060 | 16.068 | 8.125 | 1.978 |
| 00:04:40.076 | 0.686 | 0.050 | 13.732 | 8.125 | 1.690 |

1. **`dt_secs` is inter-packet wall-clock, not a game-tick delta.** 0.035–0.060 s means the client
   sends at ~17–29 Hz while our tick is 100 ms. Dividing ~0.5 u by ~0.04 s amplifies every
   quantization artifact. The guard at `crates/entity/src/movement_validation/mod.rs:308`
   (`if dt_secs > 1e-4`) only stops a literal divide-by-zero. Degenerate windows are reaching the
   metric: p50 `implied_speed` came back **`Inf`** for the `top_speed = 4.0625` cohort and **`NaN`**
   for the `32.5` and `24.375` cohorts. **Fix: accumulate distance over a fixed game-tick window
   (3–5 ticks) and evaluate once per window.** Also note `mod.rs:195` assumes a "~10 Hz client update
   rate"; measured is 2–3× that, so `MAX_SNAP_BACK_CORRECTIONS = 5` covers ~0.2 s, not ~0.5 s.
2. **The 8.125 baseline is probably low.** Ratios cluster tightly at 1.6–2.0 rather than scattering —
   the signature of a systematically low baseline. `mod.rs:160-168` already flags that
   `worlds.sql run_speed = 8.125` conflicts with `runSpeed = 6.0` in `mercury/world_data`.

**Do not tune the tolerance until (1) is fixed** — calibrating a multiplier against an unstable
estimator just hides the instability.

The 26 GM bypasses raise a question for the AI layer: an NPC chasing an off-mesh GM is pathing to an
unreachable target. **Target-selection half → npc-ai-spawn-advisor**; the kinematic half is A4.

---

# B. Movement logging design for future playtests

The owner's ask: per NPC movement decision/leg — where it is, where it is going, where it is facing
(yaw **as sent on the wire**), what it is following/chasing, plus whatever else makes the next
session diagnosable from logs alone. This is a concrete spec, consistent with
`docs/architecture/observability.md` and `docs/architecture/instrumentation-discipline.md`.

## B0. The organising idea: a leg id

Every current event is a disconnected point. Introduce **`leg_id`** — a per-NPC monotonic counter
incremented every time `nav_path` is assigned — and stamp it on **every** movement event. That single
field turns the stream into joinable legs and makes "the second leg is the broken one" a `WHERE
leg_seq = 2` query instead of an eyeball exercise. Carry `leg_seq` (the leg's ordinal within the
current AI engagement) alongside it.

Add `npc_name` / `template_name` to every movement event — neither appears on any row in tonight's
window, which is why we cannot identify Dr. Zerutska's entity id (C6).

## B1. `movement.npc` / `event = "leg_start"` — **new, INFO, unsampled**

The single most valuable addition. One row per path assignment; low volume (124 waypoints → maybe
40 legs in the whole session).

| Field | Why |
|---|---|
| `npc_id`, `npc_name`, `template_name` | identify the actor |
| `leg_id`, `leg_seq` | join key (B0) |
| `reason` | `aggro` \| `chase_repath` \| `follow_out_of_band` \| `patrol_next` \| `wander` \| `investigate` \| `min_range_backup` \| `cover_move` \| `leash` — **why this leg exists** |
| `from_x/y/z` | where the NPC is |
| `to_x/y/z` | where it is going |
| `target_id`, `target_kind`, `target_name` | **what it is chasing/following** |
| `target_x/y/z` | the raw target position the destination was derived from |
| `path_source` | `navmesh` \| `straightline_fallback` \| `no_mesh` — **the single highest-signal field in this spec** |
| `corner_count` | path complexity |
| `corners` | compact `[[x,y,z],…]`, capped at 16 — lets us replay the leg offline |
| `start_poly_ref`, `end_poly_ref` | `0` means the Detour lookup failed |
| `dy_total` | `to_y - from_y`; a large value on a `straightline_fallback` **is** the air-climb signature |
| `ground_y_at_from`, `ground_y_at_to` | from `get_navmesh_height`; `null` = no mesh |
| `y_error_at_from` | `from_y - ground_y_at_from` — **directly measures A0's sawtooth** |
| `move_speed`, `speed_mod` | effective pace |
| `world_name`, `space_id` | scope |

Emit in `npc_movement_tick`? No — emit at the **assignment sites**: `follow.rs:102-111`,
`fight.rs:394-403` and `:451-452`, `investigate.rs:126-132`, `patrol.rs`, `wander.rs`. A shared
`log_leg_start(...)` helper keeps the field set identical across all six.

## B2. `movement.npc` / `event = "step"` — **extend the existing event**

Add: `leg_id`, `leg_seq`, `yaw_rad`, **`yaw_byte`** (post-`pack_angle` — the value actually on the
wire), `vx/vy/vz`, `ground_y` (`get_navmesh_height`, `null` if no mesh), `y_error`
(`new_y - ground_y`), `y_source` (`lerp` \| `waypoint_snap` \| `navmesh_query`), `movement_type_sent`,
`wp_index`, `wp_remaining`.

**`yaw_byte` alone would have made A1 self-evident** — every negative-bearing NPC logging
`yaw_byte = 0`.

**And fix the sampler.** `npc_movement.rs:171` gates on
`npc_id.is_multiple_of(NPC_STEP_LOG_SAMPLE)` — that is **10% of NPCs, 100% of their steps**,
permanently and deterministically, not "~10% of step events" as the comment at `:8` claims. An NPC
whose id is not ≡0 mod 10 is **never observable**. That is why 124 waypoint events produced only 40
step events, all from ids 100170 and 100140 — **the moonwalking Cellblock guard is almost certainly
in the 90% that can never be sampled.** Replace with a per-event counter, plus an operator-settable
`CIMMERIA_TRACE_NPC_ID` that unsamples one specific NPC for targeted debugging.

## B3. `movement.npc` / `event = "leg_end"` — **new, DEBUG, unsampled**

Fires on `path_complete`. `leg_id`, `npc_id`, `end_x/y/z`, `ground_y`, `y_error`, `final_yaw_rad`,
`final_yaw_byte`, `ticks_elapsed`, `distance_travelled`, `distance_planned`,
`stop_update_sent` (bool), `outcome` (`arrived` \| `repathed` \| `interrupted` \| `target_lost`).
`y_error` here vs. `y_error_at_from` in B1 **quantifies the sawtooth per leg**.

## B4. `movement.navmesh` — **new target, WARN**

- `event = "mesh_missing"`, **WARN**, once per space creation: `world`, `space_id`, `path`. Promoted
  from the current DEBUG "optional" at `space_manager/lifecycle.rs:36-39`. A world whose NPCs path
  blind is not optional.
- `event = "straightline_fallback"`, **WARN**, rate-limited per NPC: `npc_id`, `world`, `reason`
  (`no_mesh` \| `no_start_poly` \| `no_end_poly` \| `no_poly_path`), `from_*`, `to_*`, `dy`.
  Castle's fallback is currently invisible **by construction** — `spatial.rs:49` returns `None`
  before `NavMesh::find_path` can log anything.
- `event = "spawn_grounding"`, DEBUG, at NPC spawn: `authored_y`, `ground_y`, `snap_dy`. A6 becomes a
  one-query answer.

## B5. `movement.movement_type` — **new target, DEBUG**

`broadcast_movement_type` (`abilities/messaging.rs:201-249`) has **three silent outcomes**:
player-guard rejection (`:212-220`), dedup suppression (`:226-231`), `kind = None` no-send
(`:235-239`). Per `docs/architecture/negative-logging-convention.md` all three are expectation seams.
Fields: `npc_id`, `kind`, `prior_kind`, `outcome` (`sent` \| `deduped` \| `cleared` \|
`rejected_player`), `witness_count`, `leg_id`. **This is the seam that would have converted A9 from
inference to fact.**

## B6. `wire.out` coverage for `0x10` — **new, DEBUG, 1-in-100**

UPDATE_AVATAR is emitted UNRELIABLE from
`crates/services/src/base/world_entry/cell_dispatch/aoi.rs:407-409` and never reaches `wire.out`, so
we cannot prove what any witness was told. Add `entity_id`, `witness_id`, `msg_id`, `pos_*`,
`yaw_byte`, `physics_byte`, `leg_id`. `physics_byte` matters because it is hardcoded `0x01`
(`update.rs:43`) and the PHYS_* table is undocumented (C4).

## B7. Leash — **give it a target and fields**

`leash.rs:67` is a bare `tracing::info!` with no `target:` and no structured fields. Make it
`target: "movement.npc"`, `event = "leash"`, with `npc_id`, `from_x/y/z`, `spawn_x/y/z`,
`snap_distance`, `was_follower` (bool — the `follow_target_id` branch at `:48`),
`velocity_zeroed` (bool), `facing_restored` (bool). Tonight's session produced **zero** leash rows
and we cannot tell whether that means leash never fired or that it fired unqueryably.

## B8. Speed validation

Add `window_ticks`, `window_distance`, `sample_count` beside the existing `distance` / `dt_secs` /
`implied_speed` at `crates/services/src/cell/space_manager/client_move.rs:328-345`, and **drop
degenerate windows** (`dt < one tick`) rather than emitting `Inf`/`NaN` into the metric.

## B9. One dashboard query to rule them all

With B0–B3 in place, the next playtest is diagnosable with:

```
scope_name = 'movement.npc' AND leg_seq >= 2 AND abs(y_error) > 0.1
```

…grouped by `path_source` and `reason`. That one query separates A0 (sawtooth, `path_source=navmesh`,
`y_error ≈ 0.15`) from A4 (`path_source=straightline_fallback`, `dy_total` large) without a single
screenshot.

---

# C. Questions only the owner / playtester can answer

**C1. Was `castle.nav` ever generated, and is its absence deliberate?** `crates/navmesh-extractor/`
exists and four other worlds have meshes. If Castle was skipped for a reason (geometry size,
extractor failure), that reason decides whether A4's fix is "run the extractor" or something harder.
**Highest-value question here** — it gates the single biggest root cause.

**C2. Does the bad behaviour reproduce in Harset or Agnos** (both meshed) **as well as in Castle?**
If yes, A0/A1/A2 dominate and are navmesh-independent. If it is markedly worse in Castle, A4
dominates. Cleanly separates them with no instrumentation.

**C3. When an NPC "stops attacking while still holding threat", does it resume if you walk back
around to its front?** If yes, that confirms a facing gate acting on the frozen yaw (A2) and makes
the turn-in-place fix the priority. If it never resumes, something else is latching.

**C4. Do you have (or can you take) a 2009-client wire capture of an NPC walking?** It would settle
the PHYS_* byte, which we hardcode to `0x01` with no documented value table
(`docs/drafts/spec/position-updates.md:173`). A wrong physics mode is an alternative explanation for
no floor-snap. This is the one genuine **RE task** I'd raise: chase `FUN_00ddb830` (the OnGround
handler) and the `sentPhysics_` comparison in the `0x31` handler in Ghidra.

**C5. Roughly how far off the ground is a floating NPC — inches or body-heights?** A0 predicts
~0.15–0.18 world units (small, a visible "not quite planted"); A4 predicts metres (clearly airborne).
The owner's answer immediately tells us which dominates in a given world.

**C6. Which entity was Dr. Zerutska?** Movement events carry `npc_id` but no name in this build, so
we cannot pull her exact path. B0 fixes this going forward.

**C7. Screenshots / clips that would materially help:**
- **00:03 UTC** — the moonwalking guard. Is the body facing *screen-absolute north* while
  translating? That distinguishes A1 (yaw snapped north) from A9 (right facing, wrong animation). A
  short clip beats a still.
- **00:13 UTC** — "enemies walk straight to me facing backwards". **If several NPCs approaching from
  different bearings all face the same absolute direction, A1 is confirmed visually with zero
  instrumentation.** This is the cheapest confirmation available.
- **00:26–00:29 UTC** — Dr. Zerutska levitating, with the map debug HUD visible. Gives follower Y and
  player Y at the same instant — the direct float measurement the logs can't provide.

---

# D. Recommended fix order

Ranked by (impact × confidence) / cost:

1. **`pack_angle` wrap** (A1) — one line, unit-testable, fixes half of all NPC facings and unblocks
   the player-facing P49 fix. No client risk.
2. **Switch NPC broadcast to the OnGround variant `0x18`** (A3) — one byte, identical wire layout.
   Fixes the A0 sawtooth, the A4 climb and the A6 spawn float at once, in every world including
   meshless ones. One in-game UAT to confirm it grounds rather than sinks.
3. **Turn-in-place / re-face for attacking NPCs** (A2) — no new wire message; direction bytes are
   already on every `0x10`. Likely the biggest *combat-feel* win, given `attack_in_place` was 123 of
   165 decisions.
4. **Leash through the real position writer, with zeroed velocity + restored facing** (A5) — small,
   and it also closes an AoI grid-desync bug.
5. **Logging: B0 (`leg_id`), B1 (`leg_start` with `path_source`), B2 (`yaw_byte` + sampler fix),
   B4 (mesh-missing WARN)** — do these before the next playtest or the next session is equally
   unfalsifiable.
6. **Generate `castle.nav`** (A4) — the real fix for through-walls; gated on C1.
7. **Clamp the straight-line fallback's Y** (A4) and **pre-emptively fix `min_range_backup`'s Y
   source** (A7) — cheap insurance on latent landmines.
8. **Couple `setMovementType` to path start/stop** (A9) — needs npc-ai-spawn-advisor on the mapping.
9. **Window the speed validator** (A10) — then recalibrate, not before.
10. **Wire `get_navmesh_height` into the movement tick** (A3) — corrects the server's own truth
    (matters for LoS/range/AoI); lower priority once (2) makes the client ground itself.

# E. Explicitly not my call

- **AoI fan-out shape**, the ungated per-tick `EntityMoved` (`aoi.rs:217-230`), and the **leash grid
  desync** (A5.1) → **aoi-witness-broadcast**.
- **Whether a facing-arc gate should exist and how wide**, which `MobMovementType` each AI state
  broadcasts, cover selection, and target selection when the target is off-mesh →
  **npc-ai-spawn-advisor**. I assert only that the server must be *able* to re-face (A2).
- **PHYS_* enum recovery from the client binary** → **game-archaeology-specialist**, scoped as C4.
- **Whether `runSpeed` 6.0 or 8.125 is client-authoritative** → touches `mercury/world_data`;
  **bigworld-engine-advisor**.
