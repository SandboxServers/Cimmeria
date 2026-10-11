//! LR-11: recorded CEGUI window clicks (LR-10's `ui.click`) as steps, the
//! D-LR9 G1 gap. A left click on a named widget is
//! `@window_click { target = <widget> }`, after a `@wait_for` on its window
//! (FS-P4's hand-written close does the same: the tutorial can open seconds
//! after the interact). Anything else a click shows is a TODO line, except
//! the two kinds that replay through another step and are dropped.

use super::super::super::clauses::dialog_timeout_ms;
use super::super::super::decode::Call;
use super::super::super::log::{RecordLog, UiClick};
use super::super::super::segment::{AnchorKind, RecordedRow};
use super::super::ticks::{host_ms, DIALOG_WIN};
use super::super::{one_line, Line};
use super::{action, int, s, table, Labels};

/// The suffix CEGUI gives a frame's title-bar close button.
const CLOSE_BUTTON: &str = "__auto_closebutton__";
/// The marker of CEGUI's other automatic child widgets (title bar, scroll
/// bar parts): stable names, but nothing a step should press.
const AUTO_CHILD: &str = "__auto_";
/// CEGUI 0.6's prefix for a window created without a name: a counter, so
/// the same widget has another name in the next session.
const GENERATED_NAME: &str = "__cewin_uid_";
/// The full-screen window under every other one. A click on it alone is a
/// click on the world, which the server sees as `interact` or
/// `setTargetID`; it is dropped. Not yet seen in a live record (LR-13).
const WORLD_WIN: &str = "BackgroundWin";

/// The row's clicks as lines, each with its tick, in click order.
pub(super) fn click_lines(
    log: &RecordLog,
    row: &RecordedRow,
    labels: &mut Labels,
) -> Vec<(u32, Line)> {
    let anchor_ms = host_ms(log, row.anchor.tick);
    let mut out = Vec::new();
    for (at, c) in &row.clicks {
        let at = *at;
        if c.window == WORLD_WIN && c.button == WORLD_WIN {
            continue;
        }
        if let Some(why) = not_a_step(c) {
            let what = format!(
                "ui click {}/{}: {why}",
                one_line(&c.window),
                one_line(&c.button)
            );
            out.push((at, Line::Todo(what)));
            continue;
        }
        if replayed_by_a_call(row, at, c) {
            continue;
        }
        if is_lua_name(&c.window) {
            let visible = format!("{0} ~= nil and {0}:isVisible()", c.window);
            let wait_ms = dialog_timeout_ms(host_ms(log, at).saturating_sub(anchor_ms));
            let args = table(vec![
                ("lua_condition", s(&visible)),
                ("timeout_ms", int(wait_ms)),
            ]);
            out.push((
                at,
                action(vec![("tool", s("@wait_for")), ("args", args)], None),
            ));
        }
        let label = if c.button.ends_with(CLOSE_BUTTON) {
            "x"
        } else {
            "click"
        };
        out.push((
            at,
            action(
                vec![
                    ("tool", s("@window_click")),
                    ("args", table(vec![("target", s(&c.button))])),
                ],
                Some(labels.take(label)),
            ),
        ));
    }
    out
}

/// Why a click cannot be replayed by name; `None` for a left click on a
/// named widget inside its window.
fn not_a_step(c: &UiClick) -> Option<String> {
    let why = match c.mouse.as_deref() {
        Some("left") => return named(c).map(str::to_string),
        Some("right") => "a right click".to_string(),
        Some("middle") => "a middle click".to_string(),
        Some(other) => format!("mouse button {}", one_line(other)),
        None => "no mouse button was recorded".to_string(),
    };
    Some(why)
}

/// Why a left click's widget cannot be named in a step.
fn named(c: &UiClick) -> Option<&'static str> {
    let b = c.button.as_str();
    if b.is_empty() || b.contains(GENERATED_NAME) {
        Some("the widget has no name")
    } else if b.chars().any(char::is_control) {
        // No layout names a widget so; as a target it would also be the
        // draft's only multi-line string.
        Some("the widget name has a control character")
    } else if c.window.is_empty() {
        Some("no window holds the widget")
    } else if b == c.window {
        Some("on the window itself, not a widget in it")
    } else if b.contains(AUTO_CHILD) && !b.ends_with(CLOSE_BUTTON) {
        Some("an automatic part of the window, not a button")
    } else {
        None
    }
}

/// The click already replays through a step its call maps to: a dialog
/// choice (`@finish_dialog`, or its TODO) for a click on `DialogWin`; the
/// row's item or ability anchor for a click on any other window (a hotbar
/// button sends `useAbility`). `interact` and region anchors come from the
/// world, never from a window. A call in the click's tick or a neighbour
/// counts: the tick reads the clicks before the tap, so a click made
/// between the two lands one tick after its call.
fn replayed_by_a_call(row: &RecordedRow, at: u32, c: &UiClick) -> bool {
    let near = |t: u32| t.abs_diff(at) <= 1;
    if c.window == DIALOG_WIN {
        row.calls
            .iter()
            .any(|(t, call)| near(*t) && matches!(call, Call::DialogChoice { .. }))
    } else {
        near(row.anchor.tick)
            && matches!(
                row.anchor.kind,
                AnchorKind::UseItem | AnchorKind::MoveItem | AnchorKind::UseAbility
            )
    }
}

/// A window name the `@wait_for` Lua can use as a global: an identifier.
/// The name is client text, so anything else gets no wait rather than
/// reaching the Lua chunk.
fn is_lua_name(w: &str) -> bool {
    let mut chars = w.chars();
    chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
        && !LUA_KEYWORDS.contains(&w)
}

const LUA_KEYWORDS: &[&str] = &[
    "and", "break", "do", "else", "elseif", "end", "false", "for", "function", "if", "in", "local",
    "nil", "not", "or", "repeat", "return", "then", "true", "until", "while",
];
