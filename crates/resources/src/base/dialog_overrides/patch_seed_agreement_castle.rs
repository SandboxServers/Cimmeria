//! Patch-versus-seed agreement for the Castle zone table (DU-02b).
//!
//! # Why this test exists
//!
//! There are two records of what buttons a dialog has, and they are
//! written in different places by different hands:
//!
//! * [`super::patches_castle::CASTLE_DIALOG_PATCHES`] is what the
//!   **client** ends up rendering. It rewrites the cooked catalogue entry
//!   at PAK load, and the cooked entry is the only thing the client reads
//!   (fact F1).
//! * `db/resources/Dialogs/Seed/dialog_screen_buttons.sql` is what the
//!   **server side** reads: the content-chain authors, the resource
//!   loaders, and the DU-L button linter that enforces the two hard rules.
//!
//! Nothing keeps them in step automatically, and the failure is silent in
//! the worst possible direction. If the seed says a dialog's only button
//! sits on its final screen but the patch table was never updated, DU-L
//! goes green, the ledger says the soft-lock is fixed, and the player
//! still pages to a bare final screen whose Done sends nothing (fact F8).
//! Reading both records in one test is what makes the seed a truthful
//! proxy for the cooked data.
//!
//! There is a third record, and it is the one that actually matters:
//! the committed cooked archive `data/cache/CookedDataDialogs.pak`, which
//! is the input the patcher transforms. A patch naming a screen the cook
//! never shipped is refused whole (`PatchError::ScreenMissing`) and the
//! client silently keeps the ORIGINAL entry — soft-lock included — while
//! the seed and the linter both report the dialog fixed. That combination
//! is undetectable from the seed alone, so
//! [`castle_patches_apply_to_the_committed_cooked_entries`] runs each plan
//! against the real archive.
//!
//! # Scope
//!
//! Castle only. DU-02a owns the equivalent guard for the Cellblock table;
//! it lives in its own file so the two packets never edit the same one.
//! The shared, table-agnostic guards (no dialog claimed twice, no dialog
//! both regenerated and patched) stay in `patch_tests/`.

use std::collections::BTreeMap;
use std::path::PathBuf;

use super::emit::escape_xml_attr;
use super::parse::parse_cooked_dialog;
use super::patch::{apply_dialog_patch, ButtonPlan, DialogPatch};
use super::patches_castle::CASTLE_DIALOG_PATCHES;

/// `CARGO_MANIFEST_DIR` is `<workspace>/crates/resources`, so two hops up
/// land on the workspace root. Same convention as the content-engine
/// linters.
fn seed_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../db/resources/Dialogs/Seed")
}

