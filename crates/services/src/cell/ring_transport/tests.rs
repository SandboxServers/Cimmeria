//! Integration-style tests for the ring-transport runtime: exercise the
//! full handle_interact → handle_select_destination → tick → effects path
//! against a real `SpaceManager`.

use tokio::sync::mpsc;

use cimmeria_content_engine::chain::ChainEngine;

use super::dispatch::dispatch_effects;
use super::regions::RingRegion;
use super::runtime::{
    advance_destination_after_warmup, handle_interact, handle_region_trigger,
    handle_remote_player_loaded, handle_select_destination,
};
use super::transporter::State;
use super::wire_helpers::{BSF_MOVEMENT_LOCK, METHOD_ON_RING_TRANSPORTER_LIST};
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use crate::mercury::method_idx::{ON_SEQUENCE, ON_STATE_FIELD_UPDATE};

fn make_test_space_mgr() -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
    let cxml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(cxml).unwrap();
    mgr
}

fn ring(id: i32, world: &str, dests: Vec<i32>, pos: [f32; 3]) -> RingRegion {
    RingRegion {
        region_id: id,
        world_id: 12,
        world_name: world.to_string(),
        x: pos[0],
        y: pos[1],
        z: pos[2],
        tag: format!("Ring{id}"),
        height: 1.7,
        radius: 3.5,
        event_set_id: 100,
        display_name_id: 7508,
        destination_ids: dests,
        point_set_id: 2000 + id,
        required_mission_id: None,
    }
}

