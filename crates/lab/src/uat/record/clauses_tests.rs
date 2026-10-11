use serde_json::{json, Value};

use super::*;
use crate::uat::record::decode::Call;
use crate::uat::record::log::{read_log, SnapItem, SnapMission, Tick};
use crate::uat::record::segment::{Anchor, AnchorKind};

pub(super) const FIXTURE: &str = include_str!("fixtures/fs_p3_record.jsonl");

pub(super) fn mission(mission_id: i32, status: i32, step: Option<i32>) -> SnapMission {
    SnapMission {
        mission_id,
        status,
        step,
        repeats: 0,
    }
}

pub(super) fn item(type_id: i32, container_id: i32, count: i32, name: Option<&str>) -> SnapItem {
    SnapItem {
        type_id,
        container_id,
        count,
        name: name.map(str::to_string),
    }
}

fn player(level: i32, exp: i64, naquadah: i64, world: &str) -> SnapPlayer {
    SnapPlayer {
        level,
        exp,
        naquadah,
        world_location: world.into(),
    }
}

/// A tick `500 * seq` host ms in.
pub(super) fn tick(seq: u32, snapshot: Option<DbSnapshot>) -> Tick {
    Tick {
        seq,
        host_ms: 500 * i64::from(seq),
        snapshot,
        ..Tick::default()
    }
}

/// A log whose snapshot is `before` at tick 1, nothing at the anchor (tick
/// 2), and `after` at tick 3; the row covers ticks 2 to 4.
pub(super) fn one_row(
    before: DbSnapshot,
    after: DbSnapshot,
    server: Vec<(u32, String, Value)>,
) -> (RecordLog, RecordedRow) {
    let log = RecordLog {
        start: read_log(FIXTURE).unwrap().start,
        ticks: vec![
            tick(0, None),
            tick(1, Some(before)),
            tick(2, None),
            tick(3, Some(after)),
            tick(4, None),
        ],
        stop: None,
    };
    let row = RecordedRow {
        anchor: Anchor {
            kind: AnchorKind::Interact,
            tick: 2,
            call: Call::Interact { target: 7 },
            target: Some(7),
        },
        first_tick: 2,
        last_tick: 4,
        calls: Vec::new(),
        setup_chat: Vec::new(),
        server,
    };
    (log, row)
}

pub(super) fn ids(clauses: &[ClauseOut]) -> Vec<&str> {
    clauses.iter().map(|c| c.id.as_str()).collect()
}

pub(super) fn field<'a>(c: &'a ClauseOut, name: &str) -> &'a Inline {
    &c.fields.iter().find(|(k, _)| *k == name).unwrap().1
}

pub(super) fn dialog(seq: u32, d: i64) -> (u32, String, Value) {
    (
        seq,
        "onDialogDisplay".into(),
        json!({ "dialog_id": d, "entity_id": 9 }),
    )
}

/// One row with a dialog, two missions, three item stacks and all four
/// player fields changed, each listed out of contract order.
pub(super) fn contract_order_row() -> (RecordLog, RecordedRow) {
    let before = DbSnapshot {
        missions: vec![mission(9, 1, Some(1)), mission(5, 1, None)],
        player: Some(player(1, 0, 10, "Castle_CellBlock")),
        items: vec![item(55, 3, 1, None), item(3, 1, 1, None)],
    };
    let after = DbSnapshot {
        missions: vec![mission(9, 1, Some(2)), mission(5, 2, None)],
        player: Some(player(2, 150, 25, "Castle")),
        items: vec![item(55, 1, 1, None), item(3, 1, 4, None)],
    };
    one_row(before, after, vec![dialog(2, 7)])
}

/// D-LR7: a floor, a ceiling, doubling, and rounding up to 250 ms.
#[test]
fn timeouts_have_floors_ceilings_and_quarter_seconds() {
    assert_eq!(clause_timeout_ms(0), 3000);
    assert_eq!(clause_timeout_ms(1600), 4750);
    assert_eq!(clause_timeout_ms(40_000), 60_000);
    assert_eq!(clause_timeout_ms(-5), 3000);
    assert_eq!(dialog_timeout_ms(0), 2000);
    assert_eq!(dialog_timeout_ms(1), 2250);
    assert_eq!(dialog_timeout_ms(20_000), 30_000);
    assert_eq!(server_wait_timeout_ms(17_000), 34_000);
    assert_eq!(server_wait_timeout_ms(0), 5000);
    assert_eq!(server_wait_timeout_ms(100_000), 120_000);
}

