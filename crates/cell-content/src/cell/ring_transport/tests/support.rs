//! Shared fixtures for the H02 timeout / disconnect guards.
//!
//! Those tests drive an injectable clock rather than sleeping: the shortest
//! bound under test is 15s and the longest 90s, so a wall-clock version
//! would take minutes and be the flakiest thing in CI.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use cimmeria_mercury::clock::Clock;

use super::{make_test_space_mgr, ring};
use crate::cell::ring_transport::State;
use crate::cell::space_manager::SpaceManager;

/// Deterministic clock for the ring FSM.
///
/// Implements `cimmeria_mercury::clock::Clock` — the same trait `Channel`
/// reads time through — rather than introducing a second clock abstraction.
/// Mercury's own `TestClock` lives behind the `test-harness` feature, which
/// `cimmeria-services` does not enable, so this is the four-line local
/// equivalent.
pub(super) struct FakeClock {
    base: Instant,
    offset: Mutex<Duration>,
}

impl FakeClock {
    pub(super) fn new() -> Arc<Self> {
        Arc::new(Self {
            base: Instant::now(),
            offset: Mutex::new(Duration::ZERO),
        })
    }

    pub(super) fn advance(&self, by: Duration) {
        *self.offset.lock().expect("FakeClock poisoned") += by;
    }
}

impl Clock for FakeClock {
    fn now(&self) -> Instant {
        self.base + *self.offset.lock().expect("FakeClock poisoned")
    }
}

/// Two-ring same-world fixture plus a third ring that also lists `2` as a
/// destination, so the "is the destination still reachable by someone else"
/// assertion has a second claimant.
pub(super) fn three_ring_mgr(clock: Arc<FakeClock>) -> SpaceManager {
    let mut mgr = make_test_space_mgr();
    let mut regions = std::collections::HashMap::new();
    regions.insert(1, ring(1, "Castle_CellBlock", vec![2, 3], [0.0, 0.0, 0.0]));
    regions.insert(
        2,
        ring(2, "Castle_CellBlock", vec![1, 3], [10.0, 20.0, 30.0]),
    );
    regions.insert(3, ring(3, "Castle_CellBlock", vec![1, 2], [40.0, 0.0, 0.0]));
    mgr.ring_transporters.load(&regions);
    mgr.ring_transporters.set_clock(clock);
    mgr.ring_point_set_to_region = regions
        .iter()
        .map(|(rid, r)| (r.point_set_id, *rid))
        .collect();
    mgr.ring_regions = regions;
    mgr.sequence_map.insert((100, 8000), 9000);
    mgr.sequence_map.insert((100, 8001), 9001);
    mgr
}

pub(super) fn spawn_player(mgr: &mut SpaceManager, entity_id: u32, player_id: i32) {
    mgr.create_entity(entity_id, "Castle_CellBlock", [0.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    mgr.connect_entity(entity_id);
    if let Some(p) = mgr.get_entity_mut(entity_id) {
        p.player_id = Some(player_id);
        p.is_player = true;
    }
}

pub(super) fn state_of(mgr: &SpaceManager, region_id: i32) -> State {
    mgr.ring_transporters.get(region_id).unwrap().state
}

/// Harset's plaza rings, keyed on `db/resources/Worlds/Seed/ring_transport_regions.sql`.
///
/// Tags are byte-exact from the seed, including the region-8 spelling
/// (`HarsetinRing…`, not `HarsetRing…`) — it is not a typo in this fixture.
pub(super) fn harset_rings(
) -> std::collections::HashMap<i32, crate::cell::ring_transport::RingRegion> {
    let rows: [(i32, &str, [f32; 3], [i32; 4]); 5] = [
        (
            4,
            "HarsetRingLeftBottomRegion",
            [-25.641, -67.828, 15.249],
            [5, 6, 7, 8],
        ),
        (
            5,
            "HarsetRingRightBottomRegion",
            [25.738, -67.828, 15.35],
            [4, 6, 7, 8],
        ),
        (
            6,
            "HarsetRingLeftRegion",
            [-197.269, -40.167, 84.35],
            [4, 5, 7, 8],
        ),
        (
            7,
            "HarsetRingLeftTopRegion",
            [-171.657, -27.015, 240.016],
            [4, 5, 6, 8],
        ),
        (
            8,
            "HarsetinRingRightRegion",
            [218.513, -34.125, 36.273],
            [4, 5, 6, 7],
        ),
    ];
    rows.iter()
        .map(|(id, tag, pos, dests)| {
            let mut r = ring(*id, "Castle_CellBlock", dests.to_vec(), *pos);
            r.tag = (*tag).to_string();
            r.point_set_id = 2048 + *id; // 2052..=2056, as seeded
            (*id, r)
        })
        .collect()
}

pub(super) fn harset_mgr(clock: Arc<FakeClock>) -> SpaceManager {
    let mut mgr = make_test_space_mgr();
    let regions = harset_rings();
    mgr.ring_transporters.load(&regions);
    mgr.ring_transporters.set_clock(clock);
    mgr.ring_point_set_to_region = regions
        .iter()
        .map(|(rid, r)| (r.point_set_id, *rid))
        .collect();
    mgr.ring_regions = regions;
    mgr
}
