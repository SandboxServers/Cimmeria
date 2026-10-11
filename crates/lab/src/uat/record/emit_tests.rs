//! LR-05 transform tests. The FS-P3 golden pins every rule the fixture
//! exercises; the others each change one fact of a log and check the one
//! line (or block) its rule writes.

use std::path::Path;

use serde_json::{json, Value};

use super::*;
use crate::uat::record::log::{read_log, DbSnapshot, SnapItem, Tick};
use crate::uat::seed_tags::{self, SeedTags};
use crate::uat::spec::{ActionSpec, SectionSpec};

const RECORD: &str = include_str!("fixtures/fs_p3_record.jsonl");
const DRAFT: &str = include_str!("fixtures/fs_p3_draft.toml");
const FIRST_SESSION: &str = include_str!("../../../../../docs/guides/uat-specs/first-session.toml");

fn fs_p3() -> RecordLog {
    read_log(RECORD).unwrap()
}

/// `[section]` copied from first-session, as `lab record stop` passes it.
fn opts() -> TransformOptions {
    TransformOptions {
        base: Some(spec::parse(FIRST_SESSION).unwrap().section),
        ..Default::default()
    }
}

fn parsed(d: &Draft) -> SectionSpec {
    spec::parse(&d.text).unwrap()
}

fn tick_mut(log: &mut RecordLog, seq: u32) -> &mut Tick {
    log.ticks.iter_mut().find(|t| t.seq == seq).unwrap()
}

/// An inbound cell call on the 0xBD sub-slot: the entity prefix, a sub-slot
/// byte, then `fields` as little-endian INT32s.
fn cell_call(name: &str, method: i32, fields: &[i32]) -> Value {
    let mut hex = String::from("000000000e");
    for f in fields {
        for b in f.to_le_bytes() {
            hex.push_str(&format!("{b:02x}"));
        }
    }
    json!({ "dir": "in", "msg_name": name, "method_index": method, "msg_id": 189, "args_hex": hex })
}

/// The steps of the draft's first row, after `@alias` resolution.
fn steps(d: &Draft) -> Vec<ActionSpec> {
    parsed(d).rows[0].steps.clone()
}

/// The whole transform, byte for byte: header, `[section]` from
/// first-session, the exact Frost pose, the stand/aim/click/wait/close steps
/// with the 2000 ms dialog wait (the dialog opens on the anchor tick), and
/// the three clauses with 3500 ms on the mission ones. Any rule change shows
/// as a diff of `fs_p3_draft.toml`; the actual text is written beside the
/// temp dir to diff against.
#[test]
fn fs_p3_transform_matches_the_committed_draft() {
    let draft = transform(&fs_p3(), &opts()).unwrap();
    // A checkout that converts line endings would otherwise fail here, not
    // in the transform.
    let want = DRAFT.replace("\r\n", "\n");
    if draft.text != want {
        let out = std::env::temp_dir().join("fs_p3_draft.actual.toml");
        std::fs::write(&out, &draft.text).unwrap();
        panic!("the FS-P3 draft changed; actual text in {}", out.display());
    }
    assert_eq!(draft.file_name, "first-session-20261012-1430.toml");
    assert_eq!(
        (draft.rows, draft.actions, draft.todos, draft.dropped_rows),
        (1, 5, 0, 0)
    );
    // The golden is a draft the drafts guard would accept.
    let seed = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../db/resources/Worlds/Seed");
    seed_tags::check(&parsed(&draft), &SeedTags::load(&seed).unwrap()).unwrap();
}

#[test]
fn the_transform_is_deterministic() {
    let log = fs_p3();
    let a = transform(&log, &opts()).unwrap();
    let b = transform(&log, &opts()).unwrap();
    assert_eq!(a, b);
}

#[test]
fn an_unknown_call_is_a_todo_line_and_the_draft_still_parses() {
    let mut log = fs_p3();
    tick_mut(&mut log, 33).tap.push(json!({
        "dir": "in", "msg_name": "gmSetGodMode", "method_index": 150, "args_hex": "00000000"
    }));
    let draft = transform(&log, &opts()).unwrap();
    assert!(
        draft
            .text
            .contains("\n  # TODO c2s gmSetGodMode (method 150)\n"),
        "{}",
        draft.text
    );
    assert_eq!((draft.rows, draft.todos), (1, 1));
    assert_eq!(parsed(&draft).rows.len(), 1);
}

