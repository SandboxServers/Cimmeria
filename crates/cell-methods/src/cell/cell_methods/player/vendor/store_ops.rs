//! The five store cell methods: `purchaseItems` (78), `sellItems` (79),
//! `buybackItems` (80), `repairItems` (81), `rechargeItems` (82).
//!
//! The client names only the lines (wire layouts in [`super::wire`]); the
//! vendor, and so the template the base prices against, always comes from
//! the server's own session: the vendor entity the player opened. No vendor
//! session, or a vendor with no template, refuses the request with a
//! feedback line (owner rule: every button press gets visible feedback).

use cimmeria_entity::cell_entity::PlayerIdentity;
use cimmeria_wire::cell::chat::{serialize_on_player_communication, CHAN_FEEDBACK};
use cimmeria_wire::cell::client_methods::communicator::ON_PLAYER_COMMUNICATION;
use tokio::sync::mpsc;

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

use super::super::constants::{
    BUYBACK_ITEMS, PURCHASE_ITEMS, RECHARGE_ITEMS, REPAIR_ITEMS, SELL_ITEMS,
};
use super::session::vendor_context;
use super::wire::{read_id_quantity_lines, read_item_ids, WireReject};

/// The feedback line for a store request with no vendor to price it.
pub(super) const NO_VENDOR_TEXT: &str = "You are not trading with a vendor.";

/// What a store method's payload decoded to.
enum Lines {
    /// `(ItemIndices | ItemIds, Quantities)` lines (purchase/sell/buyback).
    Quantified(Vec<(i32, i32)>),
    /// `ItemIds` (repair/recharge).
    Ids(Vec<i32>),
}

impl Lines {
    fn len(&self) -> usize {
        match self {
            Self::Quantified(lines) => lines.len(),
            Self::Ids(ids) => ids.len(),
        }
    }
}

/// `(op name, telemetry action)` for a store method. The `action` matches
/// the base `vendor` rows (`buy`, not `purchase`), so one request's cell and
/// base rows share a value.
pub(super) fn names(method_index: u16) -> Option<(&'static str, &'static str)> {
    Some(match method_index {
        PURCHASE_ITEMS => ("purchaseItems", "buy"),
        SELL_ITEMS => ("sellItems", "sell"),
        BUYBACK_ITEMS => ("buybackItems", "buyback"),
        REPAIR_ITEMS => ("repairItems", "repair"),
        RECHARGE_ITEMS => ("rechargeItems", "recharge"),
        _ => return None,
    })
}

