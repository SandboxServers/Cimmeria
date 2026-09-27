//! BV-02 smoke: a right-click on a Banker, driven through the player
//! dispatcher from the wire's method index, on an NPC spawned from a
//! template record, so the `INT_BANKER` + `vault_scope` derivation, the
//! outer range gate, the pin and the Banker arm all run as in a live cell.

use tokio::sync::mpsc;

use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_entity::cell_entity::VaultScope;
use cimmeria_entity::interaction_flags::{INT_BANKER, INT_VENDOR_GENERAL};

use crate::cell::cell_methods::player::{dispatch, INTERACT};
use crate::cell::messages::{BankCellToBase, CellToBaseMsg};
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

/// Out of range: no `onVaultOpen` and no session, only the refusal line.
/// Fails if the session were set before the range gate, or if the
/// refusal line is dropped.
#[tokio::test]
async fn banker_click_out_of_range_opens_nothing_and_sets_no_session() {
    let (mut mgr, banker) = stage([30.0, 0.0, 0.0], INT_BANKER, VaultScope::Personal);

    let sent = click(&mut mgr, banker).await;

    let methods: Vec<u16> = sent.iter().map(|(_, m, _)| *m).collect();
    assert_eq!(
        methods,
        vec![ON_PLAYER_COMMUNICATION],
        "only the refusal line for a click from 30 u: {sent:?}"
    );
    assert!(mgr.get_entity(PLAYER).unwrap().vault_session.is_none());
}

/// A Team-scope Banker, through the wire dispatcher, asks the base for the
/// Team vault (bank-vault BV-07): one `BankCellToBase::OrgVaultOpen`, no
/// client method yet, and no session until the base grants it.
#[tokio::test]
async fn team_banker_click_asks_the_base_for_the_team_vault() {
    let (mut mgr, banker) = stage([3.0, 0.0, 0.0], INT_BANKER, VaultScope::Team);
    let (tx, mut rx) = mpsc::channel(64);
    assert!(
        dispatch(
            PLAYER,
            INTERACT,
            &(banker as i32).to_le_bytes(),
            &tx,
            &mut mgr,
            &ChainEngine::new()
        )
        .await
    );
    let mut sent = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        sent.push(msg);
    }
    assert!(
        matches!(
            sent.as_slice(),
            [CellToBaseMsg::Bank(BankCellToBase::OrgVaultOpen {
                scope: VaultScope::Team,
                banker_id,
                ..
            })] if *banker_id == banker
        ),
        "one org vault request and nothing else: {sent:?}"
    );
    assert!(mgr.get_entity(PLAYER).unwrap().vault_session.is_none());
}

/// Every bank message the dispatcher sent.
async fn choose(
    mgr: &mut SpaceManager,
    dialog_id: i32,
    button_id: i32,
) -> Vec<crate::cell::messages::BankCellToBase> {
    let (tx, mut rx) = mpsc::channel(64);
    let mut args = dialog_id.to_le_bytes().to_vec();
    args.extend_from_slice(&button_id.to_le_bytes());
    assert!(
        dispatch(
            PLAYER,
            crate::cell::cell_methods::player::DIALOG_BUTTON_CHOICE,
            &args,
            &tx,
            mgr,
            &ChainEngine::new()
        )
        .await
    );
    let mut out = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        if let CellToBaseMsg::Bank(b) = msg {
            out.push(b);
        }
    }
    out
}

/// BV-05 smoke through the wire dispatcher: after a Banker click and the
/// base's offer, `dialogButtonChoice(60110, 8)` reaches the purchase path
/// with the offered size and an open verdict, once. A forged choice for the
/// Expand dialog that was never shown is stopped by the #479 gate before
/// the purchase path. Fails if the dialog id is not routed (the choice
/// would go to the content engine and send no `Expand`).
#[tokio::test]
async fn the_expand_dialog_answer_reaches_the_purchase_path_once() {
    use crate::cell::messages::BankCellToBase;
    use cimmeria_wire::cell::vault::VAULT_EXPAND_DIALOG_ID;

    let (mut mgr, banker) = stage([3.0, 0.0, 0.0], INT_BANKER, VaultScope::Personal);
    mgr.get_entity_mut(PLAYER).unwrap().player_id = Some(42);

    // Not offered yet: the gate drops it.
    assert!(choose(&mut mgr, VAULT_EXPAND_DIALOG_ID, 8).await.is_empty());

    click(&mut mgr, banker).await;
    let (tx, _rx) = mpsc::channel(64);
    crate::cell::interactions::offer_vault_expansion(PLAYER, 42, banker, 40, 100, &tx, &mut mgr)
        .await;
    // The dialog is quarantined (#943), so the offer only records; show it
    // as a served dialog would, which is what records the #479 offer.
    let shown = cimmeria_entity::cell_entity::ExpansionOffer {
        from_slots: 40,
        price: 100,
    };
    crate::cell::interactions::show_expand_offer(PLAYER, banker, shown, &tx, &mut mgr).await;

    let sent = choose(&mut mgr, VAULT_EXPAND_DIALOG_ID, 8).await;
    let [BankCellToBase::Expand {
        player_id,
        offer,
        vault,
        ..
    }] = sent.as_slice()
    else {
        panic!("one Expand: {sent:?}");
    };
    assert_eq!(*player_id, 42);
    assert_eq!(offer.map(|o| (o.from_slots, o.price)), Some((40, 100)));
    assert!(vault.opens_personal_vault(), "{vault:?}");
    assert_eq!(vault.banker_id(), Some(banker));

    // The offer was one-shot at the gate too: a replayed choice is dropped.
    assert!(choose(&mut mgr, VAULT_EXPAND_DIALOG_ID, 8).await.is_empty());
}
