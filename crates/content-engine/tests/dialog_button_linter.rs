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
//! # The four rules
//!
//! R1 and R2 are about WHERE the buttons sit. R3 and R4 are about
//! whether the window can draw them at all. All four end in the same
//! failure for the player, because the client counts COOKED buttons —
//! not rendered ones — when it decides whether to emit the close event.
//!
//! * **R1** — every dialog id that keys a `dialog_choice` chain in the
//!   four Castle / Cellblock chain seeds has either zero button rows, or
//!   at least one button row on its final screen.
//! * **R2** — the inherited never-add-a-button list stays zero-button.
//!   Those dialogs advance only through F8's `-1`, and a button on the
//!   final screen would satisfy R1 while still silencing them, so R2 is
//!   a separate rule rather than a special case of R1.
//! * **R3** — every button on a chain-referenced dialog is one its own
//!   window can draw: `BlurbWin` draws More Info (1) and Accept (2),
//!   `DialogWin` draws Accept (2) and Generic1-3 (4, 5, 6), and Radio
//!   and Realization are the same window as Dialog (F4). This is a
//!   soft-lock rule, not a style rule — the client counts COOKED
//!   buttons to decide whether to send the `-1` close, so an undrawable
//!   button is invisible to the player and silences the close event at
//!   the same time.
//! * **R4** — a chain-keyed dialog is a window that can carry the
//!   interaction at all. Tutorial draws no cooked buttons, and
//!   `DUIST_None` is the type-0 "TEMP HACK" Blurb, so keying either is
//!   a wiring mistake rather than a layout one.
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

use rules::{
    enum_labels, r1_violations, r2_violations, r3_violations, r4_violations, untaught_enum_labels,
    NEVER_ADD_A_BUTTON, R1_ALLOWLIST,
};
use seed_model::{
    load_chain_refs, load_dialog_seed, read, workspace_root, ChainRefs, DialogSeed, CHAIN_FILES,
};

