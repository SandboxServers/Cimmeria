//! Patch-versus-seed-versus-cooked agreement for the Cellblock zone
//! table (DU-02a).
//!
//! # Three records, none of which keeps the others honest
//!
//! * [`super::patches_cellblock::CELLBLOCK_DIALOG_PATCHES`] is what the
//!   **client** ends up rendering. It rewrites the cooked catalogue entry
//!   at PAK load, and the cooked entry is the only thing the client reads
//!   (fact F1).
//! * `db/resources/Dialogs/Seed/dialog_screen_buttons.sql` is what the
//!   **server side** reads: chain authors, the resource loaders, and the
//!   DU-L button linter that enforces the two hard rules.
//! * `data/cache/CookedDataDialogs.pak` is the **input** the patcher
//!   transforms. It is committed, alongside the twenty other cooked
//!   archives `base/resources/tests/committed_paks.rs` already reads.
//!
//! Nothing syncs them, and the failure modes are silent in opposite
//! directions. A seed that still lists a button the patch strips makes
//! DU-L lint the wrong shape. A patch naming a dialog or screen the cook
//! never shipped is refused whole ([`super::patch::PatchError`]), the
//! caller keeps the ORIGINAL bytes, and the player keeps whatever the
//! 2009 data gave them — while the seed and the linter both report the
//! dialog fixed. With 3999 out of the DU-L allowlist, that last
//! combination would leave no red test anywhere.
//!
//! So all three are compared here: the plan against the seed, and the
//! plan against the archive it actually runs on.
//!
//! # Scope
//!
//! Cellblock only. DU-02b owns the Castle equivalent in its own file, so
//! the two packets never edit the same one; the scanners are deliberately
//! duplicated rather than shared, because a small duplicate is cheaper
//! than a merge conflict between two parallel branches. The shared,
//! table-agnostic guards (no dialog claimed twice, no dialog both
//! regenerated and patched) stay in `patch_tests.rs`.

use std::collections::HashMap;
use std::path::PathBuf;

use super::parse::parse_cooked_dialog;
use super::patch::{apply_dialog_patch, ButtonPlan};
use super::patches_cellblock::CELLBLOCK_DIALOG_PATCHES;

/// The twelve dialogs DU-02a is responsible for.
const ROSTER: [u32; 12] = [
    2299, 4001, 5022, 3999, 5023, 2309, 2516, 5859, 2305, 4000, 2308, 2518,
];

/// `CARGO_MANIFEST_DIR` is `<workspace>/crates/services`, so two hops up
/// land on the workspace root.
fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read_seed(name: &str) -> String {
    let path = workspace_root()
        .join("db/resources/Dialogs/Seed")
        .join(name);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("seed file {} must be readable: {e}", path.display()))
}

// ---------------------------------------------------------------------
// Seed scanning
// ---------------------------------------------------------------------

const SCREENS_HEADER: &str =
    "INSERT INTO dialog_screens (dialog_id, screen_id, text, speaker_id, index) VALUES (";
const BUTTONS_HEADER: &str = "INSERT INTO dialog_screen_buttons (screen_button_id, button_id, \
     screen_id, button_type, text) VALUES (";

/// One seed button row, reduced to the fields a patch can describe.
///
/// `screen_button_id` is deliberately absent: it is a bare primary key
/// with a sequence default and nothing in `db/resources/` references it,
/// so which id a surviving row carries is bookkeeping, not behaviour.
#[derive(Debug, PartialEq, Eq)]
struct SeedButton {
    screen_id: u32,
    button_type: u32,
    button_id: u32,
}

/// Yield the text following each occurrence of `header`.
///
/// Both seed files write one single-tuple `INSERT` per statement with the
/// column list spelled out in full, so finding the header is the whole of
/// the parse. `dialog_screens.text` contains raw newlines and apostrophes
/// — a line-at-a-time scan silently drops 1,013 of its 13,467 rows — but
/// it is the THIRD column, and every field read here sits in front of it.
/// No quote tracking is needed, and
/// [`the_cellblock_seed_scan_reads_every_insert_row`] pins the match count
/// against the raw header count so a format change fails loudly rather
/// than parsing a subset.
fn after_each<'a>(sql: &'a str, header: &str) -> Vec<&'a str> {
    sql.match_indices(header)
        .map(|(i, _)| &sql[i + header.len()..])
        .collect()
}

