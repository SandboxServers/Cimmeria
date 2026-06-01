//! Outbound serializers for the trade flow.
//!
//! - `onTradeState(144)` — full `(localProposal, remoteProposal)` snapshot
//! - `onTradeResults(145)` — terminal status (Completed / Cancelled / etc.)
//!
//! Includes the `stub_inv_items_for` builder that pads `RemoteTradeProposal`
//! with sentinel-bearing `InvItem` records — the cell doesn't own the full
//! inventory state (base does), but the wire format requires a FIXED_DICT
//! per item, so we emit obvious sentinels rather than plausible lies.
//! See the [`UNRESOLVED_INV_ITEM_FIELD`] doc comment for the security
//! rationale.

use cimmeria_entity::inventory::InvItem;
use cimmeria_entity::trade::{serialize_on_trade_results, serialize_on_trade_state, TradeProposal};
use tokio::sync::mpsc;

use crate::cell::client_methods::player::{ON_TRADE_RESULTS, ON_TRADE_STATE};
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

pub(super) async fn send_on_trade_results(
    entity_id: u32,
    partner_entity_id: i32,
    result: i32,
    tx: &mpsc::Sender<CellToBaseMsg>,
) {
    let args = serialize_on_trade_results(partner_entity_id, result);
    if let Err(e) = tx
        .send(CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index: ON_TRADE_RESULTS,
            args,
        })
        .await
    {
        tracing::warn!(
            entity_id,
            partner_entity_id,
            result,
            error = %e,
            "send onTradeResults: cell→base channel closed",
        );
    }
}

/// Send `onTradeState` to both `entity_id` and `partner_entity_id`,
/// each from their own perspective (local = self, remote = partner).
pub(super) async fn send_on_trade_state_to_both(
    entity_id: u32,
    partner_entity_id: i32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) {
    // Snapshot both proposals so we can build packets without holding
    // a borrow across `.send().await`.
    let (a_proposal, b_proposal) = match (
        space_mgr.get_entity(entity_id),
        space_mgr.get_entity(partner_entity_id as u32),
    ) {
        (Some(a), Some(b)) => match (a.trade_proposal.clone(), b.trade_proposal.clone()) {
            (Some(ap), Some(bp)) => (ap, bp),
            _ => {
                tracing::warn!(
                    entity_id,
                    partner_entity_id,
                    "send_on_trade_state_to_both: missing proposal state"
                );
                return;
            }
        },
        _ => {
            tracing::warn!(
                entity_id,
                partner_entity_id,
                "send_on_trade_state_to_both: missing entity"
            );
            return;
        }
    };

    // Phase 1: we don't have a cell-side mirror of the full inventory
    // (the cell only caches the bandolier — full inventory lives in DB,
    // owned by base). The partner's RemoteTradeProposal would normally
    // carry full `InvItem` payloads so the partner's client can render
    // names + icons. For now we emit stub `InvItem`s built from the
    // (instance_id, slot_id) pairs we DO have — the client will see the
    // correct slot count and instance ids but icons may render as
    // placeholders. A future phase can fetch full InvItem rows from
    // base on every state change; tracked separately.
    let a_items = stub_inv_items_for(&a_proposal);
    let b_items = stub_inv_items_for(&b_proposal);

    // Packet to `entity_id`: local = a, remote = b (partner from a's view)
    let pkt_a = serialize_on_trade_state(partner_entity_id, &a_proposal, &b_proposal, &b_items);
    if let Err(e) = tx
        .send(CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index: ON_TRADE_STATE,
            args: pkt_a,
        })
        .await
    {
        tracing::warn!(
            entity_id,
            partner_entity_id,
            error = %e,
            "send onTradeState (a): cell→base channel closed",
        );
    }

    // Packet to `partner_entity_id`: local = b, remote = a
    let pkt_b = serialize_on_trade_state(entity_id as i32, &b_proposal, &a_proposal, &a_items);
    if let Err(e) = tx
        .send(CellToBaseMsg::EntityMethodCall {
            entity_id: partner_entity_id as u32,
            method_index: ON_TRADE_STATE,
            args: pkt_b,
        })
        .await
    {
        tracing::warn!(
            entity_id = partner_entity_id,
            partner_entity_id = entity_id,
            error = %e,
            "send onTradeState (b): cell→base channel closed",
        );
    }
}

