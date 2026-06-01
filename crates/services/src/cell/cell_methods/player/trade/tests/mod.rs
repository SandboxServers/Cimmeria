//! Trade-flow unit tests.
//!
//! Grouped by primary surface under test:
//! - [`handlers`] — happy-path dispatch + validation rejects for the
//!   four inbound methods.
//! - [`lock_state`] — `tradeLockState` (107) progression + downgrade
//!   + value-range rejection.
//! - [`handoff`] — the cell→base atomic-commit handoff + the
//!   final-mile distance recheck regression guard.
//! - [`wire`] — wire-byte fixtures (currently the `stub_inv_items_for`
//!   info-leak sentinel guard).

use cimmeria_entity::trade::TradeProposal;

use crate::cell::space_manager::SpaceManager;

mod handlers;
mod handoff;
mod lock_state;
mod wire;

/// Set up two players in the same space, separated by `dist` along the
/// X axis. Both are flagged `is_player = true` and given player IDs so
/// the full commit path can be exercised end-to-end.
pub(super) fn make_two_players(mgr: &mut SpaceManager, a: u32, b: u32, dist: f32) {
    mgr.create_entity(a, "Agnos", [0.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    mgr.create_entity(b, "Agnos", [dist, 0.0, 0.0], [0.0; 3])
        .unwrap();
    if let Some(e) = mgr.get_entity_mut(a) {
        e.is_player = true;
        e.player_id = Some(1000);
    }
    if let Some(e) = mgr.get_entity_mut(b) {
        e.is_player = true;
        e.player_id = Some(2000);
    }
}

/// Build the inbound wire payload for `tradeRequest` / `tradeUpdateProposal`:
/// INT32 partner entity ID followed by a serialized `LocalTradeProposal`.
pub(super) fn build_trade_request_args(partner: i32, p: &TradeProposal) -> Vec<u8> {
    let mut buf = Vec::new();
    buf.extend_from_slice(&partner.to_le_bytes());
    p.serialize_local(&mut buf);
    buf
}
