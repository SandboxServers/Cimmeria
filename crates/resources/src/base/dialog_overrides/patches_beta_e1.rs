//! Beta Site E1 dialog patches (UB-DP).
//!
//! Two Beta offer dialogs (1997, 3228) break the dialog-button rule R1: the
//! `dialog_choice` chain keys the dialog, but its only Accept sits on a
//! middle screen. Two `show`-only dialogs (1916, 3905) carry a type-1 More
//! Info that `DialogWin` cannot draw (R3). See `mission-seal.md` Q2 and C5
//! in `docs/analysis/union-beta-e1-continuation/` for the findings.
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

/// Beta Site E1 patch rows (UB-DP).
///
/// The two offers move their Accept (type 2, id 8, as shipped) from the
/// middle screen onto the FINAL screen. The two More Info rows are stripped
/// entirely: neither dialog is `dialog_choice`-keyed, so a zero-button close
/// sends `(id, -1)` to no chain.
pub(super) const BETA_E1_DIALOG_PATCHES: &[DialogPatch] = &[
    // 477 offer (dsm 2456): Accept moves from screen 64047 to the final screen 64049.
    DialogPatch {
        dialog_id: 1997,
        ui_screen_type: None,
        buttons: ButtonPlan::OnlyOn {
            screen_id: 64049,
            button_type: 2,
            button_id: 8,
            text: "Accept",
        },
    },
    // 479 offer (dsm 2461): Accept moves from screen 64066 to the final screen 64068.
    DialogPatch {
        dialog_id: 3228,
        ui_screen_type: None,
        buttons: ButtonPlan::OnlyOn {
            screen_id: 64068,
            button_type: 2,
            button_id: 8,
            text: "Accept",
        },
    },
    // 1916 (UB-31 chain 23317 shows it): DialogWin cannot draw its type-1
    // More Info (R3), so the More Info rows are stripped.
    DialogPatch {
        dialog_id: 1916,
        ui_screen_type: None,
        buttons: ButtonPlan::StripAll,
    },
    // 3905 (UB-33 chain 23395 shows it): same R3 fix.
    DialogPatch {
        dialog_id: 3905,
        ui_screen_type: None,
        buttons: ButtonPlan::StripAll,
    },
];
