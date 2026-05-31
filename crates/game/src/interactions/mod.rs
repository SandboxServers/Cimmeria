pub mod lootable;
pub mod stargate;
pub mod vendor;

pub use lootable::*;
pub use stargate::*;
pub use vendor::*;

// Trainer interaction lives in `cimmeria-services` —
// `crates/services/src/cell/interactions/trainer.rs`. The old stub here
// (`TrainerSession`, `TrainableAbility`, `TrainerResult` with `todo!()`)
// was dead code that predated the real implementation. Removed in #55.
