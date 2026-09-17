//! Read-only console queries — `.help`, the `.search*` family, and `.players`
//! (category D). Search runs base-side (the cell caches resource ids but
//! not display names); the rest read cell state and reply via the feedback
//! channel.
//!
//! Legacy reference: `deprecated/python/cell/commands/Resource.py`
//! (`searchItem`/`searchMission`/`searchTemplate`) and
//! `deprecated/python/cell/commands/Misc.py` (`players`).

use tokio::sync::mpsc;

use super::registry::arg_specs;
use super::{send_gm_feedback, Spec, COMMANDS};
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

/// `.help [filter]` — list the console commands (optionally filtered by
/// substring) **sorted by command name**, each with its one-line summary.
/// Mirrors the legacy `help`, which also sorted by name.
///
/// When the filtered result set has `<= 3` commands, also prints each
/// documented argument (see [`arg_specs`]) as `    name (type): desc` when
/// required or `    [name] (type): desc` when optional — mirroring legacy
/// `ConsoleCommands.py::help`'s detail view. Legacy determined
/// required-vs-optional via `index <= cmd.argsMin`, an off-by-one that
/// mis-marks a command's *last* optional argument as required (e.g. its own
/// `command` arg, or `searchitem`'s `name2`); this restores the intended
/// `index < min` semantics instead of reproducing the bug (D02).
pub(super) async fn help(caller_id: u32, args: &[&str], tx: &mpsc::Sender<CellToBaseMsg>) {
    let matches = help_specs(args.first().copied());
    for spec in &matches {
        send_gm_feedback(caller_id, &format!(".{}: {}", spec.name, spec.help), tx).await;
        if matches.len() <= 3 {
            for (i, a) in arg_specs(spec.name).iter().enumerate() {
                let line = if i < spec.min {
                    format!("    {} ({}): {}", a.name, a.ty, a.desc)
                } else {
                    format!("    [{}] ({}): {}", a.name, a.ty, a.desc)
                };
                send_gm_feedback(caller_id, &line, tx).await;
            }
        }
    }
    if matches.is_empty() {
        // Verbatim legacy wording (`ConsoleCommands.py::help`'s
        // `player.feedback('No command found with that name')`), not the
        // "{name}: ..." prefix convention other feedback lines use — kept
        // exact so it's a stable, testable string.
        send_gm_feedback(caller_id, "No command found with that name", tx).await;
    }
}

/// The `.help` result set: commands matching the optional substring `filter`,
/// sorted alphabetically by name. Split out so the sort is unit-testable.
fn help_specs(filter: Option<&str>) -> Vec<&'static Spec> {
    let needle = filter.map(str::to_ascii_lowercase);
    let mut matches: Vec<&'static Spec> = COMMANDS
        .iter()
        .filter(|spec| needle.as_deref().is_none_or(|f| spec.name.contains(f)))
        .collect();
    matches.sort_by_key(|spec| spec.name);
    matches
}

/// Build the search query string from one or two name args (legacy joined a
/// second optional word with a space) and dispatch the base-side search.
async fn search(caller_id: u32, kind: u8, args: &[&str], tx: &mpsc::Sender<CellToBaseMsg>) {
    let query = args.join(" ");
    let _ = tx
        .send(CellToBaseMsg::ConsoleSearch {
            entity_id: caller_id,
            kind,
            query,
        })
        .await;
}

/// `.searchitem <name> [name2]` — search item designs by name.
pub(super) async fn search_item(
    caller_id: u32,
    args: &[&str],
    tx: &mpsc::Sender<CellToBaseMsg>,
    _space_mgr: &mut SpaceManager,
) {
    search(caller_id, 0, args, tx).await;
}

/// `.searchmission <name> [name2]` — search mission designs by name.
pub(super) async fn search_mission(
    caller_id: u32,
    args: &[&str],
    tx: &mpsc::Sender<CellToBaseMsg>,
    _space_mgr: &mut SpaceManager,
) {
    search(caller_id, 1, args, tx).await;
}

/// `.searchtemplate <name> [name2]` — search entity templates by name.
pub(super) async fn search_template(
    caller_id: u32,
    args: &[&str],
    tx: &mpsc::Sender<CellToBaseMsg>,
    _space_mgr: &mut SpaceManager,
) {
    search(caller_id, 2, args, tx).await;
}

/// `.players` — list the players in the caller's space. Cell-scoped (the cell
/// only knows its own spaces); mirrors the legacy `players` / `gmUsers`.
pub(super) async fn players(
    caller_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let Some(space_id) = space_mgr.get_entity_space_id(caller_id) else {
        send_gm_feedback(caller_id, "players: you are not in a space.", tx).await;
        return;
    };
    let mut ids: Vec<u32> = space_mgr
        .all_player_entity_ids()
        .into_iter()
        .filter(|&pid| space_mgr.get_entity_space_id(pid) == Some(space_id))
        .collect();
    ids.sort_unstable();
    if ids.is_empty() {
        send_gm_feedback(caller_id, "players: none in your space.", tx).await;
        return;
    }
    let list = ids
        .iter()
        .map(|pid| {
            let char_id = space_mgr.get_entity(*pid).and_then(|e| e.player_id);
            match char_id {
                Some(c) => format!("{pid} (char {c})"),
                None => pid.to_string(),
            }
        })
        .collect::<Vec<_>>()
        .join(", ");
    send_gm_feedback(
        caller_id,
        &format!("players ({} in space): {list}", ids.len()),
        tx,
    )
    .await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn help_is_sorted_by_name_and_lists_all() {
        let all = help_specs(None);
        let names: Vec<&str> = all.iter().map(|s| s.name).collect();
        let mut sorted = names.clone();
        sorted.sort_unstable();
        assert_eq!(names, sorted, ".help must be sorted alphabetically by name");
        assert_eq!(
            all.len(),
            COMMANDS.len(),
            "unfiltered .help lists every command"
        );
    }

    #[test]
    fn help_filter_matches_substring_only() {
        let paths = help_specs(Some("path"));
        assert!(!paths.is_empty());
        assert!(paths.iter().all(|s| s.name.contains("path")));
        assert!(help_specs(Some("definitely-no-such-command")).is_empty());
    }
}
