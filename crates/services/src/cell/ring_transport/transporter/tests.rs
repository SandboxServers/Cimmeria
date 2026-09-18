//! Tests for the ring-transporter FSM.

use super::*;
use crate::cell::ring_transport::regions::RingRegion;

fn make_region(id: i32, world: &str, dests: Vec<i32>) -> RingRegion {
    RingRegion {
        region_id: id,
        world_id: 12,
        world_name: world.to_string(),
        x: 1.0,
        y: 2.0,
        z: 3.0,
        tag: format!("Ring{id}"),
        height: 1.7,
        radius: 3.5,
        event_set_id: 100,
        display_name_id: 7508,
        destination_ids: dests,
        point_set_id: 200,
        required_mission_id: None,
    }
}

#[test]
fn fsm_starts_idle() {
    let r = RingTransporter::from_region(&make_region(1, "Castle", vec![2]));
    assert_eq!(r.state, State::Idle);
    assert!(r.players.is_empty());
}

#[test]
fn validate_destination_rejects_self_busy_unknown() {
    let mut r = RingTransporter::from_region(&make_region(1, "Castle", vec![2, 3]));
    assert_eq!(
        r.validate_destination(99),
        Err("destination not in region's destination list")
    );
    assert_eq!(
        r.validate_destination(1),
        Err("destination not in region's destination list")
    );
    // Self-as-dest is filtered at load time, so it'd hit the "not in list" branch first.
    // Force a self-list entry to test the source==dest guard.
    r.destination_ids.push(1);
    assert_eq!(
        r.validate_destination(1),
        Err("source and destination cannot be the same")
    );

    r.state = State::SendWait;
    assert_eq!(r.validate_destination(2), Err("source ring is busy"));
}

#[test]
fn idle_to_send_wait_via_enter_send_wait() {
    let mut r = RingTransporter::from_region(&make_region(1, "Castle", vec![2]));
    r.enter_send_wait(2, 100, Instant::now());
    assert_eq!(r.state, State::SendWait);
    assert_eq!(r.remote_region_id, Some(2));
}

#[test]
fn remote_wait_idle_to_recv_wait() {
    let mut r = RingTransporter::from_region(&make_region(2, "Castle", vec![1]));
    r.remote_wait(1, Instant::now());
    assert_eq!(r.state, State::RecvWait);
    assert_eq!(r.remote_region_id, Some(1));
}

#[test]
fn full_cycle_source_side() {
    let mut src = RingTransporter::from_region(&make_region(1, "Castle", vec![2]));
    let now = Instant::now();
    src.enter_send_wait(2, 100, now);

    // Player walks onto the pad
    src.region_triggered(true, 100);
    assert!(src.should_auto_start());

    let effs = src.start_sending(now);
    assert_eq!(src.state, State::SendWarmup);
    // Should produce: PlaySequence (1) + OnTeleportOut + LockMovement = 3 for one player.
    assert!(matches!(effs[0], Effect::PlaySequence { .. }));
    assert!(matches!(
        effs[1],
        Effect::OnTeleportOut {
            region_id: 1,
            destination_id: 2,
            ..
        }
    ));
    assert!(matches!(effs[2], Effect::LockMovement { entity_id: 100 }));
    assert_eq!(effs.len(), 3);

    // Hide timer
    let hide_now = now + HIDE_DELAY;
    let effs = src.hide_timer_expired();
    assert_eq!(effs.len(), 1);
    assert!(matches!(effs[0], Effect::HidePlayer { entity_id: 100 }));
    assert_eq!(src.state, State::SendWarmup);
    let _ = hide_now;

    // Warmup timer → teleport. The source's job ends here: it goes back
    // to `Idle` and clears its transient state — without this the
    // source rejects the next trip as "source ring is busy".
    let effs = src.warmup_timer_expired([10.0, 20.0, 30.0], "Castle");
    assert_eq!(src.state, State::Idle);
    assert_eq!(src.remote_region_id, None);
    assert!(src.send_players.is_empty());
    assert_eq!(effs.len(), 1);
    match &effs[0] {
        Effect::TeleportPlayer {
            entity_id,
            position,
            world_name,
            destination_region_id,
        } => {
            assert_eq!(*entity_id, 100);
            assert_eq!(*position, [10.0, 20.0, 30.0]);
            assert_eq!(world_name, "Castle");
            assert_eq!(*destination_region_id, 2);
        }
        _ => panic!("expected TeleportPlayer"),
    }

    // Invariant: after a full source-side cycle, the next
    // validate_destination must succeed. The source must reset to Idle
    // (rather than staying in RemoteLoadWait) so it doesn't reject the
    // next trip as busy.
    assert_eq!(src.validate_destination(2), Ok(()));
}

