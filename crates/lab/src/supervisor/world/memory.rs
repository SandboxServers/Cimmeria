//! Memory reads the world tools need: actor poses and the unit-slot map.
//!
//! Layout (Ghidra, this client build; live checks noted in
//! docs/guides/live-research-lab.md § World tools):
//!
//! - `unitPosition(u)` (`FUN_00aeb960`) resolves the unit to an `Entity*`
//!   and returns `BW__unknown_00e685c0(entity)`: the entity's actor
//!   (`+0x08`) plus `0xDC`, i.e. UE3 `AActor::Location`. The `FRotator`
//!   (`Pitch`, `Yaw`, `Roll` as `int32`) follows at `+0xE8`, which is what
//!   `unitOrientation` (`FUN_00aeb9b0`) turns into a fraction of a turn.
//! - Units are **slots**, not entity ids: `FUN_00c67120` looks the Lua unit
//!   number up in a `std::map<int, int>` (slot -> entity id) at
//!   `GameEntityManager + 0x130` (head `+0x134`, size `+0x138`). `Unit.Player`,
//!   `Unit.Target`, `Unit.MouseOver`, ... are keys of that map.
//! - `FUN_00c67bd0` (thiscall: manager, slot, entity id) writes a slot and
//!   raises `Event_UI_UnitMappingChanged(slot)`. The only Lua handler
//!   (`UnitFramesMod.onUnitMappingChanged`) ignores slots it does not track
//!   and the portrait manager only acts on registered portrait slots, so a
//!   slot number no stock code uses is a private handle: the lab pins an
//!   entity there and reads it with the stock `unit*` Lua functions.
//! - The local camera actor: `unitPosition`'s fallback reads
//!   `[[[[g_pGLevel + 0x50] + 0x3C]] + 0x35C]` (`APlayerController`
//!   helper `0x0054d8d0`, `g_pGLevel` at `0x01EE2684`).

use super::geometry::{rotator_to_rad, Pose, Vec3};
#[cfg(test)]
use crate::supervisor::entity_table::walk_tree;
use crate::supervisor::entity_table::{self, Memory};
use std::collections::HashSet;

/// Offset of `AActor::Location` (then `Rotation` at +0xC).
pub const ACTOR_LOCATION_OFF: u32 = 0xDC;
/// Bytes read for a pose: Location (3 x f32) + Rotation (3 x i32).
pub const POSE_LEN: u32 = 0x18;
/// The slot -> entity id map inside the `GameEntityManager`.
pub const SLOT_MAP_OFF: u32 = 0x130;
/// Unslid VA of the slot writer (thiscall: manager, slot, entity id).
pub const SET_UNIT_SLOT_VA: u32 = 0x00C6_7BD0;
/// Unslid VA of the `g_pGLevel` pointer.
pub const GLEVEL_PTR_VA: u32 = 0x01EE_2684;
/// First private unit slot. Stock slots are small (Pet1 = 10 .. Dialog =
/// 17, DialogSpeaker = 18); this range is the lab's alone.
pub const LAB_SLOT_BASE: i32 = 7700;
/// How many private slots `client_entity_find` may pin in one call.
pub const LAB_SLOT_COUNT: usize = 48;
/// Max slot-map entries read by the test-only full walk.
#[cfg(test)]
const SLOT_MAP_MAX: usize = 256;
/// Bound a corrupt search path without rejecting a valid large slot map.
const SLOT_SEARCH_MAX_DEPTH: usize = 256;

