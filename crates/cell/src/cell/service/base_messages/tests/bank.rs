//! `BaseToCellMsg::Bank` routing (BV-05): the base's expansion offer
//! reaches `interactions::offer_vault_expansion`, which records it on the
//! vault session and shows the Expand dialog (`onDialogDisplay` [105]).

use super::*;
use crate::cell::messages::BankBaseToCell;
use cimmeria_entity::cell_entity::{VaultScope, VaultSession};

#[tokio::test]
async fn an_expansion_offer_reaches_the_vault_session() {
    let mut mgr = crate::test_support::make_space_manager();
    mgr.create_entity(10, "Agnos", [0.0; 3], [0.0; 3]).unwrap();
    mgr.connect_entity(10);
    let space_id = mgr.get_entity_space_id(10).unwrap();
    let p = mgr.get_entity_mut(10).unwrap();
    p.player_id = Some(1000);
    // A GM `.bank` session: the dialog speaks through the player's entity.
    p.vault_session = Some(VaultSession {
        scope: VaultScope::Personal,
        banker_id: None,
        space_id,
        opened_at: std::time::Instant::now(),
        expansion_offer: None,
    });
    let (tx, mut rx) = mpsc::channel(8);
    let engine = ChainEngine::new();
    let msg = BaseToCellMsg::Bank(BankBaseToCell::OfferExpansion {
        entity_id: 10,
        player_id: 1000,
        speaker_id: 10,
        from_slots: 60,
        price: 100,
    });

    handle_base_message(msg, &tx, &mut mgr, &engine, &[]).await;

    let session = mgr.get_entity(10).unwrap().vault_session.clone().unwrap();
    assert_eq!(
        session.expansion_offer,
        Some(cimmeria_entity::cell_entity::ExpansionOffer {
            from_slots: 60,
            price: 100
        })
    );
    match rx.try_recv() {
        Ok(CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index,
            ..
        }) => assert_eq!((entity_id, method_index), (10, 105)),
        _ => panic!("expected onDialogDisplay first"),
    }
}
