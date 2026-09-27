//! Unit tests for the Banker open path, the vault session lifecycle and
//! `vault_move_allowed`. The end-to-end click through the wire dispatcher
//! (range gate included) is in `cimmeria-cell-methods`'
//! `interaction/bank_dispatch_tests.rs`.

use tokio::sync::mpsc;

use cimmeria_entity::cell_entity::{NpcInteractionType, VaultScope, VaultSession};
use cimmeria_wire::cell::client_methods::communicator::ON_PLAYER_COMMUNICATION;
use cimmeria_wire::cell::client_methods::player::ON_VAULT_OPEN;
use cimmeria_wire::cell::vault::build_vault_open_args;

use super::*;
use crate::cell::interactions::handle_interact;
use crate::cell::messages::CellToBaseMsg;

pub(super) const PLAYER: u32 = 1;

/// Two worlds, so a Banker can be put in "another space". The player
/// carries a known identity so the telemetry tests can check correlators.
pub(super) fn two_space_manager() -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    mgr.parse_spaces_xml(
        r#"<?xml version="1.0"?><Spaces>
<Space WorldName="Agnos" Instanced="false" MinX="-500" MaxX="500" MinY="-500" MaxY="500" />
<Space WorldName="Harset" Instanced="false" MinX="-500" MaxX="500" MinY="-500" MaxY="500" />
</Spaces>"#,
    )
    .unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" /><Space WorldName="Harset" /></Spaces>"#,
    )
    .unwrap();
    mgr.create_entity(PLAYER, "Agnos", [0.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    mgr.connect_entity(PLAYER);
    let p = mgr.get_entity_mut(PLAYER).unwrap();
    p.account_id = Some(6);
    p.player_id = Some(12);
    mgr
}

/// A Banker NPC of `scope` at `pos` in `world`.
pub(super) fn spawn_banker(
    mgr: &mut SpaceManager,
    world: &str,
    pos: [f32; 3],
    scope: VaultScope,
) -> u32 {
    let id = mgr.allocate_npc_id();
    mgr.spawn_npc(id, world, pos, [0.0; 3]).unwrap();
    let npc = mgr.get_entity_mut(id).unwrap();
    npc.interaction_type = Some(NpcInteractionType::Banker { scope });
    npc.faction = 1;
    id
}

fn session(mgr: &SpaceManager) -> Option<VaultSession> {
    mgr.get_entity(PLAYER).unwrap().vault_session.clone()
}

fn allowed(mgr: &SpaceManager) -> Result<(), VaultReject> {
    vault_move_allowed(mgr.get_entity(PLAYER).unwrap(), mgr)
}

fn drain(rx: &mut mpsc::Receiver<CellToBaseMsg>) -> Vec<(u32, u16, Vec<u8>)> {
    let mut out = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        if let CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index,
            args,
        } = msg
        {
            out.push((entity_id, method_index, args));
        }
    }
    out
}

/// A personal Banker in range: exactly one `onVaultOpen` to the player,
/// carrying the Banker's id and position, and a session pinned to it in the
/// player's space. Fails if the Banker arm is removed from
/// `handle_interact` (the click goes to the `None` arm and sends nothing).
#[tokio::test]
async fn personal_banker_click_sends_one_vault_open_and_opens_a_session() {
    let mut mgr = two_space_manager();
    let banker = spawn_banker(&mut mgr, "Agnos", [2.0, 0.5, -1.0], VaultScope::Personal);
    let (tx, mut rx) = mpsc::channel(16);

    assert!(handle_interact(PLAYER, banker, &tx, &mut mgr)
        .await
        .is_none());

    let sent = drain(&mut rx);
    assert_eq!(
        sent,
        vec![(
            PLAYER,
            ON_VAULT_OPEN,
            build_vault_open_args(banker as i32, [2.0, 0.5, -1.0])
        )],
        "exactly one onVaultOpen(banker_id, banker_position)"
    );
    let s = session(&mgr).expect("the click opens a vault session");
    assert_eq!(s.scope, VaultScope::Personal);
    assert_eq!(s.banker_id, Some(banker));
    assert_eq!(Some(s.space_id), mgr.get_entity_space_id(PLAYER));
    assert_eq!(allowed(&mgr), Ok(()));
}

