//! Event-store flows against a fake bridge: `client_wait_event` cursor
//! semantics end to end, `client_events_read` not stealing from waits, and
//! the combat log's Lua-ring pump (seq marks, epoch change).

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde_json::{json, Value};

use super::fake_bridge::{self, empty_rings, events, is_ring_pump, lua_ok, Responder};
use super::predicate::EventPredicate;
use super::wait::WaitRequest;
use crate::supervisor::combat::combat_log::CombatLogRequest;

/// A fake whose `events_read` hands out scripted batches in order (then
/// nothing), whose ring pump is empty, and whose other Lua answers `vis`.
fn scripted(batches: Vec<Value>, vis: &'static str) -> (Responder, Arc<Mutex<u32>>) {
    let q = Arc::new(Mutex::new(VecDeque::from(batches)));
    let reads = Arc::new(Mutex::new(0u32));
    let r2 = reads.clone();
    let responder: Responder = Arc::new(move |method, params| match method {
        "events_read" => {
            *r2.lock().unwrap() += 1;
            Ok(q.lock()
                .unwrap()
                .pop_front()
                .unwrap_or_else(|| events(vec![])))
        }
        "lua_eval" if is_ring_pump(params) => Ok(empty_rings()),
        "lua_eval" => Ok(lua_ok(&[vis])),
        other => Err(format!("unexpected {other}")),
    });
    (responder, reads)
}

fn seq_ev(name: &str, t: i64) -> (&'static str, i64, Value) {
    ("cme.event", t, json!({ "event": name, "kind": "net_in" }))
}

fn wait(name: Option<&str>, cursor: &str, timeout_ms: u64) -> WaitRequest {
    WaitRequest {
        predicate: EventPredicate {
            kind: Some("cme.event".into()),
            name: name.map(str::to_string),
            ..Default::default()
        },
        window: None,
        cursor: cursor.into(),
        since_seq: None,
        count: 1,
        arm: false,
        timeout: Duration::from_millis(timeout_ms),
        poll: Duration::from_millis(20),
    }
}

/// Arm, then two waits for the same event name on one cursor. Both events
/// arrive in one bridge batch during the first wait; the second wait still
/// finds the second one although no new event arrives after it started.
/// With a drain-and-clear reader it would have been consumed and lost.
#[tokio::test]
async fn successive_waits_on_one_cursor_find_successive_events() {
    let (r, _) = scripted(
        vec![
            events(vec![]),
            events(vec![
                seq_ev("Event_NetIn_onSequence", 10),
                seq_ev("Event_NetIn_onSequence", 11),
            ]),
        ],
        "false",
    );
    let sup = fake_bridge::supervisor(r).await;
    let mut arm = wait(None, "c", 0);
    arm.arm = true;
    let a = sup.wait_event(arm).await.unwrap();
    assert_eq!(a["armed"], true);
    assert_eq!(a["cursor"]["seq"], 0);

    let w1 = sup
        .wait_event(wait(Some("*onSequence"), "c", 2000))
        .await
        .unwrap();
    assert_eq!(w1["met"], true);
    assert_eq!(w1["matched"][0]["seq"], 1);
    assert_eq!(w1["cursor"]["after"], 1);

    let w2 = sup
        .wait_event(wait(Some("*onSequence"), "c", 2000))
        .await
        .unwrap();
    assert_eq!(w2["met"], true, "{w2}");
    assert_eq!(w2["matched"][0]["seq"], 2);
    assert_eq!(w2["matched"][0]["ts_ms"], 11);
}

/// A timeout is `met: false`, not an error, and leaves the cursor: a later
/// wait for something else still sees the event the first one scanned.
#[tokio::test]
async fn a_timeout_leaves_the_cursor_for_the_next_predicate() {
    let (r, _) = scripted(
        vec![
            events(vec![]),
            events(vec![seq_ev("Event_NetIn_onTimerUpdate", 5)]),
        ],
        "false",
    );
    let sup = fake_bridge::supervisor(r).await;
    let mut arm = wait(None, "c", 0);
    arm.arm = true;
    sup.wait_event(arm).await.unwrap();
    let miss = sup
        .wait_event(wait(Some("*onErrorCode"), "c", 150))
        .await
        .unwrap();
    assert_eq!(miss["met"], false);
    assert_eq!(miss["matched_by"], "none");
    assert_eq!(miss["cursor"]["after"], 0);
    assert!(miss["scanned"].as_u64().unwrap() >= 1);
    let hit = sup
        .wait_event(wait(Some("*onTimerUpdate"), "c", 500))
        .await
        .unwrap();
    assert_eq!(hit["met"], true);
}

