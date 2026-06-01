---
name: pr1-bounds-seam
description: Where the bounds validator landed in the cell seam, and the seam pattern PR2/3/4 should follow
metadata:
  type: project
---

PR1 of issue #63 (bounds-check layer) landed on branch `feat/movement-validation-bounds-63-pr1`. The seam pattern is the load-bearing decision for PR2/3/4 — record it so the speed / teleport-detection / navmesh-containment work doesn't have to re-derive it.

**Why:** the controller-level `PlayerMovementController::apply_validated_client_update` isn't yet wired into the production cell path. The live seam for client position updates is `BaseToCellMsg::EntityMove → SpaceManager::apply_client_position_update`. Validation must happen here, not in the controller.

**How to apply:**

- Validator lives on `SpaceManager` as `movement_validator: MovementValidator` (stateless for PR1; PR2/3 add per-entity state to the same field).
- New entry point on `SpaceManager`: `apply_client_position_update(entity_id, position, direction, velocity) -> ClientMoveOutcome`. Returns `Accepted` / `Rejected { reason, last_valid, space_id, bounds }` / `EntityMissing`. The unchecked `update_entity_position` stays — server-authoritative callers (ring transport, respawn, content teleport, NPC movement) use it directly and bypass validation.
- The `EntityMove` arm in `cell/service/base_messages/mod.rs` is the single consumer. On `Rejected`, it emits `CellToBaseMsg::TeleportPlayer { entity_id, space_id, position: last_valid, prev_pos: last_valid }` which routes through the existing `handle_teleport_player` → `compose_forced_position_body` path — no new wire-format risk.
- Bounds source: `space.navmesh.bmin/bmax` if loaded, else `SpaceBounds::FALLBACK = ([-10_000, -2_000, -10_000], [10_000, 10_000, 10_000])`. Spaces without a loaded navmesh use the fallback so non-Castle zones don't snap-fest.

**AoI refresh:** by NOT writing the rejected position, the AoI tick (100 ms) naturally rebroadcasts the unchanged last-valid position to witnesses. No explicit AoI fan-out needed. The owner gets the immediate snap via `BASEMSG_FORCED_POSITION` through `TeleportPlayer`.

**Negative-log shape pinned in tests:**
- `target: "movement.validation"`, level `warn!`, message starts `"movement.bounds_violation:"`.
- Fields: `reason="bounds"`, `entity_id`, `space_id`, `client_{x,y,z}`, `last_valid_{x,y,z}`, `bounds_{min,max}_{x,y,z}`, `reject=?MovementReject` (Debug-formatted).
- LogCapture pinned in `cell::service::base_messages::tests::general::entity_move_out_of_bounds_rejects_and_emits_teleport_player_snap_back`.

**Test seams worth knowing:**
- `Castle_CellBlock` in spaces.xml is `Instanced="true"` — every `create_entity` builds a fresh space, so co-spatial tests (witness + offender) must use a non-instanced world (`Agnos` works). Tripped over this twice; record so it doesn't trip a third time.
- `external/` is not symlinked into worktrees by `git worktree add`; native MSVC entity tests need a `New-Item -ItemType Junction` to the main repo's `external/` before `cargo check -p cimmeria-entity` compiles.

**Wire-format byte layout** for `BASEMSG_FORCED_POSITION (0x31)` is 50 bytes — pinned by `snap_back_message_routes_through_compose_forced_position_body`:
- `[0]` = 0x31
- `[1..5]` = entity_id u32 LE
- `[5..9]` = space_id u32 LE
- `[9..13]` = vehicle_id u32 LE (= 0)
- `[13..25]` = position xyz f32 LE
- `[25..37]` = prev_pos xyz f32 LE
- `[37..49]` = rotation yaw/pitch/roll f32 LE (= 0,0,0)
- `[49]` = flags u8 (= 0x01)

For the snap-back: position == prev_pos == last_valid, so client interpolation is zero-distance (no visible jitter).

See [[movement-validation-anchors]] for the Ghidra anchors and [[authorized-teleport-paths]] for the PR3 list.
