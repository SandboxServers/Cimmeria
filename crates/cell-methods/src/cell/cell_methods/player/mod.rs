pub mod combat;
pub use cimmeria_wire::cell::cell_methods::player::constants;
pub mod crafting;
mod dispatch;
pub mod interaction;
pub mod social;
pub mod trade;
pub mod vendor;
pub mod world;
// Trainer interaction lives in `cell::interactions::trainer::try_open_trainer`
// — the single canonical source-of-truth path. The callsite in
// `interaction.rs::dispatch` routes through it.

pub use constants::*;
pub use dispatch::dispatch;
