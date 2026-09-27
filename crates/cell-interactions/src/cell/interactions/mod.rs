//! NPC interaction handler for the CellService.
//!
//! Processes `interact(entityId)` calls from the client: validates distance,
//! looks up the NPC's interaction type, and sends the appropriate response
//! (dialog, vendor, trainer, banker, or loot).
//!
//! Reference: `python/cell/SGWPlayer.py:1148-1203`

mod bank;
pub mod crafting_stations;
mod dhd;
// The dialog display choke point is in `cimmeria-cell-content` (wave C3):
// the content executor opens dialogs through it. Imported under its old name,
// so `super::super::dialog::send_dialog_display` in `dispatch` is unchanged.
use cimmeria_cell_content::cell::interactions::dialog;
mod dispatch;
mod loot;
mod respec_feedback;
mod trainer;
mod trainer_authority;
mod vendor;

pub use bank::{
    open_vault_at_banker, open_vault_gm, pin_interaction_target, reject_banker_out_of_range,
    reject_vault_open, vault_move_allowed, VaultOpenReject, VaultReject,
};
pub use dialog::send_dialog_display;
pub use dispatch::{handle_initial_response, handle_interact};
pub use dispatch::{interact_range, interact_target_in_range, InteractRangeFail};
pub use loot::handle_loot_item;
pub use respec_feedback::send_respec_rejection;
pub use trainer::try_open_trainer;
pub use trainer_authority::{resend_pinned_trainer, trainer_pin};
