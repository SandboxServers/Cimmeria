# Movement Validation (server-authoritative position)

> **Status**: Shipped in issue **#478** (CAT-B-01 + CAT-B-06 + CAT-B-09),
> building on the bounds layer from #437 (PR1 of 4). Applies to every
> inbound client position update (`AVATAR_UPDATE_EXPLICIT`, system message
> `0x03`).

## Context

The SGW client is *client-authoritative* for its own avatar position: it
streams raw `f32` world coordinates in `AVATAR_UPDATE_EXPLICIT` (0x03) at
~10 Hz and expects the server to mirror them into the cell entity. Before
issue #478, the cell wrote those coordinates with **zero validation** —
every per-tick update was a free teleport. Every position-derived system (AoI /
witness scope, region triggers, mission gates, threat radius, navmesh
distance) reads from the cell entity's `position`, so a single tampered
0x03 corrupted all of them downstream. See
[CAT-B-movement.md](../security-audit/2026-05-31-server-authority/findings/CAT-B-movement.md)
(CAT-B-01, -06, -09).

## Decision

A single validation seam,
`SpaceManager::apply_client_position_update`, gates **every** inbound
client position. It is the only path the `EntityMove` handler calls;
server-authoritative writers are the source of truth for those entities
and never go through the validator, but they split on whether the move
should reorient the entity. Respawn (player and NPC) and NPC movement
have a real facing to set — a spawn-defined heading or the direction of
travel — and call the unchecked `update_entity_position` directly, which
writes `direction` verbatim from its `[i8; 3]` parameter. Ring transport,
content-engine teleport (including `MoveWaypoint`), and GM travel are
pure relocations with no orientation change, so they call
`SpaceManager::update_position_preserving_facing` — a position-only
writer that never touches `direction` at all, making facing survive by
construction instead of by each caller capturing and restoring it.

The validator (`cimmeria_entity::movement_validation::MovementValidator`)
runs five layers. The table is in **execution order** as wired in
`apply_client_position_update_at` — cheapest spatial gate first, then the
stateful checks. Four layers hard-reject (snap-back); the speed layer is
**warn-only** until calibrated. Speed (`observe_speed`, a per-entity
window) and teleport (`check_kinematics`, one packet) are separate calls
with different actions.

