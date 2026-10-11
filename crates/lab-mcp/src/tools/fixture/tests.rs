//! FX-04 unit tests: `server_stage_fixture`'s gates, parse and shape
//! checks, and its refusal logging, without a database. The live-DB tests
//! are in `live_db_tests.rs` and share the helpers here.

use std::future::Future;
use std::time::Duration;

use serde_json::{json, Value};
use sqlx::PgPool;
use tracing::Level;

use cimmeria_services::base::lab_fixture::{StagedItem, StagedMission, StagedMissionState};
use cimmeria_services::base::OnlinePlayer;

use super::{
    check_shape, gate_character, is_fixture_name, parse_args, refuse_if_online, stage_from,
    Character, Refusal, StageContext, REFUSED,
};
use crate::test_support::LogCapture;

/// The Cellblock weapon mission and its middle step.
pub(super) const MISSION: i32 = 622;
pub(super) const MISSION_STEP: i32 = 80623;
pub(super) const OTHER_MISSION: i32 = 1360;

/// A [`StageContext`] with a fixed pool and roster: what `LabState` hands
/// the tool, without a running server.
pub(super) struct FakeContext {
    pub(super) pool: Option<PgPool>,
    pub(super) roster: Vec<OnlinePlayer>,
}

impl StageContext for FakeContext {
    fn pool(&self) -> impl Future<Output = Option<PgPool>> + Send {
        let pool = self.pool.clone();
        async move { pool }
    }

    fn roster(&self) -> impl Future<Output = Vec<OnlinePlayer>> + Send {
        let roster = self.roster.clone();
        async move { roster }
    }
}

/// The runner's arguments for `name` (`crate::uat::fixture::stage_args` in
/// cimmeria-lab): a f32 position as f64 JSON, snake_case states, nulls
/// dropped.
pub(super) fn runner_args(name: &str) -> Value {
    json!({
        "character": name,
        "world": "Castle_CellBlock",
        "position": [-319.0, 73.5999984741211, -212.5],
        "missions": [
            { "id": MISSION, "step": MISSION_STEP },
            { "id": OTHER_MISSION, "state": "not_active" }
        ],
        "items": [],
        "tutorials_unseen": [5882]
    })
}

fn character(player_id: i32) -> Character {
    Character {
        player_id,
        player_name: "Uat Fxpraxisdefa".to_string(),
        account_id: player_id - 1,
        account_name: "lab".to_string(),
        accesslevel: 2,
    }
}

pub(super) fn session(player_id: Option<i32>, name: &str) -> OnlinePlayer {
    OnlinePlayer {
        id: 100,
        name: name.to_string(),
        player_id,
        account_id: 10,
        account_name: Some("lab".to_string()),
        archetype: "Soldier",
        level: 1,
        zone: "Castle_CellBlock".to_string(),
        ping: None,
        status: "in_world",
        session: "127.0.0.1:50000".to_string(),
    }
}

/// The refusal of `r`, checked for the contract prefix.
pub(super) fn refusal(r: Result<impl std::fmt::Debug, Refusal>) -> Refusal {
    let e = r.expect_err("must be refused");
    assert!(
        e.message.starts_with(REFUSED),
        "refusal without the prefix: {e:?}"
    );
    e
}

// ── Gates ─────────────────────────────────────────────────────────────

/// **Guard: an online character is refused.** The base reads a fixture
/// only at Play, so one staged for a character in the world would land on
/// a later login. A session counts by player id, or by name before
/// `playCharacter` set the id.
#[test]
fn an_online_character_is_refused() {
    let ch = character(77);
    let e = refusal(refuse_if_online(
        "Fxpraxisdefa",
        &ch,
        &[
            session(Some(5), "Someone"),
            session(Some(77), "Uat Fxpraxisdefa"),
        ],
    ));
    assert_eq!(e.message, "stage refused: character Fxpraxisdefa is online");
    assert_eq!(e.reason, "online");
    refusal(refuse_if_online(
        "Fxpraxisdefa",
        &ch,
        &[session(None, "uat fxpraxisdefa")],
    ));
    assert!(refuse_if_online("Fxpraxisdefa", &ch, &[session(Some(5), "Someone")]).is_ok());
    assert!(
        refuse_if_online("Fxpraxisdefa", &ch, &[session(None, "")]).is_ok(),
        "a nameless character-select session is no character's"
    );
}

