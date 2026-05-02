//! NPC interaction handler for the CellService.
//!
//! Processes `interact(entityId)` calls from the client: validates distance,
//! looks up the NPC's interaction type, and sends the appropriate response
//! (dialog, vendor, trainer, or loot).
//!
//! Reference: `python/cell/SGWPlayer.py:1148-1203`

mod dialog;
mod dispatch;
mod loot;
mod trainer;
mod vendor;

pub use dialog::send_dialog_display;
pub use dispatch::{handle_initial_response, handle_interact};
pub use loot::handle_loot_item;
