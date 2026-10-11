//! The pure half of a row fixture (lab-fixtures FX-05): the character a
//! fixture is staged for, the `server_stage_fixture` arguments, and the
//! read-only checks that prove the database holds what was staged. The
//! runner's session step (FX-06) and verify step (FX-07) call these; no
//! I/O happens here.

use serde_json::{json, Value};

use super::spec::FixtureSpec;
use super::spec_fixture::FixtureMissionState;

/// The variable a row's specs read for the fixture character's name.
// Read by the FX-06 session step; no caller reads it yet (tests included).
#[allow(dead_code)]
pub const FIXTURE_CHARACTER_VAR: &str = "fixture_character";

/// Containers a fixture counts as carried: main through crafting. This
/// mirrors `FIXTURE_CARRIED_CONTAINERS` in base-session, which the server
/// uses when it stages; the two must move together.
// FX-06 is the first non-test caller.
#[cfg_attr(not(test), allow(dead_code))]
pub const FIXTURE_CARRIED_CONTAINERS: &[i32] = &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15];

/// One read-only check: a label for the BLOCKED reason, the SQL, a JSON
/// pointer into the `server_db_query` result and the value it must equal.
// FX-06 is the first non-test caller.
#[cfg_attr(not(test), allow(dead_code))]
#[derive(Debug, Clone, PartialEq)]
pub struct FixtureCheck {
    /// The label a BLOCKED reason names, e.g. `mission 622 active`.
    pub what: String,
    /// The read-only SQL the check runs through `server_db_query`.
    pub sql: String,
    /// A JSON pointer into the query result, e.g. `/rows/0/n`.
    pub pointer: String,
    /// The value at `pointer` that proves the check.
    pub want: Value,
}

/// The fixture character's name for a profile in a lab instance: `Fx`,
/// the profile's letters (at most 8), then the instance label's letters
/// (digits become `a` to `j`, at most 4). Letters only, so the name can
/// be formatted into the SQL of [`verify_checks`] without escaping.
// FX-06 is the first non-test caller.
#[cfg_attr(not(test), allow(dead_code))]
pub fn fixture_character_name(profile: &str, instance: &str) -> String {
    let p: String = profile
        .chars()
        .filter(char::is_ascii_alphabetic)
        .take(8)
        .map(|c| c.to_ascii_lowercase())
        .collect();
    let i: String = instance
        .chars()
        .filter_map(|c| match c {
            '0'..='9' => Some((b'a' + (c as u8 - b'0')) as char),
            c if c.is_ascii_alphabetic() => Some(c.to_ascii_lowercase()),
            _ => None,
        })
        .take(4)
        .collect();
    format!("Fx{p}{i}")
}

/// The `server_stage_fixture` arguments: `f` as JSON with `character` set
/// to `name`. Null fields are dropped, so the tool sees only what the spec
/// says (an absent `step` is not `"step": null`).
// FX-06 is the first non-test caller.
#[cfg_attr(not(test), allow(dead_code))]
pub fn stage_args(f: &FixtureSpec, name: &str) -> Value {
    let mut v = serde_json::to_value(f).unwrap_or(Value::Null);
    drop_nulls(&mut v);
    if let Value::Object(o) = &mut v {
        o.insert("character".into(), json!(name));
    }
    v
}

fn drop_nulls(v: &mut Value) {
    match v {
        Value::Object(o) => {
            o.retain(|_, x| !x.is_null());
            o.values_mut().for_each(drop_nulls);
        }
        Value::Array(a) => a.iter_mut().for_each(drop_nulls),
        _ => {}
    }
}

