//! NPC interaction handler for the CellService.
//!
//! Processes `interact(entityId)` calls from the client: validates distance,
//! looks up the NPC's interaction type, and sends the appropriate response
//! (dialog, vendor, trainer, or loot).
//!
//! Reference: `python/cell/SGWPlayer.py:1148-1203`

mod dhd;
mod dialog;
mod dispatch;
mod loot;
mod trainer;
mod vendor;

pub use dialog::send_dialog_display;
pub(crate) use dispatch::interact_target_in_range;
pub use dispatch::{handle_initial_response, handle_interact};
pub use loot::handle_loot_item;
pub(crate) use trainer::try_open_trainer;
