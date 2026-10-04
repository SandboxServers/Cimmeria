//! `messaging` routing guards: the three fan-out helpers and the movement
//! type cache.

use super::*;
use crate::cell::abilities::broadcast_movement_type;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

/// Two players + one NPC in the same Castle space. Both players are
/// connected and AoI is computed, so each player sees the others +
/// the NPC. Returns the manager + an mpsc rx for asserting on emitted
/// `CellToBaseMsg` traffic.
fn make_mgr_two_players_and_npc() -> (SpaceManager, mpsc::Receiver<CellToBaseMsg>) {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#,
    )
    .unwrap();
    // Two players + one NPC, all co-located so AoI naturally captures all.
    mgr.create_entity(1, "Castle", [0.0; 3], [0.0; 3]).unwrap();
    mgr.create_entity(2, "Castle", [0.0; 3], [0.0; 3]).unwrap();
    mgr.create_entity(3, "Castle", [0.0; 3], [0.0; 3]).unwrap();
    if let Some(p) = mgr.get_entity_mut(1) {
        p.is_player = true;
        p.player_id = Some(100);
    }
    if let Some(p) = mgr.get_entity_mut(2) {
        p.is_player = true;
        p.player_id = Some(200);
    }
    // entity 3 stays an NPC.
    mgr.connect_entity(1);
    mgr.connect_entity(2);
    let _ = mgr.compute_aoi_changes();
    let (_tx, rx) = mpsc::channel(64);
    (mgr, rx)
}

fn drain(rx: &mut mpsc::Receiver<CellToBaseMsg>) -> Vec<CellToBaseMsg> {
    let mut out = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        out.push(msg);
    }
    out
}

/// Witness-only fanout for a player who has one other player in AoI:
/// emits exactly one `WitnessEntityMethod` to that other player, and
/// zero `EntityMethodCall` to self.
#[tokio::test]
async fn witnesses_only_fanout_skips_self_and_addresses_each_observer() {
    let (mgr, _rx) = make_mgr_two_players_and_npc();
    let (tx, mut rx) = mpsc::channel(64);
    let count = send_entity_method_to_witnesses(
        1,
        19, // ON_STATE_FIELD_UPDATE — arbitrary; the helper is method-agnostic
        vec![0xDE, 0xAD],
        &tx,
        &mgr,
    )
    .await;
    // Player 2 sees player 1, so exactly one witness.
    assert_eq!(count, 1);
    let msgs = drain(&mut rx);
    assert_eq!(msgs.len(), 1);
    match &msgs[0] {
        CellToBaseMsg::WitnessEntityMethod {
            witness_id,
            entity_id,
            method_index,
            args,
            ..
        } => {
            assert_eq!(*witness_id, 2);
            assert_eq!(*entity_id, 1);
            assert_eq!(*method_index, 19);
            assert_eq!(args, &vec![0xDE, 0xAD]);
        }
        other => panic!("expected WitnessEntityMethod, got {other:?}"),
    }
}

/// Witness-only with no observers: returns 0, emits nothing, does NOT
/// log a warning. This is the path a player alone in a space hits when
/// their state flips — the helper must stay silent rather than spam.
#[tokio::test]
async fn witnesses_only_with_no_observers_is_a_clean_zero() {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#,
    )
    .unwrap();
    mgr.create_entity(1, "Castle", [0.0; 3], [0.0; 3]).unwrap();
    if let Some(p) = mgr.get_entity_mut(1) {
        p.is_player = true;
        p.player_id = Some(100);
    }
    mgr.connect_entity(1);
    let _ = mgr.compute_aoi_changes();
    let (tx, mut rx) = mpsc::channel(64);

    let count = send_entity_method_to_witnesses(1, 19, vec![], &tx, &mgr).await;
    assert_eq!(count, 0);
    assert!(drain(&mut rx).is_empty());
}

