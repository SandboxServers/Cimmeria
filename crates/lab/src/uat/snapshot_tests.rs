//! Snapshot and parity unit tests (lab-fixtures FX-10).

use serde_json::json;

use super::*;
use crate::uat::grade::Grade;
use crate::uat::spec_fixture::{FixtureItem, FixtureMission};

/// FS-P4's fixture: 622 on 80623, 1360 on 4037, no pistol, tutorial
/// 5882 unseen, in the cell block.
fn fs_p4() -> FixtureSpec {
    FixtureSpec {
        character: "praxis".into(),
        world: Some("Castle_CellBlock".into()),
        position: None,
        missions: vec![
            FixtureMission {
                id: 622,
                step: Some(80623),
                state: None,
            },
            FixtureMission {
                id: 1360,
                step: Some(4037),
                state: None,
            },
        ],
        items: vec![FixtureItem {
            id: 55,
            count: 1,
            container: None,
        }],
        tutorials_unseen: vec![5882],
    }
}

fn mission(id: i32, status: &str, step: Option<i32>) -> SnapMission {
    SnapMission {
        id,
        status: status.into(),
        step,
    }
}

fn item(id: i32, container: i32, count: i64) -> SnapItem {
    SnapItem {
        id,
        container,
        count,
    }
}

/// The chain state FS-P4's fixture names exactly.
fn matching() -> RowSnapshot {
    RowSnapshot {
        schema: SNAPSHOT_SCHEMA,
        character: "Labone".into(),
        world: Some("Castle_CellBlock".into()),
        missions: vec![
            mission(622, "active", Some(80623)),
            mission(1360, "active", Some(4037)),
        ],
        items: vec![item(55, 1, 1)],
        tutorials_seen: vec![],
        host_ms: 0,
    }
}

#[test]
fn a_matching_snapshot_has_parity() {
    assert_eq!(parity(&fs_p4(), &matching()), Parity::default());
}

#[test]
fn parity_reports_a_wrong_step_item_count_and_seen_tutorial() {
    let mut snap = matching();
    snap.missions[0].step = Some(2113);
    snap.items.clear();
    snap.tutorials_seen = vec![5882];
    let p = parity(&fs_p4(), &snap);
    assert_eq!(
        p.mismatches,
        vec![
            "mission 622: chain has step 2113, fixture wants 80623".to_string(),
            "item 55: chain carries 0, fixture wants 1".to_string(),
            "tutorial 5882: chain has seen it, fixture wants it unseen".to_string(),
        ]
    );
    assert!(p.unspecified.is_empty());
}

#[test]
fn parity_lists_unnamed_missions_as_unspecified_without_failing() {
    let mut snap = matching();
    snap.missions.push(mission(688, "active", Some(3001)));
    snap.missions.push(mission(700, "completed", None));
    let p = parity(&fs_p4(), &snap);
    assert!(p.mismatches.is_empty(), "{p:?}");
    assert_eq!(
        p.unspecified,
        vec![
            "mission 688: step 3001".to_string(),
            "mission 700: it completed".to_string()
        ]
    );
    let mut g = pass();
    apply_parity(&mut g, Some(&p));
    assert_eq!(g.result, RowResult::Pass);
    assert!(g.reasons.is_empty(), "{:?}", g.reasons);
    assert_eq!(g.flags.len(), 1, "{:?}", g.flags);
}

#[test]
fn parity_checks_the_world_and_each_mission_state() {
    let mut f = fs_p4();
    f.missions = vec![
        FixtureMission {
            id: 622,
            step: None,
            state: Some(FixtureMissionState::Completed),
        },
        FixtureMission {
            id: 1360,
            step: None,
            state: Some(FixtureMissionState::NotActive),
        },
        FixtureMission {
            id: 77,
            step: Some(9),
            state: None,
        },
    ];
    f.items.clear();
    let mut snap = matching();
    snap.world = Some("Castle".into());
    let p = parity(&f, &snap);
    assert_eq!(
        p.mismatches,
        vec![
            "world: chain is in Castle, fixture wants Castle_CellBlock".to_string(),
            "mission 622: chain has step 80623, fixture wants it completed".to_string(),
            "mission 1360: chain has step 4037, fixture wants it not active".to_string(),
            "mission 77: chain does not have it, fixture wants 9".to_string(),
        ]
    );
    // A completed mission matches on status alone (no step on its row).
    snap.world = None;
    snap.missions = vec![
        mission(622, "completed", None),
        mission(77, "active", Some(9)),
    ];
    let p = parity(&f, &snap);
    assert_eq!(
        p.mismatches,
        vec!["world: chain has no session, fixture wants Castle_CellBlock".to_string()]
    );
}

#[test]
fn parity_checks_the_named_container_once_the_total_holds() {
    let mut f = fs_p4();
    f.items = vec![FixtureItem {
        id: 3730,
        count: 1,
        container: Some("mission".into()),
    }];
    let mut snap = matching();
    // One carried, but in main: the total holds, the container does not.
    snap.items = vec![item(3730, 1, 1)];
    let p = parity(&f, &snap);
    assert_eq!(
        p.mismatches,
        vec!["item 3730: chain carries 0 in container 2, fixture wants 1".to_string()]
    );
    // Counts in several carried containers add up for the total.
    f.items[0].count = 3;
    snap.items = vec![item(3730, 2, 2), item(3730, 5, 1)];
    let p = parity(&f, &snap);
    assert_eq!(
        p.mismatches,
        vec!["item 3730: chain carries 2 in container 2, fixture wants 3".to_string()]
    );
}

