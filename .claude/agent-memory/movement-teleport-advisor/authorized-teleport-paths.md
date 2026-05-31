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

Future paths to add:
- GM `/teleport` command (when admin tools land)
- `/stuck` self-rescue command (when player tools land)
- Mount/dismount position adjustment (when vehicles land)

Pattern verification: the canonical `handle_teleport_player` already uses `compose_forced_position_body` + `onPlayerTeleport` in one bundle; PR3 wires `note_authorized_teleport` *before* the bundle sends so even if the client's position is mid-flight, the post-bundle client update will be re-seeded rather than rejected.

See [[movement-validation-anchors]] for the speed-tolerance and Ghidra anchors.
