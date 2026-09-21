//! Linter for dialogs that key a `dialog_choice` content chain but whose
//! buttons stop before their final screen — the shape that soft-locks a
//! player who reads the dialog to the end.
//!
//! # The bug shape
//!
//! Read from the client, not inferred. The facts are the Client Contract
//! rows F6-F9 of `docs/analysis/dialog-ui-redesign/work-packets.md`:
//!
//! * **F8.** Closing a dialog that has ZERO buttons sends
//!   `dialogButtonChoice(dialogId, -1)`. Closing a dialog that has ANY
//!   button sends NOTHING. Clicking a button sends its cooked
//!   `ButtonID`.
//! * **F9.** `dialog_choice` chains match on the dialog id alone
//!   (`crates/content-engine/src/triggers/matching.rs:139-140`). There is
//!   no authorable `button_id` condition yet, so every button on a keyed
//!   dialog fires the same chain, and the close of a zero-button dialog
//!   fires it too.
//! * **F6.** Next, Previous and Done are client chrome on `DialogWin`.
//!   Done is a close, not a button.
//!
//! Put together: a dialog whose buttons sit on screens 1..n-1 but not on
//! screen n offers a player who pages to the end only Done. Done closes,
//! the dialog has buttons, so nothing goes on the wire — and the chain
//! keyed on that dialog never fires. Nothing errors anywhere; from the
//! server's side the interaction simply never happened. Dialogs 3999,
//! 5861 and 2576 ship in exactly that shape today and are carried in
//! `rules::R1_ALLOWLIST` until DU-02a and DU-02b fix them.
//!
//! # The three rules
//!
//! * **R1** — every dialog id that keys a `dialog_choice` chain in the
//!   four Castle / Cellblock chain seeds has either zero button rows, or
//!   at least one button row on its final screen.
//! * **R2** — the inherited never-add-a-button list stays zero-button.
//!   Those dialogs advance only through F8's `-1`, and a button on the
//!   final screen would satisfy R1 while still silencing them, so R2 is
//!   a separate rule rather than a special case of R1.
//! * **R3** — a Blurb (`ui_screen_type = 'DUIST_DefaultBlurb'`)
//!   referenced by those chain files carries only button types 1 (More
//!   Info) and 2 (Accept). `BlurbWin` renders nothing else and has no
//!   Next (F7), so any other type is drawn nowhere and clickable never.
//!
//! # Seed versus cooked data
//!
//! The client draws its buttons from its own cooked entry, not from
//! these tables (F1), so this linter is a proxy: it lints the seed that
//! the override generator and the chain authors read. The proxy was
//! checked against `data/cache/CookedDataDialogs.pak` on 2026-09-21 for
//! all three allowlisted dialogs — cooked `<Screens ScreenID>` order and
//! `<Buttons ButtonType ButtonID Text>` rows match the seed exactly. The
//! pak is not in git and nothing here depends on it.
//!
//! # Layout
//!
//! Cargo fixes the entry point at `tests/dialog_button_linter.rs` and
//! resolves a bare `mod` from a test root against `tests/` itself, where
//! every `.rs` file becomes its own test target. So the submodules live
//! in `tests/dialog_button_linter/` and are pulled in with `#[path]`;
//! the usual `foo/mod.rs` house style cannot apply here. `sql_scan`
//! explains why these seeds cannot be read a line at a time the way
//! `interact_tag_linter.rs` reads its own.

#[path = "dialog_button_linter/rule_guards.rs"]
mod rule_guards;
#[path = "dialog_button_linter/rules.rs"]
mod rules;
#[path = "dialog_button_linter/seed_model.rs"]
mod seed_model;
#[path = "dialog_button_linter/sql_scan.rs"]
mod sql_scan;

use rules::{r1_violations, r2_violations, r3_violations, NEVER_ADD_A_BUTTON, R1_ALLOWLIST};
use seed_model::{
    load_chain_refs, load_dialog_seed, read, workspace_root, ChainRefs, DialogSeed, CHAIN_FILES,
};

// ---------------------------------------------------------------------
// The three rules, against the live seed
// ---------------------------------------------------------------------