/// The runner's `fixture_character_name` shape, `^Fx[A-Za-z]+$`, on the
/// stored name's last word.
#[test]
fn only_a_fixture_last_name_is_a_fixture_character() {
    for name in ["Uat Fxpraxisdefa", "Fxa", "Lab Uat FxAbc"] {
        assert!(is_fixture_name(name), "{name} is a fixture name");
    }
    for name in [
        "Uat Smithers",
        "Uat Fx",
        "Uat Fx04abc",
        "Uat fxabc",
        "Fxabc Smithers",
        "Uat Fxab-c",
        "Uat Fxabc ",
        "",
    ] {
        assert!(!is_fixture_name(name), "{name:?} is not a fixture name");
    }
}

/// The fixture-name gate comes before the GM gate, so a real character's
/// name tells the caller nothing about its account.
#[test]
fn a_real_character_is_refused_before_its_account_is_looked_at() {
    let mut ch = character(77);
    ch.player_name = "Uat Smithers".to_string();
    ch.accesslevel = 0;
    let e = refusal(gate_character("Smithers", &ch, &[]));
    assert_eq!(e.reason, "not_fixture_character");
    assert_eq!(
        e.message,
        "stage refused: character Uat Smithers is not a lab fixture character (last name must be Fx...)"
    );
    ch.player_name = "Uat Fxsmithers".to_string();
    assert_eq!(
        refusal(gate_character("Fxsmithers", &ch, &[])).reason,
        "not_gm"
    );
    ch.accesslevel = 2;
    assert!(gate_character("Fxsmithers", &ch, &[]).is_ok());
}

// ── Parse and shape ───────────────────────────────────────────────────

#[test]
fn the_runner_arguments_parse_into_the_staged_shape() {
    let args = parse_args(runner_args("Fxpraxisdefa")).expect("runner args parse");
    assert_eq!(args.position, Some([-319.0, 73.6, -212.5]));
    let (missions, items) = check_shape(&args).expect("shape");
    assert_eq!(
        missions,
        vec![
            StagedMission {
                mission_id: MISSION,
                state: StagedMissionState::Active {
                    step_id: MISSION_STEP
                }
            },
            StagedMission {
                mission_id: OTHER_MISSION,
                state: StagedMissionState::NotActive
            },
        ]
    );
    assert!(items.is_empty());

    let args = parse_args(json!({
        "character": "X",
        "missions": [{ "id": 1, "state": "completed" }, { "id": 2, "step": 3, "state": "active" }],
        "items": [{ "id": 55, "count": 0 }, { "id": 3730, "count": 10, "container": "mission" }]
    }))
    .expect("parse");
    let (missions, items) = check_shape(&args).expect("shape");
    assert_eq!(missions[0].state, StagedMissionState::Completed);
    assert_eq!(missions[1].state, StagedMissionState::Active { step_id: 3 });
    assert_eq!(
        items,
        vec![
            StagedItem {
                type_id: 55,
                count: 0,
                container_id: 1
            },
            StagedItem {
                type_id: 3730,
                count: 10,
                container_id: 2
            },
        ]
    );
}

/// A malformed call is an `arguments` refusal that names the field.
#[test]
fn a_malformed_call_is_a_refusal() {
    let cases = [
        (
            json!({ "character": "X", "rewind": true }),
            "unknown field `rewind`",
        ),
        (
            json!({ "character": "X", "missions": [{ "id": 1, "state": "done" }] }),
            "arguments: missions[0].state: ",
        ),
        (
            json!({ "character": "X", "items": [{ "id": 5, "count": -1 }] }),
            "arguments: items[0].count: ",
        ),
        (
            json!({ "world": "Castle" }),
            "arguments: character: missing",
        ),
        (json!({ "character": 7 }), "arguments: character: "),
        (json!([1, 2]), "arguments: the call is not a JSON object"),
    ];
    for (args, want) in cases {
        let e = refusal(parse_args(args.clone()));
        assert_eq!(e.reason, "arguments", "{args}");
        assert!(e.message.contains(want), "{args}: want {want:?} in {e:?}");
    }
}

