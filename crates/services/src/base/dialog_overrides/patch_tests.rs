//! Patch-engine tests.
//!
//! Every fixture is an **inline** QA-shape string. `data/cache/*.pak` is a
//! committed binary but the packet forbids reading it from a test, so the
//! fixtures are modelled by hand on the real `_2576` and `_3999` entries
//! (read once, 2026-09-21, as a read-only reference): SOAP namespaces on
//! the root, QA attribute order `SpeakerID Text ScreenID`, raw apostrophes,
//! nested `<Buttons ButtonType ButtonID Text></Buttons>`.

use std::collections::HashMap;

use super::parse::parse_cooked_dialog;
use super::patch::{apply_dialog_patch, ButtonPlan, PatchError};
use super::*;
use crate::test_support::LogCapture;

/// Modelled on `_2576` (Copplemann's offer): five screens, Take Missions
/// (type 4, id 71) on the first three, nothing on the last two. That is
/// the broken shape the hard rule names — a player who reads to the final
/// screen has no button to press, so Done sends `-1`, and the chain that
/// grants missions 702 and 703 never fires.
const QA_2576: &str = concat!(
    "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n",
    "<COOKED_DIALOG xmlns:SOAP-ENV=\"http://schemas.xmlsoap.org/soap/envelope/\" ",
    "xmlns:SOAP-ENC=\"http://schemas.xmlsoap.org/soap/encoding/\" ",
    "xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\" ",
    "xmlns:xsd=\"http://www.w3.org/2001/XMLSchema\" ",
    "xmlns:CookedData1=\"SGW\" ",
    "DialogFlags=\"0\" KismetEventSetID=\"0\" UIScreenType=\"2\" DialogID=\"2576\">",
    "<Screens SpeakerID=\"1110\" Text=\"I need you to rescue Dr. Zuritska.\" ScreenID=\"96821\">",
    "<Buttons ButtonType=\"4\" ButtonID=\"71\" Text=\"Take Missions\"></Buttons></Screens>",
    "<Screens SpeakerID=\"0\" Text=\"Where is he?\" ScreenID=\"96822\">",
    "<Buttons ButtonType=\"4\" ButtonID=\"71\" Text=\"Take Missions\"></Buttons></Screens>",
    "<Screens SpeakerID=\"1110\" Text=\"Romney took him to the Interrogation Block.  \" ",
    "ScreenID=\"96823\">",
    "<Buttons ButtonType=\"4\" ButtonID=\"71\" Text=\"Take Missions\"></Buttons></Screens>",
    "<Screens SpeakerID=\"0\" Text=\"Romney...\" ScreenID=\"96824\"></Screens>",
    "<Screens SpeakerID=\"0\" Text=\"Go find Romney&apos;s files. ",
    "Marsh may have been part of the conspiracy.&#xA;\" ScreenID=\"96825\"></Screens>",
    "</COOKED_DIALOG>",
);

/// Modelled on `_3999` (Future Self hands over a weapon): Receive Item
/// (type 4, id 70) on the leading screens, nothing on the last two. Same
/// read-to-end soft-lock shape as 2576, different button.
const QA_3999: &str = concat!(
    "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n",
    "<COOKED_DIALOG xmlns:SOAP-ENV=\"http://schemas.xmlsoap.org/soap/envelope/\" ",
    "xmlns:CookedData1=\"SGW\" ",
    "DialogFlags=\"0\" KismetEventSetID=\"0\" UIScreenType=\"2\" DialogID=\"3999\">",
    "<Screens SpeakerID=\"256\" ",
    "Text=\"Nice to hold a combat weapon in your hands again, isn't it?\" ScreenID=\"96252\">",
    "<Buttons ButtonType=\"4\" ButtonID=\"70\" Text=\"Receive Item\"></Buttons></Screens>",
    "<Screens SpeakerID=\"0\" Text=\"There's a war coming?\" ScreenID=\"96253\">",
    "<Buttons ButtonType=\"4\" ButtonID=\"70\" Text=\"Receive Item\"></Buttons></Screens>",
    "<Screens SpeakerID=\"256\" Text=\"An enemy called the Straegis will appear. \" ",
    "ScreenID=\"96254\">",
    "<Buttons ButtonType=\"4\" ButtonID=\"70\" Text=\"Receive Item\"></Buttons></Screens>",
    "<Screens SpeakerID=\"0\" Text=\"They still think we're criminals?\" ScreenID=\"96259\">",
    "</Screens>",
    "<Screens SpeakerID=\"256\" Text=\"Use the terminal to free the troopers. \" ",
    "ScreenID=\"96260\"></Screens>",
    "</COOKED_DIALOG>",
);

