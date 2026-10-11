//! Row fixture spec tests (lab-fixtures FX-01): parsing, `writes_state`
//! and every `check_fixture` rule.

use super::tests::MINI;
use super::*;
use crate::uat::spec_fixture::FixtureMissionState;

const FIRST_SESSION: &str = r#"
character = "praxis"
world = "Castle_CellBlock"
missions = [{ id = 622, step = 80623 }, { id = 1360, step = 4037 }]
items = [{ id = 55, count = 0 }]
tutorials_unseen = [5882]
position = [-319.0, 73.6, -212.5]
"#;

/// [`MINI`] with a `praxis` profile, and `fixture` as the row's `[row.fixture]`.
fn with_fixture(fixture: &str) -> String {
    MINI.replace(
        "\n[[row]]",
        "\n[section.profiles.praxis]\nalignment = \"good\"\narchetype = \"soldier\"\ngender = \"male\"\n\n[[row]]",
    )
    .replace(
        "\n[[row.expect]]",
        &format!("\n[row.fixture]\n{fixture}\n\n[[row.expect]]"),
    )
}

#[test]
fn a_fixture_row_parses_with_missions_items_and_tutorials() {
    let s = parse(&with_fixture(FIRST_SESSION)).unwrap();
    assert_eq!(s.section.profiles["praxis"].archetype, "soldier");
    let f = s.rows[0].fixture.as_ref().unwrap();
    assert_eq!(f.missions.len(), 2);
    for m in &f.missions {
        assert_eq!(m.resolved_state().unwrap(), FixtureMissionState::Active);
    }
    assert_eq!(f.items[0].container_id().unwrap(), 1);
    assert_eq!(f.tutorials_unseen, vec![5882]);
    assert!(f.writes_state());
}

#[test]
fn a_position_only_fixture_does_not_write_state() {
    let text =
        "character = \"praxis\"\nworld = \"Castle_CellBlock\"\nposition = [-319.0, 73.6, -212.5]";
    let s = parse(&with_fixture(text)).unwrap();
    assert!(!s.rows[0].fixture.as_ref().unwrap().writes_state());
}

#[test]
fn integer_position_coordinates_parse_as_floats() {
    let text = "character = \"praxis\"\nposition = [0, 73, -212]";
    let s = parse(&with_fixture(text)).unwrap();
    let f = s.rows[0].fixture.as_ref().unwrap();
    assert_eq!(f.position, Some([0.0, 73.0, -212.0]));
}

#[test]
fn each_state_setting_field_alone_writes_state() {
    let one_field = [
        "character = \"praxis\"\nmissions = [{ id = 622, step = 1 }]",
        "character = \"praxis\"\nitems = [{ id = 55, count = 0 }]",
        "character = \"praxis\"\ntutorials_unseen = [5882]",
    ];
    for text in one_field {
        let s = parse(&with_fixture(text)).unwrap();
        assert!(s.rows[0].fixture.as_ref().unwrap().writes_state(), "{text}");
    }
}

#[test]
fn a_mission_state_resolves_to_its_variant() {
    let text = "character = \"praxis\"\nmissions = [\
        { id = 1, state = \"not_active\" }, \
        { id = 2, state = \"completed\" }, \
        { id = 3, step = 5, state = \"active\" }, \
        { id = 4, step = 5 }]";
    let s = parse(&with_fixture(text)).unwrap();
    let m = &s.rows[0].fixture.as_ref().unwrap().missions;
    assert_eq!(
        m[0].resolved_state().unwrap(),
        FixtureMissionState::NotActive
    );
    assert_eq!(
        m[1].resolved_state().unwrap(),
        FixtureMissionState::Completed
    );
    assert_eq!(m[2].resolved_state().unwrap(), FixtureMissionState::Active);
    assert_eq!(m[3].resolved_state().unwrap(), FixtureMissionState::Active);
}

#[test]
fn an_item_container_maps_to_its_inventory_id() {
    let text = "character = \"praxis\"\nitems = [\
        { id = 55, count = 1 }, \
        { id = 56, count = 1, container = \"main\" }, \
        { id = 57, count = 1, container = \"mission\" }]";
    let s = parse(&with_fixture(text)).unwrap();
    let items = &s.rows[0].fixture.as_ref().unwrap().items;
    assert_eq!(items[0].container_id(), Ok(1));
    assert_eq!(items[1].container_id(), Ok(1));
    assert_eq!(items[2].container_id(), Ok(2));
}

