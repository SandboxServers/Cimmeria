//! Unit tests for the pose sampler and the controller pointer cache, over a fake address space.

use serde_json::json;

use super::controller::InputAxes;
use super::frame::{controller_ptr, PcCache, PcWatch, PC_RESCAN_MAX_MS, PC_RESCAN_MS};
use super::pose::{
    angle_delta_deg, changed, pose_fields, PoseSampler, PoseSnapshot, POSE_PERIOD_MS, POS_EPS,
};
use crate::hooks::camera::layout::{
    GLEVEL_PTR_VA, LEVEL_ACTORS_OFF, PLAYER_CONTROLLER_VTABLE_VA, WORLD_LEVEL_OFF,
};
use crate::hooks::camera::read::find_player_controller;
use crate::hooks::entity_trace::map::fake::FakeMem;
use crate::hooks::entity_trace::map::Mem;

/// Virtual addresses for the fake world: a `UWorld`, its level, the Actors array, and the controller.
const WORLD: u32 = 0x2000_0000;
const LEVEL: u32 = 0x2000_1000;
const ACTORS: u32 = 0x2000_2000;
const PC: u32 = 0x2100_0000;

/// A pose with a pawn, a yaw just short of north, and no axes.
fn snap() -> PoseSnapshot {
    PoseSnapshot {
        zoom: 300.0,
        yaw_offset_deg: 359.5,
        pitch_offset_deg: 10.0,
        pawn: Some(([1.0, 2.0, 3.0], 90.0)),
        axes: None,
        mode: 0,
        buttons: 0,
        hover_id: 0,
    }
}

/// The first offer emits; the same snapshot again does not.
#[test]
fn first_offer_emits_and_the_same_snapshot_does_not() {
    let mut s = PoseSampler::default();
    assert!(s.offer(0, snap()));
    assert!(!s.offer(250, snap()));
}

/// A yaw move of 0.9 degrees is under the threshold; 1.5 degrees is over it.
#[test]
fn yaw_wraps_and_needs_a_full_degree() {
    let base = snap();
    let mut moved = base;
    moved.yaw_offset_deg = 0.4;
    assert!(!changed(&base, &moved));
    moved.yaw_offset_deg = 1.0;
    assert!(changed(&base, &moved));
    assert!((angle_delta_deg(359.5, 0.4) - 0.9).abs() < 1e-3);
}

/// A zoom move of 0.5 is under the threshold; 1.5 is over it.
#[test]
fn zoom_needs_a_full_unit() {
    let base = snap();
    let mut moved = base;
    moved.zoom = 300.5;
    assert!(!changed(&base, &moved));
    moved.zoom = 301.5;
    assert!(changed(&base, &moved));
}

/// The pawn going from present to absent is a change.
#[test]
fn pawn_going_none_is_a_change() {
    let base = snap();
    let mut moved = base;
    moved.pawn = None;
    assert!(changed(&base, &moved));
}

/// A hover change is a change, even when nothing else moved.
#[test]
fn hover_change_is_a_change() {
    let base = snap();
    let mut moved = base;
    moved.hover_id = 7;
    assert!(changed(&base, &moved));
}

/// An axis move past [`super::pose::AXIS_EPS`] is a change, and so is the axes going absent.
#[test]
fn axes_move_and_going_absent_are_changes() {
    let mut base = snap();
    base.axes = Some(InputAxes {
        turn: 0.0,
        fwd: 1.0,
        strafe: 0.0,
    });
    let mut moved = base;
    moved.axes = Some(InputAxes {
        turn: 0.0,
        fwd: 1.0,
        strafe: 0.02,
    });
    assert!(changed(&base, &moved));
    moved.axes = None;
    assert!(changed(&base, &moved));
}

