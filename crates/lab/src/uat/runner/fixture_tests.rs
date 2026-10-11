//! Fixture-row runner tests (lab-fixtures FX-06), part one: the shared
//! fake lab endpoint and fixture mode (which rows chain, which stage). The
//! session tests (order, bootstrap, drains, failures after Play) are in
//! `fixture_session_tests.rs`.

use std::collections::VecDeque;
use std::sync::Mutex;

use serde_json::{json, Value};

use super::tests::{request, Fake};
use super::{FixtureMode, LastRow, LoadedSpec, RunRequest, Runner};
use crate::uat::evidence::{RowEvidence, RowResult, RunDir};
use crate::uat::invoke::{BoxedOutcome, ServerInvoker, ToolOutcome};
use crate::uat::spec;

/// The fixture character of profile `praxis` on the default instance.
pub(super) const FX: &str = "Fxpraxisdefa";

pub(super) const PROFILE: &str = r#"
[section.profiles.praxis]
alignment = "good"
archetype = "soldier"
gender = "male"
"#;

/// A row with no fixture that passes when `.help` answers.
pub(super) const CHAIN_ROW: &str = r#"
[[row]]
id = "FS-A"
title = "chained"
expected = "Help answers."
step = [{ chat = ".help" }]
[[row.expect]]
id = "c"
text = "help answers"
source = "chat"
contains = "Available commands"
"#;

/// A fixture row with a setup line, so a test can see setup did not run.
pub(super) const FIXTURE_ROW: &str = r#"
[[row]]
id = "FS-B"
title = "from a fixture"
expected = "Help answers."
setup = [{ chat = ".cleareffects" }]
step = [{ chat = ".help" }]
[row.fixture]
character = "praxis"
world = "Castle_CellBlock"
missions = [{ id = 622, step = 80623 }]
[[row.expect]]
id = "c"
text = "help answers"
source = "chat"
contains = "Available commands"
"#;

/// A second fixture row on the same profile.
pub(super) fn fixture_row_c() -> String {
    FIXTURE_ROW.replace("FS-B", "FS-C")
}

/// Substrings that tell the three snapshot statements apart (FX-10).
pub(super) const SNAP_MISSIONS: &str = "m.mission_id AS id";
pub(super) const SNAP_ITEMS: &str = "i.container_id AS container";
pub(super) const SNAP_TUTORIALS: &str = "t.tutorial_id AS id";

/// A pretend lab endpoint. `server_sessions` lists the character the fake
/// client is in world as (the last `lab_play_character`, else the fake's
/// own name: entity 41 for that one, 42 for a played one), and `stuck`
/// once that character has had a session. `server_stage_fixture` refuses
/// a character the list shows, as FX-04 does for one online.
pub(super) struct Endpoint<'f> {
    client: &'f Fake,
    pub(super) calls: Mutex<Vec<(String, Value)>>,
    /// `server_stage_fixture` fails with this message.
    pub(super) refuse: Option<String>,
    /// A name the server never drops once its character has been online.
    pub(super) stuck: Option<String>,
    /// The in-world session is missing from the list.
    pub(super) omit_in_world: bool,
    /// The world every session is listed in (`FIXTURE_ROW`'s world).
    pub(super) world: String,
    /// Sessions listed ahead of the in-world one, as given.
    pub(super) others: Vec<Value>,
    /// The player's position `server_entity_get` reports.
    pub(super) position: [f64; 3],
    /// `server_entity_get` answers `{"entity": null}`.
    pub(super) no_entity: bool,
    /// `server_db_query` fails with this message.
    pub(super) db_error: Option<String>,
    /// `server_sessions` fails with this message.
    pub(super) sessions_error: Option<String>,
    /// Scripted `server_db_query` replies per SQL substring, popped in
    /// order (the last repeats); see `db_replies`.
    db: Mutex<Vec<(String, VecDeque<Value>)>>,
}