#[test]
fn full_cycle_destination_side() {
    let mut dst = RingTransporter::from_region(&make_region(2, "Castle", vec![1]));
    let now = Instant::now();
    dst.remote_wait(1, now);
    dst.remote_send(now);
    assert_eq!(dst.state, State::RecvWarmup);
    dst.remote_expect(vec![100, 101]);
    dst.remote_transport(now);
    assert_eq!(dst.state, State::RemoteLoadWait);
    // 1 of 2 loaded → not yet ready.
    assert!(!dst.player_loaded(100));
    // 2 of 2 → ready.
    assert!(dst.player_loaded(101));
}

#[test]
fn destination_full_cycle_to_idle() {
    let mut dst = RingTransporter::from_region(&make_region(2, "Castle", vec![1]));
    let now = Instant::now();
    dst.remote_wait(1, now);
    dst.remote_send(now);
    dst.remote_expect(vec![100]);
    dst.remote_transport(now);

    assert!(dst.player_loaded(100));

    let effs = dst.all_players_loaded(now);
    assert_eq!(dst.state, State::RemoteWarmup);
    assert_eq!(effs.len(), 1); // PlaySequence(TeleportIn) for first player

    let effs = dst.remote_warmup_timer_expired(now + REMOTE_WARMUP_DELAY);
    assert_eq!(dst.state, State::Cooldown);
    assert_eq!(effs.len(), 1);
    assert!(matches!(effs[0], Effect::ShowPlayer { entity_id: 100 }));

    let effs = dst.cooldown_timer_expired();
    assert_eq!(dst.state, State::Idle);
    assert_eq!(effs.len(), 2); // unlock + fire teleport_in
    assert!(matches!(effs[0], Effect::UnlockMovement { entity_id: 100 }));
    assert!(matches!(
        effs[1],
        Effect::FireTeleportIn {
            entity_id: 100,
            region_id: 2
        }
    ));
    assert!(dst.players_loaded.is_empty());
    assert!(dst.remote_region_id.is_none());
}

#[test]
fn destination_with_no_loaded_players_skips_sequence() {
    let mut dst = RingTransporter::from_region(&make_region(2, "Castle", vec![1]));
    let now = Instant::now();
    dst.remote_wait(1, now);
    dst.remote_send(now);
    dst.remote_expect(Vec::new());
    dst.remote_transport(now);

    let effs = dst.all_players_loaded(now);
    assert!(effs.is_empty());
    assert_eq!(dst.state, State::RemoteWarmup);
}

// ---------------------------------------------------------------------------
// H02: bounded stall deadlines (audit defect H-B3)
// ---------------------------------------------------------------------------

