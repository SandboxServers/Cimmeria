//! Ability runner tests (AB-L3) over the fake client's event store and a
//! fake `cimmeria-lab-mcp`: client_event clauses read only what arrived
//! after their mark, `${cast_id}` is found by each of its three paths in
//! order, the ability lab commands wait for their own feedback line, and
//! `server_ability_state` reads the right entity.

use std::sync::Mutex;

use serde_json::{json, Value};

use super::tests::{request, Fake};
use super::Runner;
use crate::uat::evidence::{RowEvidence, RowResult, RunDir, Verdict};
use crate::uat::invoke::{BoxedOutcome, ServerInvoker, ToolOutcome};

/// What the fake's `client_wait_event` matches: an exact kind (or a
/// trailing `*`) and equal fields (numbers compare by value).
fn event_matches(e: &Value, kind: &str, fields: &Value) -> bool {
    let have = e["kind"].as_str().unwrap_or_default();
    let kind_ok = match kind.strip_suffix('*') {
        Some(prefix) => have.starts_with(prefix),
        None => have == kind,
    };
    let text = |v: &Value| match v {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    };
    kind_ok
        && fields.as_object().is_none_or(|m| {
            m.iter().all(|(k, want)| {
                e["fields"]
                    .get(k)
                    .is_some_and(|got| text(got) == text(want))
            })
        })
}

impl Fake {
    /// Script `kind` with `fields` to land when `on` runs (a tool name, or
    /// `chat:<line>` for a typed line).
    pub(super) fn on_call(self, on: &str, kind: &str, fields: Value) -> Self {
        self.triggers
            .lock()
            .unwrap()
            .push((on.into(), kind.into(), fields));
        self
    }

    /// An event already in the store before the row starts.
    fn with_event(self, kind: &str, fields: Value) -> Self {
        self.push_event(kind, fields);
        self
    }

    fn push_event(&self, kind: &str, fields: Value) {
        let mut ev = self.events.lock().unwrap();
        let seq = ev.len() as u64 + 1;
        ev.push(json!({ "seq": seq, "kind": kind, "ts_ms": seq, "fields": fields }));
    }

    fn head(&self) -> u64 {
        self.events.lock().unwrap().len() as u64
    }

    /// Land every event scripted for `on`, once.
    pub(super) fn fire(&self, on: &str) {
        let due: Vec<(String, String, Value)> = {
            let mut t = self.triggers.lock().unwrap();
            let (due, keep) = t.drain(..).partition(|x| x.0 == on);
            *t = keep;
            due
        };
        for (_, kind, fields) in due {
            self.push_event(&kind, fields);
        }
    }

    pub(super) fn wait_event(&self, args: &Value) -> Value {
        if args["arm"] == true {
            return json!({ "armed": true, "cursor": { "seq": self.head() } });
        }
        let since = args["since_seq"].as_u64().unwrap_or(0);
        let count = args["count"].as_u64().unwrap_or(1) as usize;
        let kind = args["kind"].as_str().unwrap_or("*");
        let matched: Vec<Value> = self
            .events
            .lock()
            .unwrap()
            .iter()
            .filter(|e| e["seq"].as_u64().unwrap_or(0) > since)
            .filter(|e| event_matches(e, kind, &args["fields"]))
            .take(count)
            .cloned()
            .collect();
        json!({ "met": matched.len() >= count, "matched": matched, "gap": false })
    }

    pub(super) fn use_ability(&self, args: &Value) -> Value {
        let before = self.head();
        self.fire("client_use_ability");
        json!({
            "ability": { "id": args["ability_id"] },
            "native_level": { "tier": "N1", "label": "real_input" },
            "result": { "verdict": "effect_applied" },
            "event_seq": { "before": before, "after": self.head() },
            "press_ms": chrono::Utc::now().timestamp_millis() + *self.press_delay_ms.lock().unwrap(),
        })
    }
}

/// A pretend lab endpoint: sessions, the server log ring and the ability
/// snapshot (which, like the real tool, refuses a string entity id).
struct FakeServer {
    log: Vec<Value>,
    calls: Mutex<Vec<(String, Value)>>,
}

impl FakeServer {
    fn new(log: Vec<Value>) -> Self {
        Self {
            log,
            calls: Mutex::new(vec![]),
        }
    }
}

