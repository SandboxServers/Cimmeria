# Movement Validation (server-authoritative position)

> **Status**: Shipped in issue **#478** (CAT-B-01 + CAT-B-06 + CAT-B-09),
> building on the bounds layer from #437 (PR1 of 4). Applies to every
> inbound client position update (`AVATAR_UPDATE_EXPLICIT`, system message
> `0x03`).

## Context

The SGW client is *client-authoritative* for its own avatar position: it
streams raw `f32` world coordinates in `AVATAR_UPDATE_EXPLICIT` (0x03) at
~10 Hz and expects the server to mirror them into the cell entity. Before
#478, the cell wrote those coordinates with **zero validation** — every
per-tick update was a free teleport. Every position-derived system (AoI /
witness scope, region triggers, mission gates, threat radius, navmesh
distance) reads from the cell entity's `position`, so a single tampered
0x03 corrupted all of them downstream. See
[CAT-B-movement.md](../security-audit/2026-05-31-server-authority/findings/CAT-B-movement.md)
(CAT-B-01, -06, -09).

## Decision

A single validation seam,
`SpaceManager::apply_client_position_update`, gates **every** inbound
client position. It is the only path the `EntityMove` handler calls;
server-authoritative writers (ring transport, respawn, gate arrival,
content-engine teleport, GM travel, NPC movement) keep using the
unchecked `update_entity_position` directly — they are the source of
truth for those entities.

The validator (`cimmeria_entity::movement_validation::MovementValidator`)
runs four layers. The table is in **execution order** as wired in
`apply_client_position_update_at` — cheapest spatial gate first, then the
stateful kinematics. Three layers hard-reject (snap-back); the speed
sub-layer is **warn-only** until calibrated. Speed and teleport share one
`check_kinematics` call (the kinematics layer); they are listed as two
rows because they have different actions.

| Order | Layer | Action | Catches |
|-------|-------|--------|---------|
| 1 | **Bounds** (`check_bounds`) | reject | NaN / ±∞ / absurd coords, **Z-axis floor-clip** (full X/Y/**Z** AABB test) |
| 2 | **Navmesh** (`is_position_valid`) | reject | off-walkable-polygon (walls, under-terrain, ceilings); fail-open when no navmesh loaded |
| 3 | **Speed** (`check_kinematics`) | **warn-only** | sustained over-tolerance velocity (`implied_speed > top_speed × 1.5`) |
| 4 | **Teleport** (`check_kinematics`) | reject | single update both `> 50 u` **and** `> top_speed × 10` (or, on the first packet with no time baseline, `> 50 u` from the authoritative spawn) |

Bounds AABB is sourced from the active space's navmesh `bmin`/`bmax`, or
`SpaceBounds::FALLBACK` (20 km × 12 km × 20 km) for navmesh-less spaces.

On reject the cell entity is **not** advanced; the handler emits
`CellToBaseMsg::TeleportPlayer { position == prev_pos == last_valid }`,
which composes `BASEMSG_FORCED_POSITION (0x31)` to snap the offending
client back. Because the cell entity never moved, the next 100 ms AoI
tick naturally rebroadcasts the last-valid position to witnesses — no
explicit AoI fan-out is needed. The structured negative log is
`target: "movement.validation"`, message `movement.validation_reject:`,
with a low-cardinality `reason` field (`bounds | navmesh | teleport`) and
the `movement_validation_rejects_total{reason}` counter.

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
accepts**, emitting the full `(distance, dt_secs, implied_speed,
top_speed)` triple on `movement.speed_warning` /
`movement_validation_warns_total{reason="speed"}`. Calibrate the
production tolerance from the SigNoz p99.9 of legitimate
`(distance/dt)/top_speed` (bucketed by RTT) before promoting it to
snap-back.

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
`dt` is measured from the teleport instant (suppressing a spurious speed
warn when an authoritative move interrupts the client's stream). It does
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

## Constants

Defined on `MovementValidator` (see source for full rationale):

| Constant | Value | Source / note |
|----------|-------|---------------|
| `DEFAULT_TOP_SPEED` | `8.125` u/s | `db/resources/Worlds/Seed/worlds.sql` `run_speed` (universal). Per-world sourcing + reconciling the `runSpeed = 6.0` drift in `mercury/world_data` is a follow-up; warn-only makes the single constant safe meanwhile. |
| `SPEED_WARN_TOLERANCE` | `1.5×` | warn threshold (warn-only) |
| `TELEPORT_JUMP_UNITS` | `50.0` u | teleport distance gate |
| `TELEPORT_SPEED_FACTOR` | `10×` | teleport implied-speed gate |

The client's own hard-snap ceiling (`USGWAvatarFilter::Input`,
`_DAT_01e69c90 = 2500 u/s`) is the upper bound on what the client
smooths; the server gates sit far below it. See
`.claude/agent-memory/movement-teleport-advisor/movement-validation-anchors.md`.

## Consequences

- **Wire-format touch**: `EntityMove` gained a `claimed_space_id` field;
  the 0x03 parser now reads `payload[0..4]`. No client-visible change.
- **Per-entity state**: `MovementValidator` holds a `HashMap<u32,
  Instant>` clock, released in `destroy_entity` via `forget` (no leak /
  no stale sample across id reuse).
- **Regression guards** (issue #478 close criteria):
  - `teleport_100m_over_50ms_is_rejected_and_not_observed`
  - `off_navmesh_position_is_rejected_and_not_observed` (real
    `castle_cellblock.nav` fixture, self-skips on fixture-less CI)
  - plus speed-warn-accepts, authorized-teleport-follow-up,
    sustained-spam, and `entity_move_space_mismatch_warns_but_still_applies`.

## Follow-ups (not in #478)

- Anti-replay on `updateId` (CAT-B-05, issue #477) — composes with this
  validation to harden against captured-packet replays.
- 0x02 / 0x04 / 0x05 avatar variants are still length-parsed but not
  dispatched (CAT-B-10) — the same validator applies when they're wired.
- Per-world `top_speed` sourcing + `runSpeed = 6.0` drift reconciliation.
- Promote the speed layer from warn-only once calibration data exists.
