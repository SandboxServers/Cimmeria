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

/// A member who disconnects in gate transit (an aborted transfer, or a
/// crash mid-transfer) has no cell entity left: `DestroyEntity` removed it
/// and the arrival never re-created it. `DisconnectEntity` must still remove
/// them, found by their last entity id, and Alice's [39] must name that id.
#[tokio::test]
async fn disconnect_in_gate_transit_removes_the_squad_member() {
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
    assert!(mgr.get_entity(12).is_none(), "fixture: Bob is in transit");
    calls(&mut rx);
    handle_base_message(
        BaseToCellMsg::DisconnectEntity { entity_id: 12 },
        &tx,
        &mut mgr,
        &engine,
        &[],
    )
    .await;
    let mut left = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        if let CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index,
            args,
        } = msg
        {
            left.push((entity_id, method_index, args));
        }
    }
    let order: Vec<(u32, u16)> = left.iter().map(|(e, m, _)| (*e, *m)).collect();
    assert_eq!(order, [(11, 39), (11, 36)]);
    assert_eq!(
        left[0].2[..4],
        12i32.to_le_bytes(),
        "[39] names Bob's last entity"
    );
    assert_eq!(mgr.squads.squad_of(2), None);
    assert_eq!(mgr.squads.squad_count(), 0);
}

/// A `DisconnectEntity` naming a member's old entity id while that member
/// is live under another id is not theirs: the member stays, and the
/// mismatch is a WARN seam (`reason = stale_entity_id`).
#[tokio::test]
async fn disconnect_of_a_stale_entity_id_keeps_the_member_and_warns() {
    let capture = crate::test_support::LogCapture::install();
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
    // Bob is live again as entity 13, before any world-entry replay has
    // re-recorded his entity.
    player(&mut mgr, 13, 2, "Bob");
    calls(&mut rx);
    handle_base_message(
        BaseToCellMsg::DisconnectEntity { entity_id: 12 },
        &tx,
        &mut mgr,
        &engine,
        &[],
    )
    .await;
    assert_eq!(mgr.squads.squad_of(2), Some(SID), "Bob keeps his squad");
    assert!(calls(&mut rx).is_empty());
    let warn = capture
        .find_event(tracing::Level::WARN, "old entity id", "stale_entity_id")
        .expect("the stale-id disconnect must WARN");
    assert!(warn.has_field("event", "squad.disconnect_stale_entity"));
    assert!(warn.has_field("player_id", "2") && warn.has_field("live_entity_id", "13"));
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

/// ORG-06's Bank hook: `OrgMembershipEnded` reaches the `Org` arm, which
/// logs `org.membership_ended` (the arm the Bank's BV-07 extends) and sends
/// nothing to any client.
#[tokio::test]
async fn membership_ended_reaches_the_org_arm() {
    use cimmeria_entity::organization::OrgLeaveReason;
    let capture = crate::test_support::LogCapture::install();
    let mut mgr = make_space_manager();
    let (tx, mut rx) = mpsc::channel(64);
    player(&mut mgr, 11, 1, "Alice");
    let ended = OrgBaseToCell::OrgMembershipEnded {
        player_id: 1,
        entity_id: 11,
        org_id: 5,
        reason: OrgLeaveReason::Disbanded,
    };
    handle_base_message(
        BaseToCellMsg::Org(ended),
        &tx,
        &mut mgr,
        &ChainEngine::new(),
        &[],
    )
    .await;
    assert!(calls(&mut rx).is_empty());
    let ev = capture
        .all()
        .into_iter()
        .find(|c| c.has_field("event", "org.membership_ended"))
        .expect("org.membership_ended");
    assert_eq!(
        (ev.level, ev.target.as_str()),
        (tracing::Level::DEBUG, "org")
    );
    assert!(
        ev.has_field("org_id", "5") && ev.has_field("live_entity", "true"),
        "{ev:?}"
    );
}

/// ORG-05 wiring: the base's `RegistrarEligible` reaches the creation
/// handler (the offer is recorded and `launchOrganizationCreation` [135]
/// goes out), `CreateResult` settles the offer, and `DisconnectEntity` drops
/// an offer still open.
#[tokio::test]
async fn creation_arms_reach_the_creation_handlers() {
    use cimmeria_entity::organization::OrgType;

    let mut mgr = make_space_manager();
    let (tx, mut rx) = mpsc::channel(64);
    let engine = ChainEngine::new();
    player(&mut mgr, 11, 1, "Alice");
    let eligible = |org_type| {
        BaseToCellMsg::Org(OrgBaseToCell::RegistrarEligible {
            player_id: 1,
            entity_id: 11,
            npc_entity_id: 900,
            org_type,
        })
    };

    handle_base_message(eligible(OrgType::Team), &tx, &mut mgr, &engine, &[]).await;
    assert_eq!(calls(&mut rx), [(11, 135)]);
    assert!(mgr.org_creations.get(1).is_some());

    let created = OrgBaseToCell::CreateResult {
        player_id: 1,
        entity_id: 11,
        org_type: OrgType::Team,
        created: true,
    };
    handle_base_message(BaseToCellMsg::Org(created), &tx, &mut mgr, &engine, &[]).await;
    assert!(
        mgr.org_creations.get(1).is_none(),
        "a creation closes the offer"
    );

    handle_base_message(eligible(OrgType::Command), &tx, &mut mgr, &engine, &[]).await;
    calls(&mut rx);
    handle_base_message(
        BaseToCellMsg::DisconnectEntity { entity_id: 11 },
        &tx,
        &mut mgr,
        &engine,
        &[],
    )
    .await;
    assert!(mgr.org_creations.is_empty(), "logging out drops the offer");
}
