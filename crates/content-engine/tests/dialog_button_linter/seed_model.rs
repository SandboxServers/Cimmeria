//! The two models the rules run against: the dialog seed (screens and
//! buttons per dialog) and the chain seeds' references to dialog ids.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use super::sql_scan::{insert_rows, int, sql_statements, text};

/// The chain seed files whose `dialog_choice` triggers define the set of
/// dialogs this linter is responsible for. Castle and Castle_CellBlock
/// only — the two zones the dialog UI redesign covers.
pub(crate) const CHAIN_FILES: [&str; 4] = [
    "castle_cellblock_chains.sql",
    "castle_701_chains.sql",
    "castle_702_704_chains.sql",
    "castle_706_708_chains.sql",
];

/// `CARGO_MANIFEST_DIR` is `<workspace>/crates/content-engine`, so two
/// `parent()` hops land on the workspace root — same as
/// `interact_tag_linter.rs`.
pub(crate) fn workspace_root() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

pub(crate) fn read(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

#[derive(Debug)]
pub(crate) struct Button {
    pub(crate) button_id: i32,
    pub(crate) button_type: i32,
    pub(crate) text: String,
}

#[derive(Default)]
pub(crate) struct DialogSeed {
    /// `dialog_id` → `ui_screen_type` enum label.
    pub(crate) ui_screen_type: BTreeMap<i32, String>,
    /// `dialog_id` → `(index, screen_id)`, kept sorted by `index`.
    pub(crate) screens: BTreeMap<i32, Vec<(i32, i32)>>,
    /// `screen_id` → its buttons, in row order.
    pub(crate) buttons: BTreeMap<i32, Vec<Button>>,
}

impl DialogSeed {
    /// The dialog's screens in the order the client pages through them.
    ///
    /// `dialog_screens.index` is the ordering column, not `screen_id`.
    /// Evidence: across all 5,412 shipped dialogs `index` is exactly
    /// `0..n-1` with no duplicates and no gaps, and the order it gives
    /// matches the `<Screens ScreenID>` order of the cooked pak entry
    /// for 2576, 3999 and 5861 (checked 2026-09-21 against
    /// `data/cache/CookedDataDialogs.pak`, which is not in git).
    ///
    /// Sorting by `screen_id` happens to produce the same order for
    /// every dialog in today's seed, which is exactly why the key has to
    /// be chosen deliberately: an authored dialog that reuses a low
    /// screen id for its last screen would silently move the "final"
    /// screen under an id sort, and the soft-lock this linter exists to
    /// catch would stop being visible.
    pub(crate) fn screens_in_order(&self, dialog_id: i32) -> Vec<i32> {
        self.screens
            .get(&dialog_id)
            .map(|v| v.iter().map(|(_, s)| *s).collect())
            .unwrap_or_default()
    }

    pub(crate) fn final_screen(&self, dialog_id: i32) -> Option<i32> {
        self.screens_in_order(dialog_id).last().copied()
    }

    pub(crate) fn buttons_on(&self, screen_id: i32) -> &[Button] {
        self.buttons.get(&screen_id).map_or(&[], |v| &v[..])
    }

    pub(crate) fn button_count(&self, dialog_id: i32) -> usize {
        self.screens_in_order(dialog_id)
            .iter()
            .map(|s| self.buttons_on(*s).len())
            .sum()
    }

    /// Screens of this dialog carrying at least one button, in order.
    pub(crate) fn screens_with_buttons(&self, dialog_id: i32) -> Vec<i32> {
        self.screens_in_order(dialog_id)
            .into_iter()
            .filter(|s| !self.buttons_on(*s).is_empty())
            .collect()
    }

    pub(crate) fn is_blurb(&self, dialog_id: i32) -> bool {
        self.ui_screen_type.get(&dialog_id).map(String::as_str) == Some("DUIST_DefaultBlurb")
    }
}

pub(crate) fn load_dialog_seed(root: &Path) -> DialogSeed {
    let dir = root.join("db/resources/Dialogs/Seed");
    let mut seed = DialogSeed::default();

    for stmt in sql_statements(&read(&dir.join("dialogs.sql"))) {
        for row in insert_rows(&stmt, "dialogs") {
            if let (Some(id), Some(ty)) = (int(&row, "dialog_id"), text(&row, "ui_screen_type")) {
                seed.ui_screen_type.insert(id, ty);
            }
        }
    }

    for stmt in sql_statements(&read(&dir.join("dialog_screens.sql"))) {
        for row in insert_rows(&stmt, "dialog_screens") {
            if let (Some(dialog), Some(screen), Some(index)) = (
                int(&row, "dialog_id"),
                int(&row, "screen_id"),
                int(&row, "index"),
            ) {
                seed.screens
                    .entry(dialog)
                    .or_default()
                    .push((index, screen));
            }
        }
    }
    for list in seed.screens.values_mut() {
        list.sort_unstable();
    }

    for stmt in sql_statements(&read(&dir.join("dialog_screen_buttons.sql"))) {
        for row in insert_rows(&stmt, "dialog_screen_buttons") {
            if let (Some(screen), Some(button_id), Some(button_type), Some(text_)) = (
                int(&row, "screen_id"),
                int(&row, "button_id"),
                int(&row, "button_type"),
                text(&row, "text"),
            ) {
                seed.buttons.entry(screen).or_default().push(Button {
                    button_id,
                    button_type,
                    text: text_,
                });
            }
        }
    }

    seed
}

/// Where a dialog id is referenced from: `(file, chain_id)`.
pub(crate) type ChainRef = (String, i32);

#[derive(Default)]
pub(crate) struct ChainRefs {
    /// `dialog_id` → chains whose `dialog_choice` trigger keys on it.
    pub(crate) keyed: BTreeMap<i32, Vec<ChainRef>>,
    /// `dialog_id` → chains with a `display_dialog` action for it.
    pub(crate) displayed: BTreeMap<i32, Vec<ChainRef>>,
}

impl ChainRefs {
    /// Render the keying chains for a failure message, so the author is
    /// pointed at the rows that make the dialog load-bearing.
    pub(crate) fn chains_for(&self, dialog_id: i32) -> String {
        match self.keyed.get(&dialog_id) {
            Some(refs) => refs
                .iter()
                .map(|(f, c)| format!("{f}:chain {c}"))
                .collect::<Vec<_>>()
                .join(", "),
            None => "(no dialog_choice chain)".to_string(),
        }
    }

    pub(crate) fn referenced(&self) -> BTreeSet<i32> {
        self.keyed
            .keys()
            .chain(self.displayed.keys())
            .copied()
            .collect()
    }
}

/// Scan the four chain seeds for `dialog_choice` triggers and
/// `display_dialog` actions.
///
/// `dialog_choice`'s dialog id arrives as a QUOTED `event_key` and is
/// parsed as an integer, mirroring the loader exactly
/// (`loader/trigger.rs`: `"dialog_choice" => Trigger::OnDialogChoice {
/// dialog_id: key?.parse().ok()? }`). `display_dialog`'s arrives as the
/// unquoted numeric `target_id`.
///
/// Every `content_chains` row in these four files carries
/// `enabled = true` (checked 2026-09-21), so nothing is filtered on it.
/// A chain disabled later would still be linted, which is the safe
/// direction: the cost is a false positive on a dead chain, not a missed
/// soft-lock on a live one.
pub(crate) fn load_chain_refs(root: &Path) -> ChainRefs {
    let dir = root.join("db/resources/Content/Seed");
    let mut refs = ChainRefs::default();

    for file in CHAIN_FILES {
        for stmt in sql_statements(&read(&dir.join(file))) {
            for row in insert_rows(&stmt, "content_triggers") {
                if text(&row, "event_type").as_deref() != Some("dialog_choice") {
                    continue;
                }
                if let (Some(chain), Some(dialog)) = (
                    int(&row, "chain_id"),
                    text(&row, "event_key").and_then(|k| k.parse::<i32>().ok()),
                ) {
                    refs.keyed
                        .entry(dialog)
                        .or_default()
                        .push((file.to_string(), chain));
                }
            }
            for row in insert_rows(&stmt, "content_actions") {
                if text(&row, "action_type").as_deref() != Some("display_dialog") {
                    continue;
                }
                if let (Some(chain), Some(dialog)) = (int(&row, "chain_id"), int(&row, "target_id"))
                {
                    refs.displayed
                        .entry(dialog)
                        .or_default()
                        .push((file.to_string(), chain));
                }
            }
        }
    }

    refs
}
