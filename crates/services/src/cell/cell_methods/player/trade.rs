//! Player-to-player trade cell-method handlers.
//!
//! Wire methods (inbound, client → server):
//! - 104 `tradeRequest` — open a session with a partner
//! - 105 `tradeRequestCancel` — close an open session
//! - 106 `tradeUpdateProposal` — push a new offer
//! - 107 `tradeLockState` — transition the lock state
//!
//! Outbound (server → client):
//! - 144 `onTradeState` — broadcast both proposals to one player
//! - 145 `onTradeResults` — terminal notification (commit / cancel)
//!
//! State lives on the two `CellEntity`s (`trade_partner_entity_id` +
//! `trade_proposal`). The atomic swap happens base-side via
//! `CellToBaseMsg::ExecuteTrade` — the cell hands off the
//! to-be-executed proposals, base wraps everything in a single sqlx tx.
//!
//! Reference: `deprecated/python/cell/Trade.py`,
//! `deprecated/python/cell/SGWPlayer.py:1676-1820`.

use cimmeria_entity::inventory::InvItem;
use cimmeria_entity::trade::{
    serialize_on_trade_results, serialize_on_trade_state, TradeProposal, ETRADELOCKSTATE_LOCKED,
    ETRADELOCKSTATE_LOCKED_AND_CONFIRMED, ETRADELOCKSTATE_NONE, ETRADERESULTS_CANCELLED,
    ETRADERESULTS_COMPLETED,
};
use tokio::sync::mpsc;

use crate::cell::client_methods::player::{ON_TRADE_RESULTS, ON_TRADE_STATE};
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

use super::constants::*;

/// Mirror of `python/common/Constants.py: MAX_INTERACT_DISTANCE = 5`.
/// Trade is gated by the same range as vendor / dialog interactions.
const MAX_INTERACT_DISTANCE: f32 = 5.0;

/// Sub-dispatcher for inbound trade methods (104..=107).
///
/// Returns `true` if the method was recognised — the caller surfaces a
/// `false` as the generic "unhandled cell method" warn. Bad-args /
/// out-of-range / no-session paths still return `true`: they ARE
/// trade methods, just rejected on validation.
pub async fn dispatch(
    entity_id: u32,
    method_index: u16,
    args: &[u8],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> bool {
    match method_index {
        TRADE_REQUEST => {
            handle_trade_request(entity_id, args, tx, space_mgr).await;
            true
        }
        TRADE_REQUEST_CANCEL => {
            handle_trade_request_cancel(entity_id, args, tx, space_mgr).await;
            true
        }
        TRADE_UPDATE_PROPOSAL => {
            handle_trade_update_proposal(entity_id, args, tx, space_mgr).await;
            true
        }
        TRADE_LOCK_STATE => {
            handle_trade_lock_state(entity_id, args, tx, space_mgr).await;
            true
        }
        _ => false,
    }
}

// ── Inbound: tradeRequest (104) ────────────────────────────────────────────

#[tracing::instrument(name = "trade.request", level = "info", skip_all, fields(entity_id))]
async fn handle_trade_request(
    entity_id: u32,
    args: &[u8],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    // Wire: INT32 partnerEntityId + LocalTradeProposal
    if args.len() < 4 {
        tracing::warn!(entity_id, "tradeRequest: truncated args");
        return;
    }
    let partner_entity_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
    let mut offset = 4;
    let proposal = match TradeProposal::parse(args, &mut offset) {
        Some(p) => p,
        None => {
            tracing::warn!(
                entity_id,
                partner_entity_id,
                "tradeRequest: malformed LocalTradeProposal"
            );
            return;
        }
    };

    if !begin_trading(entity_id, partner_entity_id, space_mgr) {
        // beginTrading failed — Python sends Cancelled to the caller only.
        send_on_trade_results(entity_id, partner_entity_id, ETRADERESULTS_CANCELLED, tx).await;
        return;
    }

    // Session is open on both sides. Now apply the proposal — the request
    // wire frame carries the initial offer. If proposal-update fails
    // (bad version, etc.) we tear the session down with Cancelled.
    if !apply_proposal(entity_id, partner_entity_id, proposal, tx, space_mgr).await {
        cancel_session(
            entity_id,
            partner_entity_id,
            ETRADERESULTS_COMPLETED,
            tx,
            space_mgr,
        )
        .await;
    }
}

// ── Inbound: tradeRequestCancel (105) ─────────────────────────────────────

#[tracing::instrument(name = "trade.cancel", level = "info", skip_all, fields(entity_id))]
async fn handle_trade_request_cancel(
    entity_id: u32,
    args: &[u8],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    if args.len() < 4 {
        tracing::warn!(entity_id, "tradeRequestCancel: truncated args");
        return;
    }
    let partner_entity_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);

    // Validate: must currently be trading with this exact partner.
    let actually_trading = space_mgr
        .get_entity(entity_id)
        .and_then(|e| e.trade_partner_entity_id)
        == Some(partner_entity_id as u32);
    if !actually_trading {
        tracing::warn!(
            entity_id,
            partner_entity_id,
            "tradeRequestCancel: no open session with that partner"
        );
        return;
    }

    // TRAP #2: cancel sends Completed (1), not Cancelled (2). See
    // Trade.py:225-228 — both players get a clean shutdown notification
    // regardless of who hit cancel. Cancelled (2) is reserved for the
    // disconnect / distance-break / commit-failure paths.
    cancel_session(
        entity_id,
        partner_entity_id,
        ETRADERESULTS_COMPLETED,
        tx,
        space_mgr,
    )
    .await;
}

// ── Inbound: tradeUpdateProposal (106) ─────────────────────────────────────