// ---------------------------------------------------------------------
// The four rules, against the live seed
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
fn chain_referenced_dialogs_only_carry_buttons_their_window_can_draw() {
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

#[test]
fn chain_keyed_dialogs_are_windows_that_can_carry_the_interaction() {
    let root = workspace_root();
    let seed = load_dialog_seed(&root);
    let refs = load_chain_refs(&root);

    let violations = r4_violations(&seed, &refs);
    assert!(
        violations.is_empty(),
        "dialog button linter (R4) found {n} problem(s):\n{body}",
        n = violations.len(),
        body = violations.join("\n"),
    );
}

/// Every `EDialogUIScreenType` label must be known to the R3 table.
///
/// R3 reports an unrecognised window type rather than skipping it, so a
/// new enum value cannot pass silently — but it would pass *noisily*,
/// one message per referenced dialog, long after the value shipped.
/// Reading the labels straight out of the schema turns that into one
/// clear failure at the point the enum changes. It also pins the
/// six-label set that DU-04 and DU-05 depend on: `DUIST_DefaultRadio`
/// and `DUIST_DefaultRealization` exist in the type but no seed row uses
/// them yet (F11), so nothing else in the suite would notice if they
/// were dropped.
#[test]
fn every_ui_screen_type_label_is_known_to_the_button_rules() {
    let enum_sql =
        read(&workspace_root().join("db/resources/Dialogs/Types/EDialogUIScreenType.sql"));
    let labels = enum_labels(&enum_sql);
    assert_eq!(
        labels,
        vec![
            "DUIST_None",
            "DUIST_DefaultBlurb",
            "DUIST_DefaultDialog",
            "DUIST_DefaultTutorial",
            "DUIST_DefaultRadio",
            "DUIST_DefaultRealization",
        ],
        "the enum's labels, in declaration order — the ordinal is what the cooked \
         UIScreenType byte carries (Blurb 1, Dialog 2, Tutorial 3, Radio 4, Realization 5, \
         and no Lua constant for 0)"
    );

    let untaught = untaught_enum_labels(&enum_sql);
    assert!(
        untaught.is_empty(),
        "ui_screen_type label(s) {untaught:?} have no entry in \
         drawable_button_types() (tests/dialog_button_linter/rules.rs). Until they do, R3 \
         cannot check any dialog that uses them."
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

/// The three dialogs that shipped in the soft-locking shape stay fixed.
///
/// 3999, 5861 and 2576 were the only R1 violators in the Castle and
/// Castle_CellBlock seeds: each carried a button on early screens and none
/// on the last, so a player who read to the end and pressed Done fired
/// nothing. DU-02a stripped 3999; DU-02b moved 5861's Accept and 2576's
/// Take Missions onto their final screens. R1 would catch a regression,
/// but only as "some dialog violates R1"; this names the three and the
/// exact fixed layout, and documents the bug shape for a reader who never
/// opens the seed.
#[test]
fn the_three_former_soft_locks_stay_fixed() {
    let seed = load_dialog_seed(&workspace_root());

    assert!(
        seed.screens_with_buttons(3999).is_empty(),
        "dialog 3999 must stay zero-button: it advances through the -1 close"
    );

    // (dialog, screens, final screen)
    let one_button_on_final: [(i32, usize, i32); 2] = [(5861, 8, 96789), (2576, 5, 96825)];
    for (dialog, screens, final_screen) in one_button_on_final {
        assert_eq!(
            seed.screens_in_order(dialog).len(),
            screens,
            "dialog {dialog}: screen count changed"
        );
        assert_eq!(
            seed.final_screen(dialog),
            Some(final_screen),
            "dialog {dialog}: final screen changed"
        );
        assert_eq!(
            seed.screens_with_buttons(dialog).len(),
            1,
            "dialog {dialog}: exactly one screen may carry a button"
        );
        assert!(
            !seed.buttons_on(final_screen).is_empty(),
            "dialog {dialog}: the button must sit on final screen {final_screen}, or a \
             player who reads to the end is soft-locked again"
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

/// R3 needs referenced dialogs of BOTH window families actually carrying
/// buttons, or its inner loop never executes and the rule is decoration.
///
/// The Blurb half was the whole of R3 before the window-capability
/// widening, and it covers five of the thirty-seven referenced dialogs.
/// The `DialogWin` half — the other thirty-two — is checked separately
/// here so that a regression narrowing R3 back to Blurbs fails loudly
/// rather than going green on a suite that never looked at a Dialog.
///
/// The button-count floors are 1, not the populations DU-L measured
/// (7 Blurb rows, 53 `DialogWin` rows, 19 Generic1). Wave 1 exists to
/// delete most of those: DU-02a stripped 45 rows off twelve Cellblock
/// dialogs and DU-02b reduces three Castle dialogs to one button each, so
/// the end state carries 2 Blurb rows, 3 `DialogWin` rows and 1 Generic1.
/// A floor of 1 still catches the failure these guards were written for —
/// a scan that returns nothing, leaving R3's inner loop unexecuted — while
/// the dialog-COUNT floors below keep their original values and remain the
/// real check that the reference scan still finds all thirty-seven.
fn assert_r3_has_subjects(seed: &DialogSeed, refs: &ChainRefs) {
    let referenced = refs.referenced();
    let buttons_across = |dialogs: &[i32]| -> usize {
        dialogs
            .iter()
            .flat_map(|d| seed.screens_in_order(*d))
            .map(|s| seed.buttons_on(s).len())
            .sum()
    };

    let blurbs: Vec<i32> = referenced
        .iter()
        .copied()
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
    assert!(
        buttons_across(&blurbs) >= 1,
        "the referenced Blurbs carry no button rows at all — R3's Blurb half is \
         inspecting nothing. 2298 is the last referenced Blurb still carrying buttons \
         after DU-02a; if its rows went too, this rule has no live subject left",
    );

    let dialog_windows: Vec<i32> = referenced
        .iter()
        .copied()
        .filter(|d| seed.ui_screen_type.get(d).map(String::as_str) == Some("DUIST_DefaultDialog"))
        .collect();
    assert!(
        dialog_windows.len() >= 20,
        "R3 has only {n} referenced DialogWin dialog(s) to check — the widened \
         window-capability rule is back to inspecting Blurbs only",
        n = dialog_windows.len(),
    );
    assert!(
        buttons_across(&dialog_windows) >= 1,
        "the referenced DialogWin dialogs carry no button rows at all — R3's DialogWin \
         half is inspecting nothing",
    );
    // Generic1 (type 4) is legal on DialogWin and illegal on BlurbWin.
    // 2576's "Take Missions" is type 4, so a rule that applied the Blurb
    // set everywhere would flag it — this pins that the per-window table
    // is really per-window. 3999's seven "Receive Item" buttons used to
    // be the bulk of this count; DU-02a stripped them.
    let generic_buttons = dialog_windows
        .iter()
        .flat_map(|d| seed.screens_in_order(*d))
        .flat_map(|s| seed.buttons_on(s))
        .filter(|b| b.button_type == 4)
        .count();
    assert!(
        generic_buttons >= 1,
        "no Generic1 (type 4) buttons on any referenced DialogWin dialog — they are legal \
         there and illegal on a Blurb, so this is what proves the table is applied per \
         window type. 2576's Take Missions is the last one; DU-02b keeps it, moved onto \
         the final screen"
    );
}