/// Every state that waits on something outside the FSM has a bound, and every
/// state that carries its own deadline does not get a second one. This is the
/// claim that justifies a single `stall_at` field.
///
/// The `match` is deliberately exhaustive with no wildcard arm: adding a ninth
/// `State` variant is a **compile error** here, which forces the author to
/// declare whether it waits on something outside the FSM. A version of this
/// test that iterated a hard-coded list would let a new unbounded waiting
/// state through silently — which is exactly the H-B3 defect shape.
#[test]
fn only_externally_waiting_states_have_a_stall_bound() {
    const ALL_STATES: [State; 8] = [
        State::Idle,
        State::SendWait,
        State::SendWarmup,
        State::RemoteLoadWait,
        State::RemoteWarmup,
        State::Cooldown,
        State::RecvWait,
        State::RecvWarmup,
    ];

    for state in ALL_STATES {
        let must_be_bounded = match state {
            // Waits on a player, the peer ring, or a client world load.
            State::SendWait | State::RecvWait | State::RecvWarmup | State::RemoteLoadWait => true,
            // Terminal, or owns its own deadline: hide+warmup, remote_warmup,
            // cooldown. A second deadline here would race the real one.
            State::Idle | State::SendWarmup | State::RemoteWarmup | State::Cooldown => false,
        };
        assert_eq!(
            stall_timeout_for(state).is_some(),
            must_be_bounded,
            "{state:?}: stall-bound presence does not match its classification"
        );
    }

    // The table carries no row for a state that is not in ALL_STATES.
    assert_eq!(STALL_TIMEOUTS.len(), 4);
}

/// The destination must never expire before the source it is reserved for.
/// If it did it would return to Idle, a third ring could claim it, and the
/// original source's passengers would land in someone else's trip.
#[test]
fn recv_wait_outlives_send_wait() {
    assert!(RECV_WAIT_TIMEOUT > SEND_WAIT_TIMEOUT);
}

/// `SendWait` is armed on entry and disarmed on the way out.
#[test]
fn send_wait_arms_and_disarms_its_stall_deadline() {
    let mut r = RingTransporter::from_region(&make_region(1, "Castle", vec![2]));
    let now = Instant::now();
    r.enter_send_wait(2, 100, now);
    assert_eq!(
        r.elapsed_deadline(now + SEND_WAIT_TIMEOUT),
        Some(DeadlineKind::Stall)
    );
    assert_eq!(r.elapsed_deadline(now + SEND_WAIT_TIMEOUT / 2), None);

    // Leaving SendWait must take the deadline with it, or the 60s bound
    // fires against a later healthy trip.
    //
    // Assert on the field, NOT via `elapsed_deadline`: `start_sending` arms
    // `hide_at` at +3.5s and the probe checks it first, so at any far-future
    // instant `elapsed_deadline` returns `Some(Hide)` whether or not the
    // stall deadline was cleared — an `assert_ne!(.., Some(Stall))` there
    // passes unconditionally and would let a deleted `arm_stall` in
    // `start_sending` through.
    r.region_triggered(true, 100);
    r.start_sending(now);
    assert!(
        r.timers.stall_at.is_none(),
        "SendWarmup owns hide+warmup and must not also carry the SendWait stall deadline"
    );
}

/// The rollback in `handle_select_destination` (destination busy) used to
/// write `state`/`remote_region_id` by hand, which left the stall deadline
/// armed on an Idle ring. `reset_to_idle` is the guard.
#[test]
fn reset_to_idle_clears_the_stall_deadline() {
    let mut r = RingTransporter::from_region(&make_region(1, "Castle", vec![2]));
    let now = Instant::now();
    r.enter_send_wait(2, 100, now);
    r.reset_to_idle();
    assert_eq!(r.state, State::Idle);
    assert_eq!(r.elapsed_deadline(now + SEND_WAIT_TIMEOUT * 10), None);
}