/// Team and Command Bankers are refused until the org vaults land: no
/// `onVaultOpen`, no session, and one visible chat line.
#[tokio::test]
async fn org_banker_click_is_refused_with_feedback_and_no_session() {
    for scope in [VaultScope::Team, VaultScope::Command] {
        let mut mgr = two_space_manager();
        let banker = spawn_banker(&mut mgr, "Agnos", [2.0, 0.0, 0.0], scope);
        let (tx, mut rx) = mpsc::channel(16);

        handle_interact(PLAYER, banker, &tx, &mut mgr).await;

        let methods: Vec<u16> = drain(&mut rx).into_iter().map(|(_, m, _)| m).collect();
        assert_eq!(
            methods,
            vec![ON_PLAYER_COMMUNICATION],
            "{scope:?}: one feedback line and nothing else"
        );
        assert_eq!(session(&mgr), None, "{scope:?} must not open a session");
    }
}

/// A later interact with another NPC ends the session; re-clicking the
/// same Banker keeps it. Fails if `handle_interact`'s pin stops going
/// through `pin_interaction_target`.
#[tokio::test]
async fn interacting_with_another_npc_ends_the_session() {
    let mut mgr = two_space_manager();
    let banker = spawn_banker(&mut mgr, "Agnos", [2.0, 0.0, 0.0], VaultScope::Personal);
    let other = mgr.allocate_npc_id();
    mgr.spawn_npc(other, "Agnos", [0.0, 0.0, 2.0], [0.0; 3])
        .unwrap();
    let (tx, _rx) = mpsc::channel(16);

    handle_interact(PLAYER, banker, &tx, &mut mgr).await;
    handle_interact(PLAYER, banker, &tx, &mut mgr).await;
    assert!(session(&mgr).is_some(), "re-clicking the Banker keeps it");

    handle_interact(PLAYER, other, &tx, &mut mgr).await;
    assert_eq!(session(&mgr), None, "another target ends the session");
    assert_eq!(allowed(&mgr), Err(VaultReject::NoSession));
}

