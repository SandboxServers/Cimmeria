//! Override-versus-seed agreement for dialog 10725, the Castle return_q2
//! dialog (CS-04cS2, Frozen (c)).
//!
//! The served override and the `dialogs.sql` / `dialog_screens.sql` rows
//! must agree, and the `dialog_sets.sql` / `dialog_set_maps.sql` rows must
//! carry the frozen interaction values.

use std::path::PathBuf;

use super::overrides_castle::RETURN_Q2_DIALOG;
use super::{DIALOG_OVERRIDES, MAX_COOKED_ELEMENT_ID};

/// The dialog id this packet serves.
const DIALOG_ID: u32 = 10725;

/// The screen id of the single screen of dialog 10725.
const SCREEN_ID: u32 = 10849;

/// The dialog set and dialog set map that carry the interaction.
const DIALOG_SET_ID: u32 = 10773;
const DIALOG_SET_MAP_ID: u32 = 1000014;

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

/// The Castle return_q2 override and the seed rows agree, the seed sets
/// hold the frozen rows, and both ids fit the 16-bit guard.
#[test]
fn castle_side_return_q2_dialog_override_matches_seed() {
    let dialogs = read_seed("dialogs.sql");
    let screens = read_seed("dialog_screens.sql");
    let buttons = read_seed("dialog_screen_buttons.sql");
    let sets = read_seed("dialog_sets.sql");
    let maps = read_seed("dialog_set_maps.sql");

    // The override is served for 10725, and only once.
    let served: Vec<_> = DIALOG_OVERRIDES
        .iter()
        .filter(|ov| ov.dialog_id == DIALOG_ID)
        .collect();
    assert_eq!(served.len(), 1, "exactly one served override for 10725");
    let ov = served[0];
    assert_eq!(ov.dialog_flags, 0, "flags 0");
    assert_eq!(
        ov.ui_screen_type, 2,
        "ui_screen_type 2 = DUIST_DefaultDialog"
    );
    assert_eq!(ov.screens.len(), 1, "one screen only");
    let screen = &ov.screens[0];
    assert_eq!(screen.screen_id, SCREEN_ID);
    assert_eq!(screen.speaker_id, 0, "speaker 0");
    assert!(screen.buttons.is_empty(), "zero buttons");
    assert_eq!(
        screen.text, RETURN_Q2_DIALOG.screens[0].text,
        "the served text is RETURN_Q2_DIALOG's text",
    );

    // The dialogs row: flags 0, DUIST_DefaultDialog, exactly once.
    let dialog_row = format!(
        "INSERT INTO dialogs (dialog_id, dialog_flags, event_set_id, ui_screen_type, tags, accepts_mission_id, name) VALUES ({DIALOG_ID}, 0, NULL, 'DUIST_DefaultDialog', NULL, NULL, NULL);"
    );
    assert_eq!(
        count_lines(&dialogs, &dialog_row),
        1,
        "dialogs.sql must hold the 10725 row once"
    );

    // The dialog_screens row: the override's text, byte for byte, speaker 0.
    let screen_row = format!(
        "INSERT INTO dialog_screens (dialog_id, screen_id, text, speaker_id, index) VALUES ({DIALOG_ID}, {SCREEN_ID}, '{}', 0, 0);",
        sql_text(screen.text),
    );
    assert_eq!(
        count_lines(&screens, &screen_row),
        1,
        "dialog_screens.sql must hold the 10849 row once, text equal to the override",
    );

    // Zero buttons: no dialog_screen_buttons row names screen 10849.
    assert!(
        !mentions_id(&buttons, SCREEN_ID),
        "dialog_screen_buttons.sql must hold no row for screen 10849",
    );

    // The interaction: the frozen set and the frozen map.
    let set_row =
        format!("INSERT INTO dialog_sets (dialog_set_id, name) VALUES ({DIALOG_SET_ID}, '');");
    assert_eq!(
        count_lines(&sets, &set_row),
        1,
        "dialog_sets.sql must hold the 10773 row once"
    );
    let map_row = format!(
        "INSERT INTO dialog_set_maps (dialog_set_map_id, dialog_set_id, dialog_id, topic_text, interaction_flags, min_level, missions_completed, missions_not_accepted, alignments, factions) VALUES ({DIALOG_SET_MAP_ID}, {DIALOG_SET_ID}, NULL, 'Speak with Moh''katan', 1073741824, 1, '{{}}', '{{}}', '{{}}', '{{}}');"
    );
    assert_eq!(
        count_lines(&maps, &map_row),
        1,
        "dialog_set_maps.sql must hold the 1000014 row once"
    );

    // Both ids fit the 16-bit guard.
    for id in [DIALOG_ID, SCREEN_ID] {
        assert!(id <= MAX_COOKED_ELEMENT_ID, "id {id} fits 16 bits");
    }
}