/// Checks are due before the first one, then once a full period has passed.
#[test]
fn due_waits_one_period() {
    let mut s = PoseSampler::default();
    assert!(s.due(0));
    s.offer(0, snap());
    assert!(!s.due(POSE_PERIOD_MS - 1));
    assert!(s.due(POSE_PERIOD_MS));
}

/// `pose_fields` sets `pawn_ok` from the pawn, and omits the pawn keys without one.
#[test]
fn pose_fields_reports_pawn_ok() {
    let with = pose_fields(&snap());
    assert!(with.contains(&("pawn_ok", json!(true))));
    assert!(with.iter().any(|(k, _)| *k == "pawn_yaw_deg"));
    let mut none = snap();
    none.pawn = None;
    let without = pose_fields(&none);
    assert!(without.contains(&("pawn_ok", json!(false))));
    assert!(!without.iter().any(|(k, _)| *k == "pawn_x"));
}

/// A level with the controller as its only actor.
fn world_with_controller(mem: &mut FakeMem) {
    mem.set(GLEVEL_PTR_VA, WORLD);
    mem.set(WORLD + WORLD_LEVEL_OFF, LEVEL);
    mem.set(LEVEL + LEVEL_ACTORS_OFF, ACTORS);
    mem.set(LEVEL + LEVEL_ACTORS_OFF + 4, 1);
    mem.set(ACTORS, PC);
    mem.set(PC, PLAYER_CONTROLLER_VTABLE_VA);
}

/// A failed scan gives `None`, and holds off the next scan until the rescan period has passed.
#[test]
fn failed_scan_holds_off_the_rescan() {
    let mut mem = FakeMem::default();
    let mut cache = PcCache::default();
    assert_eq!(controller_ptr(&mem, 0, &mut cache), None);
    assert_eq!(cache.next_scan_ms, PC_RESCAN_MS);
    world_with_controller(&mut mem);
    assert_eq!(controller_ptr(&mem, PC_RESCAN_MS - 1, &mut cache), None);
    assert_eq!(controller_ptr(&mem, PC_RESCAN_MS, &mut cache), Some(PC));
    assert_eq!(cache.ptr, PC);
}

/// Repeated failed scans wait longer each time, up to the cap, and a hit resets the wait.
#[test]
fn failed_scans_back_off_to_the_cap() {
    let mut mem = FakeMem::default();
    let mut cache = PcCache::default();
    let mut now = 0;
    let mut waits = Vec::new();
    for _ in 0..5 {
        assert_eq!(controller_ptr(&mem, now, &mut cache), None);
        waits.push(cache.next_scan_ms - now);
        now = cache.next_scan_ms;
    }
    assert_eq!(
        waits,
        vec![1000, 2000, 4000, PC_RESCAN_MAX_MS, PC_RESCAN_MAX_MS]
    );
    world_with_controller(&mut mem);
    assert_eq!(controller_ptr(&mem, now, &mut cache), Some(PC));
    assert_eq!(cache.backoff_ms, 0);
}

/// The watch reports the controller appearing, changing and going, and nothing in between.
#[test]
fn pc_watch_reports_only_changes() {
    let mut w = PcWatch::new();
    assert!(w.changed(Some(PC)));
    assert!(!w.changed(Some(PC)));
    assert!(w.changed(Some(PC + 0x100)));
    assert!(w.changed(None));
    assert!(!w.changed(None));
}

/// A memory that counts bulk reads, to prove the Actors array is read in one call.
struct Counting {
    inner: FakeMem,
    bulk: std::cell::Cell<u32>,
}

impl Mem for Counting {
    fn u32_at(&self, addr: u32) -> Option<u32> {
        self.inner.u32_at(addr)
    }

    fn bytes_at(&self, addr: u32, len: usize) -> Option<Vec<u8>> {
        self.bulk.set(self.bulk.get() + 1);
        self.inner.bytes_at(addr, len)
    }
}

