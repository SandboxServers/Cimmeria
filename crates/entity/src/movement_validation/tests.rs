use super::*;

fn unit_bounds() -> SpaceBounds {
    SpaceBounds::new([-100.0, -50.0, -100.0], [100.0, 200.0, 100.0])
}

#[test]
fn validator_accepts_in_bounds_position() {
    let v = MovementValidator::new();
    assert_eq!(v.check_bounds(Vector3::zero(), &unit_bounds()), Ok(()));
}

#[test]
fn validator_rejects_out_of_bounds_position_with_out_of_bounds_variant() {
    let v = MovementValidator::new();
    let pos = Vector3::new(1_000.0, 0.0, 0.0);
    assert_eq!(
        v.check_bounds(pos, &unit_bounds()),
        Err(MovementReject::OutOfBounds)
    );
}

// ── Speed / teleport layer (kinematics) ─────────────────────────────

const TOP: f32 = MovementValidator::DEFAULT_TOP_SPEED;

#[test]
fn no_baseline_small_first_move_is_accepted() {
    let v = MovementValidator::new();
    let t0 = Instant::now();
    // `prev = None` (first packet since spawn). A small move from the
    // authoritative spawn is fine — no speed to measure, distance is
    // under the teleport gate.
    let out = v.check_kinematics(t0, None, Vector3::zero(), Vector3::new(2.0, 0.0, 0.0), TOP);
    assert_eq!(out, KinematicsOutcome::default());
}

/// First-packet bypass guard: with no time baseline we can't compute a
/// speed, but a first move farther than the teleport jump distance
/// from the authoritative spawn is still a teleport (distance gate).
/// Without this gate, the very first client packet after spawn could
/// jump anywhere in-bounds for free.
#[test]
fn no_baseline_large_first_move_is_teleport_rejected() {
    let v = MovementValidator::new();
    let t0 = Instant::now();
    let out = v.check_kinematics(
        t0,
        None,
        Vector3::zero(),
        Vector3::new(9_000.0, 0.0, 0.0),
        TOP,
    );
    assert_eq!(out.reject, Some(MovementReject::Teleport));
}

#[test]
fn small_legitimate_move_is_not_rejected() {
    let v = MovementValidator::new();
    let t0 = Instant::now();
    // ~0.8 u over 100 ms = top speed exactly — under the 1.5× warn.
    let out = v.check_kinematics(
        t0 + Duration::from_millis(100),
        Some(t0),
        Vector3::zero(),
        Vector3::new(0.8, 0.0, 0.0),
        TOP,
    );
    assert!(out.reject.is_none(), "legit move must not reject");
}

/// Canonical teleport shape: 100 units in 50 ms → 2000 u/s,
/// ~246× top speed. Both gates trip → hard reject.
#[test]
fn hundred_units_in_fifty_ms_is_teleport_rejected() {
    let v = MovementValidator::new();
    let t0 = Instant::now();
    let out = v.check_kinematics(
        t0 + Duration::from_millis(50),
        Some(t0),
        Vector3::zero(),
        Vector3::new(100.0, 0.0, 0.0),
        TOP,
    );
    assert_eq!(out.reject, Some(MovementReject::Teleport));
}

/// A lagged client sending one large *but slow* catch-up packet
/// (60 u over 10 s = 6 u/s) must NOT be teleport-rejected — distance
/// gate trips but the speed gate does not. This is the false-positive
/// the dual gate exists to avoid.
#[test]
fn far_but_slow_catchup_is_not_teleport() {
    let v = MovementValidator::new();
    let t0 = Instant::now();
    let out = v.check_kinematics(
        t0 + Duration::from_secs(10),
        Some(t0),
        Vector3::zero(),
        Vector3::new(60.0, 0.0, 0.0),
        TOP,
    );
    assert!(
        out.reject.is_none(),
        "far-but-slow catch-up must not be a teleport"
    );
}

