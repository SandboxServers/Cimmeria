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
//! - [`handoff`] — `request_execute_trade` (final cell-side checkpoint
//!   before the base-side atomic commit).
//!
//! The session state machine (`state`) and the outbound serializers (`wire`)
//! are in `cell::trade`, one layer below these handlers,
//! because gate travel and the GM space transfer cancel an open trade on
//! departure (`docs/architecture/services-crate-split.md` §2H). They are
//! imported here, so the handlers still reach them as `super::state` and
//! `super::wire`.
//!
//! Public surface is re-exported here so external callers can keep using
//! `crate::cell::cell_methods::player::trade::{dispatch, cancel_trade_on_disconnect}`
//! without knowing about the internal split.

mod handlers;
mod handoff;

use crate::cell::trade::{state, wire};

#[cfg(test)]
mod tests;

pub use handlers::dispatch;
pub use state::cancel_trade_on_disconnect;
