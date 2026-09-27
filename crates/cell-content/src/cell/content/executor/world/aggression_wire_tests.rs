//! Wire-format + fan-out guards for `set_aggression`'s witness broadcast
//! (NA33).
//!
//! `set_aggression` used to be silent on the wire (audit gap, NA13 open
//! item — see `docs/gameplay/npc-ai.md` "Wire: not broadcast yet"). It now
//! sends `onAggressionOverrideUpdate` (SGWMob flat method index 27,
//! `docs/reverse-engineering/findings/npc-aggression-broadcast.md`) to every
//! AoI witness of the target NPC. Kept apart from `aggression_log_tests.rs`
//! (which is about the negative-log/level-validation shape, not the wire
//! payload) and `tests.rs` (already past the file-size cap).

use super::*;
use cimmeria_common::EntityId;
use cimmeria_entity::cell_entity::MobAggression;

/// Player `player_id` witnessing NPC `npc_id`, tagged `Drone`, no override
/// yet. Mirrors `tests::stage_drone_with_witness`'s shape (kept local since
/// that helper is private to the `tests` sibling module).
fn stage_drone_with_witness(mgr: &mut SpaceManager, player_id: u32, npc_id: u32) {
    mgr.create_entity(npc_id, "Agnos", [10.0, 0.0, 10.0], [0.0; 3])
        .unwrap();
    mgr.get_entity_mut(npc_id)
        .expect("npc entity must exist immediately after create_entity")
        .tag = Some("Drone".to_string());
    mgr.create_entity(player_id, "Agnos", [0.0; 3], [0.0; 3])
        .unwrap();
    let p = mgr
        .get_entity_mut(player_id)
        .expect("player entity must exist immediately after create_entity");
    p.is_player = true;
    p.player_id = Some(42);
    p.witnesses.insert(EntityId(npc_id as i32));
    mgr.connect_entity(player_id);
}

fn make_space_mgr() -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" Instanced="false" MinX="0" MaxX="100" MinY="0" MaxY="100" /></Spaces>"#;
    let cxml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(cxml).unwrap();
    mgr
}

/// Byte-exact wire-format guard (TESTING.md type 2): a HOSTILE override
/// (content level `1`) must serialize as `method_index = 27` (SGWMob's
/// `onAggressionOverrideUpdate`, not any of SGWPlayer's Communicator
/// indices that numerically collide at 27-33) with a single-byte INT8
/// payload equal to `MobAggression::Hostile as u8` (`1`). A regression that
/// substituted `mercury::method_idx::ON_PLAYER_COMMUNICATION` (also in the
/// 27-33 range, but for SGWPlayer) or that sent the level as `i32`/LE-u32
/// instead of a bare `INT8` would trip this.
#[tokio::test]
async fn set_aggression_broadcasts_byte_exact_wire_payload() {
    let mut mgr = make_space_mgr();
    stage_drone_with_witness(&mut mgr, /* player */ 1, /* npc */ 101);
    let (tx, mut rx) = mpsc::channel(8);

    set_aggression("Drone".to_string(), 1, 1, 1032, &tx, &mut mgr).await;

    let msg = rx.try_recv().expect("exactly one message must be sent");
    match msg {
        CellToBaseMsg::WitnessEntityMethod {
            witness_id,
            entity_id,
            method_index,
            args,
            entity_is_player,
        } => {
            assert_eq!(witness_id, 1, "witness_id must be the player_eid");
            assert_eq!(entity_id, 101, "entity_id must be the target NPC's eid");
            assert_eq!(
                method_index,
                crate::mercury::method_idx::ON_AGGRESSION_OVERRIDE_UPDATE,
                "method_index must be SGWMob's flat index 27, not SGWPlayer's \
                 Communicator range which numerically collides at 27-33"
            );
            assert_eq!(
                args,
                vec![MobAggression::Hostile.level()],
                "payload must be exactly one INT8 byte (the EMobAggressionLevel \
                 value), not a wider integer encoding"
            );
            assert!(
                !entity_is_player,
                "the target is an NPC ghost -- entity_is_player selects the \
                 IDBASE_NPC_DEFAULT idbase on the base side"
            );
        }
        other => panic!("expected WitnessEntityMethod, got {other:?}"),
    }
    assert!(rx.try_recv().is_err(), "no second message expected");
}

/// Fan-out cardinality guard (TESTING.md type 8): exactly one
/// `WitnessEntityMethod` per witness, addressed to that witness — never to
/// the NPC's own non-existent client, never double-sent, never dropped for
/// a witness beyond the first.
#[tokio::test]
async fn set_aggression_broadcasts_to_every_witness_exactly_once() {
    let mut mgr = make_space_mgr();
    stage_drone_with_witness(&mut mgr, /* player */ 1, /* npc */ 101);
    // Second witness.
    mgr.create_entity(2, "Agnos", [0.0; 3], [0.0; 3]).unwrap();
    let p2 = mgr
        .get_entity_mut(2)
        .expect("second player entity must exist immediately after create_entity");
    p2.is_player = true;
    p2.player_id = Some(43);
    p2.witnesses.insert(EntityId(101));
    mgr.connect_entity(2);
    let (tx, mut rx) = mpsc::channel(16);

    set_aggression("Drone".to_string(), 3, 1, 1032, &tx, &mut mgr).await;

    let mut emitted: Vec<(u32, u32, u16, Vec<u8>)> = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        match msg {
            CellToBaseMsg::WitnessEntityMethod {
                witness_id,
                entity_id,
                method_index,
                args,
                ..
            } => emitted.push((witness_id, entity_id, method_index, args)),
            other => panic!("expected WitnessEntityMethod, got {other:?}"),
        }
    }
    emitted.sort_by_key(|(w, _, _, _)| *w);

    assert_eq!(
        emitted.len(),
        2,
        "exactly one WitnessEntityMethod per witness -- a regression that \
         iterated the wrong entity's witness set, or fanned out twice, \
         would change this count"
    );
    let expected_payload = vec![MobAggression::Neutral.level()];
    for (i, expected_witness) in [1u32, 2u32].iter().enumerate() {
        let (witness_id, entity_id, method_index, args) = &emitted[i];
        assert_eq!(witness_id, expected_witness);
        assert_eq!(*entity_id, 101);
        assert_eq!(
            *method_index,
            crate::mercury::method_idx::ON_AGGRESSION_OVERRIDE_UPDATE
        );
        assert_eq!(args, &expected_payload);
    }
}

/// A tag miss must not send any wire message — the log-only negative path
/// (`aggression_log_tests.rs`) already covers the WARN; this pins that the
/// silent-no-mutation path is also silent-on-the-wire.
#[tokio::test]
async fn set_aggression_tag_miss_sends_no_wire_message() {
    let mut mgr = make_space_mgr();
    stage_drone_with_witness(&mut mgr, 1, 101);
    let (tx, mut rx) = mpsc::channel(8);

    set_aggression("NotDrone".to_string(), 1, 1, 1032, &tx, &mut mgr).await;

    assert!(
        rx.try_recv().is_err(),
        "a tag miss must not emit any WitnessEntityMethod"
    );
}
