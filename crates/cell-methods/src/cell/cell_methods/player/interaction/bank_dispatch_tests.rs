//! BV-02 smoke: a right-click on a Banker, driven through the player
//! dispatcher from the wire's method index, on an NPC spawned from a
//! template record, so the `INT_BANKER` + `vault_scope` derivation, the
//! outer range gate, the pin and the Banker arm all run as in a live cell.

use tokio::sync::mpsc;

use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_entity::cell_entity::VaultScope;
use cimmeria_entity::interaction_flags::{INT_BANKER, INT_VENDOR_GENERAL};

use crate::cell::cell_methods::player::{dispatch, INTERACT};
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use crate::test_support::{make_space_manager, npc_spawn_record};

const PLAYER: u32 = 1;
const ON_PLAYER_COMMUNICATION: u16 = 28;
const ON_VAULT_OPEN: u16 = 106;

/// The player at the origin and a Banker template NPC at `banker_at`.
fn stage(banker_at: [f32; 3], bits: i64, scope: VaultScope) -> (SpaceManager, u32) {
    let mut mgr = make_space_manager();
    mgr.create_entity(PLAYER, "Agnos", [0.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    mgr.connect_entity(PLAYER);
    let banker = mgr.allocate_npc_id();
    mgr.spawn_npc_from_record(banker, &npc_spawn_record("Agnos", banker_at, bits, scope))
        .unwrap();
    (mgr, banker)
}

/// Right-click `npc` through the wire dispatcher; every `EntityMethodCall`
/// it produced, as `(recipient, method, args)`.
async fn click(mgr: &mut SpaceManager, npc: u32) -> Vec<(u32, u16, Vec<u8>)> {
    let (tx, mut rx) = mpsc::channel(64);
    assert!(
        dispatch(
            PLAYER,
            INTERACT,
            &(npc as i32).to_le_bytes(),
            &tx,
            mgr,
            &ChainEngine::new()
        )
        .await
    );
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

/// In range: exactly one 106, with the Banker's id and position, and a
/// session pinned to it. Also covers the precedence rule: the template
/// carries a vendor bit too, and the Banker still answers. Fails if the
/// spawn derivation or the Banker arm is removed.
#[tokio::test]
async fn banker_click_in_range_sends_one_vault_open_and_sets_the_session() {
    let (mut mgr, banker) = stage(
        [3.0, 0.0, 0.0],
        INT_BANKER | INT_VENDOR_GENERAL,
        VaultScope::Personal,
    );

    let sent = click(&mut mgr, banker).await;

    let opens: Vec<_> = sent
        .iter()
        .filter(|(_, m, _)| *m == ON_VAULT_OPEN)
        .collect();
    assert_eq!(opens.len(), 1, "exactly one onVaultOpen: {sent:?}");
    let (to, _, args) = opens[0];
    assert_eq!(*to, PLAYER);
    assert_eq!(
        args,
        &cimmeria_wire::cell::vault::build_vault_open_args(banker as i32, [3.0, 0.0, 0.0])
    );
    let session = mgr.get_entity(PLAYER).unwrap().vault_session.clone();
    assert_eq!(session.and_then(|s| s.banker_id), Some(banker));
}

/// Out of range: nothing is sent and no session opens. Fails if the
/// session were set before the range gate.
#[tokio::test]
async fn banker_click_out_of_range_sends_nothing_and_sets_no_session() {
    let (mut mgr, banker) = stage([30.0, 0.0, 0.0], INT_BANKER, VaultScope::Personal);

    let sent = click(&mut mgr, banker).await;

    assert!(sent.is_empty(), "nothing for a click from 30 u: {sent:?}");
    assert!(mgr.get_entity(PLAYER).unwrap().vault_session.is_none());
}

/// A Team-scope Banker is refused with a visible line and no session.
#[tokio::test]
async fn team_banker_click_is_refused_with_feedback() {
    let (mut mgr, banker) = stage([3.0, 0.0, 0.0], INT_BANKER, VaultScope::Team);

    let sent = click(&mut mgr, banker).await;

    let methods: Vec<u16> = sent.iter().map(|(_, m, _)| *m).collect();
    assert_eq!(methods, vec![ON_PLAYER_COMMUNICATION], "{sent:?}");
    assert!(mgr.get_entity(PLAYER).unwrap().vault_session.is_none());
}
