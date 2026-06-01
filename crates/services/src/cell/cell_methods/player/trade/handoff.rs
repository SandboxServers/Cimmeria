//! Cell→base handoff for the atomic commit.
//!
//! When both sides have reached `LockedAndConfirmed`, the cell snapshots
//! the proposals, runs a last-mile distance recheck, clears its own
//! state, and sends `CellToBaseMsg::ExecuteTrade`. From that point on
//! the base layer owns the DB transaction and the `onTradeResults`
//! emission to both players.
//!
//! The distance recheck at the top of [`request_execute_trade`] is the
//! final TOCTOU guard: any `.await` between the lock-state handler's
//! check and this entry point is a potential window. We re-check at
//! the *handoff* boundary, not the entry-side boundary.

use cimmeria_entity::trade::ETRADERESULTS_CANCELLED;
use tokio::sync::mpsc;

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

use super::state::{cancel_session, clear_trade_state, partners_in_range};

/// Both sides reached `LockedAndConfirmed` — kick the execution off to
/// the base layer (which owns the DB). The cell clears the in-memory
/// session state immediately; base will fire `onTradeResults` to both
/// clients itself, with the appropriate per-side result code (Completed
/// on success, NoLocalCash/NoLocalSpace/NoRemoteCash/NoRemoteSpace on a
/// validation failure inside the FOR UPDATE tx).
pub(super) async fn request_execute_trade(
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
