//! Squad wiring in the base-message loop (ORG-03): the `Org` arm reaches
//! the squad handlers, `DisconnectEntity` removes the member, and
//! `InitPlayerState` re-sends the squad. The handlers themselves are tested
//! in `cimmeria-cell-methods` (`cell_methods::organization::tests`); these
//! guard only that each arm calls them.

use super::*;
use crate::cell::messages::OrgBaseToCell;
use crate::test_support::make_space_manager;

const SID: i32 = 0x4000_0000;

fn player(mgr: &mut SpaceManager, entity_id: u32, player_id: i32, name: &str) {
    mgr.create_entity(entity_id, "Agnos", [0.0; 3], [0.0; 3])
        .unwrap();
    mgr.connect_entity(entity_id);
    let e = mgr.get_entity_mut(entity_id).unwrap();
    e.player_id = Some(player_id);
    e.character_name = Some(name.into());
}

/// The client-method calls queued for the base, as `(entity, method)`.
fn calls(rx: &mut mpsc::Receiver<CellToBaseMsg>) -> Vec<(u32, u16)> {
    let mut out = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        if let CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index,
            ..
        } = msg
        {
            out.push((entity_id, method_index));
        }
    }
    out
}

/// Alice (entity 11) and Bob (12) in one squad, through the real arms:
/// the base-forwarded invite, then Bob's CM 8 accept.
async fn pair(
    mgr: &mut SpaceManager,
    tx: &mpsc::Sender<CellToBaseMsg>,
    rx: &mut mpsc::Receiver<CellToBaseMsg>,
    engine: &ChainEngine,
) {
    player(mgr, 11, 1, "Alice");
    player(mgr, 12, 2, "Bob");
    let invite = OrgBaseToCell::SquadInvite {
        player_id: 1,
        entity_id: 11,
        target_name: "Bob".into(),
    };
    handle_base_message(BaseToCellMsg::Org(invite), tx, mgr, engine, &[]).await;
    assert!(
        calls(rx).contains(&(12, 34)),
        "SquadInvite must reach the squad handler and send onOrganizationInvite"
    );
    // CM 8 `organizationInviteResponse(1, accept)`.
    let accept = BaseToCellMsg::CellMethodCall {
        entity_id: 12,
        method_index: 8,
        args: vec![1, 0, 0, 0, 1],
    };
    handle_base_message(accept, tx, mgr, engine, &[]).await;
    assert_eq!(mgr.squads.squad_of(2), Some(SID));
    calls(rx);
}

#[tokio::test]
async fn squad_kick_from_the_base_reaches_the_squad_handler() {
    let mut mgr = make_space_manager();
    let (tx, mut rx) = mpsc::channel(64);
    let engine = ChainEngine::new();
    pair(&mut mgr, &tx, &mut rx, &engine).await;
    let kick = OrgBaseToCell::SquadKick {
        player_id: 1,
        entity_id: 11,
        org_id: SID,
        target_name: "Bob".into(),
    };
    handle_base_message(BaseToCellMsg::Org(kick), &tx, &mut mgr, &engine, &[]).await;
    assert!(calls(&mut rx).contains(&(12, 36)), "Bob is told he left");
    assert_eq!(mgr.squads.squad_count(), 0);
}

/// `DisconnectEntity` removes the member: Alice is told Bob left and the
/// squad of one dissolved.
#[tokio::test]
async fn disconnect_entity_removes_the_squad_member() {
    let mut mgr = make_space_manager();
    let (tx, mut rx) = mpsc::channel(64);
    let engine = ChainEngine::new();
    pair(&mut mgr, &tx, &mut rx, &engine).await;
    handle_base_message(
        BaseToCellMsg::DisconnectEntity { entity_id: 12 },
        &tx,
        &mut mgr,
        &engine,
        &[],
    )
    .await;
    assert_eq!(calls(&mut rx), [(11, 39), (11, 36)]);
    assert_eq!(mgr.squads.squad_count(), 0);
}

/// `DestroyEntity` is also the gate-travel teardown: it must NOT touch the
/// squad, or a member would lose it on every world change.
#[tokio::test]
async fn destroy_entity_keeps_the_squad() {
    let mut mgr = make_space_manager();
    let (tx, mut rx) = mpsc::channel(64);
    let engine = ChainEngine::new();
    pair(&mut mgr, &tx, &mut rx, &engine).await;
    handle_base_message(
        BaseToCellMsg::DestroyEntity { entity_id: 12 },
        &tx,
        &mut mgr,
        &engine,
        &[],
    )
    .await;
    assert_eq!(mgr.squads.squad_of(2), Some(SID));
    assert!(!calls(&mut rx).iter().any(|&(_, m)| (35..=39).contains(&m)));
}

/// `InitPlayerState` (every world entry) re-sends the squad to a member
/// whose entity was re-created, and re-stamps `squad_id`.
#[tokio::test]
async fn init_player_state_replays_the_squad() {
    let mut mgr = make_space_manager();
    let (tx, mut rx) = mpsc::channel(256);
    let engine = ChainEngine::new();
    pair(&mut mgr, &tx, &mut rx, &engine).await;
    mgr.get_entity_mut(12).unwrap().squad_id = None;
    handle_base_message(
        BaseToCellMsg::InitPlayerState {
            entity_id: 12,
            player_id: 2,
            account_id: 6,
            world_name: "Agnos".into(),
            archetype_id: 1,
            saved_missions: vec![],
            abilities: vec![],
            active_bandolier_slot: 0,
            bandolier_items: vec![],
            system_options: cimmeria_entity::cell_entity::SystemOptions::default(),
            state_field: 0,
            access_level: 0,
            known_stargates: vec![],
            tree_progress: Default::default(),
            level: 12,
            character_name: Some("Bob".into()),
            body_set: None,
        },
        &tx,
        &mut mgr,
        &engine,
        &[],
    )
    .await;
    let squad_calls: Vec<_> = calls(&mut rx)
        .into_iter()
        .filter(|&(_, m)| (35..=51).contains(&m))
        .collect();
    assert_eq!(squad_calls, [(12, 35), (12, 38), (12, 37), (12, 51)]);
    assert_eq!(mgr.get_entity(12).unwrap().squad_id, Some(SID));
}