/// The read-only checks that prove a staged fixture is in the database for
/// the character `name`: per mission its row, per item its carried count,
/// per tutorial that it is no longer unseen. `name` is formatted into SQL,
/// so it must be ASCII letters.
// FX-06 is the first non-test caller.
#[cfg_attr(not(test), allow(dead_code))]
pub fn verify_checks(f: &FixtureSpec, name: &str) -> Result<Vec<FixtureCheck>, String> {
    if name.is_empty() || !name.chars().all(|c| c.is_ascii_alphabetic()) {
        return Err(format!(
            "fixture character {name:?} must be ASCII letters only"
        ));
    }
    let n = name.to_ascii_lowercase();
    let mut out = Vec::new();
    for m in &f.missions {
        let sql = format!(
            "SELECT count(*) AS n_rows, max(m.status) AS status, max(m.current_step_id) AS step \
             FROM sgw_mission m JOIN sgw_player p ON p.player_id = m.player_id \
             WHERE p.player_name ILIKE '% {n}' AND m.mission_id = {}",
            m.id
        );
        match m.resolved_state()? {
            FixtureMissionState::Active => {
                // `resolved_state` only yields Active with a step.
                let step = m
                    .step
                    .ok_or_else(|| format!("mission {}: active needs a step", m.id))?;
                out.push(check(
                    format!("mission {} on step {step}", m.id),
                    &sql,
                    "/rows/0/step",
                    json!(step),
                ));
                out.push(check(
                    format!("mission {} active", m.id),
                    &sql,
                    "/rows/0/status",
                    json!(1),
                ));
            }
            FixtureMissionState::Completed => out.push(check(
                format!("mission {} completed", m.id),
                &sql,
                "/rows/0/status",
                json!(2),
            )),
            FixtureMissionState::NotActive => out.push(check(
                format!("mission {} not active", m.id),
                &sql,
                "/rows/0/n_rows",
                json!(0),
            )),
        }
    }
    let carried = FIXTURE_CARRIED_CONTAINERS
        .iter()
        .map(i32::to_string)
        .collect::<Vec<_>>()
        .join(", ");
    for i in &f.items {
        let sql = format!(
            "SELECT coalesce(sum(i.stack_size), 0) AS n FROM sgw_inventory i \
             JOIN sgw_player p ON p.player_id = i.character_id \
             WHERE p.player_name ILIKE '% {n}' AND i.type_id = {} \
             AND i.container_id IN ({carried})",
            i.id
        );
        out.push(check(
            format!("item {} count {}", i.id, i.count),
            &sql,
            "/rows/0/n",
            json!(i.count),
        ));
        // The item's own container holds the count too (`main` is 1,
        // `mission` is 2), so a count carried in the wrong place fails.
        let cid = i.container_id()?;
        let sql = format!(
            "SELECT coalesce(sum(i.stack_size), 0) AS n FROM sgw_inventory i \
             JOIN sgw_player p ON p.player_id = i.character_id \
             WHERE p.player_name ILIKE '% {n}' AND i.type_id = {} \
             AND i.container_id = {cid}",
            i.id
        );
        out.push(check(
            format!("item {} in container {cid}", i.id),
            &sql,
            "/rows/0/n",
            json!(i.count),
        ));
    }
    for t in &f.tutorials_unseen {
        let sql = format!(
            "SELECT count(*) AS n FROM sgw_player_tutorials t \
             JOIN sgw_player p ON p.player_id = t.player_id \
             WHERE p.player_name ILIKE '% {n}' AND t.tutorial_id = {t}"
        );
        out.push(check(
            format!("tutorial {t} unseen"),
            &sql,
            "/rows/0/n",
            json!(0),
        ));
    }
    Ok(out)
}

