//! Guards for the rule predicates, on synthetic seeds.
//!
//! The three live-seed tests in the parent module all assert "no
//! violations", so every one of them passes on an empty model. These
//! guards are the other half: they drive the SAME predicate functions
//! the live tests call, with data shaped to make each rule fire, and
//! assert both that it fires and that it names the right dialog. A
//! predicate that was loosened to return nothing would fail here even
//! though the live tests went green.

use super::rules::{r1_violations, r2_violations, r3_violations, R1Exemption, NEVER_ADD_A_BUTTON};
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

#[test]
fn r3_flags_non_blurb_button_types_and_only_on_referenced_blurbs() {
    let refs = keyed_only(&[2298]);
    // More Info (1) + Accept (2) are legal; Take Missions (type 4) is
    // not — BlurbWin never draws it.
    let layout: &[(i32, i32)] = &[(9, 1), (8, 2), (71, 4)];

    let blurb = synthetic(&[(2298, 0, 60, layout)], "DUIST_DefaultBlurb");
    let violations = r3_violations(&blurb, &refs);
    assert_eq!(
        violations.len(),
        1,
        "only the type-4 button is illegal on a Blurb. Got: {violations:?}"
    );
    assert!(
        violations[0].contains("screen 60") && violations[0].contains("type 4"),
        "the message must name the screen and the offending type: {violations:?}"
    );

    // The same layout on a Dialog is legal: 2576's real "Take Missions"
    // is a type-4 button on a DUIST_DefaultDialog and must not be
    // flagged here.
    let dialog = synthetic(&[(2298, 0, 60, layout)], "DUIST_DefaultDialog");
    assert!(
        r3_violations(&dialog, &refs).is_empty(),
        "R3 applies to Blurbs only"
    );

    // A Blurb that none of the four chain files references is out of
    // scope — this linter does not police the other 5,000 dialogs.
    assert!(
        r3_violations(&blurb, &ChainRefs::default()).is_empty(),
        "R3 covers only Blurbs referenced by the four chain seeds"
    );
}