/// **Regression guard (colo smoke test, 2026-10-04).** A lone player's
/// Heal Focus `onStatUpdate` fan-out found no witnesses and logged the one
/// row of the cast with no `event` and no ids. The row now has a stable
/// event, the player and the cast. Fails on revert: `event` is absent.
#[tokio::test]
async fn a_fan_out_with_no_witnesses_names_its_event_player_and_cast() {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#,
    )
    .unwrap();
    mgr.create_entity(1, "Castle", [0.0; 3], [0.0; 3]).unwrap();
    if let Some(p) = mgr.get_entity_mut(1) {
        p.is_player = true;
        p.player_id = Some(100);
    }
    mgr.connect_entity(1);
    let _ = mgr.compute_aoi_changes();
    let (tx, _rx) = mpsc::channel(64);
    let logs = crate::test_support::LogCapture::install();

    let outer = mgr.enter_cast_scope(Some(7));
    send_entity_method_to_self_and_witnesses(1, 20, 0u32.to_le_bytes().to_vec(), &tx, &mgr).await;
    mgr.exit_cast_scope(outer);

    let row = logs
        .all()
        .into_iter()
        .find(|c| c.target == "abilities.wire" && c.message_contains("no witnesses"))
        .expect("the no-witnesses row");
    assert!(row.has_field("event", EVENT_NO_WITNESSES), "{row:?}");
    assert!(row.has_field("cast_id", "7"), "{row:?}");
    assert!(row.has_field("player_id", "100"), "{row:?}");
    assert!(row.has_field("entity_id", "1"), "{row:?}");
    assert!(row.has_field("method", "onStatUpdate"), "{row:?}");
    assert!(row.has_field("route", "self_and_witnesses"), "{row:?}");
    assert!(row.has_field("self_send", "true"), "{row:?}");
    // The owner's send still went out, so the text must not claim otherwise
    // (Copilot on #1198).
    assert!(!row.message_contains("nothing emitted"), "{row:?}");
}

/// Self + witnesses for a player: one `EntityMethodCall` to self, one
/// `WitnessEntityMethod` per observer. Returns the witness count
/// (not counting the self send).
#[tokio::test]
async fn self_and_witnesses_for_player_sends_both() {
    let (mgr, _rx) = make_mgr_two_players_and_npc();
    let (tx, mut rx) = mpsc::channel(64);

    let witness_count =
        send_entity_method_to_self_and_witnesses(1, 19, vec![0xBE, 0xEF], &tx, &mgr).await;
    assert_eq!(witness_count, 1);

    let msgs = drain(&mut rx);
    let self_sends: Vec<_> = msgs
        .iter()
        .filter(|m| matches!(m, CellToBaseMsg::EntityMethodCall { entity_id: 1, .. }))
        .collect();
    let witness_sends: Vec<_> = msgs
        .iter()
        .filter(|m| matches!(m, CellToBaseMsg::WitnessEntityMethod { entity_id: 1, .. }))
        .collect();
    assert_eq!(self_sends.len(), 1, "expected exactly one self send");
    assert_eq!(witness_sends.len(), 1, "expected exactly one witness send");
}

/// Self + witnesses for an NPC: the "self" path is a no-op (NPCs have
/// no client), and the helper collapses to witnesses-only. Verifies
/// no `EntityMethodCall` is emitted for the NPC.
#[tokio::test]
async fn self_and_witnesses_for_npc_skips_self_send() {
    let (mgr, _rx) = make_mgr_two_players_and_npc();
    let (tx, mut rx) = mpsc::channel(64);

    // Entity 3 is the NPC; both players witness it.
    let witness_count = send_entity_method_to_self_and_witnesses(3, 19, vec![], &tx, &mgr).await;
    // Both players are co-located and see the NPC.
    assert_eq!(witness_count, 2);

    let msgs = drain(&mut rx);
    let self_sends: Vec<_> = msgs
        .iter()
        .filter(|m| matches!(m, CellToBaseMsg::EntityMethodCall { entity_id: 3, .. }))
        .collect();
    let witness_sends: Vec<_> = msgs
        .iter()
        .filter(|m| matches!(m, CellToBaseMsg::WitnessEntityMethod { entity_id: 3, .. }))
        .collect();
    assert!(
        self_sends.is_empty(),
        "NPC must not receive an EntityMethodCall — NPCs have no client"
    );
    assert_eq!(witness_sends.len(), 2);
}