#[test]
fn a_far_pose_comments_the_row_out() {
    let mut log = fs_p3();
    let target = log.ticks[32].entities[0].pos;
    for t in &mut log.ticks {
        if let Some(p) = &mut t.player {
            p.pos = [target[0] + 9.3, target[1], target[2]];
        }
    }
    let draft = transform(&log, &opts()).unwrap();
    assert!(
        draft
            .text
            .contains("\n# TODO REC-01 not runnable: the player stood 9.30 m from the target"),
        "{}",
        draft.text
    );
    assert!(draft.text.contains("\n# [[row]]\n"), "{}", draft.text);
    assert!(!draft.text.lines().any(|l| l.starts_with("[[row")));
    assert_eq!((draft.rows, draft.actions, draft.todos), (0, 0, 1));
    assert!(parsed(&draft).rows.is_empty());
}

#[test]
fn the_character_expression_replaces_the_name_in_sql() {
    let mut o = opts();
    o.character_expr = Some("Px${run_id}".into());
    let draft = transform(&fs_p3(), &o).unwrap();
    assert!(
        draft.text.contains("p.player_name = 'Px${run_id}'"),
        "{}",
        draft.text
    );
    assert!(!draft.text.contains("${character}"), "{}", draft.text);
    assert!(!draft.text.contains("Pxfixture"), "{}", draft.text);

    // Without one, the runner's own variable.
    let draft = transform(&fs_p3(), &opts()).unwrap();
    assert!(draft.text.contains("p.player_name = '${character}'"));
}

/// A dot command typed before the interact is the row's first setup action.
#[test]
fn a_dot_chat_line_becomes_a_tier_g_setup_chat() {
    let mut log = fs_p3();
    // channel 9, an empty target WSTRING, then ".gmsetgodmode 1".
    let text = ".gmsetgodmode 1";
    let mut hex = format!("09{}{:02x}000000", "00000000", text.len());
    for u in text.encode_utf16() {
        hex.push_str(&format!("{:02x}{:02x}", u & 0xff, u >> 8));
    }
    tick_mut(&mut log, 30).tap.push(json!({
        "dir": "in", "msg_name": "sendPlayerCommunication", "method_index": 2, "msg_id": 194, "args_hex": hex
    }));
    let draft = transform(&log, &opts()).unwrap();
    assert!(
        draft.text.contains(
            "setup = [\n  { chat = \".gmsetgodmode 1\", tier = \"G\" },\n  { tool = \"@stand\""
        ),
        "{}",
        draft.text
    );
    let setup = &parsed(&draft).rows[0].setup;
    assert_eq!(setup[0].chat.as_deref(), Some(text));
    assert_eq!(setup[0].tier, Some(crate::uat::tier::Tier::G));
}

/// A one-row log: a snapshot, the anchor call in tick 1, then `after`.
fn item_log(call: Value, before: Vec<SnapItem>, after: Vec<SnapItem>) -> RecordLog {
    let mut log = fs_p3();
    log.ticks.truncate(3);
    let snap = |items| DbSnapshot {
        items,
        ..log.ticks[0].snapshot.clone().unwrap()
    };
    let (b, a) = (snap(before), snap(after));
    for t in &mut log.ticks {
        t.tap.clear();
    }
    log.ticks[0].snapshot = Some(b.clone());
    log.ticks[1].snapshot = Some(b);
    log.ticks[1].tap.push(call);
    log.ticks[2].snapshot = Some(a);
    log
}

fn item(type_id: i32, container: i32, count: i32) -> SnapItem {
    SnapItem {
        type_id,
        container_id: container,
        count,
        name: Some(format!("Item {type_id}")),
    }
}

/// `useItem` is `@item_action` use, named from the carried stack it used up
/// (the call carries an instance id the snapshots do not hold).
#[test]
fn a_use_item_row_maps_to_item_action_use_with_the_item_name() {
    let log = item_log(
        cell_call("useItem", 39, &[7001, 0]),
        vec![item(900, 1, 2)],
        vec![item(900, 1, 1)],
    );
    let draft = transform(&log, &opts()).unwrap();
    let step = &steps(&draft)[0];
    assert_eq!(step.tool.as_deref(), Some("client_item_action"));
    assert_eq!(
        step.args,
        Some(json!({ "action": "use", "name": "Item 900" }))
    );
    assert!(draft.text.contains("title = \"Use Item 900\""));
}

