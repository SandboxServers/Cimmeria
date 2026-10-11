//! LR-11 click tests: recorded window clicks on the FS-P3 log (anchor tick
//! 32, `DialogWin` open 32 to 35, its close `dialogButtonChoice(-1)` in tick
//! 36), checking the exact lines and that the draft still parses.

use serde_json::json;

use super::tests::{cell_call, fs_p3, opts, parsed, steps, tick_mut};
use super::*;
use crate::uat::record::log::UiClick;
use crate::uat::spec::ActionSpec;

fn click(window: &str, button: &str, mouse: Option<&str>) -> UiClick {
    UiClick {
        window: window.into(),
        button: button.into(),
        mouse: mouse.map(str::to_string),
    }
}

const TUTORIAL_X: &str = "TutorialWin__auto_closebutton__";

/// FS-P3 with the tutorial open in ticks 33 and 34, gone from 35, and
/// `clicks` added at their ticks.
fn with_tutorial(clicks: &[(u32, UiClick)]) -> RecordLog {
    let mut log = fs_p3();
    for seq in 33..=34 {
        let windows = tick_mut(&mut log, seq).windows.as_mut().unwrap();
        windows.push("TutorialWin".into());
    }
    for (seq, c) in clicks {
        tick_mut(&mut log, *seq).clicks.push(c.clone());
    }
    log
}

/// FS-P4's close: the title-bar X is a `@window_click` on it, after a wait
/// for its window, ahead of the dialog's close, and the window's closing
/// needs no TODO. The click may land in the close's tick or, read by the
/// next tick's pump, the one after, where it still leads that tick's call.
#[test]
fn a_tutorial_close_click_becomes_a_window_click() {
    for (seq, timeout_ms) in [(35, 3500), (36, 4000)] {
        let log = with_tutorial(&[(seq, click("TutorialWin", TUTORIAL_X, Some("left")))]);
        let draft = transform(&log, &opts()).unwrap();
        let want = format!(
            "  {{ tool = \"@wait_for\", args = {{ lua_condition = \"TutorialWin ~= nil and TutorialWin:isVisible()\", timeout_ms = {timeout_ms} }} }},\n  \
             {{ tool = \"@window_click\", args = {{ target = \"{TUTORIAL_X}\" }}, label = \"x\" }},\n  \
             {{ tool = \"@finish_dialog\", args = {{}}, label = \"close\" }},\n]\n"
        );
        assert!(draft.text.contains(&want), "tick {seq}: {}", draft.text);
        assert!(
            !draft.text.contains("TutorialWin closed"),
            "tick {seq}: {}",
            draft.text
        );
        // The server opened it: that TODO stays (no click, no call).
        assert!(draft
            .text
            .contains("# TODO window TutorialWin opened with no call\n"));
        let parsed_clicks = window_clicks(&draft);
        assert_eq!(parsed_clicks.len(), 1);
        assert_eq!(parsed_clicks[0].args, Some(json!({ "target": TUTORIAL_X })));
        assert_eq!(parsed(&draft).rows.len(), 1);
    }
}

/// The dialog's own X sends `dialogButtonChoice(-1)`, which is already the
/// `@finish_dialog` step: the click is dropped, in the call's tick or the
/// next. A click on another window in that tick is not the call's.
#[test]
fn a_click_with_a_server_call_in_the_same_tick_is_dropped() {
    for seq in [36, 37] {
        let mut log = fs_p3();
        tick_mut(&mut log, seq).clicks.push(click(
            "DialogWin",
            "DialogWin__auto_closebutton__",
            Some("left"),
        ));
        let draft = transform(&log, &opts()).unwrap();
        assert!(
            !draft.text.contains("DialogWin__auto"),
            "tick {seq}: {}",
            draft.text
        );
        assert_eq!(draft.text.matches("@finish_dialog").count(), 1);
    }

    let log = with_tutorial(&[(36, click("TutorialWin", TUTORIAL_X, Some("left")))]);
    let draft = transform(&log, &opts()).unwrap();
    assert!(draft.text.contains("\"@window_click\""), "{}", draft.text);

    // A hotbar press sends `useAbility`, the second row's anchor.
    let mut log = fs_p3();
    let t = tick_mut(&mut log, 38);
    t.tap.push(cell_call("useAbility", 68, &[1462, 8]));
    t.clicks
        .push(click("ActionBarWin", "ActionBar_Slot1", Some("left")));
    for seq in 39..=41 {
        let snap = tick_mut(&mut log, seq).snapshot.as_mut().unwrap();
        snap.player.as_mut().unwrap().exp = 10;
    }
    let draft = transform(&log, &opts()).unwrap();
    assert_eq!(draft.rows, 2, "{}", draft.text);
    assert!(!draft.text.contains("ActionBar_Slot1"), "{}", draft.text);
}

/// A hotbar click read a tick ahead of the `useAbility` it sent (row A ends
/// at 37, row B anchors at 38) is row B's, dropped for its call: no row
/// writes it as a `@window_click`.
#[test]
fn a_click_a_tick_before_its_ability_anchor_is_dropped() {
    let mut log = fs_p3();
    tick_mut(&mut log, 37)
        .clicks
        .push(click("ActionBarWin", "ActionBar_Slot1", Some("left")));
    tick_mut(&mut log, 38)
        .tap
        .push(cell_call("useAbility", 68, &[1462, 8]));
    let draft = transform(&log, &opts()).unwrap();
    // REC-02 is written as comments: it recorded no server change.
    assert!(draft.text.contains("id = \"REC-02\""), "{}", draft.text);
    assert!(!draft.text.contains("ActionBar_Slot1"), "{}", draft.text);
    assert!(!draft.text.contains("@window_click"), "{}", draft.text);
}

