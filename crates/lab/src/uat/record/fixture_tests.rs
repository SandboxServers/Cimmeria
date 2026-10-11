//! LR-09 fixture tests: the FS-P3 row's fixture from its start snapshot,
//! the commented and profile forms in a draft, and each start state the
//! schema cannot set becoming a TODO line.

use super::*;
use crate::uat::record::emit::{transform, Draft, TransformOptions};
use crate::uat::record::log::{read_log, SnapItem, SnapMission};
use crate::uat::record::segment::segment;
use crate::uat::spec;

const RECORD: &str = include_str!("fixtures/fs_p3_record.jsonl");
const FIRST_SESSION: &str = include_str!("../../../../../docs/guides/uat-specs/first-session.toml");

fn fs_p3() -> RecordLog {
    read_log(RECORD).unwrap()
}

/// `[section]` from first-session, which defines the `praxis` profile.
fn opts(profile: Option<&str>) -> TransformOptions {
    TransformOptions {
        base: Some(spec::parse(FIRST_SESSION).unwrap().section),
        profile: profile.map(str::to_string),
        ..Default::default()
    }
}

fn mission(id: i32, step: Option<i32>, state: Option<FixtureMissionState>) -> FixtureMission {
    FixtureMission { id, step, state }
}

fn spec_item(id: i32, count: u32, container: &str) -> FixtureItem {
    FixtureItem {
        id,
        count,
        container: Some(container.into()),
    }
}

/// The FS-P3 row's start: 622 active on step 2113, 1360 not yet taken.
fn fs_p3_fixture(character: &str) -> FixtureSpec {
    FixtureSpec {
        character: character.into(),
        world: None,
        position: None,
        missions: vec![
            mission(622, Some(2113), None),
            mission(1360, None, Some(FixtureMissionState::NotActive)),
        ],
        items: Vec::new(),
        tutorials_unseen: Vec::new(),
    }
}

fn stack(type_id: i32, container_id: i32, count: i32) -> SnapItem {
    SnapItem {
        type_id,
        container_id,
        count,
        name: None,
    }
}

fn changed(type_id: i32, container: i32) -> Diff {
    Diff::ItemCount {
        type_id,
        container,
        from: 0,
        to: 1,
        name: None,
    }
}

fn row_lines(d: &Draft) -> Vec<&str> {
    d.text
        .lines()
        .skip_while(|l| !l.starts_with("[[row]]"))
        .collect()
}

/// The fixture is the row's start: the end snapshot has 622 on 80623 and
/// 1360 active on 4037, so building from it changes both missions.
#[test]
fn fs_p3_fixture_puts_622_on_2113_and_1360_not_active() {
    let log = fs_p3();
    let rows = segment(&log);
    let (before, after) = row_snapshots(&log, &rows[0]).unwrap();
    let f = row_fixture("praxis", before, &diff_snapshots(before, after));
    assert_eq!(f.spec, fs_p3_fixture("praxis"));
    assert!(f.todos.is_empty(), "{:?}", f.todos);
}

/// Without `-Profile` the block is comments under its header, the draft
/// parses, and the row has no fixture.
#[test]
fn without_a_profile_the_fixture_is_commented_and_the_draft_parses() {
    let draft = transform(&fs_p3(), &opts(None)).unwrap();
    let want = "timeout_ms = 3500\n\
                # fixture (pass -Profile to enable):\n\
                # [row.fixture]\n\
                # character = \"<profile>\"\n\
                # missions = [\n\
                #   { id = 622, step = 2113 },\n\
                #   { id = 1360, state = \"not_active\" },\n\
                # ]\n";
    assert!(draft.text.ends_with(want), "{}", draft.text);
    let s = spec::parse(&draft.text).unwrap();
    assert_eq!(s.rows[0].fixture, None);
    assert!(s.section.profiles.is_empty());
}