/// Into bags 3 to 14 is an equip, named from the stack that grew there;
/// back to bag 1 an unequip; any other bag a TODO, which leaves the row no
/// step to run.
#[test]
fn move_item_into_an_equipment_bag_is_an_equip() {
    let bag1 = || vec![item(55, 1, 1)];
    let bag3 = || vec![item(55, 3, 1)];
    let log = item_log(cell_call("moveItem", 38, &[7001, 3, 0, 1]), bag1(), bag3());
    let step = &steps(&transform(&log, &opts()).unwrap())[0];
    assert_eq!(
        step.args,
        Some(json!({ "action": "equip", "name": "Item 55" }))
    );
    // The last equipment bag is still one.
    let log = item_log(
        cell_call("moveItem", 38, &[7001, 14, 0, 1]),
        bag1(),
        vec![item(55, 14, 1)],
    );
    let step = &steps(&transform(&log, &opts()).unwrap())[0];
    assert_eq!(step.args.as_ref().unwrap()["action"], "equip");

    let log = item_log(cell_call("moveItem", 38, &[7001, 1, 0, 1]), bag3(), bag1());
    let step = &steps(&transform(&log, &opts()).unwrap())[0];
    assert_eq!(
        step.args,
        Some(json!({ "action": "unequip", "name": "Item 55" }))
    );

    let log = item_log(
        cell_call("moveItem", 38, &[7001, 2, 0, 1]),
        bag1(),
        vec![item(55, 2, 1)],
    );
    let draft = transform(&log, &opts()).unwrap();
    assert!(
        draft.text.contains("#   # TODO moveItem 7001 to bag 2\n"),
        "{}",
        draft.text
    );
    assert_eq!(draft.rows, 0);
}

/// An `interact(0)` with nothing targeted has no target to pose on: the
/// row is a commented block.
#[test]
fn an_interact_with_no_target_is_a_todo_block() {
    let mut log = fs_p3();
    for row in &mut tick_mut(&mut log, 32).tap {
        if row["msg_name"] == "interact" {
            row["args_hex"] = json!("000000000d00000000");
        }
    }
    let draft = transform(&log, &opts()).unwrap();
    assert!(
        draft
            .text
            .contains("\n# TODO REC-01 not runnable: interact named no target"),
        "{}",
        draft.text
    );
    assert_eq!(draft.rows, 0);
    assert!(parsed(&draft).rows.is_empty());
}

/// Each recording fault is a header TODO line, and the draft still parses.
#[test]
fn recording_faults_are_header_todo_lines() {
    let mut log = fs_p3();
    log.stop = None;
    tick_mut(&mut log, 5).tap_dropped = 3;
    tick_mut(&mut log, 6).tap_dropped = 4;
    tick_mut(&mut log, 7).windows = None;
    let draft = transform(&log, &opts()).unwrap();
    for line in [
        "\n# TODO tap dropped 7 messages\n",
        "\n# TODO recording has no stop line\n",
        "\n# TODO 1 UI read failures\n",
    ] {
        assert!(draft.text.contains(line), "{line:?} in {}", draft.text);
    }
    assert_eq!(draft.todos, 3);
}

/// Rows anchored before `from_tick` are dropped and counted.
#[test]
fn rows_before_from_tick_are_dropped() {
    let mut o = opts();
    o.from_tick = Some(33);
    let draft = transform(&fs_p3(), &o).unwrap();
    assert_eq!((draft.rows, draft.dropped_rows), (0, 1));
    o.from_tick = Some(32);
    let draft = transform(&fs_p3(), &o).unwrap();
    assert_eq!((draft.rows, draft.dropped_rows), (1, 0));
}

/// Without a named section the recorder's own meta and the fallbacks are
/// written; a `fresh` base character is never copied.
#[test]
fn the_section_falls_back_without_a_base() {
    let o = TransformOptions {
        account: "non-gm".into(),
        character: "lab".into(),
        ..Default::default()
    };
    let s = parsed(&transform(&fs_p3(), &o).unwrap()).section;
    assert_eq!(s.id, "rec-first-session-20261012-1430");
    assert_eq!(
        (s.system.as_str(), s.account.as_str(), s.character.as_str()),
        ("recorded", "non-gm", "lab")
    );
    let mut o = opts();
    o.base.as_mut().unwrap().character = "fresh".into();
    let s = parsed(&transform(&fs_p3(), &o).unwrap()).section;
    assert_eq!(s.character, "lab");
}