/// `touch_clock` advances the per-entity clock and hands back the
/// previous sample, which is what feeds `check_kinematics`'s `prev`.
#[test]
fn touch_clock_returns_prior_and_advances() {
    let mut v = MovementValidator::new();
    let t0 = Instant::now();
    assert_eq!(v.touch_clock(1, t0), None, "first touch has no prior");
    let t1 = t0 + Duration::from_millis(100);
    assert_eq!(v.touch_clock(1, t1), Some(t0), "second touch returns t0");
}

/// A spamming attacker who keeps the cell entity pinned (last_pos
/// unchanged) must be rejected on *every* packet — touching the clock
/// each packet keeps dt ~one tick so the implied speed never decays
/// under the teleport gate.
#[test]
fn sustained_teleport_spam_keeps_rejecting() {
    let mut v = MovementValidator::new();
    let mut t = Instant::now();
    v.touch_clock(1, t); // seed
    for _ in 0..5 {
        t += Duration::from_millis(100);
        let prev = v.touch_clock(1, t);
        let out = v.check_kinematics(t, prev, Vector3::zero(), Vector3::new(100.0, 0.0, 0.0), TOP);
        assert_eq!(
            out.reject,
            Some(MovementReject::Teleport),
            "each spam packet must re-reject; dt must not decay the speed gate"
        );
    }
}

#[test]
fn note_authorized_teleport_reseeds_clock_and_suppresses_followup_reject() {
    let mut v = MovementValidator::new();
    let t0 = Instant::now();
    // Player moving normally, last sample at t0.
    v.touch_clock(1, t0);
    // Authoritative teleport 5 s later reseeds the clock.
    let t_tp = t0 + Duration::from_secs(5);
    v.note_authorized_teleport(1, t_tp);
    // First post-teleport client packet, 100 ms after the teleport,
    // a small delta from the new (destination) last_pos: dt is
    // measured from t_tp, not t0, so the speed is sane.
    let t_pkt = t_tp + Duration::from_millis(100);
    let prev = v.touch_clock(1, t_pkt);
    assert_eq!(
        prev,
        Some(t_tp),
        "reseed must make the prior the teleport instant"
    );
    let dst = Vector3::new(500.0, 0.0, 500.0);
    let out = v.check_kinematics(t_pkt, prev, dst, Vector3::new(500.5, 0.0, 500.0), TOP);
    assert!(
        out.reject.is_none(),
        "reseeded clock must make the post-teleport packet look slow"
    );
}

// ── Snap-back correction budget ─────────────────────────────────────

#[test]
fn note_reject_counts_consecutive_rejects_and_clear_resets() {
    let mut v = MovementValidator::new();
    assert_eq!(v.note_reject(1), 1);
    assert_eq!(v.note_reject(1), 2);
    assert_eq!(v.note_reject(2), 1, "budgets are per-entity");
    v.clear_rejects(1);
    assert_eq!(
        v.note_reject(1),
        1,
        "an accepted position must reset the budget, not merely pause it"
    );
    assert_eq!(
        v.note_reject(2),
        2,
        "clearing one entity must not clear another"
    );
}

/// `forget` has to release the reject budget as well as the clock —
/// otherwise a recycled `entity_id` inherits a spent budget and its
/// very first bad packet is treated as the sixth in a row.
#[test]
fn forget_clears_the_reject_budget_too() {
    let mut v = MovementValidator::new();
    for _ in 0..MovementValidator::MAX_SNAP_BACK_CORRECTIONS {
        v.note_reject(7);
    }
    v.forget(7);
    assert_eq!(v.note_reject(7), 1);
}

#[test]
fn forget_clears_clock_so_next_sample_has_no_baseline() {
    let mut v = MovementValidator::new();
    let t0 = Instant::now();
    v.touch_clock(1, t0); // seed
    v.forget(1);
    // After forget there is no time baseline, so the next touch
    // returns None and the move is gated on distance alone (a fresh
    // start) — a small move is accepted.
    let t1 = t0 + Duration::from_millis(50);
    let prev = v.touch_clock(1, t1);
    assert_eq!(prev, None, "forget must clear the prior sample");
    let out = v.check_kinematics(t1, prev, Vector3::zero(), Vector3::new(2.0, 0.0, 0.0), TOP);
    assert_eq!(out, KinematicsOutcome::default());
}

