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
use crate::supervisor::entity_table::{self, walk_tree, Memory};

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
/// Max slot-map entries read (the stock map holds a few dozen).
const SLOT_MAP_MAX: usize = 256;

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

    pub fn to_json(self) -> serde_json::Value {
        serde_json::json!({
            "zoom": self.distance,
            "yaw_offset_deg": f64::from(self.yaw) * 360.0 / 65536.0,
            "pitch_offset_deg": f64::from(self.pitch) * 360.0 / 65536.0,
            "gain": self.gain,
            "pitch_gain": self.pitch_gain(),
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
mod tests {
    use super::*;

    /// Byte-addressed fake client memory.
    struct Fake(std::collections::HashMap<u32, Vec<u8>>);

    impl Memory for Fake {
        async fn read(&mut self, addr: u32, len: u32) -> Result<Vec<u8>, String> {
            let base = self
                .0
                .iter()
                .find(|(a, b)| **a <= addr && addr + len <= **a + b.len() as u32)
                .ok_or_else(|| format!("unmapped {addr:#x}+{len}"))?;
            let off = (addr - base.0) as usize;
            Ok(base.1[off..off + len as usize].to_vec())
        }
    }

    fn words(ws: &[u32]) -> Vec<u8> {
        ws.iter().flat_map(|w| w.to_le_bytes()).collect()
    }

    fn node(left: u32, parent: u32, right: u32, key: u32, value: u32, nil: bool) -> Vec<u8> {
        let mut b = words(&[left, parent, right, key, value]);
        b.extend_from_slice(&[0, nil as u8, 0, 0]);
        b
    }

    /// The slot map is the manager's `std::map<int, int>` at +0x130: head
    /// pointer at +0x134, size at +0x138. Two slots: Target (1) -> 42,
    /// MouseOver (14) -> 0 (unmapped reads as 0).
    #[tokio::test]
    async fn slot_map_walks_the_managers_tree() {
        let (mgr, head, a, b) = (0x1000u32, 0x2000u32, 0x3000u32, 0x4000u32);
        let mut m = std::collections::HashMap::new();
        m.insert(mgr + SLOT_MAP_OFF + 4, words(&[head, 2]));
        m.insert(head, node(a, a, b, 0, 0, true));
        m.insert(a, node(head, head, b, 1, 42, false));
        m.insert(b, node(head, a, head, 14, 0, false));
        let slots = read_slots(&mut Fake(m), mgr).await.unwrap();
        assert_eq!(slots, vec![(1, 42), (14, 0)]);
        assert_eq!(slot_entity(&slots, 1), 42);
    }

    /// Regression guard (2026-10-10): the camera is found by scanning the
    /// level's actors for the player controller's vtable, not by the old
    /// WorldInfo+0x35c chain (0 live). Actors[0] is a WorldInfo whose
    /// +0x35c is 0, so the old chain would fail on this layout.
    #[tokio::test]
    async fn the_camera_is_found_by_scanning_for_the_player_controller() {
        let slide = 0x10000i64;
        let vt = |va: u32| words(&[rebase(va, slide)]);
        let g = rebase(GLEVEL_PTR_VA, slide);
        let (world, level, data, info, other, pc, cam) = (
            0x5000u32, 0x6000u32, 0x7000u32, 0x8000u32, 0x8800u32, 0x9000u32, 0xA000u32,
        );
        let mut m = std::collections::HashMap::new();
        m.insert(g, words(&[world]));
        m.insert(world + 0x50, words(&[level]));
        m.insert(level + 0x3C, words(&[data, 3]));
        m.insert(data, words(&[info, other, pc]));
        m.insert(info, words(&[0x0018_a02b4]));
        m.insert(other, words(&[0x1234]));
        m.insert(pc, vt(PLAYER_CONTROLLER_VTABLE_VA));
        m.insert(pc + CONTROLLER_CAMERA_OFF, words(&[cam]));
        m.insert(cam, vt(PLAYER_CAMERA_VTABLE_VA));
        // From +0x348: flags (pitch inverted), three floats, then gain,
        // +0x35c, distance, +0x364, pitch, yaw.
        let mut state = 0x8u32.to_le_bytes().to_vec();
        for v in [10.0f32, 3.0, 0.0, 20.0, 0.0, 250.0, 0.0] {
            state.extend_from_slice(&v.to_le_bytes());
        }
        state.extend_from_slice(&(-0x2000i32).to_le_bytes());
        state.extend_from_slice(&0x4000i32.to_le_bytes());
        m.insert(cam + CAMERA_FLAGS_OFF, state);
        let found = find_player_camera(&mut Fake(m.clone()), slide)
            .await
            .unwrap();
        assert_eq!(
            found,
            PlayerCamera {
                controller: pc,
                camera: cam
            }
        );
        let s = camera_state(&mut Fake(m.clone()), cam).await.unwrap();
        assert_eq!(
            (s.gain, s.distance, s.pitch, s.yaw),
            (20.0, 250.0, -0x2000, 0x4000)
        );
        assert_eq!(s.to_json()["yaw_offset_deg"], 90.0);
        // Regression guard (review of #1309): the pitch-invert flag flips
        // the per-count gain the clamp uses.
        assert_eq!(s.flags, 0x8);
        assert_eq!(s.pitch_gain(), -20.0);
        assert_eq!(CameraState { flags: 0x3, ..s }.pitch_gain(), 20.0);

        // A camera pointer that is not an ASGWCamera_Player is refused.
        m.insert(cam, words(&[0xdead]));
        let e = find_player_camera(&mut Fake(m.clone()), slide)
            .await
            .unwrap_err();
        assert!(e.contains("no ASGWCamera_Player"), "{e}");
        // No controller in the level.
        m.insert(pc, words(&[0xbeef]));
        let e = find_player_camera(&mut Fake(m), slide).await.unwrap_err();
        assert!(e.contains("no ASGWController_Player among 3"), "{e}");
    }

    /// Regression guard (2026-10-10): repeated pitch turns ran the offset to
    /// 1563 degrees. Each turn is clamped, and a wild pitch is pulled back.
    #[test]
    fn pitch_turns_stay_inside_the_limit() {
        let lim = CAMERA_PITCH_LIMIT;
        // Room to move: unchanged.
        assert_eq!(clamped_pitch_counts(0, 20.0, -100.0), -100.0);
        // Would overshoot: cut to land on the limit.
        let c = clamped_pitch_counts(lim - 200, 20.0, 600.0);
        assert!((c - 10.0).abs() < 1e-3, "{c}");
        // At the limit, pushing further: no call.
        assert_eq!(clamped_pitch_counts(lim, 20.0, 50.0), 0.0);
        // Already far outside (the live 1563 degrees): pulled back, even
        // when asked to go further out.
        let wild = (1563.79 * 65536.0 / 360.0) as i32;
        let back = clamped_pitch_counts(wild, 20.0, 600.0);
        assert!(
            (wild as f32 + 20.0 * back - lim as f32).abs() < 1.0,
            "{back}"
        );
    }

    #[test]
    fn zoom_notches_round_to_the_30_unit_step_and_clamp() {
        assert_eq!(zoom_notches(250.0, 100.0), 5);
        assert_eq!(zoom_notches(250.0, 400.0), -5);
        assert_eq!(zoom_notches(250.0, 2000.0), -18);
        assert_eq!(zoom_notches(250.0, 260.0), 0);
    }

    fn pose_bytes(x: f32, y: f32, z: f32, pitch: i32, yaw: i32) -> Vec<u8> {
        let mut b = Vec::new();
        for v in [x, y, z] {
            b.extend_from_slice(&v.to_le_bytes());
        }
        for v in [pitch, yaw, 0] {
            b.extend_from_slice(&v.to_le_bytes());
        }
        b
    }

    #[test]
    fn pose_decodes_location_then_rotator() {
        let p = decode_pose(&pose_bytes(100.5, -200.0, 50.0, 0, 16384)).unwrap();
        assert_eq!(p.pos, Vec3::new(100.5, -200.0, 50.0));
        assert!((p.yaw - std::f64::consts::FRAC_PI_2).abs() < 1e-9);
    }

    /// A freed actor reads as garbage: refuse it rather than steer by it.
    #[test]
    fn pose_refuses_garbage() {
        assert!(decode_pose(&pose_bytes(f32::NAN, 0.0, 0.0, 0, 0)).is_err());
        assert!(decode_pose(&pose_bytes(3.0e9, 0.0, 0.0, 0, 0)).is_err());
        assert!(decode_pose(&[0u8; 8]).is_err());
    }

    #[test]
    fn slot_lookup_defaults_to_unmapped() {
        let slots = [(1, 100u32), (2, 0), (7700, 555)];
        assert_eq!(slot_entity(&slots, 1), 100);
        assert_eq!(slot_entity(&slots, 7700), 555);
        assert_eq!(slot_entity(&slots, 3), 0);
    }
}
