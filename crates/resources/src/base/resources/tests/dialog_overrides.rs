//! Dialog cooked-data override tests: `compute_dialog_metadata_bump` across
//! both override kinds, and the `apply_dialog_overrides` in-memory mutation
//! path.
//!
//! Split out of [`super::overrides`] when patch-mode overrides pushed that
//! file past the 700-line cap. The engine's own tests live next to the code,
//! in `crates/resources/src/base/dialog_overrides/`.

use super::super::*;

/// Build a `CookedDataDialogs` category fixture. `present` lists dialog ids
/// that already exist in the PAK (with arbitrary stale bytes); the override
/// regenerates them wholesale, so the original content is irrelevant.
fn dialogs_category_with(present: &[u32], metadata: u32) -> CategoryData {
    let mut elements = HashMap::new();
    for &id in present {
        elements.insert(id, format!("<STALE id={id}/>").into_bytes());
    }
    CategoryData { metadata, elements }
}

/// Post-condition pin for the dialog-override path: a registered dialog that
/// already exists in the PAK (3995, Frost) is regenerated wholesale, a brand
/// new one (3996, Guard) is inserted, the metadata is bumped, and the
/// overridden-ids map names both in ascending order. Mirrors the mission/item
/// override post-condition tests.
#[test]
fn apply_dialog_overrides_regenerates_existing_and_inserts_new() {
    let starting_metadata = 4242;
    let mut categories: HashMap<u32, CategoryData> = HashMap::new();
    // 3995 present (stale); 3996 absent so we exercise the new-key insert.
    categories.insert(
        CATEGORY_DIALOGS,
        dialogs_category_with(&[3995], starting_metadata),
    );

    let overridden = ResourceCache::apply_dialog_overrides(&mut categories);

    let dialogs = categories
        .get(&CATEGORY_DIALOGS)
        .expect("dialogs category must remain after apply");

    // Metadata bumped, low bit set.
    assert_ne!(
        dialogs.metadata, starting_metadata,
        "apply must bump the dialogs metadata so the client invalidates and refetches",
    );
    assert_eq!(
        dialogs.metadata.wrapping_sub(starting_metadata) & 0x1,
        0x1,
        "low bit of bump must be set",
    );

    // 3995 regenerated: real Server-Build XML, letter-only, no stale marker.
    let d3995 = std::str::from_utf8(dialogs.elements.get(&3995).unwrap()).unwrap();
    assert!(
        d3995.contains("<COOKED_DIALOG") && d3995.contains("DialogID=\"3995\""),
        "3995 must be regenerated as a COOKED_DIALOG; got: {d3995}",
    );
    assert!(
        !d3995.contains("STALE"),
        "3995's stale PAK bytes must be fully replaced; got: {d3995}",
    );
    assert!(
        d3995.contains("letter") && !d3995.to_lowercase().contains("pistol"),
        "3995 (Frost) must be letter-only after the loot split; got: {d3995}",
    );

    // 3996 inserted (was absent in the fixture).
    let d3996 = std::str::from_utf8(
        dialogs
            .elements
            .get(&3996)
            .expect("3996 must be inserted even though it was absent from the PAK"),
    )
    .unwrap();
    assert!(
        d3996.contains("DialogID=\"3996\"") && d3996.to_lowercase().contains("pistol"),
        "3996 (Guard) must be the pistol-search dialog; got: {d3996}",
    );

    let ids = overridden
        .get(&CATEGORY_DIALOGS)
        .expect("dialogs must appear in the returned map");
    assert_eq!(
        ids.as_slice(),
        &[3995u32, 3996u32],
        "overridden_elements must name both dialog ids in ascending order",
    );
}

/// Defensive path: dialogs category absent (PAK missing) → no-op return,
/// server startup keeps going. Mirrors the mission/item analogues.
#[test]
fn apply_dialog_overrides_no_op_when_category_missing() {
    let mut categories: HashMap<u32, CategoryData> = HashMap::new();
    let overridden = ResourceCache::apply_dialog_overrides(&mut categories);
    assert!(
        overridden.is_empty(),
        "missing dialogs category must not produce override entries"
    );
}

/// `compute_dialog_metadata_bump` is deterministic across calls, always sets
/// the low bit, and changes when any screen field changes (so the client
/// refetches). Companion of the mission/item bump pins.
#[test]
fn compute_dialog_metadata_bump_is_deterministic_and_change_sensitive() {
    use crate::base::dialog_overrides::{DialogOverride, DialogScreen};

    const BASE: &[DialogOverride] = &[DialogOverride {
        dialog_id: 3996,
        dialog_flags: 0,
        kismet_event_set_id: 0,
        ui_screen_type: 2,
        screens: &[DialogScreen {
            screen_id: 96109,
            speaker_id: 0,
            text: "original",
            buttons: &[],
        }],
    }];
    const CHANGED_TEXT: &[DialogOverride] = &[DialogOverride {
        dialog_id: 3996,
        dialog_flags: 0,
        kismet_event_set_id: 0,
        ui_screen_type: 2,
        screens: &[DialogScreen {
            screen_id: 96109,
            speaker_id: 0,
            text: "edited",
            buttons: &[],
        }],
    }];

    let a = compute_dialog_metadata_bump(BASE, &[]);
    assert_eq!(
        a,
        compute_dialog_metadata_bump(BASE, &[]),
        "same content → same bump"
    );
    assert_eq!(a & 0x1, 0x1, "low bit must be set");
    assert_ne!(
        a,
        compute_dialog_metadata_bump(CHANGED_TEXT, &[]),
        "a screen-text edit must change the bump so the client refetches",
    );
}

