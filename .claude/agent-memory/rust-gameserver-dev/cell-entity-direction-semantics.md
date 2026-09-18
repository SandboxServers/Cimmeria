# `CellEntity.direction` semantics — settled

`crates/entity/src/cell_entity/entity_struct.rs`'s doc comment still says
"unit vector or yaw/pitch/roll encoded". That ambiguity is wrong and has
produced a whole family of shipped bugs. The settled answer:

**`direction` is `[pitch, yaw, roll]` in RADIANS.** `direction.y` is yaw.

Evidence (Rust, outbound):

- `crates/services/src/mercury/aoi/{create,update}.rs` pack
  `pack_angle(direction[1])` as yaw, `[0]` pitch, `[2]` roll —
  unconditionally, for players and NPCs alike.
- `pack_angle` (`mercury/aoi/mod.rs:47`) divides by `0.024543693` = 2*pi/256,
  so the input is radians.
- `cell/service/ticks/npc_movement.rs` writes
  `npc.direction = Vector3::new(0.0, yaw, 0.0)` with `yaw = dx.atan2(dz)`.

Evidence (legacy python, same convention):

- `SGWPlayer.connected`: `dir = Atrea.Vector3(0, self.heading, 0)` then
  `self.rotation = dir`.
- `SGWPlayer.save`: persists `heading = rot.y`.
- `SGWSpawnableEntity.lookAt`: computes a heading and writes only `rot.y`.
- `SGWSpawnableEntity.facing`: reads `self.rotation.y`.
- `cell/commands/Resource.py:176`: `rot = target.rotation.y`.

## Known bugs in this family

- `cell/cell_methods/gm/query.rs::handle_show_rotation` reads
  `d.x.atan2(d.z)` — wrong. Tracked as **P48**.
- `cell/console/entity.rs::look_at` (`.lookat`) writes a Cartesian unit
  vector `Vector3::new(dx/len, 0.0, dz/len)` — wrong; should be
  `direction.y = dx.atan2(dz)`. Recommended for P48's scope.
- `console/spawn/mod.rs`'s `heading_of` had the same `atan2(x, z)` bug —
  already fixed.
- **Inbound client direction is never unpacked.**
  `base/connect_loop/encrypted/mod.rs:278` reads three raw packed-angle
  bytes (`payload[32..35] as i8`) and they reach `direction` as `i8 as f32`
  via `apply_client_position_update_at` -> `update_entity_position`. So a
  moving player's `direction` holds byte units (-128..127), which
  `pack_angle` then re-packs as if they were radians. Needs an
  `unpack_angle` on the inbound path. Flagged in P18's handoff; not yet
  ticketed as of 2026-09-17.

## `update_entity_position` zeroes facing

`SpaceManager::update_entity_position(entity_id, position: [f32;3],
direction: [i8;3], velocity: [f32;3])` writes
`cell_entity.direction = Vector3::new(d[0] as f32, d[1] as f32, d[2] as f32)`
**unconditionally**. Every server-authoritative caller passes `[0, 0, 0]`:
`.gotoxyz`, native `gmGotoXYZ`/`gmGoto`/`gmSummon`, respawn, ring transport,
the content executor's transport action. All of them silently reset the
moved entity's facing.

The `[i8; 3]` parameter cannot express a float orientation at all. To set or
preserve a real orientation, use the established pattern:

```rust
let facing = e.direction;                 // capture BEFORE
space_mgr.update_entity_position(id, pos, [0, 0, 0], [0.0; 3]);
if let Some(e) = space_mgr.get_entity_mut(id) {
    e.direction = facing;                 // or a new Vector3::new(p, y, r)
}
```

`cell/service/ticks/npc_movement.rs` and
`cell/console/placement.rs::location` (P18) are the only two callers that do
this today.

## AoI fan-out for orientation

No explicit fan-out is needed for a `direction` write.
`cell/space_manager/aoi.rs` emits
`CellToBaseMsg::EntityMoved { position, direction, velocity, .. }` for
*every* entity in a witness's current AoI on *every* tick — the emission is
not gated on a position delta. So a rotation-only change reaches witnesses
on the next tick.

## No wire surface for a player's own camera yaw

`compose_forced_position_body` (`mercury/aoi/update.rs:110-128`) writes
entity id, space id, `vehicleID = 0`, position, previous position — **no
angles**. `BASEMSG_FORCED_POSITION` cannot snap a player's own camera
orientation. Any feature that needs that is a new wire surface.
