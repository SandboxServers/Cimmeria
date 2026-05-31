pub mod lootable;
pub mod stargate;
pub mod vendor;

pub use lootable::*;
pub use stargate::*;
pub use vendor::*;

// Trainer interaction lives in `cimmeria-services` —
// `crates/services/src/cell/interactions/trainer.rs`. No game-crate-side
// types are needed; the cell-side handler reads `SpaceManager` state
// directly.