fn contains_ci(hay: &str, needle: &str) -> bool {
    hay.to_ascii_lowercase()
        .contains(&needle.to_ascii_lowercase())
}

/// The `server_db_query` answer for a fixture the server staged in full:
/// mission 622 on its step and active, nothing else carried or unseen. The
/// chain snapshot (FX-10) sees the same state, so a chained `FIXTURE_ROW`
/// has parity.
fn default_db_answer(sql: &str) -> Value {
    if sql.contains(SNAP_MISSIONS) {
        json!({ "row_count": 1, "truncated": false, "row_cap": 500,
                "rows": [{ "id": 622, "status": 1, "step": 80623 }] })
    } else if sql.contains(SNAP_ITEMS) || sql.contains(SNAP_TUTORIALS) {
        json!({ "row_count": 0, "truncated": false, "row_cap": 500, "rows": [] })
    } else if sql.contains("m.mission_id = 622") {
        json!({ "rows": [{ "n_rows": 1, "status": 1, "step": 80623 }] })
    } else if sql.contains("sgw_mission") {
        json!({ "rows": [{ "n_rows": 0, "status": null, "step": null }] })
    } else {
        json!({ "rows": [{ "n": 0 }] })
    }
}

impl<'f> Endpoint<'f> {
    pub(super) fn new(client: &'f Fake) -> Self {
        Self {
            client,
            calls: Mutex::new(vec![]),
            refuse: None,
            stuck: None,
            omit_in_world: false,
            world: "Castle_CellBlock".into(),
            others: vec![],
            position: [-319.0, 73.6, -212.5],
            no_entity: false,
            db_error: None,
            sessions_error: None,
            db: Mutex::new(vec![]),
        }
    }

    pub(super) fn names(&self) -> Vec<String> {
        self.calls
            .lock()
            .unwrap()
            .iter()
            .map(|c| c.0.clone())
            .collect()
    }

    pub(super) fn count(&self, tool: &str) -> usize {
        self.names().iter().filter(|n| *n == tool).count()
    }

    /// Script the `server_db_query` replies for SQL holding `needle`, in
    /// order; the last one repeats. The first needle a query holds wins.
    pub(super) fn db_replies(&self, needle: &str, replies: Vec<Value>) {
        self.db
            .lock()
            .unwrap()
            .push((needle.to_string(), VecDeque::from(replies)));
    }

    fn db_query(&self, sql: &str) -> ToolOutcome {
        if let Some(e) = &self.db_error {
            return ToolOutcome::err(e.clone());
        }
        let mut scripted = self.db.lock().unwrap();
        let json = match scripted.iter_mut().find(|(n, _)| sql.contains(n.as_str())) {
            Some((_, replies)) if replies.len() > 1 => replies.pop_front().unwrap_or_default(),
            Some((_, replies)) => replies.front().cloned().unwrap_or_default(),
            None => default_db_answer(sql),
        };
        ToolOutcome {
            ok: true,
            json,
            ..Default::default()
        }
    }

    fn played(&self) -> Vec<String> {
        self.client
            .log
            .lock()
            .unwrap()
            .iter()
            .filter(|(t, _)| t == "lab_play_character")
            .filter_map(|(_, a)| a["name"].as_str().map(str::to_string))
            .collect()
    }

    fn sessions(&self) -> Value {
        let mut list = self.others.clone();
        let played = self.played();
        if *self.client.login.lock().unwrap() == "in_world" && !self.omit_in_world {
            let (name, id) = match played.last() {
                Some(p) if *p != self.client.name => (p.clone(), 42),
                _ => (self.client.name.clone(), 41),
            };
            list.push(json!({ "entity_id": id, "name": format!("Uat {name}"),
                              "world": self.world }));
        }
        if let Some(s) = &self.stuck {
            let had_session =
                contains_ci(s, &self.client.name) || played.iter().any(|p| contains_ci(s, p));
            let listed = list.iter().any(|r| r["name"] == s.as_str());
            if had_session && !listed {
                list.push(json!({ "entity_id": 43, "name": s, "world": self.world }));
            }
        }
        json!({ "count": list.len(), "sessions": list })
    }

