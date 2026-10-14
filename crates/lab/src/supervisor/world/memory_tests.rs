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

struct CountingFake {
    mem: Fake,
    reads: usize,
}

impl Memory for CountingFake {
    async fn read(&mut self, addr: u32, len: u32) -> Result<Vec<u8>, String> {
        self.reads += 1;
        self.mem.read(addr, len).await
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

#[tokio::test]
async fn selected_slots_work_with_large_private_slot_maps() {
    let (mgr, head) = (0x1000u32, 0x2000u32);
    let mut keys = vec![1u32, 2, 14];
    keys.extend(7700..8000);
    assert!(keys.len() > SLOT_MAP_MAX);
    let addresses: Vec<u32> = (0..keys.len()).map(|i| 0x3000 + i as u32 * 0x100).collect();
    fn build(
        lo: usize,
        hi: usize,
        parent: u32,
        keys: &[u32],
        addresses: &[u32],
        map: &mut std::collections::HashMap<u32, Vec<u8>>,
        head: u32,
    ) -> u32 {
        if lo == hi {
            return head;
        }
        let mid = (lo + hi) / 2;
        let addr = addresses[mid];
        let left = build(lo, mid, addr, keys, addresses, map, head);
        let right = build(mid + 1, hi, addr, keys, addresses, map, head);
        map.insert(
            addr,
            node(left, parent, right, keys[mid], keys[mid] + 100, false),
        );
        addr
    }
    let mut map = std::collections::HashMap::new();
    let root = build(0, keys.len(), head, &keys, &addresses, &mut map, head);
    map.insert(
        head,
        node(addresses[0], root, *addresses.last().unwrap(), 0, 0, true),
    );
    map.insert(mgr + SLOT_MAP_OFF + 4, words(&[head, keys.len() as u32]));
    let mut mem = CountingFake {
        mem: Fake(map),
        reads: 0,
    };
    let selected = read_selected_slots(&mut mem, mgr, &[1, 2, 14])
        .await
        .unwrap();
    assert_eq!(selected, vec![(1, 101), (2, 102), (14, 114)]);
    assert!(mem.reads < 30, "{} bridge reads for three slots", mem.reads);
}

#[tokio::test]
async fn selected_slots_reject_a_search_cycle() {
    let (mgr, head, root) = (0x1000u32, 0x2000u32, 0x3000u32);
    let mut map = std::collections::HashMap::new();
    map.insert(mgr + SLOT_MAP_OFF + 4, words(&[head, 500]));
    map.insert(head, node(root, root, root, 0, 0, true));
    map.insert(root, node(root, head, head, 10, 42, false));
    let err = read_selected_slots(&mut Fake(map), mgr, &[1])
        .await
        .unwrap_err();
    assert!(err.contains("cycle or depth limit"), "{err}");
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
    assert_eq!(s.yaw_gain(), 20.0);
    assert_eq!(CameraState { flags: 0x4, ..s }.yaw_gain(), -20.0);

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

#[test]
fn a_pitch_at_the_limit_is_pinned() {
    assert_eq!(
        CAMERA_PITCH_LIMIT_DEG,
        f64::from(CAMERA_PITCH_LIMIT) * 360.0 / 65536.0
    );
    assert!(pitch_pinned(78.75));
    assert!(pitch_pinned(-78.4));
    assert!(pitch_pinned(1563.0));
    assert!(!pitch_pinned(78.0));
    assert!(!pitch_pinned(0.0));
    assert!(!pitch_pinned(f64::NAN));
}

#[test]
fn an_all_zero_camera_location_is_not_a_pose() {
    let at = |x, y, z| Pose {
        pos: Vec3::new(x, y, z),
        yaw: 1.0,
        pitch: 0.0,
    };
    assert_eq!(real_camera_pose(at(0.0, 0.0, 0.0)), None);
    assert!(real_camera_pose(at(0.0, 0.0, 5.0)).is_some());
}
