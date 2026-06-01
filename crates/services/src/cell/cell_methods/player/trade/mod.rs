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
//!
//! ## Module layout
//!
//! - [`handlers`] — the 4 inbound cell-method handlers + `dispatch`.
//! - [`state`] — session lifecycle helpers (`begin_trading`, `apply_proposal`,
//!   `cancel_session`, `clear_trade_state`, `partners_in_range`, and the
//!   public `cancel_trade_on_disconnect` hook).
//! - [`wire`] — outbound `onTradeState` / `onTradeResults` serializers
//!   and the `stub_inv_items_for` info-leak-mitigating stub builder.
//! - [`handoff`] — `request_execute_trade` (final cell-side checkpoint
//!   before the base-side atomic commit).
//!
//! Public surface is re-exported here so external callers can keep using
//! `crate::cell::cell_methods::player::trade::{dispatch, cancel_trade_on_disconnect}`
//! without knowing about the internal split.

mod handlers;
mod handoff;
mod state;
mod wire;

#[cfg(test)]
mod tests;

pub use handlers::dispatch;
pub use state::cancel_trade_on_disconnect;

/// Mirror of `python/common/Constants.py: MAX_INTERACT_DISTANCE = 5`.
/// Trade is gated by the same range as vendor / dialog interactions.
const MAX_INTERACT_DISTANCE: f32 = 5.0;