/// Modelled on `_2572` (Gerschon's Blurb offer): one screen carrying More
/// Info then Accept, in that order.
const QA_2572: &str = concat!(
    "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n",
    "<COOKED_DIALOG xmlns:CookedData1=\"SGW\" ",
    "DialogFlags=\"0\" KismetEventSetID=\"0\" UIScreenType=\"1\" DialogID=\"2572\">",
    "<Screens SpeakerID=\"0\" Text=\"Reinforce Copplemann.\" ScreenID=\"96765\">",
    "<Buttons ButtonType=\"1\" ButtonID=\"9\" Text=\"More Info\"></Buttons>",
    "<Buttons ButtonType=\"2\" ButtonID=\"8\" Text=\"Accept\"></Buttons>",
    "</Screens></COOKED_DIALOG>",
);

fn parse(xml: &[u8]) -> CookedDialog {
    parse_cooked_dialog(xml).expect("fixture must parse")
}

/// Every screen's speaker, id and text, for before/after comparison.
fn bodies(d: &CookedDialog) -> Vec<(u32, Option<u32>, String)> {
    d.screens
        .iter()
        .map(|s| (s.speaker_id, s.screen_id, s.text_escaped.clone()))
        .collect()
}

// ── ButtonPlan::OnlyOn ──────────────────────────────────────────────────

/// The DU-02b shape for 2576: move Take Missions onto the final screen.
/// Exactly one button survives, it is on 96825, and every screen's text
/// and speaker is byte-identical to the canonical entry.
#[test]
fn only_on_leaves_exactly_one_button_on_the_named_screen() {
    let before = parse(QA_2576.as_bytes());
    let patched = apply_dialog_patch(
        QA_2576.as_bytes(),
        &DialogPatch {
            dialog_id: 2576,
            ui_screen_type: None,
            buttons: ButtonPlan::OnlyOn {
                screen_id: 96825,
                button_type: 4,
                button_id: 71,
                text: "Take Missions",
            },
        },
    )
    .expect("2576 patch must apply");
    let after = parse(&patched);

    let with_buttons: Vec<_> = after
        .screens
        .iter()
        .filter(|s| !s.buttons.is_empty())
        .collect();
    assert_eq!(
        with_buttons.len(),
        1,
        "exactly one screen may keep a button, got {:?}",
        after
            .screens
            .iter()
            .map(|s| (s.screen_id, s.buttons.len()))
            .collect::<Vec<_>>(),
    );
    assert_eq!(with_buttons[0].screen_id, Some(96825));
    assert_eq!(with_buttons[0].buttons.len(), 1);
    assert_eq!(with_buttons[0].buttons[0].button_type, 4);
    assert_eq!(with_buttons[0].buttons[0].button_id, 71);
    assert_eq!(with_buttons[0].buttons[0].text_escaped, "Take Missions");

    assert_eq!(
        bodies(&before),
        bodies(&after),
        "every screen's speaker, id and text must survive the patch verbatim",
    );
    assert_eq!(after.dialog_id, 2576);
    assert_eq!(after.ui_screen_type, 2, "type must be untouched when None");
}

/// The three screens that carried Take Missions must lose it. Written as
/// its own assertion because "one screen has a button" would also pass if
/// the patcher had added a fourth button and stripped nothing — the
/// count above catches that, this names the bug shape.
#[test]
fn only_on_strips_the_mid_dialog_buttons_it_replaces() {
    let patched = apply_dialog_patch(
        QA_2576.as_bytes(),
        &DialogPatch {
            dialog_id: 2576,
            ui_screen_type: None,
            buttons: ButtonPlan::OnlyOn {
                screen_id: 96825,
                button_type: 4,
                button_id: 71,
                text: "Take Missions",
            },
        },
    )
    .expect("patch must apply");
    let after = parse(&patched);

    for screen_id in [96821, 96822, 96823] {
        let screen = after
            .screens
            .iter()
            .find(|s| s.screen_id == Some(screen_id))
            .unwrap_or_else(|| panic!("screen {screen_id} must survive"));
        assert!(
            screen.buttons.is_empty(),
            "screen {screen_id} must lose its mid-dialog Take Missions button",
        );
    }
}