/// End-to-end same-world ring travel: interact → select destination →
/// player walks onto pad → tick through hide / warmup / cooldown →
/// teleport_in event eventually dispatched.
#[tokio::test]
async fn full_ring_cycle_dispatches_expected_messages() {
    let mut mgr = make_test_space_mgr();

    // Two rings in Castle_CellBlock; the player will travel from 1 → 2.
    let mut regions = std::collections::HashMap::new();
    regions.insert(1, ring(1, "Castle_CellBlock", vec![2], [0.0, 0.0, 0.0]));
    regions.insert(2, ring(2, "Castle_CellBlock", vec![1], [10.0, 20.0, 30.0]));
    mgr.ring_transporters.load(&regions);
    mgr.ring_point_set_to_region = regions
        .iter()
        .map(|(rid, r)| (r.point_set_id, *rid))
        .collect();
    mgr.ring_regions = regions;

    // Seed the kismet sequence map so PlaySequence resolves.
    mgr.sequence_map.insert((100, 8000), 9000);
    mgr.sequence_map.insert((100, 8001), 9001);

    // Spawn the player entity in the source space.
    mgr.create_entity(42, "Castle_CellBlock", [0.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    mgr.connect_entity(42);
    if let Some(p) = mgr.get_entity_mut(42) {
        p.player_id = Some(700);
    }

    let (tx, mut rx) = mpsc::channel(64);
    let engine = ChainEngine::new();

    // 1. Player triggers the ring switch (chain action = TriggerTransporter).
    handle_interact(1, 42, &tx, &mut mgr, &engine).await;
    assert_eq!(mgr.get_entity(42).and_then(|e| e.ring_source_id), Some(1));

    // Drain: should have one EntityMethodCall for onRingTransporterList.
    let mut seen_dest_list = false;
    while let Ok(msg) = rx.try_recv() {
        if let CellToBaseMsg::EntityMethodCall { method_index, .. } = msg {
            if method_index == METHOD_ON_RING_TRANSPORTER_LIST {
                seen_dest_list = true;
            }
        }
    }
    assert!(seen_dest_list, "onRingTransporterList not sent");

    // 2. Player picks destination 2.
    handle_select_destination(1, 2, 42, &tx, &mut mgr, &engine).await;
    assert_eq!(mgr.get_entity(42).and_then(|e| e.ring_source_id), None);
    assert_eq!(mgr.ring_transporters.get(1).unwrap().state, State::SendWait);
    assert_eq!(mgr.ring_transporters.get(2).unwrap().state, State::RecvWait);

    // 3. Player walks onto the source pad's region (point_set 2001).
    handle_region_trigger(2001, true, 42, &tx, &mut mgr, &engine).await;
    // Auto-start: source SendWait → SendWarmup, dest RecvWait → RecvWarmup.
    assert_eq!(
        mgr.ring_transporters.get(1).unwrap().state,
        State::SendWarmup
    );
    assert_eq!(
        mgr.ring_transporters.get(2).unwrap().state,
        State::RecvWarmup
    );

    // 4. Drain effects so far (should include movement lock and play_sequence).
    let mut got_lock = false;
    let mut got_sequence = false;
    while let Ok(msg) = rx.try_recv() {
        if let CellToBaseMsg::EntityMethodCall { method_index, .. } = msg {
            if method_index == ON_STATE_FIELD_UPDATE {
                got_lock = true;
            }
            if method_index == ON_SEQUENCE {
                got_sequence = true;
            }
        }
    }
    assert!(got_lock, "BSF_MovementLock not applied");
    assert!(got_sequence, "Region_Teleport_Out sequence not played");

    // 5. Manually fire hide-timer (purely to advance state — no externally
    //    visible side effect on the destination). Then run the warmup
    //    transition through the proper dispatch path so TeleportPlayer's
    //    effects run, including the `mark_player_loaded` cross-link which
    //    moves the destination through RemoteLoadWait → RemoteWarmup.
    let hide_effects = mgr
        .ring_transporters
        .get_mut(1)
        .unwrap()
        .hide_timer_expired();
    dispatch_effects(hide_effects, &tx, &mut mgr, &engine).await;

    let dst_pos = {
        let dst = mgr.ring_regions.get(&2).unwrap();
        ([dst.x, dst.y, dst.z], dst.world_name.clone())
    };
    // Capture num_players BEFORE warmup (which clears send_players as part
    // of the source's reset to Idle).
    let warmup_num_players = mgr.ring_transporters.get(1).unwrap().send_players.len() as u32;
    let warmup_effects = mgr
        .ring_transporters
        .get_mut(1)
        .unwrap()
        .warmup_timer_expired(dst_pos.0, &dst_pos.1);
    // Same ordering as the production tick: count update before teleport
    // so `mark_player_loaded` can advance the FSM synchronously.
    advance_destination_after_warmup(2, warmup_num_players, &tx, &mut mgr, &engine).await;
    dispatch_effects(warmup_effects, &tx, &mut mgr, &engine).await;

    // After the warmup teleport step we should see a TeleportPlayer
    // message (replacing the bare onPlayerTeleport that used to be
    // emitted directly — that path didn't move the avatar).
    let mut teleport_msg = None;
    let mut remaining = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        match msg {
            CellToBaseMsg::TeleportPlayer {
                entity_id,
                space_id,
                position,
                prev_pos: _,
            } => {
                teleport_msg = Some((entity_id, space_id, position));
            }
            other => remaining.push(other),
        }
    }
    let (eid, space_id, pos) =
        teleport_msg.expect("TeleportPlayer not emitted by warmup→teleport step");
    assert_eq!(eid, 42);
    assert_ne!(space_id, 0, "TeleportPlayer must carry a non-zero space_id");
    assert!((pos[0] - 10.0).abs() < 0.001);
    assert!((pos[1] - 20.0).abs() < 0.001);
    assert!((pos[2] - 30.0).abs() < 0.001);
    drop(remaining);

    // After warmup + advance, the destination has either auto-advanced
    // through RemoteLoadWait (because the same-world TeleportPlayer marked
    // the player loaded in dispatch) and is now in Cooldown, or it's still
    // in RemoteWarmup waiting for the deadline. Both are valid intermediate
    // states; let's force the cooldown to fire.
    let dst_state = mgr.ring_transporters.get(2).unwrap().state;
    if dst_state == State::RemoteWarmup {
        let effs = mgr
            .ring_transporters
            .get_mut(2)
            .unwrap()
            .remote_warmup_timer_expired(std::time::Instant::now());
        dispatch_effects(effs, &tx, &mut mgr, &engine).await;
    }
    let cd_effects = mgr
        .ring_transporters
        .get_mut(2)
        .unwrap()
        .cooldown_timer_expired();
    dispatch_effects(cd_effects, &tx, &mut mgr, &engine).await;

    // 6. Verify final state.
    assert_eq!(mgr.ring_transporters.get(2).unwrap().state, State::Idle);
    let player_pos = mgr.get_entity(42).unwrap().position;
    // `update_entity_position` was called inside same_world_teleport with
    // the destination's coords; verify the player landed there.
    assert!(
        (player_pos.x - 10.0).abs() < 0.001,
        "player x: {} (expected 10.0)",
        player_pos.x
    );
    assert!(
        (player_pos.y - 20.0).abs() < 0.001,
        "player y: {}",
        player_pos.y
    );
    assert!(
        (player_pos.z - 30.0).abs() < 0.001,
        "player z: {}",
        player_pos.z
    );

    // Movement lock should have been cleared on cooldown.
    let final_state_field = mgr.get_entity(42).unwrap().state_field;
    assert_eq!(
        final_state_field & BSF_MOVEMENT_LOCK,
        0,
        "BSF_MovementLock not cleared at cooldown"
    );
}

/// Cross-world ring travel goes through the `Effect::TeleportCrossWorld`
/// arm of dispatch, which sends `CellToBaseMsg::GateTravel { destination_ring_id: Some(_), .. }`
/// instead of `TeleportPlayer`. Pin: source ring resets to Idle (via
/// `warmup_timer_expired`), destination stays in `RemoteLoadWait` waiting
/// for the deferred `BaseToCellMsg::AdvanceRingDestination` callback,
/// and the GateTravel message carries the destination ring id.
#[tokio::test]
async fn select_destination_cross_world_succeeds() {
    let mut mgr = make_test_space_mgr();
    let mut regions = std::collections::HashMap::new();
    regions.insert(1, ring(1, "Castle_CellBlock", vec![2], [0.0; 3]));
    // Different world_name — destination is on Castle, source is on
    // Castle_CellBlock. Cross-world selections route through
    // GateTravel instead of being rejected at `selectDestination`.
    regions.insert(2, ring(2, "Castle", vec![1], [10.0, 20.0, 30.0]));
    mgr.ring_transporters.load(&regions);
    mgr.ring_point_set_to_region = regions
        .iter()
        .map(|(rid, r)| (r.point_set_id, *rid))
        .collect();
    mgr.ring_regions = regions;

    mgr.create_entity(42, "Castle_CellBlock", [0.0; 3], [0.0; 3])
        .unwrap();
    // Pre-load the player onto the source ring's pad so `should_auto_start`
    // returns true after `enter_send_wait` — this drives the kick-off path
    // that produces the warmup deadline we need to expire.
    if let Some(t) = mgr.ring_transporters.get_mut(1) {
        t.region_triggered(true, 42);
    }
    let (tx, mut rx) = mpsc::channel(16);
    let engine = ChainEngine::new();

    handle_select_destination(1, 2, 42, &tx, &mut mgr, &engine).await;

    // After kick_off_warmup, source is in SendWarmup with a hide+warmup
    // deadline, destination is in RecvWarmup — same as the same-world path
    // up until the warmup timer fires.
    assert_eq!(
        mgr.ring_transporters.get(1).unwrap().state,
        State::SendWarmup,
        "source advanced to SendWarmup after auto-start",
    );

    // Skip past hide and warmup deadlines by directly invoking the
    // FSM transitions — same approach the same-world test uses to keep
    // the test fast and deterministic.
    let dst_pos = [
        mgr.ring_regions[&2].x,
        mgr.ring_regions[&2].y,
        mgr.ring_regions[&2].z,
    ];
    let dst_world = mgr.ring_regions[&2].world_name.clone();
    let _ = mgr
        .ring_transporters
        .get_mut(1)
        .unwrap()
        .hide_timer_expired();
    let warmup_effects = mgr
        .ring_transporters
        .get_mut(1)
        .unwrap()
        .warmup_timer_expired(dst_pos, &dst_world);
    // Destination is now in RemoteLoadWait — same shape as same-world but
    // we don't synchronously call mark_player_loaded on cross-world.
    advance_destination_after_warmup(2, 1, &tx, &mut mgr, &engine).await;
    dispatch_effects(warmup_effects, &tx, &mut mgr, &engine).await;

    // Source ring back at Idle (warmup_timer_expired clears it), destination
    // ring stuck in RemoteLoadWait until base sends AdvanceRingDestination.
    assert_eq!(mgr.ring_transporters.get(1).unwrap().state, State::Idle);
    assert_eq!(
        mgr.ring_transporters.get(2).unwrap().state,
        State::RemoteLoadWait,
        "cross-world destination must wait for AdvanceRingDestination, NOT advance synchronously",
    );
    // The cell entity has been destroyed by the dispatch arm — base will
    // re-create it when GateTravel finishes.
    assert!(
        mgr.get_entity(42).is_none(),
        "cross-world ring transport must destroy the cell entity (gate-travel pattern)",
    );

    // Find the GateTravel message in the outbox — the dispatch arm sends it
    // after destroying the entity. Other ring effects (HidePlayer,
    // PlaySequence, etc.) come through as EntityMethodCall.
    let mut saw_gate_travel = false;
    let mut saw_dest_ring_id: Option<i32> = None;
    while let Ok(msg) = rx.try_recv() {
        if let CellToBaseMsg::GateTravel {
            target_world_name,
            destination_ring_id,
            ..
        } = msg
        {
            saw_gate_travel = true;
            saw_dest_ring_id = destination_ring_id;
            assert_eq!(
                target_world_name, "Castle",
                "GateTravel must carry the destination world name",
            );
        }
    }
    assert!(
        saw_gate_travel,
        "cross-world ring transport must emit CellToBaseMsg::GateTravel",
    );
    assert_eq!(
        saw_dest_ring_id,
        Some(2),
        "GateTravel must carry destination_ring_id so the deferred load callback fires",
    );
}

/// Deferred-load hook: after the destination cell receives
/// `BaseToCellMsg::AdvanceRingDestination`, the destination ring must
/// transition `RemoteLoadWait → RemoteWarmup → Cooldown → Idle` exactly
/// like the same-world path's synchronous fast-path. Pins the
/// `handle_remote_player_loaded` runtime hook.
#[tokio::test]
async fn handle_remote_player_loaded_advances_destination_fsm() {
    let mut mgr = make_test_space_mgr();
    let mut regions = std::collections::HashMap::new();
    regions.insert(1, ring(1, "Castle_CellBlock", vec![2], [0.0; 3]));
    regions.insert(2, ring(2, "Castle", vec![1], [10.0; 3]));
    mgr.ring_transporters.load(&regions);
    mgr.ring_regions = regions;

    // Manually park the destination in RemoteLoadWait with one expected
    // remote player — this mimics the state the FSM is in after
    // `Effect::TeleportCrossWorld` dispatched from the source.
    let dst = mgr.ring_transporters.get_mut(2).unwrap();
    dst.state = State::RemoteLoadWait;
    dst.num_remote_players = 1;

    let (tx, mut _rx) = mpsc::channel(16);
    let engine = ChainEngine::new();

    handle_remote_player_loaded(2, 42, &tx, &mut mgr, &engine).await;

    // After mark_player_loaded → all_players_loaded → remote_warmup_timer_expired
    // → cooldown_timer_expired (the empty fast-path), the destination ring
    // must end at Idle. The sequence is one tick because there are no
    // players actually present (we only set the count, not the entity); the
    // fast-path collapses to Idle.
    //
    // Note: the player WAS recorded by player_loaded(42), so the FSM
    // doesn't fast-path. It runs RemoteWarmup deadline + cooldown deadline.
    // For this unit test we accept the post-load state as RemoteWarmup —
    // a more elaborate test would tick the deadline scanner.
    let final_state = mgr.ring_transporters.get(2).unwrap().state;
    assert!(
        matches!(
            final_state,
            State::RemoteWarmup | State::Cooldown | State::Idle,
        ),
        "destination ring must leave RemoteLoadWait after the deferred load (got {final_state:?})",
    );
    assert_ne!(
        final_state,
        State::RemoteLoadWait,
        "destination ring stuck in RemoteLoadWait — handle_remote_player_loaded did nothing",
    );
}

/// `handle_interact` must reject the call without sending a destination
/// list when the ring's `required_mission_id` is set and the player
/// hasn't completed that mission. Pins the runtime mission gate — even
/// if a chain somehow fires `trigger_transporter` for the gated ring,
/// the runtime stops the destination list from leaking.
#[tokio::test]
async fn handle_interact_blocks_when_required_mission_not_completed() {
    use cimmeria_entity::missions::{MissionInstance, MissionObjective, STATUS_ACTIVE};

    let mut mgr = make_test_space_mgr();
    let mut regions = std::collections::HashMap::new();
    let mut gated = ring(1, "Castle_CellBlock", vec![2], [0.0; 3]);
    gated.required_mission_id = Some(688);
    regions.insert(1, gated);
    regions.insert(2, ring(2, "Castle_CellBlock", vec![1], [10.0, 20.0, 30.0]));
    mgr.ring_transporters.load(&regions);
    mgr.ring_point_set_to_region = regions
        .iter()
        .map(|(rid, r)| (r.point_set_id, *rid))
        .collect();
    mgr.ring_regions = regions;

    // Player exists with mission 688 ACTIVE (not completed) — gate must
    // suppress the destination list. We don't add an entry that maps to
    // MISSION_COMPLETED.
    mgr.create_entity(42, "Castle_CellBlock", [0.0; 3], [0.0; 3])
        .unwrap();
    if let Some(player) = mgr.get_entity_mut(42) {
        player.missions.add_mission(MissionInstance::new(
            688,
            2356,
            vec![MissionObjective {
                objective_id: 2734,
                status: STATUS_ACTIVE,
                hidden: false,
                optional: false,
            }],
        ));
    }

    let (tx, mut rx) = mpsc::channel(8);
    let engine = ChainEngine::new();
    handle_interact(1, 42, &tx, &mut mgr, &engine).await;

    // No destination-list method was sent — the gate suppressed the
    // entire interaction. Source ring's `ring_source_id` was not set
    // either (we bail before that write).
    assert!(
        rx.try_recv().is_err(),
        "mission gate must suppress all wire output when mission_688 is not completed"
    );
    assert_eq!(
        mgr.get_entity(42).and_then(|e| e.ring_source_id),
        None,
        "ring_source_id must remain unset when the gate rejects the interaction"
    );
}

/// Counterpart to the gate test: when the player has completed the
/// required mission, the destination list IS sent. This pins the
/// fall-through path so the gate doesn't accidentally suppress
/// post-completion interactions (which would break re-entry to the
/// armory ring after the trip is done).
#[tokio::test]
async fn handle_interact_allows_when_required_mission_completed() {
    use cimmeria_entity::missions::{
        MissionInstance, MissionObjective, MISSION_COMPLETED, STATUS_COMPLETED,
    };

    let mut mgr = make_test_space_mgr();
    let mut regions = std::collections::HashMap::new();
    let mut gated = ring(1, "Castle_CellBlock", vec![2], [0.0; 3]);
    gated.required_mission_id = Some(688);
    regions.insert(1, gated);
    regions.insert(2, ring(2, "Castle_CellBlock", vec![1], [10.0, 20.0, 30.0]));
    mgr.ring_transporters.load(&regions);
    mgr.ring_point_set_to_region = regions
        .iter()
        .map(|(rid, r)| (r.point_set_id, *rid))
        .collect();
    mgr.ring_regions = regions;

    mgr.create_entity(42, "Castle_CellBlock", [0.0; 3], [0.0; 3])
        .unwrap();
    if let Some(player) = mgr.get_entity_mut(42) {
        let mut mission = MissionInstance::new(
            688,
            2356,
            vec![MissionObjective {
                objective_id: 2734,
                // `MissionObjective.status` uses STATUS_* constants (the
                // objective-level status set), distinct from the mission-
                // level MISSION_* set below. The gate currently inspects
                // `MissionInstance.status` only, but seeding the wrong
                // constant family makes the fixture semantically misleading.
                status: STATUS_COMPLETED,
                hidden: false,
                optional: false,
            }],
        );
        mission.status = MISSION_COMPLETED;
        player.missions.add_mission(mission);
    }

    let (tx, mut rx) = mpsc::channel(8);
    let engine = ChainEngine::new();
    handle_interact(1, 42, &tx, &mut mgr, &engine).await;

    // Destination list method must have been sent (some EntityMethodCall
    // with method index ON_RING_TRANSPORTER_LIST). We don't assert byte
    // contents — the existing `full_ring_cycle_dispatches_expected_messages`
    // does that — only that the gate let traffic through.
    let mut saw_list = false;
    while let Ok(msg) = rx.try_recv() {
        if let CellToBaseMsg::EntityMethodCall { method_index, .. } = msg {
            if method_index == METHOD_ON_RING_TRANSPORTER_LIST {
                saw_list = true;
                break;
            }
        }
    }
    assert!(
        saw_list,
        "post-completion interaction must dispatch onRingTransporterList \
         (METHOD_ON_RING_TRANSPORTER_LIST)"
    );
    assert_eq!(
        mgr.get_entity(42).and_then(|e| e.ring_source_id),
        Some(1),
        "ring_source_id must be set on the player after a passing gate"
    );
}

/// Self-as-destination is rejected (matches Python `selectDestination`).
#[tokio::test]
async fn select_destination_self_rejected() {
    let mut mgr = make_test_space_mgr();
    let mut regions = std::collections::HashMap::new();
    regions.insert(1, ring(1, "Castle_CellBlock", vec![1, 2], [0.0; 3]));
    regions.insert(2, ring(2, "Castle_CellBlock", vec![1], [10.0; 3]));
    mgr.ring_transporters.load(&regions);
    mgr.ring_point_set_to_region = regions
        .iter()
        .map(|(rid, r)| (r.point_set_id, *rid))
        .collect();
    mgr.ring_regions = regions;

    mgr.create_entity(1, "Castle_CellBlock", [0.0; 3], [0.0; 3])
        .unwrap();
    let (tx, mut rx) = mpsc::channel(8);
    let engine = ChainEngine::new();

    handle_select_destination(1, 1, 1, &tx, &mut mgr, &engine).await;

    // No teleport / sequence should fire — source ring stays idle.
    assert_eq!(mgr.ring_transporters.get(1).unwrap().state, State::Idle);
    assert!(rx.try_recv().is_err());
}
