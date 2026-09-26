//! Layers 2+3 — speed (warn-only) + teleport (hard reject).
//!
//! These drive the time-injected
//! [`SpaceManager::apply_client_position_update_at`](super::super::super::SpaceManager::apply_client_position_update_at)
//! so the kinematics deltas are deterministic.

use std::time::{Duration, Instant};

use cimmeria_common::Vector3;
use cimmeria_entity::movement_validation::MovementReject;
use cimmeria_entity::stats::MOVEMENT_SPEED_MOD;
use tracing::Level;

use super::super::super::ClientMoveOutcome;
use super::{make_manager, seed_clock, SPAWN_POS};
use crate::test_support::LogCapture;

/// **Canonical teleport regression guard.** A captured
/// `AVATAR_UPDATE_EXPLICIT` whose `new_pos` is 100 units from `last_pos`
/// over 50 ms (= 2000 u/s, ~246× the 8.125 u/s class top speed) must be
/// rejected at the validation seam, and the cell entity must NOT advance
/// (so AoI never observes the teleported position).
///
/// Reverting the kinematics wiring in `apply_client_position_update_at`
/// makes this Accept and writes the 100 u jump through — the guard fires.
#[test]
fn teleport_100m_over_50ms_is_rejected_and_not_observed() {
    let mut mgr = make_manager();
    let space_id = mgr
        .create_entity(100, "Agnos", SPAWN_POS, [0.0; 3])
        .unwrap();
    let t0 = Instant::now();
    seed_clock(&mut mgr, 100, t0);

    // 100 units along +X from spawn, 50 ms later. Inside the Agnos AABB
    // (so bounds passes) and Agnos has no navmesh (so containment fails
    // open) — kinematics is the only layer that can catch it.
    let teleport = [SPAWN_POS[0] + 100.0, 0.0, SPAWN_POS[2]];
    let outcome = mgr.apply_client_position_update_at(
        t0 + Duration::from_millis(50),
        100,
        teleport,
        [0, 0, 0],
        [0.0; 3],
    );

    match outcome {
        ClientMoveOutcome::Rejected {
            reason, last_valid, ..
        } => {
            assert_eq!(reason, MovementReject::Teleport);
            assert_eq!(last_valid, SPAWN_POS);
        }
        other => panic!("expected Rejected(Teleport), got {other:?}"),
    }
    // AoI reads the cell entity position; it must still be spawn.
    let entity = &mgr.spaces[&space_id].entities[&100];
    assert_eq!(
        entity.position,
        Vector3::new(SPAWN_POS[0], SPAWN_POS[1], SPAWN_POS[2]),
        "teleport-rejected position must not have been written to the cell entity"
    );
}

/// Sub-teleport but over-tolerance speed (5 u in 50 ms = 100 u/s): the
/// warn-only speed layer accepts the move (and logs/counts it) — it must
/// NOT snap the client back. This pins the warn-only policy: a fast-but-
/// short hop is observed, not punished, until the tolerance is
/// calibrated from telemetry.
#[test]
fn over_tolerance_short_hop_is_accepted_warn_only() {
    let mut mgr = make_manager();
    let space_id = mgr
        .create_entity(100, "Agnos", SPAWN_POS, [0.0; 3])
        .unwrap();
    let t0 = Instant::now();
    seed_clock(&mut mgr, 100, t0);

    // 5 u (< 50 u teleport gate) in 50 ms → 100 u/s (> 1.5× top speed).
    let hop = [SPAWN_POS[0] + 5.0, 0.0, SPAWN_POS[2]];
    let outcome = mgr.apply_client_position_update_at(
        t0 + Duration::from_millis(50),
        100,
        hop,
        [0, 0, 0],
        [0.0; 3],
    );
    assert!(
        matches!(outcome, ClientMoveOutcome::Accepted { position } if position == hop),
        "over-tolerance short hop must be accepted under the warn-only policy, got {outcome:?}"
    );
    let entity = &mgr.spaces[&space_id].entities[&100];
    assert_eq!(entity.position, Vector3::new(hop[0], hop[1], hop[2]));
}