fn u32_at(b: &[u8], off: usize) -> u32 {
    b.get(off..off + 4)
        .map(|s| u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
        .unwrap_or(0)
}

fn f32_at(b: &[u8], off: usize) -> f32 {
    f32::from_bits(u32_at(b, off))
}

/// Decode a [`POSE_LEN`] read at `actor + ACTOR_LOCATION_OFF`. A
/// non-finite or absurd location (a freed actor) is an error, so a caller
/// re-resolves the actor instead of steering by garbage.
pub fn decode_pose(b: &[u8]) -> Result<Pose, String> {
    if b.len() < POSE_LEN as usize {
        return Err(format!("short pose read ({} bytes)", b.len()));
    }
    let pos = Vec3::new(
        f32_at(b, 0) as f64,
        f32_at(b, 4) as f64,
        f32_at(b, 8) as f64,
    );
    if !pos.is_finite() || pos.x.abs() > 1.0e7 || pos.y.abs() > 1.0e7 || pos.z.abs() > 1.0e7 {
        return Err(format!("implausible actor location {pos:?}"));
    }
    Ok(Pose {
        pos,
        pitch: rotator_to_rad(u32_at(b, 0xC) as i32),
        yaw: rotator_to_rad(u32_at(b, 0x10) as i32),
    })
}

/// Read an actor's pose.
pub async fn actor_pose<M: Memory>(mem: &mut M, actor: u32) -> Result<Pose, String> {
    if actor == 0 {
        return Err("no actor".into());
    }
    decode_pose(&mem.read(actor + ACTOR_LOCATION_OFF, POSE_LEN).await?)
        .map_err(|e| format!("actor {actor:#x}: {e}"))
}

/// One entity of the client's entity map, with its pose when it has an actor.
#[derive(Debug, Clone, PartialEq)]
pub struct WorldEntity {
    pub id: u32,
    pub ptr: u32,
    pub actor: u32,
    /// The entity's "rendered once" flag bit (see `entity_table`).
    pub rendered_flag: bool,
    pub pose: Option<Pose>,
    pub pose_error: Option<String>,
}

impl WorldEntity {
    /// Has an actor in the scene (the client draws it).
    pub fn rendered(&self) -> bool {
        self.actor != 0
    }
}

/// The client's entities, from one pass of reads.
#[derive(Debug, Clone)]
pub struct WorldSnapshot {
    pub manager: u32,
    pub player_id: u32,
    pub entities: Vec<WorldEntity>,
    pub truncated: bool,
}

impl WorldSnapshot {
    pub fn entity(&self, id: u32) -> Option<&WorldEntity> {
        self.entities.iter().find(|e| e.id == id)
    }

    pub fn player(&self) -> Option<&WorldEntity> {
        self.entity(self.player_id)
    }
}

/// The entity id in `slot`, or 0 when the slot is unmapped.
pub fn slot_entity(slots: &[(i32, u32)], slot: i32) -> u32 {
    slots
        .iter()
        .find(|(s, _)| *s == slot)
        .map(|(_, e)| *e)
        .unwrap_or(0)
}

/// Read the slot map of the manager at `manager`.
#[cfg(test)]
pub async fn read_slots<M: Memory>(mem: &mut M, manager: u32) -> Result<Vec<(i32, u32)>, String> {
    let hdr = mem.read(manager + SLOT_MAP_OFF + 4, 8).await?;
    let (head, size) = (u32_at(&hdr, 0), u32_at(&hdr, 4));
    let walk = walk_tree(mem, head, size, SLOT_MAP_MAX).await?;
    Ok(walk
        .entries
        .into_iter()
        .map(|(k, v)| (k as i32, v))
        .collect())
}

/// Resolve only the requested unit slots through the map's search tree.
/// Hover needs three stock slots, while entity discovery can pin 48 private
/// slots; walking the entire map on every cursor move can starve the bridge.
pub async fn read_selected_slots<M: Memory>(
    mem: &mut M,
    manager: u32,
    keys: &[i32],
) -> Result<Vec<(i32, u32)>, String> {
    let head = u32_at(&mem.read(manager + SLOT_MAP_OFF + 4, 4).await?, 0);
    if head == 0 {
        return Err("slot map head is null".into());
    }
    let root = entity_table::Node::decode(&mem.read(head, entity_table::NODE_LEN).await?)?.parent;
    let mut result = Vec::with_capacity(keys.len());
    for &key in keys {
        let mut cur = root;
        let mut seen = HashSet::new();
        while cur != head && cur != 0 {
            if seen.len() >= SLOT_SEARCH_MAX_DEPTH || !seen.insert(cur) {
                return Err(format!("slot map search cycle or depth limit at {cur:#x}"));
            }
            let node = entity_table::Node::decode(&mem.read(cur, entity_table::NODE_LEN).await?)?;
            if node.is_nil {
                break;
            }
            if key == node.key as i32 {
                result.push((key, node.value));
                break;
            }
            cur = if key < node.key as i32 {
                node.left
            } else {
                node.right
            };
        }
    }
    Ok(result)
}

/// Resolve the `GameEntityManager*` (null means "not in the world").
pub async fn manager<M: Memory>(mem: &mut M, slide: i64) -> Result<u32, String> {
    let va = (entity_table::MANAGER_PTR_VA as i64 + slide) as u32;
    match u32_at(&mem.read(va, 4).await?, 0) {
        0 => Err("GameEntityManager is null (not in the world yet?)".into()),
        m => Ok(m),
    }
}

/// Walk the entity map and read every rendered entity's pose. About three reads per entity; callers that loop (movement)
/// read one actor per tick instead and call this only to (re)resolve.
pub async fn snapshot<M: Memory>(
    mem: &mut M,
    slide: i64,
    max_nodes: usize,
) -> Result<WorldSnapshot, String> {
    let t = entity_table::read_table(mem, slide, None, max_nodes).await?;
    let mut entities = Vec::with_capacity(t.details.len());
    for (id, ptr, fields) in &t.details {
        let (actor, rendered_flag) = match fields {
            Ok(f) => (f.actor, f.flags & entity_table::RENDERED_FLAG != 0),
            Err(_) => (0, false),
        };
        let (pose, pose_error) = if actor == 0 {
            (None, None)
        } else {
            match actor_pose(mem, actor).await {
                Ok(p) => (Some(p), None),
                Err(e) => (None, Some(e)),
            }
        };
        entities.push(WorldEntity {
            id: *id,
            ptr: *ptr,
            actor,
            rendered_flag,
            pose,
            pose_error,
        });
    }
    Ok(WorldSnapshot {
        manager: t.manager,
        player_id: t.header.player_id,
        entities,
        truncated: t.entities.truncated,
    })
}

/// `ASGWController_Player` vtable (findings `cegui-mouse-input-feed.md` §10.2).
pub const PLAYER_CONTROLLER_VTABLE_VA: u32 = 0x019E_2B2C;
/// `ASGWCamera_Player` vtable (§10.3).
pub const PLAYER_CAMERA_VTABLE_VA: u32 = 0x019E_1134;
/// `ASGWController_Player + 0x2b0`: its `ASGWCamera_Player*`.
pub const CONTROLLER_CAMERA_OFF: u32 = 0x2B0;
/// Most level actors scanned for the controller (live: 0x32).
const MAX_LEVEL_ACTORS: u32 = 8192;

/// Camera fields (§10.3): look gain, distance (zoom), pitch and yaw offsets.
pub const CAMERA_GAIN_OFF: u32 = 0x358;
pub const CAMERA_DIST_OFF: u32 = 0x360;
pub const CAMERA_PITCH_OFF: u32 = 0x368;
pub const CAMERA_YAW_OFF: u32 = 0x36C;
/// Third-person zoom limits and the step per notch (§10.3).
pub const CAMERA_DIST_MIN: f32 = 100.0;
pub const CAMERA_DIST_MAX: f32 = 775.0;
pub const CAMERA_ZOOM_STEP: f32 = 30.0;
/// The camera's own handlers (§10.4), all `thiscall` on the camera:
/// zoom in / out (`void()`), turn yaw / pitch (`void(float counts)`).
pub const CAMERA_ZOOM_IN_VA: u32 = 0x00E7_E560;
pub const CAMERA_ZOOM_OUT_VA: u32 = 0x00E7_E870;
pub const CAMERA_TURN_YAW_VA: u32 = 0x00E7_E600;
pub const CAMERA_TURN_PITCH_VA: u32 = 0x00E7_E590;

fn rebase(va: u32, slide: i64) -> u32 {
    (va as i64 + slide) as u32
}

/// The local player's controller and camera.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerCamera {
    pub controller: u32,
    pub camera: u32,
}