/// Decode, check the vendor session, and forward one store request to the
/// base. Every refusal is one WARN on the `vendor` target.
pub(super) async fn handle_store_op(
    entity_id: u32,
    method_index: u16,
    args: &[u8],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) {
    let Some((op_name, action)) = names(method_index) else {
        return;
    };
    let identity = space_mgr.player_identity(entity_id);

    let decoded = match method_index {
        REPAIR_ITEMS | RECHARGE_ITEMS => read_item_ids(args).map(Lines::Ids),
        _ => read_id_quantity_lines(args).map(Lines::Quantified),
    };

    let session = vendor_context(entity_id, space_mgr);
    let vendor_entity_id = session.as_ref().map(|s| s.vendor_entity_id);

    let lines = match decoded {
        Ok(lines) => lines,
        Err(reject) => {
            // A stock client cannot build a malformed payload, so no
            // feedback line: answering would hand a forging client a free
            // message per packet.
            log_refusal(
                Refusal {
                    entity_id,
                    identity,
                    action,
                    op_name,
                    vendor_entity_id,
                    item_count: None,
                    reason: "malformed_args",
                    detail: Some(reject),
                },
                args.len(),
                space_mgr,
            );
            return;
        }
    };

    let (player_id, vendor_entity_id, vendor_template_id) = match session {
        Some(super::session::VendorSession {
            player_id,
            vendor_entity_id,
            server_template_id: Some(template),
        }) => (player_id, vendor_entity_id, template),
        other => {
            let reason = if other.is_some() {
                // The opened vendor is gone or carries no template: the
                // base has nothing to price against.
                "vendor_has_no_template"
            } else {
                "no_vendor_session"
            };
            log_refusal(
                Refusal {
                    entity_id,
                    identity,
                    action,
                    op_name,
                    vendor_entity_id,
                    item_count: Some(lines.len()),
                    reason,
                    detail: None,
                },
                args.len(),
                space_mgr,
            );
            send_feedback(entity_id, NO_VENDOR_TEXT, tx, space_mgr).await;
            return;
        }
    };

    let msg = match (method_index, lines) {
        (PURCHASE_ITEMS, Lines::Quantified(items)) => CellToBaseMsg::PurchaseVendorItems {
            entity_id,
            player_id,
            vendor_entity_id,
            vendor_template_id,
            items,
        },
        (SELL_ITEMS, Lines::Quantified(items)) => CellToBaseMsg::SellVendorItems {
            entity_id,
            player_id,
            vendor_entity_id,
            vendor_template_id,
            items,
        },
        (BUYBACK_ITEMS, Lines::Quantified(items)) => CellToBaseMsg::BuybackVendorItems {
            entity_id,
            player_id,
            vendor_entity_id,
            vendor_template_id,
            items,
        },
        (REPAIR_ITEMS, Lines::Ids(item_ids)) => CellToBaseMsg::RepairInventoryItems {
            entity_id,
            player_id,
            item_ids,
            vendor_template_id: Some(vendor_template_id),
        },
        (RECHARGE_ITEMS, Lines::Ids(item_ids)) => CellToBaseMsg::RechargeInventoryItems {
            entity_id,
            player_id,
            item_ids,
            vendor_template_id: Some(vendor_template_id),
        },
        // `decoded` picks the shape from the same method index.
        _ => unreachable!("store op shape follows the method index"),
    };

    if let Err(e) = tx.send(msg).await {
        tracing::warn!(
            target: "vendor",
            event = "failed",
            action,
            reason = "base_channel_closed",
            account_id = identity.account_id,
            account_name = identity.account_name,
            player_id,
            player_name = identity.player_name,
            entity_id,
            entity_name = identity.player_name,
            vendor_entity_id,
            vendor_entity_name = space_mgr.entity_label(vendor_entity_id as u32),
            op = op_name,
            "vendor op: cell->base channel closed: {e}"
        );
    }
}

/// One refused store request.
struct Refusal {
    entity_id: u32,
    identity: PlayerIdentity,
    action: &'static str,
    op_name: &'static str,
    vendor_entity_id: Option<i32>,
    item_count: Option<usize>,
    reason: &'static str,
    /// The decoder's verdict, for `malformed_args`.
    detail: Option<WireReject>,
}

fn log_refusal(r: Refusal, args_len: usize, space_mgr: &SpaceManager) {
    tracing::warn!(
        target: "vendor",
        event = "refused",
        action = r.action,
        reason = r.reason,
        wire_reason = r.detail.map(WireReject::reason),
        account_id = r.identity.account_id,
        account_name = r.identity.account_name,
        player_id = r.identity.player_id,
        player_name = r.identity.player_name,
        entity_id = r.entity_id,
        entity_name = r.identity.player_name,
        vendor_entity_id = r.vendor_entity_id,
        vendor_entity_name = space_mgr.entity_label(r.vendor_entity_id.map(|v| v as u32)),
        item_count = r.item_count,
        op = r.op_name,
        args_len,
        "vendor op refused"
    );
}

/// One `SYSTEM` line on the feedback channel to the player's own client.
async fn send_feedback(
    entity_id: u32,
    text: &str,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) {
    if let Err(e) = tx
        .send(CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index: ON_PLAYER_COMMUNICATION,
            args: serialize_on_player_communication("SYSTEM", 0, CHAN_FEEDBACK, text),
        })
        .await
    {
        tracing::warn!(
            target: "vendor",
            event = "feedback_send_failed",
            entity_id,
            entity_name = space_mgr.entity_label(entity_id),
            reason = "base_channel_closed",
            "vendor op: refusal feedback line could not be queued: {e}"
        );
    }
}
