//! Crafting subsystem — BaseApp side.
//!
//! Crafting state and inventory are base-owned, so every crafting verb is
//! decided here (`docs/analysis/crafting/work-packets.md`, "Contract fixed
//! by this ledger"):
//!
//! - [`request`]: the entry point for `CellToBaseMsg::Crafting`. It logs the
//!   request and routes each verb; until a verb's packet lands, the verb is
//!   answered with a "not available yet" line.
//! - [`feedback`]: the rejection path. Every refused request gets a visible
//!   text line (D-CR14).
//! - [`sync`]: the owner-only pushes of crafting state to the client (136,
//!   138, 139, the ASP property) and the login sync (CR-03).
//! - [`persistence`]: the `CraftingState` load/save round-trip between
//!   `sgw_player` + `sgw_player_discipline_expertise` and
//!   `cimmeria_entity::crafting::CraftingState`, with the D-CR03 starting
//!   paradigm levels applied on load.
//! - [`handlers`]: the GM grants (`gmGiveExpertise`,
//!   `gmGiveAppliedSciencePoints`).

pub mod feedback;
pub mod handlers;
pub mod persistence;
pub mod request;
pub mod sync;

#[cfg(test)]
mod test_players;
