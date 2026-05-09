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
    r.enter_send_wait(2);
    assert_eq!(r.state, State::SendWait);
    assert_eq!(r.remote_region_id, Some(2));
}

#[test]
fn remote_wait_idle_to_recv_wait() {
    let mut r = RingTransporter::from_region(&make_region(2, "Castle", vec![1]));
    r.remote_wait(1);
    assert_eq!(r.state, State::RecvWait);
    assert_eq!(r.remote_region_id, Some(1));
}

#[test]
fn full_cycle_source_side() {
    let mut src = RingTransporter::from_region(&make_region(1, "Castle", vec![2]));
    src.enter_send_wait(2);

    // Player walks onto the pad
    src.region_triggered(true, 100);
    assert!(src.should_auto_start());

    let now = Instant::now();
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
    dst.remote_wait(1);
    dst.remote_send();
    assert_eq!(dst.state, State::RecvWarmup);
    dst.remote_count_update(2);
    dst.remote_transport();
    assert_eq!(dst.state, State::RemoteLoadWait);
    // 1 of 2 loaded → not yet ready.
    assert!(!dst.player_loaded(100));
    // 2 of 2 → ready.
    assert!(dst.player_loaded(101));
}

#[test]
fn destination_full_cycle_to_idle() {
    let mut dst = RingTransporter::from_region(&make_region(2, "Castle", vec![1]));
    dst.remote_wait(1);
    dst.remote_send();
    dst.remote_count_update(1);
    dst.remote_transport();

    assert!(dst.player_loaded(100));

    let now = Instant::now();
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
    dst.remote_wait(1);
    dst.remote_send();
    dst.remote_count_update(0);
    dst.remote_transport();

    let effs = dst.all_players_loaded(Instant::now());
    assert!(effs.is_empty());
    assert_eq!(dst.state, State::RemoteWarmup);
}
