//! Failure paths, `apply_dialog_patches` over a loaded category, and the
//! invariants on the shipped zone tables.

use std::collections::HashMap;

use super::super::patch::PatchError;
use super::*;
use crate::test_support::LogCapture;

// ── Failure paths ───────────────────────────────────────────────────────

/// A typo'd or never-shipped `screen_id` refuses the whole patch. Landing
/// the button on some other screen would be worse than landing it nowhere,
/// and a half-applied patch (buttons stripped, replacement never added)
/// would be worse still.
#[test]
fn only_on_missing_screen_is_refused_whole() {
    let err = apply_dialog_patch(
        QA_2576.as_bytes(),
        &DialogPatch {
            dialog_id: 2576,
            ui_screen_type: None,
            buttons: ButtonPlan::OnlyOn {
                screen_id: 99999,
                button_type: 4,
                button_id: 71,
                text: "Take Missions",
            },
        },
    )
    .expect_err("a missing screen must not silently succeed");
    assert_eq!(err, PatchError::ScreenMissing { screen_id: 99999 });
}

/// A cooked entry whose shape drifted is left alone rather than guessed
/// at.
#[test]
fn unparsable_entry_is_refused() {
    let err = apply_dialog_patch(
        b"<COOKED_DIALOG DialogFlags=\"0\"></COOKED_DIALOG>",
        &DialogPatch {
            dialog_id: 1,
            ui_screen_type: None,
            buttons: ButtonPlan::StripAll,
        },
    )
    .expect_err("a malformed root must be refused");
    assert_eq!(err, PatchError::Unparsable);
}

/// An entry the parser would once have accepted — a `<Buttons>` nested
/// inside another — is refused, and the batch path leaves its bytes
/// exactly as cooked instead of shipping a flattened rewrite.
#[test]
fn nested_buttons_entry_is_refused_and_left_byte_identical() {
    let nested = concat!(
        "<COOKED_DIALOG DialogFlags=\"0\" DialogID=\"2576\" KismetEventSetID=\"0\" ",
        "UIScreenType=\"2\"><Screens SpeakerID=\"0\" ScreenID=\"96825\" Text=\"a\">",
        "<Buttons ButtonType=\"4\" ButtonID=\"71\" Text=\"Take Missions\">",
        "<Buttons ButtonType=\"2\" ButtonID=\"8\" Text=\"Accept\"></Buttons>",
        "</Buttons></Screens></COOKED_DIALOG>",
    );
    let patch = DialogPatch {
        dialog_id: 2576,
        ui_screen_type: None,
        buttons: ButtonPlan::StripAll,
    };
    assert_eq!(
        apply_dialog_patch(nested.as_bytes(), &patch),
        Err(PatchError::Unparsable),
    );

    let mut elements = elements_with(&[(2576, nested)]);
    let before = elements.clone();
    let applied = apply_dialog_patches(&mut elements, &[&[patch]]);
    assert!(
        applied.is_empty(),
        "a refused entry must not be reported applied"
    );
    assert_eq!(
        elements, before,
        "a refused entry must keep its cooked bytes"
    );
}

// ── apply_dialog_patches over a loaded category ─────────────────────────

fn elements_with(entries: &[(u32, &str)]) -> HashMap<u32, Vec<u8>> {
    entries
        .iter()
        .map(|(id, xml)| (*id, xml.as_bytes().to_vec()))
        .collect()
}

#[test]
fn apply_dialog_patches_patches_listed_ids_and_leaves_others_alone() {
    let mut elements = elements_with(&[(2576, QA_2576), (3999, QA_3999), (2572, QA_2572)]);
    let table: &[DialogPatch] = &[
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
        DialogPatch {
            dialog_id: 3999,
            ui_screen_type: None,
            buttons: ButtonPlan::StripAll,
        },
    ];

    let applied = apply_dialog_patches(&mut elements, &[table]);

    assert_eq!(applied, vec![2576, 3999]);
    assert_eq!(
        elements.get(&2572).map(|v| v.as_slice()),
        Some(QA_2572.as_bytes()),
        "an unlisted dialog must keep its canonical bytes byte-for-byte",
    );
    assert_eq!(
        parse(elements.get(&3999).unwrap())
            .screens
            .iter()
            .filter(|s| !s.buttons.is_empty())
            .count(),
        0,
    );
}

/// Negative-log guard: a patch naming a dialog the PAK does not hold must
/// `warn!` with `reason = "dialog_entry_absent"`. Without the warn a typo
/// in a zone table is invisible — the dialog simply keeps its broken
/// buttons and the soft-lock survives the "fix".
#[test]
fn missing_dialog_entry_warns_and_skips() {
    let capture = LogCapture::install();
    let mut elements = elements_with(&[(2576, QA_2576)]);
    let table: &[DialogPatch] = &[DialogPatch {
        dialog_id: 424242,
        ui_screen_type: None,
        buttons: ButtonPlan::StripAll,
    }];

    let applied = apply_dialog_patches(&mut elements, &[table]);

    assert!(applied.is_empty(), "nothing may be reported as applied");
    let event = capture
        .find_event(
            tracing::Level::WARN,
            "dialog patch skipped",
            "dialog_entry_absent",
        )
        .expect("a missing dialog entry must warn with reason=dialog_entry_absent");
    assert!(event.has_field("dialog_id", "424242"));
}