#[test]
fn shape_refusals_name_the_field() {
    let shape = |v: Value| check_shape(&parse_args(v).expect("parse"));
    let cases = [
        (
            json!({ "character": "X", "items": [{ "id": 5, "count": 11 }] }),
            "item",
            "items[0]: count 11",
        ),
        (
            json!({ "character": "X", "items": [{ "id": 5, "count": 1, "container": "bank" }] }),
            "item",
            "items[0]: container \"bank\"",
        ),
        (
            json!({ "character": "X", "items": [{ "id": 5, "count": 1 }, { "id": 5, "count": 0, "container": "mission" }] }),
            "item",
            "items[1]: item 5 is listed twice",
        ),
        (
            json!({ "character": "X", "missions": [{ "id": 1, "state": "active" }] }),
            "mission",
            "missions[0]: mission 1 is active but has no step",
        ),
        (
            json!({ "character": "X", "missions": [{ "id": 1, "step": 2, "state": "not_active" }] }),
            "mission",
            "missions[0]: mission 1 has a step",
        ),
        (
            json!({ "character": "X", "missions": [{ "id": 1 }] }),
            "mission",
            "missions[0]: mission 1 needs a step or a state",
        ),
        (
            json!({ "character": "X", "missions": [{ "id": 1, "state": "completed" }, { "id": 1, "state": "not_active" }] }),
            "mission",
            "missions[1]: mission 1 is listed twice",
        ),
        (
            json!({ "character": "X", "tutorials_unseen": [5882, 0] }),
            "tutorial",
            "tutorials_unseen: 0",
        ),
        (
            json!({ "character": "X", "position": [0.0, 1e39, 0.0] }),
            "position",
            "position",
        ),
        (
            json!({ "character": "X", "world": " " }),
            "world",
            "world is empty",
        ),
    ];
    for (args, reason, want) in cases {
        let e = refusal(shape(args.clone()));
        assert_eq!(e.reason, reason, "{args}");
        assert!(e.message.contains(want), "{args}: want {want:?} in {e:?}");
    }
}

// ── The tool path, through a context ──────────────────────────────────

/// No database: a caller refusal, logged once as WARN with its reason.
#[tokio::test]
async fn no_database_is_a_logged_refusal() {
    let capture = LogCapture::install();
    let ctx = FakeContext {
        pool: None,
        roster: Vec::new(),
    };
    let e = refusal(stage_from(&ctx, runner_args("Fxpraxisdefa")).await);
    assert_eq!(e.message, "stage refused: no database");
    assert_eq!(e.reason, "no_database");
    assert!(!e.internal);
    assert!(
        capture
            .find_event(Level::WARN, "lab fixture stage refused", "no_database")
            .is_some(),
        "{:#?}",
        capture.all()
    );
}

/// A malformed call is logged too, before any database work.
#[tokio::test]
async fn a_malformed_call_is_logged() {
    let capture = LogCapture::install();
    let ctx = FakeContext {
        pool: None,
        roster: Vec::new(),
    };
    refusal(stage_from(&ctx, json!({ "character": "X", "rewind": 1 })).await);
    assert!(
        capture
            .find_event(Level::WARN, "lab fixture stage refused", "arguments")
            .is_some(),
        "{:#?}",
        capture.all()
    );
}

/// **Guard: a database failure is the server's, not the caller's.** It
/// logs ERROR `lookup_failed` and is marked internal, which the adapter
/// returns as an internal error instead of `invalid_params`.
#[tokio::test]
async fn a_database_failure_is_an_internal_error() {
    let capture = LogCapture::install();
    // Nothing listens on port 1 (the address the live-DB URL guard allows
    // for an unreachable pool); the short acquire timeout bounds the wait
    // whatever the platform does with a refused connection.
    let pool = sqlx::postgres::PgPoolOptions::new()
        .acquire_timeout(Duration::from_millis(500))
        .connect_lazy("postgres://fx04@127.0.0.1:1/none")
        .expect("lazy pool");
    let ctx = FakeContext {
        pool: Some(pool),
        roster: Vec::new(),
    };
    let e = stage_from(&ctx, runner_args("Fxpraxisdefa"))
        .await
        .expect_err("no database answers");
    assert!(e.internal, "{e:?}");
    assert_eq!(e.reason, "lookup_failed");
    assert!(
        e.message
            .starts_with("stage refused: character Fxpraxisdefa lookup failed: "),
        "{e:?}"
    );
    assert!(
        capture
            .find_event(Level::ERROR, "lab fixture stage refused", "lookup_failed")
            .is_some(),
        "{:#?}",
        capture.all()
    );
}
