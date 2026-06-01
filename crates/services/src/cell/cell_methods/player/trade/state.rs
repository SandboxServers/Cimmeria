//! Trade session state-machine helpers.
//!
//! Owns the lifecycle transitions on the two `CellEntity`s — opening a
//! session, applying proposals, tearing down on cancel/disconnect, and
//! the geometric proximity check.
//!
//! These functions don't talk to the wire directly; they mutate cell
//! state and then defer outbound serialization to [`super::wire`].

use std::collections::HashSet;

use cimmeria_entity::trade::{TradeProposal, ETRADELOCKSTATE_NONE, ETRADERESULTS_CANCELLED};
use tokio::sync::mpsc;

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

use super::wire::{send_on_trade_results, send_on_trade_state_to_both};
use super::MAX_INTERACT_DISTANCE;

/// Open a trade session between two entities. Performs the Python
/// `beginTrading` validation gauntlet: not already trading, not self,
/// partner exists and is a player in the same space within
/// MAX_INTERACT_DISTANCE, partner not already trading.
///
/// On success, writes `trade_partner_entity_id` + fresh empty
/// `trade_proposal` on BOTH entities and returns `true`.
pub(super) fn begin_trading(
    entity_id: u32,
    partner_entity_id: i32,
    space_mgr: &mut SpaceManager,
) -> bool {
    if partner_entity_id <= 0 {
        tracing::warn!(entity_id, partner_entity_id, "beginTrading: invalid id");
        return false;
    }
    if partner_entity_id as u32 == entity_id {
        tracing::warn!(entity_id, "beginTrading: cannot trade with self");
        return false;
    }

    let me = match space_mgr.get_entity(entity_id) {
        Some(e) => e,
        None => {
            tracing::warn!(entity_id, "beginTrading: caller entity not found");
            return false;
        }
    };
    // Symmetric `is_player` gate — we already reject non-player partners
    // below, and the same gate must apply to the caller. Without this,
    // any non-player entity that gets routed into this handler (e.g.,
    // an NPC granted a player_id by a content-engine bug, or a future
    // entity type that reuses cell methods) could open a trade session
    // from the server side. Python only reached this code path from
    // SGWPlayer.def-dispatched methods, so the guarantee was structural;
    // our Rust split routes inbound methods through `dispatch` which
    // does NOT enforce that structurally, so we enforce it here.
    if !me.is_player {
        tracing::warn!(
            entity_id,
            partner_entity_id,
            "beginTrading: caller is not a player — rejecting"
        );
        return false;
    }
    if me.trade_partner_entity_id.is_some() {
        tracing::warn!(
            entity_id,
            "beginTrading: caller already in another trade session"
        );
        return false;
    }
    let my_pos = me.position;
    let my_space = me.space_id;

    let partner = match space_mgr.get_entity(partner_entity_id as u32) {
        Some(e) => e,
        None => {
            tracing::info!(
                entity_id,
                partner_entity_id,
                "beginTrading: partner not found"
            );
            return false;
        }
    };
    if !partner.is_player {
        tracing::info!(
            entity_id,
            partner_entity_id,
            "beginTrading: target is not a player"
        );
        return false;
    }
    if partner.space_id != my_space {
        tracing::info!(
            entity_id,
            partner_entity_id,
            "beginTrading: partner in a different space"
        );
        return false;
    }
    let dist = my_pos.distance_squared_to(&partner.position).sqrt();
    if dist > MAX_INTERACT_DISTANCE {
        tracing::info!(
            entity_id,
            partner_entity_id,
            dist,
            "beginTrading: too far away"
        );
        return false;
    }
    if partner.trade_partner_entity_id.is_some() {
        tracing::info!(
            entity_id,
            partner_entity_id,
            "beginTrading: partner already trading"
        );
        return false;
    }

    // All validations passed — wire both sides into the session.
    if let Some(me) = space_mgr.get_entity_mut(entity_id) {
        me.trade_partner_entity_id = Some(partner_entity_id as u32);
        me.trade_proposal = Some(TradeProposal::default());
    }
    if let Some(p) = space_mgr.get_entity_mut(partner_entity_id as u32) {
        p.trade_partner_entity_id = Some(entity_id);
        p.trade_proposal = Some(TradeProposal::default());
    }
    tracing::info!(entity_id, partner_entity_id, "trade session opened");
    true
}

