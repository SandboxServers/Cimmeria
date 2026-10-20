//! Override-versus-seed agreement for the Castle repeatable_q1 dialogs 10729
//! (the supply officer's offer) and 10730 (the turn-in), CS-05b1 from the
//! CS-05S3 worknote, Frozen (c).
//!
//! The served overrides and the `dialogs.sql` / `dialog_screens.sql` rows
//! must agree, the `dialog_sets.sql` / `dialog_set_maps.sql` rows must carry
//! the frozen interaction values, and neither monologue carries a button.

use std::path::PathBuf;

use super::overrides_castle::{REPEATABLE_Q1_DIALOG, REPEATABLE_Q1_TURN_IN_DIALOG};
use super::{DialogOverride, DIALOG_OVERRIDES, MAX_COOKED_ELEMENT_ID};

/// The offer monologue: dialog 10729 and its single screen.
const OFFER_DIALOG_ID: u32 = 10729;
const OFFER_SCREEN_ID: u32 = 10857;

/// The turn-in monologue: dialog 10730 and its single screen.
const TURN_IN_DIALOG_ID: u32 = 10730;
const TURN_IN_SCREEN_ID: u32 = 10858;

/// The dialog sets and dialog set maps that carry the offer and the turn-in.
const OFFER_SET_ID: u32 = 10775;
const TURN_IN_SET_ID: u32 = 10776;
const OFFER_SET_MAP_ID: u32 = 1000016;
const TURN_IN_SET_MAP_ID: u32 = 1000017;

/// `CARGO_MANIFEST_DIR` is `<workspace>/crates/resources`.
fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Read a dialog seed file with CRLF folded to LF.
fn read_seed(name: &str) -> String {
    let path = workspace_root()
        .join("db/resources/Dialogs/Seed")
        .join(name);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("seed file {} must be readable: {e}", path.display()))
        .replace("\r\n", "\n")
}

/// SQL string literal body: the seed doubles apostrophes.
fn sql_text(s: &str) -> String {
    s.replace('\'', "''")
}

/// How many seed lines equal `line` exactly.
fn count_lines(seed: &str, line: &str) -> usize {
    seed.lines().filter(|l| *l == line).count()
}

/// True when some seed line carries `id` as a whole number token.
fn mentions_id(seed: &str, id: u32) -> bool {
    let id = id.to_string();
    seed.lines()
        .any(|l| l.split(|c: char| !c.is_ascii_digit()).any(|tok| tok == id))
}

/// The one served override for `dialog_id`.
fn served(dialog_id: u32) -> &'static DialogOverride {
    let found: Vec<_> = DIALOG_OVERRIDES
        .iter()
        .filter(|ov| ov.dialog_id == dialog_id)
        .collect();
    assert_eq!(
        found.len(),
        1,
        "exactly one served override for {dialog_id}"
    );
    found[0]
}

/// The served monologue is one screen, speaker 0, zero buttons, UI type 2,
/// and its `dialogs` and `dialog_screens` rows agree with it byte for byte.
fn assert_monologue_matches_seed(dialog_id: u32, screen_id: u32, expected: &DialogOverride) {
    let dialogs = read_seed("dialogs.sql");
    let screens = read_seed("dialog_screens.sql");
    let ov = served(dialog_id);
    assert_eq!(ov.dialog_flags, 0, "flags 0");
    assert_eq!(
        ov.ui_screen_type, 2,
        "ui_screen_type 2 = DUIST_DefaultDialog"
    );
    assert_eq!(ov.screens.len(), 1, "one screen only");
    let screen = &ov.screens[0];
    assert_eq!(screen.screen_id, screen_id);
    assert_eq!(screen.speaker_id, 0, "speaker 0");
    assert!(screen.buttons.is_empty(), "zero buttons");
    assert_eq!(
        screen.text, expected.screens[0].text,
        "the served text is the dialog's constant",
    );

    // The dialogs row: flags 0, DUIST_DefaultDialog, exactly once.
    let dialog_row = format!(
        "INSERT INTO dialogs (dialog_id, dialog_flags, event_set_id, ui_screen_type, tags, accepts_mission_id, name) VALUES ({dialog_id}, 0, NULL, 'DUIST_DefaultDialog', NULL, NULL, NULL);"
    );
    assert_eq!(
        count_lines(&dialogs, &dialog_row),
        1,
        "dialogs.sql must hold the {dialog_id} row once"
    );

    // The dialog_screens row: the override's text, byte for byte, speaker 0.
    let screen_row = format!(
        "INSERT INTO dialog_screens (dialog_id, screen_id, text, speaker_id, index) VALUES ({dialog_id}, {screen_id}, '{}', 0, 0);",
        sql_text(screen.text),
    );
    assert_eq!(
        count_lines(&screens, &screen_row),
        1,
        "dialog_screens.sql must hold the {screen_id} row once, text equal to the override",
    );
    assert!(
        dialog_id <= MAX_COOKED_ELEMENT_ID && screen_id <= MAX_COOKED_ELEMENT_ID,
        "ids {dialog_id} and {screen_id} fit the 16-bit guard"
    );
}

