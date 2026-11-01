//! Corroboration against an injected fake server: each expectation's
//! verdict, and that the packet tap (which the UAT runner drains) is never
//! read.

use std::sync::Mutex;

use serde_json::{json, Value};

use super::{check, Expect};
use crate::proof::{Check, Source, Why};
use crate::uat::invoke::{BoxedOutcome, ServerInvoker, ToolOutcome};

/// Answers `server_entity_get` and `server_log_tail` with the scripted
/// replies (`None` is a failed call) and records every tool name it is asked.
struct Fake {
    entity: Option<Value>,
    log: Option<Value>,
    calls: Mutex<Vec<String>>,
}

impl Fake {
    fn new(entity: Option<Value>, log: Option<Value>) -> Self {
        Self {
            entity,
            log,
            calls: Mutex::new(vec![]),
        }
    }
}

fn outcome(reply: &Option<Value>) -> ToolOutcome {
    match reply {
        Some(json) => ToolOutcome {
            ok: true,
            json: json.clone(),
            ..Default::default()
        },
        None => ToolOutcome::err("lab-mcp HTTP 403 Forbidden: Host header is not allowed"),
    }
}

impl ServerInvoker for Fake {
    fn url(&self) -> &str {
        "http://127.0.0.1:1/mcp"
    }

    fn call<'a>(&'a self, name: &'a str, _args: Value) -> BoxedOutcome<'a> {
        self.calls.lock().unwrap().push(name.to_string());
        let out = match name {
            "server_entity_get" => outcome(&self.entity),
            "server_log_tail" => outcome(&self.log),
            _ => ToolOutcome::err("unexpected tool"),
        };
        Box::pin(async move { out })
    }
}

fn near(at: [f64; 3], within_m: f64) -> Expect {
    Expect::EntityPosNear {
        entity_id: 7,
        at,
        within_m,
    }
}

/// An entity reply with the given position.
fn entity_at(pos: [f64; 3]) -> Value {
    json!({ "entity": { "entity_id": 7, "position": pos } })
}

async fn run(fake: &Fake, e: &Expect) -> Check {
    check(Some(fake as &dyn ServerInvoker), "server_check", e, true).await
}

#[tokio::test]
async fn pos_near_within_radius_confirms() {
    let fake = Fake::new(Some(entity_at([1.0, 0.0, 0.0])), None);
    let c = run(&fake, &near([0.0, 0.0, 0.0], 2.0)).await;
    assert_eq!(c.ok, Some(true));
    assert_eq!(c.source, Source::Server);
    assert_eq!(c.why, None);
}

#[tokio::test]
async fn pos_far_is_server_disagrees() {
    let fake = Fake::new(Some(entity_at([5.0, 0.0, 0.0])), None);
    let c = run(&fake, &near([0.0, 0.0, 0.0], 2.0)).await;
    assert_eq!(c.ok, Some(false));
    assert_eq!(c.why, Some(Why::ServerDisagrees));
}

#[tokio::test]
async fn no_server_is_unreachable_not_false() {
    let c = check(None, "server_check", &near([0.0, 0.0, 0.0], 2.0), true).await;
    assert_eq!(c.ok, None);
    assert_eq!(c.why, Some(Why::ServerUnreachable));
}

#[tokio::test]
async fn failed_call_is_unreachable() {
    let fake = Fake::new(None, None);
    let c = run(&fake, &near([0.0, 0.0, 0.0], 2.0)).await;
    assert_eq!(c.ok, None);
    assert_eq!(c.why, Some(Why::ServerUnreachable));
}

#[tokio::test]
async fn gone_entity_is_server_disagrees_not_unreachable() {
    let fake = Fake::new(Some(json!({ "entity": null })), None);
    let c = run(&fake, &near([0.0, 0.0, 0.0], 2.0)).await;
    assert_eq!(c.ok, Some(false));
    assert_eq!(c.why, Some(Why::ServerDisagrees));
}

#[tokio::test]
async fn entity_field_compares_the_pointer() {
    let reply = json!({ "entity": { "space_id": 12 } });
    let fake = Fake::new(Some(reply), None);
    let hit = Expect::EntityField {
        entity_id: 7,
        pointer: "/entity/space_id".into(),
        equals: json!(12),
    };
    let miss = Expect::EntityField {
        entity_id: 7,
        pointer: "/entity/space_id".into(),
        equals: json!(13),
    };
    assert_eq!(run(&fake, &hit).await.ok, Some(true));
    let c = run(&fake, &miss).await;
    assert_eq!(c.ok, Some(false));
    assert_eq!(c.why, Some(Why::ServerDisagrees));
}

#[tokio::test]
async fn log_line_found_and_missing() {
    let e = Expect::LogLine {
        contains: "moved".into(),
        since_ms: 100,
    };
    let found = Fake::new(None, Some(json!({ "entries": [{ "message": "moved" }] })));
    assert_eq!(run(&found, &e).await.ok, Some(true));
    let missing = Fake::new(None, Some(json!({ "entries": [] })));
    let c = run(&missing, &e).await;
    assert_eq!(c.ok, Some(false));
    assert_eq!(c.why, Some(Why::ServerDisagrees));
}

#[tokio::test]
async fn never_reads_the_packet_tap() {
    let fake = Fake::new(
        Some(entity_at([0.0, 0.0, 0.0])),
        Some(json!({ "entries": [{ "message": "x" }] })),
    );
    let expects = [
        near([0.0, 0.0, 0.0], 2.0),
        Expect::EntityField {
            entity_id: 7,
            pointer: "/entity/space_id".into(),
            equals: json!(1),
        },
        Expect::LogLine {
            contains: "x".into(),
            since_ms: 0,
        },
        near([9.0, 9.0, 9.0], 1.0),
    ];
    for e in &expects {
        run(&fake, e).await;
    }
    let calls = fake.calls.lock().unwrap().clone();
    assert_eq!(calls.len(), 4);
    assert!(calls.iter().all(|n| n != "server_packet_tap_read"));
}
