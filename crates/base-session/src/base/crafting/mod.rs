//! Crafting subsystem — BaseApp side.
//!
//! Crafting state and inventory are base-owned, so every crafting verb is
//! decided here (`docs/analysis/crafting/work-packets.md`, "Contract fixed
//! by this ledger"):
//!
//! - [`request`]: the entry point for `CellToBaseMsg::Crafting`. It logs the
//!   request and routes each verb to its handler.
//! - [`spend`]: `spendAppliedSciencePoints`, learning a discipline.
//! - [`research`] and [`reverse_engineer`]: the `research` and
//!   `reverseEngineer` inductions; [`induction_verb`] and [`item_lookup`]
//!   hold what an induction verb does around its own rule.
//! - [`alloy`]: `alloying`, one component plus elementary components into
//!   an alloy, as an induction.
//! - [`feedback`]: the rejection path. Every refused request gets a visible
//!   text line.
//! - [`sync`]: the owner-only pushes of crafting state to the client (136,
//!   138, 139, the ASP property) and the login sync.
//! - [`persistence`]: the `CraftingState` load/save round-trip between
//!   `sgw_player` + `sgw_player_discipline_expertise` and
//!   `cimmeria_entity::crafting::CraftingState`, with the starting paradigm
//!   levels applied on load.
//! - [`telemetry`]: the identity every crafting event carries, the three
//!   counters (`crafting_requests_total`, `crafting_rejections_total`,
//!   `crafting_jobs_total`), the induction job ids and the checked sends.
//! - [`handlers`]: the GM grants (`gmGiveExpertise`,
//!   `gmGiveAppliedSciencePoints`).
//! - [`gate`]: the station gate every craft-family verb passes first: a
//!   station in reach, a covering Field Crafting Tool, or "craft anywhere".
//! - [`tools`]: the Field Crafting Tool rule.
//! - [`options`]: `onUpdateCraftingOptions` (140) and the per-session
//!   stations, tools and "craft anywhere" behind it.
//! - [`allcraft`]: the GM `.allcraft` grant.
//! - [`gm_grant`]: the GM `.craftkit` and `.learnblueprint` grants.
//! - [`inventory_locks`]: the advisory locks every crafting write takes
//!   on a player's inventory before any row.
//! - [`session`]: the induction engine. Each player runs one induction at
//!   a time, with up to [`session::MAX_INDUCTIONS`] held; the client's bar
//!   is the type-16 timer, and the job runs when it expires.
//! - [`transaction`]: the one database transaction an item verb's job
//!   runs at completion (consume inputs, grant products, adjust
//!   expertise), and the client updates after it.
//! - [`rng`]: the injectable RNG the rolling verbs use.
//! - [`item_use`]: using a Blueprint item or a Racial Paradigm Guide.
//! - [`craft`]: `craft`, making a known blueprint's product from one of
//!   its component sets.
//! - [`respec`]: `.respeccraft` and `respecCrafting` (100), the two-step
//!   crafting respec.

pub mod allcraft;
pub mod alloy;
pub mod craft;
pub mod feedback;
pub mod gate;
pub mod gm_grant;
pub mod handlers;
pub mod induction_verb;
pub mod inventory_locks;
pub mod item_lookup;
pub mod item_use;
pub mod options;
pub mod persistence;
pub mod request;
pub mod research;
pub mod respec;
pub mod reverse_engineer;
pub mod rng;
pub mod session;
pub mod spend;
pub mod sync;
pub mod telemetry;
pub mod tools;
pub mod transaction;

#[cfg(test)]
mod lock_order_tests;
#[cfg(test)]
mod test_packets;
#[cfg(test)]
mod test_players;
#[cfg(test)]
mod test_verbs;
