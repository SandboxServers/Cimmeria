//! LR-05 mapping-table tests: one rule each, on the FS-P3 log with one fact
//! changed (or a one-row item log), checking the exact line the rule writes
//! and that the draft still parses.

use serde_json::{json, Value};

use super::tests::{cell_call, fs_p3, item, item_log, opts, parsed, tick_mut};
use super::*;
use crate::uat::record::log::{DialogSample, UiClick};

/// `triggerClientHintedGenericRegion`: region INT32, entering UINT8, after
/// the entity prefix and the sub-slot byte.
fn region_call(region: i32) -> Value {
    let mut hex = String::from("000000000e");
    for b in region.to_le_bytes() {
        hex.push_str(&format!("{b:02x}"));
    }
    hex.push_str("01");
    json!({ "dir": "in", "msg_name": "triggerClientHintedGenericRegion", "method_index": 85, "msg_id": 189, "args_hex": hex })
}

/// `dialogButtonChoice(3995, button)`.
fn choice_call(button: i32) -> Value {
    let mut hex = String::from("000000000e9b0f0000");
    for b in button.to_le_bytes() {
        hex.push_str(&format!("{b:02x}"));
    }
    json!({ "dir": "in", "msg_name": "dialogButtonChoice", "method_index": 75, "msg_id": 189, "args_hex": hex })
}

#[test]
fn default_options_write_a_valid_section_without_a_base() {
    let draft = transform(&fs_p3(), &TransformOptions::default()).unwrap();
    let s = parsed(&draft).section;
    assert_eq!((s.account.as_str(), s.character.as_str()), ("gm", "lab"));
}

/// A region entry a mission diff follows is a row: `@stand` at the
/// player's position in that tick.
#[test]
fn a_region_anchor_stands_at_the_players_position() {
    let mut log = fs_p3();
    for row in &mut tick_mut(&mut log, 32).tap {
        if row["msg_name"] == "interact" {
            *row = region_call(42);
        }
    }
    let draft = transform(&log, &opts()).unwrap();
    let want = "title = \"Enter region 42\"\n";
    assert!(draft.text.contains(want), "{}", draft.text);
    let want = "step = [\n  { tool = \"@stand\", args = { point = { x = -325.00, y = 73.60, z = -212.80 } }, label = \"enter\" },\n";
    assert!(draft.text.contains(want), "{}", draft.text);
    assert_eq!(draft.rows, 1);
    assert_eq!(parsed(&draft).rows.len(), 1);
}

/// A known server event is a `wait_server` matched to this player, in tick
/// order; an unknown key is a TODO.
#[test]
fn a_known_server_event_is_a_wait_server_for_this_player() {
    let mut log = fs_p3();
    tick_mut(&mut log, 34).server_events = vec!["cinematic-hold".into()];
    let draft = transform(&log, &opts()).unwrap();
    let want = "timeout_ms = 2000 } },\n  \
        { wait_server = { log = \"Cinematic AoI hold: released\", target = \"aoi.cinematic_hold\", \
        fields = { event = \"hold_released\", witness_id = \"${player_entity_id}\" }, timeout_ms = 6000 }, \
        label = \"cinematic-hold\" },\n  { tool = \"@finish_dialog\"";
    assert!(draft.text.contains(want), "{}", draft.text);
    assert_eq!(parsed(&draft).rows.len(), 1);

    tick_mut(&mut log, 34).server_events = vec!["door-open".into()];
    let draft = transform(&log, &opts()).unwrap();
    assert!(
        draft.text.contains("\n  # TODO server event door-open\n"),
        "{}",
        draft.text
    );
}

/// The close at tick 36 becomes `button`, with `before` shown in tick 35
/// and `on_choice` in tick 36.
fn dialog_draft(button: i32, before: &[&str], on_choice: &[&str]) -> Draft {
    let mut log = fs_p3();
    let sample = |b: &[&str]| DialogSample {
        title: Some("Corporal Frost".into()),
        buttons: b.iter().map(|s| s.to_string()).collect(),
    };
    tick_mut(&mut log, 35).dialog = Some(sample(before));
    let t = tick_mut(&mut log, 36);
    t.dialog = Some(sample(on_choice));
    t.tap = vec![choice_call(button)];
    transform(&log, &opts()).unwrap()
}

