//! Live-DB chain-replay regression guards.
//!
//! Each test loads a specific seeded `content_*` chain from the database
//! through the same `build_chains_from_rows` pipeline that the cell service
//! uses at startup, registers it in a fresh `ChainEngine`, and fires
//! synthetic events through `resolve_event` to assert the chain matches
//! (or doesn't) under specific `ExecutionContext` shapes.
//!
//! Loading from DB rather than hand-constructing the chain in Rust is
//! deliberate: the whole point is to catch silent drift in the SQL seed
//! (e.g. someone removes a `mission_status` condition that was added to fix
//! a bug). A pure-Rust replica would let that drift pass.
//!
//! Skip cleanly when DATABASE_URL is unset.
//!
//! Files are organised by mission family — each sibling module pins the
//! chains for one mission's branches and edges so a regression surfaces
//! near the seed it touches. Two modules are organised by *action verb*
//! instead, because the risk they guard is the executor arm rather than
//! any one mission's wiring: [`sgc_w1_move_entity`] and [`grant_xp`].
//! Those two also run the resolved actions through
//! `executor::execute_actions` and assert on the resulting
//! `CellToBaseMsg` traffic — a resolve-only test cannot tell a wired
//! executor arm from the `other =>` catch-all.

mod cover_demo;
mod grant_xp;
mod mission_1562;
mod mission_622;
mod mission_638;
mod mission_639;
mod mission_641;
mod mission_687;
mod mission_688;
mod sgc_w1_move_entity;