#[tracing::instrument(
    name = "trade.update_proposal",
    level = "info",
    skip_all,
    fields(entity_id)
)]
async fn handle_trade_update_proposal(
    entity_id: u32,
    args: &[u8],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    if args.len() < 4 {
        tracing::warn!(entity_id, "tradeUpdateProposal: truncated args");
        return;
    }
    let partner_entity_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
    let mut offset = 4;
    let proposal = match TradeProposal::parse(args, &mut offset) {
        Some(p) => p,
        None => {
            tracing::warn!(
                entity_id,
                partner_entity_id,
                "tradeUpdateProposal: malformed LocalTradeProposal"
            );
            return;
        }
    };

    // TRAP #3: QA 0.8384 client skips `tradeRequest` entirely (see
    // SGWPlayer.py:1785-1790). If no session is open we must call
    // beginTrading() here, not reject. Without this, trading never
    // works on the QA client.
    let already_trading = space_mgr
        .get_entity(entity_id)
        .and_then(|e| e.trade_partner_entity_id)
        .is_some();
    if !already_trading {
        tracing::info!(
            entity_id,
            partner_entity_id,
            "tradeUpdateProposal: no session open — applying QA-client workaround \
             (Python SGWPlayer.py:1785-1790): calling beginTrading() first"
        );
        if !begin_trading(entity_id, partner_entity_id, space_mgr) {
            send_on_trade_results(entity_id, partner_entity_id, ETRADERESULTS_CANCELLED, tx).await;
            return;
        }
    }

    // Validate: the session must be with THIS partner (not someone else).
    let partner_ok = space_mgr
        .get_entity(entity_id)
        .and_then(|e| e.trade_partner_entity_id)
        == Some(partner_entity_id as u32);
    if !partner_ok {
        tracing::warn!(
            entity_id,
            partner_entity_id,
            "tradeUpdateProposal: open session is with a different partner"
        );
        return;
    }

    // Distance re-check on every update — improves on Python's begin-only
    // check (deep dive recommendation). A trade open while one player
    // walks away should auto-cancel rather than complete from too-far.
    if !partners_in_range(entity_id, partner_entity_id, space_mgr) {
        tracing::info!(
            entity_id,
            partner_entity_id,
            "tradeUpdateProposal: partners out of range — auto-cancelling"
        );
        cancel_session(
            entity_id,
            partner_entity_id,
            ETRADERESULTS_CANCELLED,
            tx,
            space_mgr,
        )
        .await;
        return;
    }

    if !apply_proposal(entity_id, partner_entity_id, proposal, tx, space_mgr).await {
        // Mirror Python: a failed update tears the session down.
        cancel_session(
            entity_id,
            partner_entity_id,
            ETRADERESULTS_COMPLETED,
            tx,
            space_mgr,
        )
        .await;
    }
}

// ── Inbound: tradeLockState (107) ──────────────────────────────────────────

#[tracing::instrument(name = "trade.lock_state", level = "info", skip_all, fields(entity_id))]
async fn handle_trade_lock_state(
    entity_id: u32,
    args: &[u8],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    if args.len() < 9 {
        tracing::warn!(entity_id, "tradeLockState: truncated args (need 9 bytes)");
        return;
    }
    let local_version = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
    let remote_version = i32::from_le_bytes([args[4], args[5], args[6], args[7]]);
    let mut new_lock = args[8] as i8;

    // Validate session.
    let partner_entity_id = match space_mgr
        .get_entity(entity_id)
        .and_then(|e| e.trade_partner_entity_id)
    {
        Some(p) => p as i32,
        None => {
            tracing::warn!(entity_id, "tradeLockState: no open trade session");
            return;
        }
    };

    // Range validation: only None, Locked, LockedAndConfirmed are valid.
    if !(ETRADELOCKSTATE_NONE..=ETRADELOCKSTATE_LOCKED_AND_CONFIRMED).contains(&new_lock) {
        tracing::warn!(
            entity_id,
            partner_entity_id,
            new_lock,
            "tradeLockState: invalid lock state value — rejecting"
        );
        return;
    }

    // Distance re-check (deep dive improvement vs. Python).
    if !partners_in_range(entity_id, partner_entity_id, space_mgr) {
        tracing::info!(
            entity_id,
            partner_entity_id,
            "tradeLockState: partners out of range — auto-cancelling"
        );
        cancel_session(
            entity_id,
            partner_entity_id,
            ETRADERESULTS_CANCELLED,
            tx,
            space_mgr,
        )
        .await;
        return;
    }

    // Version check on the actor's own proposal — Python rejects if
    // `localVersionId != proposal.version`. Stops a stale-version lock
    // (e.g., the player tried to lock against their own previous
    // proposal after we already accepted a newer one).
    let (my_version, partner_version, prev_my_lock) = {
        let me = match space_mgr.get_entity(entity_id) {
            Some(e) => e,
            None => return,
        };
        let my_prop = match &me.trade_proposal {
            Some(p) => p,
            None => {
                tracing::warn!(entity_id, "tradeLockState: missing own proposal state");
                return;
            }
        };
        let partner = match space_mgr.get_entity(partner_entity_id as u32) {
            Some(p) => p,
            None => {
                tracing::warn!(
                    entity_id,
                    partner_entity_id,
                    "tradeLockState: partner entity missing"
                );
                return;
            }
        };
        let partner_prop = match &partner.trade_proposal {
            Some(p) => p,
            None => {
                tracing::warn!(
                    entity_id,
                    partner_entity_id,
                    "tradeLockState: partner has no proposal state"
                );
                return;
            }
        };
        (my_prop.version, partner_prop.version, my_prop.lock_state)
    };

    if local_version != my_version {
        tracing::warn!(
            entity_id,
            partner_entity_id,
            local_version,
            my_version,
            "tradeLockState: stale local version — rejecting"
        );
        return;
    }

    // Per Trade.py:201-204: if the actor hasn't seen the latest partner
    // proposal yet (remoteVersionId != partner.version), silently downgrade
    // any "Locked"/"LockedAndConfirmed" attempt to None — the client has
    // to catch up first.
    if new_lock != ETRADELOCKSTATE_NONE && remote_version != partner_version {
        tracing::debug!(
            entity_id,
            partner_entity_id,
            local_version,
            remote_version,
            partner_version,
            "tradeLockState: actor has stale view of partner — downgrading to None"
        );
        new_lock = ETRADELOCKSTATE_NONE;
    }

    // Per Trade.py:207-210: if the requested lock is None AND the actor
    // wasn't currently in Locked state, also clear the partner's lock.
    // (Effect: any not-yet-confirmed unlock propagates back to the partner;
    // a partner who had locked stays locked only while the actor was
    // already locked — the comment in Python says "if one of the players
    // released the trade lock we'll release the lock on the partner as
    // well", which is a slight misread of the actual conditional but
    // the conditional is what the client expects.)
    let propagate_unlock_to_partner =
        new_lock == ETRADELOCKSTATE_NONE && prev_my_lock != ETRADELOCKSTATE_LOCKED;

    // Apply the new lock state to the actor's proposal.
    if let Some(me) = space_mgr.get_entity_mut(entity_id) {
        if let Some(p) = me.trade_proposal.as_mut() {
            p.lock_state = new_lock;
        }
    }
    if propagate_unlock_to_partner {
        if let Some(partner) = space_mgr.get_entity_mut(partner_entity_id as u32) {
            if let Some(p) = partner.trade_proposal.as_mut() {
                p.lock_state = ETRADELOCKSTATE_NONE;
            }
        }
    }

    // Broadcast onTradeState to both sides.
    send_on_trade_state_to_both(entity_id, partner_entity_id, tx, space_mgr).await;

    // If both sides have reached LockedAndConfirmed → commit.
    let (my_lock, their_lock) = {
        let me = space_mgr.get_entity(entity_id);
        let them = space_mgr.get_entity(partner_entity_id as u32);
        match (me, them) {
            (Some(m), Some(t)) => (
                m.trade_proposal
                    .as_ref()
                    .map(|p| p.lock_state)
                    .unwrap_or(ETRADELOCKSTATE_NONE),
                t.trade_proposal
                    .as_ref()
                    .map(|p| p.lock_state)
                    .unwrap_or(ETRADELOCKSTATE_NONE),
            ),
            _ => (ETRADELOCKSTATE_NONE, ETRADELOCKSTATE_NONE),
        }
    };

    if my_lock == ETRADELOCKSTATE_LOCKED_AND_CONFIRMED
        && their_lock == ETRADELOCKSTATE_LOCKED_AND_CONFIRMED
    {
        request_execute_trade(entity_id, partner_entity_id, tx, space_mgr).await;
    }
}