/// The offer override and its seed rows agree (Frozen (c), dialog 10729).
#[test]
fn castle_side_repeatable_q1_dialog_override_matches_seed_10729() {
    assert_monologue_matches_seed(OFFER_DIALOG_ID, OFFER_SCREEN_ID, &REPEATABLE_Q1_DIALOG);
}

/// The turn-in override and its seed rows agree (Frozen (c), dialog 10730).
#[test]
fn castle_side_repeatable_q1_dialog_override_matches_seed_10730() {
    assert_monologue_matches_seed(
        TURN_IN_DIALOG_ID,
        TURN_IN_SCREEN_ID,
        &REPEATABLE_Q1_TURN_IN_DIALOG,
    );
}

/// The dialog sets and dialog set maps hold the frozen interaction rows:
/// NULL dialog, the topic text, and the MISSION_WORLD_OBJECT (offer) and
/// turn-in ladder flags.
#[test]
fn castle_side_repeatable_q1_dialog_set_maps_load() {
    let sets = read_seed("dialog_sets.sql");
    let maps = read_seed("dialog_set_maps.sql");

    for set_id in [OFFER_SET_ID, TURN_IN_SET_ID] {
        let set_row =
            format!("INSERT INTO dialog_sets (dialog_set_id, name) VALUES ({set_id}, '');");
        assert_eq!(
            count_lines(&sets, &set_row),
            1,
            "dialog_sets.sql must hold the {set_id} row once"
        );
    }

    let offer_map = format!(
        "INSERT INTO dialog_set_maps (dialog_set_map_id, dialog_set_id, dialog_id, topic_text, interaction_flags, min_level, missions_completed, missions_not_accepted, alignments, factions) VALUES ({OFFER_SET_MAP_ID}, {OFFER_SET_ID}, NULL, 'Speak with the Supply Officer', 1073741824, 1, '{{}}', '{{}}', '{{}}', '{{}}');"
    );
    assert_eq!(
        count_lines(&maps, &offer_map),
        1,
        "dialog_set_maps.sql must hold the {OFFER_SET_MAP_ID} row once"
    );
    let turn_in_map = format!(
        "INSERT INTO dialog_set_maps (dialog_set_map_id, dialog_set_id, dialog_id, topic_text, interaction_flags, min_level, missions_completed, missions_not_accepted, alignments, factions) VALUES ({TURN_IN_SET_MAP_ID}, {TURN_IN_SET_ID}, NULL, 'Report to the Supply Officer', 33554432, 1, '{{}}', '{{}}', '{{}}', '{{}}');"
    );
    assert_eq!(
        count_lines(&maps, &turn_in_map),
        1,
        "dialog_set_maps.sql must hold the {TURN_IN_SET_MAP_ID} row once"
    );
}

/// Neither monologue has a button: no `dialog_screen_buttons` row names
/// screen 10857 or 10858, and both served screens carry zero buttons.
#[test]
fn castle_side_repeatable_q1_monologues_have_no_buttons() {
    let buttons = read_seed("dialog_screen_buttons.sql");
    for screen_id in [OFFER_SCREEN_ID, TURN_IN_SCREEN_ID] {
        assert!(
            !mentions_id(&buttons, screen_id),
            "dialog_screen_buttons.sql must hold no row for screen {screen_id}",
        );
    }
    for dialog_id in [OFFER_DIALOG_ID, TURN_IN_DIALOG_ID] {
        let ov = served(dialog_id);
        assert!(
            ov.screens.iter().all(|s| s.buttons.is_empty()),
            "dialog {dialog_id} serves zero buttons"
        );
    }
}

/// Template 424 (the supply officer) appears in exactly one spawn row, so the
/// per-player `add_dialog_set` binding (slot 424) cannot reach a second NPC
/// (CS-05S3 Frozen (e), G9). The template id is the seventh column of
/// `spawnlist.sql`; the first six columns are numeric, so a split on commas
/// reads it safely.
#[test]
fn castle_side_repeatable_q1_slot_424_has_one_spawn() {
    let path = workspace_root().join("db/resources/Worlds/Seed/spawnlist.sql");
    let spawns = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("seed file {} must be readable: {e}", path.display()))
        .replace("\r\n", "\n");
    let spawned: Vec<&str> = spawns
        .lines()
        .filter_map(|l| l.split_once("VALUES ("))
        .filter_map(|(_, values)| values.split(',').nth(6))
        .map(str::trim)
        .filter(|t| *t == "424")
        .collect();
    assert_eq!(
        spawned.len(),
        1,
        "template 424 must appear in exactly one spawnlist row"
    );
}
