//! Patch-mode dialog overrides: transform the canonical cooked entry
//! instead of re-authoring it.
//!
//! A [`super::DialogOverride`] regenerates a whole `<COOKED_DIALOG>` from
//! Rust-authored text. That is right for a dialog Cimmeria wrote (3995,
//! 3996) and wrong for one of the 5,405 the game shipped: restating tens of
//! screens of voiced dialogue in a Rust source file to change one button is
//! a transcription error waiting to happen.
//!
//! A [`DialogPatch`] instead parses the shipped entry, edits only the
//! fields named in the plan, and re-emits through the same emitter. Every
//! screen's `SpeakerID`, `ScreenID` and `Text` come out byte-identical to
//! the source.
//!
//! # Why buttons need patching at all
//!
//! Closing a dialog that has ZERO buttons makes the client send
//! `dialogButtonChoice(dialogId, -1)`. Closing one that has ANY button
//! sends nothing (ledger fact F8). So a dialog keying a `dialog_choice`
//! chain must have either no buttons, or a button on its FINAL screen —
//! otherwise a player who reads to the end and presses Done soft-locks.
//! Dialogs 3999, 5861 and 2576 ship in that broken shape.
//!
//! Button order inside a screen is load-bearing: the client turns a click
//! into a position in the screen's button array and sends the `ButtonID`
//! sitting at that position (Native Findings, `FUN_00ad8690`). The parser
//! and emitter preserve document order, and [`ButtonPlan::Keep`] is a true
//! identity.
//!
//! # Where the rows live
//!
//! Nowhere in this file. The plans live in one table per zone —
//! [`super::patches_cellblock`] and [`super::patches_castle`] — so two
//! packets filling different zones never touch the same file. Both ship
//! empty; DU-02a and DU-02b fill them.

use std::collections::HashMap;

use super::emit::{emit_cooked_dialog, escape_xml_attr, CookedButton};
use super::parse::parse_cooked_dialog;

/// What to do with a dialog's buttons.
///
/// `dead_code` is allowed because the only non-test constructors live in
/// the two zone tables, and DU-01 ships both empty. Drop the attribute in
/// whichever Wave 1 packet adds the first row.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ButtonPlan {
    /// Leave every button exactly where the cook put it. Use with a
    /// `ui_screen_type` change, which is the only other thing a patch can
    /// do.
    Keep,
    /// Remove every button from every screen. The dialog then closes with
    /// `dialogButtonChoice(id, -1)` (F8), which is what a keyed dialog
    /// wants when the button is redundant with reading to the end.
    StripAll,
    /// Remove every button from every screen, then put exactly one on the
    /// named screen. Use to move a mid-dialog button onto the FINAL
    /// screen, which is the shape the hard rule requires.
    ///
    /// `button_id` is the cooked id the client puts on the wire when the
    /// button is clicked (F14: 8 Accept, 9 More Info, 70 Receive Item,
    /// 71 Take Missions) — not an index. `button_type` selects the widget
    /// (F7: `DialogWin` renders 2 and 4+, `BlurbWin` renders 1 and 2).
    ///
    /// `text` is written plainly and escaped on the way out.
    OnlyOn {
        screen_id: u32,
        button_type: u32,
        button_id: u32,
        text: &'static str,
    },
}

/// One dialog's patch: which cooked entry to transform and how.
///
/// `dialog_id` is the cooked catalogue key (`_<dialog_id>` in the PAK),
/// the same id the server's `displayDialog` path carries.
///
/// `ui_screen_type` replaces the root attribute when `Some`. Changing it
/// between 1, 2, 3, 4 and 5 has no visual effect on a dialog displayed
/// immediately — all of them register the same window and init function
/// (F4) — but it does decide which lure queue a non-immediate dialog lands
/// in (F10), so it is a label today and a mechanism for DU-04.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DialogPatch {
    pub dialog_id: u32,
    pub ui_screen_type: Option<u32>,
    pub buttons: ButtonPlan,
}

/// Why a patch did not apply. The caller keeps the canonical entry and
/// logs; nothing panics, and one bad row never stops server startup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PatchError {
    /// The cooked entry is not a `<COOKED_DIALOG>` this module fully
    /// understands. See [`super::parse`] for what is accepted.
    Unparsable,
    /// The entry parsed, but [`ButtonPlan::OnlyOn`] named a `screen_id`
    /// that is not in it — a typo, or a screen the cook never shipped.
    /// Landing the button on some other screen would be worse than
    /// landing it nowhere, so the whole patch is refused.
    ScreenMissing { screen_id: u32 },
}

