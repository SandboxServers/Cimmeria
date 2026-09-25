//! Guards for the rule predicates, on synthetic seeds.
//!
//! The four live-seed rule tests in the parent module all assert "no
//! violations", so every one of them passes on an empty model. These
//! guards are the other half: they drive the SAME predicate functions
//! the live tests call, with data shaped to make each rule fire, and
//! assert both that it fires and that it names the right dialog. A
//! predicate that was loosened to return nothing would fail here even
//! though the live tests went green.
//!
//! R3 and R4 have no violator in the current seed at all, so these
//! guards are the only place either rule is ever observed firing.

use super::rules::{
    enum_labels, r1_violations, r2_violations, r3_violations, r4_violations, untaught_enum_labels,
    R1Exemption, NEVER_ADD_A_BUTTON,
};
use super::seed_model::{Button, ChainRefs, DialogSeed};

/// Build a tiny seed. Each row is `(dialog, index, screen, buttons)`
/// where a button is `(button_id, button_type)`.
fn synthetic(rows: &[(i32, i32, i32, &[(i32, i32)])], ui_screen_type: &str) -> DialogSeed {
    let mut seed = DialogSeed::default();
    for (dialog, index, screen, buttons) in rows {
        seed.ui_screen_type
            .insert(*dialog, ui_screen_type.to_string());
        seed.screens
            .entry(*dialog)
            .or_default()
            .push((*index, *screen));
        for (button_id, button_type) in buttons.iter() {
            seed.buttons.entry(*screen).or_default().push(Button {
                button_id: *button_id,
                button_type: *button_type,
                text: "x".to_string(),
            });
        }
    }
    for list in seed.screens.values_mut() {
        list.sort_unstable();
    }
    seed
}

/// Key each dialog from one synthetic chain, `9000 + n`.
fn keyed_only(dialogs: &[i32]) -> ChainRefs {
    let mut refs = ChainRefs::default();
    for (i, dialog) in dialogs.iter().enumerate() {
        refs.keyed.insert(
            *dialog,
            vec![("synthetic_chains.sql".to_string(), 9000 + i as i32)],
        );
    }
    refs
}

#[test]
fn r1_flags_a_mid_screen_button_and_accepts_both_legal_shapes() {
    let seed = synthetic(
        &[
            // 100: the soft-lock — button on screen 1 of 2.
            (100, 0, 10, &[(71, 4)]),
            (100, 1, 11, &[]),
            // 200: legal — zero buttons, so the close emits -1.
            (200, 0, 20, &[]),
            (200, 1, 21, &[]),
            // 300: legal — button on the final screen.
            (300, 0, 30, &[]),
            (300, 1, 31, &[(8, 2)]),
        ],
        "DUIST_DefaultDialog",
    );
    let violations = r1_violations(&seed, &keyed_only(&[100, 200, 300]), &[]);

    assert_eq!(
        violations.len(),
        1,
        "exactly the mid-screen dialog violates R1; zero-button and button-on-final are \
         both legal shapes. Got: {violations:?}"
    );
    let message = &violations[0];
    assert!(
        message.contains("R1 dialog 100"),
        "the report must name the offending dialog: {message}"
    );
    assert!(
        message.contains("chain 9000"),
        "…and the chain keyed on it, or the author cannot find the wiring: {message}"
    );
    assert!(
        message.contains("11"),
        "…and the final screen id, which is where the fix goes: {message}"
    );
}

