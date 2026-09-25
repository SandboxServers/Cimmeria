---
name: npc-broadcast-facing-and-grounding
description: NPC broadcast facing/grounding notes — pack_angle north-snap fixed in #677; the "send OnGround 0x18 to ground NPCs" idea is WRONG (corrected 2026-09-24), 0x18 keeps the client's current height
metadata:
  type: reference
---

> **Status 2026-09-19 — partly fixed.** Defect 1 (`pack_angle` saturating negative yaw to north) was
> FIXED in #677: it now wraps into `[0, TAU)` and rounds. Do not block on it. `get_navmesh_height` is
> no longer caller-less either: #680 wired it into `ticks/npc_movement.rs`. Defect 2 is **still true**
> on `main`: `BASEMSG_UPDATE_AVATAR_NO_ALIAS_FULL_POS_YPR = 0x10` is the only variant sent, so the
> client never grounds NPCs. Defect 3 (movement type not broadcast on path start/stop) was not
> re-verified. Line numbers are as of 2026-09-18.
>
> **Correction 2026-09-24 (NPC AI audit M4 and M6).** Defect 2's conclusion is **wrong**. OnGround
> (`0x18`) does not ground anything: `FUN_00ddb830` writes `DAT_019d1a44` = **-13000.0f** (bytes
> `00 20 4b c6`, not FLT_MAX) into Y, and `BW_client_entity_manager_6` (`0x00dd1859`) replaces a
> sentinel component with the actor's **current client Location**. There is no ray-cast and no height
> map. Sending `0x18` would pin every NPC at its creation height. Keep `0x10` and ground NPCs on the
> server. Also, `get_navmesh_height` is not ground truth before NA01 (PR #774): it searched around
> world Y = 0 and returned the wrong storey on multi-level meshes. Evidence:
> `docs/analysis/npc-ai-restoration/evidence/npc-ground-audit.md` §B. The text below is kept as history.

Three independent defects in the NPC position broadcast, all confirmed 2026-09-18 from the colo
playtest. They compose into the long-standing "NPCs face the wrong way, walk up the air, moonwalk"
report, so fixing one alone will not make the symptom go away.

**1. `pack_angle` saturates the negative half-circle to due north.**
`crates/services/src/mercury/aoi/mod.rs:47-50` is `(radians / SCALE) as u8`. Rust's float→int `as`
is **saturating** (1.45+), so any negative quotient becomes `0u8`. NPC yaw is `dx.atan2(dz)`
(`cell/service/ticks/npc_movement.rs:103`, `:110`, `:156`) with range `(-π, π]`, so **yaw in
`(-π, 0)` → byte 0 → due north**; `(0, π]` → `0..128` correct. The doc comment claims it "Matches
C++ `(uint8_t)(...)`" — it does not; C++ truncates modularly on x86. Fix is
`(radians.rem_euclid(TAU) / SCALE) as u8`. **Any regression guard must use a negative angle** — a
`[0, π]` fixture still passes with the bug present.

Two more paths to a spurious north facing in the same tick: the degenerate `nd <= 0.001` branch sets
`yaw = 0.0` (`npc_movement.rs:106`), and `update_entity_position(..., [0,0,0], ...)` zeroes facing
before the tick patches it back (`:113-118`, `:184` — see [[facing-preservation-primitive]]); the
latter is latent only because `write_position` does not broadcast.

**2. We only ever send `FullPos`, so the client renders our Y exactly and never grounds the NPC.**
`BASEMSG_UPDATE_AVATAR_NO_ALIAS_FULL_POS_YPR = 0x10` is hardcoded
(`mercury/aoi/mod.rs:38`, built in `aoi/update.rs:34-47`, `aoi/create.rs:101-104`). The UPDATE_AVATAR
variant index bits[3:2] pick the position type, and the three types are **byte-identical on the
wire** — only the client's handler differs (`docs/drafts/spec/position-updates.md:119-127`):
FullPos `FUN_00ddb0c0` reads wire Y as-is; OnChunk `FUN_00ddb220` and OnGround `FUN_00ddb830`
**discard wire Y** (FLT_MAX sentinel) and take it from the chunk height map / a terrain ray-cast.
UE3 collision is disabled on `ABigWorldEntity` and neither AvatarFilter does a ground query, so
nothing on the client can correct us.

→ **Switching the NPC broadcast to the OnGround variant (`0x18`) is a one-byte change that fixes
floating in every world, including the ones with no navmesh.** Cheapest high-value fix in the area.
Confirm in-game that it grounds rather than sinks before trusting it, and do not blanket-apply it to
flying/swimming NPCs. The `physics` byte is separately hardcoded `0x01` (`update.rs:43`, pinned by
`aoi/tests.rs:207`) and the PHYS_* value table is **undocumented** — worth an RE pass at
`FUN_00ddb830` / the `sentPhysics_` compare.

**3. Animation and translation are decoupled channels.** The client picks mob animation from the
`setMovementType` byte (SGWBeing method 1, client FSM `FUN_00deb660`), **not** from velocity or
position deltas — `crates/entity/src/cell_entity/mod.rs:180-190`. `npc_movement_tick` never calls
`broadcast_movement_type`; only the AI-state handlers do, and `messaging.rs:226-231` dedups identical
kinds while `:235-239` sends nothing for `kind = None`. An NPC that translates while the client still
holds a stationary pose is a moonwalk by construction. Movement type must be broadcast on path
start/stop, not only on AI-state change. (Which enum value per state → npc-ai-spawn-advisor.)

**Grounding truth on the server is also unwired:** `SpaceManager::get_navmesh_height`
(`cell/space_manager/spatial.rs:71-76`) exists and has **no production caller** — the tick lerps Y
linearly between waypoints instead (`npc_movement.rs:152`), and Detour straight-path corners sit on
the poly mesh, not the detail mesh. Vertical axis is **Y** (see [[arrival-coordinate-offnavmesh]]).

See [[castle-has-no-navmesh]] for the straight-line fallback this stacks with.