/// Whether `ptr`'s first word is the vtable `va` (rebased).
pub async fn has_vtable<M: Memory>(mem: &mut M, ptr: u32, va: u32, slide: i64) -> bool {
    ptr != 0
        && mem
            .read(ptr, 4)
            .await
            .is_ok_and(|b| u32_at(&b, 0) == rebase(va, slide))
}

/// Find the local `ASGWController_Player` by scanning the level's Actors
/// array for its vtable, then its camera (`+0x2b0`), checked by vtable.
/// The old chain (`[[[[g_pGLevel+0x50]+0x3C]]+0x35C]`, `0x0054d8d0`) reads
/// a WorldInfo field that is 0 in a normal game (§10.1). Callers cache the
/// result and re-check both vtables before each use.
pub async fn find_player_camera<M: Memory>(
    mem: &mut M,
    slide: i64,
) -> Result<PlayerCamera, String> {
    let world = u32_at(&mem.read(rebase(GLEVEL_PTR_VA, slide), 4).await?, 0);
    if world == 0 {
        return Err("g_pGLevel is null (no world loaded)".into());
    }
    let level = u32_at(&mem.read(world + 0x50, 4).await?, 0);
    if level == 0 {
        return Err("UWorld has no level".into());
    }
    let hdr = mem.read(level + 0x3C, 8).await?;
    let (data, count) = (u32_at(&hdr, 0), u32_at(&hdr, 4).min(MAX_LEVEL_ACTORS));
    if data == 0 || count == 0 {
        return Err("the level's Actors array is empty".into());
    }
    let actors = mem.read(data, count * 4).await?;
    let want = rebase(PLAYER_CONTROLLER_VTABLE_VA, slide);
    for i in 0..count as usize {
        let a = u32_at(&actors, i * 4);
        if a == 0 {
            continue;
        }
        let Ok(vt) = mem.read(a, 4).await else {
            continue;
        };
        if u32_at(&vt, 0) != want {
            continue;
        }
        let camera = u32_at(&mem.read(a + CONTROLLER_CAMERA_OFF, 4).await?, 0);
        if !has_vtable(mem, camera, PLAYER_CAMERA_VTABLE_VA, slide).await {
            return Err(format!(
                "player controller {a:#x} has no ASGWCamera_Player at +0x2b0 ({camera:#x})"
            ));
        }
        return Ok(PlayerCamera {
            controller: a,
            camera,
        });
    }
    Err(format!(
        "no ASGWController_Player among {count} level actors"
    ))
}

