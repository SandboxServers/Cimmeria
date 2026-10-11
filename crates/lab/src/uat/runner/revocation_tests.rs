//! A lost lease stops the run (review 2026-10-04).

use std::time::Duration;

use serde_json::{json, Value};
use tokio::sync::watch;

use super::test_server::FakeServer;
use super::tests::{request, Fake};
use super::Runner;
use crate::uat::evidence::{RowResult, RunDir};
use crate::uat::invoke::ServerInvoker;

const ROWS: &str = r#"
[[row]]
id = "R1"
title = "long"
expected = "Answers, then a long wait."
step = [{ chat = ".help", label = "help" }, { wait_ms = 60000 }]
[[row.expect]]
id = "help-answers"
text = ".help lists commands"
source = "chat"
contains = "Available commands"

[[row]]
id = "R2"
title = "after"
expected = "Answers."
step = [{ chat = ".who", label = "who" }]
[[row.expect]]
id = "who-answers"
text = ".who lists players"
source = "chat"
contains = "Players"
"#;

/// Regression guard: a takeover during a row's 60 s `wait_ms` stops the run
/// at once. That row and every later one are BLOCKED "lease revoked", and
/// the later row drives nothing (before the fix the run slept out the wait
/// and went on to type `.who` under someone else's lease).
#[tokio::test]
async fn a_takeover_stops_the_run_and_blocks_every_remaining_row() {
    let fake = Fake::new(&[
        (".help", &["Available commands: .help"]),
        (".who", &["Players online: 1"]),
    ]);
    let tmp = tempfile::tempdir().unwrap().keep();
    let (tx, rx) = watch::channel(None::<String>);
    let runner = Runner::new(&fake, None, request(&tmp, ROWS))
        .unwrap()
        .with_revocation(rx);
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(300)).await;
        let _ = tx.send(Some("lease taken over by session-b".into()));
    });
    let out = tokio::time::timeout(Duration::from_secs(10), runner.run_all())
        .await
        .expect("the run stopped instead of sleeping out the wait")
        .unwrap();

    let run = RunDir::open(std::path::Path::new(&out.run_dir)).unwrap();
    let rows = run.rows().unwrap();
    assert_eq!(rows.len(), 2);
    for r in &rows {
        assert_eq!(r.result, RowResult::Blocked, "{} {:?}", r.row_id, r.reasons);
        assert!(
            r.reasons.iter().any(|x| x.contains("lease revoked")),
            "{} {:?}",
            r.row_id,
            r.reasons
        );
    }
    let typed_who = fake
        .log
        .lock()
        .unwrap()
        .iter()
        .any(|(_, args)| args.to_string().contains(".who"));
    assert!(!typed_who, "R2 was driven after the lease was lost");
}

/// A row tapped from its start, then a 60 s wait the takeover cuts short.
const TAPPED: &str = r#"
[[row]]
id = "T1"
title = "tapped"
expected = "Waits."
anchor = false
step = [{ tool = "client_wait_for", tier = "N1", label = "ready" }, { wait_ms = 60000 }]
[[row.expect]]
id = "waited"
text = "the wait is met"
source = "wait"
lua_condition = "true"
"#;

/// The same row with a packet clause, so `tap_start` owns its tap.
const TAPPED_PACKET: &str = r#"
[[row.expect]]
id = "timer"
text = "a timer"
source = "packet"
message = "onTimerUpdate"
direction = "to_client"
min_rows = 1
"#;

/// Regression guard (GD-06 review): a takeover mid-row stops the row's
/// packet tap (the lab MCP is not lease-gated) and still writes the
/// fingerprint attachments. Before the fix the tap kept running, so the
/// next run's start reported `replaced_existing` and a golden run refused
/// it, and the row lost its windows, entities and packet-tap files.
#[tokio::test]
async fn a_takeover_stops_the_rows_tap_and_keeps_its_capture() {
    let packet_rows = format!("{TAPPED}{TAPPED_PACKET}");
    for (rows, fingerprint) in [(TAPPED, true), (packet_rows.as_str(), false)] {
        let fake = Fake::new(&[]);
        let server = FakeServer::new(json!({ "messages": [] }));
        let tmp = tempfile::tempdir().unwrap().keep();
        let mut req = request(&tmp, rows);
        req.fingerprint = fingerprint;
        let (tx, rx) = watch::channel(None::<String>);
        let runner = Runner::new(&fake, Some(&server as &dyn ServerInvoker), req)
            .unwrap()
            .with_revocation(rx);
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(300)).await;
            let _ = tx.send(Some("lease taken over by session-b".into()));
        });
        let out = tokio::time::timeout(Duration::from_secs(10), runner.run_all())
            .await
            .expect("the run stopped instead of sleeping out the wait")
            .unwrap();

        let run = RunDir::open(std::path::Path::new(&out.run_dir)).unwrap();
        let row = run.rows().unwrap().remove(0);
        assert_eq!(row.result, RowResult::Blocked, "{:?}", row.reasons);
        let names = server.names();
        assert_eq!(
            names.last().map(String::as_str),
            Some("server_packet_tap_stop"),
            "{names:?}"
        );
        let attached = |name: &str| -> Option<Value> {
            let a = row.attachments.iter().find(|a| a.name == name)?;
            let text = std::fs::read_to_string(run.root.join(&a.path)).unwrap();
            Some(serde_json::from_str(&text).unwrap())
        };
        let tap = attached("packet_tap").expect("packet-tap.json");
        assert_eq!(tap["stopped"], true, "{tap}");
        assert_eq!(tap["read_ok"], false, "{tap}");
        let started =
            |a: &&crate::uat::evidence::ActionRecord| a.requested == "server_packet_tap_start";
        assert_eq!(row.actions.iter().filter(started).count(), 1);
        if fingerprint {
            assert_eq!(tap["coverage"], "full", "{tap}");
            assert!(attached("windows").is_some());
            let e = attached("entities").expect("entities.json");
            assert_eq!(e["snapshots"][0]["at"], "bind", "{e}");
        } else {
            assert!(tap.get("coverage").is_none(), "{tap}");
        }
    }
}