/// Apply a new proposal to `entity_id`'s side. Returns `false` if the
/// version check fails (stale proposal — typically a replayed packet).
///
/// On success, both sides' `lock_state` is reset to `None` and the
/// updated `onTradeState` is fanned out to both clients.
pub(super) async fn apply_proposal(
    entity_id: u32,
    partner_entity_id: i32,
    new_proposal: TradeProposal,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> bool {
    // Per Trade.py:35-38: must monotonically bump version by 1.
    let expected_version = space_mgr
        .get_entity(entity_id)
        .and_then(|e| e.trade_proposal.as_ref().map(|p| p.version + 1))
        .unwrap_or(0);
    if new_proposal.version != expected_version {
        tracing::warn!(
            entity_id,
            partner_entity_id,
            got = new_proposal.version,
            expected = expected_version,
            "applyProposal: bad version"
        );
        return false;
    }

    // De-duplicate item instances (Trade.py:53-58). A client sending the
    // same instance twice gets the second occurrence dropped silently —
    // INTENTIONAL Python parity, not an oversight. The partner's UI will
    // show fewer items than the actor's proposal claimed (the actor sent
    // `[A, A, A]` but the partner sees `[A]` after dedup); this is fine
    // because (a) the partner only commits after they personally lock,
    // so the visible-count mismatch is observable before any goods move,
    // and (b) the base-side atomic commit also dedups in `lock_items`
    // and rejects duplicate instance_ids outright — so even if a
    // hostile client managed to bypass cell-side dedup, the swap cannot
    // double-spend. We DON'T validate item ownership / canSell() here —
    // base-side commit re-validates against the DB inside the FOR UPDATE
    // transaction, which is the only TOCTOU-safe point to check.
    let mut seen = HashSet::with_capacity(new_proposal.items.len());
    let deduped_items: Vec<_> = new_proposal
        .items
        .iter()
        .filter(|it| seen.insert(it.instance_id))
        .copied()
        .collect();

    // Commit the new proposal on the actor; reset both sides' locks.
    if let Some(me) = space_mgr.get_entity_mut(entity_id) {
        if let Some(p) = me.trade_proposal.as_mut() {
            p.version = new_proposal.version;
            p.cash = new_proposal.cash;
            p.items = deduped_items;
            p.lock_state = ETRADELOCKSTATE_NONE;
        }
    }
    if let Some(partner) = space_mgr.get_entity_mut(partner_entity_id as u32) {
        if let Some(p) = partner.trade_proposal.as_mut() {
            p.lock_state = ETRADELOCKSTATE_NONE;
        }
    }

    // Broadcast onTradeState to both sides.
    send_on_trade_state_to_both(entity_id, partner_entity_id, tx, space_mgr).await;
    true
}

/// Clear the trade state on both participants. Called from cancel /
/// commit / disconnect paths.
pub(super) fn clear_trade_state(
    entity_id: u32,
    partner_entity_id: i32,
    space_mgr: &mut SpaceManager,
) {
    if let Some(e) = space_mgr.get_entity_mut(entity_id) {
        e.trade_partner_entity_id = None;
        e.trade_proposal = None;
    }
    if let Some(p) = space_mgr.get_entity_mut(partner_entity_id as u32) {
        p.trade_partner_entity_id = None;
        p.trade_proposal = None;
    }
}

/// Tear down a session and notify both clients with `onTradeResults(result)`.
pub(super) async fn cancel_session(
    entity_id: u32,
    partner_entity_id: i32,
    result: i32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    clear_trade_state(entity_id, partner_entity_id, space_mgr);
    // Each player sees their partner's entity id in the wire payload.
    send_on_trade_results(entity_id, partner_entity_id, result, tx).await;
    send_on_trade_results(partner_entity_id as u32, entity_id as i32, result, tx).await;
    tracing::info!(
        entity_id,
        partner_entity_id,
        result,
        "trade session cancelled"
    );
}

/// Public entry point for the disconnect / DestroyEntity hook in
/// `service::base_messages`. If the entity has an open trade, this
/// closes it with `Cancelled` (NOT `Completed` — disconnect is a fault
/// path, not a user-initiated cancel).
///
/// Returns `Some(partner_entity_id)` so the caller can correlate logs.
pub async fn cancel_trade_on_disconnect(
    entity_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> Option<u32> {
    let partner = space_mgr.get_entity(entity_id)?.trade_partner_entity_id?;
    cancel_session(
        entity_id,
        partner as i32,
        ETRADERESULTS_CANCELLED,
        tx,
        space_mgr,
    )
    .await;
    Some(partner)
}

/// Whether the two participants are within `MAX_INTERACT_DISTANCE` AND
/// in the same space. `false` whenever either side cannot be looked up
/// (treat missing entities as out-of-range so we tear down cleanly).
///
/// Distance is 3D (includes Y/elevation) by convention — matches Python
/// `Atrea.utils.distance3d` used throughout the original cell, and the
/// same metric vendor / dialog / loot interactions use. A player on a
/// balcony 4m above another player is "in range" if they're within
/// 3m horizontally; a player on a balcony 6m above is not. Switching
/// to a 2D (horizontal-only) check would diverge from every other
/// interaction range gate in the codebase — out of scope for this PR.
pub(super) fn partners_in_range(
    entity_id: u32,
    partner_entity_id: i32,
    space_mgr: &SpaceManager,
) -> bool {
    let me = match space_mgr.get_entity(entity_id) {
        Some(e) => e,
        None => return false,
    };
    let partner = match space_mgr.get_entity(partner_entity_id as u32) {
        Some(e) => e,
        None => return false,
    };
    if me.space_id != partner.space_id {
        return false;
    }
    me.position.distance_squared_to(&partner.position).sqrt() <= MAX_INTERACT_DISTANCE
}
