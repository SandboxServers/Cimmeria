//! Inbound cell-method handlers for the trade flow.
//!
//! Wire methods 104-107. The dispatcher routes to one of these four
//! based on the method index; each handler validates the payload then
//! delegates to the state machine in [`super::state`] and the outbound
//! serializers in [`super::wire`].

use cimmeria_entity::trade::{
    TradeProposal, ETRADELOCKSTATE_LOCKED, ETRADELOCKSTATE_LOCKED_AND_CONFIRMED,
    ETRADELOCKSTATE_NONE, ETRADERESULTS_CANCELLED, ETRADERESULTS_COMPLETED,
};
use tokio::sync::mpsc;

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

use super::super::constants::*;
use super::handoff::request_execute_trade;
use super::state::{apply_proposal, begin_trading, cancel_session, partners_in_range};
use super::wire::{send_on_trade_results, send_on_trade_state_to_both};

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