/// Read the first `n` comma-separated integers of a VALUES tuple.
fn leading_ints(tail: &str, n: usize) -> Vec<u32> {
    tail.split(',')
        .take(n)
        .map(|f| {
            f.trim()
                .parse()
                .unwrap_or_else(|e| panic!("expected an integer field, got {f:?}: {e}"))
        })
        .collect()
}

/// `dialog_id -> its screen ids`, unordered. Ordering is not needed here:
/// `StripAll` is a whole-dialog assertion and `OnlyOn` names its screen
/// explicitly. The cooked guard below takes screen ORDER from the cooked
/// document, which is what the client pages through.
fn screens_by_dialog() -> HashMap<u32, Vec<u32>> {
    let sql = read_seed("dialog_screens.sql");
    let mut out: HashMap<u32, Vec<u32>> = HashMap::new();
    for tail in after_each(&sql, SCREENS_HEADER) {
        let ids = leading_ints(tail, 2);
        out.entry(ids[0]).or_default().push(ids[1]);
    }
    out
}

/// Every seed button row, keyed by screen id.
fn buttons_by_screen() -> HashMap<u32, Vec<SeedButton>> {
    let sql = read_seed("dialog_screen_buttons.sql");
    let mut out: HashMap<u32, Vec<SeedButton>> = HashMap::new();
    for tail in after_each(&sql, BUTTONS_HEADER) {
        // (screen_button_id, button_id, screen_id, button_type, text)
        let ids = leading_ints(tail, 4);
        out.entry(ids[2]).or_default().push(SeedButton {
            screen_id: ids[2],
            button_type: ids[3],
            button_id: ids[1],
        });
    }
    out
}

/// Every seed button row belonging to `dialog_id`.
fn seed_buttons_of(dialog_id: u32) -> Vec<SeedButton> {
    let screens = screens_by_dialog();
    let buttons = buttons_by_screen();
    let mut out = Vec::new();
    for screen in screens.get(&dialog_id).into_iter().flatten() {
        for button in buttons.get(screen).into_iter().flatten() {
            out.push(SeedButton {
                screen_id: button.screen_id,
                button_type: button.button_type,
                button_id: button.button_id,
            });
        }
    }
    out
}

// ---------------------------------------------------------------------
// The cooked archive
// ---------------------------------------------------------------------

/// Pull one `_<dialog_id>` entry out of the committed dialog archive.
///
/// The archive is a zip and it IS in git (`git ls-files data/cache/`).
/// Read directly rather than through `ResourceCache::load_pak` only
/// because that helper is private to the `resources` module.
fn cooked_entry(dialog_id: u32) -> Vec<u8> {
    let path = workspace_root().join("data/cache/CookedDataDialogs.pak");
    let file =
        std::fs::File::open(&path).unwrap_or_else(|e| panic!("open {}: {e}", path.display()));
    let mut archive = zip::ZipArchive::new(file)
        .unwrap_or_else(|e| panic!("{} is not a zip: {e}", path.display()));
    let name = format!("_{dialog_id}");
    let mut entry = archive.by_name(&name).unwrap_or_else(|e| {
        panic!("cooked entry {name} is not in the committed dialog archive: {e}")
    });
    let mut buf = Vec::new();
    std::io::Read::read_to_end(&mut entry, &mut buf).expect("read cooked entry");
    buf
}

// ---------------------------------------------------------------------
// The guards
// ---------------------------------------------------------------------

