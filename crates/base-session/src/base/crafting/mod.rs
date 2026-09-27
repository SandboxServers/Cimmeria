//! Crafting subsystem — BaseApp side.
//!
//! Crafting state and inventory are base-owned, so every crafting verb is
//! decided here (`docs/analysis/crafting/work-packets.md`, "Contract fixed
//! by this ledger"):
//!
//! - [`request`]: the entry point for `CellToBaseMsg::Crafting`. It logs the
//!   request and routes each verb; a verb with no handler yet is answered
//!   with a "not available yet" line.
//! - [`spend`]: `spendAppliedSciencePoints`, learning a discipline.
//! - [`feedback`]: the rejection path. Every refused request gets a visible
//!   text line.
//! - [`sync`]: the owner-only pushes of crafting state to the client (136,
//!   138, 139, the ASP property) and the login sync.
//! - [`persistence`]: the `CraftingState` load/save round-trip between
//!   `sgw_player` + `sgw_player_discipline_expertise` and
//!   `cimmeria_entity::crafting::CraftingState`, with the starting paradigm
//!   levels applied on load.
//! - [`telemetry`]: the identity every crafting event carries and the two
//!   counters (`crafting_requests_total`, `crafting_rejections_total`).
//! - [`handlers`]: the GM grants (`gmGiveExpertise`,
//!   `gmGiveAppliedSciencePoints`).

pub mod feedback;
pub mod handlers;
pub mod persistence;
pub mod request;
pub mod spend;
pub mod sync;
pub mod telemetry;

#[cfg(test)]
mod test_players;