    fn stage(&self, args: &Value) -> ToolOutcome {
        let target = args["character"].as_str().unwrap_or_default();
        let online = self.sessions()["sessions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| contains_ci(r["name"].as_str().unwrap_or_default(), target));
        let refusal = match &self.refuse {
            Some(msg) => Some(msg.clone()),
            None if online => Some(format!("stage refused: character {target} is online")),
            None => None,
        };
        match refusal {
            // A refusal is a JSON-RPC `invalid_params` error, worded the
            // way the HTTP session (`timeline::packet_tap`) surfaces one.
            Some(msg) => ToolOutcome::err(format!("lab-mcp error -32602: {msg}")),
            None => ToolOutcome {
                ok: true,
                json: json!({ "staged": true, "player_id": 9, "replaced": false }),
                ..Default::default()
            },
        }
    }
}

impl ServerInvoker for Endpoint<'_> {
    fn url(&self) -> &str {
        "http://127.0.0.1:1/mcp"
    }

    fn call<'a>(&'a self, name: &'a str, args: Value) -> BoxedOutcome<'a> {
        self.calls
            .lock()
            .unwrap()
            .push((name.to_string(), args.clone()));
        let out = match name {
            "server_sessions" if self.sessions_error.is_some() => {
                ToolOutcome::err(self.sessions_error.clone().unwrap_or_default())
            }
            "server_sessions" => ToolOutcome {
                ok: true,
                json: self.sessions(),
                ..Default::default()
            },
            "server_stage_fixture" => self.stage(&args),
            "server_db_query" => self.db_query(args["sql"].as_str().unwrap_or_default()),
            "server_entity_get" => ToolOutcome {
                ok: true,
                json: if self.no_entity {
                    json!({ "entity": null })
                } else {
                    json!({ "entity": { "entity_id": args["entity_id"],
                                        "position": self.position } })
                },
                ..Default::default()
            },
            _ => ToolOutcome::err(format!("unknown tool {name}")),
        };
        Box::pin(async move { out })
    }
}

/// A fake client that answers `.help` and lists `listed` characters.
pub(super) fn client(listed: Vec<Value>) -> Fake {
    Fake::new(&[(".help", &["Available commands: .help"])]).answer("lab_characters", listed)
}

pub(super) fn listing(names: &[&str]) -> Value {
    let rows: Vec<Value> = names.iter().map(|n| json!({ "name": n })).collect();
    json!({ "characters": rows })
}

/// The listing every test that is not about the bootstrap starts from.
pub(super) fn fx_listed() -> Vec<Value> {
    vec![listing(&[&format!("Uat {FX}")])]
}

pub(super) fn fixture_request(rows: &str, mode: FixtureMode) -> RunRequest {
    let tmp = tempfile::tempdir().unwrap().keep();
    let mut req = request(&tmp, &format!("{PROFILE}{rows}"));
    req.fixtures = mode;
    req
}

pub(super) async fn run_request(
    fake: &Fake,
    server: Option<&Endpoint<'_>>,
    req: RunRequest,
) -> Vec<RowEvidence> {
    let server = server.map(|s| s as &dyn ServerInvoker);
    let out = Runner::new(fake, server, req)
        .unwrap()
        .run_all()
        .await
        .unwrap();
    let mut rows = RunDir::open(std::path::Path::new(&out.run_dir))
        .unwrap()
        .rows()
        .unwrap();
    rows.sort_by_key(|r| out.rows.iter().position(|s| s.row == r.row_id));
    rows
}

pub(super) async fn run(
    fake: &Fake,
    server: Option<&Endpoint<'_>>,
    rows: &str,
    mode: FixtureMode,
) -> Vec<RowEvidence> {
    run_request(fake, server, fixture_request(rows, mode)).await
}

