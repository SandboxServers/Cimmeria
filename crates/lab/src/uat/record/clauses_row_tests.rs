//! Row boundaries, settle times and character substitution for
//! [`row_clauses`]; the shared fixtures live in `clauses_tests.rs`.

use serde_json::{json, Value};

use super::tests::{contract_order_row, dialog, field, ids, item, mission, one_row, tick, FIXTURE};
use super::*;
use crate::uat::record::log::{read_log, Tick};
use crate::uat::record::segment::segment;

/// An inbound `interact(target)` as the tap writes it: method 74 rides 0xBD
/// with sub-slot 13 after the entity id.
fn interact(target: i32) -> Value {
    let mut args = vec![0u8, 0, 0, 0, 74 - 61];
    args.extend(target.to_le_bytes());
    let hex: String = args.iter().map(|b| format!("{b:02x}")).collect();
    json!({"dir": "in", "msg_name": "interact", "method_index": 74, "msg_id": 0xBD, "args_hex": hex})
}

fn tick_with(seq: u32, tap: Vec<Value>, snapshot: Option<DbSnapshot>) -> Tick {
    Tick {
        tap,
        ..tick(seq, snapshot)
    }
}

fn missions(ids: &[i32]) -> DbSnapshot {
    DbSnapshot {
        missions: ids.iter().map(|&m| mission(m, 1, None)).collect(),
        ..DbSnapshot::default()
    }
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
    let rendered: Vec<(&str, String)> = dialog
        .fields
        .iter()
        .map(|(k, v)| (*k, v.render()))
        .collect();
    assert_eq!(
        rendered,
        [
            ("source", "\"packet\"".to_string()),
            ("message", "\"onDialogDisplay\"".to_string()),
            ("direction", "\"to_client\"".to_string()),
            ("match_fields", "{ dialog_id = 3995 }".to_string()),
            ("min_rows", "1".to_string()),
        ]
    );

    let step = &clauses[1];
    assert_eq!(step.text, "sgw_mission: 622 on step 80623");
    assert_eq!(field(step, "value"), &Inline::Int(80623));
    assert_eq!(
        field(step, "pointer"),
        &Inline::Str("/rows/0/current_step_id".into())
    );
    assert_eq!(
        field(step, "args").render(),
        "{ sql = \"SELECT m.current_step_id FROM sgw_mission m JOIN sgw_player p ON p.player_id = m.player_id WHERE p.player_name = '${character}' AND m.mission_id = 622\" }"
    );
    let status = &clauses[2];
    assert_eq!(status.text, "sgw_mission: 1360 status 1");
    assert_eq!(field(status, "value"), &Inline::Int(1));
    assert_eq!(
        field(status, "pointer"),
        &Inline::Str("/rows/0/status".into())
    );
    assert_eq!(
        field(status, "args").render(),
        "{ sql = \"SELECT m.status FROM sgw_mission m JOIN sgw_player p ON p.player_id = m.player_id WHERE p.player_name = '${character}' AND m.mission_id = 1360\" }"
    );
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

/// Each row's end snapshot is the last at or before its own `last_tick`:
/// row 0 does not see the mission row 1 added.
#[test]
fn the_end_snapshot_is_bounded_by_the_rows_last_tick() {
    let log = RecordLog {
        start: read_log(FIXTURE).unwrap().start,
        ticks: vec![
            tick(0, Some(missions(&[]))),
            tick_with(1, vec![interact(100)], None),
            tick(2, Some(missions(&[10]))),
            tick_with(3, vec![interact(200)], None),
            tick(4, Some(missions(&[10, 20]))),
        ],
        stop: None,
    };
    let rows = segment(&log);
    assert_eq!(rows.len(), 2, "{rows:#?}");
    assert_eq!(
        ids(&row_clauses(&log, &rows[0], "${character}")),
        ["mission-10-status"]
    );
    assert_eq!(
        ids(&row_clauses(&log, &rows[1], "${character}")),
        ["mission-20-status"]
    );
}

/// A snapshot after `last_tick` belongs to the next row, not this one.
#[test]
fn a_snapshot_after_the_last_tick_is_ignored() {
    let (mut log, row) = one_row(DbSnapshot::default(), DbSnapshot::default(), Vec::new());
    log.ticks.push(tick(5, Some(missions(&[99]))));
    let clauses = row_clauses(&log, &row, "${character}");
    assert!(!ids(&clauses).contains(&"mission-99-status"), "{clauses:?}");
    assert!(clauses.is_empty());
}

/// With no snapshot before the anchor there is nothing to diff, so only
/// the dialog clauses come out.
#[test]
fn no_snapshot_before_the_anchor_gives_only_dialog_clauses() {
    let (mut log, row) = one_row(DbSnapshot::default(), missions(&[10]), vec![dialog(2, 7)]);
    log.ticks[1].snapshot = None;
    assert_eq!(ids(&row_clauses(&log, &row, "${character}")), ["dialog-7"]);
}

/// `character_sql` goes into every server clause's SQL as given.
#[test]
fn character_sql_is_substituted_in_every_server_clause() {
    let (log, row) = contract_order_row();
    let clauses = row_clauses(&log, &row, "Px${run_id}");
    let server: Vec<&ClauseOut> = clauses
        .iter()
        .filter(|c| field(c, "source") == &Inline::Str("server".into()))
        .collect();
    assert_eq!(server.len(), 9);
    for c in server {
        let args = field(c, "args").render();
        assert!(
            args.contains("player_name = 'Px${run_id}'"),
            "{}: {args}",
            c.id
        );
        assert!(!args.contains("'${character}'"), "{}: {args}", c.id);
    }
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

/// Each diff settles on its own: the mission shows at tick 3 (500 ms after
/// the anchor), the item only at tick 4 (1000 ms).
#[test]
fn each_diff_settles_on_its_own_tick() {
    let (mut log, row) = one_row(DbSnapshot::default(), DbSnapshot::default(), Vec::new());
    log.ticks[3].snapshot = Some(missions(&[10]));
    log.ticks[4].snapshot = Some(DbSnapshot {
        items: vec![item(55, 1, 1, None)],
        ..missions(&[10])
    });
    let clauses = row_clauses(&log, &row, "${character}");
    assert_eq!(ids(&clauses), ["mission-10-status", "item-55-1"]);
    assert_eq!(field(&clauses[0], "timeout_ms"), &Inline::Int(3500));
    assert_eq!(field(&clauses[1], "timeout_ms"), &Inline::Int(4000));
}

/// A diff no snapshot in the row shows gets the row's whole span: tick 2
/// (1000 ms) to tick 4 (2000 ms).
#[test]
fn a_diff_the_row_never_shows_settles_at_the_whole_span() {
    let (log, row) = one_row(DbSnapshot::default(), DbSnapshot::default(), Vec::new());
    let d = Diff::MissionStatus {
        mission: 999,
        from: None,
        to: Some(5),
    };
    assert_eq!(settle_ms(&log, &row, &d), 1000);
}

/// `host_ms` comes from the JSONL: extreme values saturate, and a clock
/// that runs backwards settles at 0.
#[test]
fn hostile_host_times_saturate() {
    let d = Diff::MissionStatus {
        mission: 999,
        from: None,
        to: Some(5),
    };
    let (mut log, row) = one_row(DbSnapshot::default(), DbSnapshot::default(), Vec::new());
    log.ticks[2].host_ms = i64::MIN;
    log.ticks[4].host_ms = i64::MAX;
    assert_eq!(settle_ms(&log, &row, &d), i64::MAX);
    assert_eq!(clause_timeout_ms(settle_ms(&log, &row, &d)), 60_000);

    log.ticks[2].host_ms = i64::MAX;
    log.ticks[4].host_ms = i64::MIN;
    assert_eq!(settle_ms(&log, &row, &d), 0);
}