fn read(name: &str) -> String {
    let path = seed_dir().join(name);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

// ---------------------------------------------------------------------
// Seed scanning
// ---------------------------------------------------------------------

/// One `dialog_screen_buttons` row, reduced to the fields a patch can
/// describe. `screen_button_id` is deliberately absent: it is a bare
/// primary key with a sequence default and no foreign key anywhere in
/// `db/resources/`, so which id a surviving row carries is bookkeeping,
/// not behaviour.
#[derive(Debug, PartialEq, Eq)]
struct SeedButton {
    screen_id: u32,
    button_type: u32,
    button_id: u32,
    text: String,
}

const BUTTON_PREFIX: &str = "INSERT INTO dialog_screen_buttons (screen_button_id, button_id, \
                             screen_id, button_type, text) VALUES (";
const SCREEN_PREFIX: &str = "INSERT INTO dialog_screens (dialog_id, screen_id, text, speaker_id, \
                             index) VALUES (";

/// Walk from `from` to the `);` that ends this statement, ignoring any
/// `)` inside a single-quoted literal.
///
/// A naive `find(");")` is wrong here for a concrete reason: 1,013
/// `dialog_screens` rows carry raw newlines inside their text and many
/// carry punctuation, so a scanner that is not quote-aware truncates a
/// statement mid-literal and loses every field after it. SQL escapes a
/// quote by doubling it, which is why the `''` case steps two.
fn statement_end(sql: &str, from: usize) -> Option<usize> {
    let bytes = sql.as_bytes();
    let mut i = from;
    let mut in_quote = false;
    while i < bytes.len() {
        match bytes[i] {
            b'\'' => {
                if in_quote && bytes.get(i + 1) == Some(&b'\'') {
                    i += 2;
                    continue;
                }
                in_quote = !in_quote;
            }
            b')' if !in_quote && bytes.get(i + 1) == Some(&b';') => return Some(i),
            _ => {}
        }
        i += 1;
    }
    None
}

/// Undo SQL's doubled-quote escaping and strip the surrounding quotes.
fn unquote(field: &str) -> Option<String> {
    let f = field.trim();
    let inner = f.strip_prefix('\'')?.strip_suffix('\'')?;
    Some(inner.replace("''", "'"))
}

/// `screen_id` → the buttons on it, in file order.
fn load_buttons(sql: &str) -> BTreeMap<u32, Vec<SeedButton>> {
    let mut out: BTreeMap<u32, Vec<SeedButton>> = BTreeMap::new();
    let mut at = 0;
    while let Some(hit) = sql[at..].find(BUTTON_PREFIX) {
        let start = at + hit + BUTTON_PREFIX.len();
        let end = statement_end(sql, start).expect("unterminated dialog_screen_buttons statement");
        let body = &sql[start..end];
        // (screen_button_id, button_id, screen_id, button_type, text) —
        // only the text can contain a comma, and it is last.
        let mut fields = body.splitn(5, ',');
        let _screen_button_id = fields.next().expect("screen_button_id");
        let button_id = fields.next().expect("button_id");
        let screen_id = fields.next().expect("screen_id");
        let button_type = fields.next().expect("button_type");
        let text = fields.next().expect("text");
        let num = |f: &str, what: &str| -> u32 {
            f.trim()
                .parse()
                .unwrap_or_else(|_| panic!("{what} is not a number in: {body}"))
        };
        out.entry(num(screen_id, "screen_id"))
            .or_default()
            .push(SeedButton {
                screen_id: num(screen_id, "screen_id"),
                button_type: num(button_type, "button_type"),
                button_id: num(button_id, "button_id"),
                text: unquote(text).unwrap_or_else(|| panic!("unquotable text in: {body}")),
            });
        at = end;
    }
    out
}

/// `dialog_id` → its screen ids ordered by `dialog_screens.index`.
///
/// `index` is the ordering column, not `screen_id`. The two happen to
/// agree for every dialog in today's seed, which is exactly why the key
/// has to be chosen on purpose rather than by luck.
fn load_screens(sql: &str) -> BTreeMap<u32, Vec<u32>> {
    let mut rows: BTreeMap<u32, Vec<(i64, u32)>> = BTreeMap::new();
    let mut at = 0;
    while let Some(hit) = sql[at..].find(SCREEN_PREFIX) {
        let start = at + hit + SCREEN_PREFIX.len();
        let end = statement_end(sql, start).expect("unterminated dialog_screens statement");
        let body = &sql[start..end];
        // dialog_id and screen_id lead; speaker_id and index trail. The
        // text sits between them and is the only field that can hold a
        // comma, so both ends are safe to split off.
        let mut head = body.splitn(3, ',');
        let dialog_id: u32 = head
            .next()
            .expect("dialog_id")
            .trim()
            .parse()
            .unwrap_or_else(|_| panic!("dialog_id is not a number in: {body}"));
        let screen_id: u32 = head
            .next()
            .expect("screen_id")
            .trim()
            .parse()
            .unwrap_or_else(|_| panic!("screen_id is not a number in: {body}"));
        let index: i64 = body
            .rsplit(',')
            .next()
            .expect("index")
            .trim()
            .parse()
            .unwrap_or_else(|_| panic!("index is not a number in: {body}"));
        rows.entry(dialog_id).or_default().push((index, screen_id));
        at = end;
    }
    rows.into_iter()
        .map(|(dialog_id, mut screens)| {
            screens.sort_by_key(|(index, _)| *index);
            (dialog_id, screens.into_iter().map(|(_, s)| s).collect())
        })
        .collect()
}

/// Every button row the seed holds for `dialog_id`, in screen order.
fn seed_buttons_for(
    dialog_id: u32,
    screens: &BTreeMap<u32, Vec<u32>>,
    buttons: &BTreeMap<u32, Vec<SeedButton>>,
) -> Vec<(u32, u32, u32, String)> {
    screens
        .get(&dialog_id)
        .map(|ss| {
            ss.iter()
                .flat_map(|s| buttons.get(s).into_iter().flatten())
                .map(|b| (b.screen_id, b.button_type, b.button_id, b.text.clone()))
                .collect()
        })
        .unwrap_or_default()
}

// ---------------------------------------------------------------------
// The agreement guard
// ---------------------------------------------------------------------

/// Every Castle patch row describes exactly the buttons the seed holds.
///
/// `OnlyOn` is checked in full — one row, that screen, that type, that id,
/// that text. `StripAll` requires the dialog to hold no button rows at
/// all. `Keep` is an identity plan and says nothing about the seed, so
/// there is nothing to compare; it is called out rather than silently
/// skipped.
#[test]
fn castle_patches_agree_with_the_committed_button_seed() {
    let screens = load_screens(&read("dialog_screens.sql"));
    let buttons = load_buttons(&read("dialog_screen_buttons.sql"));

    for patch in CASTLE_DIALOG_PATCHES {
        let DialogPatch {
            dialog_id,
            buttons: plan,
            ..
        } = patch;
        let found = seed_buttons_for(*dialog_id, &screens, &buttons);

        match plan {
            ButtonPlan::Keep => {
                assert!(
                    screens.contains_key(dialog_id),
                    "dialog {dialog_id} has a Keep patch but no rows in dialog_screens.sql — \
                     the patch names a dialog the seed does not have"
                );
            }
            ButtonPlan::StripAll => assert!(
                found.is_empty(),
                "dialog {dialog_id}: the patch strips every button, but the seed still holds \
                 {found:?}. The client would render no button while the seed — and therefore \
                 the DU-L linter and every chain author — believes there is one. Delete the \
                 rows from db/resources/Dialogs/Seed/dialog_screen_buttons.sql."
            ),
            ButtonPlan::OnlyOn {
                screen_id,
                button_type,
                button_id,
                text,
            } => {
                let expected = vec![(*screen_id, *button_type, *button_id, (*text).to_string())];
                assert_eq!(
                    found, expected,
                    "dialog {dialog_id}: the patch puts exactly one button on screen \
                     {screen_id} (type {button_type}, id {button_id}, {text:?}), but the seed \
                     holds {found:?}. The client renders the patch and the linter reads the \
                     seed, so a disagreement means one of the two is lying about where the \
                     player's only button is."
                );
            }
        }
    }
}

/// Every `OnlyOn` target is the dialog's FINAL screen.
///
/// This is the hard rule the whole packet exists to satisfy, asserted
/// against the patch table rather than the seed. DU-L already enforces it
/// on the seed; enforcing it here as well is what stops someone
/// "fixing" a soft-lock by landing the button on screen four of five,
/// where the seed would agree with the patch and both would be wrong.
#[test]
fn castle_only_on_patches_target_the_final_screen() {
    let screens = load_screens(&read("dialog_screens.sql"));

    for patch in CASTLE_DIALOG_PATCHES {
        let ButtonPlan::OnlyOn { screen_id, .. } = &patch.buttons else {
            continue;
        };
        let order = screens
            .get(&patch.dialog_id)
            .unwrap_or_else(|| panic!("dialog {} has no dialog_screens rows", patch.dialog_id));
        assert_eq!(
            order.last(),
            Some(screen_id),
            "dialog {dialog}: the patch lands its only button on screen {screen_id}, but the \
             dialog's screens in dialog_screens.index order are {order:?}. A button before the \
             last screen soft-locks a player who reads to the end: Done is a close, and \
             closing a dialog that HAS buttons sends nothing (fact F8).",
            dialog = patch.dialog_id,
        );
    }
}

// ---------------------------------------------------------------------
// The cooked archive — the input the patcher actually transforms
// ---------------------------------------------------------------------

/// Pull one `_<dialog_id>` entry out of the committed dialog archive.
///
/// `data/cache/CookedDataDialogs.pak` is a zip and it IS in git
/// (`git ls-files data/cache/`), alongside the nineteen other cooked
/// archives that `base/resources/tests/committed_paks.rs` already reads.
/// It is read here, not loaded through `ResourceCache::load_pak`, only
/// because that helper is private to the `resources` module.
fn cooked_entry(dialog_id: u32) -> Vec<u8> {
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/cache/CookedDataDialogs.pak");
    let file =
        std::fs::File::open(&path).unwrap_or_else(|e| panic!("open {}: {e}", path.display()));
    let mut archive = zip::ZipArchive::new(file)
        .unwrap_or_else(|e| panic!("{} is not a zip: {e}", path.display()));
    let name = format!("_{dialog_id}");
    let mut entry = archive
        .by_name(&name)
        .unwrap_or_else(|e| panic!("cooked entry {name} is not in the committed archive: {e}"));
    let mut buf = Vec::new();
    std::io::Read::read_to_end(&mut entry, &mut buf).expect("read cooked entry");
    buf
}

/// Every Castle patch applies cleanly to the entry the client ships, and
/// lands its button on the cooked entry's LAST screen.
///
/// This is the guard the seed cannot provide. The patcher refuses a plan
/// whose `screen_id` is absent from the cooked entry and the caller keeps
/// the original bytes — a fail-closed choice that is right for an
/// unattended server and silent for everyone else. With 5861 and 2576 now
/// out of the DU-L allowlist, a typo in a screen id would leave no red
/// test anywhere: the seed would say fixed, the linter would agree, and
/// the player would still page to a bare final screen.
///
/// Screen ORDER is taken from the cooked document, not from the seed's
/// `index` column, because document order is what the client pages
/// through. The two agreeing is the point of the assertion.
#[test]
fn castle_patches_apply_to_the_committed_cooked_entries() {
    for patch in CASTLE_DIALOG_PATCHES {
        let original = cooked_entry(patch.dialog_id);
        let patched = apply_dialog_patch(&original, patch).unwrap_or_else(|e| {
            panic!(
                "dialog {}: the patch does not apply to the committed cooked entry ({e:?}). \
                 A ScreenMissing here is the silent failure this test exists for: the server \
                 keeps the canonical entry, the player keeps the soft-lock, and nothing else \
                 in the suite goes red.",
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
                "dialog {}: StripAll left buttons behind — {layout:?}",
                patch.dialog_id
            ),
            ButtonPlan::OnlyOn {
                screen_id,
                button_type,
                button_id,
                text,
            } => {
                let last = after.screens.last().expect("cooked entry has screens");
                assert_eq!(
                    last.screen_id,
                    Some(*screen_id),
                    "dialog {dialog}: the patch targets screen {screen_id}, but the cooked \
                     entry's screens in document order are {layout:?} — the button would land \
                     before the last page and Done would still send nothing (fact F8).",
                    dialog = patch.dialog_id,
                );
                assert_eq!(
                    last.buttons.len(),
                    1,
                    "dialog {}: expected exactly one button on the final screen, got {layout:?}",
                    patch.dialog_id
                );
                let b = &last.buttons[0];
                // The plan holds plain text and the emitter escapes it on
                // the way out, so the comparison has to run the same
                // escape rather than assume the string survives untouched.
                let want = escape_xml_attr(text);
                assert_eq!(
                    (b.button_type, b.button_id, b.text_escaped.as_str()),
                    (*button_type, *button_id, want.as_str()),
                    "dialog {}: the emitted button does not match the plan. The ButtonID is \
                     what the client puts on the wire when the button is clicked (fact F14), \
                     so a changed id silently re-points the click.",
                    patch.dialog_id
                );
                let elsewhere: usize = after.screens[..after.screens.len() - 1]
                    .iter()
                    .map(|s| s.buttons.len())
                    .sum();
                assert_eq!(
                    elsewhere, 0,
                    "dialog {}: {elsewhere} button(s) survive on non-final screens — {layout:?}. \
                     The player could still act before reading the briefing.",
                    patch.dialog_id
                );
            }
        }
    }
}

/// Non-vacuity. Both assertions above pass on an empty model.
///
/// `StripAll` in particular asserts "no rows", which a scanner that
/// parsed nothing satisfies perfectly. Comparing the parsed row count
/// against the raw count of `INSERT INTO` occurrences makes the scan
/// self-checking, and pinning the table's size stops the whole file
/// going green because someone emptied `CASTLE_DIALOG_PATCHES`.
#[test]
fn the_castle_agreement_scan_is_reading_real_rows() {
    let screens_sql = read("dialog_screens.sql");
    let buttons_sql = read("dialog_screen_buttons.sql");
    let screens = load_screens(&screens_sql);
    let buttons = load_buttons(&buttons_sql);

    let parsed_screens: usize = screens.values().map(Vec::len).sum();
    let raw_screens = screens_sql.matches("INSERT INTO dialog_screens ").count();
    assert!(raw_screens > 1000, "dialog_screens.sql looks emptied");
    assert_eq!(
        parsed_screens, raw_screens,
        "parsed {parsed_screens} dialog_screens rows but the file holds {raw_screens} — the \
         quote-aware statement scan is losing the rows whose text contains a raw newline"
    );

    let parsed_buttons: usize = buttons.values().map(Vec::len).sum();
    let raw_buttons = buttons_sql
        .matches("INSERT INTO dialog_screen_buttons ")
        .count();
    assert!(
        raw_buttons > 1000,
        "dialog_screen_buttons.sql looks emptied"
    );
    assert_eq!(
        parsed_buttons, raw_buttons,
        "the button scan is losing rows"
    );

    assert_eq!(
        CASTLE_DIALOG_PATCHES.len(),
        3,
        "DU-02b ships three Castle patches (2573, 5861, 2576). If a packet added or removed \
         one, update this count and the UAT rows in docs/analysis/castle-rebuild/ with it."
    );
    for dialog_id in [2573u32, 5861, 2576] {
        assert!(
            CASTLE_DIALOG_PATCHES
                .iter()
                .any(|p| p.dialog_id == dialog_id),
            "dialog {dialog_id} must carry a Castle patch row"
        );
        assert!(
            !seed_buttons_for(dialog_id, &screens, &buttons).is_empty(),
            "dialog {dialog_id} must keep exactly one button row in the seed — zero rows would \
             make the OnlyOn comparison above fail for the right reason but this guard checks \
             the scan found the dialog at all"
        );
    }
}