// ── Sliding speed window (#1249) ────────────────────────────────────

#[test]
fn zero_distance_at_zero_dt_never_warns() {
    let mut v = MovementValidator::new();
    for _ in 0..20 {
        assert!(v.observe_speed(1, 0.0, 0.0, TOP).is_none());
    }
}

#[test]
fn jittered_packets_at_top_speed_do_not_warn() {
    // 25 ms then 175 ms packets, each 0.8125 u: the per-packet ratio
    // is 4x on the short ones, the window average is top speed.
    let mut v = MovementValidator::new();
    for i in 0..40 {
        let dt = if i % 2 == 0 { 0.025 } else { 0.175 };
        assert!(v.observe_speed(1, 0.8125, dt, TOP).is_none(), "packet {i}");
    }
}

#[test]
fn bunched_burst_after_a_gap_does_not_warn() {
    let mut v = MovementValidator::new();
    assert!(v.observe_speed(1, 0.8125, 0.4, TOP).is_none());
    for _ in 0..3 {
        assert!(v.observe_speed(1, 0.8125, 0.0, TOP).is_none());
    }
}

#[test]
fn single_fast_hop_does_not_warn() {
    let mut v = MovementValidator::new();
    assert!(v.observe_speed(1, 1.0, 0.05, TOP).is_none());
}

#[test]
fn sustained_triple_speed_warns() {
    let mut v = MovementValidator::new();
    let sample = (0..4)
        .find_map(|_| v.observe_speed(1, 2.4375, 0.1, TOP))
        .expect("triple speed must warn within 4 packets");
    assert!(sample.implied_speed > TOP * 1.5);
}

#[test]
fn teleport_and_forget_clear_the_window() {
    let t0 = Instant::now();

    let mut v = MovementValidator::new();
    let warned = (0..4).any(|_| v.observe_speed(1, 2.4375, 0.1, TOP).is_some());
    assert!(warned, "fast samples must warn before the reset");
    v.note_authorized_teleport(1, t0);
    assert!(v.observe_speed(1, 0.8125, 0.1, TOP).is_none());

    let mut v = MovementValidator::new();
    let warned = (0..4).any(|_| v.observe_speed(1, 2.4375, 0.1, TOP).is_some());
    assert!(warned, "fast samples must warn before the reset");
    v.forget(1);
    assert!(v.observe_speed(1, 0.8125, 0.1, TOP).is_none());
}

#[test]
fn zero_dt_flood_keeps_its_distance_past_the_sample_limit() {
    // 100 zero-dt hops of 1.0 u: the cap folds the oldest samples into the
    // next, so memory stays bounded but the sum is the full 100.0.
    let mut v = MovementValidator::new();
    let mut last = None;
    for _ in 0..100 {
        last = v.observe_speed(1, 1.0, 0.0, TOP);
    }
    let sample = last.expect("a zero-dt flood of 1 u hops must warn");
    assert_eq!(sample.distance, 100.0);
    assert_eq!(sample.dt_secs, 0.0);
}

#[test]
fn lag_stall_then_queued_burst_past_the_cap_does_not_warn() {
    // A 3.3 s stall, then 40 queued packets at zero dt: 41 hops of top-speed
    // steps over 3.3 s is ~10 u/s, under the 1.5x warn. Evicting the stall's
    // sample at the 32-sample cap would drop its dt and read ~104 u/s.
    let mut v = MovementValidator::new();
    assert!(v.observe_speed(1, 0.8125, 3.3, TOP).is_none());
    for i in 0..40 {
        assert!(v.observe_speed(1, 0.8125, 0.0, TOP).is_none(), "hop {i}");
    }
}
#[test]
fn negative_dt_counts_as_zero() {
    let mut v = MovementValidator::new();
    let sample = v
        .observe_speed(1, 5.0, -5.0, TOP)
        .expect("5 u over the clamped window must warn");
    assert_eq!(sample.dt_secs, 0.0);
    assert_eq!(sample.distance, 5.0);
}