// ── State helpers ──────────────────────────────────────────────────────────

/// Open a trade session between two entities. Performs the Python
/// `beginTrading` validation gauntlet: not already trading, not self,
/// partner exists and is a player in the same space within
/// MAX_INTERACT_DISTANCE, partner not already trading.
///
/// On success, writes `trade_partner_entity_id` + fresh empty
/// `trade_proposal` on BOTH entities and returns `true`.
fn begin_trading(entity_id: u32, partner_entity_id: i32, space_mgr: &mut SpaceManager) -> bool {
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
async fn apply_proposal(
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
    // same instance twice gets the second occurrence dropped silently.
    // We DON'T validate item ownership / canSell() here — base-side
    // commit re-validates against the DB inside the FOR UPDATE
    // transaction, which is the only TOCTOU-safe point to check.
    let mut seen = std::collections::HashSet::with_capacity(new_proposal.items.len());
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
fn clear_trade_state(entity_id: u32, partner_entity_id: i32, space_mgr: &mut SpaceManager) {
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
async fn cancel_session(
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
fn partners_in_range(entity_id: u32, partner_entity_id: i32, space_mgr: &SpaceManager) -> bool {
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

// ── Outbound senders ───────────────────────────────────────────────────────

async fn send_on_trade_results(
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
async fn send_on_trade_state_to_both(
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

/// Build placeholder `InvItem` records from the (instance_id, slot_id)
/// pairs in a proposal. Stack/container/durability/etc. are best-effort
/// zeros — the client's trade panel renders by `id` lookup against the
/// cached inventory it already has from `onUpdateItem`, so the
/// stub-icon concern is mostly hypothetical when both players already
/// know each other's full inventory through some other channel.
fn stub_inv_items_for(p: &TradeProposal) -> Vec<InvItem> {
    p.items
        .iter()
        .map(|t| InvItem {
            id: t.instance_id,
            dbid: 0,
            stack_size: 1,
            slot_id: t.slot_id,
            container_id: 0,
            is_bound: false,
            durability: 100,
            ammo_types: vec![],
            cur_ammo_type: 0,
            charges: 0,
        })
        .collect()
}

// ── Atomic commit hand-off ─────────────────────────────────────────────────

/// Both sides reached `LockedAndConfirmed` — kick the execution off to
/// the base layer (which owns the DB). The cell clears the in-memory
/// session state immediately; base will fire `onTradeResults` to both
/// clients itself, with the appropriate per-side result code (Completed
/// on success, NoLocalCash/NoLocalSpace/NoRemoteCash/NoRemoteSpace on a
/// validation failure inside the FOR UPDATE tx).
async fn request_execute_trade(
    entity_id: u32,
    partner_entity_id: i32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    // Final distance checkpoint before the cell→base handoff.
    //
    // `tradeUpdateProposal` and `tradeLockState` already re-check
    // distance on every inbound message, but a small window exists
    // between the *partner's* `LockedAndConfirmed` arriving and this
    // function running: one player can walk out of range while the
    // partner's confirmation is in flight. Once the `ExecuteTrade`
    // message is queued for base, the swap commits regardless of
    // where the players are standing. Refuse the handoff if the
    // proximity invariant just broke; both clients see Cancelled (2)
    // and start over — no items move, no cash changes.
    if !partners_in_range(entity_id, partner_entity_id, space_mgr) {
        tracing::info!(
            entity_id,
            partner_entity_id,
            "request_execute_trade: partners out of range at handoff — \
             auto-cancelling instead of committing"
        );
        cancel_session(
            entity_id,
            partner_entity_id,
            ETRADERESULTS_CANCELLED,
            tx,
            space_mgr,
        )
        .await;
        return;
    }

    // Snapshot the state we need for the base-side commit. After the
    // snapshot we drop the proposals from the cell (sessions are torn
    // down regardless of commit outcome) — the source of truth for the
    // rest of the flow lives in the message payload.
    let snapshot = {
        let me = match space_mgr.get_entity(entity_id) {
            Some(e) => e,
            None => {
                tracing::error!(entity_id, "request_execute_trade: caller entity gone");
                return;
            }
        };
        let partner = match space_mgr.get_entity(partner_entity_id as u32) {
            Some(e) => e,
            None => {
                tracing::error!(
                    entity_id,
                    partner_entity_id,
                    "request_execute_trade: partner entity gone"
                );
                return;
            }
        };
        let my_pid = match me.player_id {
            Some(p) => p,
            None => {
                tracing::warn!(
                    entity_id,
                    "request_execute_trade: caller has no player_id — refusing"
                );
                return;
            }
        };
        let p_pid = match partner.player_id {
            Some(p) => p,
            None => {
                tracing::warn!(
                    partner_entity_id,
                    "request_execute_trade: partner has no player_id — refusing"
                );
                return;
            }
        };
        let my_prop = match &me.trade_proposal {
            Some(p) => p.clone(),
            None => return,
        };
        let p_prop = match &partner.trade_proposal {
            Some(p) => p.clone(),
            None => return,
        };
        ExecuteTradeSnapshot {
            my_pid,
            p_pid,
            my_items: my_prop.items.iter().map(|t| t.instance_id).collect(),
            my_cash: my_prop.cash,
            p_items: p_prop.items.iter().map(|t| t.instance_id).collect(),
            p_cash: p_prop.cash,
        }
    };

    // Clear the cell-side state — once the base task takes ownership,
    // re-entering the same proposal on the cell would be a bug. Any
    // failure result from base will be communicated via the
    // base→cell-issued `onTradeResults` packet directly.
    clear_trade_state(entity_id, partner_entity_id, space_mgr);

    if let Err(e) = tx
        .send(CellToBaseMsg::ExecuteTrade {
            entity_id,
            player_id: snapshot.my_pid,
            partner_entity_id: partner_entity_id as u32,
            partner_player_id: snapshot.p_pid,
            p1_item_instance_ids: snapshot.my_items,
            p1_cash: snapshot.my_cash,
            p2_item_instance_ids: snapshot.p_items,
            p2_cash: snapshot.p_cash,
        })
        .await
    {
        // The base task is down — we already cleared the cell state, so
        // the players' next ack from base will time out and their UI
        // will linger. Surface loudly so operators can correlate.
        tracing::error!(
            entity_id,
            partner_entity_id,
            error = %e,
            "request_execute_trade: cell→base channel closed mid-handoff — \
             trade was Locked&Confirmed on both sides but never committed",
        );
    } else {
        tracing::info!(
            entity_id,
            partner_entity_id,
            p1_cash = snapshot.my_cash,
            p2_cash = snapshot.p_cash,
            "trade execute requested → base"
        );
    }
}

struct ExecuteTradeSnapshot {
    my_pid: i32,
    p_pid: i32,
    my_items: Vec<i32>,
    my_cash: i32,
    p_items: Vec<i32>,
    p_cash: i32,
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::make_space_manager;
    use cimmeria_entity::trade::TradeItem;

    fn make_two_players(mgr: &mut SpaceManager, a: u32, b: u32, dist: f32) {
        mgr.create_entity(a, "Agnos", [0.0, 0.0, 0.0], [0.0; 3])
            .unwrap();
        mgr.create_entity(b, "Agnos", [dist, 0.0, 0.0], [0.0; 3])
            .unwrap();
        // Both must be `is_player` and have player_ids for trade to fully
        // exercise the commit path.
        if let Some(e) = mgr.get_entity_mut(a) {
            e.is_player = true;
            e.player_id = Some(1000);
        }
        if let Some(e) = mgr.get_entity_mut(b) {
            e.is_player = true;
            e.player_id = Some(2000);
        }
    }

    fn build_trade_request_args(partner: i32, p: &TradeProposal) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(&partner.to_le_bytes());
        p.serialize_local(&mut buf);
        buf
    }

    #[tokio::test]
    async fn trade_request_opens_session_on_both_sides() {
        let mut mgr = make_space_manager();
        make_two_players(&mut mgr, 1, 2, 2.0);

        let (tx, mut rx) = mpsc::channel(16);
        let args = build_trade_request_args(
            2,
            &TradeProposal {
                version: 1,
                items: vec![],
                cash: 0,
                lock_state: ETRADELOCKSTATE_NONE,
            },
        );
        assert!(dispatch(1, TRADE_REQUEST, &args, &tx, &mut mgr).await);
        // Both sides should now be wired into the session.
        assert_eq!(mgr.get_entity(1).unwrap().trade_partner_entity_id, Some(2));
        assert_eq!(mgr.get_entity(2).unwrap().trade_partner_entity_id, Some(1));
        // And both clients should have received an onTradeState (one method
        // call per side).
        let mut on_state_count = 0;
        while let Ok(msg) = rx.try_recv() {
            if let CellToBaseMsg::EntityMethodCall { method_index, .. } = msg {
                if method_index == ON_TRADE_STATE {
                    on_state_count += 1;
                }
            }
        }
        assert_eq!(on_state_count, 2, "onTradeState must fire to both players");
    }

    #[tokio::test]
    async fn trade_request_too_far_rejects_with_cancelled_to_caller_only() {
        let mut mgr = make_space_manager();
        make_two_players(&mut mgr, 1, 2, 100.0); // way outside MAX_INTERACT_DISTANCE=5

        let (tx, mut rx) = mpsc::channel(8);
        let args = build_trade_request_args(2, &TradeProposal::default());
        dispatch(1, TRADE_REQUEST, &args, &tx, &mut mgr).await;
        // No session opened.
        assert!(mgr.get_entity(1).unwrap().trade_partner_entity_id.is_none());
        assert!(mgr.get_entity(2).unwrap().trade_partner_entity_id.is_none());
        // Only the caller gets onTradeResults(Cancelled).
        let mut events = Vec::new();
        while let Ok(msg) = rx.try_recv() {
            events.push(msg);
        }
        assert_eq!(events.len(), 1, "out-of-range rejects with one message");
        match &events[0] {
            CellToBaseMsg::EntityMethodCall {
                entity_id,
                method_index,
                args,
            } => {
                assert_eq!(*entity_id, 1);
                assert_eq!(*method_index, ON_TRADE_RESULTS);
                // INT32 result, expect Cancelled (2)
                assert_eq!(&args[4..8], &ETRADERESULTS_CANCELLED.to_le_bytes());
            }
            other => panic!("expected EntityMethodCall, got {other:?}"),
        }
    }

    /// TRAP #3 regression guard: QA 0.8384 client never sends `tradeRequest` —
    /// it jumps straight to `tradeUpdateProposal`. The handler must
    /// implicitly open the session.
    #[tokio::test]
    async fn trade_update_proposal_without_prior_request_begins_trading() {
        let mut mgr = make_space_manager();
        make_two_players(&mut mgr, 1, 2, 2.0);

        let (tx, _rx) = mpsc::channel(16);
        let proposal = TradeProposal {
            version: 1,
            items: vec![],
            cash: 0,
            lock_state: ETRADELOCKSTATE_NONE,
        };
        let args = build_trade_request_args(2, &proposal); // same layout — int32 partner + LocalTradeProposal

        // Pre-condition: nobody is trading.
        assert!(mgr.get_entity(1).unwrap().trade_partner_entity_id.is_none());

        // QA-client path: send TRADE_UPDATE_PROPOSAL with no open session.
        // The handler MUST open the session itself (per Python
        // SGWPlayer.py:1785-1790 workaround). Without this, every QA
        // client trade attempt silently no-ops.
        let handled = dispatch(1, TRADE_UPDATE_PROPOSAL, &args, &tx, &mut mgr).await;
        assert!(handled);

        // The session is now open on both sides — that's the assertion that
        // distinguishes "QA workaround applied" from "method silently rejected
        // because no session existed."
        assert_eq!(
            mgr.get_entity(1).unwrap().trade_partner_entity_id,
            Some(2),
            "tradeUpdateProposal with no prior tradeRequest MUST implicitly \
             begin a session (QA 0.8384 client never sends tradeRequest — \
             see SGWPlayer.py:1785-1790 workaround comment)"
        );
        assert_eq!(mgr.get_entity(2).unwrap().trade_partner_entity_id, Some(1));
    }

    #[tokio::test]
    async fn lock_state_progression_none_to_locked_to_confirmed() {
        let mut mgr = make_space_manager();
        make_two_players(&mut mgr, 1, 2, 2.0);

        // Open a session with the standard tradeRequest path.
        let (tx, _rx) = mpsc::channel(64);
        let proposal = TradeProposal {
            version: 1,
            items: vec![],
            cash: 0,
            lock_state: ETRADELOCKSTATE_NONE,
        };
        dispatch(
            1,
            TRADE_REQUEST,
            &build_trade_request_args(2, &proposal),
            &tx,
            &mut mgr,
        )
        .await;
        // Partner pushes their own initial proposal too, so both
        // versions are at 1 — otherwise the "stale partner view"
        // downgrade in tradeLockState rejects our test lock attempts.
        dispatch(
            2,
            TRADE_UPDATE_PROPOSAL,
            &build_trade_request_args(
                1,
                &TradeProposal {
                    version: 1,
                    ..Default::default()
                },
            ),
            &tx,
            &mut mgr,
        )
        .await;
        // Each side starts with version=1, lock = None.
        assert_eq!(
            mgr.get_entity(1)
                .unwrap()
                .trade_proposal
                .as_ref()
                .unwrap()
                .version,
            1
        );
        assert_eq!(
            mgr.get_entity(2)
                .unwrap()
                .trade_proposal
                .as_ref()
                .unwrap()
                .version,
            1
        );

        // Send tradeLockState(local=1, remote=1, Locked) from entity 1.
        let mut args = Vec::new();
        args.extend_from_slice(&1i32.to_le_bytes()); // local_version
        args.extend_from_slice(&1i32.to_le_bytes()); // remote_version
        args.push(ETRADELOCKSTATE_LOCKED as u8);
        dispatch(1, TRADE_LOCK_STATE, &args, &tx, &mut mgr).await;
        assert_eq!(
            mgr.get_entity(1)
                .unwrap()
                .trade_proposal
                .as_ref()
                .unwrap()
                .lock_state,
            ETRADELOCKSTATE_LOCKED
        );
        // Partner unaffected.
        assert_eq!(
            mgr.get_entity(2)
                .unwrap()
                .trade_proposal
                .as_ref()
                .unwrap()
                .lock_state,
            ETRADELOCKSTATE_NONE
        );

        // Stale-version reject: replay with local_version=999.
        let mut bad = Vec::new();
        bad.extend_from_slice(&999i32.to_le_bytes());
        bad.extend_from_slice(&1i32.to_le_bytes());
        bad.push(ETRADELOCKSTATE_LOCKED_AND_CONFIRMED as u8);
        dispatch(1, TRADE_LOCK_STATE, &bad, &tx, &mut mgr).await;
        // Did NOT change to LockedAndConfirmed.
        assert_eq!(
            mgr.get_entity(1)
                .unwrap()
                .trade_proposal
                .as_ref()
                .unwrap()
                .lock_state,
            ETRADELOCKSTATE_LOCKED
        );

        // Stale-partner downgrade: send with remote_version=0 (we know
        // it's actually 1). Lock=Locked should silently downgrade to None.
        let mut downgrade_args = Vec::new();
        downgrade_args.extend_from_slice(&1i32.to_le_bytes()); // local OK
        downgrade_args.extend_from_slice(&0i32.to_le_bytes()); // remote stale
        downgrade_args.push(ETRADELOCKSTATE_LOCKED as u8);
        dispatch(1, TRADE_LOCK_STATE, &downgrade_args, &tx, &mut mgr).await;
        assert_eq!(
            mgr.get_entity(1)
                .unwrap()
                .trade_proposal
                .as_ref()
                .unwrap()
                .lock_state,
            ETRADELOCKSTATE_NONE,
            "stale remote_version must silently downgrade Locked → None"
        );
    }

    #[tokio::test]
    async fn cancel_clears_state_on_both_sides_and_sends_completed_to_both() {
        let mut mgr = make_space_manager();
        make_two_players(&mut mgr, 1, 2, 2.0);

        let (tx, mut rx) = mpsc::channel(64);
        dispatch(
            1,
            TRADE_REQUEST,
            &build_trade_request_args(
                2,
                &TradeProposal {
                    version: 1,
                    ..Default::default()
                },
            ),
            &tx,
            &mut mgr,
        )
        .await;
        // Drain the initial onTradeState packets.
        while rx.try_recv().is_ok() {}

        let mut cancel_args = Vec::new();
        cancel_args.extend_from_slice(&2i32.to_le_bytes());
        dispatch(1, TRADE_REQUEST_CANCEL, &cancel_args, &tx, &mut mgr).await;

        // State cleared on both sides.
        assert!(mgr.get_entity(1).unwrap().trade_partner_entity_id.is_none());
        assert!(mgr.get_entity(1).unwrap().trade_proposal.is_none());
        assert!(mgr.get_entity(2).unwrap().trade_partner_entity_id.is_none());
        assert!(mgr.get_entity(2).unwrap().trade_proposal.is_none());

        // Both clients get onTradeResults(Completed).
        // TRAP #2: user-initiated cancel sends Completed (1), NOT Cancelled.
        let mut results = Vec::new();
        while let Ok(msg) = rx.try_recv() {
            if let CellToBaseMsg::EntityMethodCall {
                method_index, args, ..
            } = msg
            {
                if method_index == ON_TRADE_RESULTS {
                    let result = i32::from_le_bytes([args[4], args[5], args[6], args[7]]);
                    results.push(result);
                }
            }
        }
        assert_eq!(results.len(), 2, "both players must receive onTradeResults");
        assert!(
            results.iter().all(|&r| r == ETRADERESULTS_COMPLETED),
            "user cancel MUST send Completed (1), not Cancelled (2) — \
             see Trade.py:225-228. Got results: {results:?}"
        );
    }

    #[tokio::test]
    async fn distance_break_during_update_auto_cancels() {
        let mut mgr = make_space_manager();
        make_two_players(&mut mgr, 1, 2, 2.0);

        let (tx, mut rx) = mpsc::channel(64);
        dispatch(
            1,
            TRADE_REQUEST,
            &build_trade_request_args(
                2,
                &TradeProposal {
                    version: 1,
                    ..Default::default()
                },
            ),
            &tx,
            &mut mgr,
        )
        .await;
        while rx.try_recv().is_ok() {}

        // Move player 2 far away.
        if let Some(e) = mgr.get_entity_mut(2) {
            e.position = cimmeria_common::Vector3::new(1000.0, 0.0, 0.0);
        }

        // Next tradeUpdateProposal — must auto-cancel.
        let next = TradeProposal {
            version: 2,
            items: vec![TradeItem {
                instance_id: 42,
                slot_id: 0,
            }],
            cash: 0,
            lock_state: ETRADELOCKSTATE_NONE,
        };
        dispatch(
            1,
            TRADE_UPDATE_PROPOSAL,
            &build_trade_request_args(2, &next),
            &tx,
            &mut mgr,
        )
        .await;

        // Both sides cleared, both clients got onTradeResults(Cancelled=2)
        // (distance break is NOT a user-initiated cancel).
        assert!(mgr.get_entity(1).unwrap().trade_partner_entity_id.is_none());
        let mut last_results = Vec::new();
        while let Ok(msg) = rx.try_recv() {
            if let CellToBaseMsg::EntityMethodCall {
                method_index, args, ..
            } = msg
            {
                if method_index == ON_TRADE_RESULTS {
                    let result = i32::from_le_bytes([args[4], args[5], args[6], args[7]]);
                    last_results.push(result);
                }
            }
        }
        assert_eq!(last_results.len(), 2);
        assert!(
            last_results.iter().all(|&r| r == ETRADERESULTS_CANCELLED),
            "distance-break MUST send Cancelled (2), not Completed (1). Got: {last_results:?}"
        );
    }

    #[tokio::test]
    async fn proposal_version_must_increment_by_one() {
        let mut mgr = make_space_manager();
        make_two_players(&mut mgr, 1, 2, 2.0);

        let (tx, _rx) = mpsc::channel(64);
        dispatch(
            1,
            TRADE_REQUEST,
            &build_trade_request_args(
                2,
                &TradeProposal {
                    version: 1,
                    ..Default::default()
                },
            ),
            &tx,
            &mut mgr,
        )
        .await;
        assert_eq!(
            mgr.get_entity(1)
                .unwrap()
                .trade_proposal
                .as_ref()
                .unwrap()
                .version,
            1
        );

        // version=5 instead of 2 → rejected → cancels the session.
        let bad = TradeProposal {
            version: 5,
            items: vec![],
            cash: 0,
            lock_state: ETRADELOCKSTATE_NONE,
        };
        dispatch(
            1,
            TRADE_UPDATE_PROPOSAL,
            &build_trade_request_args(2, &bad),
            &tx,
            &mut mgr,
        )
        .await;
        // Session is torn down.
        assert!(mgr.get_entity(1).unwrap().trade_partner_entity_id.is_none());
    }

    #[tokio::test]
    async fn cannot_trade_with_self() {
        let mut mgr = make_space_manager();
        make_two_players(&mut mgr, 1, 2, 2.0);
        let (tx, _rx) = mpsc::channel(8);
        let args = build_trade_request_args(1, &TradeProposal::default()); // partner = self
        dispatch(1, TRADE_REQUEST, &args, &tx, &mut mgr).await;
        assert!(mgr.get_entity(1).unwrap().trade_partner_entity_id.is_none());
    }

    #[tokio::test]
    async fn cannot_trade_with_npc() {
        let mut mgr = make_space_manager();
        make_two_players(&mut mgr, 1, 2, 2.0);
        // Demote entity 2 to a non-player.
        if let Some(e) = mgr.get_entity_mut(2) {
            e.is_player = false;
        }
        let (tx, _rx) = mpsc::channel(8);
        let args = build_trade_request_args(2, &TradeProposal::default());
        dispatch(1, TRADE_REQUEST, &args, &tx, &mut mgr).await;
        assert!(mgr.get_entity(1).unwrap().trade_partner_entity_id.is_none());
    }

    #[tokio::test]
    async fn both_confirm_triggers_execute_trade_hand_off() {
        let mut mgr = make_space_manager();
        make_two_players(&mut mgr, 1, 2, 2.0);

        let (tx, mut rx) = mpsc::channel(64);
        // Open session.
        dispatch(
            1,
            TRADE_REQUEST,
            &build_trade_request_args(
                2,
                &TradeProposal {
                    version: 1,
                    ..Default::default()
                },
            ),
            &tx,
            &mut mgr,
        )
        .await;
        // Partner pushes a proposal too so both versions are at 1.
        dispatch(
            2,
            TRADE_UPDATE_PROPOSAL,
            &build_trade_request_args(
                1,
                &TradeProposal {
                    version: 1,
                    ..Default::default()
                },
            ),
            &tx,
            &mut mgr,
        )
        .await;
        while rx.try_recv().is_ok() {} // drain

        // Both sides walk: None → Locked → LockedAndConfirmed.
        for actor in [1u32, 2u32] {
            let mut args = Vec::new();
            args.extend_from_slice(&1i32.to_le_bytes());
            args.extend_from_slice(&1i32.to_le_bytes());
            args.push(ETRADELOCKSTATE_LOCKED as u8);
            dispatch(actor, TRADE_LOCK_STATE, &args, &tx, &mut mgr).await;
        }
        for actor in [1u32, 2u32] {
            let mut args = Vec::new();
            args.extend_from_slice(&1i32.to_le_bytes());
            args.extend_from_slice(&1i32.to_le_bytes());
            args.push(ETRADELOCKSTATE_LOCKED_AND_CONFIRMED as u8);
            dispatch(actor, TRADE_LOCK_STATE, &args, &tx, &mut mgr).await;
        }

        // ExecuteTrade should have been emitted exactly once.
        let mut executes = 0;
        while let Ok(msg) = rx.try_recv() {
            if matches!(msg, CellToBaseMsg::ExecuteTrade { .. }) {
                executes += 1;
            }
        }
        assert_eq!(
            executes, 1,
            "both-confirmed should trigger exactly one ExecuteTrade hand-off"
        );

        // Cell-side state is cleared regardless of base outcome.
        assert!(mgr.get_entity(1).unwrap().trade_partner_entity_id.is_none());
        assert!(mgr.get_entity(2).unwrap().trade_partner_entity_id.is_none());
    }

    /// Regression guard for the security review on PR #438.
    ///
    /// `request_execute_trade` is the very last cell-side checkpoint
    /// before the swap commits at base. The earlier distance checks
    /// (`tradeUpdateProposal`, `tradeLockState`) fire on every inbound
    /// message — but a small window exists between the `tradeLockState`
    /// handler's distance check (top of the handler) and the actual
    /// `request_execute_trade` call (bottom): any future refactor that
    /// introduces an `.await` between those points opens the gap for
    /// one player to walk out of range while the second confirmation
    /// is being applied. The handoff must independently verify, not
    /// trust the earlier check.
    ///
    /// We exercise the **handoff entry point directly** (not via
    /// `dispatch`) because the `tradeLockState` handler's own distance
    /// check would otherwise intercept the out-of-range mutation
    /// before `request_execute_trade` ever ran — that would test the
    /// handler check, not the handoff check. The point of this guard
    /// is the *handoff* check.
    ///
    /// This test:
    /// 1. Manually sets up both sides as if they had reached
    ///    `LockedAndConfirmed` (the precondition that normally calls
    ///    `request_execute_trade`).
    /// 2. Moves player 2 out of range.
    /// 3. Calls `request_execute_trade` directly.
    /// 4. Asserts no `ExecuteTrade` was queued, both sides got
    ///    `onTradeResults(Cancelled)`, and both sides have their
    ///    trade state cleared.
    ///
    /// Revert-verifier: removing the `partners_in_range` block at the
    /// top of `request_execute_trade` causes an `ExecuteTrade` to
    /// appear in the channel even though the players walked apart.
    #[tokio::test]
    async fn execute_handoff_rechecks_distance_and_cancels_on_break() {
        let mut mgr = make_space_manager();
        make_two_players(&mut mgr, 1, 2, 2.0);

        // Hand-roll the "both sides at LockedAndConfirmed" precondition
        // without going through the lock-state handler (which would
        // intercept our position mutation via its own distance check
        // and we want to exercise the handoff check, not the handler).
        if let Some(e) = mgr.get_entity_mut(1) {
            e.trade_partner_entity_id = Some(2);
            e.trade_proposal = Some(TradeProposal {
                version: 1,
                items: vec![],
                cash: 0,
                lock_state: ETRADELOCKSTATE_LOCKED_AND_CONFIRMED,
            });
        }
        if let Some(e) = mgr.get_entity_mut(2) {
            e.trade_partner_entity_id = Some(1);
            e.trade_proposal = Some(TradeProposal {
                version: 1,
                items: vec![],
                cash: 0,
                lock_state: ETRADELOCKSTATE_LOCKED_AND_CONFIRMED,
            });
        }

        // Now walk player 2 far out of range — the proximity invariant
        // is broken at exactly the moment the handoff is about to run.
        if let Some(e) = mgr.get_entity_mut(2) {
            e.position = cimmeria_common::Vector3::new(1000.0, 0.0, 0.0);
        }

        let (tx, mut rx) = mpsc::channel(64);
        // Drive the handoff path directly — this is the function the
        // lock-state handler calls when both sides reach
        // LockedAndConfirmed.
        request_execute_trade(1, 2, &tx, &mut mgr).await;

        // Collect what landed in the channel.
        let mut executes = 0;
        let mut results: Vec<(u32, i32)> = Vec::new();
        while let Ok(msg) = rx.try_recv() {
            match msg {
                CellToBaseMsg::ExecuteTrade { .. } => executes += 1,
                CellToBaseMsg::EntityMethodCall {
                    entity_id,
                    method_index,
                    args,
                } if method_index == ON_TRADE_RESULTS => {
                    let result = i32::from_le_bytes([args[4], args[5], args[6], args[7]]);
                    results.push((entity_id, result));
                }
                _ => {}
            }
        }

        assert_eq!(
            executes, 0,
            "request_execute_trade MUST NOT hand off to base when \
             partners are out of range at the handoff. Saw ExecuteTrade \
             in the channel — distance recheck missing."
        );
        // Both sides see Cancelled, not Completed: distance-break is a
        // fault path, same convention as the disconnect cancel.
        assert_eq!(results.len(), 2, "both sides must receive onTradeResults");
        assert!(
            results.iter().all(|(_, r)| *r == ETRADERESULTS_CANCELLED),
            "distance-break at handoff must send Cancelled (2), \
             not Completed (1). Got: {results:?}"
        );
        // Cell state is gone on both sides — same teardown as any
        // other cancel path.
        assert!(
            mgr.get_entity(1).unwrap().trade_partner_entity_id.is_none(),
            "entity 1's trade_partner_entity_id must be cleared"
        );
        assert!(
            mgr.get_entity(2).unwrap().trade_partner_entity_id.is_none(),
            "entity 2's trade_partner_entity_id must be cleared"
        );
        assert!(mgr.get_entity(1).unwrap().trade_proposal.is_none());
        assert!(mgr.get_entity(2).unwrap().trade_proposal.is_none());
    }

    #[tokio::test]
    async fn invalid_lock_state_value_is_rejected() {
        let mut mgr = make_space_manager();
        make_two_players(&mut mgr, 1, 2, 2.0);

        let (tx, _rx) = mpsc::channel(64);
        dispatch(
            1,
            TRADE_REQUEST,
            &build_trade_request_args(
                2,
                &TradeProposal {
                    version: 1,
                    ..Default::default()
                },
            ),
            &tx,
            &mut mgr,
        )
        .await;

        // Send a junk lock state (5 is outside [0, 2]).
        let mut args = Vec::new();
        args.extend_from_slice(&1i32.to_le_bytes());
        args.extend_from_slice(&1i32.to_le_bytes());
        args.push(5u8);
        dispatch(1, TRADE_LOCK_STATE, &args, &tx, &mut mgr).await;
        // State unchanged.
        assert_eq!(
            mgr.get_entity(1)
                .unwrap()
                .trade_proposal
                .as_ref()
                .unwrap()
                .lock_state,
            ETRADELOCKSTATE_NONE
        );
    }

    #[tokio::test]
    async fn disconnect_cancellation_clears_partner_state() {
        let mut mgr = make_space_manager();
        make_two_players(&mut mgr, 1, 2, 2.0);

        let (tx, mut rx) = mpsc::channel(16);
        dispatch(
            1,
            TRADE_REQUEST,
            &build_trade_request_args(
                2,
                &TradeProposal {
                    version: 1,
                    ..Default::default()
                },
            ),
            &tx,
            &mut mgr,
        )
        .await;
        while rx.try_recv().is_ok() {}

        // Simulate entity 1 disconnecting mid-trade.
        let partner = cancel_trade_on_disconnect(1, &tx, &mut mgr).await;
        assert_eq!(partner, Some(2));

        // Partner has been cleared, and was notified with Cancelled.
        assert!(mgr.get_entity(2).unwrap().trade_partner_entity_id.is_none());
        let mut got_partner_cancel = false;
        while let Ok(msg) = rx.try_recv() {
            if let CellToBaseMsg::EntityMethodCall {
                entity_id,
                method_index,
                args,
            } = msg
            {
                if entity_id == 2 && method_index == ON_TRADE_RESULTS {
                    let result = i32::from_le_bytes([args[4], args[5], args[6], args[7]]);
                    if result == ETRADERESULTS_CANCELLED {
                        got_partner_cancel = true;
                    }
                }
            }
        }
        assert!(
            got_partner_cancel,
            "partner must receive onTradeResults(Cancelled) on disconnect — \
             NOT Completed: the surviving player did not initiate this cancel"
        );
    }
}