fn check(what: String, sql: &str, pointer: &str, want: Value) -> FixtureCheck {
    FixtureCheck {
        what,
        sql: sql.to_string(),
        pointer: pointer.to_string(),
        want,
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::uat::invoke::{ToolInvoker, ToolOutcome};
    use crate::uat::runner::{FixtureMode, RunRequest, Runner};
    use crate::uat::spec;
    use crate::uat::spec_fixture::{FixtureItem, FixtureMission};

    /// The README example: a fixture with a stepped mission, a not-active
    /// mission, two items (one in the mission container) and a tutorial.
    fn example() -> FixtureSpec {
        FixtureSpec {
            character: "praxis".into(),
            world: Some("Castle_CellBlock".into()),
            position: Some([-319.0, 73.6, -212.5]),
            missions: vec![
                FixtureMission {
                    id: 622,
                    step: Some(80623),
                    state: None,
                },
                FixtureMission {
                    id: 1360,
                    step: None,
                    state: Some(FixtureMissionState::NotActive),
                },
            ],
            items: vec![
                FixtureItem {
                    id: 55,
                    count: 0,
                    container: None,
                },
                FixtureItem {
                    id: 3730,
                    count: 0,
                    container: Some("mission".into()),
                },
            ],
            tutorials_unseen: vec![5882],
        }
    }

    #[test]
    fn fixture_names_are_letters_and_distinct_per_instance() {
        assert_eq!(fixture_character_name("praxis", "default"), "Fxpraxisdefa");
        assert_eq!(fixture_character_name("praxis", "p2"), "Fxpraxispc");
        assert_ne!(
            fixture_character_name("praxis", "p3"),
            fixture_character_name("praxis", "p2")
        );
        for (p, i) in [
            ("praxis", "default"),
            ("praxis", "p2"),
            ("praxis", "p3"),
            ("PraxisLongerThanEight", "Ab-9x"),
            ("x", ""),
        ] {
            let n = fixture_character_name(p, i);
            let mut cs = n.chars();
            assert!(cs.next().is_some_and(|c| c.is_ascii_uppercase()), "{n}");
            assert!(n.len() >= 2 && cs.all(|c| c.is_ascii_lowercase()), "{n}");
        }
    }

    #[test]
    fn stage_args_match_the_tool_contract() {
        let got = stage_args(&example(), "Fxpraxisdefa");
        // f32 literals: the arguments carry the f32 values the spec holds.
        let want = json!({
            "character": "Fxpraxisdefa",
            "world": "Castle_CellBlock",
            "position": [-319.0_f32, 73.6_f32, -212.5_f32],
            "missions": [{ "id": 622, "step": 80623 }, { "id": 1360, "state": "not_active" }],
            "items": [{ "id": 55, "count": 0 }, { "id": 3730, "count": 0, "container": "mission" }],
            "tutorials_unseen": [5882]
        });
        assert_eq!(got, want);
        assert!(!got.to_string().contains("null"), "{got}");
    }

    #[test]
    fn verify_checks_cover_each_mission_state_item_and_tutorial() {
        let mut f = example();
        f.missions.push(FixtureMission {
            id: 77,
            step: None,
            state: Some(FixtureMissionState::Completed),
        });
        let checks = verify_checks(&f, "Fxpraxisdefa").unwrap();
        // 4 mission checks, 2 per item (carried total and own container), 1 tutorial.
        assert_eq!(checks.len(), 9, "{checks:?}");
        let has = |mission_or_item: &str, pointer: &str, want: Value| {
            checks
                .iter()
                .any(|c| c.sql.contains(mission_or_item) && c.pointer == pointer && c.want == want)
        };
        assert!(has("m.mission_id = 622", "/rows/0/step", json!(80623)));
        assert!(has("m.mission_id = 622", "/rows/0/status", json!(1)));
        assert!(has("m.mission_id = 77", "/rows/0/status", json!(2)));
        assert!(has("m.mission_id = 1360", "/rows/0/n_rows", json!(0)));
        assert!(has("i.type_id = 55", "/rows/0/n", json!(0)));
        assert!(has("i.type_id = 3730", "/rows/0/n", json!(0)));
        assert!(has("t.tutorial_id = 5882", "/rows/0/n", json!(0)));
        for c in &checks {
            assert!(c.sql.contains("ILIKE '% fxpraxisdefa'"), "{}", c.sql);
        }
        assert!(checks[4]
            .sql
            .contains("IN (1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15)"));
        // The mission-container item (3730) is checked in container 2 as well.
        let in_mission = checks
            .iter()
            .find(|c| c.what == "item 3730 in container 2")
            .expect("container check for item 3730");
        assert!(
            in_mission.sql.contains("i.type_id = 3730"),
            "{}",
            in_mission.sql
        );
        assert!(
            in_mission.sql.contains("i.container_id = 2"),
            "{}",
            in_mission.sql
        );
        assert_eq!(in_mission.pointer, "/rows/0/n");
        assert_eq!(in_mission.want, json!(0));
        let in_main = checks
            .iter()
            .find(|c| c.what == "item 55 in container 1")
            .expect("container check for item 55");
        assert!(
            in_main.sql.contains("i.container_id = 1"),
            "{}",
            in_main.sql
        );
    }

    #[test]
    fn verify_sql_does_not_match_a_longer_name() {
        let checks = verify_checks(&example(), "Fxab").unwrap();
        for c in &checks {
            assert!(c.sql.contains("ILIKE '% fxab'"), "{}", c.sql);
            assert!(!c.sql.contains("'%fxab%'"), "{}", c.sql);
        }
        // Non-ASCII letters are not letters for this check.
        assert!(verify_checks(&example(), "Fxé").is_err());
    }

    #[test]
    fn verify_checks_refuse_a_name_that_is_not_letters() {
        let f = example();
        assert!(verify_checks(&f, "Fx'; drop").is_err());
        assert!(verify_checks(&f, "Fxpraxis2").is_err());
        assert!(verify_checks(&f, "").is_err());
    }

    /// Calls no tool: `base_vars` only reads the request.
    struct NoTools;

    impl ToolInvoker for NoTools {
        fn has_tool(&self, _name: &str) -> bool {
            false
        }
        fn tool_names(&self) -> Vec<String> {
            Vec::new()
        }
        async fn call(&self, _name: &str, _args: Value) -> ToolOutcome {
            unreachable!("base_vars calls no tool")
        }
    }

    #[test]
    fn base_vars_carry_the_instance() {
        const TEXT: &str = r#"
schema = 1
[section]
id = "gm-parity"
system = "GM console command parity"
guide = "unified-uat.md#gm-console-command-parity"
ledger = "legacy-command-parity/README.md"
[[row]]
id = "M1-1"
title = "help"
expected = "Each answers in chat."
step = [{ chat = ".help", label = "help" }]
[[row.expect]]
id = "help-answers"
text = ".help lists commands"
source = "chat"
contains = "Available commands"
"#;
        let section = spec::parse(TEXT).unwrap();
        let tmp = tempfile::tempdir().unwrap();
        let req = RunRequest {
            root: tmp.path().to_path_buf(),
            instance: Some("p2".into()),
            no_settle: true,
            ..Default::default()
        };
        let runner = Runner::new(&NoTools, None, req).unwrap();
        let vars = runner.base_vars(&section, &section.rows[0]);
        assert_eq!(vars.get("instance"), Some(&json!("p2")));

        let req = RunRequest {
            root: tmp.path().to_path_buf(),
            instance: None,
            no_settle: true,
            ..Default::default()
        };
        let runner = Runner::new(&NoTools, None, req).unwrap();
        let vars = runner.base_vars(&section, &section.rows[0]);
        assert!(!vars.contains_key("instance"), "{vars:?}");
    }

    #[test]
    fn fixture_mode_defaults_to_auto_and_parses_snake_case() {
        assert_eq!(FixtureMode::default(), FixtureMode::Auto);
        let parse = |s: &str| serde_json::from_str::<FixtureMode>(s);
        assert_eq!(parse("\"always\"").unwrap(), FixtureMode::Always);
        assert_eq!(parse("\"never\"").unwrap(), FixtureMode::Never);
        assert!(parse("\"Always\"").is_err());
    }
}
