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

/// The local camera/controller actor through `g_pGLevel` (see module doc).
pub async fn camera_actor<M: Memory>(mem: &mut M, slide: i64) -> Result<u32, String> {
    let va = (GLEVEL_PTR_VA as i64 + slide) as u32;
    let mut p = u32_at(&mem.read(va, 4).await?, 0);
    for (i, off) in [0x50u32, 0x3C, 0x0, 0x35C].into_iter().enumerate() {
        if p == 0 {
            return Err(format!("camera chain: null at hop {i}"));
        }
        p = u32_at(&mem.read(p + off, 4).await?, 0);
    }
    if p == 0 {
        return Err("camera chain: null actor".into());
    }
    Ok(p)
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

    /// The camera chain follows four pointers from `g_pGLevel`.
    #[tokio::test]
    async fn camera_chain_follows_the_level_pointers() {
        let slide = 0x10000i64;
        let g = (GLEVEL_PTR_VA as i64 + slide) as u32;
        let mut m = std::collections::HashMap::new();
        m.insert(g, words(&[0x5000]));
        m.insert(0x5000 + 0x50, words(&[0x6000]));
        m.insert(0x6000 + 0x3C, words(&[0x7000]));
        m.insert(0x7000, words(&[0x8000]));
        m.insert(0x8000 + 0x35C, words(&[0x9000]));
        assert_eq!(camera_actor(&mut Fake(m.clone()), slide).await, Ok(0x9000));
        m.insert(0x6000 + 0x3C, words(&[0]));
        assert!(camera_actor(&mut Fake(m), slide)
            .await
            .unwrap_err()
            .contains("hop 2"));
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
