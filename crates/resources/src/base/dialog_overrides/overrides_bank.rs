//! The Banker's Expand vault offer (BV-05 / #479).
//!
//! Served since 2026-10-22 (D-LN3). It was quarantined 2026-09-27 to
//! 2026-10-22 with the debug-hub dialogs, after a Castle_CellBlock map-load
//! crash whose cause RE never named
//! (`docs/reverse-engineering/findings/cooked-dialog-override-crash.md`).

use super::{DialogButton, DialogOverride, DialogScreen};

/// Served since 2026-10-22 (D-LN3). The Banker's Expand vault offer (60110,
/// `cimmeria_wire::cell::vault::VAULT_EXPAND_DIALOG_ID`). One screen,
/// one Generic 1 button (the 60100 reasoning). The server shows it beside
/// `onVaultOpen` while the vault is below 100 slots; the answer goes to
/// the purchase path, which re-checks everything. Closing with X sends
/// nothing, so nothing is bought. Speaker 0, because any Banker (or the
/// GM's own entity after `.bank`) speaks it.
pub(super) const VAULT_EXPAND_DIALOG: DialogOverride = DialogOverride {
    dialog_id: 60110,
    dialog_flags: 0,
    kismet_event_set_id: 0,
    ui_screen_type: 2,
    screens: &[DialogScreen {
        screen_id: 200010,
        speaker_id: 0,
        text: "Vault expansion. I can add 10 slots to your vault, up to 100 in all. The \
               price of the next 10 slots is in your chat window.",
        buttons: &[DialogButton {
            button_type: 4,
            button_id: 8,
            text: "Expand vault",
        }],
    }],
};