impl ServerInvoker for FakeServer {
    fn url(&self) -> &str {
        "http://127.0.0.1:1/mcp"
    }

    fn call<'a>(&'a self, name: &'a str, args: Value) -> BoxedOutcome<'a> {
        self.calls
            .lock()
            .unwrap()
            .push((name.to_string(), args.clone()));
        let ok = |json: Value| ToolOutcome {
            ok: true,
            json,
            ..Default::default()
        };
        let out = match name {
            "server_sessions" => ok(json!({ "sessions": [
                { "entity_id": 7, "name": "Labone" }, { "entity_id": 9, "name": "Labtwo" },
            ]})),
            "server_log_tail" => ok(json!({ "count": self.log.len(), "entries": self.log })),
            "server_ability_state" => match args["entity_id"].as_u64() {
                Some(id) => ok(json!({ "state": {
                    "entity_id": id,
                    "stats": [{ "stat_id": 4, "cur": 90 }, { "stat_id": 8, "cur": 140 }],
                }})),
                None => ToolOutcome::err("entity_id: invalid type: string, expected u32"),
            },
            _ => ToolOutcome::err(format!("unknown tool {name}")),
        };
        Box::pin(async move { out })
    }
}

async fn run(fake: &Fake, server: Option<&FakeServer>, rows: &str) -> RowEvidence {
    let tmp = tempfile::tempdir().unwrap().keep();
    let server = server.map(|s| s as &dyn ServerInvoker);
    let out = Runner::new(fake, server, request(&tmp, rows))
        .unwrap()
        .run_all()
        .await
        .unwrap();
    RunDir::open(std::path::Path::new(&out.run_dir))
        .unwrap()
        .rows()
        .unwrap()
        .remove(0)
}

fn ability_client() -> Fake {
    Fake::new(&[])
        .with("client_use_ability")
        .with("client_wait_event")
}

fn clause<'a>(row: &'a RowEvidence, id: &str) -> &'a crate::uat::evidence::ClauseResult {
    row.clauses.iter().find(|c| c.id == id).unwrap()
}

/// The press's cast-id note on its action.
fn cast_note(row: &RowEvidence) -> Value {
    row.actions
        .iter()
        .find(|a| a.tool.as_deref() == Some("client_use_ability"))
        .and_then(|a| a.calls.iter().find(|c| c.get("cast_id").is_some()))
        .cloned()
        .unwrap()
}

const PRESS_ROW: &str = r#"
[[row]]
id = "AB-U1"
title = "Heal Focus on no target"
expected = "The press leaves the client with no target."
step = [
  { tool = "@use_ability", args = { ability_id = 597 }, label = "press" },
  { chat = ".help", label = "second" },
]
[[row.expect]]
id = "sent"
text = "the press is sent with target 0"
source = "client_event"
event = "client.ability.sent"
match_fields = { ability_id = 597 }
field = "target_id"
op = "eq"
value = 0
[[row.expect]]
id = "no-drop"
text = "the client did not drop it"
source = "client_event"
event = "client.ability.press_dropped"
max_rows = 0
timeout_ms = 0
[[row.expect]]
id = "none-after"
text = "nothing more is sent after the second step"
source = "client_event"
event = "client.ability.sent"
since = "second"
max_rows = 0
timeout_ms = 0
[[row.expect]]
id = "help-printed"
text = "the second step printed"
source = "client_event"
event = "client.lua.print"
since = "second"
[[row.expect]]
id = "signoz"
text = "every cast row has the cast_id"
source = "signoz"
filter = "cast_id = ${cast_id}"
"#;