/// Every patch row agrees with the seed.
///
/// This is what fails when someone re-adds a seed button row for a
/// patched dialog, or patches a dialog and forgets the seed. Note that
/// re-adding a button on a KEYED dialog's final screen leaves the DU-L
/// linter green — R1 is satisfied by it — so this test is the only thing
/// that catches that particular drift.
#[test]
fn cellblock_patches_agree_with_the_dialog_seed() {
    for patch in CELLBLOCK_DIALOG_PATCHES {
        let seed = seed_buttons_of(patch.dialog_id);
        match &patch.buttons {
            ButtonPlan::StripAll => assert!(
                seed.is_empty(),
                "dialog {id}: the patch strips every button, but the seed still has {seed:?}. \
                 The client would show no button and the seed would claim one — delete those \
                 rows from db/resources/Dialogs/Seed/dialog_screen_buttons.sql.",
                id = patch.dialog_id,
            ),
            ButtonPlan::OnlyOn {
                screen_id,
                button_type,
                button_id,
                ..
            } => assert_eq!(
                seed,
                vec![SeedButton {
                    screen_id: *screen_id,
                    button_type: *button_type,
                    button_id: *button_id,
                }],
                "dialog {id}: the patch leaves exactly one button on screen {screen_id}, but \
                 the seed says otherwise",
                id = patch.dialog_id,
            ),
            // `Keep` is an identity on the buttons, so the seed is correct
            // by construction and there is nothing to compare. Called out
            // rather than silently skipped. DU-05's type-label patches are
            // all `Keep`.
            ButtonPlan::Keep => assert!(
                screens_by_dialog().contains_key(&patch.dialog_id),
                "dialog {id} has a Keep patch but no dialog_screens rows — the patch names a \
                 dialog the seed does not have",
                id = patch.dialog_id,
            ),
        }
    }
}

/// The table still covers every dialog DU-02a is responsible for.
///
/// Without this, deleting a patch row would leave the loop above with
/// nothing to check for that dialog while its seed rows stayed deleted —
/// the seed and the client would silently disagree in the other
/// direction, and the dialog would get its buttons back at the next PAK
/// load.
#[test]
fn cellblock_patch_table_covers_exactly_the_du02a_roster() {
    let mut ids: Vec<u32> = CELLBLOCK_DIALOG_PATCHES
        .iter()
        .map(|p| p.dialog_id)
        .collect();
    ids.sort_unstable();
    let mut expected = ROSTER;
    expected.sort_unstable();

    assert_eq!(
        ids,
        expected.to_vec(),
        "CELLBLOCK_DIALOG_PATCHES no longer covers exactly the DU-02a roster. A row removed \
         here without restoring its dialog_screen_buttons.sql rows leaves the client showing \
         buttons the seed says are gone. Adding a Cellblock dialog is fine — extend ROSTER in \
         the same commit and say why in the worknote.",
    );

    for patch in CELLBLOCK_DIALOG_PATCHES {
        assert_eq!(
            patch.buttons,
            ButtonPlan::StripAll,
            "dialog {id}: every DU-02a row is StripAll. A plan change needs the matching seed \
             rows and a new arm in the agreement test above.",
            id = patch.dialog_id,
        );
        assert_eq!(
            patch.ui_screen_type,
            None,
            "dialog {id}: DU-02a changes buttons only. Window types belong to DU-05.",
            id = patch.dialog_id,
        );
    }
}

