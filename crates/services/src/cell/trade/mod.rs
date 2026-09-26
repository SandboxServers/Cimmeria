//! Player-to-player trade: the session state machine and its outbound wire.
//!
//! The four inbound cell-method handlers (`tradeRequest`,
//! `tradeRequestCancel`, `tradeUpdateProposal`, `tradeLockState`) and the
//! base-side commit handoff are in `cell_methods::player::trade`. What they
//! share with the departure paths sits here, one layer below them: gate
//! travel and the GM space transfer both close an open session with
//! [`cancel_trade_on_disconnect`] before the player leaves the space
//! (`docs/architecture/services-crate-split.md` §2H). It belongs to
//! `cimmeria-cell-interactions`, beside gate travel and the space transfer,
//! but is not under `cell::interactions`: the `interactions.log` file layer
//! keeps that whole tree, and the trade rows have never been in that file.
//!
//! - [`state`] — session lifecycle helpers (`begin_trading`, `apply_proposal`,
//!   `cancel_session`, `clear_trade_state`, `partners_in_range`, and the
//!   public `cancel_trade_on_disconnect` hook).
//! - [`wire`] — outbound `onTradeState` / `onTradeResults` serializers
//!   and the `stub_inv_items_for` info-leak-mitigating stub builder.

pub(crate) mod state;
pub(crate) mod wire;

#[cfg(test)]
mod tests;

pub use state::cancel_trade_on_disconnect;

/// Mirror of `python/common/Constants.py: MAX_INTERACT_DISTANCE = 5`.
/// Trade is gated by the same range as vendor / dialog interactions.
const MAX_INTERACT_DISTANCE: f32 = 5.0;
