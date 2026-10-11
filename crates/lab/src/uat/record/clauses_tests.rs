use serde_json::{json, Value};

use super::*;
use crate::uat::record::decode::Call;
use crate::uat::record::log::{read_log, SnapItem, SnapMission, Tick};
use crate::uat::record::segment::{segment, Anchor, AnchorKind};

const FIXTURE: &str = include_str!("fixtures/fs_p3_record.jsonl");

fn mission(mission_id: i32, status: i32, step: Option<i32>) -> SnapMission {
    SnapMission {
        mission_id,
        status,
        step,
        repeats: 0,
    }
}

fn item(type_id: i32, container_id: i32, count: i32, name: Option<&str>) -> SnapItem {
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

fn tick(seq: u32, snapshot: Option<DbSnapshot>) -> Tick {
    Tick {
        seq,
        host_ms: 500 * i64::from(seq),
        snapshot,
        ..Tick::default()
    }
}

/// A log whose snapshot is `before` at tick 1, nothing at the anchor (tick
/// 2), and `after` at tick 3; the row covers ticks 2 to 4.
fn one_row(
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

fn ids(clauses: &[ClauseOut]) -> Vec<&str> {
    clauses.iter().map(|c| c.id.as_str()).collect()
}

fn field<'a>(c: &'a ClauseOut, name: &str) -> &'a Inline {
    &c.fields.iter().find(|(k, _)| *k == name).unwrap().1
}

fn dialog(seq: u32, d: i64) -> (u32, String, Value) {
    (
        seq,
        "onDialogDisplay".into(),
        json!({ "dialog_id": d, "entity_id": 9 }),
    )
}

/// FS-P3's one row: the dialog the server opened, mission 622's step and
/// the new mission 1360, the server clauses at 3500 ms (settled 500 ms after
/// the anchor). Fails if the start snapshot is taken after the anchor's
/// tick: tick 33 already holds the new state, so nothing would differ.
#[test]
fn fs_p3_row_gives_a_dialog_clause_and_two_mission_clauses() {
    let log = read_log(FIXTURE).unwrap();
    let rows = segment(&log);
    assert_eq!(rows.len(), 1);
    let row = &rows[0];
    let clauses = row_clauses(&log, row, "${character}");
    assert_eq!(
        ids(&clauses),
        ["dialog-3995", "mission-622-step", "mission-1360-status"]
    );

    let dialog = &clauses[0];
    assert_eq!(dialog.text, "onDialogDisplay 3995 to the player");
    assert_eq!(field(dialog, "source"), &Inline::Str("packet".into()));
    assert_eq!(
        field(dialog, "match_fields").render(),
        "{ dialog_id = 3995 }"
    );

    let step = &clauses[1];
    assert_eq!(step.text, "sgw_mission: 622 on step 80623");
    assert_eq!(field(step, "value"), &Inline::Int(80623));
    assert_eq!(
        field(step, "args").render(),
        "{ sql = \"SELECT m.current_step_id FROM sgw_mission m JOIN sgw_player p ON p.player_id = m.player_id WHERE p.player_name = '${character}' AND m.mission_id = 622\" }"
    );
    let status = &clauses[2];
    assert_eq!(status.text, "sgw_mission: 1360 status 1");
    assert_eq!(field(status, "value"), &Inline::Int(1));
    for c in &clauses[1..] {
        assert_eq!(field(c, "timeout_ms"), &Inline::Int(3500), "{}", c.id);
        assert_eq!(field(c, "op"), &Inline::Str("eq".into()));
    }
    let keys: Vec<&str> = step.fields.iter().map(|(k, _)| *k).collect();
    assert_eq!(
        keys,
        [
            "source",
            "tool",
            "args",
            "pointer",
            "op",
            "value",
            "timeout_ms"
        ]
    );
}