#[test]
fn r1_reports_an_allowlist_entry_that_no_longer_violates() {
    // Dialog 100 has been fixed — its button moved to the final screen —
    // but the exemption is still in the list.
    let seed = synthetic(
        &[(100, 0, 10, &[]), (100, 1, 11, &[(71, 4)])],
        "DUIST_DefaultDialog",
    );
    let allowlist = [R1Exemption {
        dialog_id: 100,
        reason: "button on 10, none on final 11",
        removed_by: "DU-TEST",
    }];

    let violations = r1_violations(&seed, &keyed_only(&[100]), &allowlist);
    assert_eq!(
        violations.len(),
        1,
        "a fixed-but-still-exempt dialog must be reported, so the fixing packet is forced \
         to empty the allowlist rather than leaving dead entries behind. Got: {violations:?}"
    );
    assert!(
        violations[0].contains("STALE") && violations[0].contains("dialog 100"),
        "the message must say the entry is stale and name the dialog: {violations:?}"
    );
    assert!(
        violations[0].contains("DU-TEST"),
        "…and name the packet that owns the deletion: {violations:?}"
    );

    // An exemption for a dialog nothing keys any more is equally stale:
    // it can never suppress anything, so it is pure noise.
    let unkeyed = r1_violations(&seed, &ChainRefs::default(), &allowlist);
    assert_eq!(unkeyed.len(), 1, "got: {unkeyed:?}");
    assert!(
        unkeyed[0].contains("STALE") && unkeyed[0].contains("dialog 100"),
        "got: {unkeyed:?}"
    );
}

#[test]
fn r1_suppresses_exactly_the_allowlisted_violator() {
    let seed = synthetic(
        &[
            (100, 0, 10, &[(71, 4)]),
            (100, 1, 11, &[]),
            (400, 0, 40, &[(8, 2)]),
            (400, 1, 41, &[]),
        ],
        "DUIST_DefaultDialog",
    );
    let allowlist = [R1Exemption {
        dialog_id: 100,
        reason: "known",
        removed_by: "DU-TEST",
    }];

    let violations = r1_violations(&seed, &keyed_only(&[100, 400]), &allowlist);
    assert_eq!(
        violations.len(),
        1,
        "the allowlist must suppress 100 and nothing else. Got: {violations:?}"
    );
    assert!(
        violations[0].contains("R1 dialog 400"),
        "the unlisted violator must still be reported: {violations:?}"
    );
}

#[test]
fn final_screen_follows_the_index_column_not_the_screen_id() {
    // Screen 5 is authored last (index 1) but sorts first by id. Under
    // an id sort the "final" screen would be 9, which HAS a button, and
    // this soft-lock would go unreported.
    let seed = synthetic(
        &[(100, 0, 9, &[(8, 2)]), (100, 1, 5, &[])],
        "DUIST_DefaultDialog",
    );
    assert_eq!(
        seed.final_screen(100),
        Some(5),
        "index 1 is the last screen, whatever the ids sort like"
    );

    let violations = r1_violations(&seed, &keyed_only(&[100]), &[]);
    assert_eq!(
        violations.len(),
        1,
        "ordering screens by screen_id instead of index hides this soft-lock entirely: \
         {violations:?}"
    );
}

#[test]
fn r2_flags_a_single_button_added_to_a_protected_dialog() {
    let refs = keyed_only(&[5003]);

    let clean = synthetic(
        &[(5003, 0, 50, &[]), (5003, 1, 51, &[])],
        "DUIST_DefaultDialog",
    );
    assert!(
        r2_violations(&clean, &refs, &NEVER_ADD_A_BUTTON).is_empty(),
        "a zero-button protected dialog is compliant"
    );

    // One button, on the FINAL screen — R1 is satisfied, so only R2 can
    // catch it. That is the entire reason R2 exists as a separate rule.
    let dirty = synthetic(
        &[(5003, 0, 50, &[]), (5003, 1, 51, &[(8, 2)])],
        "DUIST_DefaultDialog",
    );
    assert!(
        r1_violations(&dirty, &refs, &[]).is_empty(),
        "R1 is deliberately blind to a button on the final screen"
    );

    let violations = r2_violations(&dirty, &refs, &NEVER_ADD_A_BUTTON);
    assert_eq!(violations.len(), 1, "got: {violations:?}");
    assert!(
        violations[0].contains("R2 dialog 5003") && violations[0].contains("chain 9000"),
        "the message must name the dialog and its chain: {violations:?}"
    );

    // A dialog not on the protected list is R2's business only if it is
    // on the list — the list is the policy, not "every zero-button
    // dialog".
    let other = synthetic(&[(4242, 0, 60, &[(8, 2)])], "DUIST_DefaultDialog");
    assert!(
        r2_violations(&other, &keyed_only(&[4242]), &NEVER_ADD_A_BUTTON).is_empty(),
        "4242 is not on the protected list"
    );
}