/// Every patch applies cleanly to the entry the client actually ships,
/// removes every button, and changes nothing else.
///
/// This is the guard the seed cannot provide. `apply_dialog_patches`
/// refuses a plan whose dialog or screen is absent from the loaded
/// catalogue, warns, and leaves the canonical bytes in place — a
/// fail-closed choice that is right for an unattended server and silent
/// for everyone else. A typo in a dialog id would otherwise leave no red
/// test: the seed would say the buttons are gone, DU-L would agree, and
/// the player would still be looking at them.
///
/// The `before` side is asserted too. `StripAll` on an entry that never
/// had a button passes perfectly, so without it a patch row pointed at
/// the wrong dialog could look like a success.
#[test]
fn cellblock_patches_apply_to_the_committed_cooked_entries() {
    for patch in CELLBLOCK_DIALOG_PATCHES {
        let original = cooked_entry(patch.dialog_id);
        let before = parse_cooked_dialog(&original).unwrap_or_else(|| {
            panic!(
                "dialog {}: the committed cooked entry must parse before it can be patched",
                patch.dialog_id
            )
        });

        let buttons_before: usize = before.screens.iter().map(|s| s.buttons.len()).sum();
        assert!(
            buttons_before > 0,
            "dialog {}: the committed cooked entry has no buttons to strip, so this row \
             changes nothing and StripAll passes vacuously. Either the id is wrong or the \
             row is dead — check the audit table in worknotes/du02a.md.",
            patch.dialog_id,
        );

        let patched = apply_dialog_patch(&original, patch).unwrap_or_else(|e| {
            panic!(
                "dialog {}: the patch does not apply to the committed cooked entry ({e:?}). \
                 This is the silent failure the test exists for: at runtime the server logs a \
                 warn, keeps the canonical entry, and the player keeps every button.",
                patch.dialog_id
            )
        });
        let after = parse_cooked_dialog(&patched)
            .unwrap_or_else(|| panic!("dialog {}: patched output must re-parse", patch.dialog_id));

        let layout: Vec<(Option<u32>, usize)> = after
            .screens
            .iter()
            .map(|s| (s.screen_id, s.buttons.len()))
            .collect();

        match &patch.buttons {
            ButtonPlan::Keep => {}
            ButtonPlan::StripAll => assert!(
                after.screens.iter().all(|s| s.buttons.is_empty()),
                "dialog {}: StripAll left buttons behind — {layout:?}. Any surviving button \
                 keeps suppressing the close event (fact F8).",
                patch.dialog_id,
            ),
            ButtonPlan::OnlyOn { screen_id, .. } => {
                let last = after.screens.last().expect("cooked entry has screens");
                assert_eq!(
                    last.screen_id,
                    Some(*screen_id),
                    "dialog {}: the patch targets a screen that is not the cooked entry's last \
                     — {layout:?}",
                    patch.dialog_id,
                );
            }
        }

        // Everything except the buttons must survive byte-for-byte. The
        // patcher re-emits the whole entry, so a regression in the
        // emitter would reflow tens of screens of voiced dialogue with
        // nothing else in the suite noticing on real data.
        assert_eq!(
            after.screens.len(),
            before.screens.len(),
            "dialog {}: screen count changed, {} -> {}",
            patch.dialog_id,
            before.screens.len(),
            after.screens.len(),
        );
        for (was, now) in before.screens.iter().zip(after.screens.iter()) {
            assert_eq!(
                (now.screen_id, now.speaker_id, now.text_escaped.as_str()),
                (was.screen_id, was.speaker_id, was.text_escaped.as_str()),
                "dialog {}: screen {:?} changed in something other than its buttons. A patch \
                 only ever edits buttons and the root's UIScreenType.",
                patch.dialog_id,
                was.screen_id,
            );
        }
        assert_eq!(
            (
                after.dialog_id,
                after.dialog_flags,
                after.kismet_event_set_id
            ),
            (
                before.dialog_id,
                before.dialog_flags,
                before.kismet_event_set_id
            ),
            "dialog {}: a root attribute other than UIScreenType changed",
            patch.dialog_id,
        );
        assert_eq!(
            after.ui_screen_type,
            patch.ui_screen_type.unwrap_or(before.ui_screen_type),
            "dialog {}: UIScreenType changed without the patch asking for it",
            patch.dialog_id,
        );
    }
}

/// The scans must find the rows and entries they claim to check.
///
/// Every assertion above is of the "no disagreement found" shape, so a
/// scanner that silently matched nothing would pass all of them.
#[test]
fn the_cellblock_seed_scan_reads_every_insert_row() {
    let screens_sql = read_seed("dialog_screens.sql");
    let buttons_sql = read_seed("dialog_screen_buttons.sql");

    assert_eq!(
        after_each(&screens_sql, SCREENS_HEADER).len(),
        screens_sql.matches("INSERT INTO dialog_screens").count(),
        "dialog_screens.sql: the scanner's header no longer matches every INSERT — the \
         statement format changed and the agreement tests are reading a subset of the seed",
    );
    assert_eq!(
        after_each(&buttons_sql, BUTTONS_HEADER).len(),
        buttons_sql
            .matches("INSERT INTO dialog_screen_buttons")
            .count(),
        "dialog_screen_buttons.sql: the scanner's header no longer matches every INSERT — see \
         the message above",
    );

    // A dialog outside this packet that still carries buttons, so a
    // scanner returning an empty button map cannot pass.
    assert!(
        !seed_buttons_of(2298).is_empty(),
        "dialog 2298 (mission 639's offer) must still carry its More Info + Accept rows; an \
         empty result means the button scan found nothing at all",
    );
    assert!(
        !CELLBLOCK_DIALOG_PATCHES.is_empty(),
        "CELLBLOCK_DIALOG_PATCHES is empty, so every loop in this file is vacuous",
    );
}