/// Authorized server teleport followed by a client follow-up packet must
/// not be teleport-rejected: `note_authorized_teleport` reseeds the clock
/// and `last_pos` is the (already-written) destination, so the small
/// follow-up delta is well within tolerance. This is the speed/teleport
/// analogue of the bounds-layer authorized-teleport guard in
/// [`super::bounds`].
#[test]
fn authorized_teleport_then_client_followup_is_accepted() {
    let mut mgr = make_manager();
    mgr.create_entity(100, "Agnos", SPAWN_POS, [0.0; 3])
        .unwrap();
    let t0 = Instant::now();
    seed_clock(&mut mgr, 100, t0);

    // Server-authoritative jump far across the world (what ring/respawn/
    // gate/GM travel do) + reseed, exactly as the production paths now do.
    let dst = [900.0, 0.0, 900.0];
    mgr.update_entity_position(100, dst, [0, 0, 0], [0.0; 3]);
    mgr.note_authorized_teleport(100);

    // Client's first post-teleport packet, a small delta from dst, 100 ms
    // later. Distance from last_pos (= dst) is tiny → not a teleport.
    let followup = [900.4, 0.0, 900.2];
    let outcome = mgr.apply_client_position_update_at(
        t0 + Duration::from_millis(100),
        100,
        followup,
        [0, 0, 0],
        [0.0; 3],
    );
    assert!(
        matches!(outcome, ClientMoveOutcome::Accepted { position } if position == followup),
        "post-authorized-teleport follow-up must be accepted, got {outcome:?}"
    );
}

/// dt-inflation guard. The processing clock must advance on **every**
/// packet, including ones a cheaper layer rejects — otherwise an attacker
/// can spam out-of-bounds (or off-navmesh) packets to let `dt` grow, then
/// send one large jump whose implied speed (`distance / dt`) looks slow
/// enough to clear the teleport gate.
///
/// Shape: seed → bounds-rejected packet 5 s later → 100 u jump 50 ms after
/// that. The jump must still be `Teleport` (dt measured from the rejected
/// packet, ~50 ms → 2000 u/s). Reverting the up-front `touch_clock` so the
/// clock only advances inside `check_kinematics` makes `dt` ≈ 5.05 s, the
/// implied speed ≈ 20 u/s clears the gate, and the jump slips through —
/// this guard fires.
#[test]
fn bounds_reject_spam_cannot_inflate_dt_to_slip_a_teleport() {
    let mut mgr = make_manager();
    mgr.create_entity(100, "Agnos", SPAWN_POS, [0.0; 3])
        .unwrap();
    let t0 = Instant::now();
    seed_clock(&mut mgr, 100, t0);

    // 5 s of "silence" spent spamming a cheaply-rejected out-of-bounds
    // packet (outside the fallback AABB). It must reject AND advance the
    // clock to this instant.
    let oob = [50_000.0, 0.0, SPAWN_POS[2]];
    let spam = mgr.apply_client_position_update_at(
        t0 + Duration::from_secs(5),
        100,
        oob,
        [0, 0, 0],
        [0.0; 3],
    );
    assert!(
        matches!(
            spam,
            ClientMoveOutcome::Rejected {
                reason: MovementReject::OutOfBounds,
                ..
            }
        ),
        "spam packet must be bounds-rejected, got {spam:?}"
    );

    // 100 u jump, 50 ms after the rejected packet. dt must be measured
    // from the rejected packet (~50 ms), not the seed (~5 s).
    let jump = [SPAWN_POS[0] + 100.0, 0.0, SPAWN_POS[2]];
    let outcome = mgr.apply_client_position_update_at(
        t0 + Duration::from_secs(5) + Duration::from_millis(50),
        100,
        jump,
        [0, 0, 0],
        [0.0; 3],
    );
    assert!(
        matches!(
            outcome,
            ClientMoveOutcome::Rejected {
                reason: MovementReject::Teleport,
                ..
            }
        ),
        "jump after bounds-reject spam must still be Teleport-rejected — the \
         clock must advance on rejects so dt can't be inflated; got {outcome:?}"
    );
}

