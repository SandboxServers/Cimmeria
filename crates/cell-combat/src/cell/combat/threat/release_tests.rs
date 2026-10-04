//! `release_npc_from_player_combat`: the drain every non-death despawn shares.
//!
//! Revert proofs: drop the drain and the fighter stays in combat; send the
//! state field to the owner only (`send_entity_method`) and the observer
//! gets nothing.

use tokio::sync::mpsc;

use crate::cell::combat::{
    generate_threat, release_npc_from_player_combat, AggroCause, BSF_IN_COMBAT,
};
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use crate::mercury::method_idx::ON_STATE_FIELD_UPDATE;

/// Fought only `NPC`.
const FIGHTER: u32 = 1;
/// Fought `NPC` and is still fighting `OTHER`.
const BUSY: u32 = 2;
/// Sees `FIGHTER`, fights nothing.
const OBSERVER: u32 = 3;

fn world() -> (SpaceManager, u32, u32) {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#,
    )
    .unwrap();
    for p in [FIGHTER, BUSY, OBSERVER] {
        mgr.create_entity(p, "Castle", [p as f32, 0.0, 0.0], [0.0; 3])
            .unwrap();
        let e = mgr.get_entity_mut(p).unwrap();
        e.is_player = true;
        e.player_id = Some(100 + p as i32);
        mgr.connect_entity(p);
    }
    mgr.get_entity_mut(OBSERVER)
        .unwrap()
        .witnesses
        .insert(cimmeria_common::EntityId(FIGHTER as i32));
    let npc = mgr.allocate_npc_id();
    mgr.spawn_npc(npc, "Castle", [5.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    let other = mgr.allocate_npc_id();
    mgr.spawn_npc(other, "Castle", [6.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    let _ = generate_threat(&mut mgr, FIGHTER, npc, 10.0, AggroCause::Damage);
    let _ = generate_threat(&mut mgr, BUSY, npc, 10.0, AggroCause::Damage);
    let _ = generate_threat(&mut mgr, BUSY, other, 10.0, AggroCause::Damage);
    (mgr, npc, other)
}

#[tokio::test]
async fn release_takes_the_fighter_out_and_tells_it_and_its_observer_once() {
    let (mut mgr, npc, other) = world();
    let (tx, mut rx) = mpsc::channel(64);

    let exits = release_npc_from_player_combat(npc, "test", &tx, &mut mgr).await;

    assert_eq!(exits, 1, "only the fighter left combat");
    let fighter = mgr.get_entity(FIGHTER).unwrap();
    assert!(fighter.threatened_mobs.is_empty());
    assert_eq!(fighter.state_field & BSF_IN_COMBAT, 0);
    let busy = mgr.get_entity(BUSY).unwrap();
    assert_eq!(
        busy.threatened_mobs.iter().copied().collect::<Vec<_>>(),
        vec![other]
    );
    assert!(busy.state_field & BSF_IN_COMBAT != 0, "still fighting");
    assert!(mgr.get_entity(npc).unwrap().threat_list.is_empty());

    let msgs: Vec<_> = std::iter::from_fn(|| rx.try_recv().ok()).collect();
    let own: Vec<u32> = msgs
        .iter()
        .filter_map(|m| match m {
            CellToBaseMsg::EntityMethodCall {
                entity_id,
                method_index: ON_STATE_FIELD_UPDATE,
                ..
            } => Some(*entity_id),
            _ => None,
        })
        .collect();
    assert_eq!(own, vec![FIGHTER], "the fighter's own client is told");
    let seen: Vec<(u32, u32, Vec<u8>)> = msgs
        .iter()
        .filter_map(|m| match m {
            CellToBaseMsg::WitnessEntityMethod {
                witness_id,
                entity_id,
                method_index: ON_STATE_FIELD_UPDATE,
                args,
                ..
            } => Some((*witness_id, *entity_id, args.clone())),
            _ => None,
        })
        .collect();
    assert_eq!(
        seen,
        vec![(
            OBSERVER,
            FIGHTER,
            fighter.state_field.to_le_bytes().to_vec()
        )],
        "exactly one witness update, to the observer, with the new state"
    );
}

#[tokio::test]
async fn release_of_a_player_id_does_nothing() {
    let (mut mgr, _npc, _other) = world();
    let (tx, mut rx) = mpsc::channel(8);
    assert_eq!(
        release_npc_from_player_combat(FIGHTER, "test", &tx, &mut mgr).await,
        0
    );
    assert!(rx.try_recv().is_err());
    assert!(mgr.get_entity(FIGHTER).unwrap().state_field & BSF_IN_COMBAT != 0);
}