/// A fresh cursor starts at the newest event: backlog that was in the
/// bridge ring before the wait does not satisfy it.
#[tokio::test]
async fn a_fresh_cursor_ignores_stale_backlog() {
    let (r, _) = scripted(
        vec![events(vec![seq_ev("Event_NetIn_onSequence", 1)])],
        "false",
    );
    let sup = fake_bridge::supervisor(r).await;
    let w = sup
        .wait_event(wait(Some("*onSequence"), "fresh", 150))
        .await
        .unwrap();
    assert_eq!(w["met"], false, "stale backlog matched: {w}");
    assert_eq!(w["cursor"]["start"], 1);
    // An explicit since_seq replays it.
    let mut replay = wait(Some("*onSequence"), "fresh", 500);
    replay.since_seq = Some(0);
    assert_eq!(sup.wait_event(replay).await.unwrap()["met"], true);
}

/// `client_events_read` reads through its own cursor: it returns each
/// event once, and a wait afterwards still finds the same events.
#[tokio::test]
async fn events_read_does_not_steal_from_waits() {
    let (r, reads) = scripted(
        vec![
            events(vec![]),
            events(vec![seq_ev("Event_NetIn_onEffectResults", 3)]),
        ],
        "false",
    );
    let sup = fake_bridge::supervisor(r).await;
    let mut arm = wait(None, "c", 0);
    arm.arm = true;
    sup.wait_event(arm).await.unwrap();
    let first = sup.events_read(None).await.unwrap();
    assert_eq!(first["returned"], 1);
    assert_eq!(first["events"][0]["seq"], 1);
    assert_eq!(sup.events_read(None).await.unwrap()["returned"], 0);
    let w = sup
        .wait_event(wait(Some("*onEffectResults"), "c", 500))
        .await
        .unwrap();
    assert_eq!(w["met"], true);
    assert!(*reads.lock().unwrap() >= 3);
}

/// Several matches: `count` waits for all of them.
#[tokio::test]
async fn count_waits_for_n_matches() {
    let (r, _) = scripted(
        vec![
            events(vec![]),
            events(vec![seq_ev("Event_NetIn_onSequence", 1)]),
            events(vec![]),
            events(vec![seq_ev("Event_NetIn_onSequence", 2)]),
        ],
        "false",
    );
    let sup = fake_bridge::supervisor(r).await;
    let mut arm = wait(None, "n", 0);
    arm.arm = true;
    sup.wait_event(arm).await.unwrap();
    let mut two = wait(Some("*onSequence"), "n", 2000);
    two.count = 2;
    let w = sup.wait_event(two).await.unwrap();
    assert_eq!(w["met"], true);
    assert_eq!(w["count"]["got"], 2);
    assert!(w["polls"].as_u64().unwrap() >= 2);
}

/// A window predicate is level-triggered through a Lua visibility read.
#[tokio::test]
async fn a_visible_window_meets_the_wait() {
    let (r, _) = scripted(vec![], "true");
    let sup = fake_bridge::supervisor(r).await;
    let mut w = wait(Some("never"), "win", 1000);
    w.window = Some("PlayerDefeatWin".into());
    let out = sup.wait_event(w).await.unwrap();
    assert_eq!(out["met"], true);
    assert_eq!(out["matched_by"], "window");
}

/// A bridge that is down is an error, not a timeout.
#[tokio::test]
async fn a_dead_bridge_is_an_error() {
    let r: Responder = Arc::new(|_, _| Err("boom".into()));
    let sup = fake_bridge::supervisor(r).await;
    assert!(sup
        .wait_event(wait(Some("x"), "c", 100))
        .await
        .unwrap_err()
        .contains("boom"));
}

// ---- combat log over the Lua ring --------------------------------------

