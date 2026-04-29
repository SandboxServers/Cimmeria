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

    // Bound count against remaining bytes BEFORE allocating: a malformed packet
    // with count=u32::MAX would otherwise reserve ~32 GiB up-front and OOM.
    let remaining = args.len().saturating_sub(*offset);
    if count.saturating_mul(8) > remaining {
        return None;
    }

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

fn read_trailing_template_id(args: &[u8], offset: usize) -> Option<i32> {
    if args.len() >= offset + 4 {
        Some(i32::from_le_bytes([
            args[offset], args[offset + 1], args[offset + 2], args[offset + 3],
        ]))
    } else {
        None
    }
}

/// Resolved vendor session for the player, looked up from server-side state.
///
/// `vendor_template_id` is read from the vendor entity itself rather than trusted
/// from the client's request — this prevents a client from opening vendor A
/// then submitting a purchase against vendor B's item lists.
struct VendorSession {
    player_id: i32,
    vendor_entity_id: i32,
    /// Server-side authoritative template id for the currently-open vendor.
    /// May be `None` if the vendor entity has no template attached.
    server_template_id: Option<i32>,
}

fn vendor_context(entity_id: u32, space_mgr: &SpaceManager) -> Option<VendorSession> {
    let player = space_mgr.get_entity(entity_id)?;
    let player_id = player.player_id?;
    let vendor_entity_id_u32 = player.vendor_entity?;
    let vendor_entity_id = vendor_entity_id_u32 as i32;
    let server_template_id = space_mgr
        .get_entity(vendor_entity_id_u32)
        .and_then(|v| v.template_id);
    Some(VendorSession { player_id, vendor_entity_id, server_template_id })
}

/// Validate that the client-supplied template id matches the vendor that was
/// opened. Returns the authoritative server-side id on success, or `None` after
/// logging the mismatch.
fn validate_template_id(
    entity_id: u32,
    op: &str,
    session: &VendorSession,
    client_template_id: i32,
) -> Option<i32> {
    match session.server_template_id {
        Some(server_id) if server_id == client_template_id => Some(server_id),
        Some(server_id) => {
            tracing::warn!(
                entity_id,
                op,
                server_template_id = server_id,
                client_template_id,
                vendor_entity_id = session.vendor_entity_id,
                "vendor op rejected: client supplied template id does not match opened vendor"
            );
            None
        }
        None => {
            tracing::warn!(
                entity_id,
                op,
                client_template_id,
                vendor_entity_id = session.vendor_entity_id,
                "vendor op rejected: opened vendor has no template id (server cannot validate)"
            );
            None
        }
    }
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

        PURCHASE_ITEMS | SELL_ITEMS | BUYBACK_ITEMS | REPAIR_ITEMS | RECHARGE_ITEMS => {
            let op_name = match method_index {
                PURCHASE_ITEMS => "purchaseItems",
                SELL_ITEMS => "sellItems",
                BUYBACK_ITEMS => "buybackItems",
                REPAIR_ITEMS => "repairItems",
                RECHARGE_ITEMS => "rechargeItems",
                _ => unreachable!(),
            };

            let session = match vendor_context(entity_id, space_mgr) {
                Some(s) => s,
                None => {
                    tracing::warn!(entity_id, op = op_name, "vendor op: no active vendor context (player_id or vendor_entity unset)");
                    return true;
                }
            };

            let mut offset = 0;
            let items = match read_i32_array(args, &mut offset) {
                Some(items) => items,
                None => {
                    tracing::warn!(entity_id, op = op_name, args_len = args.len(), "vendor op: malformed item array in args");
                    return true;
                }
            };

            let trailing_template_id = read_trailing_template_id(args, offset);

            // For paid Repair/Recharge, `trailing_template_id` is optional (None
            // signals "free repair"). For Purchase/Sell/Buyback, it's required;
            // and in all cases where the client supplied one, it must match the
            // vendor that was actually opened so a client can't spoof it.
            let validated_template_id = match trailing_template_id {
                Some(client_id) => match validate_template_id(entity_id, op_name, &session, client_id) {
                    Some(server_id) => Some(server_id),
                    None => return true,
                },
                None => None,
            };

            let msg = match method_index {
                PURCHASE_ITEMS => match validated_template_id {
                    Some(vendor_template_id) => CellToBaseMsg::PurchaseVendorItems {
                        entity_id, player_id: session.player_id,
                        vendor_entity_id: session.vendor_entity_id,
                        vendor_template_id, items,
                    },
                    None => {
                        tracing::warn!(entity_id, op = op_name, "vendor op: missing vendor_template_id");
                        return true;
                    }
                },
                SELL_ITEMS => match validated_template_id {
                    Some(vendor_template_id) => CellToBaseMsg::SellVendorItems {
                        entity_id, player_id: session.player_id,
                        vendor_entity_id: session.vendor_entity_id,
                        vendor_template_id, items,
                    },
                    None => {
                        tracing::warn!(entity_id, op = op_name, "vendor op: missing vendor_template_id");
                        return true;
                    }
                },
                BUYBACK_ITEMS => match validated_template_id {
                    Some(vendor_template_id) => CellToBaseMsg::BuybackVendorItems {
                        entity_id, player_id: session.player_id,
                        vendor_entity_id: session.vendor_entity_id,
                        vendor_template_id, items,
                    },
                    None => {
                        tracing::warn!(entity_id, op = op_name, "vendor op: missing vendor_template_id");
                        return true;
                    }
                },
                REPAIR_ITEMS => CellToBaseMsg::RepairInventoryItems {
                    entity_id,
                    player_id: session.player_id,
                    item_ids: items.iter().map(|(id, _)| *id).collect(),
                    vendor_template_id: validated_template_id,
                },
                RECHARGE_ITEMS => CellToBaseMsg::RechargeInventoryItems {
                    entity_id,
                    player_id: session.player_id,
                    item_ids: items.iter().map(|(id, _)| *id).collect(),
                    vendor_template_id: validated_template_id,
                },
                _ => unreachable!(),
            };

            if let Err(e) = tx.send(msg).await {
                tracing::warn!(entity_id, op = op_name, "vendor op: cell->base channel closed: {e}");
            }
            true
        }

        _ => false,
    }
}