/// Events from before the row (a 99 target) are not this row's; the
/// press's own events grade every clause; `since` marks a later step.
#[tokio::test]
async fn client_event_clauses_read_only_what_arrived_after_their_mark() {
    let fake = ability_client()
        .with_event(
            "ability.sent",
            json!({ "ability_id": 597, "target_id": 99 }),
        )
        .on_call(
            "client_use_ability",
            "ability.sent",
            json!({ "ability_id": 597, "target_id": 0, "send_id": 3 }),
        )
        .on_call(
            "client_use_ability",
            "ability.recv",
            json!({ "method": "onEffectResults", "ability_id": 597, "cast_id": 4242, "source_id": 7 }),
        )
        .on_call("chat:.help", "lua.print", json!({ "text": "help" }));
    let row = run(&fake, None, PRESS_ROW).await;
    for id in ["sent", "no-drop", "none-after", "help-printed"] {
        let c = clause(&row, id);
        assert_eq!(
            c.verdict,
            Verdict::Pass,
            "{id}: {:?} {:?}",
            c.detail,
            c.observed
        );
    }
    assert_eq!(clause(&row, "sent").observed["matching_events"], 1);
    // The cast came from the client's own receipt.
    assert_eq!(cast_note(&row)["via"], "client_recv");
    assert_eq!(row.vars["cast_id"], 4242);
    assert_eq!(row.vars["cast_id_press"], 4242);
    // A cast_id is per caster: the composite names the caster too.
    assert_eq!(row.vars["cast_entity_id"], 7);
    assert_eq!(
        row.vars["cast_key"],
        "cast_id = 4242 AND (entity_id = 7 OR source_id = 7 OR invoker_id = 7)"
    );
    let q = clause(&row, "signoz").query.as_ref().unwrap();
    assert!(q["filter"].as_str().unwrap().contains("cast_id = 4242"));
    // The runner never drains the operator's client_events_read cursor.
    assert!(!fake.names().iter().any(|n| n == "client_events_read"));
}

/// The same clauses FAIL when the client did drop the press, and the
/// whole row is BLOCKED, naming the reader, when the store is not routed.
#[tokio::test]
async fn a_dropped_press_fails_and_a_missing_reader_blocks() {
    let fake = ability_client().on_call(
        "client_use_ability",
        "ability.press_dropped",
        json!({ "ability_id": 597, "reason": "not_known" }),
    );
    let row = run(&fake, None, PRESS_ROW).await;
    assert_eq!(row.result, RowResult::Fail);
    assert_eq!(clause(&row, "no-drop").verdict, Verdict::Fail);
    assert_eq!(clause(&row, "sent").verdict, Verdict::Fail);
    // No cast found anywhere: the vars are cleared and the SigNoz clause says so.
    for v in ["cast_id", "cast_entity_id", "cast_key", "cast_id_press"] {
        assert!(row.vars.get(v).is_none(), "{v}");
    }
    assert!(cast_note(&row)["tried"].as_array().unwrap().len() >= 2);
    let s = clause(&row, "signoz");
    assert!(s
        .detail
        .as_ref()
        .unwrap()
        .contains("${cast_id} was not captured"));

    let bare = Fake::new(&[]).with("client_use_ability");
    let row = run(&bare, None, PRESS_ROW).await;
    assert_eq!(row.result, RowResult::Blocked);
    assert!(
        row.reasons.iter().any(|r| r.contains("client_wait_event")),
        "{:?}",
        row.reasons
    );
}

fn log_row(ts: i64, fields: Value) -> Value {
    json!({ "timestamp_ms": ts, "level": "DEBUG", "target": "abilities", "message": "", "fields": fields })
}

const CAST_ROW: &str = r#"
[[row]]
id = "AB-U7"
title = "Aim"
expected = "x"
step = [{ tool = "@use_ability", args = { ability_id = 637 }, label = "aim" }]
[[row.expect]]
id = "signoz"
text = "the launch"
source = "signoz"
filter = "event = 'ability_launched' AND cast_id = ${cast_id}"
"#;