/// Behavior parity: `send_entity_method` for an NPC fans out to the same
/// witness set `send_entity_method_to_witnesses` would. This pins that
/// the new witness-only helper is a non-disruptive extension — paths
/// that already use the entity-aware default keep their existing
/// behavior unchanged when the new helper lands.
#[tokio::test]
async fn npc_send_entity_method_matches_witnesses_only_helper() {
    let (mgr, _rx) = make_mgr_two_players_and_npc();
    let (tx_a, mut rx_a) = mpsc::channel(64);
    let (tx_b, mut rx_b) = mpsc::channel(64);

    send_entity_method(3, 19, vec![1, 2, 3], &tx_a, &mgr).await;
    let count_b = send_entity_method_to_witnesses(3, 19, vec![1, 2, 3], &tx_b, &mgr).await;

    let msgs_a = drain(&mut rx_a);
    let msgs_b = drain(&mut rx_b);
    assert_eq!(msgs_a.len(), msgs_b.len());
    assert_eq!(msgs_a.len(), count_b);
}

// ── broadcast_movement_type ────────────────────────────────────────────

fn movement_type_rows(logs: &crate::test_support::LogCaptureGuard, outcome: &str) -> usize {
    logs.all()
        .into_iter()
        .filter(|c| c.target == "movement.movement_type" && c.has_field("outcome", outcome))
        .count()
}

/// NA10 regression guard. A movement-type change is recorded in the cache
/// and sends **nothing**. It used to send each witness a
/// `WitnessEntityMethod` with method index 1 and payload `[kind]`. Client
/// method 1 is `onSequence`, so that was a truncated Kismet-sequence
/// trigger, not a movement type. Restoring the send fails this test.
#[tokio::test]
async fn broadcast_movement_type_records_the_cache_and_sends_nothing() {
    use cimmeria_entity::cell_entity::MobMovementType;

    let (mut mgr, _rx) = make_mgr_two_players_and_npc();
    let (tx, mut rx) = mpsc::channel(64);
    let logs = crate::test_support::LogCapture::install();

    broadcast_movement_type(3, Some(MobMovementType::Patrol), &tx, &mut mgr).await;

    let msgs = drain(&mut rx);
    assert!(
        msgs.is_empty(),
        "no wire message may go out for a movement type: {msgs:?}"
    );
    assert_eq!(
        mgr.get_entity(3).unwrap().last_movement_type,
        Some(MobMovementType::Patrol),
    );
    assert_eq!(movement_type_rows(&logs, "suppressed"), 1);
}

/// Re-asserting the same kind is deduplicated: one `suppressed` row per
/// change, not one per AI tick.
#[tokio::test]
async fn broadcast_movement_type_same_kind_logs_once() {
    use cimmeria_entity::cell_entity::MobMovementType;

    let (mut mgr, _rx) = make_mgr_two_players_and_npc();
    let (tx, _rx2) = mpsc::channel(64);
    let logs = crate::test_support::LogCapture::install();

    broadcast_movement_type(3, Some(MobMovementType::CombatAdvance), &tx, &mut mgr).await;
    broadcast_movement_type(3, Some(MobMovementType::CombatAdvance), &tx, &mut mgr).await;
    assert_eq!(movement_type_rows(&logs, "suppressed"), 1);
}

/// `None` clears the cache, so the next kind is a change again.
#[tokio::test]
async fn broadcast_movement_type_none_clears_the_cache() {
    use cimmeria_entity::cell_entity::MobMovementType;

    let (mut mgr, _rx) = make_mgr_two_players_and_npc();
    let (tx, mut rx) = mpsc::channel(64);
    let logs = crate::test_support::LogCapture::install();

    broadcast_movement_type(3, Some(MobMovementType::Patrol), &tx, &mut mgr).await;
    broadcast_movement_type(3, None, &tx, &mut mgr).await;
    assert_eq!(mgr.get_entity(3).unwrap().last_movement_type, None);
    broadcast_movement_type(3, Some(MobMovementType::Patrol), &tx, &mut mgr).await;

    assert_eq!(movement_type_rows(&logs, "cleared"), 1);
    assert_eq!(movement_type_rows(&logs, "suppressed"), 2);
    assert!(drain(&mut rx).is_empty());
}