/// The emitted bytes, in full. Pins the nesting, the attribute order and
/// the escaping of the untouched final-screen text in one place.
#[test]
fn only_on_emits_expected_server_build_bytes() {
    let patched = apply_dialog_patch(
        QA_2576.as_bytes(),
        &DialogPatch {
            dialog_id: 2576,
            ui_screen_type: None,
            buttons: ButtonPlan::OnlyOn {
                screen_id: 96825,
                button_type: 4,
                button_id: 71,
                text: "Take Missions",
            },
        },
    )
    .expect("patch must apply");

    assert_eq!(
        String::from_utf8(patched).unwrap(),
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\
         <COOKED_DIALOG DialogFlags=\"0\" DialogID=\"2576\" KismetEventSetID=\"0\" \
         UIScreenType=\"2\">\
         <Screens SpeakerID=\"1110\" ScreenID=\"96821\" \
         Text=\"I need you to rescue Dr. Zuritska.\"></Screens>\
         <Screens SpeakerID=\"0\" ScreenID=\"96822\" Text=\"Where is he?\"></Screens>\
         <Screens SpeakerID=\"1110\" ScreenID=\"96823\" \
         Text=\"Romney took him to the Interrogation Block.  \"></Screens>\
         <Screens SpeakerID=\"0\" ScreenID=\"96824\" Text=\"Romney...\"></Screens>\
         <Screens SpeakerID=\"0\" ScreenID=\"96825\" \
         Text=\"Go find Romney&apos;s files. Marsh may have been part of the \
         conspiracy.&#xA;\">\
         <Buttons ButtonType=\"4\" ButtonID=\"71\" Text=\"Take Missions\"></Buttons>\
         </Screens>\
         </COOKED_DIALOG>",
    );
}

// ── ButtonPlan::StripAll ────────────────────────────────────────────────

/// The DU-02a shape for 3999: every Receive Item goes, the dialog then
/// closes with `dialogButtonChoice(3999, -1)` and the chain fires however
/// far the player read. Text and speakers untouched.
#[test]
fn strip_all_leaves_no_buttons_and_no_text_changes() {
    let before = parse(QA_3999.as_bytes());
    assert_eq!(
        before
            .screens
            .iter()
            .filter(|s| !s.buttons.is_empty())
            .count(),
        3,
        "fixture must start with the broken mid-dialog buttons",
    );

    let patched = apply_dialog_patch(
        QA_3999.as_bytes(),
        &DialogPatch {
            dialog_id: 3999,
            ui_screen_type: None,
            buttons: ButtonPlan::StripAll,
        },
    )
    .expect("3999 patch must apply");
    let after = parse(&patched);

    assert!(
        after.screens.iter().all(|s| s.buttons.is_empty()),
        "StripAll must leave no button anywhere, got {:?}",
        after
            .screens
            .iter()
            .map(|s| (s.screen_id, s.buttons.len()))
            .collect::<Vec<_>>(),
    );
    assert_eq!(after.screens.len(), 5, "no screen may be dropped");
    assert_eq!(
        bodies(&before),
        bodies(&after),
        "every screen's speaker, id and text must survive the strip verbatim",
    );
    assert!(
        !String::from_utf8(emit_cooked_dialog(&after))
            .unwrap()
            .contains("<Buttons"),
        "no <Buttons> element may reach the wire",
    );
}

/// Raw apostrophes are what the QA cook actually wrote (`isn't`, not
/// `isn&apos;t`). Preserving the escaped form verbatim means they stay
/// raw; a decode-then-re-encode round trip would rewrite them, changing
/// bytes the patch promised not to touch.
#[test]
fn strip_all_preserves_the_cooks_own_escaping_choices() {
    let patched = apply_dialog_patch(
        QA_3999.as_bytes(),
        &DialogPatch {
            dialog_id: 3999,
            ui_screen_type: None,
            buttons: ButtonPlan::StripAll,
        },
    )
    .expect("patch must apply");
    let s = String::from_utf8(patched).unwrap();

    assert!(
        s.contains("isn't it?"),
        "a raw apostrophe must stay raw, not become &apos;: {s}",
    );
    assert!(
        !s.contains("isn&apos;t"),
        "text must not be re-escaped by the round trip: {s}",
    );
}

