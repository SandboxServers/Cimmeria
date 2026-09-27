//! `BaseToCellMsg::Bank` handler: bank and vault traffic from the base.
//!
//! The vault-expansion offer (BV-05) and the Team and Command vault grant
//! (BV-07). Later bank packets add their arms here rather than to `mod.rs`.

use tokio::sync::mpsc;

use super::super::super::interactions::{grant_org_vault, offer_vault_expansion};
use super::super::super::messages::{BankBaseToCell, CellToBaseMsg};
use super::super::super::space_manager::SpaceManager;

/// Handle one bank message from the base.
pub(super) async fn handle(
    msg: BankBaseToCell,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    match msg {
        BankBaseToCell::OfferExpansion {
            entity_id,
            player_id,
            speaker_id,
            from_slots,
            price,
        } => {
            offer_vault_expansion(
                entity_id, player_id, speaker_id, from_slots, price, tx, space_mgr,
            )
            .await;
        }
        BankBaseToCell::OrgVaultGranted {
            entity_id,
            player_id,
            scope,
            org_id,
            banker_id,
        } => {
            grant_org_vault(
                entity_id, player_id, scope, org_id, banker_id, tx, space_mgr,
            )
            .await
        }
    }
}
