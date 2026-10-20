//! Override-versus-seed agreement for dialog 60150, Prisoner 329's report
//! for mission 62050 *The Remaining Pods* (CB-N05-2).
//!
//! The served override, the `dialogs.sql` / `dialog_screens.sql` rows and
//! the committed client archive must agree: the client draws only the
//! override entry, and the server reads only the seed. The emitted bytes
//! are frozen in w3-frozen.md (F3) and pinned here byte for byte.

use std::path::PathBuf;

use super::overrides_cellblock::REMAINING_PODS_DIALOG;
use super::{generate_dialog_xml, DIALOG_OVERRIDES, QUARANTINED_DIALOG_OVERRIDES};

/// The 305 bytes F3 freezes. No newline anywhere.
const EXPECTED: &[u8] = br#"<?xml version="1.0" encoding="UTF-8"?><COOKED_DIALOG DialogFlags="0" DialogID="60150" KismetEventSetID="0" UIScreenType="2"><Screens SpeakerID="0" ScreenID="96110" Text="Prisoner 329 says there are still people in the other pods. Tell Col. Marsh to check them before you leave."></Screens></COOKED_DIALOG>"#;

/// The dialog id this packet serves.
const DIALOG_ID: u32 = 60150;

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

/// The emitted XML is the frozen 305 bytes, and the override is the one
/// the cache serves for 60150.
#[test]
fn remaining_pods_dialog_emits_the_frozen_bytes() {
    let bytes = generate_dialog_xml(&REMAINING_PODS_DIALOG);
    assert_eq!(bytes.len(), 305, "F3 freezes 305 bytes");
    assert_eq!(bytes.as_slice(), EXPECTED);
    assert!(!bytes.contains(&b'\n'), "F3 has no newline anywhere");
}

/// The override keeps the shape 3995 and 3996 already serve: one screen,
/// speaker 0, no buttons, UI type 2, flags 0, and not quarantined.
#[test]
fn remaining_pods_dialog_keeps_the_served_shape() {
    let served: Vec<_> = DIALOG_OVERRIDES
        .iter()
        .filter(|ov| ov.dialog_id == DIALOG_ID)
        .collect();
    assert_eq!(served.len(), 1, "exactly one served override for 60150");
    let ov = served[0];
    assert_eq!(ov.dialog_flags, 0);
    assert_eq!(ov.ui_screen_type, 2);
    assert_eq!(ov.screens.len(), 1, "one screen only");
    assert_eq!(ov.screens[0].speaker_id, 0);
    assert!(ov.screens[0].buttons.is_empty(), "zero buttons");
    assert_eq!(ov.screens[0].screen_id, 96110);
    assert!(
        QUARANTINED_DIALOG_OVERRIDES
            .iter()
            .all(|q| q.dialog_id != DIALOG_ID),
        "60150 must not be in any quarantine list",
    );
}

/// The `dialogs.sql` and `dialog_screens.sql` rows for 60150 are present
/// exactly once, and the seed screen text equals the override's text.
#[test]
fn remaining_pods_dialog_matches_its_seed() {
    let dialogs = read_seed("dialogs.sql");
    let screens = read_seed("dialog_screens.sql");
    let head = "INSERT INTO dialogs (dialog_id, dialog_flags, event_set_id, ui_screen_type, \
                tags, accepts_mission_id, name) VALUES (60150, 0, NULL, \
                'DUIST_DefaultDialog', NULL, NULL, \
                'Cellblock - The Remaining Pods: Prisoner 329 report');";
    assert_eq!(
        dialogs.matches(head).count(),
        1,
        "dialogs.sql must carry the 60150 row exactly once",
    );

    let screen = &REMAINING_PODS_DIALOG.screens[0];
    let row = format!(
        "INSERT INTO dialog_screens (dialog_id, screen_id, text, speaker_id, index) \
         VALUES ({DIALOG_ID}, {}, '{}', {}, 0);",
        screen.screen_id,
        sql_text(screen.text),
        screen.speaker_id,
    );
    assert_eq!(
        screens.matches(&row).count(),
        1,
        "dialog_screens.sql must carry this row exactly once:\n{row}",
    );
    assert_eq!(
        screens
            .matches(&format!(
                "(dialog_id, screen_id, text, speaker_id, index) VALUES ({DIALOG_ID}, "
            ))
            .count(),
        1,
        "60150 has one seeded screen, as the override does",
    );
}

/// Dialog 60150 is new to the client: the committed stock dialog archive
/// has no `_60150` member, so the override is the only source of its text.
#[test]
fn remaining_pods_dialog_is_new_to_the_client() {
    let path = workspace_root().join("data/cache/CookedDataDialogs.pak");
    let file =
        std::fs::File::open(&path).unwrap_or_else(|e| panic!("open {}: {e}", path.display()));
    let mut archive = zip::ZipArchive::new(file)
        .unwrap_or_else(|e| panic!("{} is not a zip: {e}", path.display()));
    let name = format!("_{DIALOG_ID}");
    assert!(
        matches!(
            archive.by_name(&name),
            Err(zip::result::ZipError::FileNotFound)
        ),
        "the committed dialog archive must not carry {name}",
    );
}
