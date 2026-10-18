---
name: avatar-update-rotation-source
description: avatarUpdate angle bytes are the controlled pawn's Rotation, never the camera view; pitch/roll pack at 256/pi; the server unpack doubles pitch
metadata:
  type: project
---

PC-06 (2026-10-18, static Ghidra only) settled where the 3 angle bytes in the client move message come from: **the controlled pawn's `Rotation`** (S-CAM: A). The camera aim never reaches the server.

- Sender chain: `FUN_00c6edf0` (connection tick) → `FUN_00dd06a0` (periodic move sender, reads the controlled entity's actor) → `FUN_0084a8a0` (FRotator → radians: Yaw, −Pitch, −Roll) → `ServerConnection_addMove` `0x00dd9330` → packer `FUN_00de1720`.
- The packer uses 128/π for yaw but **256/π for pitch and roll** (512 steps per turn). Older docs said 256/(2π) for all three; `docs/protocol/position-updates.md` was corrected.
- The camera yaw writers `FUN_00e7e600`, `FUN_00e7e680` and `FUN_00e7eae0` only read the pawn yaw. `FUN_00e7eae0` chases the camera yaw toward the pawn yaw while moving, so the camera yaw looks **absolute**, not an offset. The lab code and `cegui-mouse-input-feed.md` call it an offset; unverified live.
- `SGWGamePawn.FaceRotation` zeroes Pitch while walking or falling, so the pitch byte is 0 for a walking avatar.
- Rotation never triggers a send; only velocity changes and the intervals do.

**Why:** the player-capture-bots campaign wanted to know if `interaction.pose` could carry the aim direction. It cannot without a client patch.

**How to apply:** treat `player_yaw_deg` as the body facing and `wire_pitch_deg` as pawn pitch (doubled by the server's 2π/256 unpack, and 0 while walking). Full finding: `docs/reverse-engineering/findings/avatar-update-rotation-source.md`. Related: [[client-entity-lifecycle-telemetry]], [[mercury-wire-format-openqs]].
