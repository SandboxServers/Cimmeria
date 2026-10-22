//! Pure decoders, pure geometry, and the `Mem` readers for the camera and
//! the pawn pose.
//!
//! Every read goes through [`Mem`], so a stale pointer is `None`. Address
//! arithmetic wraps, because a corrupt pointer must not panic inside the
//! game's process.

use std::sync::atomic::{AtomicU32, Ordering};

use super::super::entity_trace::map::Mem;
use super::layout::{
    ACTOR_LOCATION_OFF, CAMERA_BLOCK_LEN, CAMERA_DIST_OFF, CAMERA_FLAGS_OFF, CAMERA_GAIN_OFF,
    CAMERA_PITCH_OFF, CAMERA_YAW_OFF, CONTROLLER_CAMERA_OFF, ENTITY_MANAGER_VA, ENTITY_PAWN_OFF,
    GLEVEL_PTR_VA, LEVEL_ACTORS_OFF, MANAGER_LOCAL_ENTITY_OFF, MAX_LEVEL_ACTORS,
    PLAYER_CAMERA_VTABLE_VA, PLAYER_CONTROLLER_VTABLE_VA, POSE_LEN, WORLD_LEVEL_OFF,
};

/// The camera's readable state (lab `CameraState`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CameraState {
    /// `+0x348` flags.
    pub flags: u32,
    /// Look gain, rotator units per mouse count.
    pub gain: f32,
    /// Third-person distance.
    pub distance: f32,
    /// Pitch offset, rotator units.
    pub pitch: i32,
    /// Yaw offset, rotator units.
    pub yaw: i32,
}

/// An actor's location and rotator, in client units.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ActorPose {
    /// Location, `x`, `y`, `z`.
    pub pos: [f32; 3],
    /// Pitch, rotator units.
    pub pitch: i32,
    /// Yaw, rotator units.
    pub yaw: i32,
}

/// The little-endian `u32` at `off`, or `None` when the slice is too short.
fn u32_at(b: &[u8], off: usize) -> Option<u32> {
    let bytes: [u8; 4] = b.get(off..off.checked_add(4)?)?.try_into().ok()?;
    Some(u32::from_le_bytes(bytes))
}

/// Decode the [`CAMERA_BLOCK_LEN`] block read at `camera + CAMERA_FLAGS_OFF`.
/// `None` when the block is shorter than that.
pub fn decode_camera(b: &[u8]) -> Option<CameraState> {
    if b.len() < CAMERA_BLOCK_LEN {
        return None;
    }
    // Each offset is relative to the start of the block.
    let at = |off: u32| (off - CAMERA_FLAGS_OFF) as usize;
    Some(CameraState {
        flags: u32_at(b, at(CAMERA_FLAGS_OFF))?,
        gain: f32::from_bits(u32_at(b, at(CAMERA_GAIN_OFF))?),
        distance: f32::from_bits(u32_at(b, at(CAMERA_DIST_OFF))?),
        pitch: u32_at(b, at(CAMERA_PITCH_OFF))? as i32,
        yaw: u32_at(b, at(CAMERA_YAW_OFF))? as i32,
    })
}

/// Decode a [`POSE_LEN`] read at `actor + ACTOR_LOCATION_OFF`. `None` when the
/// read is shorter than that.
pub fn decode_actor_pose(b: &[u8]) -> Option<ActorPose> {
    if b.len() < POSE_LEN {
        return None;
    }
    let pos = [
        f32::from_bits(u32_at(b, 0)?),
        f32::from_bits(u32_at(b, 4)?),
        f32::from_bits(u32_at(b, 8)?),
    ];
    Some(ActorPose {
        pos,
        pitch: u32_at(b, 0xc)? as i32,
        yaw: u32_at(b, 0x10)? as i32,
    })
}

/// Rotator units to degrees, not wrapped: `units * 360 / 65536`.
pub fn rot_deg(units: i32) -> f32 {
    units as f32 * 360.0 / 65536.0
}

/// Rotator units to degrees, wrapped to `[0, 360)`.
pub fn rot_deg_360(units: i32) -> f32 {
    rot_deg(units).rem_euclid(360.0)
}

/// Client location to server units: `[p[1] / 100, p[2] / 100, p[0] / 100]`.
pub fn client_to_server(p: [f32; 3]) -> [f32; 3] {
    [p[1] / 100.0, p[2] / 100.0, p[0] / 100.0]
}

/// Is `ptr` non-null and does the word at `ptr` equal `vtable`?
fn has_vtable(mem: &impl Mem, ptr: u32, vtable: u32) -> bool {
    ptr != 0 && mem.u32_at(ptr) == Some(vtable)
}

