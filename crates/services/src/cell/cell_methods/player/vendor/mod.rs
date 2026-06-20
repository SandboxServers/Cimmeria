//! SGWPlayer vendor cell methods: ability training plus the
//! purchase / sell / buyback / repair / recharge op group. Dispatch lives
//! here; the wire decoders, session validation, and training validation
//! live in the sibling submodules.

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use tokio::sync::mpsc;

use super::constants::*;

mod session;
mod train;
mod wire;

use session::{validate_template_id, vendor_context};
use train::handle_train_ability;
use wire::{read_i32_array, read_trailing_template_id};

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
                handle_train_ability(entity_id, ability_id, tx, space_mgr).await;
            } else {
                tracing::warn!(
                    entity_id,
                    args_len = args.len(),
                    "trainAbility: truncated args (need 4 bytes ability_id)"
                );
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
                    tracing::warn!(
                        entity_id,
                        op = op_name,
                        "vendor op: no active vendor context (player_id or vendor_entity unset)"
                    );
                    return true;
                }
            };

            let mut offset = 0;
            let items = match read_i32_array(args, &mut offset) {
                Some(items) => items,
                None => {
                    tracing::warn!(
                        entity_id,
                        op = op_name,
                        args_len = args.len(),
                        "vendor op: malformed item array in args"
                    );
                    return true;
                }
            };

            let trailing_template_id = read_trailing_template_id(args, offset);

            // For paid Repair/Recharge, `trailing_template_id` is optional (None
            // signals "free repair"). For Purchase/Sell/Buyback, it's required;
            // and in all cases where the client supplied one, it must match the
            // vendor that was actually opened so a client can't spoof it.
            let validated_template_id = match trailing_template_id {
                Some(client_id) => {
                    match validate_template_id(entity_id, op_name, &session, client_id) {
                        Some(server_id) => Some(server_id),
                        None => return true,
                    }
                }
                None => None,
            };

            let msg = match method_index {
                PURCHASE_ITEMS => match validated_template_id {
                    Some(vendor_template_id) => CellToBaseMsg::PurchaseVendorItems {
                        entity_id,
                        player_id: session.player_id,
                        vendor_entity_id: session.vendor_entity_id,
                        vendor_template_id,
                        items,
                    },
                    None => {
                        tracing::warn!(
                            entity_id,
                            op = op_name,
                            "vendor op: missing vendor_template_id"
                        );
                        return true;
                    }
                },
                SELL_ITEMS => match validated_template_id {
                    Some(vendor_template_id) => CellToBaseMsg::SellVendorItems {
                        entity_id,
                        player_id: session.player_id,
                        vendor_entity_id: session.vendor_entity_id,
                        vendor_template_id,
                        items,
                    },
                    None => {
                        tracing::warn!(
                            entity_id,
                            op = op_name,
                            "vendor op: missing vendor_template_id"
                        );
                        return true;
                    }
                },
                BUYBACK_ITEMS => match validated_template_id {
                    Some(vendor_template_id) => CellToBaseMsg::BuybackVendorItems {
                        entity_id,
                        player_id: session.player_id,
                        vendor_entity_id: session.vendor_entity_id,
                        vendor_template_id,
                        items,
                    },
                    None => {
                        tracing::warn!(
                            entity_id,
                            op = op_name,
                            "vendor op: missing vendor_template_id"
                        );
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
                tracing::warn!(
                    entity_id,
                    op = op_name,
                    "vendor op: cell->base channel closed: {e}"
                );
            }
            true
        }

        _ => false,
    }
}