/// Negative-log guard the packet names explicitly: an `OnlyOn` screen the
/// cooked entry does not have must `warn!` with `reason = "screen_absent"`
/// and leave the entry untouched. This is the guard that fails if the
/// warn is deleted.
#[test]
fn missing_screen_warns_and_leaves_the_entry_untouched() {
    let capture = LogCapture::install();
    let mut elements = elements_with(&[(2576, QA_2576)]);
    let table: &[DialogPatch] = &[DialogPatch {
        dialog_id: 2576,
        ui_screen_type: None,
        buttons: ButtonPlan::OnlyOn {
            screen_id: 99999,
            button_type: 4,
            button_id: 71,
            text: "Take Missions",
        },
    }];

    let applied = apply_dialog_patches(&mut elements, &[table]);

    assert!(applied.is_empty(), "a refused patch is not an applied id");
    assert_eq!(
        elements.get(&2576).map(|v| v.as_slice()),
        Some(QA_2576.as_bytes()),
        "the canonical entry must survive byte-for-byte when the patch is refused",
    );
    let event = capture
        .find_event(
            tracing::Level::WARN,
            "dialog patch skipped",
            "screen_absent",
        )
        .expect("a missing screen must warn with reason=screen_absent");
    assert!(event.has_field("dialog_id", "2576"));
    assert!(event.has_field("screen_id", "99999"));
}

/// Negative-log guard: a cooked entry that no longer parses must warn
/// rather than panic, and must keep its bytes.
#[test]
fn unparsable_entry_warns_and_leaves_the_entry_untouched() {
    let capture = LogCapture::install();
    const DRIFTED: &str = "<COOKED_DIALOG DialogFlags=\"0\"></COOKED_DIALOG>";
    let mut elements = elements_with(&[(2576, DRIFTED)]);
    let table: &[DialogPatch] = &[DialogPatch {
        dialog_id: 2576,
        ui_screen_type: None,
        buttons: ButtonPlan::StripAll,
    }];

    let applied = apply_dialog_patches(&mut elements, &[table]);

    assert!(applied.is_empty());
    assert_eq!(
        elements.get(&2576).map(|v| v.as_slice()),
        Some(DRIFTED.as_bytes()),
    );
    assert!(
        capture
            .find_event(
                tracing::Level::WARN,
                "dialog patch skipped",
                "unparsable_entry"
            )
            .is_some(),
        "an unparsable entry must warn with reason=unparsable_entry",
    );
}

/// A patch can be applied on top of a full-regeneration override, because
/// the emitter's output parses back. Not used today — the two shipped
/// overrides carry no patches — but it means the two kinds compose
/// instead of silently fighting over an id.
#[test]
fn a_patch_composes_on_top_of_a_generated_override() {
    let generated = generate_dialog_xml(&DIALOG_OVERRIDES[0]);
    let patched = apply_dialog_patch(
        &generated,
        &DialogPatch {
            dialog_id: 3995,
            ui_screen_type: Some(5),
            buttons: ButtonPlan::Keep,
        },
    )
    .expect("a generated entry must be patchable");
    let after = parse(&patched);

    assert_eq!(after.ui_screen_type, 5);
    assert_eq!(
        bodies(&parse(&generated)),
        bodies(&after),
        "Frost's text must survive the type patch",
    );
}

// ── The shipped tables ──────────────────────────────────────────────────

/// The shipped tables touch only the dialogs they name: every id reported
/// as applied is a row in a zone table, and every entry no row names comes
/// out byte-identical. Guards against a table that silently rewrites every
/// entry it walks. Holds whether the tables are empty (DU-01) or filled
/// (DU-02a / DU-02b), so neither Wave 1 packet needs to edit this file.
#[test]
fn shipped_patch_tables_touch_only_the_dialogs_they_name() {
    let mut elements = elements_with(&[(2576, QA_2576), (3999, QA_3999), (2572, QA_2572)]);
    let before = elements.clone();
    let named: Vec<u32> = DIALOG_PATCH_TABLES
        .iter()
        .copied()
        .flatten()
        .map(|patch| patch.dialog_id)
        .collect();

    let applied = apply_dialog_patches(&mut elements, DIALOG_PATCH_TABLES);

    for id in &applied {
        assert!(
            named.contains(id),
            "dialog {id} was reported applied but no table row names it",
        );
    }
    for (id, bytes) in &before {
        if !named.contains(id) {
            assert_eq!(
                elements.get(id),
                Some(bytes),
                "dialog {id} is not named by any row and must be untouched",
            );
        }
    }
}

/// No dialog id may appear in more than one zone table, and no table may
/// list the same dialog twice: the second row would silently win and the
/// first would look applied while its plan was discarded. Trivially true
/// while both tables are empty; it is DU-02a and DU-02b landing in
/// parallel that this is here for.
#[test]
fn no_dialog_id_is_claimed_by_two_patch_rows() {
    let mut seen: Vec<u32> = Vec::new();
    for patch in DIALOG_PATCH_TABLES.iter().copied().flatten() {
        assert!(
            !seen.contains(&patch.dialog_id),
            "dialog {} is patched by two rows; merge them into one plan",
            patch.dialog_id,
        );
        seen.push(patch.dialog_id);
    }
}

/// A registered patch must not also be a full-regeneration override:
/// the override regenerates the entry from Rust text and the patch would
/// then transform that, not the canonical cook — almost certainly not
/// what the author meant.
#[test]
fn no_dialog_is_both_regenerated_and_patched() {
    for patch in DIALOG_PATCH_TABLES.iter().copied().flatten() {
        assert!(
            !DIALOG_OVERRIDES
                .iter()
                .any(|ov| ov.dialog_id == patch.dialog_id),
            "dialog {} is both a DialogOverride and a DialogPatch; pick one",
            patch.dialog_id,
        );
    }
}