/// The camera's readable state.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CameraState {
    /// Third-person distance (zoom), 100..775.
    pub distance: f32,
    /// Yaw and pitch offsets from the pawn, rotator units (65536 per turn).
    pub yaw: i32,
    pub pitch: i32,
    /// Look gain: rotator units per mouse count.
    pub gain: f32,
    /// `+0x348` flags: bit 2 (0x4) inverts yaw, bit 3 (0x8) inverts pitch
    /// (findings §10.3).
    pub flags: u32,
}

impl CameraState {
    /// Rotator units the pitch handler adds per count: `gain * inv`, where
    /// `inv` is -1 when the pitch-invert flag (0x8) is set.
    pub fn pitch_gain(self) -> f32 {
        if self.flags & 0x8 != 0 {
            -self.gain
        } else {
            self.gain
        }
    }

    /// Rotator units the yaw handler adds per count: `gain * inv`, where
    /// `inv` is -1 when the yaw-invert flag (0x4) is set.
    pub fn yaw_gain(self) -> f32 {
        if self.flags & 0x4 != 0 {
            -self.gain
        } else {
            self.gain
        }
    }

    pub fn to_json(self) -> serde_json::Value {
        serde_json::json!({
            "zoom": self.distance,
            "yaw_offset_deg": f64::from(self.yaw) * 360.0 / 65536.0,
            "pitch_offset_deg": f64::from(self.pitch) * 360.0 / 65536.0,
            "gain": self.gain,
            "pitch_gain": self.pitch_gain(),
            "yaw_gain": self.yaw_gain(),
        })
    }
}