/// The two window families accept DIFFERENT button sets, and each
/// rejects a type the other allows. Running the same layout through both
/// is what proves the rule is per-window rather than one global set.
#[test]
fn r3_applies_a_different_button_set_to_each_window_family() {
    let refs = keyed_only(&[2298]);
    // More Info (1), Accept (2), Take Missions (Generic1, 4).
    let layout: &[(i32, i32)] = &[(9, 1), (8, 2), (71, 4)];

    // BlurbWin draws 1 and 2 — the Generic1 is undrawable.
    let blurb = synthetic(&[(2298, 0, 60, layout)], "DUIST_DefaultBlurb");
    let on_blurb = r3_violations(&blurb, &refs);
    assert_eq!(
        on_blurb.len(),
        1,
        "on a Blurb only the type-4 button is undrawable. Got: {on_blurb:?}"
    );
    assert!(
        on_blurb[0].contains("screen 60") && on_blurb[0].contains("type 4"),
        "the message must name the screen and the offending type: {on_blurb:?}"
    );

    // DialogWin draws 2 and 4-6 — now it is the More Info that is
    // undrawable. This is the case the original Blurb-only rule missed
    // entirely: a type-1 button on a default Dialog is invisible AND
    // suppresses the close event.
    let dialog = synthetic(&[(2298, 0, 60, layout)], "DUIST_DefaultDialog");
    let on_dialog = r3_violations(&dialog, &refs);
    assert_eq!(
        on_dialog.len(),
        1,
        "on a DialogWin only the type-1 More Info is undrawable. Got: {on_dialog:?}"
    );
    assert!(
        on_dialog[0].contains("type 1"),
        "the message must name the More Info button, not the Generic1: {on_dialog:?}"
    );

    // Radio and Realization register the same window as Dialog (F4), so
    // they must accept the same set. A table that special-cased only
    // "DUIST_DefaultDialog" would flag the legal type-4 here.
    for window in ["DUIST_DefaultRadio", "DUIST_DefaultRealization"] {
        let seed = synthetic(&[(2298, 0, 60, &[(8, 2), (71, 4)])], window);
        assert!(
            r3_violations(&seed, &refs).is_empty(),
            "{window} is the same window as DUIST_DefaultDialog (F4) and must accept \
             Accept (2) and Generic1 (4)"
        );
    }

    // A dialog none of the four chain files references is out of scope —
    // this linter does not police the other 5,000 dialogs.
    assert!(
        r3_violations(&blurb, &ChainRefs::default()).is_empty(),
        "R3 covers only dialogs referenced by the four chain seeds"
    );
}

/// The failure must read as a soft-lock, not as a cosmetic complaint.
///
/// The distinction is the whole reason R3 is a correctness rule: the
/// client counts COOKED buttons when deciding whether to emit
/// `dialogButtonChoice(id, -1)`, so an undrawable button both hides
/// itself and silences the close. A reviewer who reads the message as
/// "this button won't render" will deprioritise a progression blocker.
#[test]
fn r3_failure_message_explains_the_silenced_close_event() {
    let seed = synthetic(&[(2298, 0, 60, &[(9, 1)])], "DUIST_DefaultDialog");
    let violations = r3_violations(&seed, &keyed_only(&[2298]));
    assert_eq!(violations.len(), 1, "got: {violations:?}");
    let message = &violations[0];
    assert!(
        message.contains("SOFT-LOCK"),
        "the message must say this is a soft-lock: {message}"
    );
    assert!(
        message.contains("dialogButtonChoice(2298, -1)"),
        "…and name the close event that is being suppressed: {message}"
    );
}