/// A span from a hostile log clamps instead of overflowing: `2t` and
/// `t + floor` both overflow at `i64::MAX`.
#[test]
fn extreme_spans_clamp_without_overflow() {
    assert_eq!(clause_timeout_ms(i64::MAX), 60_000);
    assert_eq!(dialog_timeout_ms(i64::MAX), 30_000);
    assert_eq!(server_wait_timeout_ms(i64::MAX), 120_000);
    assert_eq!(clause_timeout_ms(i64::MAX - 1000), 60_000);
    assert_eq!(clause_timeout_ms(i64::MIN), 3000);
    assert_eq!(dialog_timeout_ms(i64::MIN), 2000);
    assert_eq!(server_wait_timeout_ms(i64::MIN), 5000);
}

/// A mission that disappeared is graded by its row being absent.
#[test]
fn a_removed_mission_is_an_absent_clause() {
    let before = DbSnapshot {
        missions: vec![mission(622, 1, Some(80623))],
        ..DbSnapshot::default()
    };
    let (log, row) = one_row(before, DbSnapshot::default(), Vec::new());
    let clauses = row_clauses(&log, &row, "${character}");
    assert_eq!(ids(&clauses), ["mission-622-status"]);
    let c = &clauses[0];
    assert_eq!(field(c, "pointer"), &Inline::Str("/rows/0".into()));
    assert_eq!(field(c, "op"), &Inline::Str("absent".into()));
    assert!(c.fields.iter().all(|(k, _)| *k != "value"));
    assert_eq!(c.text, "sgw_mission: 622 has no row");
}

/// A step cleared to NULL is graded by the step being absent, on the step
/// pointer.
#[test]
fn a_step_going_to_none_is_an_absent_step_clause() {
    let before = DbSnapshot {
        missions: vec![mission(1, 1, Some(10))],
        ..DbSnapshot::default()
    };
    let after = DbSnapshot {
        missions: vec![mission(1, 1, None)],
        ..DbSnapshot::default()
    };
    let (log, row) = one_row(before, after, Vec::new());
    let clauses = row_clauses(&log, &row, "${character}");
    assert_eq!(ids(&clauses), ["mission-1-step"]);
    let c = &clauses[0];
    assert_eq!(c.text, "sgw_mission: 1 on no step");
    assert_eq!(field(c, "op"), &Inline::Str("absent".into()));
    assert_eq!(
        field(c, "pointer"),
        &Inline::Str("/rows/0/current_step_id".into())
    );
    assert!(c.fields.iter().all(|(k, _)| *k != "value"));
}

/// A new mission is one status clause even when it has a step; a status
/// change with a step change is still only the status; the same status on
/// a new step is a step clause.
#[test]
fn a_new_mission_is_a_status_clause_and_a_changed_step_a_step_clause() {
    let before = DbSnapshot {
        missions: vec![mission(10, 1, Some(100)), mission(20, 1, Some(200))],
        ..DbSnapshot::default()
    };
    let after = DbSnapshot {
        missions: vec![
            mission(10, 2, Some(101)),
            mission(20, 1, Some(201)),
            mission(30, 1, Some(300)),
        ],
        ..DbSnapshot::default()
    };
    assert_eq!(
        diff_snapshots(&before, &after),
        [
            Diff::MissionStatus {
                mission: 10,
                from: Some(1),
                to: Some(2)
            },
            Diff::MissionStep {
                mission: 20,
                from: Some(200),
                to: Some(201)
            },
            Diff::MissionStatus {
                mission: 30,
                from: None,
                to: Some(1)
            },
        ]
    );
    let (log, row) = one_row(before, after, Vec::new());
    let clauses = row_clauses(&log, &row, "${character}");
    assert_eq!(
        ids(&clauses),
        ["mission-10-status", "mission-20-step", "mission-30-status"]
    );
    assert_eq!(field(&clauses[2], "value"), &Inline::Int(1));
    assert_eq!(
        field(&clauses[2], "pointer"),
        &Inline::Str("/rows/0/status".into())
    );
}

/// One clause per changed carried stack, keyed by type and container; the
/// bank (17) and an unchanged stack make none; a stack that went away
/// counts 0.
#[test]
fn an_item_count_change_is_one_clause_per_container() {
    let before = DbSnapshot {
        items: vec![
            item(55, 1, 1, Some("SI 3 9mm Pistol")),
            item(60, 1, 2, None),
            item(70, 17, 1, None),
            item(80, 2, 1, None),
        ],
        ..DbSnapshot::default()
    };
    let after = DbSnapshot {
        items: vec![
            item(55, 1, 2, Some("SI 3 9mm Pistol")),
            item(55, 3, 1, Some("SI 3 9mm Pistol")),
            item(60, 1, 2, None),
            item(70, 17, 3, None),
        ],
        ..DbSnapshot::default()
    };
    let (log, row) = one_row(before, after, Vec::new());
    let clauses = row_clauses(&log, &row, "Px${run_id}");
    assert_eq!(ids(&clauses), ["item-55-1", "item-55-3", "item-80-2"]);
    assert_eq!(clauses[0].text, "carried SI 3 9mm Pistol (55) in bag 1: 2");
    assert_eq!(clauses[1].text, "carried SI 3 9mm Pistol (55) in bag 3: 1");
    assert_eq!(clauses[2].text, "carried item 80 in bag 2: 0");
    assert_eq!(field(&clauses[2], "value"), &Inline::Int(0));
    assert_eq!(
        field(&clauses[1], "args").render(),
        "{ sql = \"SELECT coalesce(sum(coalesce(i.stack_size,1)),0)::int AS n FROM sgw_inventory i JOIN sgw_player p ON p.player_id = i.character_id WHERE p.player_name = 'Px${run_id}' AND i.type_id = 55 AND i.container_id = 3\" }"
    );
    assert_eq!(
        field(&clauses[1], "pointer"),
        &Inline::Str("/rows/0/n".into())
    );
}

