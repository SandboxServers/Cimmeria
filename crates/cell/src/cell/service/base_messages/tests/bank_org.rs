//! The Team and Command vault in the base-message loop (bank-vault BV-07):
//! the `Bank` arm reaches the grant, and ORG-06's `OrgMembershipEnded` ends
//! an open vault session of that organization with `reason = org_left`.
//! The grant's own checks are tested in `cimmeria-cell-interactions`
//! (`bank::org_open_tests`).

use std::time::Instant;

use cimmeria_entity::cell_entity::{NpcInteractionType, VaultScope, VaultSession};
use cimmeria_entity::organization::OrgLeaveReason;
use cimmeria_wire::cell::client_methods::player::ON_TEAM_VAULT_OPEN;

use super::*;
use crate::cell::messages::{BankBaseToCell, OrgBaseToCell};
use crate::test_support::{make_space_manager, LogCapture};

const ENTITY: u32 = 11;
const PLAYER_ID: i32 = 0x7000_B9D0;
const ORG: i32 = 0x7000_B9D1;

fn staged() -> (SpaceManager, u32) {
    let mut mgr = make_space_manager();
    mgr.create_entity(ENTITY, "Agnos", [0.0; 3], [0.0; 3])
        .unwrap();
    mgr.connect_entity(ENTITY);
    let p = mgr.get_entity_mut(ENTITY).unwrap();
    p.player_id = Some(PLAYER_ID);
    p.account_id = Some(7);
    let banker = mgr.allocate_npc_id();
    mgr.spawn_npc(banker, "Agnos", [2.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    let npc = mgr.get_entity_mut(banker).unwrap();
    npc.interaction_type = Some(NpcInteractionType::Banker {
        scope: VaultScope::Team,
    });
    mgr.get_entity_mut(ENTITY).unwrap().last_interaction_target = Some(banker);
    (mgr, banker)
}

fn open(mgr: &mut SpaceManager, scope: VaultScope, org_id: Option<i32>) {
    let space_id = mgr.get_entity_space_id(ENTITY).unwrap();
    mgr.get_entity_mut(ENTITY).unwrap().vault_session = Some(VaultSession {
        expansion_offer: None,
        scope,
        org_id,
        banker_id: Some(1),
        space_id,
        opened_at: Instant::now(),
    });
}

async fn ended(mgr: &mut SpaceManager, player_id: i32, org_id: i32) {
    ended_for(mgr, player_id, org_id, OrgLeaveReason::Requested).await;
}

async fn ended_for(mgr: &mut SpaceManager, player_id: i32, org_id: i32, reason: OrgLeaveReason) {
    let (tx, _rx) = mpsc::channel(64);
    let msg = OrgBaseToCell::OrgMembershipEnded {
        player_id,
        entity_id: ENTITY,
        org_id,
        reason,
    };
    handle_base_message(BaseToCellMsg::Org(msg), &tx, mgr, &ChainEngine::new(), &[]).await;
}

/// The `Bank` arm hands the grant to the cell's open: a session naming the
/// org, and `onTeamVaultOpen`. Fails if the arm is not routed.
#[tokio::test]
async fn the_bank_arm_opens_a_granted_org_vault() {
    let (mut mgr, banker) = staged();
    let (tx, mut rx) = mpsc::channel(64);
    let grant = BankBaseToCell::OrgVaultGranted {
        entity_id: ENTITY,
        player_id: PLAYER_ID,
        scope: VaultScope::Team,
        org_id: ORG,
        banker_id: banker,
    };
    handle_base_message(
        BaseToCellMsg::Bank(grant),
        &tx,
        &mut mgr,
        &ChainEngine::new(),
        &[],
    )
    .await;
    let session = mgr.get_entity(ENTITY).unwrap().vault_session.clone();
    assert_eq!(session.and_then(|s| s.org_id), Some(ORG));
    let mut methods = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        if let CellToBaseMsg::EntityMethodCall { method_index, .. } = msg {
            methods.push(method_index);
        }
    }
    assert_eq!(methods, [ON_TEAM_VAULT_OPEN]);
}

/// Leaving the organization ends its vault session with
/// `vault_session_closed reason=org_left`. Fails if `membership_ended`
/// stops ending it.
#[tokio::test]
async fn leaving_the_org_closes_its_vault_session_with_org_left() {
    let (mut mgr, _) = staged();
    open(&mut mgr, VaultScope::Team, Some(ORG));
    let capture = LogCapture::install();

    ended(&mut mgr, PLAYER_ID, ORG).await;

    assert_eq!(mgr.get_entity(ENTITY).unwrap().vault_session, None);
    let rows: Vec<_> = capture
        .all()
        .into_iter()
        .filter(|c| c.target == "bank" && c.has_field("event", "vault_session_closed"))
        .collect();
    assert_eq!(rows.len(), 1, "{:#?}", capture.all());
    let row = &rows[0];
    assert_eq!(row.level, tracing::Level::DEBUG);
    for (k, v) in [
        ("reason", "org_left"),
        ("scope", "team"),
        ("org_id", &ORG.to_string()[..]),
        ("player_id", &PLAYER_ID.to_string()[..]),
        ("account_id", "7"),
        ("entity_id", "11"),
    ] {
        assert!(row.has_field(k, v), "{k}={v}: {row:#?}");
    }
}

/// Only that organization's session ends: the personal vault, another
/// organization's vault, and a message about a stale entity (another
/// character now) leave the session alone.
#[tokio::test]
async fn membership_ended_leaves_other_sessions_alone() {
    let (mut mgr, _) = staged();
    open(&mut mgr, VaultScope::Personal, None);
    ended(&mut mgr, PLAYER_ID, ORG).await;
    assert!(
        mgr.get_entity(ENTITY).unwrap().vault_session.is_some(),
        "personal"
    );

    open(&mut mgr, VaultScope::Command, Some(ORG + 1));
    ended(&mut mgr, PLAYER_ID, ORG).await;
    assert!(
        mgr.get_entity(ENTITY).unwrap().vault_session.is_some(),
        "another org"
    );

    open(&mut mgr, VaultScope::Team, Some(ORG));
    ended(&mut mgr, PLAYER_ID + 1, ORG).await;
    assert!(
        mgr.get_entity(ENTITY).unwrap().vault_session.is_some(),
        "stale entity"
    );
}

/// Every way a membership ends closes the org's vault session: a leave, a
/// kick (ORG-07 sends `OrgMembershipEnded` with `Kicked`) and a disband.
/// Fails if the close keys on the reason.
#[tokio::test]
async fn a_kick_or_a_disband_closes_the_vault_session_too() {
    for reason in [OrgLeaveReason::Kicked, OrgLeaveReason::Disbanded] {
        let (mut mgr, _) = staged();
        open(&mut mgr, VaultScope::Team, Some(ORG));
        ended_for(&mut mgr, PLAYER_ID, ORG, reason).await;
        assert_eq!(
            mgr.get_entity(ENTITY).unwrap().vault_session,
            None,
            "{reason:?}"
        );
    }
}