/// The start snapshot is the last strictly before the anchor's tick. With
/// the effect already in tick 32's snapshot (the anchor's own tick), the
/// clauses still come out, settled at 0 ms. Fails if the start snapshot is
/// taken at the anchor's tick.
#[test]
fn the_start_snapshot_is_strictly_before_the_anchor_tick() {
    let mut log = read_log(FIXTURE).unwrap();
    log.ticks[32].snapshot = log.ticks[33].snapshot.clone();
    let rows = segment(&log);
    let clauses = row_clauses(&log, &rows[0], "${character}");
    assert_eq!(
        ids(&clauses),
        ["dialog-3995", "mission-622-step", "mission-1360-status"]
    );
    assert_eq!(field(&clauses[1], "timeout_ms"), &Inline::Int(3000));
}

/// D-LR7: a floor, a ceiling, doubling, and rounding up to 250 ms.
/// Deviation: the packet's `dialog_timeout_ms(1) == 2000` contradicts its
/// own "rounded up" rule (2001 rounds up to 2250), so 0 and 1 are pinned.
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
        "{ sql = \"SELECT count(*)::int AS n FROM sgw_inventory i JOIN sgw_player p ON p.player_id = i.character_id WHERE p.player_name = 'Px${run_id}' AND i.type_id = 55 AND i.container_id = 3\" }"
    );
    assert_eq!(
        field(&clauses[1], "pointer"),
        &Inline::Str("/rows/0/n".into())
    );
}

/// Dialogs, then missions by id, then items by type and container, then
/// the player's level, exp, naquadah and world, whatever order the
/// snapshots list them in.
#[test]
fn clauses_come_in_contract_order_across_kinds() {
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
    let (log, row) = one_row(before, after, vec![dialog(2, 7)]);
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
    let level = &clauses[6];
    assert_eq!(level.text, "sgw_player: level 2");
    assert_eq!(field(level, "value"), &Inline::Int(2));
    assert_eq!(
        field(level, "args").render(),
        "{ sql = \"SELECT p.level FROM sgw_player p WHERE p.player_name = '${character}'\" }"
    );
    let world = &clauses[9];
    assert_eq!(field(world, "value"), &Inline::Str("Castle".into()));
    assert_eq!(
        field(world, "pointer"),
        &Inline::Str("/rows/0/world_location".into())
    );
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

/// The settle time is from the anchor's tick to the first snapshot that
/// shows the diff's end state, not merely the first that differs.
#[test]
fn settle_time_waits_for_the_diffs_end_state() {
    let (mut log, row) = one_row(
        DbSnapshot {
            missions: vec![mission(1, 1, Some(10))],
            ..DbSnapshot::default()
        },
        DbSnapshot::default(),
        Vec::new(),
    );
    let state = |step| DbSnapshot {
        missions: vec![mission(1, 1, Some(step))],
        ..DbSnapshot::default()
    };
    log.ticks[3].snapshot = Some(state(11));
    log.ticks[4].snapshot = Some(state(12));
    let d = Diff::MissionStep {
        mission: 1,
        from: Some(10),
        to: Some(12),
    };
    assert_eq!(settle_ms(&log, &row, &d), 1000);
    let clauses = row_clauses(&log, &row, "${character}");
    assert_eq!(field(&clauses[0], "value"), &Inline::Int(12));
    assert_eq!(field(&clauses[0], "timeout_ms"), &Inline::Int(4000));
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
/// empty tables are `{}`, arrays are `[a, b]`.
#[test]
fn inline_render_shapes() {
    assert_eq!(Inline::Float(4.156, 2).render(), "4.16");
    assert_eq!(Inline::Float(250.0, 0).render(), "250.0");
    assert_eq!(Inline::Table(Vec::new()).render(), "{}");
    assert_eq!(
        Inline::Array(vec![Inline::Int(1), Inline::Bool(true)]).render(),
        "[1, true]"
    );
    let doc = format!(
        "a = {}\nb = {}\n",
        Inline::Float(250.0, 0).render(),
        Inline::Float(f64::NEG_INFINITY, 1).render()
    );
    let parsed: toml::Table = toml::from_str(&doc).unwrap();
    assert!(parsed["a"].is_float());
    assert_eq!(parsed["b"].as_float(), Some(f64::NEG_INFINITY));
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
