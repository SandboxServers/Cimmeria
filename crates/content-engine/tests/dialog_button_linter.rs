//// The three dialogs that shipped in the soft-locking shape stay fixed.
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
            "dialog {dialog}: the button must sit on final screen {final_screen}, or a              player who reads to the end is soft-locked again"
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