#[test]
fn from_rows_reads_the_server_db_query_shape() {
    let missions = json!({ "row_count": 2, "truncated": false, "row_cap": 500, "rows": [
        { "id": 1360, "status": 2, "step": null },
        { "id": 622, "status": 1.0, "step": 80623 },
    ]});
    let items = json!({ "row_count": 2, "truncated": false, "row_cap": 500, "rows": [
        { "id": 55, "container": 1, "n": 1 },
        { "id": 3730, "container": 2, "n": 2 },
    ]});
    let tutorials = json!({ "row_count": 1, "truncated": false, "row_cap": 500,
                            "rows": [{ "id": 5882 }] });
    let snap = from_rows("Labone", &missions, &items, &tutorials).unwrap();
    assert_eq!(snap.schema, SNAPSHOT_SCHEMA);
    assert_eq!(snap.character, "Labone");
    assert_eq!(
        snap.missions,
        vec![
            mission(622, "active", Some(80623)),
            mission(1360, "completed", None)
        ]
    );
    assert_eq!(snap.items, vec![item(55, 1, 1), item(3730, 2, 2)]);
    assert_eq!(snap.tutorials_seen, vec![5882]);
    // A status the runner does not name keeps its number.
    let odd = json!({ "rows": [{ "id": 9, "status": 3, "step": 1 }] });
    let empty = json!({ "rows": [] });
    let snap = from_rows("Labone", &odd, &empty, &empty).unwrap();
    assert_eq!(snap.missions[0].status, "status 3");
}

#[test]
fn from_rows_refuses_a_truncated_or_malformed_reply() {
    let empty = json!({ "rows": [] });
    let cut = json!({ "rows": [], "truncated": true, "row_cap": 500 });
    let e = from_rows("Labone", &empty, &cut, &empty).unwrap_err();
    assert_eq!(e, "items: result truncated at 500 rows");
    let text = json!({ "rows": [{ "id": "622", "status": 1, "step": 1 }] });
    assert!(from_rows("Labone", &text, &empty, &empty)
        .unwrap_err()
        .starts_with("missions: id"));
    assert!(from_rows("Labone", &json!({}), &empty, &empty).is_err());
    assert!(from_rows("Lab one", &empty, &empty, &empty).is_err());
}

#[test]
fn snapshot_sql_is_end_anchored_and_refuses_a_name_that_is_not_letters() {
    let sql = snapshot_sql("Fxpraxisdefa").unwrap();
    for s in &sql {
        assert!(s.contains("ILIKE '% fxpraxisdefa'"), "{s}");
        assert!(!s.contains("'%fxpraxisdefa%'"), "{s}");
    }
    assert!(sql[1].contains("IN (1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15)"));
    for bad in ["", "Fx'; drop", "Fxpraxis2", "Fxé", "Fx_a"] {
        assert!(snapshot_sql(bad).is_err(), "{bad:?}");
    }
}

fn pass() -> Grade {
    Grade {
        result: RowResult::Pass,
        reasons: vec![],
        flags: vec![],
        native_used: None,
    }
}

#[test]
fn a_mismatch_turns_a_pass_into_a_fail_and_flags_any_other_result() {
    let p = Parity {
        mismatches: vec!["item 55: chain carries 0, fixture wants 1".into()],
        ..Default::default()
    };
    let mut g = pass();
    apply_parity(&mut g, Some(&p));
    assert_eq!(g.result, RowResult::Fail);
    assert_eq!(
        g.reasons,
        vec!["fixture parity: item 55: chain carries 0, fixture wants 1".to_string()]
    );
    let mut g = Grade {
        result: RowResult::NeedsHuman,
        ..pass()
    };
    apply_parity(&mut g, Some(&p));
    assert_eq!(g.result, RowResult::NeedsHuman);
    assert!(g.reasons.is_empty());
    assert_eq!(g.flags.len(), 1);
    // No parity (a row without a fixture, or run from it): nothing changes.
    let mut g = pass();
    apply_parity(&mut g, None);
    assert_eq!(g, pass());
}

#[test]
fn a_snapshot_error_is_a_reason_that_keeps_the_pass() {
    let p = Parity {
        snapshot_error: Some("missions: lab-mcp HTTP 403".into()),
        ..Default::default()
    };
    let mut g = pass();
    apply_parity(&mut g, Some(&p));
    assert_eq!(g.result, RowResult::Pass);
    assert_eq!(
        g.reasons,
        vec!["fixture parity not checked: missions: lab-mcp HTTP 403".to_string()]
    );
}

#[test]
fn the_parity_reason_is_capped_by_characters() {
    let p = Parity {
        mismatches: (0..40)
            .map(|i| format!("mission {i}: chain has step é, fixture wants 1"))
            .collect(),
        ..Default::default()
    };
    let mut g = pass();
    apply_parity(&mut g, Some(&p));
    let r = &g.reasons[0];
    assert_eq!(r.chars().count(), PARITY_CAP, "{r}");
    assert!(r.starts_with("fixture parity: mission 0: "), "{r}");
    assert!(r.ends_with("..."), "{r}");
}

#[test]
fn parity_round_trips_through_the_row_json() {
    let p = Parity {
        mismatches: vec!["a".into()],
        unspecified: vec!["b".into()],
        snapshot_error: None,
    };
    let v = serde_json::to_value(&p).unwrap();
    assert!(v.get("snapshot_error").is_none(), "{v}");
    let back: Parity = serde_json::from_value(v).unwrap();
    assert_eq!(back, p);
}