/// `RemoteLoadWait` is the state a stalled cross-world arrival parks in.
/// Abort must release the in-flight passengers — including the ones that
/// never reported loaded, which is only possible because the expectation
/// carries ids rather than a bare count.
#[test]
fn abort_releases_expected_and_loaded_passengers_show_before_unlock() {
    let mut dst = RingTransporter::from_region(&make_region(2, "Castle", vec![1]));
    let now = Instant::now();
    dst.remote_wait(1, now);
    dst.remote_send(now);
    dst.remote_expect(vec![100, 101]);
    dst.remote_transport(now);
    // Only 100 made it; 101 is still loading when the deadline fires.
    dst.player_loaded(100);

    assert_eq!(
        dst.elapsed_deadline(now + REMOTE_LOAD_WAIT_TIMEOUT),
        Some(DeadlineKind::Stall)
    );

    let effs = dst.abort_to_idle(None);
    assert_eq!(dst.state, State::Idle);
    assert!(dst.expected_players.is_empty());
    assert!(dst.players_loaded.is_empty());
    assert_eq!(dst.num_remote_players(), 0);
    assert_eq!(dst.remote_region_id, None);

    // Both passengers released, show strictly before unlock for each:
    // unlocking first lets the player move while witnesses still hold them
    // hidden, and `send_visible` resolves its witness set at call time.
    assert_eq!(effs.len(), 4);
    assert_eq!(effs[0], Effect::ShowPlayer { entity_id: 100 });
    assert_eq!(effs[1], Effect::UnlockMovement { entity_id: 100 });
    assert_eq!(effs[2], Effect::ShowPlayer { entity_id: 101 });
    assert_eq!(effs[3], Effect::UnlockMovement { entity_id: 101 });
}

/// Players merely standing on the pad were never locked or hidden, so an
/// abort must not broadcast a stray `onVisible(1)` for them.
#[test]
fn abort_does_not_release_players_who_only_stood_on_the_pad() {
    let mut src = RingTransporter::from_region(&make_region(1, "Castle", vec![2]));
    let now = Instant::now();
    src.enter_send_wait(2, 100, now);
    src.region_triggered(true, 100);

    let effs = src.abort_to_idle(None);
    assert!(
        effs.is_empty(),
        "pad occupants get no Show/Unlock — they never got Hide/Lock: {effs:?}"
    );
    assert_eq!(src.state, State::Idle);
}

/// The entity whose teardown caused the abort is already out of the space;
/// addressing effects to it would only produce a witness-lookup miss.
#[test]
fn abort_skips_the_departing_entity() {
    let mut src = RingTransporter::from_region(&make_region(1, "Castle", vec![2]));
    let now = Instant::now();
    src.enter_send_wait(2, 100, now);
    src.region_triggered(true, 100);
    src.region_triggered(true, 101);
    src.start_sending(now);

    let effs = src.abort_to_idle(Some(100));
    assert_eq!(effs.len(), 2);
    assert_eq!(effs[0], Effect::ShowPlayer { entity_id: 101 });
    assert_eq!(effs[1], Effect::UnlockMovement { entity_id: 101 });
}

/// Removing a passenger must clear them from BOTH destination sets. Clearing
/// only the expectation leaves `players_loaded.len() != num_remote_players()`
/// forever — a fresh stall in place of the old one.
#[test]
fn forget_participant_clears_both_destination_sets() {
    let mut dst = RingTransporter::from_region(&make_region(2, "Castle", vec![1]));
    let now = Instant::now();
    dst.remote_wait(1, now);
    dst.remote_send(now);
    dst.remote_expect(vec![100, 101]);
    dst.remote_transport(now);
    dst.player_loaded(100);

    assert!(dst.forget_participant(100));
    assert_eq!(dst.expected_players, vec![101]);
    assert!(dst.players_loaded.is_empty());
    assert_eq!(dst.num_remote_players(), 1);

    // Idempotent: a second forget of the same id reports no change and
    // cannot underflow the derived count.
    assert!(!dst.forget_participant(100));
    assert_eq!(dst.num_remote_players(), 1);
}

/// A real deadline must win over the abort if the exclusivity invariant on
/// `stall_at` is ever violated by a future change.
#[test]
fn real_deadlines_are_checked_before_the_stall_deadline() {
    let mut r = RingTransporter::from_region(&make_region(1, "Castle", vec![2]));
    let now = Instant::now();
    r.enter_send_wait(2, 100, now);
    // Force the (normally impossible) overlap.
    r.timers.hide_at = Some(now);
    assert_eq!(
        r.elapsed_deadline(now + SEND_WAIT_TIMEOUT),
        Some(DeadlineKind::Hide)
    );
}