#[test]
fn an_empty_profile_field_is_a_spec_error() {
    for (field, value) in [
        ("alignment", "good"),
        ("archetype", "soldier"),
        ("gender", "male"),
    ] {
        let text = with_fixture(FIRST_SESSION).replace(
            &format!("{field} = \"{value}\""),
            &format!("{field} = \"\""),
        );
        let e = parse(&text).unwrap_err();
        assert!(
            e.contains(&format!("section.profiles.praxis: {field} is empty")),
            "{e}"
        );
    }
}

#[test]
fn a_profile_key_is_one_to_eight_lowercase_letters() {
    // The key becomes part of the fixture character name, so it is checked
    // on the profile itself, not on a row.
    for key in ["commando1", "averylongname", "Praxis"] {
        let text = with_fixture(FIRST_SESSION).replace("praxis", key);
        let e = parse(&text).unwrap_err();
        assert!(
            e.contains(&format!("section.profiles.{key}: the key must be")),
            "{e}"
        );
    }
    let text = with_fixture(FIRST_SESSION).replace("praxis", "commando");
    assert!(parse(&text).is_ok());
}

#[test]
fn fixture_spec_errors_name_the_row() {
    let cases: [(String, &str); 19] = [
        (
            with_fixture(&FIRST_SESSION.replace("praxis", "bard")),
            "no [section.profiles.bard]",
        ),
        (
            with_fixture(FIRST_SESSION).replace("id = \"M1-1\"", "id = \"M1-1\"\nstate = \"char_select\""),
            "state \"char_select\"",
        ),
        (
            with_fixture("character = \"praxis\"\nmissions = [{ id = 622, step = 80623, state = \"completed\" }]"),
            "mission 622",
        ),
        (
            with_fixture("character = \"praxis\"\nmissions = [{ id = 622 }]"),
            "needs a step or a state",
        ),
        (
            with_fixture("character = \"praxis\"\nmissions = [{ id = 622, state = \"active\" }]"),
            "mission 622: active needs a step",
        ),
        (
            with_fixture("character = \"praxis\"\nitems = [{ id = 55, count = 11 }]"),
            "count 11",
        ),
        (
            with_fixture("character = \"praxis\"\nitems = [{ id = 55, count = 0, container = \"bank\" }]"),
            "\"bank\"",
        ),
        (
            with_fixture("character = \"praxis\"\nmissions = [{ id = 622, step = 1 }, { id = 622, step = 2 }]"),
            "duplicate mission 622",
        ),
        (
            with_fixture("character = \"praxis\"\nposition = [nan, 0.0, 0.0]"),
            "position",
        ),
        (
            with_fixture("character = \"praxis\"\nitems = [{ id = 55, count = 1 }]")
                .replace("id = \"M1-1\"", "id = \"M1-1\"\nplayers = 2"),
            "fixtures are one-player only",
        ),
        (
            with_fixture("character = \"praxis\"\nmissions = [{ id = 0, step = 1 }]"),
            "mission id 0 must be > 0",
        ),
        (
            with_fixture("character = \"praxis\"\nmissions = [{ id = 622, step = 0 }]"),
            "mission 622 step must be > 0",
        ),
        (
            with_fixture("character = \"praxis\"\nitems = [{ id = 0, count = 1 }]"),
            "item id 0 must be > 0",
        ),
        (
            with_fixture("character = \"praxis\"\nitems = [{ id = 55, count = 1 }, { id = 55, count = 2 }]"),
            "duplicate item 55",
        ),
        (
            with_fixture("character = \"praxis\"\ntutorials_unseen = [0]"),
            "tutorial id 0 must be > 0",
        ),
        (
            with_fixture("character = \"praxis\"\ntutorials_unseen = [5882, 5882]"),
            "duplicate tutorial 5882",
        ),
        (
            with_fixture("character = \"praxis\"\nworld = \"\""),
            "world \"\" must be non-empty",
        ),
        (
            with_fixture("character = \"praxis\"\nworld = \"Castle Block\""),
            "world \"Castle Block\" must be non-empty",
        ),
        (
            with_fixture("character = \"praxis\"\nposition = [inf, 0.0, 0.0]"),
            "three finite numbers",
        ),
    ];
    for (text, needle) in cases {
        let e = parse(&text).unwrap_err();
        assert!(e.contains("M1-1/fixture"), "{e}");
        assert!(e.contains(needle), "{e}");
    }
}
