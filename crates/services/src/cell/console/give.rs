//! Player-grant console commands: `.givecash`, `.givexp`.
//!
//! Both route through the same base-side grant sinks the native `gmGiveCash`/
//! `gmGiveXp` methods use (`crates/services/src/cell/cell_methods/gm/give.rs`
//! -> `CellToBaseMsg::GrantCash`/`GrantXP`) — but unlike those caller-grants-
//! to-self native paths, `.givecash`/`.givexp` grant to a *selected* target
//! while the calling GM receives the feedback line. This caller/subject split
//! is exactly what `gm_feedback_to: Option<u32>` exists for (see
//! `CellToBaseMsg::GrantCash`/`GrantXP`'s doc comments): passing
//! `Some(caller_id)` here tells the base to send the definitive post-commit
//! feedback line to the caller, never to the target.
//!
//! No optimistic "requested" feedback is sent from here — the base sends the
//! real outcome once the DB write actually commits.
//!
//! Legacy reference: `deprecated/python/cell/commands/Player.py::giveCash`/
//! `giveExperience`.

use tokio::sync::mpsc;

use super::send_gm_feedback;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

/// `.givecash <amount>` — grant naquadah to the selected target.
///
/// Legacy `giveCash` has no lower bound on `amount`; the current native
/// `gmGiveCash` rejects `<= 0` (a no-op grant at best, a footgun for an
/// accidental balance decrease at worst) — D02 keeps that bound rather than
/// reproducing the legacy gap.
pub(super) async fn give_cash(
    caller_id: u32,
    target: u32,
    args: &[&str],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let Some(amount) = super::parse_i32(caller_id, args, 0, "amount", tx).await else {
        return;
    };
    if amount <= 0 {
        send_gm_feedback(caller_id, "givecash: amount must be positive", tx).await;
        return;
    }
    let Some(player_id) = space_mgr.get_entity(target).and_then(|e| e.player_id) else {
        send_gm_feedback(caller_id, "givecash: target has no player id", tx).await;
        return;
    };
    tracing::info!(caller_id, target, player_id, amount, "GM .givecash");
    let _ = tx
        .send(CellToBaseMsg::GrantCash {
            entity_id: target,
            player_id,
            amount,
            gm_feedback_to: Some(caller_id),
        })
        .await;
}

/// `.givexp <amount>` — grant experience to the selected target.
///
/// `xp_amount` is `u64` on the wire message; `amount > 0` is confirmed before
/// the cast so a negative `i32` can't wrap into an absurd unsigned grant
/// (mirrors the native `gmGiveXp` ordering).
pub(super) async fn give_xp(
    caller_id: u32,
    target: u32,
    args: &[&str],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let Some(amount) = super::parse_i32(caller_id, args, 0, "amount", tx).await else {
        return;
    };
    if amount <= 0 {
        send_gm_feedback(caller_id, "givexp: amount must be positive", tx).await;
        return;
    }
    if space_mgr
        .get_entity(target)
        .and_then(|e| e.player_id)
        .is_none()
    {
        send_gm_feedback(caller_id, "givexp: target has no player id", tx).await;
        return;
    }
    tracing::info!(caller_id, target, amount, "GM .givexp");
    let _ = tx
        .send(CellToBaseMsg::GrantXP {
            entity_id: target,
            xp_amount: amount as u64,
            gm_feedback_to: Some(caller_id),
        })
        .await;
}
