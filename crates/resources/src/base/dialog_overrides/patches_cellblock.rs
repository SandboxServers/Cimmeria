//! Castle_CellBlock dialog patches.
//!
//! Owned by DU-02a. Ships empty from DU-01 so the two zone tables can be
//! filled by two packets in parallel without touching the same file.
//!
//! Adding a row, in one commit:
//!
//!   1. Add the [`DialogPatch`] here.
//!   2. Make `db/resources/Dialogs/Seed/dialog_screen_buttons.sql` agree.
//!      The client renders from this patch; the seed is the committed
//!      record and what the DU-L linter reads. They are kept in sync by
//!      hand; `patch_seed_agreement_cellblock.rs` fails if they drift, and
//!      also runs the plan against the committed cooked archive.
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
//!     dialog_id: 3999,
//!     ui_screen_type: None,
//!     buttons: ButtonPlan::StripAll,
//! },
//! ```

use super::patch::{ButtonPlan, DialogPatch};

/// Castle_CellBlock patch rows.
///
/// Every row here is [`ButtonPlan::StripAll`]. The shipped Cellblock
/// dialogs repeat one navigation button — Accept, or a "Receive Item"
/// that receives nothing — on most screens of a dialog, and per fact F8
/// that repetition is not free: the client sends
/// `dialogButtonChoice(id, -1)` on close **only** when the dialog's total
/// button count is zero. A button anywhere therefore replaces the close
/// event rather than adding to it.
///
/// Stripping is safe for all twelve because `dialog_choice` chains match
/// on dialog id alone (F9), so a `-1` close resolves to exactly the same
/// actions the old click did. The five keyed dialogs already carried
/// their button on the FIRST screen as well as the last, so the
/// "accept after one screen" path these rows preserve is the one the
/// 2009 data already shipped, not a new one.
///
/// Verified against the committed cooked archive
/// `data/cache/CookedDataDialogs.pak` and against the seed on 2026-09-21;
/// the two agreed for all twelve. Both agreements are now tests, in
/// [`super::patch_seed_agreement_cellblock`].
pub(super) const CELLBLOCK_DIALOG_PATCHES: &[DialogPatch] = &[
    // ---- Mission 638, "Agree to escape" (Human branch) ----
    // Shipped: Accept (type 2, id 8) on all 5 screens 96175-96179.
    // Goes because the Accept is pure navigation noise — chain 1019 keys
    // on the dialog id, so the close emits the same choice the click did.
    // Safe: 2299's Accept is already on screen 96175 (index 0), so a
    // player could always commit from screen one; the strip removes the
    // duplicate button, not the early-out.
    DialogPatch {
        dialog_id: 2299,
        ui_screen_type: None,
        buttons: ButtonPlan::StripAll,
    },
    // ---- Mission 641 briefing, non-Jaffa ----
    // Shipped: Accept on all 5 screens 96247-96251.
    // Goes for the same reason as 2299; chain 1053 is keyed on 4001 and
    // carries no `button_id` condition (F9 — none exists to carry).
    DialogPatch {
        dialog_id: 4001,
        ui_screen_type: None,
        buttons: ButtonPlan::StripAll,
    },
    // ---- Mission 641 briefing, Jaffa ----
    // Shipped: Accept on 8 of 10 screens (96261, 96264-96270; 96262 and
    // 96263 are bare). Goes with 4001's — chain 1054 is its mirror.
    DialogPatch {
        dialog_id: 5022,
        ui_screen_type: None,
        buttons: ButtonPlan::StripAll,
    },
    // ---- Mission 641, Marsh's second briefing, non-Jaffa ----
    // Shipped: "Receive Item" (type 4, id 70) on 7 of 9 screens
    // 96252-96258. Screens 96259 AND 96260 are bare, and 96260 is the
    // final screen.
    //
    // This one is actively broken today, not merely noisy: a player who
    // pages to the end has no button to press, and Done on a dialog that
    // HAS buttons sends nothing (F8), so chain 1058 never fires and step
    // 3564 never opens. Stripping restores the close event.
    //
    // The button's label is a lie in both shapes: nothing is granted here.
    // Item 21 (the SGHC 6 SMG) comes from chain 1055's `add_item` on the
    // locker interact, which the player must already have done — and
    // equipped, via chain 1066 — before 3999 is reachable at all.
    DialogPatch {
        dialog_id: 3999,
        ui_screen_type: None,
        buttons: ButtonPlan::StripAll,
    },
    // ---- Mission 641, Marsh's second briefing, Jaffa ----
    // Shipped: "Receive Item" on 9 of 11 screens (96282, 96285-96292;
    // 96283 and 96284 are bare). Unlike 3999 the final screen 96292 does
    // carry the button, so 5023 is not soft-locked today — it goes for
    // consistency with its non-Jaffa mirror 3999 and because the label
    // grants nothing here either (chain 1059, like 1058, only advances a
    // step and moves quest markers).
    DialogPatch {
        dialog_id: 5023,
        ui_screen_type: None,
        buttons: ButtonPlan::StripAll,
    },
    // ---- Narration, not offers ----
    // 2309: Accept on all 3 screens 96336-96338. Displayed by chain 1171
    // as a pre-ring-travel beat; nothing keys it, so the Accept commits
    // to nothing and only shows the client's automatic Decline beside it
    // (F6). Cosmetic strip.
    DialogPatch {
        dialog_id: 2309,
        ui_screen_type: None,
        buttons: ButtonPlan::StripAll,
    },
    // 2516: Accept on its single screen 96392. Displayed by chain 1161
    // after the Straegis scene. Same reasoning as 2309.
    DialogPatch {
        dialog_id: 2516,
        ui_screen_type: None,
        buttons: ButtonPlan::StripAll,
    },
    // 5859: Accept on its single screen 96367. A Blurb, so the client
    // draws Accept AND the inert Decline that `Blurb.lua` never wires to
    // the button (F7). Objective-update text with nothing to accept;
    // X-close only.
    DialogPatch {
        dialog_id: 5859,
        ui_screen_type: None,
        buttons: ButtonPlan::StripAll,
    },
    // ---- The four post-accept blurbs (decision D-DU1: strip all) ----
    // Each is displayed by a chain that fires on `mission_accepted`, so
    // the mission is already the player's by the time the blurb opens and
    // there is nothing left for Accept to accept. On a Blurb, showing
    // Accept also shows the inert Decline (F7), which would be a button
    // that visibly does nothing on the first press.
    //
    // 2305: Accept on screen 18795. Chain 1151, after mission 640.
    DialogPatch {
        dialog_id: 2305,
        ui_screen_type: None,
        buttons: ButtonPlan::StripAll,
    },
    // 4000: Accept AND More Info (type 1, id 9) on screen 96219. Chain
    // 1152, after mission 641. More Info is dead either way — F9 means
    // the server cannot tell it from Accept.
    DialogPatch {
        dialog_id: 4000,
        ui_screen_type: None,
        buttons: ButtonPlan::StripAll,
    },
    // 2308: Accept AND More Info on screen 96323. Its display chain 1153
    // is reserved but not authored, so this row changes nothing in game
    // today; it lands now so the reserved chain inherits the fixed shape
    // rather than the broken one.
    DialogPatch {
        dialog_id: 2308,
        ui_screen_type: None,
        buttons: ButtonPlan::StripAll,
    },
    // 2518: Accept on screen 96406. Chain 1154, after mission 688.
    DialogPatch {
        dialog_id: 2518,
        ui_screen_type: None,
        buttons: ButtonPlan::StripAll,
    },
];