/// With `-Profile praxis` the row carries the fixture, after its clauses,
/// and the draft copies `[section.profiles.praxis]` so the fixture's
/// character resolves.
#[test]
fn with_a_profile_the_draft_parses_and_the_row_has_the_fixture() {
    let draft = transform(&fs_p3(), &opts(Some("praxis"))).unwrap();
    let s = spec::parse(&draft.text).unwrap();
    assert_eq!(s.rows[0].fixture, Some(fs_p3_fixture("praxis")));
    let p = &s.section.profiles["praxis"];
    assert_eq!(
        (
            p.alignment.as_str(),
            p.archetype.as_str(),
            p.gender.as_str()
        ),
        ("praxis", "Soldier", "male")
    );
    let lines = row_lines(&draft);
    let fixture = lines.iter().position(|l| *l == "[row.fixture]").unwrap();
    let last_expect = lines.iter().rposition(|l| *l == "[[row.expect]]").unwrap();
    assert!(fixture > last_expect, "{}", draft.text);
    assert!(!draft.text.contains("pass -Profile"));
    assert_eq!(draft.todos, 0);
}

/// A profile the named section does not define would not load: an error,
/// not a draft.
#[test]
fn a_profile_the_section_lacks_is_an_error() {
    let err = transform(&fs_p3(), &opts(Some("bard"))).unwrap_err();
    assert!(err.contains("\"bard\""), "{err}");
}

/// A row written as comments keeps its fixture commented, profile or not.
#[test]
fn a_commented_row_keeps_its_fixture_commented() {
    let mut log = fs_p3();
    let target = log.ticks[32].entities[0].pos;
    for t in &mut log.ticks {
        if let Some(p) = &mut t.player {
            p.pos = [target[0] + 9.3, target[1], target[2]];
        }
    }
    let draft = transform(&log, &opts(Some("praxis"))).unwrap();
    assert_eq!(draft.rows, 0);
    assert!(draft.text.contains("\n# [row.fixture]\n"), "{}", draft.text);
    assert!(!draft.text.lines().any(|l| l.starts_with("[row")));
}

/// Fixtures set bags 1 and 2 by name, at most 10: a start stack in bag 3,
/// over the cap, or in both bags is a TODO and the item is left out whole
/// (staging would delete the stack it cannot set). An item with no start
/// stack is `count = 0`.
#[test]
fn an_item_in_bag_3_or_over_the_cap_is_a_todo_line() {
    let before = DbSnapshot {
        items: vec![
            stack(55, 3, 1),
            stack(55, 1, 1),
            stack(66, 1, 11),
            stack(77, 2, 10),
            stack(88, 1, 1),
            stack(88, 2, 1),
            // The bank is not rewritten by a fixture.
            stack(99, 17, 4),
        ],
        ..Default::default()
    };
    let diffs: Vec<Diff> = [55, 66, 77, 88, 99, 111]
        .into_iter()
        .map(|id| changed(id, 1))
        .collect();
    let f = row_fixture("praxis", &before, &diffs);
    assert_eq!(
        f.spec.items,
        vec![
            spec_item(77, 10, "mission"),
            spec_item(99, 0, "main"),
            spec_item(111, 0, "main"),
        ]
    );
    assert_eq!(
        f.todos,
        vec![
            "item 55 started in bag 3",
            "item 66 started at count 11 in bag 1 (a fixture sets 0 to 10)",
            "item 88 started in bags 1 and 2 (a fixture sets one bag)",
        ]
    );
    assert_eq!(
        fixture_toml(&f)[2..],
        [
            "items = [",
            "  { id = 77, count = 10, container = \"mission\" },",
            "  { id = 99, count = 0, container = \"main\" },",
            "  { id = 111, count = 0, container = \"main\" },",
            "]",
            "# TODO item 55 started in bag 3",
            "# TODO item 66 started at count 11 in bag 1 (a fixture sets 0 to 10)",
            "# TODO item 88 started in bags 1 and 2 (a fixture sets one bag)",
        ]
    );
}

/// With no start stack the item is `count = 0` in the bag the row gained
/// it in: bag 2 is `mission`; bags 3 to 14 have no fixture name, so a TODO
/// and the item left out.
#[test]
fn an_item_with_no_start_stack_takes_the_bag_it_was_gained_in() {
    let diffs = [changed(55, 2), changed(66, 3)];
    let f = row_fixture("praxis", &DbSnapshot::default(), &diffs);
    assert_eq!(f.spec.items, vec![spec_item(55, 0, "mission")]);
    assert_eq!(
        f.todos,
        vec!["item 66 gained in bag 3; the fixture cannot clear it"]
    );
}

