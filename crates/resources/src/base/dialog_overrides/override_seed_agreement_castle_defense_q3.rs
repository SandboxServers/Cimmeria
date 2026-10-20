//! Override-versus-seed agreement for the Castle defense_q3 dialogs 10717
//! and 10718 (CS-04bD2b, Frozen (c)).
//!
//! The served overrides and the `dialogs.sql` / `dialog_screens.sql` rows
//! must agree, and the `dialog_sets.sql` / `dialog_set_maps.sql` rows must
//! carry the frozen interaction values.

use std::path::PathBuf;

use super::overrides_castle::{DEFENSE_Q3_DONE_DIALOG, DEFENSE_Q3_OFFER_DIALOG};
use super::{DialogOverride, DIALOG_OVERRIDES, MAX_COOKED_ELEMENT_ID};

/// The dialog set and dialog set map that carry the interaction.
const DIALOG_SET_ID: u32 = 10769;
const DIALOG_SET_MAP_ID: u32 = 1000010;

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

/// The served override for `dialog_id`, which must be served exactly once.
fn served_once(dialog_id: u32) -> &'static DialogOverride {
    let served: Vec<_> = DIALOG_OVERRIDES
        .iter()
        .filter(|ov| ov.dialog_id == dialog_id)
        .collect();
    assert_eq!(
        served.len(),
        1,
        "exactly one served override for {dialog_id}"
    );
    served[0]
}

/// Each defense_q3 override is served once, and its text, speaker, buttons
/// and flags agree with the `dialogs.sql` and `dialog_screens.sql` rows.
#[test]
fn castle_side_defense_q3_dialog_override_matches_seed() {
    let dialogs = read_seed("dialogs.sql");
    let screens = read_seed("dialog_screens.sql");
    let buttons = read_seed("dialog_screen_buttons.sql");

    let cases = [
        (10717, 10833, DEFENSE_Q3_OFFER_DIALOG.screens[0].text),
        (10718, 10834, DEFENSE_Q3_DONE_DIALOG.screens[0].text),
    ];
    for (dialog_id, screen_id, text) in cases {
        let ov = served_once(dialog_id);
        assert_eq!(ov.dialog_flags, 0, "{dialog_id}: flags 0");
        assert_eq!(
            ov.ui_screen_type, 2,
            "{dialog_id}: ui_screen_type 2 = DUIST_DefaultDialog"
        );
        assert_eq!(ov.screens.len(), 1, "{dialog_id}: one screen only");
        let screen = &ov.screens[0];
        assert_eq!(screen.screen_id, screen_id, "{dialog_id}: screen id");
        assert_eq!(screen.speaker_id, 0, "{dialog_id}: speaker 0");
        assert!(screen.buttons.is_empty(), "{dialog_id}: zero buttons");
        assert_eq!(screen.text, text, "{dialog_id}: served text");

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

        // Zero buttons: no dialog_screen_buttons row names this screen.
        assert!(
            !mentions_id(&buttons, screen_id),
            "dialog_screen_buttons.sql must hold no row for screen {screen_id}",
        );
    }
}

/// The set 10769 and map 1000010 carry the frozen interaction values: a
/// NULL-dialog, interaction-only map with flags 1073741824.
#[test]
fn castle_side_defense_q3_dialog_set_seed() {
    let sets = read_seed("dialog_sets.sql");
    let maps = read_seed("dialog_set_maps.sql");

    let set_row =
        format!("INSERT INTO dialog_sets (dialog_set_id, name) VALUES ({DIALOG_SET_ID}, '');");
    assert_eq!(
        count_lines(&sets, &set_row),
        1,
        "dialog_sets.sql must hold the 10769 row once"
    );
    let map_row = format!(
        "INSERT INTO dialog_set_maps (dialog_set_map_id, dialog_set_id, dialog_id, topic_text, interaction_flags, min_level, missions_completed, missions_not_accepted, alignments, factions) VALUES ({DIALOG_SET_MAP_ID}, {DIALOG_SET_ID}, NULL, 'Speak with Sergeant Stanton', 1073741824, 1, '{{}}', '{{}}', '{{}}', '{{}}');"
    );
    assert_eq!(
        count_lines(&maps, &map_row),
        1,
        "dialog_set_maps.sql must hold the 1000010 row once"
    );

    // The dialog, screen and set ids fit the 16-bit guard.
    for id in [10717, 10718, 10833, 10834, DIALOG_SET_ID] {
        assert!(id <= MAX_COOKED_ELEMENT_ID, "id {id} fits 16 bits");
    }
}

/// Both defense_q3 dialogs are in `DIALOG_OVERRIDES`, and the served entry
/// carries the constant's text.
#[test]
fn castle_side_defense_q3_overrides_registered() {
    for (dialog_id, expected) in [
        (10717, &DEFENSE_Q3_OFFER_DIALOG),
        (10718, &DEFENSE_Q3_DONE_DIALOG),
    ] {
        let ov = served_once(dialog_id);
        assert_eq!(
            ov.screens[0].text, expected.screens[0].text,
            "{dialog_id}: the served entry is the registered constant"
        );
    }
}
