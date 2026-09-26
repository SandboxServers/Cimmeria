//! Gate travel's cell->base round trips: tests that drive the cell's gate
//! handlers (`cell::gate_travel`) and then the base's world entry on the
//! messages they emit. Test-only.
//!
//! Each was a test of `base::world_entry`, which moved to
//! `cimmeria-base-world-entry` in wave B3 of
//! docs/architecture/services-crate-split.md. The cell half is still in this
//! crate, and the base crate cannot depend on the cell, so they stay here;
//! they need both tracks, so their final home is the facade.
//!
//! - [`stargate_fanout`]: the gate `onSequence` fan-out, from the cell
//!   emitter through the base dispatcher to the wire.
//! - [`dial_to_gate_travel`]: `handle_dial_gate`'s `GateTravel` fed into
//!   `handle_gate_travel`.
//! - [`dial_refusal_persist`]: a refused dial never reaches the arrival
//!   write, and the same dial accepted does (live DB).

mod dial_refusal_persist;
mod dial_to_gate_travel;
mod stargate_fanout;