pub(super) fn tools(row: &RowEvidence) -> Vec<&str> {
    row.actions
        .iter()
        .filter_map(|a| a.tool.as_deref())
        .collect()
}

/// `want` appears in `seen` in this order (other calls may sit between).
pub(super) fn in_order(seen: &[&str], want: &[&str]) -> bool {
    let mut it = seen.iter();
    want.iter().all(|w| it.any(|s| s == w))
}

pub(super) fn ran(row: &RowEvidence, line: &str) -> bool {
    row.actions.iter().any(|a| a.requested == line)
}

#[tokio::test]
async fn auto_mode_continues_a_passing_chain() {
    let fake = client(fx_listed());
    let server = Endpoint::new(&fake);
    let rows = format!("{CHAIN_ROW}{FIXTURE_ROW}");
    let rows = run(&fake, Some(&server), &rows, FixtureMode::Auto).await;
    assert_eq!(rows[0].result, RowResult::Pass, "{:?}", rows[0].reasons);
    let b = &rows[1];
    assert_eq!(b.result, RowResult::Pass, "{:?}", b.reasons);
    assert_eq!(server.count("server_stage_fixture"), 0);
    assert_eq!(b.vars["fixture_mode"], "chain");
    assert_eq!(b.vars["fixture_character"], "Labone");
    // A chained row keeps the session: nothing stopped the client.
    assert!(!fake.names().contains(&"lab_client_stop".to_string()));
}

#[tokio::test]
async fn auto_mode_uses_the_fixture_after_a_failed_row() {
    // No `.help` reply: FS-A fails its clause.
    let fake = Fake::new(&[]).answer("lab_characters", fx_listed());
    let server = Endpoint::new(&fake);
    let rows = format!("{CHAIN_ROW}{FIXTURE_ROW}");
    let rows = run(&fake, Some(&server), &rows, FixtureMode::Auto).await;
    assert_eq!(rows[0].result, RowResult::Fail, "{:?}", rows[0].reasons);
    assert_eq!(rows[1].vars["fixture_mode"], "fixture");
    assert_eq!(
        server.count("server_stage_fixture"),
        1,
        "{:?}",
        server.names()
    );
}

/// `auto` chains only from the row right before in the spec: a skipped
/// row between them breaks the chain even though the previous row RUN
/// passed.
#[tokio::test]
async fn auto_mode_needs_the_spec_neighbour_not_the_previous_row_run() {
    let fake = client(fx_listed());
    let server = Endpoint::new(&fake);
    let between = CHAIN_ROW.replace("FS-A", "FS-Z");
    let mut req = fixture_request(
        &format!("{CHAIN_ROW}{between}{FIXTURE_ROW}"),
        FixtureMode::Auto,
    );
    req.rows = Some(vec!["FS-A".into(), "FS-B".into()]);
    let rows = run_request(&fake, Some(&server), req).await;
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].result, RowResult::Pass, "{:?}", rows[0].reasons);
    assert_eq!(rows[1].row_id, "FS-B");
    assert_eq!(rows[1].vars["fixture_mode"], "fixture");
}

/// The same row id in another section is not the neighbour either.
#[tokio::test]
async fn auto_mode_does_not_chain_across_sections() {
    let fake = client(fx_listed());
    let tmp = tempfile::tempdir().unwrap().keep();
    let mut req = request(&tmp, &format!("{PROFILE}{CHAIN_ROW}"));
    let other = format!(
        "{}{PROFILE}{CHAIN_ROW}{FIXTURE_ROW}",
        super::tests::HEAD.replace("id = \"gm-parity\"", "id = \"other\"")
    );
    req.sections.push(LoadedSpec {
        path: "other.toml".into(),
        sha256: "0".into(),
        spec: spec::parse(&other).unwrap(),
    });
    let mut runner = Runner::new(&fake, None, req).unwrap();
    let spec = runner.req.sections[1].spec.clone();
    let row = spec.rows.iter().find(|r| r.id == "FS-B").unwrap();
    runner.in_world_as = Some("Labone".into());
    runner.last_row = Some(LastRow {
        section: "gm-parity".into(),
        row_id: "FS-A".into(),
        passed: true,
    });
    assert!(runner.applies_fixture(&spec, row), "another section's FS-A");
    // The control: this section's own FS-A would continue the chain.
    runner.last_row = Some(LastRow {
        section: "other".into(),
        row_id: "FS-A".into(),
        passed: true,
    });
    assert!(!runner.applies_fixture(&spec, row));
}

