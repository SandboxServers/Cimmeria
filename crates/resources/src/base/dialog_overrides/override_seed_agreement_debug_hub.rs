//! Override-versus-seed agreement for the debug-hub dialogs 60100 and
//! 60101 (the Castle_CellBlock stasis-room dialog NPC) and 60104 (the Gate
//! Mail Clerk, SS-U3).
//!
//! The two records drift apart silently, and each drift fails differently:
//!
//! * the client draws only the [`DIALOG_OVERRIDES`] entry (fact F1), so a
//!   seed-only edit changes nothing a player sees;
//! * the server reads only the seed. `display_dialog` binds the player as
//!   speaker when every screen of a dialog has `speaker_id = 0`
//!   (`load_monologue_dialog_ids`), and the `dialog_button_linter` enforces
//!   the button hard rules against `dialog_screen_buttons.sql`. A seed that
//!   disagrees with the override lints and binds a dialog nobody sees.
//!
//! So every screen and button of the two overrides must appear in the seed
//! exactly, and the seed must carry nothing the overrides do not. No
//! database: the seed files are read from disk.

use std::path::PathBuf;

use super::{DialogOverride, DIALOG_OVERRIDES};

/// The Cimmeria-authored dialogs of the debug hub.
const HUB_DIALOGS: [u32; 3] = [60100, 60101, 60104];

/// `CARGO_MANIFEST_DIR` is `<workspace>/crates/resources`.
fn read_seed(name: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../db/resources/Dialogs/Seed")
        .join(name);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("seed file {} must be readable: {e}", path.display()))
}

fn hub_override(dialog_id: u32) -> &'static DialogOverride {
    DIALOG_OVERRIDES
        .iter()
        .find(|ov| ov.dialog_id == dialog_id)
        .unwrap_or_else(|| panic!("debug-hub dialog {dialog_id} must be a DialogOverride"))
}

/// SQL string literal body: the seed doubles apostrophes.
fn sql_text(s: &str) -> String {
    s.replace('\'', "''")
}

/// Every override screen is in `dialog_screens.sql` with the same text,
/// speaker and page index, and the seed has no extra screen for either
/// dialog. `index` is the order the client pages through, so it is the
/// screen's position in the override.
#[test]
fn debug_hub_override_screens_match_the_seed_exactly() {
    let sql = read_seed("dialog_screens.sql");
    for dialog_id in HUB_DIALOGS {
        let ov = hub_override(dialog_id);
        for (index, screen) in ov.screens.iter().enumerate() {
            let row = format!(
                "INSERT INTO dialog_screens (dialog_id, screen_id, text, speaker_id, index) \
                 VALUES ({dialog_id}, {}, '{}', {}, {index});",
                screen.screen_id,
                sql_text(screen.text),
                screen.speaker_id,
            );
            assert!(
                sql.contains(&row),
                "dialog_screens.sql must carry exactly this row for override \
                 {dialog_id} screen {} (text, speaker and index included):\n{row}",
                screen.screen_id,
            );
        }
        let seeded = sql
            .matches(&format!(
                "(dialog_id, screen_id, text, speaker_id, index) VALUES ({dialog_id}, "
            ))
            .count();
        assert_eq!(
            seeded,
            ov.screens.len(),
            "dialog {dialog_id}: the seed has {seeded} screen rows, the override {}",
            ov.screens.len(),
        );
    }
}

/// The `screen_id` (third column) of every `dialog_screen_buttons` row.
fn button_screen_ids(sql: &str) -> impl Iterator<Item = u32> + '_ {
    const HEADER: &str = "INSERT INTO dialog_screen_buttons (screen_button_id, button_id, screen_id, button_type, text) VALUES (";
    sql.match_indices(HEADER).map(|(i, _)| {
        let field = sql[i + HEADER.len()..]
            .split(',')
            .nth(2)
            .expect("a button row has a screen_id column");
        field
            .trim()
            .parse()
            .unwrap_or_else(|e| panic!("screen_id {field:?} must be an integer: {e}"))
    })
}

/// Every override button is in `dialog_screen_buttons.sql` with the same id,
/// type and label, and no seeded button sits on a hub screen the override
/// leaves bare. `screen_button_id` is a bare primary key, so it is not
/// compared.
#[test]
fn debug_hub_override_buttons_match_the_seed_exactly() {
    let sql = read_seed("dialog_screen_buttons.sql");
    for dialog_id in HUB_DIALOGS {
        for screen in hub_override(dialog_id).screens {
            let seeded = button_screen_ids(&sql)
                .filter(|&id| id == screen.screen_id)
                .count();
            for button in screen.buttons {
                let tail = format!(
                    ", {}, {}, {}, '{}');",
                    button.button_id,
                    screen.screen_id,
                    button.button_type,
                    sql_text(button.text),
                );
                assert!(
                    sql.contains(&tail),
                    "dialog_screen_buttons.sql must carry the button \
                     `{}` of dialog {dialog_id} screen {} as `...{tail}`",
                    button.text,
                    screen.screen_id,
                );
            }
            assert_eq!(
                seeded,
                screen.buttons.len(),
                "dialog {dialog_id} screen {}: the seed has {seeded} button rows, \
                 the override {}",
                screen.screen_id,
                screen.buttons.len(),
            );
        }
    }
}

/// The seed's `ui_screen_type` names the same window the override emits.
/// `2` is `DUIST_DefaultDialog`, the only window the linter lets carry a
/// Generic button.
#[test]
fn debug_hub_override_window_type_matches_the_seed() {
    let sql = read_seed("dialogs.sql");
    for dialog_id in HUB_DIALOGS {
        assert_eq!(hub_override(dialog_id).ui_screen_type, 2);
        let head = format!(
            "INSERT INTO dialogs (dialog_id, dialog_flags, event_set_id, ui_screen_type, \
             tags, accepts_mission_id, name) VALUES ({dialog_id}, 0, NULL, \
             'DUIST_DefaultDialog', "
        );
        assert!(
            sql.contains(&head),
            "dialogs.sql must declare {dialog_id} as a DUIST_DefaultDialog: {head}",
        );
    }
}
