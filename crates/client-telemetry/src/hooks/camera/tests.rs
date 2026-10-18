//! Unit tests for the camera and pawn-pose readers, over a fake address space.

use std::sync::atomic::AtomicU32;

use super::layout::{
    ACTOR_LOCATION_OFF, CAMERA_BLOCK_LEN, CAMERA_DIST_OFF, CAMERA_FLAGS_OFF, CAMERA_GAIN_OFF,
    CAMERA_YAW_OFF, CONTROLLER_CAMERA_OFF, ENTITY_MANAGER_VA, ENTITY_PAWN_OFF, GLEVEL_PTR_VA,
    MANAGER_LOCAL_ENTITY_OFF, PLAYER_CAMERA_VTABLE_VA, PLAYER_CONTROLLER_VTABLE_VA,
};
use super::read::{
    camera_ptr, client_to_server, decode_camera, find_player_camera, read_camera, read_camera_pose,
    read_pawn_pose, rot_deg, rot_deg_360, ActorPose, CameraState,
};
use crate::hooks::entity_trace::map::fake::FakeMem;

/// Virtual addresses for the fake world: a `UWorld`, its level, the Actors array, and actors.
const WORLD: u32 = 0x2000_0000;
const LEVEL: u32 = 0x2000_1000;
const ACTORS: u32 = 0x2000_2000;
const ACTOR_BASE: u32 = 0x2100_0000;
const CAMERA: u32 = 0x2200_0000;
const CAMERA_2: u32 = 0x2200_1000;

/// Write a little-endian `u32` into a byte buffer at `off`.
fn put_u32(b: &mut [u8], off: usize, v: u32) {
    b[off..off + 4].copy_from_slice(&v.to_le_bytes());
}

/// Lay out one actor per slot in the level's Actors array, and point the global at the world.
fn build_world(mem: &mut FakeMem, actors: &[u32]) {
    mem.set(GLEVEL_PTR_VA, WORLD);
    mem.set(WORLD + 0x50, LEVEL);
    mem.set(LEVEL + 0x3c, ACTORS);
    mem.set(LEVEL + 0x40, actors.len() as u32);
    for (i, a) in actors.iter().enumerate() {
        mem.set(ACTORS + 4 * i as u32, *a);
    }
}

/// An `ASGWController_Player` at `at` whose camera pointer is `camera`.
fn controller(mem: &mut FakeMem, at: u32, camera: u32) {
    mem.set(at, PLAYER_CONTROLLER_VTABLE_VA);
    mem.set(at + CONTROLLER_CAMERA_OFF, camera);
}

/// Write the six words of a pose read at `actor + ACTOR_LOCATION_OFF`.
fn put_pose(mem: &mut FakeMem, actor: u32, pos: [f32; 3], pitch: i32, yaw: i32) {
    let base = actor + ACTOR_LOCATION_OFF;
    for (i, v) in pos.iter().enumerate() {
        mem.set(base + 4 * i as u32, v.to_bits());
    }
    mem.set(base + 0xc, pitch as u32);
    mem.set(base + 0x10, yaw as u32);
    // The read is POSE_LEN bytes, so the last word must exist too.
    mem.set(base + 0x14, 0);
}

/// The decoder reads every camera field at its offset, and rejects a short block.
#[test]
fn decode_camera_reads_each_field() {
    let mut b = vec![0u8; CAMERA_BLOCK_LEN];
    put_u32(&mut b, 0, 5);
    put_u32(&mut b, 0x10, 1.5f32.to_bits());
    put_u32(&mut b, 0x18, 400.0f32.to_bits());
    put_u32(&mut b, 0x20, (-2048i32) as u32);
    put_u32(&mut b, 0x24, 16384u32);
    assert_eq!(
        decode_camera(&b),
        Some(CameraState {
            flags: 5,
            gain: 1.5,
            distance: 400.0,
            pitch: -2048,
            yaw: 16384,
        })
    );
    assert_eq!(decode_camera(&b[..CAMERA_BLOCK_LEN - 1]), None);
}

/// The block is read through `Mem` at `camera + CAMERA_FLAGS_OFF`, so one missing word gives `None`.
#[test]
fn read_camera_reads_the_block_through_mem() {
    let mut mem = FakeMem::default();
    for off in (0..CAMERA_BLOCK_LEN as u32).step_by(4) {
        mem.set(CAMERA + CAMERA_FLAGS_OFF + off, 0);
    }
    mem.set(CAMERA + CAMERA_FLAGS_OFF, 4);
    mem.set(CAMERA + CAMERA_GAIN_OFF, 2.0f32.to_bits());
    mem.set(CAMERA + CAMERA_DIST_OFF, 300.0f32.to_bits());
    mem.set(CAMERA + CAMERA_YAW_OFF, (-1i32) as u32);
    assert_eq!(
        read_camera(&mem, CAMERA),
        Some(CameraState {
            flags: 4,
            gain: 2.0,
            distance: 300.0,
            pitch: 0,
            yaw: -1,
        })
    );

    mem.words.remove(&(CAMERA + CAMERA_YAW_OFF));
    assert_eq!(read_camera(&mem, CAMERA), None);
}

