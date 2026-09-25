//! Content-derived `MetaData` bumps for the cooked-data categories.
//!
//! The client compares a category's `MetaData` against its own cached value
//! to decide whether to refresh anything at all. Each bump is hashed from
//! every field that changes what the client would see, so two server starts
//! on identical override content produce the same value (no re-invalidation
//! churn) and any edit produces a different one (the client refetches).
//!
//! Two invariants are shared by all four:
//!
//! * `& 0xFFFF` keeps the bump small enough to stay well inside `u32` on top
//!   of the QA-build MetaData values.
//! * `| 0x1` guarantees it is non-zero. A zero bump would leave `MetaData`
//!   unchanged, the client would never see a mismatch, and the patched entry
//!   would never reach it.
//!
//! Split out of `resources/mod.rs` when the dialog bump grew a second
//! argument and pushed that file past the 700-line cap. See
//! `docs/architecture/mission-pak-overrides.md` for the policy behind them.

/// Compute the deterministic metadata bump for a set of overrides.
///
/// The bump is hashed from every field that affects what the client sees:
/// `mission_id`, `insert_after_step_id`, and the injected XML. Two server
/// starts on the same override content produce the same bump (no
/// re-invalidation churn); changing any field changes the bump (client
/// mismatches and refetches).
///
/// The result is OR'd with 1 so the low bit is always set — guards
/// against the rare hash that lands on `0`, which would leave the
/// metadata unchanged and the client stuck on stale entries.
pub(super) fn compute_metadata_bump(
    overrides: &[crate::base::mission_overrides::MissionOverride],
    text_overrides: &[crate::base::mission_overrides::StepTextOverride],
) -> u32 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    for ov in overrides {
        ov.mission_id.hash(&mut hasher);
        ov.insert_after_step_id.hash(&mut hasher);
        ov.injected_steps_xml.hash(&mut hasher);
    }
    for ov in text_overrides {
        ov.mission_id.hash(&mut hasher);
        ov.step_id.hash(&mut hasher);
        ov.new_step_display_log_text.hash(&mut hasher);
    }
    ((hasher.finish() as u32) & 0xFFFF) | 0x1
}

/// Companion of [`compute_metadata_bump`] for the items category.
/// Same deterministic-hash + low-bit-set discipline so two server
/// starts on identical override content produce identical bumps.
pub(super) fn compute_item_metadata_bump(
    overrides: &[crate::base::item_overrides::ItemOverride],
) -> u32 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    for ov in overrides {
        ov.item_id.hash(&mut hasher);
        ov.new_icon_location.hash(&mut hasher);
        ov.new_max_stack_size.hash(&mut hasher);
    }
    ((hasher.finish() as u32) & 0xFFFF) | 0x1
}

/// Companion of [`compute_metadata_bump`] for the dialogs category.
///
/// Hashes every field that affects the rendered XML, across BOTH override
/// kinds: a full regeneration's id / flags / kismet id / screen type and
/// each screen's id, speaker, text and buttons; and a patch's dialog id,
/// replacement screen type and button plan. Two server starts on identical
/// content produce identical bumps and no re-invalidation churn; any edit
/// to either kind changes the bump and the client refetches.
///
/// Fields are hashed one at a time rather than by hashing the slices, so
/// an empty button list or an empty patch table writes nothing to the
/// hasher. That is what makes shipping the two zone patch tables empty a
/// true no-op: the bump is identical to the pre-patch-engine value and no
/// client refetches for a change it cannot see.
pub(super) fn compute_dialog_metadata_bump(
    overrides: &[crate::base::dialog_overrides::DialogOverride],
    patch_tables: &[&[crate::base::dialog_overrides::DialogPatch]],
) -> u32 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    for ov in overrides {
        ov.dialog_id.hash(&mut hasher);
        ov.dialog_flags.hash(&mut hasher);
        ov.kismet_event_set_id.hash(&mut hasher);
        ov.ui_screen_type.hash(&mut hasher);
        for screen in ov.screens {
            screen.screen_id.hash(&mut hasher);
            screen.speaker_id.hash(&mut hasher);
            screen.text.hash(&mut hasher);
            for button in screen.buttons {
                button.button_type.hash(&mut hasher);
                button.button_id.hash(&mut hasher);
                button.text.hash(&mut hasher);
            }
        }
    }
    for patch in patch_tables.iter().copied().flatten() {
        patch.dialog_id.hash(&mut hasher);
        patch.ui_screen_type.hash(&mut hasher);
        patch.buttons.hash(&mut hasher);
    }
    ((hasher.finish() as u32) & 0xFFFF) | 0x1
}

/// Companion bump for the Kismet sequences category. Hashes every field the
/// client sees, so identical override content gives an identical version
/// across server starts and any edit changes it.
pub(super) fn compute_sequence_metadata_bump(
    overrides: &[crate::base::sequence_overrides::SequenceOverride],
) -> u32 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    for ov in overrides {
        ov.sequence_id.hash(&mut hasher);
        ov.event_id.hash(&mut hasher);
        ov.kismet_script_name.hash(&mut hasher);
    }
    ((hasher.finish() as u32) & 0xFFFF) | 0x1
}