/// The cap is on units, as the snapshot counts them: 11 units in one
/// stack is over it.
#[test]
fn the_count_cap_compares_units() {
    let before = DbSnapshot {
        items: vec![stack(66, 1, 11), stack(77, 1, 10)],
        ..Default::default()
    };
    let f = row_fixture("praxis", &before, &[changed(66, 1), changed(77, 1)]);
    assert_eq!(f.spec.items, vec![spec_item(77, 10, "main")]);
    assert_eq!(
        f.todos,
        vec!["item 66 started at count 11 in bag 1 (a fixture sets 0 to 10)"]
    );
}

/// A log of `before` at tick 1 and `after` at tick 3, one row over ticks 2
/// to 4.
fn one_row(before: DbSnapshot, after: DbSnapshot) -> (RecordLog, RecordedRow) {
    use crate::uat::record::decode::Call;
    use crate::uat::record::log::Tick;
    use crate::uat::record::segment::{Anchor, AnchorKind};
    let tick = |seq: u32, snapshot: Option<DbSnapshot>| Tick {
        seq,
        host_ms: 500 * i64::from(seq),
        snapshot,
        ..Tick::default()
    };
    let log = RecordLog {
        start: fs_p3().start,
        ticks: vec![
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
        server: Vec::new(),
        clicks: Vec::new(),
    };
    (log, row)
}

/// A row that restores nothing writes no fixture, profile or not; one that
/// only changed player fields writes just the TODO naming them.
#[test]
fn a_row_that_restores_nothing_writes_no_fixture() {
    let player = |level, exp| {
        Some(crate::uat::record::log::SnapPlayer {
            level,
            exp,
            naquadah: 0,
            world_location: "Castle".into(),
        })
    };
    let (log, row) = one_row(DbSnapshot::default(), DbSnapshot::default());
    for profile in [None, Some("praxis")] {
        assert_eq!(row_block(&log, &row, profile), (Vec::<String>::new(), 0));
    }

    let before = DbSnapshot {
        player: player(1, 0),
        ..Default::default()
    };
    let after = DbSnapshot {
        player: player(2, 150),
        ..Default::default()
    };
    let (log, row) = one_row(before, after);
    let want = vec!["# TODO row changes player level, exp; a fixture cannot set them".to_string()];
    for profile in [None, Some("praxis")] {
        assert_eq!(row_block(&log, &row, profile), (want.clone(), 1));
    }
}

/// The runner binds `${character}` to the fixture's character, so a
/// profile beside a `character_expr` is refused.
#[test]
fn a_profile_with_a_character_expr_is_an_error() {
    let mut o = opts(Some("praxis"));
    o.character_expr = Some("Px${run_id}".into());
    let err = transform(&fs_p3(), &o).unwrap_err();
    assert!(err.contains("-CharacterExpr"), "{err}");
}

/// With no named section there is no profile to copy: a clear error.
#[test]
fn a_profile_with_no_named_section_is_an_error() {
    let o = TransformOptions {
        profile: Some("praxis".into()),
        ..Default::default()
    };
    let err = transform(&fs_p3(), &o).unwrap_err();
    assert_eq!(
        err,
        "-Profile needs a named section to copy the profile from"
    );
}

/// Status 2 at the start is `state = "completed"`; an active mission with
/// no step, or another status, is a TODO.
#[test]
fn a_mission_completed_at_the_start_is_state_completed() {
    let row = |mission_id, status, step| SnapMission {
        mission_id,
        status,
        step,
        repeats: 1,
    };
    let before = DbSnapshot {
        missions: vec![row(10, 2, Some(5)), row(11, 1, None), row(12, 3, Some(7))],
        ..Default::default()
    };
    let diffs: Vec<Diff> = [10, 11, 12]
        .into_iter()
        .map(|mission| Diff::MissionStatus {
            mission,
            from: Some(1),
            to: None,
        })
        .collect();
    let f = row_fixture("praxis", &before, &diffs);
    assert_eq!(
        f.spec.missions,
        vec![mission(10, None, Some(FixtureMissionState::Completed))]
    );
    assert_eq!(
        f.todos,
        vec![
            "mission 11 started active on no step",
            "mission 12 started in status 3",
        ]
    );
    assert!(fixture_toml(&f).contains(&"  { id = 10, state = \"completed\" },".to_string()));
}