/// Rotator units convert at 65536 per turn, and the wrapped form stays in [0, 360).
#[test]
fn rotator_units_to_degrees() {
    assert_eq!(rot_deg(16384), 90.0);
    assert_eq!(rot_deg(-2048), -11.25);
    assert_eq!(rot_deg_360(-16384), 270.0);
}

/// The server frame is client x and y swapped and rescaled by 100, with z moved to the end.
#[test]
fn client_to_server_swaps_axes() {
    assert_eq!(client_to_server([100.0, 200.0, 300.0]), [2.0, 3.0, 1.0]);
}

/// The scan finds the camera on the second actor, and rejects a camera with the wrong vtable.
#[test]
fn finds_the_camera_through_the_level() {
    let mut mem = FakeMem::default();
    let (a0, a1, a2) = (ACTOR_BASE, ACTOR_BASE + 0x1000, ACTOR_BASE + 0x2000);
    build_world(&mut mem, &[a0, a1, a2]);
    mem.set(a0, 0x1234); // an actor that is not a controller
    controller(&mut mem, a1, CAMERA);
    mem.set(CAMERA, PLAYER_CAMERA_VTABLE_VA);
    mem.set(a2, 0x1234);
    assert_eq!(find_player_camera(&mem), Some(CAMERA));

    mem.set(CAMERA, 0); // the camera vtable no longer matches
    assert_eq!(find_player_camera(&mem), None);
}

/// The cache is rechecked by vtable, so a moved camera is found again by a rescan.
#[test]
fn cache_rechecks_the_vtable() {
    let mut mem = FakeMem::default();
    build_world(&mut mem, &[ACTOR_BASE]);
    controller(&mut mem, ACTOR_BASE, CAMERA);
    mem.set(CAMERA, PLAYER_CAMERA_VTABLE_VA);
    let cache = AtomicU32::new(0);
    assert_eq!(camera_ptr(&mem, &cache), Some(CAMERA));

    // The first camera is gone; the controller now points at a second one.
    mem.set(CAMERA, 0);
    mem.set(ACTOR_BASE + CONTROLLER_CAMERA_OFF, CAMERA_2);
    mem.set(CAMERA_2, PLAYER_CAMERA_VTABLE_VA);
    assert_eq!(camera_ptr(&mem, &cache), Some(CAMERA_2));
}

/// A zero camera location gives no pose, and a non-zero one gives a pose.
#[test]
fn zero_camera_pose_is_none() {
    let mut mem = FakeMem::default();
    put_pose(&mut mem, CAMERA, [0.0; 3], 0, 0);
    assert_eq!(read_camera_pose(&mem, CAMERA), None);

    put_pose(&mut mem, CAMERA, [1.0, 0.0, 0.0], 0, 0);
    assert!(read_camera_pose(&mem, CAMERA).is_some());
}

/// The pawn pose follows the manager, the entity and the pawn, and a null link gives `None`.
#[test]
fn pawn_pose_follows_the_manager() {
    let (entity, pawn) = (ACTOR_BASE + 0x100, ACTOR_BASE + 0x200);
    let mut mem = FakeMem::default();
    mem.set(ENTITY_MANAGER_VA, ACTOR_BASE);
    mem.set(ACTOR_BASE + MANAGER_LOCAL_ENTITY_OFF, entity);
    mem.set(entity + ENTITY_PAWN_OFF, pawn);
    put_pose(&mut mem, pawn, [100.0, 200.0, 300.0], 1, 2);
    assert_eq!(
        read_pawn_pose(&mem),
        Some(ActorPose {
            pos: [100.0, 200.0, 300.0],
            pitch: 1,
            yaw: 2,
        })
    );

    // Each null link breaks the path.
    mem.set(entity + ENTITY_PAWN_OFF, 0);
    assert_eq!(read_pawn_pose(&mem), None);
    mem.set(entity + ENTITY_PAWN_OFF, pawn);
    mem.set(ACTOR_BASE + MANAGER_LOCAL_ENTITY_OFF, 0);
    assert_eq!(read_pawn_pose(&mem), None);
    mem.set(ACTOR_BASE + MANAGER_LOCAL_ENTITY_OFF, entity);
    mem.set(ENTITY_MANAGER_VA, 0);
    assert_eq!(read_pawn_pose(&mem), None);
}
