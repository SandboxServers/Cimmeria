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
//!      hand, exactly as the full-regeneration overrides already require.
//!      The `tests` module at the bottom of this file fails if they drift.
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
/// Verified against `data/cache/CookedDataDialogs.pak` (read-only, not in
/// git) and the seed on 2026-09-21; the two agreed for all twelve.
pub const CELLBLOCK_DIALOG_PATCHES: &[DialogPatch] = &[
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Where the seed lives, relative to `crates/services`. Same
    /// `CARGO_MANIFEST_DIR` hop the committed-PAK tests use.
    const SEED_DIR: &str = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../db/resources/Dialogs/Seed"
    );

    const SCREENS_HEADER: &str =
        "INSERT INTO dialog_screens (dialog_id, screen_id, text, speaker_id, index) VALUES (";
    const BUTTONS_HEADER: &str = "INSERT INTO dialog_screen_buttons (screen_button_id, \
         button_id, screen_id, button_type, text) VALUES (";

    /// One seed button row, in the order the client cares about.
    #[derive(Debug, PartialEq, Eq)]
    struct SeedButton {
        screen_id: u32,
        button_type: u32,
        button_id: u32,
    }

    fn read_seed(name: &str) -> String {
        let path = std::path::Path::new(SEED_DIR).join(name);
        std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("seed file {} must be readable: {e}", path.display()))
    }

    /// Yield the text following each occurrence of `header`.
    ///
    /// Both seed files write one single-tuple `INSERT` per statement with
    /// the column list spelled out in full, so finding the header is the
    /// whole of the parse. `dialog_screens.text` contains raw newlines and
    /// apostrophes, but it is the THIRD column — every field this scanner
    /// reads sits in front of it, so no quote tracking is needed. The
    /// vacuity guard below pins the match count against the raw header
    /// count so a format change fails loudly instead of parsing nothing.
    fn after_each<'a>(sql: &'a str, header: &str) -> Vec<&'a str> {
        sql.match_indices(header)
            .map(|(i, _)| &sql[i + header.len()..])
            .collect()
    }

    /// Read the first `n` comma-separated integers of a VALUES tuple.
    fn leading_ints(tail: &str, n: usize) -> Vec<u32> {
        tail.split(',')
            .take(n)
            .map(|f| {
                f.trim()
                    .parse()
                    .unwrap_or_else(|e| panic!("expected an integer field, got {f:?}: {e}"))
            })
            .collect()
    }

    /// `dialog_id -> its screen ids`, unordered. Ordering is not needed:
    /// `StripAll` is a whole-dialog assertion, and `OnlyOn` names its
    /// screen explicitly.
    fn screens_by_dialog() -> std::collections::HashMap<u32, Vec<u32>> {
        let sql = read_seed("dialog_screens.sql");
        let mut out: std::collections::HashMap<u32, Vec<u32>> = std::collections::HashMap::new();
        for tail in after_each(&sql, SCREENS_HEADER) {
            let ids = leading_ints(tail, 2);
            out.entry(ids[0]).or_default().push(ids[1]);
        }
        out
    }

    /// Every seed button row, keyed by screen id.
    fn buttons_by_screen() -> std::collections::HashMap<u32, Vec<SeedButton>> {
        let sql = read_seed("dialog_screen_buttons.sql");
        let mut out: std::collections::HashMap<u32, Vec<SeedButton>> =
            std::collections::HashMap::new();
        for tail in after_each(&sql, BUTTONS_HEADER) {
            // (screen_button_id, button_id, screen_id, button_type, text)
            let ids = leading_ints(tail, 4);
            out.entry(ids[2]).or_default().push(SeedButton {
                screen_id: ids[2],
                button_type: ids[3],
                button_id: ids[1],
            });
        }
        out
    }

    /// Every seed button row belonging to `dialog_id`.
    fn seed_buttons_of(dialog_id: u32) -> Vec<SeedButton> {
        let screens = screens_by_dialog();
        let buttons = buttons_by_screen();
        let mut out = Vec::new();
        for screen in screens.get(&dialog_id).into_iter().flatten() {
            for button in buttons.get(screen).into_iter().flatten() {
                out.push(SeedButton {
                    screen_id: button.screen_id,
                    button_type: button.button_type,
                    button_id: button.button_id,
                });
            }
        }
        out
    }

    /// The scan must find the rows it claims to check.
    ///
    /// Every assertion below is of the "no disagreement found" shape, so a
    /// scanner that silently matched nothing would pass all of them. Pin
    /// the parsed count against the raw `INSERT INTO` count per file, the
    /// same vacuity guard the DU-L seed linter uses.
    #[test]
    fn the_seed_scan_reads_every_insert_row() {
        let screens_sql = read_seed("dialog_screens.sql");
        let buttons_sql = read_seed("dialog_screen_buttons.sql");

        assert_eq!(
            after_each(&screens_sql, SCREENS_HEADER).len(),
            screens_sql.matches("INSERT INTO dialog_screens").count(),
            "dialog_screens.sql: the scanner's header no longer matches every INSERT — \
             the statement format changed and the agreement tests below are reading a \
             subset of the seed",
        );
        assert_eq!(
            after_each(&buttons_sql, BUTTONS_HEADER).len(),
            buttons_sql
                .matches("INSERT INTO dialog_screen_buttons")
                .count(),
            "dialog_screen_buttons.sql: the scanner's header no longer matches every \
             INSERT — see the message above",
        );

        // A dialog outside this packet that still carries buttons, so a
        // scanner returning an empty button map cannot pass.
        assert!(
            !seed_buttons_of(2298).is_empty(),
            "dialog 2298 (mission 639's offer) must still carry its More Info + Accept \
             rows; an empty result means the button scan found nothing at all",
        );
    }

    /// Every patch row agrees with the seed.
    ///
    /// F1: the client renders from the patch, the seed is the committed
    /// record and what the DU-L linter reads. Nothing makes them agree
    /// automatically, so this is the thing that fails when someone
    /// re-adds a seed button row for a patched dialog — or patches a
    /// dialog and forgets the seed.
    #[test]
    fn cellblock_patches_agree_with_the_dialog_seed() {
        for patch in CELLBLOCK_DIALOG_PATCHES {
            let seed = seed_buttons_of(patch.dialog_id);
            match &patch.buttons {
                ButtonPlan::StripAll => assert!(
                    seed.is_empty(),
                    "dialog {id}: the patch strips every button, but the seed still has \
                     {seed:?}. The client would show no button and the seed would claim \
                     one — delete those rows from \
                     db/resources/Dialogs/Seed/dialog_screen_buttons.sql.",
                    id = patch.dialog_id,
                ),
                ButtonPlan::OnlyOn {
                    screen_id,
                    button_type,
                    button_id,
                    ..
                } => assert_eq!(
                    seed,
                    vec![SeedButton {
                        screen_id: *screen_id,
                        button_type: *button_type,
                        button_id: *button_id,
                    }],
                    "dialog {id}: the patch leaves exactly one button on screen \
                     {screen_id}, but the seed says otherwise",
                    id = patch.dialog_id,
                ),
                // `Keep` is an identity on the buttons, so the seed is
                // correct by construction and there is nothing to compare.
                // DU-05's type-label patches are all `Keep`.
                ButtonPlan::Keep => {}
            }
        }
    }

    /// The table still covers every dialog DU-02a is responsible for.
    ///
    /// Without this, deleting a patch row would leave the loop above with
    /// nothing to check for that dialog while its seed rows stayed
    /// deleted — the seed and the client would silently disagree in the
    /// other direction, and the dialog would get its buttons back at the
    /// next PAK load.
    #[test]
    fn cellblock_patch_table_covers_exactly_the_du02a_roster() {
        const ROSTER: [u32; 12] = [
            2299, 4001, 5022, 3999, 5023, 2309, 2516, 5859, 2305, 4000, 2308, 2518,
        ];

        let mut ids: Vec<u32> = CELLBLOCK_DIALOG_PATCHES
            .iter()
            .map(|p| p.dialog_id)
            .collect();
        ids.sort_unstable();
        let mut expected = ROSTER;
        expected.sort_unstable();

        assert_eq!(
            ids,
            expected.to_vec(),
            "CELLBLOCK_DIALOG_PATCHES no longer covers exactly the DU-02a roster. A row \
             removed here without restoring its dialog_screen_buttons.sql rows leaves the \
             client showing buttons the seed says are gone. Adding a Cellblock dialog is \
             fine — extend ROSTER in the same commit and say why in the worknote.",
        );

        for patch in CELLBLOCK_DIALOG_PATCHES {
            assert_eq!(
                patch.buttons,
                ButtonPlan::StripAll,
                "dialog {id}: every DU-02a row is StripAll. A plan change needs the \
                 matching seed rows and a new row in the agreement test above.",
                id = patch.dialog_id,
            );
            assert_eq!(
                patch.ui_screen_type,
                None,
                "dialog {id}: DU-02a changes buttons only. Window types belong to DU-05.",
                id = patch.dialog_id,
            );
        }
    }
}
