//! Override-versus-seed agreement for the Castle medical_q2 dialogs 10701 and
//! 10702 (CS-04aD2, Frozen (c)).
//!
//! The served overrides and the `dialogs.sql` / `dialog_screens.sql` rows
//! must agree, the `dialog_sets.sql` / `dialog_set_maps.sql` rows must carry
//! the frozen interaction values, and no map or set routes to either dialog.

use std::path::PathBuf;

use super::overrides_castle::{MEDICAL_Q2_MEDIC_DIALOG, MEDICAL_Q2_PRISONER_DIALOG};
use super::{DialogOverride, DIALOG_OVERRIDES, MAX_COOKED_ELEMENT_ID};

/// The dialog sets and dialog set maps that carry the two interactions.
const PRISONER_SET_ID: u32 = 10761;
const MEDIC_SET_ID: u32 = 10762;
const PRISONER_SET_MAP_ID: u32 = 1000002;
const MEDIC_SET_MAP_ID: u32 = 1000003;

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

/// The served override for `expected`'s dialog is unique and equals the
/// frozen constant, and the seed rows hold the same dialog and screen.
fn assert_override_matches_seed(expected: &DialogOverride, dialogs: &str, screens: &str) {
    let dialog_id = expected.dialog_id;
    let frozen = &expected.screens[0];

    let served: Vec<_> = DIALOG_OVERRIDES
        .iter()
        .filter(|ov| ov.dialog_id == dialog_id)
        .collect();
    assert_eq!(
        served.len(),
        1,
        "exactly one served override for {dialog_id}"
    );
    let ov = served[0];
    assert_eq!(ov.dialog_flags, 0, "flags 0");
    assert_eq!(
        ov.ui_screen_type, 2,
        "ui_screen_type 2 = DUIST_DefaultDialog"
    );
    assert_eq!(ov.screens.len(), 1, "one screen only");
    let screen = &ov.screens[0];
    assert_eq!(screen.screen_id, frozen.screen_id);
    assert_eq!(screen.speaker_id, 0, "speaker 0");
    assert!(screen.buttons.is_empty(), "zero buttons");
    assert_eq!(
        screen.text, frozen.text,
        "the served text is the frozen constant's text",
    );

    // The dialogs row: flags 0, DUIST_DefaultDialog, exactly once.
    let dialog_row = format!(
        "INSERT INTO dialogs (dialog_id, dialog_flags, event_set_id, ui_screen_type, tags, accepts_mission_id, name) VALUES ({dialog_id}, 0, NULL, 'DUIST_DefaultDialog', NULL, NULL, NULL);"
    );
    assert_eq!(
        count_lines(dialogs, &dialog_row),
        1,
        "dialogs.sql must hold the {dialog_id} row once"
    );

    // The dialog_screens row: the override's text, byte for byte, speaker 0.
    let screen_row = format!(
        "INSERT INTO dialog_screens (dialog_id, screen_id, text, speaker_id, index) VALUES ({dialog_id}, {}, '{}', 0, 0);",
        frozen.screen_id,
        sql_text(frozen.text),
    );
    assert_eq!(
        count_lines(screens, &screen_row),
        1,
        "dialog_screens.sql must hold screen {} once, text equal to the override",
        frozen.screen_id,
    );

    // Both ids fit the 16-bit guard.
    for id in [dialog_id, frozen.screen_id] {
        assert!(id <= MAX_COOKED_ELEMENT_ID, "id {id} fits 16 bits");
    }
}

/// The Castle medical_q2 prisoner override (10701) agrees with its seed rows.
#[test]
fn castle_side_medical_q2_dialog_override_matches_seed_10701() {
    let dialogs = read_seed("dialogs.sql");
    let screens = read_seed("dialog_screens.sql");
    assert_override_matches_seed(&MEDICAL_Q2_PRISONER_DIALOG, &dialogs, &screens);
}

/// The Castle medical_q2 medic reply override (10702) agrees with its seed rows.
#[test]
fn castle_side_medical_q2_dialog_override_matches_seed_10702() {
    let dialogs = read_seed("dialogs.sql");
    let screens = read_seed("dialog_screens.sql");
    assert_override_matches_seed(&MEDICAL_Q2_MEDIC_DIALOG, &dialogs, &screens);
}

/// The two sets and their maps are the frozen flag carriers: the maps hold no
/// dialog id, carry the interaction flag, and no button row names a screen.
#[test]
fn castle_side_medical_q2_set_maps_are_flag_carriers() {
    let buttons = read_seed("dialog_screen_buttons.sql");
    let sets = read_seed("dialog_sets.sql");
    let maps = read_seed("dialog_set_maps.sql");

    for set_id in [PRISONER_SET_ID, MEDIC_SET_ID] {
        let set_row =
            format!("INSERT INTO dialog_sets (dialog_set_id, name) VALUES ({set_id}, '');");
        assert_eq!(
            count_lines(&sets, &set_row),
            1,
            "dialog_sets.sql must hold set {set_id} once"
        );
    }

    let prisoner_map = format!(
        "INSERT INTO dialog_set_maps (dialog_set_map_id, dialog_set_id, dialog_id, topic_text, interaction_flags, min_level, missions_completed, missions_not_accepted, alignments, factions) VALUES ({PRISONER_SET_MAP_ID}, {PRISONER_SET_ID}, NULL, 'Speak with the Prisoner', 1073741824, 1, '{{}}', '{{}}', '{{}}', '{{}}');"
    );
    assert_eq!(
        count_lines(&maps, &prisoner_map),
        1,
        "dialog_set_maps.sql must hold the 1000002 row once"
    );
    let medic_map = format!(
        "INSERT INTO dialog_set_maps (dialog_set_map_id, dialog_set_id, dialog_id, topic_text, interaction_flags, min_level, missions_completed, missions_not_accepted, alignments, factions) VALUES ({MEDIC_SET_MAP_ID}, {MEDIC_SET_ID}, NULL, 'Report to the Medic', 1073741824, 1, '{{}}', '{{}}', '{{}}', '{{}}');"
    );
    assert_eq!(
        count_lines(&maps, &medic_map),
        1,
        "dialog_set_maps.sql must hold the 1000003 row once"
    );

    for screen_id in [10801, 10802] {
        assert!(
            !mentions_id(&buttons, screen_id),
            "dialog_screen_buttons.sql must hold no row for screen {screen_id}",
        );
    }
}

/// No map and no set routes to 10701 or 10702, so they appear only through
/// `display_dialog` from a chain.
#[test]
fn castle_side_medical_q2_dialogs_not_reachable_from_maps() {
    let sets = read_seed("dialog_sets.sql");
    let maps = read_seed("dialog_set_maps.sql");
    for dialog_id in [10701, 10702] {
        assert!(
            !mentions_id(&maps, dialog_id),
            "dialog_set_maps.sql must not name dialog {dialog_id}",
        );
        assert!(
            !mentions_id(&sets, dialog_id),
            "dialog_sets.sql must not name dialog {dialog_id}",
        );
    }
}