/// A window type the table has never been taught must be reported, not
/// skipped. Skipping is the vacuous-pass shape: R3 would go green on a
/// dialog it has not looked at.
#[test]
fn r3_reports_a_window_type_it_does_not_know() {
    let seed = synthetic(&[(2298, 0, 60, &[(8, 2)])], "DUIST_SomethingNew");
    let violations = r3_violations(&seed, &keyed_only(&[2298]));
    assert_eq!(violations.len(), 1, "got: {violations:?}");
    assert!(
        violations[0].contains("DUIST_SomethingNew")
            && violations[0].contains("drawable_button_types"),
        "the message must name the unknown type and where to teach it: {violations:?}"
    );

    // A referenced dialog with no row in dialogs.sql at all is a
    // dangling reference and equally must not pass in silence.
    let empty = DialogSeed::default();
    let dangling = r3_violations(&empty, &keyed_only(&[2298]));
    assert_eq!(dangling.len(), 1, "got: {dangling:?}");
    assert!(
        dangling[0].contains("no row in dialogs.sql"),
        "got: {dangling:?}"
    );
}

#[test]
fn r4_rejects_tutorial_and_none_as_chain_keys() {
    let refs = keyed_only(&[2298]);

    for window in ["DUIST_DefaultBlurb", "DUIST_DefaultDialog"] {
        let ok = synthetic(&[(2298, 0, 60, &[])], window);
        assert!(
            r4_violations(&ok, &refs).is_empty(),
            "{window} may key a dialog_choice chain"
        );
    }

    // Tutorial renders no cooked buttons at all (TutorialScreen.lua), so
    // the player has nothing to press.
    let tutorial = synthetic(&[(2298, 0, 60, &[])], "DUIST_DefaultTutorial");
    let violations = r4_violations(&tutorial, &refs);
    assert_eq!(violations.len(), 1, "got: {violations:?}");
    assert!(
        violations[0].contains("R4 dialog 2298")
            && violations[0].contains("DUIST_DefaultTutorial")
            && violations[0].contains("chain 9000"),
        "the message must name the dialog, its window type and its chain: {violations:?}"
    );
    assert!(
        violations[0].contains("no cooked buttons"),
        "…and say why a Tutorial cannot carry the interaction: {violations:?}"
    );

    // Type 0 is the "TEMP HACK" Blurb registration (F5).
    let none = synthetic(&[(2298, 0, 60, &[])], "DUIST_None");
    let violations = r4_violations(&none, &refs);
    assert_eq!(violations.len(), 1, "got: {violations:?}");
    assert!(
        violations[0].contains("DUIST_None") && violations[0].contains("TEMP HACK"),
        "got: {violations:?}"
    );

    // R4 is about KEYED dialogs. A Tutorial that is only ever displayed
    // is not a wiring mistake, so it must not be reported.
    let mut displayed_only = ChainRefs::default();
    displayed_only
        .displayed
        .insert(2298, vec![("synthetic_chains.sql".to_string(), 9100)]);
    assert!(
        r4_violations(&tutorial, &displayed_only).is_empty(),
        "R4 covers dialog_choice keys only"
    );

    // A keyed dialog with no seed row cannot fire at all.
    let missing = r4_violations(&DialogSeed::default(), &refs);
    assert_eq!(missing.len(), 1, "got: {missing:?}");
    assert!(
        missing[0].contains("no row in dialogs.sql"),
        "got: {missing:?}"
    );
}

/// The R3 table must stay exhaustive over the real enum, and the parser
/// that reads it must actually find labels.
#[test]
fn enum_labels_are_parsed_and_all_are_taught() {
    let sql = "CREATE TYPE \"EDialogUIScreenType\" AS ENUM (\n\
               -- Ba'al's comment, to prove comments are stripped first\n\
               'DUIST_None',\n    'DUIST_DefaultBlurb'\n);";
    assert_eq!(
        enum_labels(sql),
        vec!["DUIST_None", "DUIST_DefaultBlurb"],
        "the enum parser must read the labels in declaration order"
    );
    assert!(
        untaught_enum_labels(sql).is_empty(),
        "both of those are known to drawable_button_types"
    );

    let with_new = "CREATE TYPE \"EDialogUIScreenType\" AS ENUM ('DUIST_None', 'DUIST_Xyz');";
    assert_eq!(
        untaught_enum_labels(with_new),
        vec!["DUIST_Xyz"],
        "an unknown label must be reported so the table gets taught"
    );

    // A file with no enum in it yields nothing rather than panicking —
    // but the live test asserts the real label list, so an empty parse
    // cannot pass there.
    assert!(enum_labels("SELECT 1;").is_empty());
}
