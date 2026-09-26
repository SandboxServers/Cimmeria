//! The GM gate of the client->server cell-method dispatcher, under the
//! `cell::dispatch` path it had in `cimmeria-services`.
//!
//! The router itself (`dispatch_cell_method`) calls every cell-method
//! handler, so it sits at the top of the cell track; the gate sits here,
//! because the movement validator's GM off-navmesh allowance and the NPC AI's
//! GM checks read it.
//!
//! `constants` is wire contract (`cimmeria-wire`); it is imported here so
//! `gm_gate` keeps naming it as `super::constants`.

use cimmeria_wire::cell::dispatch::constants;

pub mod gm_gate;

// The canonical raw-column → typed `AccessLevel` conversion, the one mapping
// every privilege check in the cell must share, and the `.`-console
// privilege test.
pub use gm_gate::{access_level_from_u32, is_gm};