/// Logout and a space change both destroy the player's cell entity, and
/// the session goes with it: the entity recreated in another space has
/// none, and a move is refused.
#[tokio::test]
async fn logout_and_space_change_end_the_session() {
    let mut mgr = two_space_manager();
    let banker = spawn_banker(&mut mgr, "Agnos", [2.0, 0.0, 0.0], VaultScope::Personal);
    let (tx, _rx) = mpsc::channel(64);
    handle_interact(PLAYER, banker, &tx, &mut mgr).await;
    assert!(session(&mgr).is_some());

    // Logout.
    mgr.disconnect_entity(PLAYER, &tx).await;
    assert!(mgr.get_entity(PLAYER).is_none());

    // Space change: the cross-world path destroys and recreates.
    mgr.create_entity(PLAYER, "Harset", [0.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    assert_eq!(session(&mgr), None);
    assert_eq!(allowed(&mgr), Err(VaultReject::NoSession));
}

/// `vault_move_allowed` on each rule: no session, in range, out of range,
/// Banker gone, Banker in another space, session from another space, and
/// the GM bypass. Each negative fails if its check is removed.
#[tokio::test]
async fn vault_move_allowed_enforces_session_space_and_proximity() {
    let mut mgr = two_space_manager();
    let banker = spawn_banker(&mut mgr, "Agnos", [2.0, 0.0, 0.0], VaultScope::Personal);
    let space = mgr.get_entity_space_id(PLAYER).unwrap();

    assert_eq!(allowed(&mgr), Err(VaultReject::NoSession));

    let open = |banker_id: Option<u32>, space_id: u32| VaultSession {
        scope: VaultScope::Personal,
        banker_id,
        space_id,
        opened_at: std::time::Instant::now(),
    };
    mgr.get_entity_mut(PLAYER).unwrap().vault_session = Some(open(Some(banker), space));
    assert_eq!(allowed(&mgr), Ok(()), "next to the Banker");

    // Walk away: 10 units from the Banker, past MAX_INTERACT_DISTANCE (5).
    mgr.update_entity_position(PLAYER, [12.0, 0.0, 0.0], [0; 3], [0.0; 3]);
    assert!(
        matches!(allowed(&mgr), Err(VaultReject::BankerOutOfRange { dist }) if (dist - 10.0).abs() < 0.01),
        "got {:?}",
        allowed(&mgr)
    );

    // GM session: no Banker, so no proximity check, even far away.
    mgr.get_entity_mut(PLAYER).unwrap().vault_session = Some(open(None, space));
    assert_eq!(allowed(&mgr), Ok(()), "a GM session skips proximity");

    // A session recorded in another space than the player's.
    mgr.get_entity_mut(PLAYER).unwrap().vault_session = Some(open(None, space + 1000));
    assert_eq!(allowed(&mgr), Err(VaultReject::SessionInOtherSpace));

    // The pinned Banker is in another space.
    let far = spawn_banker(&mut mgr, "Harset", [12.0, 0.0, 0.0], VaultScope::Personal);
    mgr.get_entity_mut(PLAYER).unwrap().vault_session = Some(open(Some(far), space));
    assert_eq!(allowed(&mgr), Err(VaultReject::BankerInOtherSpace));

    // The pinned Banker despawned.
    mgr.get_entity_mut(PLAYER).unwrap().vault_session = Some(open(Some(banker), space));
    mgr.destroy_entity(banker);
    assert_eq!(allowed(&mgr), Err(VaultReject::BankerGone));
}

/// GM `.bank`'s open: a Banker-less session and `onVaultOpen` addressed to
/// the GM's own entity and position.
#[tokio::test]
async fn gm_open_sends_vault_open_for_self_with_a_bankerless_session() {
    let mut mgr = two_space_manager();
    mgr.update_entity_position(PLAYER, [7.0, 1.0, -3.0], [0; 3], [0.0; 3]);
    let (tx, mut rx) = mpsc::channel(16);

    open_vault_gm(PLAYER, &tx, &mut mgr).await;

    assert_eq!(
        drain(&mut rx),
        vec![(
            PLAYER,
            ON_VAULT_OPEN,
            build_vault_open_args(PLAYER as i32, [7.0, 1.0, -3.0])
        )]
    );
    let s = session(&mgr).expect("GM session");
    assert_eq!(s.banker_id, None);
    assert_eq!(allowed(&mgr), Ok(()));
}

/// `vault_access`, the verdict BV-03 attaches to every forwarded inventory
/// request: each `VaultReject` passes through as its reason label, with the
/// Banker and (out of range) the distance; an open session carries the
/// Banker and its distance, a GM session neither. A missing player entity
/// is `player_missing`.
#[tokio::test]
async fn vault_access_maps_every_verdict() {
    use cimmeria_wire::cell::vault::VaultAccess;

    let mut mgr = two_space_manager();
    let banker = spawn_banker(&mut mgr, "Agnos", [2.0, 0.0, 0.0], VaultScope::Personal);
    let space = mgr.get_entity_space_id(PLAYER).unwrap();
    let open = |banker_id: Option<u32>, space_id: u32| VaultSession {
        scope: VaultScope::Personal,
        banker_id,
        space_id,
        opened_at: std::time::Instant::now(),
    };

    assert_eq!(vault_access(PLAYER, &mgr), VaultAccess::NO_SESSION);
    assert_eq!(
        vault_access(4242, &mgr).reason(),
        Some("player_missing"),
        "no entity"
    );

    mgr.get_entity_mut(PLAYER).unwrap().vault_session = Some(open(Some(banker), space));
    let near = vault_access(PLAYER, &mgr);
    assert!(near.is_open() && !near.gm_override(), "{near:?}");
    assert_eq!(near.banker_id(), Some(banker));
    assert!(near.distance().is_some_and(|d| (d - 2.0).abs() < 0.01));

    mgr.update_entity_position(PLAYER, [12.0, 0.0, 0.0], [0; 3], [0.0; 3]);
    let far = vault_access(PLAYER, &mgr);
    assert_eq!(far.reason(), Some("banker_out_of_range"));
    assert!(far.distance().is_some_and(|d| (d - 10.0).abs() < 0.01));

    mgr.get_entity_mut(PLAYER).unwrap().vault_session = Some(open(None, space));
    let gm = vault_access(PLAYER, &mgr);
    assert!(gm.is_open() && gm.gm_override(), "{gm:?}");
    assert_eq!((gm.banker_id(), gm.distance()), (None, None));

    mgr.get_entity_mut(PLAYER).unwrap().vault_session = Some(open(None, space + 1000));
    assert_eq!(
        vault_access(PLAYER, &mgr).reason(),
        Some("vault_session_other_space")
    );

    let elsewhere = spawn_banker(&mut mgr, "Harset", [12.0, 0.0, 0.0], VaultScope::Personal);
    mgr.get_entity_mut(PLAYER).unwrap().vault_session = Some(open(Some(elsewhere), space));
    assert_eq!(
        vault_access(PLAYER, &mgr).reason(),
        Some("banker_other_space")
    );

    mgr.get_entity_mut(PLAYER).unwrap().vault_session = Some(open(Some(banker), space));
    mgr.destroy_entity(banker);
    let gone = vault_access(PLAYER, &mgr);
    assert_eq!(gone.reason(), Some("banker_gone"));
    assert_eq!(gone.banker_id(), Some(banker));
}