/// `&#xA;` must survive as a character reference. Decoding it to a literal
/// newline inside an attribute would let the client's XML reader normalise
/// it to a space, silently reflowing the line.
#[test]
fn patch_preserves_numeric_character_references() {
    let patched = apply_dialog_patch(
        QA_2576.as_bytes(),
        &DialogPatch {
            dialog_id: 2576,
            ui_screen_type: None,
            buttons: ButtonPlan::StripAll,
        },
    )
    .expect("patch must apply");
    let s = String::from_utf8(patched).unwrap();

    assert!(
        s.contains("conspiracy.&#xA;\""),
        "the &#xA; newline reference must survive verbatim: {s}",
    );
    assert!(
        !s.contains("conspiracy.\n"),
        "it must not be decoded to a literal newline: {s}",
    );
    assert!(
        s.contains("Romney&apos;s"),
        "an escaped apostrophe must stay escaped: {s}",
    );
}

/// Authored button text IS escaped on the way in — the one place the
/// patcher writes new text rather than copying it.
#[test]
fn authored_button_text_is_escaped() {
    let patched = apply_dialog_patch(
        QA_2576.as_bytes(),
        &DialogPatch {
            dialog_id: 2576,
            ui_screen_type: None,
            buttons: ButtonPlan::OnlyOn {
                screen_id: 96825,
                button_type: 4,
                button_id: 71,
                text: "Take \"All\" & Go",
            },
        },
    )
    .expect("patch must apply");
    let s = String::from_utf8(patched).unwrap();

    assert!(
        s.contains("Text=\"Take &quot;All&quot; &amp; Go\"></Buttons>"),
        "authored button text must be escaped before it lands in the attribute: {s}",
    );
}

// ── ButtonPlan::Keep and ui_screen_type ─────────────────────────────────

/// `Keep` is an identity on buttons, including their order. Order is
/// load-bearing: the client turns a click into a position in this array
/// and sends whichever `ButtonID` sits there, so swapping More Info and
/// Accept on 2572 would make "More Info" send 8.
#[test]
fn keep_preserves_buttons_and_their_order() {
    let before = parse(QA_2572.as_bytes());
    let patched = apply_dialog_patch(
        QA_2572.as_bytes(),
        &DialogPatch {
            dialog_id: 2572,
            ui_screen_type: None,
            buttons: ButtonPlan::Keep,
        },
    )
    .expect("patch must apply");
    let after = parse(&patched);

    assert_eq!(
        before, after,
        "Keep with no type change must be an identity"
    );
    assert_eq!(
        after.screens[0]
            .buttons
            .iter()
            .map(|b| (b.button_type, b.button_id))
            .collect::<Vec<_>>(),
        vec![(1, 9), (2, 8)],
        "More Info must stay at index 0 and Accept at index 1",
    );
}

/// A type-only patch (the DU-04 / DU-05 shape) rewrites the root
/// attribute and nothing else.
#[test]
fn ui_screen_type_is_replaced_without_touching_buttons_or_text() {
    let before = parse(QA_2572.as_bytes());
    let patched = apply_dialog_patch(
        QA_2572.as_bytes(),
        &DialogPatch {
            dialog_id: 2572,
            ui_screen_type: Some(5),
            buttons: ButtonPlan::Keep,
        },
    )
    .expect("patch must apply");
    let after = parse(&patched);

    assert_eq!(before.ui_screen_type, 1, "fixture starts as a Blurb");
    assert_eq!(after.ui_screen_type, 5, "type must be rewritten");
    assert_eq!(bodies(&before), bodies(&after), "text must be untouched");
    assert_eq!(
        before.screens[0].buttons, after.screens[0].buttons,
        "Keep must leave the buttons alone even with a type change",
    );
    assert!(String::from_utf8(emit_cooked_dialog(&after))
        .unwrap()
        .contains("UIScreenType=\"5\""));
}

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