/// No client receipt: the sent packet range joins the server's receipt row
/// and that entity's launch; the other player's launch is not taken.
#[tokio::test]
async fn the_cast_id_falls_back_to_the_packet_seq_join() {
    let fake = ability_client()
        .on_call(
            "client_use_ability",
            "ability.sent",
            json!({ "ability_id": 637, "send_id": 5 }),
        )
        .on_call(
            "client_use_ability",
            "ability.sent_seq",
            json!({ "send_id": 5, "mercury_seq_first": 200, "mercury_seq_last": 201 }),
        );
    let server = FakeServer::new(vec![
        // Another connection with the same packet seq and ability, first:
        // packet seqs are per connection, so it is not this press's.
        log_row(
            0,
            json!({ "event": "use_ability_recv", "entity_id": 9, "ability_id": 637, "mercury_seq": 201 }),
        ),
        log_row(
            1,
            json!({ "event": "use_ability_recv", "entity_id": 7, "ability_id": 637, "mercury_seq": 201 }),
        ),
        log_row(
            2,
            json!({ "event": "ability_launched", "entity_id": 9, "ability_id": 637, "cast_id": 76 }),
        ),
        log_row(
            3,
            json!({ "event": "ability_launched", "entity_id": 7, "ability_id": 637, "cast_id": 77, "player_id": 700 }),
        ),
    ]);
    let row = run(&fake, Some(&server), CAST_ROW).await;
    assert_eq!(cast_note(&row)["via"], "seq_join", "{}", cast_note(&row));
    assert_eq!(row.vars["cast_id"], 77);
    assert_eq!(row.vars["cast_player_id"], 700);
    assert_eq!(row.vars["cast_entity_id"], 7);
}

/// Nothing from the client at all: the launch of (lab entity, ability)
/// nearest the press on the server clock (the anchor's bookmark), within
/// two seconds.
#[tokio::test]
async fn the_cast_id_falls_back_to_the_press_window() {
    let fake = ability_client();
    // The key goes out 5 s after the tool was called (lookup, placement):
    // the window is centred on the tool's `press_ms`, not the action start
    // (Copilot, #1183). Centred on the start it would pick cast 54.
    *fake.press_delay_ms.lock().unwrap() = 5000;
    // The fake's bookmark id is the server clock at the anchor, a moment
    // before the press.
    let at = 1_790_650_000_000_i64;
    let server = FakeServer::new(vec![
        log_row(
            at - 60_000,
            json!({ "event": "ability_launched", "entity_id": 7, "ability_id": 637, "cast_id": 50 }),
        ),
        log_row(
            at + 300,
            json!({ "event": "ability_launched", "entity_id": 7, "ability_id": 637, "cast_id": 54 }),
        ),
        log_row(
            at + 5300,
            json!({ "event": "ability_launched", "entity_id": 7, "ability_id": 637, "cast_id": 55 }),
        ),
        log_row(
            at + 5300,
            json!({ "event": "ability_launched", "entity_id": 9, "ability_id": 637, "cast_id": 56 }),
        ),
    ]);
    let row = run(&fake, Some(&server), CAST_ROW).await;
    assert_eq!(
        cast_note(&row)["via"],
        "press_window",
        "{}",
        cast_note(&row)
    );
    assert_eq!(row.vars["cast_id"], 55);
    assert_eq!(cast_note(&row)["press_time_from"], "tool");
}

/// A suppressed press shows only as `suppressed` on a later press row, and
/// its `sent` is gone without a trace: an upper bound on `sent` must not
/// pass (Copilot, #1183).
#[tokio::test]
async fn a_throttled_press_makes_an_upper_bound_on_sends_unverified() {
    let rows = r#"
[[row]]
id = "AB-U6"
title = "one send"
expected = "x"
step = [{ tool = "@use_ability", args = { ability_id = 2944 }, label = "press" }]
[[row.expect]]
id = "one-send"
text = "exactly one send"
source = "client_event"
event = "client.ability.sent"
match_fields = { ability_id = 2944 }
since = "press"
max_rows = 1
timeout_ms = 0
"#;
    let fake = ability_client()
        .on_call(
            "client_use_ability",
            "ability.sent",
            json!({ "ability_id": 2944 }),
        )
        .on_call(
            "client_use_ability",
            "ability.press",
            json!({ "ability_id": 2944, "suppressed": 3 }),
        );
    let row = run(&fake, None, rows).await;
    let c = clause(&row, "one-send");
    assert_eq!(c.verdict, Verdict::Unverified, "{:?}", c.observed);
    assert!(c.detail.as_ref().unwrap().contains("suppressed 3"));
}

