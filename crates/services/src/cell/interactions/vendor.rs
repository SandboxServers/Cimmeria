//! Vendor open — `OpenVendorStore` cell→base request.

use tokio::sync::mpsc;

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

/// Open a vendor store for a player by requesting store data from base.
///
/// Sets the vendor_entity on the player entity and sends OpenVendorStore
/// to BaseApp, which will load the full vendor store data and send it to the client.
pub(super) async fn send_store_open(
    player_id: u32,
    vendor_entity_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    // Store vendor entity ID on player for reference during vendor operations
    if let Some(player) = space_mgr.get_entity_mut(player_id) {
        player.vendor_entity = Some(vendor_entity_id);
    }

    // Get vendor template ID from the vendor entity
    let vendor_template_id = space_mgr.get_entity(vendor_entity_id)
        .and_then(|e| e.template_id);

    // Get player_id for base app message
    let player_db_id = match space_mgr.get_entity(player_id).and_then(|e| e.player_id) {
        Some(id) => id,
        None => {
            tracing::warn!(player_id, vendor_entity_id, "send_store_open: missing player_id; aborting vendor open");
            return;
        }
    };

    let vendor_entity_id_i32 = match i32::try_from(vendor_entity_id) {
        Ok(v) => v,
        Err(_) => {
            tracing::warn!(player_id, vendor_entity_id, "send_store_open: vendor entity id exceeds i32; aborting");
            return;
        }
    };

    tracing::info!(player_id, vendor_entity_id, ?vendor_template_id, "Opening vendor store");
    let _ = tx.send(CellToBaseMsg::OpenVendorStore {
        entity_id: player_id,
        player_id: player_db_id,
        vendor_entity_id: vendor_entity_id_i32,
        vendor_template_id,
    }).await;
}