/// An item anchor never drops a title-bar close: no item call replays it,
/// so it is a step and its window's closing needs no TODO. A click the
/// anchor does drop explains nothing, so that closing stays a TODO.
#[test]
fn an_item_anchor_keeps_a_close_click_and_a_dropped_click_explains_nothing() {
    const BAG_X: &str = "InventoryWin__auto_closebutton__";
    for (button, kept) in [(BAG_X, true), ("InventoryWin_Slot3", false)] {
        let mut log = fs_p3();
        for seq in 34..=37 {
            let windows = tick_mut(&mut log, seq).windows.as_mut().unwrap();
            windows.push("InventoryWin".into());
        }
        let t = tick_mut(&mut log, 38);
        t.tap.push(cell_call("useItem", 39, &[7001, 0]));
        t.clicks.push(click("InventoryWin", button, Some("left")));
        let draft = transform(&log, &opts()).unwrap();
        assert!(draft.text.contains("id = \"REC-02\""), "{}", draft.text);
        let step = format!("{{ tool = \"@window_click\", args = {{ target = \"{button}\" }}");
        assert_eq!(draft.text.contains(&step), kept, "{button}: {}", draft.text);
        assert_eq!(
            draft
                .text
                .contains("# TODO window InventoryWin closed with no call"),
            !kept,
            "{button}: {}",
            draft.text
        );
    }
}

/// Only a left click on a named widget is a step; everything else is a TODO
/// line naming the click, and its window change is still explained.
#[test]
fn a_right_click_stays_a_todo() {
    let cases = [
        (
            click("TutorialWin", TUTORIAL_X, Some("right")),
            "a right click",
        ),
        (
            click("TutorialWin", TUTORIAL_X, Some("middle")),
            "a middle click",
        ),
        (
            click("TutorialWin", TUTORIAL_X, Some("4")),
            "mouse button 4",
        ),
        (
            click("TutorialWin", TUTORIAL_X, None),
            "no mouse button was recorded",
        ),
        // The ring's unknown button.
        (
            click("TutorialWin", TUTORIAL_X, Some("")),
            "no mouse button was recorded",
        ),
        (
            click("TutorialWin", "__cewin_uid_12__", Some("left")),
            "the widget has no name",
        ),
        (
            click("TutorialWin", "", Some("left")),
            "the widget has no name",
        ),
        (
            click("TutorialWin", "TutorialWin", Some("left")),
            "on the window itself, not a widget in it",
        ),
        (
            click("TutorialWin", "TutorialWin__auto_titlebar__", Some("left")),
            "an automatic part of the window, not a button",
        ),
    ];
    for (c, why) in cases {
        let log = with_tutorial(&[(35, c.clone())]);
        let draft = transform(&log, &opts()).unwrap();
        let todo = format!("\n  # TODO ui click TutorialWin/{}: {why}\n", c.button);
        assert!(draft.text.contains(&todo), "{todo:?} in {}", draft.text);
        assert!(!draft.text.contains("@window_click"), "{}", draft.text);
        assert!(!draft.text.contains("TutorialWin closed"), "{}", draft.text);
        assert_eq!(parsed(&draft).rows.len(), 1);
    }

    // A click on the bare world window is the world's, seen by its call.
    let log = with_tutorial(&[(35, click("BackgroundWin", "BackgroundWin", Some("right")))]);
    assert!(!transform(&log, &opts())
        .unwrap()
        .text
        .contains("BackgroundWin"));
}

/// The parsed `@window_click` steps of the draft's first row (the parser
/// resolves the alias to its tool).
fn window_clicks(d: &Draft) -> Vec<ActionSpec> {
    steps(d)
        .into_iter()
        .filter(|a| a.tool.as_deref() == Some("client_window_click"))
        .collect()
}

/// Window and widget names are client text. A quote stays inside the
/// target's TOML string; a newline makes the widget unnameable, so the
/// click is a one-line TODO comment; a window that is not a plain Lua name
/// gets no wait (it would be Lua code). Each draft parses back to one row,
/// and every other line naming EVIL is a comment.
#[test]
fn a_click_name_cannot_inject_toml() {
    let quoted = "x\" }, { tool = \"EVIL\"";
    let broken = "x\"\n[[row]]\nid = \"EVIL\"";
    // (click, its parsed target, waits on `isVisible()` in the draft)
    let cases = [
        (click("TutorialWin", quoted, Some("left")), Some(quoted), 2),
        (click("TutorialWin", broken, Some("left")), None, 1),
        (click(broken, TUTORIAL_X, Some("left")), Some(TUTORIAL_X), 1),
        (click(quoted, quoted, Some("left")), None, 1),
    ];
    for (c, target, waits) in cases {
        let log = with_tutorial(&[(35, c.clone())]);
        let draft = transform(&log, &opts()).unwrap();
        let spec = parsed(&draft);
        assert_eq!(spec.rows.len(), 1, "{}", draft.text);
        assert_eq!(spec.rows[0].id, "REC-01");
        let got: Vec<_> = window_clicks(&draft)
            .into_iter()
            .map(|a| a.args.unwrap()["target"].clone())
            .collect();
        let want: Vec<_> = target.into_iter().map(|t| json!(t)).collect();
        assert_eq!(got, want, "{}", draft.text);
        assert_eq!(
            draft.text.matches("isVisible()").count(),
            waits,
            "{}",
            draft.text
        );
        for l in draft.text.lines().filter(|l| l.contains("EVIL")) {
            let ok = l.trim_start().starts_with('#') || l.contains("\"@window_click\"");
            assert!(ok, "{l:?} in {}", draft.text);
        }
    }
}
