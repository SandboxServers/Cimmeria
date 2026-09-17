---
name: authorized-teleport-paths
description: Every server-side path that legitimately writes entity position — each must update last_pos to avoid the canonical false-positive
metadata:
  type: reference
---

The canonical movement-validator failure mode is: an authoritative path moves a player by 500 units, the validator's `last_pos` still points at the source, the next inbound client position triggers the speed/teleport check, and the player gets snapped back to source. Every path below must call `validator.note_authorized_teleport(entity_id)` before its position write (or rely on a designed seam that does so).

Verified to exist as of 2026-05-27:

| Path | File | Trigger |
|---|---|---|
| Same-world teleport | `crates/services/src/base/world_entry/teleport.rs::handle_teleport_player` | Server-issued teleport (mission warp, GM tools) |
| Cross-world gate travel | `crates/services/src/base/world_entry/gate_travel/` | Stargate transition; arrival is fresh spawn so `last_pos` initializes from scratch |
| Ring transport arrival | `crates/services/src/cell/ring_transport/transporter/mod.rs` | Ring-platform pad-to-pad teleport |
| Respawn after death | `crates/services/src/cell/cell_methods/player/combat/respawn.rs` | Death → respawner point snap |
| World entry / play character | `crates/services/src/base/world_entry/play_character.rs` | Initial spawn into world |
| Reanchor (recovery) | `crates/services/src/base/world_entry/reanchor_player.rs` | Desync recovery snap |

Added since (verified 2026-09-17):

| Path | File | Trigger |
|---|---|---|
| GM console travel | `crates/services/src/cell/console/travel/` (`.gotoxyz`, `.goto`, `.summon`, `.gotolocation`, `.gotospace`) | `snap_in_current_space` calls `note_authorized_teleport` unconditionally; cross-space legs go through `cell/space_transfer` |

`cell/space_transfer` has **two** entry points, and picking the wrong one silently breaks `.gotospace`: `transfer_player_to_space` resolves a (possibly typed) world name through `canonical_world_name`, while `transfer_player_to_loaded_space` takes a pre-verified space id and never consults the world table. `.gotospace`'s whole promise is reaching a live instance whose world the table may not declare, so routing it through the name-based one re-imposes exactly the `UnknownWorld` dead-end it exists to avoid — and only the same-space fast path would still appear to work. The by-id path re-checks that the instance is still loaded, because arrival (`handle_create_entity`) degrades a stale `destination_space_id` to `find_or_create_space`, which for an undeclared world fails *after* teardown (un-spaced player).
| Off-navmesh recovery | `crates/services/src/cell/space_manager/client_move.rs::reject_outcome` | Validator relocating a stranded entity — see [[snap-back-termination]] |

**These are all unchecked `update_entity_position` writes**, with one exception. None of the GM/content/ring/respawn paths validates that the destination is somewhere the client can legally stand, which is what made the rubber-band loop in [[snap-back-termination]] reachable. Any new path added to this table inherits that hazard.

The exception is the last row: `reject_outcome`'s recovery **is** validated. `resolve_recovery_position` runs every candidate — Detour reprojection, nearest world respawner, AABB clamp — through `position_within_bounds` and `NavMesh::is_point_valid` before returning it, and answers `None` (→ `CorrectionSuppressed`) when nothing passes. It has to: the recovery write calls `note_authorized_teleport`, which clears the correction budget, so an unsound recovery target restarts the loop with nothing left to spend.

Future paths to add:
- `/stuck` self-rescue command (when player tools land)
- Mount/dismount position adjustment (when vehicles land)

Pattern verification: the canonical `handle_teleport_player` already uses `compose_forced_position_body` + `onPlayerTeleport` in one bundle; PR3 wires `note_authorized_teleport` *before* the bundle sends so even if the client's position is mid-flight, the post-bundle client update will be re-seeded rather than rejected.

See [[movement-validation-anchors]] for the speed-tolerance and Ghidra anchors.