#[tokio::test]
async fn always_mode_stages_even_after_a_pass() {
    let fake = client(fx_listed());
    let server = Endpoint::new(&fake);
    let rows = format!("{CHAIN_ROW}{FIXTURE_ROW}");
    let rows = run(&fake, Some(&server), &rows, FixtureMode::Always).await;
    assert_eq!(rows[0].result, RowResult::Pass, "{:?}", rows[0].reasons);
    assert_eq!(rows[1].result, RowResult::Pass, "{:?}", rows[1].reasons);
    assert_eq!(rows[1].vars["fixture_mode"], "fixture");
    assert_eq!(server.count("server_stage_fixture"), 1);
}

#[tokio::test]
async fn never_mode_never_stages() {
    let fake = client(fx_listed());
    let server = Endpoint::new(&fake);
    // A fixture row first: `never` runs it from wherever the client is.
    let rows = format!("{FIXTURE_ROW}{}", fixture_row_c());
    let rows = run(&fake, Some(&server), &rows, FixtureMode::Never).await;
    for r in &rows {
        assert_eq!(r.result, RowResult::Pass, "{} {:?}", r.row_id, r.reasons);
        assert_eq!(r.vars["fixture_mode"], "chain");
        assert_eq!(r.vars["fixture_character"], "Labone");
    }
    assert_eq!(server.count("server_stage_fixture"), 0);
    assert!(!fake.names().contains(&"lab_client_stop".to_string()));
}

#[tokio::test]
async fn no_lab_endpoint_blocks_a_fixture_row_and_ends_the_session() {
    let fake = client(fx_listed());
    let rows = run(&fake, None, FIXTURE_ROW, FixtureMode::Auto).await;
    let row = &rows[0];
    assert_eq!(row.result, RowResult::Blocked);
    assert_eq!(row.vars["fixture_mode"], "fixture");
    assert!(
        row.reasons.iter().any(|r| r
            == "fixture not established: fixture needs the server lab MCP \
                (CIMMERIA_LAB_MCP_URL/_TOKEN)"),
        "{:?}",
        row.reasons
    );
    assert!(!ran(row, ".cleareffects") && !ran(row, ".help"));
    assert!(!fake.names().contains(&"lab_play_character".to_string()));
    // The failure path still ends the session the run found: logged out
    // and stopped, though nothing can drain it without the endpoint.
    assert!(
        in_order(&tools(row), &["lab_logout", "lab_client_stop"]),
        "{:?}",
        tools(row)
    );
    assert!(!*fake.running.lock().unwrap());
}

/// A caller's `player_entity_id` names some other session: the fixture
/// row resolves its own (42), not the caller's (7) nor the lab
/// character's (41).
#[tokio::test]
async fn a_fixture_row_resolves_its_own_player_entity() {
    let fake = client(fx_listed());
    let server = Endpoint::new(&fake);
    let mut req = fixture_request(FIXTURE_ROW, FixtureMode::Auto);
    req.vars.insert("player_entity_id".into(), json!(7));
    let rows = run_request(&fake, Some(&server), req).await;
    assert_eq!(rows[0].result, RowResult::Pass, "{:?}", rows[0].reasons);
    assert_eq!(rows[0].vars["player_entity_id"], 42);
}