| Order | Layer | Action | Catches |
|-------|-------|--------|---------|
| 0a | **Content hold** (`check_movement_hold`) | reject (`Locked`) | a player pinned by a `movement_lock` hold moving more than 1.0 m horizontally from its fixed anchor. Runs **before** row 0, spends no correction budget and is not counted in `movement_validation_rejects_total` - see [Content movement hold](#content-movement-hold) |
| 0 | **Minigame hold** (`movement_lock::check`) | reject | a player held by `BSF_PLAYING_MINIGAME` moving more than 0.5 m horizontally from its hold anchor (where the hold found it; reset by an authorized teleport), so small steps cannot add up (`reason=movement_locked`); game masters exempt; a stun's lock alone is not gated |
| 1 | **Bounds** (`check_bounds`) | reject | NaN / ±∞ / absurd coords, **Z-axis floor-clip** (full X/Y/**Z** AABB test) |
| 2 | **Navmesh** (`is_position_valid`) | reject | off-walkable-polygon (walls, under-terrain, ceilings); fail-open when no navmesh loaded **or when the world is seeded `navmesh_mode = 'advisory'`** — see [navmesh-containment-modes.md](navmesh-containment-modes.md). Horizontal (X/Z) containment is tight (`agent_radius`-based) in both directions; vertical (Y) containment is asymmetric — up to `JUMP_HEIGHT_TOLERANCE` (4.0 units, physics-derived) *above* the surface so a legitimate jump apex isn't rejected, but only `agent_radius * 2.0` *below* it — see "Jump-height fix" below. |
| 3 | **Speed** (`observe_speed`) | **warn-only** | sustained over-tolerance velocity: over a per-entity sliding window, `sum(distance) / max(sum(dt), 250 ms) > top_speed × 1.5` (see "The speed window" below) |
| 4 | **Teleport** (`check_kinematics`) | reject | single update both `> 50 u` **and** `> top_speed × 10` (or, on the first packet with no time baseline, `> 50 u` from the authoritative spawn) |

`top_speed` in rows 3 and 4 is **not** the flat `DEFAULT_TOP_SPEED`: it is
that constant scaled by the entity's own `movementSpeedMod` stat
(`CellEntity::stats.movement_speed_scale()`), the same stat the NPC
path-stepping tick scales by and the same one the client scales its local
prediction by (`cur / 100`) the moment it arrives in an `onStatUpdate`.
A GM `.speed 300` — or any future haste/snare effect writing that stat —
therefore raises the gate along with the movement the server itself
authorised, instead of warning on every packet of it. The
`movement.speed_warning` log's `top_speed` field reports the scaled value it
actually compared against.

Bounds AABB is sourced from the active space's navmesh `bmin`/`bmax`, or
`SpaceBounds::FALLBACK` (20 km × 12 km × 20 km) for navmesh-less spaces.

On reject the cell entity is **not** advanced; the handler emits
`CellToBaseMsg::TeleportPlayer { position == prev_pos == last_valid }`,
which composes `BASEMSG_FORCED_POSITION (0x31)` to snap the offending
client back. Because the cell entity never moved, the next 100 ms AoI
tick naturally rebroadcasts the last-valid position to witnesses — no
explicit AoI fan-out is needed. The structured negative log is
`target: "movement.validation"`, message `movement.validation_reject:`,
with a low-cardinality `reason` field (`bounds | navmesh | teleport | movement_locked`) and
the `movement_validation_rejects_total{reason, world, gate}` counter.

## What a reject reports

Moved to its own document:
**[movement-telemetry.md](movement-telemetry.md)**. It covers the reject
rows and which of them carry `world`, the navmesh containment
diagnosis, the `navmesh_hash` mesh fingerprint, the per-entity throttle
and where its state is released, the accepted-position sampler, and the
advisory-world gap. Nothing there changes a decision made here.

Two contracts from that document are load-bearing for the table above:

- **Every hard reject is counted and diagnosed**, whichever of the three
  outcomes in [Correction termination](#correction-termination-snap-back-recovery)
  it produced, through one seam
  (`SpaceManager::account_hard_reject`). `Recovered` and
  `CorrectionSuppressed` are not exceptions to
  `movement_validation_rejects_total`; their own counters are a
  breakdown of it.
- **Per-entity telemetry state is released on every teardown path**, not
  just `destroy_entity` — see
  [State release](movement-telemetry.md#state-release).

## Layer rationale

### Why the teleport gate is a dual gate (distance AND speed)

A pure distance threshold false-positives on a legitimately lagged
client that goes quiet for several seconds and then sends one large
catch-up packet (far, but slow). A pure speed threshold can't tell a
1-unit jitter at high implied speed from a real teleport. Requiring
**both** `distance > TELEPORT_JUMP_UNITS` and
`implied_speed > top_speed × TELEPORT_SPEED_FACTOR` rejects the
100m-in-50ms teleport (≈246× top speed) while passing the far-but-slow
catch-up. Sub-teleport-but-fast moves fall through to the warn-only speed
layer.

### Why speed is warn-only

The legitimate-traffic speed distribution under real RTT is unknown
ahead of telemetry. Snapping on a guessed tolerance would rubber-band
players on bad connections. The speed layer therefore **logs + counts but
accepts**, emitting the window's `(distance, dt_secs, implied_speed,
top_speed)` triple (window sums, see "The speed window") on
`movement.speed_warning` /
`movement_validation_warns_total{reason="speed"}`. Calibrate the
production tolerance from the SigNoz p99.9 of legitimate
`implied_speed/top_speed` (bucketed by RTT) before promoting it to
snap-back.

### The speed window

Speed is judged over a per-entity sliding window (#1249), not per packet.
The client's send timing jitters (packets 25 ms then 175 ms apart while it
runs at exactly top speed), so a single packet's `distance / dt` can read
as 4x top speed. Each accepted move that had a time baseline calls
`MovementValidator::observe_speed` with its distance and server-clock dt:

- The window keeps the newest samples that cover at least 250 ms
  (`SPEED_WINDOW`), at most 32 of them. Past 32 the oldest sample is
  folded into the next (distance and dt both), so the window totals are
  kept: a lag stall's dt is not lost to a burst of zero-dt packets that
  follows it.
- It warns when `sum(distance) / max(sum(dt), SPEED_WINDOW)` exceeds
  `top_speed x 1.5`. The 250 ms floor on the divisor stops a burst of
  near-zero-dt packets from reading as infinite speed.
- A move under 0.01 u (`MIN_SPEED_DISTANCE`) adds its dt to the window but
  never warns.
- The window is dropped on an authorized teleport, on `forget`, and on every packet accepted under the GM physics bypass (`movement_unrestricted`).

The `distance` and `dt_secs` on the `movement.speed_warning` row are the
window sums. The old `speed_bunched` DEBUG row is gone: the window absorbs
bunched packets, so there is nothing separate to report. `check_kinematics`
now only decides the teleport reject. The row also carries `window_secs = max(dt_secs, SPEED_WINDOW)`, the divisor the verdict used, so `implied_speed = distance / window_secs` (they differ in the first 250 ms after a spawn or teleport).

### Time source — server clock, not client timestamp

`dt` is measured from the server's own monotonic `std::time::Instant`,
sampled when the packet is processed — **never** a client-supplied
timestamp (which is spoofable: inflate `dt` → any distance looks slow).
The instant is injected into `apply_client_position_update_at` /
`check_kinematics` so the speed/teleport logic is deterministic under
test.

### Authorized teleports (no allowlist needed)

The kinematics layer measures `distance` against the entity's **current
authoritative position** — already advanced by any server-side teleport
via `update_entity_position`. So a legitimate ring/respawn/gate/content/GM
move can't produce a self-inflicted false reject: the next client packet
is measured from the destination, not the source. Each authoritative path
additionally calls `SpaceManager::note_authorized_teleport(entity_id)`,
which reseeds the per-entity clock so the first post-teleport packet's
`dt` is measured from the teleport instant, and drops the speed window and
any hold anchor (suppressing a spurious speed warn when an authoritative
move interrupts the client's stream). It does
**not** suppress hard rejects — a stale in-flight packet pointing at the
old location *should* snap to the new one. Paths are catalogued in
`.claude/agent-memory/movement-teleport-advisor/authorized-teleport-paths.md`.

### spaceId cross-check (CAT-B-06) is warn-only by design

The 0x03 payload's leading `spaceId` is parsed and forwarded as
`EntityMove::claimed_space_id`, but the write **never** uses it — the
authoritative space is the cell's own `entity_space` binding. A mismatch
therefore cannot corrupt the spatial grid, so the check is warn-only
(`movement.space_mismatch`, `reason="space_mismatch"`): it exists to make
gate-travel / instance-reset races observable, not to gate movement. A
claimed id of `0` is the pre-confirmation sentinel and is skipped.

## GM validator bypass (`onPhysics` / fly-ghost)

> Shipped alongside the `onPhysics` GM cell method (index 221 on
> `SGWGmPlayer`). See the "Test / loot / vision / cover (212–225)" table in
> [cell-method-dispatch-table.md](../protocol/cell-method-dispatch-table.md)
> for the wire shape.

`/gmsetfly` and `/gmsetghost` both route through the same client method,
`onPhysics(UINT8 bTurnOn)` — the client can't distinguish which slash
command triggered it and doesn't need to. The client also changes its own
pawn physics mode (gravity, collision) **locally and instantly** the
moment the GM types the command; the `onPhysics` wire send that follows is
a best-effort notification, not a gate on the client's own movement. Left
unhandled, a GM who is actually flying/ghosting client-side would still
have every position update run through the four layers above and get
rejected the instant they left the navmesh or exceeded ground speed —
there was no way to legitimately fly/ghost as a GM without tripping
validation.

The fix is a single per-entity bool, `CellEntity::movement_unrestricted`
(default `false`, in-memory only — never persisted, matching the client's
own no-save-across-sessions behavior). `console::gm::physics::handle_physics`
flips it on `onPhysics`, with **inverted wire polarity**: `bTurnOn=0`
(physics off, client is flying/ghosting) sets `movement_unrestricted =
true`; `bTurnOn=1` (physics restored) sets it back to `false`. The GM gets
a feedback line, and the cell an INFO row, only when the flag actually
changes: the GM client sends `onPhysics(1)` by itself at every world entry,
which used to print "movement validation restored" on every GM login (Class
Start v6 CS-08 F2). A no-change call logs DEBUG and stays silent.

`apply_client_position_update_at` checks the flag immediately after
resolving `bounds`/`last_valid` — before Layer 1 — and, if set, skips all
four rejection layers and calls `update_entity_position` directly,
returning `Accepted`. Three details matter for correctness:

- **The per-entity kinematics clock (`touch_clock`) still runs** on the
  bypass path, so `dt` stays fresh for when physics is restored — an
  unclocked bypass period would otherwise leave the next real check
  comparing against a stale, multi-minute-old sample.
- **`update_entity_position` still runs** on the bypass path (spatial
  grid and AoI source-of-truth), so `last_valid` keeps tracking the GM's
  actual position while flying. Skipping this would freeze `last_valid`
  at the position where flight began; the first client packet after
  physics is restored would then measure a huge apparent jump from that
  stale point and get rejected as a teleport, rubber-banding the GM back
  to wherever they started flying.
- **The `is_finite()` gate runs unconditionally, even under the bypass**
  — before the `movement_unrestricted` branch, not folded into the
  Layer-1 bounds check it would otherwise share code with. Bounds,
  navmesh, and kinematics are all skipped while unrestricted, but a
  non-finite (`NaN`/`±Infinity`) coordinate is rejected regardless.
  Without this, `update_entity_position` (which does no sanitization of
  its own) would write `NaN` straight into `cell_entity.position` while
  the GM is flying. That doesn't corrupt the spatial grid (float→int
  cell indexing saturates), but it poisons the entity's *own*
  `check_kinematics` state: `distance_to` against a `NaN` last-position
  is `NaN`, and every `NaN` comparison (including
  `distance > TELEPORT_JUMP_UNITS`) is `false` under IEEE754 — so the
  hard teleport-reject would silently and permanently stop firing for
  that entity the moment physics was restored, until the next
  disconnect clears its `CellEntity`. No legitimate fly/ghost movement
  needs a non-finite coordinate, so the reject is unconditional and
  costs nothing.

The bypass is scoped to the flagged entity only — every other entity's
`movement_unrestricted` defaults `false` and is validated exactly as
before. Regression guards (prefix `feat_onphysics_`) live in
`crates/cell-world/src/cell/space_manager/tests/movement_validation.rs`
(bypass accepts out-of-bounds / off-navmesh / teleport-shaped moves; the
default-false negative control still rejects; a NaN poisoning attempt is
rejected and the teleport gate keeps working afterward; two entities in
the same space with only one flagged prove the bypass doesn't leak to
the other) and `crates/cell-console/src/cell/console/gm/tests/physics.rs`
(polarity, feedback text, truncated-arg rejection without mutation).

## GM off-navmesh allowance (`access_level`)

`movement_unrestricted` above is an explicit, GM-toggled state. Separately,
the **navmesh containment layer alone** is warn-only for any caller whose
`CellEntity::access_level` is `GameMaster` or higher, with no toggle at
all: standing inside geometry, on a rooftop, or over a gap is how a GM
inspects a broken spawn or an unreachable region, and a GM who typed
`.gotoxyz` into an unwalkable spot should not be rubber-banded out of it.

The remaining layers stay enforced for GMs. Bounds still hard-rejects, so
a GM cannot write a NaN or an absurd coordinate into the spatial grid, and
the teleport gate still hard-rejects — the GM travel commands already call
`note_authorized_teleport`, so their own moves are unaffected.

The GM allowance is also the reason a partial navmesh stays invisible: only
ordinary players hit the holes, so no GM tester reports them. That is the
blind spot the per-world containment mode closes — the navmesh layer is now
gated on `resources.worlds.navmesh_mode`, and an `advisory` world skips
containment for *everyone* while keeping the mesh for pathing, line of sight
and height. See
[navmesh-containment-modes.md](navmesh-containment-modes.md).

`access_level` is read from the `account.accesslevel` column at login and
carried into the cell by `InitPlayerState`; it is never derived from a
client-supplied byte. This is the same trust model as
[`cell::dispatch::gm_gate`](../../crates/cell-world/src/cell/dispatch/gm_gate.rs)
and the `.`-console channel gate. The bypass emits a warn-level
`movement.navmesh_gm_bypass` event and a
`movement_validation_warns_total{reason="navmesh_gm_bypass"}` counter, so
it is visible rather than silent.

## Content movement hold

> Added by the cellblock-boot-lock campaign (BL-01..BL-06). Not yet exercised
> against a real client; see [what is and is not tested](#content-hold-test-status).

A content chain can pin a player to a fixed anchor with the `movement_lock`
verb ([params](../content/content-engine-vocabulary.md#movement_lock-params)).
The Cellblock boot lock is the first user: a new character cannot walk away
from the Prison Boot Control until they win Livewire. The hold is a
`MovementHold` stored in `CellEntity::extensions` and the gate is
`SpaceManager::check_movement_hold`
([client_move_lock.rs](../../crates/cell-world/src/cell/space_manager/client_move_lock.rs)),
called from `apply_client_position_update_at`.

**Where it sits.** The order inside `apply_client_position_update_at` is:

1. The non-finite reject (always).
2. The GM bypass (`movement_unrestricted`). `/gmsetfly` therefore frees a stuck
   player, and completing mission 689 releases the hold through chain 1048.
3. **The content hold.**
4. The server hold for a minigame or a movement-only effect (row 0 above).
5. Layers 1-4.

It sits before the layers on purpose: a held player never reaches the check
that spends the snap-back budget, so a player who keeps pushing against the
hold is never "suppressed" into silence and never recovered to a safe point.

| Rule | Value | Why |
|---|---|---|
| Slack | `HOLD_RADIUS` = 1.0 u, horizontal (x/z) from the anchor. A move inside falls through to the normal layers. | The player can turn and shuffle without being snapped. Vertical is ignored, so a jump or a fall is not held. |
| Outcome outside the slack | `ClientMoveOutcome::Locked { anchor, space_id, snap, feedback }`. No position is written, the snap-back budget is untouched. | The same reject-and-snap primitive as every other reject, without its bookkeeping. |
| Snap throttle | At most one snap per `HOLD_SNAP_INTERVAL` = 500 ms per player. | Each snap is a `TeleportPlayer { position == prev_pos == anchor }`, which the base turns into a database write. |
| Feedback throttle | The hold `message` repeats at most every `HOLD_FEEDBACK_INTERVAL` = 5 s. The line sent when the hold is taken counts as the first, so a first step does not repeat it at once. | Movement refusals need visible feedback, but a line per packet would flood chat. |
| Stale hold | The hold records its `space_id`. The first move in a different space removes it and falls through. | GM travel or a world change must not carry a hold into a space it was never taken in. |
| Persistence | None. A relog rebuilds the entity without the hold and the seed re-applies it. | The mission (689) is the persisted state, not the hold. |

**Telemetry.** A snap (not every refused packet) increments
`movement_hold_snaps_total{reason="content_hold"}` and writes one INFO row on
`target: "movement.validation"` with `event = "movement.hold_snap"`. The row
carries the player and account identity, `space_id`, `world`, the client's
refused `client_x` / `client_z`, the `anchor_x` / `anchor_z` and
`reason = "content_hold"`. A hold is a decision the content made, not a
rejection of bad input, so it is **not** one of the `reason` values of
`movement_validation_rejects_total` and has no `movement.validation_reject`
row. The throttled-away packets log nothing.

**Interaction with the Prison Boot effect.** Chain 1023 does not cast ability
1597 (BL-06 N1). Its only hold is the content hold. Ability 1597's effect 1939
runs the `MovementHold` effect script (MG-63b) and sets `BSF_MOVEMENT_LOCK`,
which row 0 gates with its own 0.5 m slack; that is Layer A, still off in every
seed until OQ-3. Were 1597 cast, both holds would be live during the boot lock:
a move beyond 1.0 m of the content anchor is `Locked` first, and a move between
the older anchor's 0.5 m and the content anchor's 1.0 m is refused by row 0 and
counted as `movement_locked`. The two gates have not been run together against
a client (see below).

### Content hold test status

| Covered by a test | Never run |
|---|---|
| The gate returns `Locked` outside 1.0 m and falls through inside; a stale-space hold is dropped; the throttles (`tests/movement_validation/locked.rs` in `cell-world`) | A real client walking away from the anchor and being snapped back |
| The `EntityMove` handler snaps to the anchor, says the line once, and sends nothing on an immediate second move (`base_messages/tests/movement_lock.rs` in `cell`) | Whether the 500 ms snap rate looks smooth or rubber-bands on the client |
| The `hold` and `release` executor, including the skips, the ref-count and an end-to-end gate check (`executor/tests/movement_lock.rs` in `cell-content`) | Both gates active at once (this hold plus the Prison Boot effect) |
| The seeded chains 1023, 1025, 1047, 1048 and the prop spawn against the live seed (`chain_replay_tests/mission_689_boot_lock.rs`, `live_db_boot_control_spawn.rs`) | The lab run (BL-U1) that is the ship gate for the campaign |

## Correction termination (snap-back recovery)

A rejection tells the offending client to snap back to `last_valid` — the
cell entity's current authoritative position. That only ends the exchange
if `last_valid` is a position the validator would itself accept. When it
is not, the client snaps to it, re-reports it, is rejected again, and
rubber-bands at its own update rate until the player disconnects.

The authoritative position can be unacceptable because **nothing validates
the server-authoritative write path**: `update_entity_position` is
deliberately unchecked so ring transport, respawn, content teleport, NPC
movement and the GM travel commands can place an entity anywhere. Observed
live: a GM ended up at `[0, 0, 0]` in Castle Cellblock (inside the space,
off the walkable mesh) and took ~12-15 `FORCED_POSITION` corrections per
second until they gave up. A stale persisted `sgw_player` row on
reconnect, or authored-but-unreachable content coordinates, reach the same
state for an ordinary player.

`SpaceManager::reject_outcome`
([client_move.rs](../../crates/cell-world/src/cell/space_manager/client_move.rs))
resolves every hard reject into one of three outcomes:

| Outcome | When | Caller action |
|---|---|---|
| `Rejected` | The snap target is in-bounds and on-navmesh, and the entity is within its correction budget. | Emit `FORCED_POSITION` to `last_valid` — the pre-existing behaviour. |
| `Recovered` | The snap target is itself unusable **and** a sound safe point exists. The entity has already been relocated there. | Emit `FORCED_POSITION` to `recovered_to`. |
| `CorrectionSuppressed` | Either the snap target is sound but the budget is spent, or it is unusable and no sound safe point exists. | Emit **nothing**. Re-sending is what produced the loop. |

An exhausted budget on a **sound** position is `CorrectionSuppressed`, never
`Recovered`. Relocating a player who never left a legal point would be a
server-initiated move they did not ask for — and because recovery clears the
budget, routing that case through `Recovered` made the budget unenforceable
on every navmesh-backed world: reprojecting an already-walkable point returns
a near-identical (but rarely bit-identical) point, which read as a successful
relocation and let the correction stream run forever.

The safe point is resolved in order of how little it disturbs the player:
the nearest walkable navmesh point (`NavMesh::get_nearest_point` — the
Z-clamp answer, so a player a metre inside the floor comes back out on the
surface they were standing on), then the world's nearest authored
respawner, then the space AABB clamped.

**Every candidate is tested against `position_within_bounds` and (where the
space has a mesh) `NavMesh::is_point_valid` before it is returned** — the
respawner and the clamp included. `load_respawners` copies its coordinates
out of `resources.respawners` unvalidated, and a clamp answers the bounds
layer by construction while saying nothing about walkability, so neither is
automatically a position the validator would accept. Returning one that isn't
writes an illegal position through *and* clears the correction budget, which
restarts the loop one position over with nothing left to spend. When nothing
passes, `None` → `CorrectionSuppressed` is the correct answer. Recovery writes through
`update_entity_position` and calls `note_authorized_teleport`, so the
spatial grid, the next AoI tick's witness broadcast, and the client all
agree, and the post-recovery client packet is measured from the relocation
instant rather than a stale clock sample.

`MAX_SNAP_BACK_CORRECTIONS` is the backstop that makes termination
unconditional: whatever the geometry, one entity cannot be corrected more
than that many times in a row. Any accepted position clears the count, so
the budget is per-incident and a legitimately lagged client is never
penalised across separate episodes. Suppression stops only the outbound
correction — the rejected position is still never written, so witnesses
continue to see the server's truth.

## Constants

Defined on `MovementValidator` (see source for full rationale):

| Constant | Value | Source / note |
|----------|-------|---------------|
| `DEFAULT_TOP_SPEED` | `8.125` u/s | `db/resources/Worlds/Seed/worlds.sql` `run_speed` (universal). **The per-entity baseline, not the gate itself**: the speed and teleport layers compare against `DEFAULT_TOP_SPEED × movement_speed_scale()`, so a hasted or GM-sped player is measured against their own top speed. Per-world sourcing + reconciling the `runSpeed = 6.0` drift in `mercury/world_data` is a follow-up; warn-only makes the single constant safe meanwhile. |
| `SPEED_WARN_TOLERANCE` | `1.5×` | warn threshold (warn-only) |
| `TELEPORT_JUMP_UNITS` | `50.0` u | teleport distance gate |
| `TELEPORT_SPEED_FACTOR` | `10×` | teleport implied-speed gate |
| `MAX_SNAP_BACK_CORRECTIONS` | `5` | consecutive corrections before the server stops re-issuing them (~0.5 s at the 10 Hz client update rate) |
| `HOLD_RADIUS` | `1.0` u | content hold: horizontal slack around the fixed anchor |
| `HOLD_SNAP_INTERVAL` | `500` ms | content hold: minimum gap between snap-backs |
| `HOLD_FEEDBACK_INTERVAL` | `5` s | content hold: minimum gap between repeats of the hold message |

The client's own hard-snap ceiling (`USGWAvatarFilter::Input`,
`_DAT_01e69c90 = 2500 u/s`) is the upper bound on what the client
smooths; the server gates sit far below it. See
`.claude/agent-memory/movement-teleport-advisor/movement-validation-anchors.md`.

## Consequences

- **Wire-format touch**: `EntityMove` gained a `claimed_space_id` field;
  the 0x03 parser now reads `payload[0..4]`. No client-visible change.
- **Per-entity state**: `MovementValidator` holds a `HashMap<u32,
  Instant>` clock, released via `forget` from `destroy_entity` **and**
  `destroy_space` (no leak / no stale sample across id reuse) — see
  [State release](movement-telemetry.md#state-release) for why the
  second path is not optional.
- **Regression guards** (issue #478 close criteria):
  - `teleport_100m_over_50ms_is_rejected_and_not_observed`
  - `off_navmesh_position_is_rejected_and_not_observed` (real
    `castle_cellblock.nav` fixture, self-skips on fixture-less CI)
  - plus speed-warn-accepts, authorized-teleport-follow-up,
    sustained-spam, and `entity_move_space_mismatch_warns_but_still_applies`.

## Jump-height fix

`NavMesh::is_point_valid` (`crates/entity/src/navigation/mod.rs`) originally
measured the raw 3D distance from a proposed position to the nearest walkable
polygon against `agent_radius * 2.0` (≈1.2 units on the `castle_cellblock`
fixture). Because the client is authoritative for jump physics and the server
never simulates it, a jump apex only slightly taller than that combined gate
already read as off-navmesh — so *every* client position packet sent while
airborne was rejected as `MovementReject::OffNavmesh`, snapping the player
back to `last_valid` via `TeleportPlayer` on each one. Visibly: standing-still
jumps always snapped the avatar's facing to north (`build_teleport_bundle`
zeroes direction on every snap), and jumping while moving produced a
backward/sideways rubber-band as the player kept getting snapped to a
several-packets-stale position.

The fix decouples horizontal from vertical containment, and makes the
vertical check asymmetric: X/Z uses the same tight `agent_radius`-based gate
as before in both directions; Y allows up to `JUMP_HEIGHT_TOLERANCE` (4.0
units) *above* the surface but only `agent_radius * 2.0` (unchanged from
before this fix) *below* it. There is no legitimate reason to be below a
walkable surface, so under-terrain clipping stays exactly as strict as
pre-fix — a single symmetric tolerance for both directions would have
widened the floor-clip allowance to match the much larger jump tolerance.

`JUMP_HEIGHT_TOLERANCE` is sized from the client's own jump physics, not
guessed: `build_world_params_args`
(`crates/wire/src/mercury/world_data/mod.rs`) hands the client
`gravity = -9.8` and `jumpSpeed = 8.0`, giving a ballistic apex of
`jumpSpeed² / (2 * |gravity|) ≈ 3.27` units above takeoff. `4.0` leaves
~0.7 units of margin for uneven ground, slope, and query jitter.

The nearest-polygon lookup is a **two-phase search**, not a single widened
one. Detour's `dtFindNearestPoly` returns whichever polygon is nearest to
the query point in raw 3D Euclidean distance, not "the polygon directly
below" — so on multi-level geometry (the real `castle_cellblock` fixture has
a mezzanine walkway a few units above and diagonally over from the
guard-spawn floor), a jump apex near that walkway's height can be *closer in
a straight line* to the airborne query point than the true floor is straight
down, and a single widened search returns the walkway's polygon instead.
Phase 1 repeats the original, unmodified search (so ground-level movement
and modest jumps are unaffected by this fix at all); phase 2 only runs when
phase 1 fails, and re-centers the search `JUMP_HEIGHT_TOLERANCE` units
*below* the query point — i.e. where the ground would be at the top of a
jump — so a true floor straight down outweighs a walkway merely diagonally
nearby.

A jump that stays over its own walkable footprint, within tolerance, is
accepted; a point that's actually off-mesh horizontally, clipped below the
surface, or too far above it, is still rejected. Regression guards (all in
`crates/entity/src/navigation/tests.rs` unless noted):
`jump_above_navmesh_same_xz_is_still_valid` (uses the real ~3.27-unit apex,
not an arbitrary smaller value), `just_above_jump_tolerance_is_still_invalid`,
`below_navmesh_small_clip_is_still_invalid` (the asymmetry guard),
`far_below_navmesh_same_xz_is_still_invalid`, and
`jump_in_place_is_accepted_not_rejected` in
`crates/cell-world/src/cell/space_manager/tests/movement_validation/mod.rs`
(end-to-end, also at the real apex).

## Follow-ups (not in #478)

- Anti-replay on `updateId` (CAT-B-05, issue #477) — composes with this
  validation to harden against captured-packet replays.
- 0x02 / 0x04 / 0x05 avatar variants are still length-parsed but not
  dispatched (CAT-B-10) — the same validator applies when they're wired.
- Per-world `top_speed` sourcing + `runSpeed = 6.0` drift reconciliation.
- Promote the speed layer from warn-only once calibration data exists.

## Adjacent validation gaps (not movement)

Folded in from the superseded server-systems survey (see
[server-systems.md](server-systems.md)). These are the anti-cheat gaps that
movement validation does **not** close, kept here because this is the doc you
will be reading when you ask "what else is unvalidated?"

**Damage sanity cap — still open.** Ability damage is computed server-side from
stats and ability definitions, so the *value* is never client-supplied. What is
missing is a ceiling: nothing detects a stat-modifier bug or a bad content row
that produces implausible damage. A max-damage assertion would catch
misconfiguration, not cheating — which is the point. Tracked as "Damage sanity
check" in [gap-analysis.md](../gap-analysis.md) §"Anti-Cheat Validation".

**Ability range — closed; line-of-sight — open.** `useAbility` rejects targets
beyond the ability's `max_range` (default 30.0) using server-side entity
positions, not client-reported ones
([`crates/cell-combat/src/cell/abilities/use_ability/handle.rs`](../../crates/cell-combat/src/cell/abilities/use_ability/handle.rs)).
Line of sight is *not* checked on that path, so an ability can still be cast
through a wall. Closing it needs the navmesh raycast that NPC AI also wants.

**Interaction distance.** Enforced for interactions. The original design note —
always validate against the server's own spatial state, never against a
client-supplied position — is the rule the four movement layers already follow
and the one any new validation should inherit.
