pub mod combat;
pub mod constants;
pub mod crafting;
mod dispatch;
pub mod interaction;
pub mod social;
pub mod vendor;
pub mod world;
// `trainer_interaction` consolidated into
// `cell::interactions::trainer::try_open_trainer` in #55 — single canonical
// source-of-truth path. Callsite in `interaction.rs` routes through that.

pub use constants::*;
pub use dispatch::dispatch;