/// Only a lone Accept in the tick before is surely the clicked button: the
/// wire carries the cooked ButtonID, not an index into the visible list.
#[test]
fn a_dialog_choice_is_an_accept_only_for_a_lone_accept_the_tick_before() {
    let accept = "{ tool = \"@finish_dialog\", args = { accept = true }, label = \"accept\" }";
    let d = dialog_draft(0, &["Accept"], &[]);
    assert!(d.text.contains(accept), "{}", d.text);
    assert_eq!(parsed(&d).rows.len(), 1);

    for (button, before, on_choice, todo) in [
        // A decline must never replay as an accept.
        (
            1,
            &["Accept", "Decline"][..],
            &[][..],
            "(3995, 1): buttons [Accept, Decline]",
        ),
        // Nor can the position name the Accept.
        (
            0,
            &["Accept", "Decline"][..],
            &[][..],
            "(3995, 0): buttons [Accept, Decline]",
        ),
        // The tick before is read, not the choice's own tick.
        (0, &[][..], &["Accept"][..], "(3995, 0): buttons []"),
        (
            0,
            &["Continue"][..],
            &[][..],
            "(3995, 0): buttons [Continue]",
        ),
    ] {
        let d = dialog_draft(button, before, on_choice);
        let line = format!("\n  # TODO dialogButtonChoice{todo}\n");
        assert!(d.text.contains(&line), "{line:?} in {}", d.text);
        assert!(!d.text.contains("accept = true"), "{}", d.text);
        assert_eq!(parsed(&d).rows.len(), 1);
    }
}

/// FS-P3 plus a `useAbility(1462, target)` at tick 38, with the exp gain
/// it caused in ticks 39 to 41 (so the second row has a clause).
fn two_rows(target: i32) -> RecordLog {
    let mut log = fs_p3();
    tick_mut(&mut log, 38)
        .tap
        .push(cell_call("useAbility", 68, &[1462, target]));
    for seq in 39..=41 {
        let snap = tick_mut(&mut log, seq).snapshot.as_mut().unwrap();
        snap.player.as_mut().unwrap().exp = 10;
    }
    log
}

#[test]
fn two_anchors_are_two_numbered_rows_with_summed_counts() {
    let draft = transform(&two_rows(8), &opts()).unwrap();
    assert_eq!(
        (draft.rows, draft.actions, draft.todos, draft.dropped_rows),
        (2, 6, 1, 0)
    );
    // The second row restores nothing: no fixture, one TODO for the exp.
    let want = "# TODO row changes player exp; a fixture cannot set them\n";
    assert!(draft.text.ends_with(want), "{}", draft.text);
    assert_eq!(draft.text.matches("[row.fixture]").count(), 1);
    let spec = parsed(&draft);
    let ids: Vec<&str> = spec.rows.iter().map(|r| r.id.as_str()).collect();
    assert_eq!(ids, ["REC-01", "REC-02"]);
    assert!(draft
        .text
        .contains("notes = \"recorded 2026-10-12, ticks 32-37\""));
    // On the player: no `@target`, just the ability.
    let want = "id = \"REC-02\"\ntitle = \"Use ability 1462\"\n";
    assert!(draft.text.contains(want), "{}", draft.text);
    let want = "step = [\n  { tool = \"@use_ability\", args = { ability_id = 1462 }, label = \"use\" },\n]\n";
    assert!(draft.text.contains(want), "{}", draft.text);

    // From tick 33 the first row is dropped and the survivor renumbered.
    let mut o = opts();
    o.from_tick = Some(33);
    let draft = transform(&two_rows(8), &o).unwrap();
    assert_eq!((draft.rows, draft.dropped_rows), (1, 1));
    let row = &parsed(&draft).rows[0];
    assert_eq!(
        (row.id.as_str(), row.title.as_str()),
        ("REC-01", "Use ability 1462")
    );
}

#[test]
fn two_closes_in_one_row_get_distinct_labels() {
    let mut log = fs_p3();
    tick_mut(&mut log, 37).tap.push(choice_call(-1));
    let draft = transform(&log, &opts()).unwrap();
    let want =
        "label = \"close\" },\n  { tool = \"@finish_dialog\", args = {}, label = \"close-2\" },\n";
    assert!(draft.text.contains(want), "{}", draft.text);
    assert_eq!(parsed(&draft).rows.len(), 1);
}

#[test]
fn a_use_ability_on_another_entity_targets_its_tag_or_writes_a_todo() {
    let draft = transform(&two_rows(100751), &opts()).unwrap();
    let want = "title = \"Use ability 1462 on ArmYourself_FrostBody\"\n";
    assert!(draft.text.contains(want), "{}", draft.text);
    let want = "step = [\n  { tool = \"@target\", args = { tag = \"ArmYourself_FrostBody\" }, label = \"target\" },\n  \
                { tool = \"@use_ability\", args = { ability_id = 1462 }, label = \"use\" },\n]\n";
    assert!(draft.text.contains(want), "{}", draft.text);

    // Untagged: the row still runs; a TODO stands where `@target` would.
    let draft = transform(&two_rows(999), &opts()).unwrap();
    let want = "step = [\n  # TODO useAbility 1462 target 999 has no tag\n  \
                { tool = \"@use_ability\", args = { ability_id = 1462 }, label = \"use\" },\n]\n";
    assert!(draft.text.contains(want), "{}", draft.text);
    // The target TODO and the second row's player-exp fixture TODO.
    assert_eq!((draft.rows, draft.todos), (2, 2));
    assert_eq!(parsed(&draft).rows.len(), 2);
}

