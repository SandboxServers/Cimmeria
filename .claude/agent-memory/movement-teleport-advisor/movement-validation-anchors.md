---
name: movement-validation-anchors
description: Ghidra + DB anchors for the movement-validation tolerance, speed source, and teleport-snap threshold
metadata:
  type: reference
---

Reference points for movement-validation design (issue #63):

**Client teleport-snap threshold (upper bound on what client smooths).** `USGWAvatarFilter::Input` at `0x00e81970` in SGW.exe contains the load-bearing decision:
```c
if (_DAT_01e69c90 < SQRT(dz*dz + dx*dx + dy*dy) * (1.0 / dt)) { hard_snap(); }
```
`_DAT_01e69c90 @ 0x01e69c90` = `0x451C4000 f32` = **2500.0 units/second**. Anything above this ratio is hard-snapped by the client without interpolation. This is the **upper ceiling, not the cheat threshold** — server-side anti-cheat tolerance sits 3 orders of magnitude below this (`top_speed × 1.5 ≈ 12 u/s`).

**Frame buffer depth.** `DAT_01e69c8c @ 0x01e69c8c` = `8` (ring buffer of 56-byte frames, minimum asserted 5 via "FrameCount > 4"). Tells us client carries ~800 ms history at 10 Hz — `TELEPORT_GRACE = 2.0 s` comfortably exceeds this.

**Top-speed data source.** Per-world from `db/resources/Worlds/Seed/worlds.sql` `run_speed` column. Every populated world uses `8.125 u/s`. The `worlds` table also carries `walk_speed`, `swim_speed`, `crouch_run_speed`, `jump_speed`, etc. (see schema in `worlds.sql`). Python source reads them as `WorldInfo.runSpeed` in `deprecated/python/common/defs/WorldInfo.py:18`.

**Drift bug — pre-existing.** `crates/services/src/mercury/world_data/mod.rs:112` hardcodes `runSpeed = 6.0` in `build_world_params_args`, while DB says 8.125. Server tells client one number, validator must use the same — PR2 of #63 reconciles by sourcing both from `WorldInfo`.

**Server tick rate.** 100 ms / 10 Hz per `deprecated/cpp-config/config/BaseService.config` `<tick_rate>100</tick_rate>`. Use this as the implicit cadence assumption when calibrating tolerance.

**Inbound client position seam.**
- Wire: `0x03 AVATAR_UPDATE_EXPLICIT` (40 bytes), parsed at `crates/services/src/base/connect_loop/encrypted/mod.rs:214`.
- Forwarded as `BaseToCellMsg::EntityMove`, handled in `crates/services/src/cell/service/base_messages/mod.rs:138`.
- Final write at `crates/services/src/cell/space_manager/entities.rs::update_entity_position:147`.
- **Gap**: `0x02 AVATAR_UPDATE_IMPLICIT`, `0x04 WARD_IMPLICIT`, `0x05 WARD_EXPLICIT` are length-parsed but never dispatched. Separate issue; same validator will apply.

**Authorized teleport bundle (canonical pattern).** `crates/services/src/base/world_entry/teleport.rs::build_teleport_bundle:151` composes `FORCED_POSITION (0x31) + onPlayerTeleport (method 116)` into one Mercury bundle. `onPlayerTeleport` is a streaming-load hint only; `FORCED_POSITION` is the authoritative snap. See [[authorized-teleport-paths]] for the full list of paths.

**Tolerance calibration methodology.** If derivation from RE alone is insufficient (it is — we have upper bound 2500 u/s and lower bound 8.125 u/s, gap is policy), ship the speed check **warn-only** first, collect SigNoz rejection logs with full `(distance, dt)` fields, compute p99.9 of legitimate `(distance / dt) / top_speed` ratios bucketed by RTT (50/200/500 ms), set the production tolerance from that distribution. Do not enforce snap-back until calibration data exists.
