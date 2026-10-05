//! The duel plugin's handlers (SS-D1..SS-D3):
//!
//! - [`response`]: `sendDuelResponse` (CM 102), the target's answer, and the
//!   5 s countdown an accept starts;
//! - [`forfeit`]: `duelForfeit` (CM 103);
//! - [`tick`]: challenge expiry, the countdown end ([`engage`]) and the
//!   tick's safety ends, run at `TickStage::AfterGateCrossing`.
//!
//! Everything else the duel needs is the duel world half in
//! `cimmeria_cell_world::cell::duel`, re-exported here whole so these
//! modules name it by the same `super::…` paths as before the move
//! (the registry, the challenge, the end paths, the outbound sends).

pub use cimmeria_cell_world::cell::duel::*;

mod duelist_names;
mod engage;
pub mod forfeit;
pub mod response;
pub mod tick;

#[cfg(test)]
mod tests;
