//! Client->server cell method dispatch for the SGWPlayer entity type.
//!
//! When the client calls an exposed CellMethod, the BaseApp forwards it as a
//! [`CellMethodCall`](super::messages::BaseToCellMsg::CellMethodCall) message.
//! This module maps the flattened method index to a named handler.
//!
//! ## Flattened EXPOSED CellMethod index ordering
//!
//! The client encodes cell method calls as `msg_id = index | 0x80` (direct, 0-60)
//! or `msg_id = 0xBD` with sub-index (extended, >= 61). The index is the
//! flattened position across the entity type hierarchy, counting only `<Exposed/>`
//! methods in CellMethods sections.
//!
//! See `docs/protocol/cell-method-dispatch-table.md` for the complete 109-method
//! table with arg formats, interface sources, and .def line references.
//!
//! ## Module layout
//!
//! - [`constants`] — `pub use` re-exports of every `CM_*` and `CLIENT_MG_*`
//!   constant from the per-interface `cell_methods` / `client_methods` modules.
//!   These are re-exported again here so existing call sites that reference
//!   `crate::cell::dispatch::CM_*` keep working.
//! - [`router`] — the [`dispatch_cell_method`] entry point that delegates to
//!   the per-interface `dispatch` functions in inheritance order.
//! - [`names`] — the [`cell_method_name`] lookup used for logging.

mod constants;
mod names;
mod router;

#[cfg(test)]
mod tests;

// Re-export the public API at the module root so external paths
// (`crate::cell::dispatch::dispatch_cell_method`,
// `crate::cell::dispatch::CM_*`,
// `crate::cell::dispatch::CLIENT_MG_*`,
// `crate::cell::dispatch::cell_method_name`) keep resolving.
pub use constants::*;
pub use names::cell_method_name;
pub use router::dispatch_cell_method;
