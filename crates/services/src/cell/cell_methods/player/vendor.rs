use tokio::sync::mpsc;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

use super::constants::*;

fn read_i32_array(args: &[u8], offset: &mut usize) -> Option<Vec<(i32, i32)>> {
    if args.len() < *offset + 4 {
        return None;
    }
    let count = u32::from_le_bytes([
        args[*offset],
        args[*offset + 1],
        args[*offset + 2],
        args[*offset + 3],
    ]) as usize;
    *offset += 4;

    let mut items = Vec::with_capacity(count);
    for _ in 0..count {
        if args.len() < *offset + 8 {
            return None;
        }
        let item_id = i32::from_le_bytes([
            args[*offset],
            args[*offset + 1],
            args[*offset + 2],
            args[*offset + 3],
        ]);
        let quantity = i32::from_le_bytes([
            args[*offset + 4],
            args[*offset + 5],
            args[*offset + 6],
            args[*offset + 7],
        ]);
        *offset += 8;
        items.push((item_id, quantity));
    }
    Some(items)
}

fn vendor_context(entity_id: u32, space_mgr: &SpaceManager) -> Option<(i32, i32)> {
    let player = space_mgr.get_entity(entity_id)?;
    let player_id = player.player_id?;
    let vendor_entity_id = player.vendor_entity.map(|v| v as i32)?;
    Some((player_id, vendor_entity_id))
}

pub async fn dispatch(
    entity_id: u32,
    method_index: u16,
    args: &[u8],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> bool {
    match method_index {
        TRAIN_ABILITY => {
            if args.len() >= 4 {
                let ability_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                tracing::debug!(entity_id, ability_id, "trainAbility (not yet implemented)");
            }
            true
        }

        PURCHASE_ITEMS => {
            if let Some((player_id, vendor_entity_id)) = vendor_context(entity_id, space_mgr) {
                let mut offset = 0;
                if let Some(items) = read_i32_array(args, &mut offset) {
                    if args.len() >= offset + 4 {
                        let vendor_template_id = i32::from_le_bytes([
                            args[offset], args[offset + 1], args[offset + 2], args[offset + 3],
                        ]);
                        let _ = tx.send(CellToBaseMsg::PurchaseVendorItems {
                            entity_id,
                            player_id,
                            vendor_entity_id,
                            vendor_template_id,
                            items,
                        }).await;
                        return true;
                    }
                }
            }
            tracing::warn!(entity_id, "purchaseItems: failed to parse arguments");
            true
        }

        SELL_ITEMS => {
            if let Some((player_id, vendor_entity_id)) = vendor_context(entity_id, space_mgr) {
                let mut offset = 0;
                if let Some(items) = read_i32_array(args, &mut offset) {
                    if args.len() >= offset + 4 {
                        let vendor_template_id = i32::from_le_bytes([
                            args[offset], args[offset + 1], args[offset + 2], args[offset + 3],
                        ]);
                        let _ = tx.send(CellToBaseMsg::SellVendorItems {
                            entity_id,
                            player_id,
                            vendor_entity_id,
                            vendor_template_id,
                            items,
                        }).await;
                        return true;
                    }
                }
            }
            tracing::warn!(entity_id, "sellItems: failed to parse arguments");
            true
        }

        BUYBACK_ITEMS => {
            if let Some((player_id, vendor_entity_id)) = vendor_context(entity_id, space_mgr) {
                let mut offset = 0;
                if let Some(items) = read_i32_array(args, &mut offset) {
                    if args.len() >= offset + 4 {
                        let vendor_template_id = i32::from_le_bytes([
                            args[offset], args[offset + 1], args[offset + 2], args[offset + 3],
                        ]);
                        let _ = tx.send(CellToBaseMsg::BuybackVendorItems {
                            entity_id,
                            player_id,
                            vendor_entity_id,
                            vendor_template_id,
                            items,
                        }).await;
                        return true;
                    }
                }
            }
            tracing::warn!(entity_id, "buybackItems: failed to parse arguments");
            true
        }

        REPAIR_ITEMS => {
            if let Some((player_id, vendor_entity_id)) = vendor_context(entity_id, space_mgr) {
                let mut offset = 0;
                if let Some(item_ids) = read_i32_array(args, &mut offset) {
                    let vendor_template_id = if args.len() >= offset + 4 {
                        Some(i32::from_le_bytes([
                            args[offset], args[offset + 1], args[offset + 2], args[offset + 3],
                        ]))
                    } else {
                        None
                    };
                    let item_ids_only: Vec<i32> = item_ids.iter().map(|(id, _)| *id).collect();
                    let _ = tx.send(CellToBaseMsg::RepairInventoryItems {
                        entity_id,
                        player_id,
                        item_ids: item_ids_only,
                        vendor_template_id,
                    }).await;
                    return true;
                }
            }
            tracing::warn!(entity_id, "repairItems: failed to parse arguments");
            true
        }

        RECHARGE_ITEMS => {
            if let Some((player_id, vendor_entity_id)) = vendor_context(entity_id, space_mgr) {
                let mut offset = 0;
                if let Some(item_ids) = read_i32_array(args, &mut offset) {
                    let vendor_template_id = if args.len() >= offset + 4 {
                        Some(i32::from_le_bytes([
                            args[offset], args[offset + 1], args[offset + 2], args[offset + 3],
                        ]))
                    } else {
                        None
                    };
                    let item_ids_only: Vec<i32> = item_ids.iter().map(|(id, _)| *id).collect();
                    let _ = tx.send(CellToBaseMsg::RechargeInventoryItems {
                        entity_id,
                        player_id,
                        item_ids: item_ids_only,
                        vendor_template_id,
                    }).await;
                    return true;
                }
            }
            tracing::warn!(entity_id, "rechargeItems: failed to parse arguments");
            true
        }

        _ => false,
    }
}
