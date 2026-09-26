//! NPC interaction handler for the CellService.
//!
//! Processes `interact(entityId)` calls from the client: validates distance,
//! looks up the NPC's interaction type, and sends the appropriate response
//! (dialog, vendor, trainer, or loot).
//!
//! Reference: `python/cell/SGWPlayer.py:1148-1203`

mod dhd;
// The dialog display choke point is in `cimmeria-cell-content` (wave C3):
// the content executor opens dialogs through it. Imported under its old name,
// so `super::super::dialog::send_dialog_display` in `dispatch` is unchanged.
use cimmeria_cell_content::cell::interactions::dialog;
mod dispatch;
mod loot;
mod trainer;
mod vendor;

pub use dialog::send_dialog_display;
pub use dispatch::interact_target_in_range;
pub use dispatch::{handle_initial_response, handle_interact};
pub use loot::handle_loot_item;
pub use trainer::try_open_trainer;