#[test]
fn chain_keyed_dialogs_have_a_button_on_their_final_screen_or_none_at_all() {
    let root = workspace_root();
    let seed = load_dialog_seed(&root);
    let refs = load_chain_refs(&root);

    let violations = r1_violations(&seed, &refs, &R1_ALLOWLIST);
    assert!(
        violations.is_empty(),
        "dialog button linter (R1) found {n} problem(s):\n{body}\n\n\
         A STALE message means the opposite of a soft-lock: the dialog has been fixed, so \
         its entry in R1_ALLOWLIST (tests/dialog_button_linter/rules.rs) must be deleted \
         in the same commit as the fix.",
        n = violations.len(),
        body = violations.join("\n"),
    );
}

#[test]
fn never_add_a_button_dialogs_still_have_zero_buttons() {
    let root = workspace_root();
    let seed = load_dialog_seed(&root);
    let refs = load_chain_refs(&root);

    let violations = r2_violations(&seed, &refs, &NEVER_ADD_A_BUTTON);
    assert!(
        violations.is_empty(),
        "dialog button linter (R2) found {n} problem(s):\n{body}",
        n = violations.len(),
        body = violations.join("\n"),
    );
}

#[test]
fn blurbs_referenced_by_castle_chains_use_only_more_info_and_accept() {
    let root = workspace_root();
    let seed = load_dialog_seed(&root);
    let refs = load_chain_refs(&root);

    let violations = r3_violations(&seed, &refs);
    assert!(
        violations.is_empty(),
        "dialog button linter (R3) found {n} problem(s):\n{body}",
        n = violations.len(),
        body = violations.join("\n"),
    );
}

// ---------------------------------------------------------------------
// Non-vacuity — a linter that parses nothing passes everything
// ---------------------------------------------------------------------

/// Every `INSERT` row of every scanned dialog seed must come back out of
/// the scanner.
///
/// This is the guard that matters most here. All three rules above
/// assert "no violations", so a scanner that quietly returned an empty
/// model would make every one of them green. Comparing the parsed row
/// count against the raw count of `INSERT INTO <table> ` occurrences
/// makes the scan self-checking: a lexer that merges two statements
/// loses a row, and one that splits inside a string literal leaves a
/// fragment with no INSERT prefix. Either way the two counts diverge.
#[test]
fn every_insert_row_in_the_dialog_seeds_is_parsed() {
    let root = workspace_root();
    let dir = root.join("db/resources/Dialogs/Seed");
    let seed = load_dialog_seed(&root);

    let cases: [(&str, &str, usize); 3] = [
        ("dialogs.sql", "dialogs", seed.ui_screen_type.len()),
        (
            "dialog_screens.sql",
            "dialog_screens",
            seed.screens.values().map(Vec::len).sum(),
        ),
        (
            "dialog_screen_buttons.sql",
            "dialog_screen_buttons",
            seed.buttons.values().map(Vec::len).sum(),
        ),
    ];
    for (file, table, parsed) in cases {
        let raw = read(&dir.join(file));
        let expected = raw.matches(&format!("INSERT INTO {table} ")).count();
        assert!(
            expected > 1000,
            "{file}: only {expected} raw INSERT statements — wrong file, or an emptied seed"
        );
        assert_eq!(
            parsed, expected,
            "{file}: parsed {parsed} rows but the file holds {expected} \
             `INSERT INTO {table}` statements. The scanner is losing rows — most likely \
             the 1,013 `dialog_screens` rows whose text contains a raw newline."
        );
    }
}