/// Sentinel for InvItem fields the cell genuinely cannot resolve.
///
/// The wire format for `RemoteTradeProposal` requires a full InvItem
/// FIXED_DICT per item (the client expects fixed-shape records — there
/// is no variable-length InvItem encoding). The canonical SGW server
/// (Python) had full inventory state in the cell, so it filled these
/// fields from the actual `sgw_inventory` row. Our split architecture
/// has inventory state on base; the cell only caches the bandolier.
///
/// Pre-fix, the cell emitted plausible-looking placeholders:
/// `durability=100, is_bound=false, charges=0, stack_size=1`. Those
/// values are **lies in the worst case** — a partner who hasn't yet
/// received `onUpdateItem` for the offered item could trust them and
/// agree to a trade believing they're getting a durable, charged item,
/// then receive a broken empty one. The atomic swap is correct
/// server-side, but the client UI is deceived. That's an
/// information-asymmetry exploit a hostile player can use socially.
///
/// `-1` (`0xFFFFFFFF` as LE i32) is the chosen sentinel: it can't be a
/// legitimate durability / charges / stack size, so a defensive client
/// implementation can detect "unresolved" and either suppress the
/// summary or query for the canonical row. Even a non-defensive client
/// renders an obviously-bogus value rather than a plausible lie.
///
/// `dbid = 0` was already a sentinel (no real item type has dbid 0),
/// and we keep it that way.
///
/// This is the **option (c)** path from the security review on the
/// trade PR. Option (a) "send no InvItem payload" is wire-incompatible
/// (the FIXED_DICT shape is mandatory). Option (b) "round-trip to base
/// for canonical InvItem rows on every onTradeState" is a larger
/// architectural change (new request/response message types, deferred
/// onTradeState emission) — tracked separately. Until that lands,
/// sentinels close the information-asymmetry gap without changing the
/// wire shape.
const UNRESOLVED_INV_ITEM_FIELD: i32 = -1;

/// Build sentinel-bearing `InvItem` records from the (instance_id,
/// slot_id) pairs in a proposal. Every field the cell cannot prove is
/// emitted as `UNRESOLVED_INV_ITEM_FIELD` (i32 fields) / `0` (`dbid`,
/// `container_id`) / `false` (`is_bound`). Only `id` and `slot_id` are
/// known — those come straight from the client's own `LocalTradeItem`
/// in the proposal it just sent us, so they're not a leak.
pub(super) fn stub_inv_items_for(p: &TradeProposal) -> Vec<InvItem> {
    p.items
        .iter()
        .map(|t| InvItem {
            // Known: from the client's proposal it sent us.
            id: t.instance_id,
            slot_id: t.slot_id,
            // dbid = 0: pre-existing sentinel (no real item has dbid 0).
            // The client uses dbid to resolve item type → name / icon.
            dbid: 0,
            // Unresolved fields — sentinels rather than plausible lies.
            stack_size: UNRESOLVED_INV_ITEM_FIELD,
            container_id: 0,
            durability: UNRESOLVED_INV_ITEM_FIELD,
            charges: UNRESOLVED_INV_ITEM_FIELD,
            cur_ammo_type: UNRESOLVED_INV_ITEM_FIELD,
            // is_bound defaults to false. We can't know without a base
            // round-trip; emitting `true` would be a worse lie (would
            // wrongly suggest the item is soul-bound and thus
            // un-tradeable, contradicting the proposal). The receiver's
            // client should consult their own cache, which we expect
            // to be empty for partner items.
            is_bound: false,
            // ammo_types: empty array, same rationale as dbid=0 — there
            // is no in-band sentinel for an i32 array, so an empty
            // array is the least-deceptive signal.
            ammo_types: vec![],
        })
        .collect()
}
