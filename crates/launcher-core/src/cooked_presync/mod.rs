//! The launcher's cooked-data presync: it fetches the server's category index
//! and bodies, stages them and commits them to the client's cache before Play.
//!
//! A presync that fails must never start the game (D-LP18), so every failure
//! here carries its step, reason and OS detail up to the engine, which blocks
//! Play and tells the player why.

pub mod apply;
pub mod fetch;
pub mod io_detail;
