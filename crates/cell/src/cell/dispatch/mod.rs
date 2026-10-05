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
//!
//! `constants` and `names` are wire contract and live in `cimmeria-wire`
//! (wave W3a of the services crate split). `gm_gate` is in
//! `cimmeria-cell-world` (wave C1), because the movement validator and the NPC
//! AI read it; it is imported here so the router and the tests keep naming it
//! as `super::gm_gate`.

use cimmeria_cell_world::cell::dispatch::gm_gate;
use cimmeria_wire::cell::dispatch::{constants, names};
pub mod ability_receipt;
mod router;

#[cfg(test)]
mod ability_receipt_tests;
#[cfg(test)]
mod gm_ability_dispatch_tests;
#[cfg(test)]
mod gm_combat_debug_dispatch_tests;
#[cfg(test)]
mod gm_dispatch_tests;
#[cfg(test)]
mod plugin_routing_tests;
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

// The canonical raw-column → typed `AccessLevel` conversion,
// `gm_gate::access_level_from_u32`, is used only inside `cimmeria-cell-world`
// now (the movement validator's GM off-navmesh allowance). The `.`-console
// privilege test, `gm_gate::is_gm`, is re-exported as `cell::console::is_gm`
// by `cimmeria-cell-console` (wave C5b), straight from `cimmeria-cell-world`.
