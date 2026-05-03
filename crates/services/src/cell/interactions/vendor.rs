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
    let vendor_template_id = space_mgr
        .get_entity(vendor_entity_id)
        .and_then(|e| e.template_id);

    // Get player_id for base app message
    let player_db_id = match space_mgr.get_entity(player_id).and_then(|e| e.player_id) {
        Some(id) => id,
        None => {
            tracing::warn!(
                player_id,
                vendor_entity_id,
                "send_store_open: missing player_id; aborting vendor open"
            );
            return;
        }
    };

    let vendor_entity_id_i32 = match i32::try_from(vendor_entity_id) {
        Ok(v) => v,
        Err(_) => {
            tracing::warn!(
                player_id,
                vendor_entity_id,
                "send_store_open: vendor entity id exceeds i32; aborting"
            );
            return;
        }
    };

    tracing::info!(
        player_id,
        vendor_entity_id,
        ?vendor_template_id,
        "Opening vendor store"
    );
    let _ = tx
        .send(CellToBaseMsg::OpenVendorStore {
            entity_id: player_id,
            player_id: player_db_id,
            vendor_entity_id: vendor_entity_id_i32,
            vendor_template_id,
        })
        .await;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_space_manager() -> SpaceManager {
        let mut mgr = SpaceManager::new(1);
        let spaces_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<Spaces><Space WorldName="Agnos" Instanced="false" MinX="-2400" MaxX="2200" MinY="-3200" MaxY="2800" /></Spaces>"#;
        let cell_spaces_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<Spaces><Space WorldName="Agnos" /></Spaces>"#;
        mgr.parse_spaces_xml(spaces_xml).unwrap();
        mgr.create_startup_spaces(cell_spaces_xml).unwrap();
        mgr
    }

    fn collect_open_vendor_messages(
        rx: &mut mpsc::Receiver<CellToBaseMsg>,
    ) -> Vec<(u32, i32, i32, Option<i32>)> {
        let mut out = Vec::new();
        while let Ok(msg) = rx.try_recv() {
            if let CellToBaseMsg::OpenVendorStore {
                entity_id,
                player_id,
                vendor_entity_id,
                vendor_template_id,
            } = msg
            {
                out.push((entity_id, player_id, vendor_entity_id, vendor_template_id));
            }
        }
        out
    }

    #[tokio::test]
    async fn happy_path_sends_open_vendor_store_with_template_id() {
        let mut mgr = make_space_manager();
        mgr.create_entity(1, "Agnos", [0.0, 0.0, 0.0], [0.0; 3])
            .unwrap();
        if let Some(p) = mgr.get_entity_mut(1) {
            p.player_id = Some(4242);
        }
        let vendor_entity_id = mgr.allocate_npc_id();
        mgr.spawn_npc(vendor_entity_id, "Agnos", [2.0, 0.0, 0.0], [0.0; 3])
            .unwrap();
        if let Some(v) = mgr.get_entity_mut(vendor_entity_id) {
            v.template_id = Some(9001);
        }

        let (tx, mut rx) = mpsc::channel(8);
        send_store_open(1, vendor_entity_id, &tx, &mut mgr).await;

        let msgs = collect_open_vendor_messages(&mut rx);
        assert_eq!(
            msgs.len(),
            1,
            "happy path must send exactly one OpenVendorStore"
        );
        let (entity_id, player_db_id, vendor_id_i32, template_id) = msgs[0];
        assert_eq!(entity_id, 1);
        assert_eq!(player_db_id, 4242);
        assert_eq!(vendor_id_i32, vendor_entity_id as i32);
        assert_eq!(template_id, Some(9001));

        // Side-effect: player.vendor_entity is set so subsequent vendor ops
        // can resolve the open vendor session via vendor_context.
        assert_eq!(
            mgr.get_entity(1).and_then(|p| p.vendor_entity),
            Some(vendor_entity_id),
        );
    }

    #[tokio::test]
    async fn does_not_send_when_player_id_missing() {
        let mut mgr = make_space_manager();
        mgr.create_entity(1, "Agnos", [0.0, 0.0, 0.0], [0.0; 3])
            .unwrap();
        // Note: deliberately NOT setting player.player_id — that's the regression.
        let vendor_entity_id = mgr.allocate_npc_id();
        mgr.spawn_npc(vendor_entity_id, "Agnos", [2.0, 0.0, 0.0], [0.0; 3])
            .unwrap();
        if let Some(v) = mgr.get_entity_mut(vendor_entity_id) {
            v.template_id = Some(9001);
        }

        let (tx, mut rx) = mpsc::channel(8);
        send_store_open(1, vendor_entity_id, &tx, &mut mgr).await;

        // Without the missing-player_id guard, this path would send
        // OpenVendorStore { player_id: 0, .. }, which then routes to whatever
        // character row has id 0 and corrupts that account's vendor session.
        // The function must return early before reaching the send.
        assert!(
            collect_open_vendor_messages(&mut rx).is_empty(),
            "must not send OpenVendorStore when player has no player_id",
        );
    }

    #[tokio::test]
    async fn does_not_send_when_vendor_entity_id_exceeds_i32_max() {
        let mut mgr = make_space_manager();
        mgr.create_entity(1, "Agnos", [0.0, 0.0, 0.0], [0.0; 3])
            .unwrap();
        if let Some(p) = mgr.get_entity_mut(1) {
            p.player_id = Some(4242);
        }
        // Out-of-range u32 id. We don't need a vendor entity to actually live
        // at this id — the i32::try_from check fails before the entity lookup
        // is meaningful, and the function must bail rather than wrap into a
        // negative i32.
        let oversized: u32 = (i32::MAX as u32) + 1;

        let (tx, mut rx) = mpsc::channel(8);
        send_store_open(1, oversized, &tx, &mut mgr).await;

        assert!(
            collect_open_vendor_messages(&mut rx).is_empty(),
            "must not send OpenVendorStore for vendor_entity_id > i32::MAX",
        );
    }

    #[tokio::test]
    async fn happy_path_with_vendor_lacking_template_id_sends_none() {
        let mut mgr = make_space_manager();
        mgr.create_entity(1, "Agnos", [0.0, 0.0, 0.0], [0.0; 3])
            .unwrap();
        if let Some(p) = mgr.get_entity_mut(1) {
            p.player_id = Some(4242);
        }
        let vendor_entity_id = mgr.allocate_npc_id();
        mgr.spawn_npc(vendor_entity_id, "Agnos", [2.0, 0.0, 0.0], [0.0; 3])
            .unwrap();
        // template_id deliberately left None.

        let (tx, mut rx) = mpsc::channel(8);
        send_store_open(1, vendor_entity_id, &tx, &mut mgr).await;

        let msgs = collect_open_vendor_messages(&mut rx);
        assert_eq!(msgs.len(), 1);
        assert_eq!(
            msgs[0].3, None,
            "missing vendor template_id must surface as None, not 0"
        );
    }
}