/// Find the local `ASGWController_Player` by scanning the level's Actors
/// array for its vtable. The scan is the lab's; the old chain reads a
/// `WorldInfo` field that is 0 in a normal game.
pub fn find_player_controller(mem: &impl Mem) -> Option<u32> {
    let world = mem.u32_at(GLEVEL_PTR_VA)?;
    if world == 0 {
        return None;
    }
    let level = mem.u32_at(world.wrapping_add(WORLD_LEVEL_OFF))?;
    if level == 0 {
        return None;
    }
    let actors_at = level.wrapping_add(LEVEL_ACTORS_OFF);
    let data = mem.u32_at(actors_at)?;
    let count = mem.u32_at(actors_at.wrapping_add(4))?.min(MAX_LEVEL_ACTORS);
    if data == 0 || count == 0 {
        return None;
    }
    // One read for the whole pointer array: this runs on the game thread, and
    // two reads per slot over 8192 slots is far too slow to repeat.
    if let Some(bytes) = mem.bytes_at(data, count as usize * 4) {
        return bytes
            .as_chunks::<4>()
            .0
            .iter()
            .map(|c| u32::from_le_bytes(*c))
            .find(|&actor| has_vtable(mem, actor, PLAYER_CONTROLLER_VTABLE_VA));
    }
    // Part of the array is unreadable: walk it slot by slot, where a slot that
    // cannot be read is skipped, the same as a null slot.
    for i in 0..count {
        let Some(actor) = mem.u32_at(data.wrapping_add(i * 4)) else {
            continue;
        };
        if has_vtable(mem, actor, PLAYER_CONTROLLER_VTABLE_VA) {
            return Some(actor);
        }
    }
    None
}

/// The local controller's camera (`+0x2b0`), when that camera has the camera
/// vtable.
pub fn find_player_camera(mem: &impl Mem) -> Option<u32> {
    let pc = find_player_controller(mem)?;
    let camera = mem.u32_at(pc.wrapping_add(CONTROLLER_CAMERA_OFF))?;
    has_vtable(mem, camera, PLAYER_CAMERA_VTABLE_VA).then_some(camera)
}

/// The camera pointer, from `cache` when its vtable still matches, else from a
/// fresh scan. The result is stored in `cache`; a failed scan clears it.
pub fn camera_ptr(mem: &impl Mem, cache: &AtomicU32) -> Option<u32> {
    let cached = cache.load(Ordering::Relaxed);
    if has_vtable(mem, cached, PLAYER_CAMERA_VTABLE_VA) {
        return Some(cached);
    }
    let found = find_player_camera(mem);
    cache.store(found.unwrap_or(0), Ordering::Relaxed);
    found
}

/// The camera's flags, gain, distance, pitch and yaw, from one block read.
pub fn read_camera(mem: &impl Mem, camera: u32) -> Option<CameraState> {
    decode_camera(&mem.bytes_at(camera.wrapping_add(CAMERA_FLAGS_OFF), CAMERA_BLOCK_LEN)?)
}

/// The pose of the camera actor. `None` when its location is exactly zero: the
/// client never wrote one, and a zero pose would make every bearing wrong.
pub fn read_camera_pose(mem: &impl Mem, camera: u32) -> Option<ActorPose> {
    let pose = read_actor_pose(mem, camera)?;
    (pose.pos != [0.0; 3]).then_some(pose)
}

/// The local pawn's pose, through `[[ENTITY_MANAGER_VA] + 0x0c] + 0x08`.
/// `None` on any null link. Only valid under the S-CAM A verdict.
pub fn read_pawn_pose(mem: &impl Mem) -> Option<ActorPose> {
    let manager = nonnull(mem.u32_at(ENTITY_MANAGER_VA))?;
    let entity = nonnull(mem.u32_at(manager.wrapping_add(MANAGER_LOCAL_ENTITY_OFF)))?;
    let pawn = nonnull(mem.u32_at(entity.wrapping_add(ENTITY_PAWN_OFF)))?;
    read_actor_pose(mem, pawn)
}

/// `Some(v)` unless `v` is `Some(0)`.
fn nonnull(v: Option<u32>) -> Option<u32> {
    v.filter(|p| *p != 0)
}

/// The location and rotator of `actor`, one [`POSE_LEN`] read.
fn read_actor_pose(mem: &impl Mem, actor: u32) -> Option<ActorPose> {
    decode_actor_pose(&mem.bytes_at(actor.wrapping_add(ACTOR_LOCATION_OFF), POSE_LEN)?)
}