/// Counts are units: one of a stack of 5 used is 5 to 4 in the same one
/// row, a diff and a clause graded on the summed `stack_size` (a row count
/// would read 1 both times and see nothing).
#[test]
fn one_unit_used_from_a_stack_of_5_is_an_item_diff_and_clause() {
    let before = DbSnapshot {
        items: vec![item(66, 1, 5, Some("Med Kit"))],
        ..DbSnapshot::default()
    };
    let after = DbSnapshot {
        items: vec![item(66, 1, 4, Some("Med Kit"))],
        ..DbSnapshot::default()
    };
    assert_eq!(
        diff_snapshots(&before, &after),
        [Diff::ItemCount {
            type_id: 66,
            container: 1,
            from: 5,
            to: 4,
            name: Some("Med Kit".into()),
        }]
    );
    let (log, row) = one_row(before, after, Vec::new());
    let clauses = row_clauses(&log, &row, "${character}");
    assert_eq!(ids(&clauses), ["item-66-1"]);
    assert_eq!(clauses[0].text, "carried Med Kit (66) in bag 1: 4");
    assert_eq!(field(&clauses[0], "value"), &Inline::Int(4));
    let Inline::Table(args) = field(&clauses[0], "args") else {
        panic!("args is not a table");
    };
    let Inline::Str(sql) = &args[0].1 else {
        panic!("sql is not a string");
    };
    assert!(
        sql.starts_with("SELECT coalesce(sum(coalesce(i.stack_size,1)),0)::int AS n "),
        "{sql}"
    );
}

/// A stack that went away keeps its name, taken from the before snapshot.
#[test]
fn a_removed_stack_takes_its_name_from_the_before_snapshot() {
    let before = DbSnapshot {
        items: vec![item(55, 1, 1, Some("SI 3 9mm Pistol"))],
        ..DbSnapshot::default()
    };
    let (log, row) = one_row(before, DbSnapshot::default(), Vec::new());
    let clauses = row_clauses(&log, &row, "${character}");
    assert_eq!(ids(&clauses), ["item-55-1"]);
    assert_eq!(clauses[0].text, "carried SI 3 9mm Pistol (55) in bag 1: 0");
}

/// The player is compared only when both snapshots have its row; a row in
/// only one makes no `player-*` clause.
#[test]
fn a_player_row_in_only_one_snapshot_makes_no_player_clause() {
    let after = DbSnapshot {
        player: Some(player(2, 150, 25, "Castle")),
        ..DbSnapshot::default()
    };
    let (log, row) = one_row(DbSnapshot::default(), after.clone(), Vec::new());
    assert!(row_clauses(&log, &row, "${character}").is_empty());
    let (log, row) = one_row(after, DbSnapshot::default(), Vec::new());
    assert!(row_clauses(&log, &row, "${character}").is_empty());
}

/// Dialogs, then missions by id, then items by type and container, then
/// the player's level, exp, naquadah and world, whatever order the
/// snapshots list them in. The player clauses read their own column.
#[test]
fn clauses_come_in_contract_order_across_kinds() {
    let (log, row) = contract_order_row();
    let clauses = row_clauses(&log, &row, "${character}");
    assert_eq!(
        ids(&clauses),
        [
            "dialog-7",
            "mission-5-status",
            "mission-9-step",
            "item-3-1",
            "item-55-1",
            "item-55-3",
            "player-level",
            "player-exp",
            "player-naquadah",
            "player-world",
        ]
    );
    assert_eq!(clauses[6].text, "sgw_player: level 2");
    let players = [
        ("level", "level", Inline::Int(2)),
        ("exp", "exp", Inline::Int(150)),
        ("naquadah", "naquadah", Inline::Int(25)),
        ("world", "world_location", Inline::Str("Castle".into())),
    ];
    for (c, (name, column, value)) in clauses[6..].iter().zip(players) {
        assert_eq!(c.id, format!("player-{name}"));
        assert_eq!(field(c, "value"), &value, "{}", c.id);
        assert_eq!(
            field(c, "pointer"),
            &Inline::Str(format!("/rows/0/{column}")),
            "{}",
            c.id
        );
        assert_eq!(
            field(c, "args").render(),
            format!(
                "{{ sql = \"SELECT p.{column} FROM sgw_player p WHERE p.player_name = '${{character}}'\" }}"
            ),
            "{}",
            c.id
        );
    }
}