/// The speed layer must measure a player against *their own* top speed, not
/// the class constant.
///
/// `movementSpeedMod` is a two-sided contract: the client scales its own
/// local prediction by `cur/100` the moment it receives the stat in an
/// `onStatUpdate`, and the NPC path-stepping tick already scales by it
/// server-side. The client-position gate was the one place still comparing
/// against the flat `DEFAULT_TOP_SPEED`, so a player the server itself sped
/// up — a GM `.speed 500`, and any future haste effect writing the same stat
/// — warned on every packet of movement the server had authorised. Warn-only
/// today, but the layer's own doc comment says snap-back is the plan, at
/// which point this becomes a hard reject on legitimate movement.
///
/// Shape: one hop, fast enough to warn at the baseline and slow enough not
/// to at 5×. The control case pins that the hop really is warn-worthy
/// unscaled, so reverting the fix trips the boosted assertion rather than
/// silently passing a test that never warned either way.
#[test]
fn speed_warn_is_measured_against_the_entitys_own_movement_speed_mod() {
    // 1 u in 50 ms = 20 u/s. Baseline warn threshold is
    // 8.125 × 1.5 = 12.19 u/s; at `movementSpeedMod = 500` it is 60.94 u/s.
    let hop = [SPAWN_POS[0] + 1.0, 0.0, SPAWN_POS[2]];

    {
        let capture = LogCapture::install();
        let mut mgr = make_manager();
        mgr.create_entity(100, "Agnos", SPAWN_POS, [0.0; 3])
            .unwrap();
        let t0 = Instant::now();
        seed_clock(&mut mgr, 100, t0);
        mgr.apply_client_position_update_at(
            t0 + Duration::from_millis(50),
            100,
            hop,
            [0, 0, 0],
            [0.0; 3],
        );
        assert!(
            capture
                .find_event(Level::WARN, "movement.speed_warning", "speed")
                .is_some(),
            "control: at the default speed mod this hop must warn — otherwise the \
             boosted case below proves nothing"
        );
    }

    let capture = LogCapture::install();
    let mut mgr = make_manager();
    mgr.create_entity(100, "Agnos", SPAWN_POS, [0.0; 3])
        .unwrap();
    mgr.get_entity_mut(100)
        .unwrap()
        .stats
        .get_mut(MOVEMENT_SPEED_MOD)
        .unwrap()
        .set_current(500);
    let t0 = Instant::now();
    seed_clock(&mut mgr, 100, t0);

    let outcome = mgr.apply_client_position_update_at(
        t0 + Duration::from_millis(50),
        100,
        hop,
        [0, 0, 0],
        [0.0; 3],
    );
    assert!(
        matches!(outcome, ClientMoveOutcome::Accepted { position } if position == hop),
        "the hop is sub-teleport either way and must be accepted, got {outcome:?}"
    );
    assert!(
        capture
            .find_event(Level::WARN, "movement.speed_warning", "speed")
            .is_none(),
        "a player the server itself sped up must not warn at a speed their own \
         movementSpeedMod permits — comparing against the flat class constant \
         warns on every packet of authorised movement"
    );

    // …but the gate still exists: past their *personal* threshold it fires.
    // 5 u in 50 ms = 100 u/s, over the boosted 60.94 u/s tolerance.
    let sprint = [SPAWN_POS[0] + 5.0, 0.0, SPAWN_POS[2]];
    mgr.apply_client_position_update_at(
        t0 + Duration::from_millis(100),
        100,
        sprint,
        [0, 0, 0],
        [0.0; 3],
    );
    let warn = capture
        .find_event(Level::WARN, "movement.speed_warning", "speed")
        .expect("past the boosted tolerance the speed layer must still warn");
    assert!(
        warn.has_field("top_speed", "40.625"),
        "the warn must report the scaled baseline it actually compared against, \
         not the class constant — the SigNoz tolerance-calibration pipeline reads \
         this field; got {warn:#?}"
    );
    assert!(
        warn.has_field("world", "Agnos"),
        "every `movement.validation` row whose space id resolves carries \
         `world`; a speed warn without it drops out of a world-filtered \
         dashboard: {warn:#?}"
    );
}
