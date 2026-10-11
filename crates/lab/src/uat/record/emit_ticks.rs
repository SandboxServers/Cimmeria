//! LR-05: tick lookups for the row mapping, and the walk over a row's
//! window samples that finds the dialog opening and the window changes
//! nothing explains.

use super::super::decode::Call;
use super::super::log::{RecordLog, Tick};
use super::super::segment::{AnchorKind, RecordedRow};
use super::one_line;

/// The dialog window's CEGUI name.
pub(super) const DIALOG_WIN: &str = "DialogWin";

/// What the row's window samples show.
pub(super) struct WindowWalk {
    /// Host ms from the anchor's tick to the tick `DialogWin` opened in.
    pub dialog_opened: Option<i64>,
    /// Window changes no call or click explains, with their tick.
    pub todos: Vec<(u32, String)>,
}

/// Compare each tick's windows in the row with the last read before it.
/// `DialogWin` opening is the interact's (or an `onDialogDisplay`'s) and
/// closing a dialog choice's; any other change needs a click on that window
/// in its tick or a neighbour, else it is a TODO (keys are never recorded,
/// D-LR9). The click explains the change whether it became a step, a TODO
/// or was dropped for its call: each of those already accounts for it.
///
/// A neighbour on both sides because a tick pumps the clicks before it
/// reads the UI: a click between the two shows its window change in this
/// tick and the click itself in the next.
pub(super) fn walk_windows(log: &RecordLog, row: &RecordedRow) -> WindowWalk {
    let mut walk = WindowWalk {
        dialog_opened: None,
        todos: Vec::new(),
    };
    let anchor_ms = host_ms(log, row.anchor.tick);
    let dialog_shown = matches!(row.anchor.kind, AnchorKind::Interact)
        || row.server.iter().any(|(_, m, _)| m == "onDialogDisplay");
    let dialog_chosen = row
        .calls
        .iter()
        .any(|(_, c)| matches!(c, Call::DialogChoice { .. }));
    let mut prev: Vec<String> = before(log, row.anchor.tick, |t| t.windows.as_ref())
        .cloned()
        .unwrap_or_default();
    for t in in_row(log, row) {
        let Some(now) = &t.windows else { continue };
        let clicked = |w: &str| {
            [t.seq.checked_sub(1), Some(t.seq), t.seq.checked_add(1)]
                .into_iter()
                .flatten()
                .filter_map(|seq| tick(log, seq))
                .any(|k| k.clicks.iter().any(|c| c.window == w))
        };
        for (w, opened) in changes(&prev, now) {
            let explained = match (w.as_str(), opened) {
                (DIALOG_WIN, true) => dialog_shown,
                (DIALOG_WIN, false) => dialog_chosen,
                _ => false,
            } || clicked(&w);
            if w == DIALOG_WIN && opened && walk.dialog_opened.is_none() {
                walk.dialog_opened = Some(t.host_ms.saturating_sub(anchor_ms));
            }
            if !explained {
                let how = if opened { "opened" } else { "closed" };
                walk.todos
                    .push((t.seq, format!("window {} {how} with no call", one_line(&w))));
            }
        }
        prev.clone_from(now);
    }
    walk
}

/// Windows that opened (true) or closed (false) from `prev` to `now`,
/// opened first, each in its list's order.
fn changes(prev: &[String], now: &[String]) -> Vec<(String, bool)> {
    let opened = now
        .iter()
        .filter(|w| !prev.contains(w))
        .map(|w| (w.clone(), true));
    let closed = prev
        .iter()
        .filter(|w| !now.contains(w))
        .map(|w| (w.clone(), false));
    opened.chain(closed).collect()
}

/// The value of `get` in the last tick before `seq` that has one, else in
/// tick `seq` itself (a recording that starts on its anchor).
pub(super) fn before<'a, T: 'a>(
    log: &'a RecordLog,
    seq: u32,
    get: impl Fn(&'a Tick) -> Option<&'a T>,
) -> Option<&'a T> {
    log.ticks
        .iter()
        .rev()
        .filter(|t| t.seq < seq)
        .find_map(&get)
        .or_else(|| tick(log, seq).and_then(get))
}

/// The tick numbered `seq`.
pub(super) fn tick(log: &RecordLog, seq: u32) -> Option<&Tick> {
    log.ticks.iter().find(|t| t.seq == seq)
}

/// The row's ticks, from the anchor's to its last.
pub(super) fn in_row<'a>(
    log: &'a RecordLog,
    row: &'a RecordedRow,
) -> impl Iterator<Item = &'a Tick> {
    log.ticks
        .iter()
        .filter(move |t| (row.anchor.tick..=row.last_tick).contains(&t.seq))
}

/// Tick `seq`'s host time; 0 when the tick is missing.
pub(super) fn host_ms(log: &RecordLog, seq: u32) -> i64 {
    tick(log, seq).map_or(0, |t| t.host_ms)
}
