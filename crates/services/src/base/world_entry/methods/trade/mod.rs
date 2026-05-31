//! Base-side atomic trade execution.
//!
//! This module owns the `CellToBaseMsg::ExecuteTrade` handler — the only
//! point where items + cash physically change hands between two players.
//! Everything before this is in-memory state on the cell; everything
//! after this writes through to the DB inside a single sqlx
//! `BEGIN/COMMIT` block.
//!
//! See `crates/services/src/cell/cell_methods/player/trade.rs` for the
//! cell-side state machine that drives the hand-off.

mod execute;

#[cfg(test)]
mod tests;

pub use execute::handle_execute_trade;