/// Each distinct dialog in the row gets its own clause, in the order sent;
/// a repeat of one does not add a second. Other messages make none.
#[test]
fn two_distinct_dialogs_in_one_row_each_get_a_clause() {
    let server = vec![
        dialog(2, 10),
        (
            2,
            "onStepUpdate".into(),
            json!({ "StepID": 1, "Status": 0 }),
        ),
        dialog(3, 11),
        dialog(4, 10),
    ];
    let (log, row) = one_row(DbSnapshot::default(), DbSnapshot::default(), server);
    let clauses = row_clauses(&log, &row, "${character}");
    assert_eq!(ids(&clauses), ["dialog-10", "dialog-11"]);
    assert_eq!(
        field(&clauses[1], "match_fields"),
        &Inline::Table(vec![("dialog_id".into(), Inline::Int(11))])
    );
}

/// Dialogs keep the order they were shown in, not their id order.
#[test]
fn dialogs_come_in_the_order_shown_not_sorted() {
    let server = vec![dialog(2, 11), dialog(3, 10)];
    let (log, row) = one_row(DbSnapshot::default(), DbSnapshot::default(), server);
    let clauses = row_clauses(&log, &row, "${character}");
    assert_eq!(ids(&clauses), ["dialog-11", "dialog-10"]);
}

/// Only `onDialogDisplay` makes a dialog clause, even when another message
/// carries a `dialog_id`.
#[test]
fn only_on_dialog_display_makes_a_dialog_clause() {
    let server = vec![(2, "onSomethingElse".into(), json!({ "dialog_id": 42 }))];
    let (log, row) = one_row(DbSnapshot::default(), DbSnapshot::default(), server);
    let clauses = row_clauses(&log, &row, "${character}");
    assert!(!ids(&clauses).contains(&"dialog-42"), "{clauses:?}");
    assert!(clauses.is_empty());
}

/// A string with a quote and a backslash renders as TOML that parses back
/// to the same text, inside a table and an array too.
#[test]
fn inline_strings_are_escaped() {
    let tricky = "a \"quoted\" C:\\path\nline";
    let value = Inline::Table(vec![
        ("sql".into(), Inline::Str(tricky.into())),
        (
            "odd key".into(),
            Inline::Array(vec![Inline::Str(tricky.into())]),
        ),
    ]);
    let doc = format!("x = {}\n", value.render());
    let parsed: toml::Table = toml::from_str(&doc).unwrap();
    assert_eq!(parsed["x"]["sql"].as_str(), Some(tricky));
    assert_eq!(parsed["x"]["odd key"][0].as_str(), Some(tricky));
}

/// Floats keep their fixed decimals (at least one, so they stay floats),
/// NaN and infinities are TOML's words, empty tables are `{}`, arrays are
/// `[a, b]`.
#[test]
fn inline_render_shapes() {
    assert_eq!(Inline::Float(4.156, 2).render(), "4.16");
    assert_eq!(Inline::Float(250.0, 0).render(), "250.0");
    assert_eq!(Inline::Float(f64::NAN, 2).render(), "nan");
    assert_eq!(Inline::Table(Vec::new()).render(), "{}");
    assert_eq!(
        Inline::Array(vec![Inline::Int(1), Inline::Bool(true)]).render(),
        "[1, true]"
    );
    let doc = format!(
        "a = {}\nb = {}\nc = {}\n",
        Inline::Float(250.0, 0).render(),
        Inline::Float(f64::NEG_INFINITY, 1).render(),
        Inline::Float(f64::NAN, 1).render()
    );
    let parsed: toml::Table = toml::from_str(&doc).unwrap();
    assert!(parsed["a"].is_float());
    assert_eq!(parsed["b"].as_float(), Some(f64::NEG_INFINITY));
    assert!(parsed["c"].as_float().is_some_and(f64::is_nan));
}

/// Only the main, mission and equipment bags are carried.
#[test]
fn carried_bags_are_1_to_14() {
    assert!(!is_carried(0));
    assert!(is_carried(INV_MAIN));
    assert!(is_carried(INV_MISSION));
    assert!(is_carried(INV_BANDOLIER));
    assert!(is_carried(INV_ARTIFACT2));
    assert!(!is_carried(15));
    assert!(!is_carried(17));
}