/// A 1000-actor level costs one bulk read of the pointer array, and still finds the controller.
#[test]
fn the_scan_reads_the_actors_array_in_one_call() {
    let mut inner = FakeMem::default();
    inner.set(GLEVEL_PTR_VA, WORLD);
    inner.set(WORLD + WORLD_LEVEL_OFF, LEVEL);
    inner.set(LEVEL + LEVEL_ACTORS_OFF, ACTORS);
    inner.set(LEVEL + LEVEL_ACTORS_OFF + 4, 1000);
    for i in 0..999u32 {
        let other = 0x3000_0000 + i * 0x10;
        inner.set(ACTORS + 4 * i, other);
        inner.set(other, 0x1234);
    }
    inner.set(ACTORS + 4 * 999, PC);
    inner.set(PC, PLAYER_CONTROLLER_VTABLE_VA);
    let mem = Counting {
        inner,
        bulk: std::cell::Cell::new(0),
    };
    assert_eq!(find_player_controller(&mem), Some(PC));
    assert_eq!(mem.bulk.get(), 1);
}

/// Pawn movement is measured at [`POS_EPS`] and a pawn turn at a full degree.
#[test]
fn pawn_position_and_yaw_thresholds() {
    let base = snap();
    let mut moved = base;
    moved.pawn = Some(([1.0 + POS_EPS * 0.9, 2.0, 3.0], 90.0));
    assert!(!changed(&base, &moved));
    moved.pawn = Some(([1.0 + POS_EPS * 1.1, 2.0, 3.0], 90.0));
    assert!(changed(&base, &moved));
    moved.pawn = Some(([1.0, 2.0, 3.0], 90.5));
    assert!(!changed(&base, &moved));
    moved.pawn = Some(([1.0, 2.0, 3.0], 92.0));
    assert!(changed(&base, &moved));
}

/// The mode bits or the button bits alone are enough to emit a pose row.
#[test]
fn mode_or_buttons_alone_trigger_a_row() {
    let base = snap();
    let mut moved = base;
    moved.mode = 1;
    assert!(changed(&base, &moved));
    let mut moved = base;
    moved.buttons = 1;
    assert!(changed(&base, &moved));
}

/// A check with no camera still spends the 250 ms period.
#[test]
fn skip_advances_the_clock() {
    let mut s = PoseSampler::default();
    s.skip(100);
    assert!(!s.due(100 + POSE_PERIOD_MS - 1));
    assert!(s.due(100 + POSE_PERIOD_MS));
}

/// Slow drift adds up against the last emitted snapshot, not against the last sample.
#[test]
fn drift_is_measured_from_the_last_emitted_snapshot() {
    let mut s = PoseSampler::default();
    assert!(s.offer(0, snap()));
    let mut a = snap();
    a.zoom += 0.6;
    assert!(!s.offer(250, a));
    let mut b = snap();
    b.zoom += 1.2;
    assert!(
        s.offer(500, b),
        "1.2 from the emitted pose, 0.6 from the last sample"
    );
}

/// A reset sampler emits a fresh baseline row and is due at once.
#[test]
fn reset_forgets_the_baseline() {
    let mut s = PoseSampler::default();
    assert!(s.offer(0, snap()));
    s.reset();
    assert!(s.due(1));
    assert!(s.offer(1, snap()));
}

/// A cached controller is reused while its vtable matches, and rescanned once it does not.
#[test]
fn cached_controller_is_reused_while_its_vtable_matches() {
    let mut mem = FakeMem::default();
    world_with_controller(&mut mem);
    let mut cache = PcCache::default();
    assert_eq!(controller_ptr(&mem, 0, &mut cache), Some(PC));
    mem.words.remove(&GLEVEL_PTR_VA);
    assert_eq!(controller_ptr(&mem, 5000, &mut cache), Some(PC));
    mem.set(PC, 0);
    assert_eq!(controller_ptr(&mem, 5000, &mut cache), None);
    assert_eq!(cache.ptr, 0);
}