/// The three allowlisted dialogs must still ship the exact broken layout
/// the allowlist records.
///
/// Pinning the layout, not merely "it violates R1", keeps the allowlist
/// honest in both directions. If DU-02a strips 3999's buttons or DU-02b
/// moves 2576's onto screen 96825, this test fails alongside the stale
/// entry message, so the fix and the allowlist deletion have to land
/// together. It also documents the bug shape for a reader who never
/// opens the seed.
#[test]
fn allowlisted_dialogs_still_ship_the_soft_locking_layout() {
    let seed = load_dialog_seed(&workspace_root());

    // (dialog, screens, screens carrying buttons, final screen)
    let expected: [(i32, usize, usize, i32); 3] = [
        (3999, 9, 7, 96260),
        (5861, 8, 5, 96789),
        (2576, 5, 3, 96825),
    ];
    for (dialog, screens, with_buttons, final_screen) in expected {
        assert_eq!(
            seed.screens_in_order(dialog).len(),
            screens,
            "dialog {dialog}: screen count changed"
        );
        assert_eq!(
            seed.screens_with_buttons(dialog).len(),
            with_buttons,
            "dialog {dialog}: the number of screens carrying buttons changed. If a fixing \
             packet did this, delete the dialog's R1_ALLOWLIST entry and update this row."
        );
        assert_eq!(
            seed.final_screen(dialog),
            Some(final_screen),
            "dialog {dialog}: final screen changed"
        );
        assert!(
            seed.buttons_on(final_screen).is_empty(),
            "dialog {dialog}: final screen {final_screen} now carries buttons — the \
             soft-lock is fixed and the R1_ALLOWLIST entry is stale"
        );
    }
}

/// The chain scan must find real work for the rules to do.
///
/// R1 iterates the keyed set and R3 the referenced set; both pass
/// trivially on empty maps. 2300 and 5003 are named because they are
/// zero-button, never-add-a-button dialogs keyed from two different
/// files: if the trigger scan breaks, R2's subjects stop being tied to
/// any chain at all and its failure messages lose their evidence.
#[test]
fn the_chain_scan_finds_the_dialogs_it_is_supposed_to_lint() {
    let root = workspace_root();
    let seed = load_dialog_seed(&root);
    let refs = load_chain_refs(&root);

    assert!(
        refs.keyed.len() >= 15,
        "only {n} dialog_choice-keyed dialogs found across {CHAIN_FILES:?} — the trigger \
         scan is losing rows",
        n = refs.keyed.len(),
    );
    for dialog in [2300, 5003, 2576, 3999, 5861] {
        assert!(
            refs.keyed.contains_key(&dialog),
            "dialog {dialog} must be found as a dialog_choice key; got {keys:?}",
            keys = refs.keyed.keys().collect::<Vec<_>>(),
        );
    }
    // Two chains key 2300, three key 2576. A scanner that collapsed
    // repeated keys, or that read only one trigger row per file, would
    // still satisfy `contains_key`.
    assert_eq!(refs.keyed[&2300].len(), 2, "dialog 2300 is keyed twice");
    assert_eq!(
        refs.keyed[&2576].len(),
        3,
        "dialog 2576 is keyed three times"
    );

    assert!(
        refs.displayed.len() >= 20,
        "only {n} display_dialog targets found — the action scan is losing rows",
        n = refs.displayed.len(),
    );
    // 5862 is displayed only from a MULTI-ROW `VALUES` list
    // (castle_701_chains.sql, chain 1205), so it is absent unless the
    // tuple scanner walks past the first tuple of an INSERT.
    assert!(
        refs.displayed.contains_key(&5862),
        "dialog 5862 is displayed from a multi-row VALUES list; missing it means only the \
         first tuple of each INSERT is being read"
    );

    assert_r3_has_subjects(&seed, &refs);
}

/// R3 needs referenced Blurbs that actually carry buttons, or its inner
/// loop never executes and the rule is decoration.
fn assert_r3_has_subjects(seed: &DialogSeed, refs: &ChainRefs) {
    let blurbs: Vec<i32> = refs
        .referenced()
        .into_iter()
        .filter(|d| seed.is_blurb(*d))
        .collect();
    assert!(
        blurbs.len() >= 3,
        "R3 has only {n} referenced Blurb(s) to check: {blurbs:?}",
        n = blurbs.len(),
    );
    assert!(
        blurbs.contains(&2298),
        "Blurb 2298 (mission 639's offer, More Info + Accept) must be in the referenced \
         set; got {blurbs:?}"
    );
    let button_rows: usize = blurbs
        .iter()
        .flat_map(|d| seed.screens_in_order(*d))
        .map(|s| seed.buttons_on(s).len())
        .sum();
    assert!(
        button_rows >= 5,
        "the referenced Blurbs carry only {button_rows} button rows between them — R3 is \
         inspecting nothing"
    );
}
