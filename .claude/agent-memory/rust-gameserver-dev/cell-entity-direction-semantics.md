# `CellEntity.direction` semantics — settled

`crates/entity/src/cell_entity/entity_struct.rs`'s doc comment still says
"unit vector or yaw/pitch/roll encoded". That ambiguity is wrong and has
produced a whole family of shipped bugs. The settled answer:

**`direction` is `[pitch, yaw, roll]` in RADIANS.** `direction.y` is yaw.

Evidence (Rust, outbound):

- `crates/wire/src/mercury/aoi/{create,update}.rs` pack
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

- `cell/console/gm/query.rs::handle_show_rotation` reads
  `d.x.atan2(d.z)` — wrong. Tracked as **P48**.
- `cell/console/entity.rs::look_at` (`.lookat`) writes a Cartesian unit
  vector `Vector3::new(dx/len, 0.0, dz/len)` — wrong; should be
  `direction.y = dx.atan2(dz)`. Recommended for P48's scope.
- `console/spawn/mod.rs`'s `heading_of` had the same `atan2(x, z)` bug —
  already fixed.
- **Inbound client direction is unpacked since P49.**
  `base/connect_loop/encrypted/mod.rs` reads three raw packed-angle bytes
  (`payload[32..35] as i8`, wire order yaw, pitch, roll). `update_entity_position`
  unpacks them with `unpack_angle` into `[pitch, yaw, roll]` radians, so a
  moving player's `direction` holds radians like everything else. Caveat
  ([avatar-update-rotation-source.md](../../../docs/reverse-engineering/findings/avatar-update-rotation-source.md)):
  `unpack_angle` uses 2*pi/256 per step for all three bytes, but the client
  packs pitch and roll at pi/256 per step, so the stored pitch (`direction.x`)
  and roll are twice the true value. It round-trips unchanged on re-pack. Only
  a reader that converts `direction.x` to degrees (`wire_pitch_deg` on
  `interaction.pose`) sees the doubling; it is 0 while walking.

## Teleports keep facing

`SpaceManager::update_entity_position(entity_id, position, direction: [i8;3],
velocity)` writes `direction` **unconditionally**, from the client's packed
bytes. A server-authoritative move (GM travel, `.gotoxyz`, respawn, ring
transport, the content executor's transport action) has no new facing to
supply, so it uses `update_position_preserving_facing(entity_id, position,
velocity)`, which never writes `direction`. Passing `[0, 0, 0]` to
`update_entity_position` would snap the entity's facing to north; do not.

To set a real orientation, write `e.direction = Vector3::new(p, y, r)` through
`get_entity_mut` after the move. `cell/service/ticks/npc_movement.rs` and
`cell/console/placement.rs::location` (P18) do this.

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