const LAB_ROW: &str = r#"
[[row]]
id = "AB-U9"
title = "Call Target on the dummy"
expected = "x"
setup = [{ tool = "@dummy", args = { disposition = "friendly" } }]
step = [{ chat = ".help" }]
teardown = [
  { tool = "@clear_effects", args = { name = "${character}" } },
  { tool = "@cooldowns_reset" },
]
[[row.expect]]
id = "dummy-defense"
text = "the dummy's stat 8"
source = "server"
tool = "@ability_state"
args = { entity_id = "${dummy_id}" }
pointer = "/state/stats[stat_id=8]/cur"
op = "eq"
value = 140
[[row.expect]]
id = "own-state"
text = "no entity_id reads the lab character"
source = "server"
tool = "@ability_state"
pointer = "/state/entity_id"
op = "eq"
value = 7
"#;

/// `@dummy` types the line, waits for its reply and keeps the id as a
/// number; `@ability_state` reads it, or the lab character with no id;
/// the teardown commands target first and confirm their replies.
#[tokio::test]
async fn lab_commands_confirm_their_reply_and_feed_the_server_clause() {
    let fake = Fake::new(&[
        (
            ".dummy friendly",
            &["dummy [4242] placed: friendly SGC Jaffa (template 34), Health 1000000, Defense 140, Accuracy 60"],
        ),
        (".cleareffects", &["cleareffects [7] Labone: nothing to clear"]),
        (".cooldowns reset", &["cooldowns reset: none were running"]),
    ])
    .with("client_target");
    let server = FakeServer::new(vec![]);
    let row = run(&fake, Some(&server), LAB_ROW).await;
    assert_eq!(row.result, RowResult::Pass, "{:?}", row.reasons);
    assert_eq!(row.vars["dummy_id"], 4242);
    let setup = row
        .actions
        .iter()
        .find(|a| a.requested == "uat_dummy")
        .unwrap();
    assert_eq!(setup.tier, Some(crate::uat::tier::Tier::G));
    let asked: Vec<Value> = server
        .calls
        .lock()
        .unwrap()
        .iter()
        .filter(|c| c.0 == "server_ability_state")
        .map(|c| c.1["entity_id"].clone())
        .collect();
    assert_eq!(asked, vec![json!(4242), json!(7)]);
    // The clear targeted the lab character before typing.
    let log = fake.log.lock().unwrap();
    let target = log.iter().position(|c| c.0 == "client_target").unwrap();
    assert_eq!(log[target].1["name"], "Labone");
    let typed = log
        .iter()
        .position(|c| c.0 == "client_type_text" && c.1["text"] == ".cleareffects")
        .unwrap();
    assert!(target < typed);
    assert!(
        row.flags.iter().all(|f| !f.contains("teardown")),
        "{:?}",
        row.flags
    );
}

/// A refused setup command BLOCKs the row with the server's own words; a
/// teardown command with no reply, or whose target will not take, fails
/// without typing (and is flagged).
#[tokio::test]
async fn a_refused_or_silent_lab_command_fails() {
    let rows = r#"
[[row]]
id = "AB-U6"
title = "x"
expected = "x"
setup = [{ tool = "@cooldowns_reset", args = { ability_id = 597 } }]
step = [{ chat = ".help" }]
[[row.expect]]
id = "c"
text = "t"
source = "chat"
contains = "never"
"#;
    let fake = Fake::new(&[(
        ".cooldowns reset 597",
        &[".cooldowns reset: abilityId must be a positive integer (got 597x)"],
    )]);
    let row = run(&fake, None, rows).await;
    assert_eq!(row.result, RowResult::Blocked);
    assert!(
        row.reasons.iter().any(|r| r.contains("refused")),
        "{:?}",
        row.reasons
    );

    let rows = LAB_ROW.replace("${character}", "Nobody");
    let fake = Fake::new(&[(
        ".dummy friendly",
        &["dummy [5] placed: friendly SGC Jaffa (template 34)"],
    )])
    .with("client_target");
    let row = run(&fake, None, &rows).await;
    let td: Vec<_> = row
        .actions
        .iter()
        .filter(|a| a.role == crate::uat::tier::Role::Teardown)
        .collect();
    assert!(!td[0].ok && td[0].error.as_ref().unwrap().contains("nothing typed"));
    assert!(!td[1].ok && td[1].error.as_ref().unwrap().contains("no reply"));
    assert!(!fake
        .log
        .lock()
        .unwrap()
        .iter()
        .any(|c| c.1["text"] == ".cleareffects"));
}