/// Adding a button to an authored override must change the bump. Without
/// the per-button hashing, a Wave 1 packet that put a button on a
/// Cimmeria-authored dialog would ship XML the client never refetches.
#[test]
fn compute_dialog_metadata_bump_changes_when_an_authored_button_changes() {
    use crate::base::dialog_overrides::{DialogButton, DialogOverride, DialogScreen};

    const NO_BUTTON: &[DialogOverride] = &[DialogOverride {
        dialog_id: 3996,
        dialog_flags: 0,
        kismet_event_set_id: 0,
        ui_screen_type: 2,
        screens: &[DialogScreen {
            screen_id: 96109,
            speaker_id: 0,
            text: "body",
            buttons: &[],
        }],
    }];
    const WITH_BUTTON: &[DialogOverride] = &[DialogOverride {
        dialog_id: 3996,
        dialog_flags: 0,
        kismet_event_set_id: 0,
        ui_screen_type: 2,
        screens: &[DialogScreen {
            screen_id: 96109,
            speaker_id: 0,
            text: "body",
            buttons: &[DialogButton {
                button_type: 2,
                button_id: 8,
                text: "Accept",
            }],
        }],
    }];

    assert_ne!(
        compute_dialog_metadata_bump(NO_BUTTON, &[]),
        compute_dialog_metadata_bump(WITH_BUTTON, &[]),
        "adding a button must change the bump so the client refetches",
    );
}

/// A patch plan participates in the bump: changing the plan, the target
/// screen or the replacement type must re-invalidate, and repeating the
/// same plan must not. Without this, a Wave 1 packet could edit a plan and
/// ship XML that every already-connected client keeps a stale copy of.
#[test]
fn compute_dialog_metadata_bump_tracks_patch_plans() {
    use crate::base::dialog_overrides::patch::{ButtonPlan, DialogPatch};

    const STRIP: &[DialogPatch] = &[DialogPatch {
        dialog_id: 3999,
        ui_screen_type: None,
        buttons: ButtonPlan::StripAll,
    }];
    const ONLY_ON_96825: &[DialogPatch] = &[DialogPatch {
        dialog_id: 3999,
        ui_screen_type: None,
        buttons: ButtonPlan::OnlyOn {
            screen_id: 96825,
            button_type: 4,
            button_id: 71,
            text: "Take Missions",
        },
    }];
    const ONLY_ON_96824: &[DialogPatch] = &[DialogPatch {
        dialog_id: 3999,
        ui_screen_type: None,
        buttons: ButtonPlan::OnlyOn {
            screen_id: 96824,
            button_type: 4,
            button_id: 71,
            text: "Take Missions",
        },
    }];
    const STRIP_AND_RETYPE: &[DialogPatch] = &[DialogPatch {
        dialog_id: 3999,
        ui_screen_type: Some(5),
        buttons: ButtonPlan::StripAll,
    }];

    let strip = compute_dialog_metadata_bump(&[], &[STRIP]);
    assert_eq!(
        strip,
        compute_dialog_metadata_bump(&[], &[STRIP]),
        "the same plan must hash to the same bump across server starts",
    );
    assert_eq!(strip & 0x1, 0x1, "low bit must be set");

    for (label, other) in [
        ("plan variant", ONLY_ON_96825),
        ("target screen", ONLY_ON_96824),
        ("replacement type", STRIP_AND_RETYPE),
    ] {
        assert_ne!(
            strip,
            compute_dialog_metadata_bump(&[], &[other]),
            "a {label} change must change the bump",
        );
    }
    assert_ne!(
        compute_dialog_metadata_bump(&[], &[ONLY_ON_96825]),
        compute_dialog_metadata_bump(&[], &[ONLY_ON_96824]),
        "moving the button to a different screen must change the bump",
    );
}

/// An empty patch table is bump-neutral: it writes nothing to the hasher,
/// so shipping DU-01 with both zone tables empty leaves the dialogs
/// metadata exactly where the pre-patch-engine code left it and no client
/// refetches for a change it cannot see.
#[test]
fn compute_dialog_metadata_bump_is_unchanged_by_an_empty_patch_table() {
    use crate::base::dialog_overrides::{DialogPatch, DIALOG_OVERRIDES};

    const EMPTY: &[DialogPatch] = &[];
    assert_eq!(
        compute_dialog_metadata_bump(DIALOG_OVERRIDES, &[]),
        compute_dialog_metadata_bump(DIALOG_OVERRIDES, &[EMPTY, EMPTY]),
        "empty patch tables must not move the bump",
    );
}
