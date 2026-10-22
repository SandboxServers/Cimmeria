//! Debug-hub dialogs (templates and chains in `docs/content/debug-hub.md`).
//!
//! Served since 2026-10-22 (D-LN3). They were quarantined 2026-09-27 to
//! 2026-10-22 after a Castle_CellBlock map-load crash whose cause RE never
//! named (`docs/reverse-engineering/findings/cooked-dialog-override-crash.md`).

use super::{DialogButton, DialogOverride, DialogScreen};

/// NEW CONTENT (debug hub): the stasis-room dialog NPC (template 302,
/// Airman Lance, speaker 754), chains 7001-7003 in debug_hub_chains.sql.
/// Two screens so the player pages with Next; ONE button, on the final
/// screen (hard rule 1), so reading to the end always leaves something to
/// press. Type 4 (Generic 1) because a `DUIST_DefaultDialog` draws only
/// types 2 and 4-6, and type 2 (Accept) would hide the label behind a
/// fixed image and bring an inert Decline.
pub(super) const STASIS_DIALOG: DialogOverride = DialogOverride {
    dialog_id: 60100,
    dialog_flags: 0,
    kismet_event_set_id: 0,
    ui_screen_type: 2,
    screens: &[
        DialogScreen {
            screen_id: 200000,
            speaker_id: 754,
            text: "Debug hub dialog test. This is screen one of two. Page forward to \
                   reach the button.",
            buttons: &[],
        },
        DialogScreen {
            screen_id: 200001,
            speaker_id: 754,
            text: "Screen two. The button below sends your choice to the server, and the \
                   server answers by opening a second dialog.",
            buttons: &[DialogButton {
                button_type: 4,
                button_id: 8,
                text: "Send my choice",
            }],
        },
    ],
};

/// NEW CONTENT (debug hub): the answer to 60100's button. Zero buttons on
/// purpose: closing it sends `dialogButtonChoice(60101, -1)` (fact F8),
/// which is the other half of the round trip (chain 7003).
pub(super) const STASIS_ANSWER_DIALOG: DialogOverride = DialogOverride {
    dialog_id: 60101,
    dialog_flags: 0,
    kismet_event_set_id: 0,
    ui_screen_type: 2,
    screens: &[DialogScreen {
        screen_id: 200002,
        speaker_id: 754,
        text: "Choice received. This dialog has no buttons, so closing it sends -1 to \
               the server, which answers in your chat window.",
        buttons: &[],
    }],
};

/// Social-systems campaign, SS-U3: the Gate Mail Clerk (template 390,
/// Sgt. Harriman, speaker 843), chains 7010-7011 in debug_hub_chains.sql.
/// One screen, one Generic 1 button (the 60100 reasoning): pressing it
/// sends `dialogButtonChoice(60104, 8)` and chain 7011 mails the player.
/// Closing with X sends nothing, so nothing is mailed.
pub(super) const MAIL_CLERK_DIALOG: DialogOverride = DialogOverride {
    dialog_id: 60104,
    dialog_flags: 0,
    kismet_event_set_id: 0,
    ui_screen_type: 2,
    screens: &[DialogScreen {
        screen_id: 200005,
        speaker_id: 843,
        text: "Gate Mail. I can send you a test mail with a stack of Health Slappacks \
               and 50 naquadah. Open your mail afterwards to take them. One mail every \
               10 minutes.",
        buttons: &[DialogButton {
            button_type: 4,
            button_id: 8,
            text: "Send me a mail",
        }],
    }],
};
