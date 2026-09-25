//! Castle dialog patches.
//!
//! Owned by DU-02b. Shipped empty from DU-01 and filled here, so the two
//! zone tables can be filled by two packets in parallel without touching
//! the same file.
//!
//! Adding a row, in one commit:
//!
//!   1. Add the [`DialogPatch`] here.
//!   2. Make `db/resources/Dialogs/Seed/dialog_screen_buttons.sql` agree.
//!      The client renders from this patch; the seed is the committed
//!      record and what the DU-L linter reads. They are kept in sync by
//!      hand, exactly as the full-regeneration overrides already require.
//!   3. Check the two hard rules from the client contract:
//!      * a dialog that keys a `dialog_choice` chain must end with either
//!        zero buttons or a button on its FINAL screen, or a player who
//!        reads to the end and presses Done soft-locks (fact F8);
//!      * never add a button to 2300, 5021, 5020, 2574, 2575, 2577, 2581,
//!        5003, 5004 or 5009.
//!
//! Syntax:
//!
//! ```ignore
//! DialogPatch {
//!     dialog_id: 2576,
//!     ui_screen_type: None,
//!     buttons: ButtonPlan::OnlyOn {
//!         screen_id: 96825,
//!         button_type: 4,
//!         button_id: 71,
//!         text: "Take Missions",
//!     },
//! },
//! ```

use super::patch::{ButtonPlan, DialogPatch};

/// Castle patch rows (DU-02b).
///
/// All three move one cooked button onto the dialog's FINAL screen. The
/// `button_type`, `button_id` and `text` of each row are the values the
/// game itself shipped, read from both `CookedDataDialogs.pak` entry
/// `_<id>` and `db/resources/Dialogs/Seed/dialog_screen_buttons.sql` on
/// 2026-09-21; the two sources agreed exactly. Keeping the shipped
/// `ButtonID` matters because the client puts that id — not the button's
/// index — on the wire when it is clicked (fact F14), and the Castle
/// chains were authored against it.
///
/// All three dialogs are `ui_screen_type` 2 (`DUIST_DefaultDialog`), so
/// `DialogWin` draws Accept (2) and Generic1-3 (4, 5, 6): both button
/// types below are drawable. No row changes `ui_screen_type` — that is
/// DU-04 / DU-05 territory.
pub const CASTLE_DIALOG_PATCHES: &[DialogPatch] = &[
    // 2573 — Sgt. Gerschon's Human/Tau'ri offer of mission 701. Accept
    // shipped on all 7 screens (113552-113558), so a player could accept
    // off screen one without reading the briefing. Not a soft-lock —
    // this dialog satisfies the hard rule today — but the design is one
    // decision at the END, and leaving Accept on every screen also shows
    // the client's automatic Decline on every screen (F6). Keyed by
    // chain 1204.
    DialogPatch {
        dialog_id: 2573,
        ui_screen_type: None,
        buttons: ButtonPlan::OnlyOn {
            screen_id: 113558,
            button_type: 2,
            button_id: 8,
            text: "Accept",
        },
    },
    // 5861 — Gerschon's Jaffa offer of the same mission (D-CA13). Accept
    // shipped on screens 96782-96786 of 8; the final screen 96789 was
    // bare. A Jaffa who read to the end saw only Done, and Done on a
    // dialog that HAS buttons sends nothing (F8), so chain 1205 never
    // fired and mission 701 was unacceptable for that player. Closes the
    // Accept gap filed under Castle audit D-CA13.
    DialogPatch {
        dialog_id: 5861,
        ui_screen_type: None,
        buttons: ButtonPlan::OnlyOn {
            screen_id: 96789,
            button_type: 2,
            button_id: 8,
            text: "Accept",
        },
    },
    // 2576 — Capt. Copplemann's 701 turn-in, which also hands out 702
    // and 703. "Take Missions" shipped on screens 96821-96823 of 5; the
    // final screen 96825 was bare, so the same read-to-the-end soft-lock
    // stranded chains 1237/1238/1239 and the whole 702/703 branch.
    // Closes GAP 3 in castle_701_chains.sql.
    DialogPatch {
        dialog_id: 2576,
        ui_screen_type: None,
        buttons: ButtonPlan::OnlyOn {
            screen_id: 96825,
            button_type: 4,
            button_id: 71,
            text: "Take Missions",
        },
    },
];
