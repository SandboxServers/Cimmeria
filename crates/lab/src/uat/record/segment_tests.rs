use serde_json::{json, Value};

use super::*;
use crate::uat::record::log::{read_log, SnapMission, Tick, UiClick};

const FIXTURE: &str = include_str!("fixtures/fs_p3_record.jsonl");

/// A log of `ticks` under the fixture's start line.
fn log_of(ticks: Vec<Tick>) -> RecordLog {
    RecordLog {
        start: read_log(FIXTURE).unwrap().start,
        ticks,
        stop: None,
    }
}

fn tick(seq: u32, tap: Vec<Value>, snapshot: Option<DbSnapshot>) -> Tick {
    Tick {
        seq,
        host_ms: 500 * i64::from(seq),
        tap,
        snapshot,
        ..Tick::default()
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// An inbound cell call as the tap writes it: methods from 61 ride 0xBD
/// with a sub-slot byte after the entity id, the rest 0x80 + index.
fn cell(name: &str, method: u8, fields: &[i32], tail: &[u8]) -> Value {
    let (msg_id, mut args) = if method >= 61 {
        (0xBD, vec![0, 0, 0, 0, method - 61])
    } else {
        (0x80 + method, vec![0, 0, 0, 0])
    };
    for f in fields {
        args.extend(f.to_le_bytes());
    }
    args.extend(tail);
    json!({"dir": "in", "msg_name": name, "method_index": method, "msg_id": msg_id, "args_hex": hex(&args)})
}

fn chat(text: &str) -> Value {
    let mut args = vec![0, 0, 0, 0, 0];
    let units: Vec<u16> = text.encode_utf16().collect();
    args.extend((units.len() as u32).to_le_bytes());
    for u in units {
        args.extend(u.to_le_bytes());
    }
    json!({"dir": "in", "msg_name": "sendPlayerCommunication", "method_index": 2, "msg_id": 0xC2, "args_hex": hex(&args)})
}

fn region(entering: bool) -> Value {
    cell(
        "triggerClientHintedGenericRegion",
        85,
        &[14],
        &[u8::from(entering)],
    )
}

fn changed() -> DbSnapshot {
    DbSnapshot {
        missions: vec![SnapMission {
            mission_id: 622,
            status: 1,
            step: Some(80623),
            repeats: 0,
        }],
        ..DbSnapshot::default()
    }
}

/// The recording holds a teleport and a region call before Frost (the
/// region at tick 1), noise throughout, and the close at tick 36. Exactly
/// one row, so neither the teleport nor the region anchored one.
#[test]
fn fs_p3_is_one_interact_row_on_frost_with_a_close() {
    let rows = segment(&read_log(FIXTURE).unwrap());
    assert_eq!(rows.len(), 1, "{rows:#?}");
    let row = &rows[0];
    assert_eq!(row.anchor.kind, AnchorKind::Interact);
    assert_eq!(row.anchor.call, Call::Interact { target: 100751 });
    assert_eq!(row.anchor.target, Some(100751));
    assert_eq!((row.first_tick, row.last_tick), (32, 41));
    assert_eq!(
        row.calls,
        vec![(
            36,
            Call::DialogChoice {
                dialog: 3995,
                button: -1
            }
        )]
    );
    assert!(row.setup_chat.is_empty());
    let dialog = row
        .server
        .iter()
        .find(|(_, name, _)| name == "onDialogDisplay")
        .expect("onDialogDisplay in the row");
    assert_eq!(dialog.0, 32);
    assert_eq!(dialog.2["dialog_id"], 3995);
}

#[test]
fn interact_zero_uses_the_last_set_target() {
    let log = log_of(vec![
        tick(0, vec![cell("setTargetID", 0, &[100751], &[])], None),
        tick(1, vec![cell("interact", 74, &[0], &[])], None),
        // Inside the row, then picked up by the next interact(0).
        tick(2, vec![cell("setTargetID", 0, &[200], &[])], None),
        tick(3, vec![cell("interact", 74, &[0], &[])], None),
    ]);
    let rows = segment(&log);
    assert_eq!(rows.len(), 2, "{rows:#?}");
    assert_eq!(rows[0].anchor.call, Call::Interact { target: 0 });
    assert_eq!(rows[0].anchor.target, Some(100751));
    assert_eq!((rows[0].first_tick, rows[0].last_tick), (1, 2));
    assert!(rows[0].calls.is_empty(), "setTargetID is not a row call");
    assert_eq!(rows[1].anchor.target, Some(200));
}

/// LR-11: a click joins the row whose ticks hold it, the later row when two
/// anchors share a tick; one before the first anchor goes nowhere.
#[test]
fn clicks_join_the_row_of_their_tick() {
    let click = |seq: u32, button: &str| {
        let mut t = tick(seq, vec![], None);
        t.clicks.push(UiClick {
            window: "W".into(),
            button: button.into(),
            mouse: Some("left".into()),
        });
        t
    };
    let mut log = log_of(vec![
        click(0, "before"),
        tick(1, vec![cell("interact", 74, &[7], &[])], None),
        click(2, "first"),
        click(3, "shared"),
        tick(4, vec![], None),
    ]);
    log.ticks[3].tap = vec![
        cell("interact", 74, &[8], &[]),
        cell("interact", 74, &[9], &[]),
    ];
    let rows = segment(&log);
    let buttons = |r: &RecordedRow| -> Vec<(u32, String)> {
        r.clicks
            .iter()
            .map(|(t, c)| (*t, c.button.clone()))
            .collect()
    };
    assert_eq!(rows.len(), 3, "{rows:#?}");
    assert_eq!(buttons(&rows[0]), [(2, "first".to_string())]);
    assert!(rows[1].clicks.is_empty(), "{rows:#?}");
    assert_eq!(buttons(&rows[2]), [(3, "shared".to_string())]);
}

#[test]
fn interact_zero_with_no_target_set_has_no_target() {
    let rows = segment(&log_of(vec![tick(
        1,
        vec![cell("interact", 74, &[0], &[])],
        None,
    )]));
    assert_eq!(rows.len(), 1, "{rows:#?}");
    assert_eq!(rows[0].anchor.target, None);
}

#[test]
fn use_ability_and_move_item_open_their_own_rows() {
    let log = log_of(vec![
        tick(1, vec![cell("useAbility", 68, &[1234, 100751], &[])], None),
        tick(3, vec![cell("moveItem", 38, &[123456, 1, 7, 3], &[])], None),
    ]);
    let rows = segment(&log);
    let kinds: Vec<_> = rows.iter().map(|r| r.anchor.kind.clone()).collect();
    assert_eq!(kinds, vec![AnchorKind::UseAbility, AnchorKind::MoveItem]);
    let targets: Vec<_> = rows.iter().map(|r| r.anchor.target).collect();
    assert_eq!(targets, vec![Some(100751), None]);
    let spans: Vec<_> = rows.iter().map(|r| (r.first_tick, r.last_tick)).collect();
    assert_eq!(spans, vec![(1, 2), (3, 3)]);
}

/// Two anchors in one tick: the first is a one-tick row, the second runs on.
#[test]
fn two_anchors_in_one_tick() {
    let log = log_of(vec![
        tick(
            3,
            vec![
                cell("interact", 74, &[100751], &[]),
                cell("useItem", 39, &[55, 100751], &[]),
            ],
            None,
        ),
        tick(5, vec![], None),
    ]);
    let rows = segment(&log);
    let spans: Vec<_> = rows.iter().map(|r| (r.first_tick, r.last_tick)).collect();
    assert_eq!(spans, vec![(3, 3), (3, 5)]);
}

/// An outbound message ahead of an anchor in the same tick, in tap order,
/// belongs to the row before.
#[test]
fn an_outbound_row_before_an_anchor_goes_to_the_previous_row() {
    let out = json!({"dir": "out", "msg_name": "onDialogDisplay", "decoded": {"dialog_id": 1}});
    let log = log_of(vec![
        tick(1, vec![cell("interact", 74, &[100751], &[])], None),
        tick(3, vec![out, cell("useItem", 39, &[55, 100751], &[])], None),
    ]);
    let rows = segment(&log);
    assert_eq!(rows.len(), 2, "{rows:#?}");
    assert_eq!(rows[0].server.len(), 1);
    assert_eq!(rows[0].server[0].0, 3);
    assert!(rows[1].server.is_empty());
}

/// A teleport and a region leave inside a row are movement, not calls.
#[test]
fn movement_inside_a_row_is_not_a_call() {
    let pos = [1.0f32, 2.0, 3.0].map(|f| f.to_bits() as i32);
    let log = log_of(vec![
        tick(1, vec![cell("interact", 74, &[100751], &[])], None),
        tick(
            2,
            vec![cell("gmGotoXYZ", 163, &pos, &[]), region(false)],
            None,
        ),
    ]);
    let rows = segment(&log);
    assert_eq!(rows.len(), 1, "{rows:#?}");
    assert!(rows[0].calls.is_empty(), "{:?}", rows[0].calls);
}

/// The baseline is the last snapshot before the region, not the first: B
/// before and B after is no change, though A came earlier.
#[test]
fn region_baseline_is_the_last_snapshot_before_it() {
    let log = log_of(vec![
        tick(0, vec![], Some(DbSnapshot::default())),
        tick(1, vec![], Some(changed())),
        tick(2, vec![region(true)], None),
        tick(3, vec![], Some(changed())),
    ]);
    assert!(segment(&log).is_empty());
}

/// The region's own tick is in the window, not the baseline.
#[test]
fn region_baseline_excludes_the_regions_own_tick() {
    let log = log_of(vec![
        tick(0, vec![], Some(DbSnapshot::default())),
        tick(1, vec![region(true)], Some(changed())),
    ]);
    let rows = segment(&log);
    assert_eq!(rows.len(), 1, "{rows:#?}");
    assert_eq!(rows[0].anchor.kind, AnchorKind::Region);
}

/// The window stops before the next anchor's tick: the interact's change at
/// tick 5 is its own.
#[test]
fn region_window_end_is_exclusive() {
    let log = log_of(vec![
        tick(0, vec![], Some(DbSnapshot::default())),
        tick(1, vec![region(true)], None),
        tick(
            5,
            vec![cell("interact", 74, &[100751], &[])],
            Some(changed()),
        ),
    ]);
    let rows = segment(&log);
    assert_eq!(rows.len(), 1, "{rows:#?}");
    assert_eq!(rows[0].anchor.kind, AnchorKind::Interact);
}

/// No change between the region (tick 1) and the next anchor (tick 5); the
/// change at tick 6 belongs to the interact, so the region stays out.
#[test]
fn a_region_with_no_following_diff_is_not_an_anchor() {
    let same = DbSnapshot::default();
    let log = log_of(vec![
        tick(0, vec![], Some(same.clone())),
        tick(1, vec![region(true)], Some(same.clone())),
        tick(2, vec![], Some(same.clone())),
        tick(5, vec![cell("interact", 74, &[100751], &[])], None),
        tick(6, vec![], Some(changed())),
    ]);
    let rows = segment(&log);
    assert_eq!(rows.len(), 1, "{rows:#?}");
    assert_eq!(rows[0].anchor.kind, AnchorKind::Interact);
    assert_eq!(rows[0].first_tick, 5);
}

#[test]
fn a_region_followed_by_a_diff_is_an_anchor() {
    let ticks = |entering| {
        vec![
            tick(0, vec![], Some(DbSnapshot::default())),
            tick(1, vec![region(entering)], None),
            tick(3, vec![], Some(changed())),
        ]
    };
    let rows = segment(&log_of(ticks(true)));
    assert_eq!(rows.len(), 1, "{rows:#?}");
    assert_eq!(rows[0].anchor.kind, AnchorKind::Region);
    assert_eq!(
        rows[0].anchor.call,
        Call::Region {
            region: 14,
            entering: true
        }
    );
    assert_eq!(rows[0].anchor.target, None);
    assert_eq!((rows[0].first_tick, rows[0].last_tick), (1, 3));
    // Leaving the region is never an anchor, whatever follows.
    assert!(segment(&log_of(ticks(false))).is_empty());
}

#[test]
fn dot_chat_goes_to_the_next_rows_setup_and_bug_lines_vanish() {
    let log = log_of(vec![
        tick(
            0,
            vec![chat(".stand here"), chat(".bug it broke"), chat("hello")],
            None,
        ),
        tick(1, vec![cell("interact", 74, &[100751], &[])], None),
        tick(2, vec![chat(".give 55"), chat(".bug again")], None),
        tick(4, vec![cell("useItem", 39, &[55, 0], &[])], None),
        tick(5, vec![chat(".after the last anchor")], None),
    ]);
    let rows = segment(&log);
    assert_eq!(rows.len(), 2, "{rows:#?}");
    assert_eq!(rows[0].setup_chat, vec![".stand here".to_string()]);
    assert_eq!(rows[1].setup_chat, vec![".give 55".to_string()]);
    assert!(rows[0].calls.is_empty() && rows[1].calls.is_empty());
    assert_eq!((rows[0].first_tick, rows[0].last_tick), (1, 3));
    assert_eq!((rows[1].first_tick, rows[1].last_tick), (4, 5));
    assert_eq!(rows[1].anchor.kind, AnchorKind::UseItem);
    // A use with target 0 has no target, like an unresolved interact(0).
    assert_eq!(rows[1].anchor.target, None);
}

/// A `.bug` line the tap cut at 256 bytes (512 hex digits) still vanishes:
/// it is neither setup chat nor a row call.
#[test]
fn a_long_bug_line_cut_by_the_tap_vanishes() {
    let mut long_bug = chat(&format!(".bug {}", "x".repeat(300)));
    let hex = long_bug["args_hex"].as_str().unwrap()[..512].to_string();
    long_bug["args_hex"] = hex.into();
    let log = log_of(vec![
        tick(1, vec![cell("interact", 74, &[100751], &[])], None),
        tick(2, vec![long_bug], None),
        tick(3, vec![cell("useItem", 39, &[55, 100751], &[])], None),
    ]);
    let rows = segment(&log);
    assert_eq!(rows.len(), 2, "{rows:#?}");
    assert!(rows[0].calls.is_empty(), "{:?}", rows[0].calls);
    assert!(rows[1].setup_chat.is_empty(), "{:?}", rows[1].setup_chat);
}

/// Calls and outbound messages before the first anchor have no row; an
/// unknown call after it is kept for a TODO line.
#[test]
fn calls_before_the_first_anchor_are_dropped_and_others_kept() {
    let out = json!({"dir": "out", "msg_name": "onDialogDisplay", "decoded": {"dialog_id": 1}});
    let log = log_of(vec![
        tick(0, vec![cell("someCall", 50, &[], &[]), out.clone()], None),
        tick(1, vec![cell("interact", 74, &[100751], &[]), out], None),
        tick(2, vec![cell("someCall", 50, &[], &[])], None),
    ]);
    let rows = segment(&log);
    assert_eq!(rows.len(), 1, "{rows:#?}");
    assert_eq!(
        rows[0].calls,
        vec![(
            2,
            Call::Other {
                name: "someCall".into(),
                method_index: 50
            }
        )]
    );
    assert_eq!(rows[0].server.len(), 1);
    assert_eq!(rows[0].server[0].0, 1);
}
