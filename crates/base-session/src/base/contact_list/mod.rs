//! Contact list subsystem — BaseApp side.
//!
//! Owns persistence (`persistence`), handler logic (`handlers`), and the
//! wire-serialization helpers (`wire`) for the five S→C contact-list methods
//! (CM 85–89). The login-push path is in `world_entry_appearance/client_ready.rs`
//! and calls into `handlers` directly.

pub mod handlers;
pub(crate) mod persistence;
pub use cimmeria_wire::base::contact_list::wire;