/// A window that opens with no call is a TODO, unless a click on it was
/// recorded in the tick before, or after (LR-11: the pump reads clicks
/// before the UI read, so a click can land a tick after its change).
#[test]
fn a_window_opened_with_no_call_is_a_todo_unless_clicked() {
    let mut log = fs_p3();
    for seq in 34..=41 {
        let windows = tick_mut(&mut log, seq).windows.as_mut().unwrap();
        windows.push("InventoryWin".into());
    }
    let todo = "\n  # TODO window InventoryWin opened with no call\n";
    let draft = transform(&log, &opts()).unwrap();
    assert!(draft.text.contains(todo), "{}", draft.text);

    for seq in [33, 35] {
        let mut log = log.clone();
        tick_mut(&mut log, seq).clicks.push(UiClick {
            window: "InventoryWin".into(),
            button: "Bag1".into(),
            mouse: Some("left".into()),
        });
        let draft = transform(&log, &opts()).unwrap();
        assert!(
            !draft.text.contains("window InventoryWin"),
            "{}",
            draft.text
        );
        let want = "{ tool = \"@window_click\", args = { target = \"Bag1\" }, label = \"click\" }";
        assert!(draft.text.contains(want), "{}", draft.text);
    }
}

#[test]
fn item_rows_without_one_named_stack_are_todos() {
    let move_to_3 = || cell_call("moveItem", 38, &[7001, 3, 0, 1]);
    // An equip that grew no stack in bag 3.
    let log = item_log(move_to_3(), vec![item(55, 1, 1)], vec![item(55, 1, 1)]);
    let draft = transform(&log, &opts()).unwrap();
    assert!(
        draft.text.contains("TODO moveItem 7001 to bag 3\n"),
        "{}",
        draft.text
    );
    assert!(!draft.text.contains("\"equip\""), "{}", draft.text);

    // Two differently named stacks grew there.
    let log = item_log(
        move_to_3(),
        vec![item(55, 1, 1), item(56, 1, 1)],
        vec![item(55, 3, 1), item(56, 3, 1)],
    );
    let draft = transform(&log, &opts()).unwrap();
    assert!(
        draft.text.contains("TODO moveItem 7001 to bag 3\n"),
        "{}",
        draft.text
    );
    assert!(!draft.text.contains("\"equip\""), "{}", draft.text);

    // A use that used nothing up.
    let use_item = |target| cell_call("useItem", 39, &[7001, target]);
    let log = item_log(use_item(0), vec![item(900, 1, 1)], vec![item(900, 1, 2)]);
    let draft = transform(&log, &opts()).unwrap();
    let want = "TODO useItem 7001: no single named carried stack fell, so the item has no name\n";
    assert!(draft.text.contains(want), "{}", draft.text);

    // A use on another entity keeps the action and notes the target.
    let fell = || (vec![item(900, 1, 2)], vec![item(900, 1, 1)]);
    let (b, a) = fell();
    let draft = transform(&item_log(use_item(999), b, a), &opts()).unwrap();
    let want = "label = \"use\" },\n  # TODO useItem target 999\n]\n";
    assert!(draft.text.contains(want), "{}", draft.text);
    assert_eq!(draft.rows, 1);
    // On the player itself there is nothing to note.
    let (b, a) = fell();
    let draft = transform(&item_log(use_item(8), b, a), &opts()).unwrap();
    assert!(!draft.text.contains("useItem target"), "{}", draft.text);
}

/// Recorded strings that try to start a row land inside comments: one row,
/// REC-01, and every line naming EVIL is a comment.
#[test]
fn recorded_text_cannot_inject_toml() {
    let evil = "x\n[[row]]\nid = \"EVIL\"";
    let mut log = fs_p3();
    tick_mut(&mut log, 33).tap.push(json!({
        "dir": "in", "msg_name": evil, "method_index": 150, "msg_id": 189, "args_hex": "00000000"
    }));
    for seq in 34..=41 {
        let windows = tick_mut(&mut log, seq).windows.as_mut().unwrap();
        windows.push("Win\r\n[[row]]\r\nid = \"EVIL\"".into());
    }
    log.start.instance = format!("p2{evil}");
    log.start.recording_id = format!("20261012-143000-p2{evil}");
    let draft = transform(&log, &opts()).unwrap();

    let spec = parsed(&draft);
    assert_eq!(spec.rows.len(), 1);
    assert_eq!(spec.rows[0].id, "REC-01");
    assert!(!draft.text.contains('\r'));
    let evil_lines: Vec<&str> = draft.text.lines().filter(|l| l.contains("EVIL")).collect();
    // The c2s TODO, the window TODO and the header line all kept it.
    assert!(evil_lines.len() >= 3, "{}", draft.text);
    for l in evil_lines {
        assert!(l.trim_start().starts_with('#'), "{l:?} in {}", draft.text);
    }
}