/// Every registered patch table, one per zone.
///
/// Both ship empty (DU-01). Wave 1 fills them: DU-02a takes
/// [`super::patches_cellblock`], DU-02b takes [`super::patches_castle`].
pub const DIALOG_PATCH_TABLES: &[&[DialogPatch]] = &[
    super::patches_cellblock::CELLBLOCK_DIALOG_PATCHES,
    super::patches_castle::CASTLE_DIALOG_PATCHES,
];

/// `true` when no zone table holds a row. Used by the caller to skip the
/// whole patch pass, including the metadata bump.
pub fn no_patches_registered() -> bool {
    DIALOG_PATCH_TABLES.iter().all(|table| table.is_empty())
}

/// Apply one patch to one canonical cooked entry.
///
/// Returns the re-emitted Server-Build bytes on success. On either
/// [`PatchError`] the caller keeps `original` byte-for-byte.
pub fn apply_dialog_patch(original: &[u8], patch: &DialogPatch) -> Result<Vec<u8>, PatchError> {
    let mut dialog = parse_cooked_dialog(original).ok_or(PatchError::Unparsable)?;

    if let Some(ui_screen_type) = patch.ui_screen_type {
        dialog.ui_screen_type = ui_screen_type;
    }

    match &patch.buttons {
        // Identity. Every screen keeps its buttons in document order.
        ButtonPlan::Keep => {}
        ButtonPlan::StripAll => {
            for screen in &mut dialog.screens {
                screen.buttons.clear();
            }
        }
        ButtonPlan::OnlyOn {
            screen_id,
            button_type,
            button_id,
            text,
        } => {
            // Check before mutating: a miss must leave the caller with an
            // untouched entry, not one that has been stripped and not
            // re-populated.
            if !dialog
                .screens
                .iter()
                .any(|s| s.screen_id == Some(*screen_id))
            {
                return Err(PatchError::ScreenMissing {
                    screen_id: *screen_id,
                });
            }
            for screen in &mut dialog.screens {
                screen.buttons.clear();
                if screen.screen_id == Some(*screen_id) {
                    screen.buttons.push(CookedButton {
                        button_type: *button_type,
                        button_id: *button_id,
                        text_escaped: escape_xml_attr(text),
                    });
                }
            }
        }
    }

    Ok(emit_cooked_dialog(&dialog))
}

/// Apply every patch in `tables` to a loaded category's element map.
///
/// Returns the dialog ids actually patched, unsorted and in table order;
/// the caller merges them into the per-category invalid-keys list.
///
/// Skips, each with a `warn!` carrying a stable `reason` field (see
/// `docs/architecture/negative-logging-convention.md`):
///
/// * `dialog_entry_absent` — the id is not in the loaded PAK. Unlike a
///   full regeneration, a patch has nothing to transform, so there is no
///   sensible fallback.
/// * `screen_absent` — [`ButtonPlan::OnlyOn`] named a screen the entry
///   does not have.
/// * `unparsable_entry` — the cooked XML shape drifted.
///
/// None of these aborts the pass or the server: the remaining patches
/// still apply and the skipped entries keep their canonical bytes.
pub fn apply_dialog_patches(
    elements: &mut HashMap<u32, Vec<u8>>,
    tables: &[&[DialogPatch]],
) -> Vec<u32> {
    let mut applied = Vec::new();

    for patch in tables.iter().copied().flatten() {
        let Some(original) = elements.get(&patch.dialog_id) else {
            tracing::warn!(
                dialog_id = patch.dialog_id,
                reason = "dialog_entry_absent",
                "dialog patch skipped: entry not present in the loaded dialog catalogue",
            );
            continue;
        };

        match apply_dialog_patch(original, patch) {
            Ok(patched) => {
                elements.insert(patch.dialog_id, patched);
                applied.push(patch.dialog_id);
                tracing::info!(
                    dialog_id = patch.dialog_id,
                    plan = ?patch.buttons,
                    ui_screen_type = ?patch.ui_screen_type,
                    "Applied Cimmeria dialog patch",
                );
            }
            Err(PatchError::ScreenMissing { screen_id }) => {
                tracing::warn!(
                    dialog_id = patch.dialog_id,
                    screen_id,
                    reason = "screen_absent",
                    "dialog patch skipped: target screen not present in the cooked entry — \
                     keeping the canonical entry",
                );
            }
            Err(PatchError::Unparsable) => {
                tracing::warn!(
                    dialog_id = patch.dialog_id,
                    reason = "unparsable_entry",
                    "dialog patch skipped: cooked XML shape did not parse — \
                     keeping the canonical entry",
                );
            }
        }
    }

    applied
}
