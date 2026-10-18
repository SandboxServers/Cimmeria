//! Unit guards for the approach ring: its size bound, its sampling
//! interval, its teleport reset, its age cut-off, its wire form and its
//! forget path.

use super::approach::*;
use super::MovementTelemetry;
use std::time::{Duration, Instant};

/// Feed one accepted position at `x` (y and z fixed) at `at`.
fn feed(t: &mut MovementTelemetry, entity_id: u32, x: f32, at: Instant) {
    t.note_approach_point(entity_id, [x, 0.0, 0.0], 0.0, at);
}

/// The ring keeps only the last eight points, so a long walk cannot grow
/// a trail without bound.
#[test]
fn ring_keeps_the_last_eight() {
    let mut t = MovementTelemetry::default();
    let t0 = Instant::now();
    for i in 0..12 {
        feed(&mut t, 1, i as f32, t0 + Duration::from_secs(i));
    }
    let trail = t.approach_trail(1, t0 + Duration::from_secs(11)).unwrap();
    assert_eq!(trail.points.len(), APPROACH_POINTS);
    let xs: Vec<f32> = trail.points.iter().map(|p| p.pos[0]).collect();
    assert_eq!(xs, vec![4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0]);
    assert_eq!(trail.points.last().unwrap().age_s, 0.0);
}

/// Points closer together than the sampling interval are skipped, or the
/// ring would fill with near-identical entries from a standing player.
#[test]
fn points_closer_than_one_second_are_skipped() {
    let mut t = MovementTelemetry::default();
    let t0 = Instant::now();
    feed(&mut t, 2, 0.0, t0);
    feed(&mut t, 2, 1.0, t0 + Duration::from_millis(500));
    assert_eq!(t.approach_trail(2, t0 + Duration::from_millis(500)), None);
}

/// A step over the jump threshold is a teleport, so the ring restarts
/// rather than drawing a line across the map.
#[test]
fn a_jump_over_ten_metres_clears_the_ring() {
    let mut t = MovementTelemetry::default();
    let t0 = Instant::now();
    for i in 0..3 {
        feed(&mut t, 3, i as f32, t0 + Duration::from_secs(i));
    }
    feed(&mut t, 3, 50.0, t0 + Duration::from_secs(3));
    assert_eq!(t.approach_trail(3, t0 + Duration::from_secs(3)), None);
    feed(&mut t, 3, 51.0, t0 + Duration::from_secs(4));
    let trail = t.approach_trail(3, t0 + Duration::from_secs(4)).unwrap();
    assert_eq!(trail.points.len(), 2);
}

/// Points older than the age cut-off are left out, so a stale trail is
/// never reported as the approach to a new interaction.
#[test]
fn old_points_age_out() {
    let mut t = MovementTelemetry::default();
    let t0 = Instant::now();
    for i in 0..3 {
        feed(&mut t, 4, i as f32, t0 + Duration::from_secs(i));
    }
    assert_eq!(t.approach_trail(4, t0 + Duration::from_secs(2 + 31)), None);
}

/// The wire form, span and path length are what the interaction row
/// carries, so their exact text and numbers are pinned.
#[test]
fn encode_span_and_length() {
    let trail = ApproachTrail {
        points: vec![
            ApproachPoint {
                pos: [1.0, 2.0, 3.0],
                yaw_deg: 90.0,
                age_s: 1.0,
            },
            ApproachPoint {
                pos: [2.0, 2.0, 3.0],
                yaw_deg: 90.0,
                age_s: 0.0,
            },
        ],
    };
    assert_eq!(
        trail.encode(),
        "1.00,2.00,3.00,90.0,1.0;2.00,2.00,3.00,90.0,0.0"
    );
    assert_eq!(trail.span_s(), 1.0);
    assert_eq!(trail.path_len_m(), 1.0);
}

/// Forgetting a player drops their ring, so a recycled entity id starts
/// with an empty trail.
#[test]
fn forget_drops_the_ring() {
    let mut t = MovementTelemetry::default();
    let t0 = Instant::now();
    feed(&mut t, 5, 0.0, t0);
    feed(&mut t, 5, 1.0, t0 + Duration::from_secs(1));
    let before = t.tracked();
    t.forget(5);
    assert_eq!(t.tracked(), before - 1);
}