/// `ASGWCamera_Player + 0x348`: the invert flags (see [`CameraState`]).
pub const CAMERA_FLAGS_OFF: u32 = 0x348;

/// Read the camera's flags, gain, distance, pitch and yaw (one read).
pub async fn camera_state<M: Memory>(mem: &mut M, camera: u32) -> Result<CameraState, String> {
    let b = mem
        .read(
            camera + CAMERA_FLAGS_OFF,
            CAMERA_YAW_OFF + 4 - CAMERA_FLAGS_OFF,
        )
        .await?;
    let at = |off: u32| (off - CAMERA_FLAGS_OFF) as usize;
    Ok(CameraState {
        flags: u32_at(&b, at(CAMERA_FLAGS_OFF)),
        gain: f32_at(&b, at(CAMERA_GAIN_OFF)),
        distance: f32_at(&b, at(CAMERA_DIST_OFF)),
        pitch: u32_at(&b, at(CAMERA_PITCH_OFF)) as i32,
        yaw: u32_at(&b, at(CAMERA_YAW_OFF)) as i32,
    })
}

/// Largest pitch offset the lab turns to: 78.75 degrees. The findings
/// read the field as clamped to +-0x4000, but the stored offset is not
/// clamped by the turn handler: live it reached 1563 degrees after repeated
/// pitch turns and the view ended up looking straight down (2026-10-10).
pub const CAMERA_PITCH_LIMIT: i32 = 0x3800;
/// [`CAMERA_PITCH_LIMIT`] in readout degrees (`pitch_offset_deg`).
pub const CAMERA_PITCH_LIMIT_DEG: f64 = 78.75;
/// A readout this close to the limit, or past it, is "pinned".
pub const PITCH_PINNED_MARGIN_DEG: f64 = 0.5;

/// Is a readout's `pitch_offset_deg` at the pitch limit? There the lab's
/// clamp ([`clamped_pitch_counts`]) turns every further push into that
/// direction into no call at all, so a closed loop that pushes that way
/// never moves the pixel and never learns anything.
pub fn pitch_pinned(pitch_offset_deg: f64) -> bool {
    pitch_offset_deg.is_finite()
        && pitch_offset_deg.abs() >= CAMERA_PITCH_LIMIT_DEG - PITCH_PINNED_MARGIN_DEG
}

/// The camera actor's pose, or `None` when its location is exactly zero: the
/// client never wrote one (seen on a second lab instance, 2026-10-12), and a
/// zero pose would make every bearing from the camera wrong.
pub fn real_camera_pose(p: Pose) -> Option<Pose> {
    (p.pos != Vec3::new(0.0, 0.0, 0.0)).then_some(p)
}

/// The pitch counts to pass to the turn handler so the offset moves by
/// `dy` counts but ends inside +-[`CAMERA_PITCH_LIMIT`]. A pitch already
/// outside the limit is brought back to it. Zero means "do not call".
pub fn clamped_pitch_counts(pitch: i32, gain: f32, dy: f32) -> f32 {
    if !gain.is_finite() || gain.abs() < 1e-3 {
        return dy;
    }
    let lim = CAMERA_PITCH_LIMIT as f32;
    let want = (pitch as f32 + gain * dy).clamp(-lim, lim);
    let counts = (want - pitch as f32) / gain;
    if counts.abs() < 0.01 {
        0.0
    } else {
        counts
    }
}

/// Zoom notches (positive = in) that bring `from` closest to `to`.
pub fn zoom_notches(from: f32, to: f32) -> i32 {
    let to = to.clamp(CAMERA_DIST_MIN, CAMERA_DIST_MAX);
    ((from - to) / CAMERA_ZOOM_STEP).round() as i32
}

#[cfg(test)]
#[path = "memory_tests.rs"]
mod tests;
