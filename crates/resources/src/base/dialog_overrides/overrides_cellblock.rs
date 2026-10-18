//! Cellblock authored dialog overrides (CB-N05, CIMMERIA_AUTHORING).
//!
//! Only the shape the client already renders from 3995/3996 is used: one
//! screen, `speaker_id` 0, zero buttons, UI type 2. Multi-screen or
//! NPC-speaker authored dialogs stay quarantined (2026-09-27 client crash).

use super::{DialogOverride, DialogScreen};

/// Mission 62050 *The Remaining Pods*: Prisoner 329's report, shown by
/// chain 11004 after the accept. Closing it (`dialogButtonChoice(60150, -1)`)
/// has no content action. Frozen in w3-frozen.md (F3).
pub(super) const REMAINING_PODS_DIALOG: DialogOverride = DialogOverride {
    dialog_id: 60150,
    dialog_flags: 0,
    kismet_event_set_id: 0,
    ui_screen_type: 2,
    screens: &[DialogScreen {
        screen_id: 96110,
        speaker_id: 0,
        text: "Prisoner 329 says there are still people in the other pods. Tell Col. Marsh to check them before you leave.",
        buttons: &[],
    }],
};