/// The seq the pump chunk asks for (`e.seq > N and n <`).
fn asked_after(chunk: &str) -> u64 {
    let i = chunk.find("e.seq > ").expect("combat filter") + "e.seq > ".len();
    chunk[i..]
        .split_whitespace()
        .next()
        .and_then(|n| n.parse().ok())
        .expect("seq")
}

fn combat_line(seq: u64, dmg: i64) -> String {
    format!(
        "combat\t{seq}\t0\t1100\tPistol Shot\t1\tHit\tLabone\t1\tGuard\t0\t0\t1\u{1f}Health\u{1f}{dmg}\u{1f}0\u{1f}\u{1f}0"
    )
}

/// A fake Lua ring: `(epoch, records)` that the test can grow or replace.
fn ring_fake(state: Arc<Mutex<(String, Vec<u64>)>>) -> Responder {
    Arc::new(move |method, params| match method {
        "events_read" => Ok(events(vec![])),
        "lua_eval" if is_ring_pump(params) => {
            let chunk = params["chunk"].as_str().unwrap();
            let after = asked_after(chunk);
            let (epoch, seqs) = state.lock().unwrap().clone();
            let mut lines = vec![
                format!("epoch\t{epoch}"),
                "install\tcombat\tinstalled".into(),
                "install\tchat\tok".into(),
            ];
            lines.extend(
                seqs.iter()
                    .filter(|s| **s > after)
                    .map(|s| combat_line(*s, -10)),
            );
            lines.push(format!(
                "head\tcombat\t{}",
                seqs.last().copied().unwrap_or(0)
            ));
            lines.push("head\tchat\t0".into());
            Ok(lua_ok(&lines))
        }
        other => Err(format!("unexpected {other}")),
    })
}

#[tokio::test]
async fn combat_log_reads_each_record_once_and_survives_a_ui_reload() {
    let state = Arc::new(Mutex::new(("E1".to_string(), vec![1, 2])));
    let sup = fake_bridge::supervisor(ring_fake(state.clone())).await;

    let first = sup.combat_log(CombatLogRequest::default()).await.unwrap();
    assert_eq!(first["events"].as_array().unwrap().len(), 2);
    assert_eq!(first["summary"]["stat_totals"]["dealt.Health"], -20.0);
    assert_eq!(first["capture"]["status"], "installed");

    // Nothing new: nothing returned, even though the ring still holds both.
    let again = sup.combat_log(CombatLogRequest::default()).await.unwrap();
    assert!(again["events"].as_array().unwrap().is_empty());

    // Peek does not move the cursor.
    state.lock().unwrap().1.push(3);
    let peek = sup
        .combat_log(CombatLogRequest {
            peek: true,
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(peek["events"].as_array().unwrap().len(), 1);
    let read = sup.combat_log(CombatLogRequest::default()).await.unwrap();
    assert_eq!(read["events"].as_array().unwrap().len(), 1);
    assert_eq!(read["events"][0]["fields"]["ring_seq"], 3);

    // Interface reload: a new Lua state, its ring restarts at 1.
    *state.lock().unwrap() = ("E2".to_string(), vec![1]);
    let after_reload = sup.combat_log(CombatLogRequest::default()).await.unwrap();
    assert_eq!(after_reload["capture"]["lua_epoch_changed"], true);
    assert_eq!(
        after_reload["events"].as_array().unwrap().len(),
        1,
        "the new ring's first record is new: {after_reload}"
    );
}

#[tokio::test]
async fn independent_combat_cursors_do_not_interfere() {
    let state = Arc::new(Mutex::new(("E1".to_string(), vec![1, 2, 3])));
    let sup = fake_bridge::supervisor(ring_fake(state)).await;
    let a = sup
        .combat_log(CombatLogRequest {
            cursor: Some("a".into()),
            max: Some(2),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(a["events"].as_array().unwrap().len(), 2);
    assert_eq!(a["truncated"], true);
    let b = sup
        .combat_log(CombatLogRequest {
            cursor: Some("b".into()),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(b["events"].as_array().unwrap().len(), 3);
    let a2 = sup
        .combat_log(CombatLogRequest {
            cursor: Some("a".into()),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(
        a2["events"].as_array().unwrap().len(),
        1,
        "a resumes after its last record"
    );
}
