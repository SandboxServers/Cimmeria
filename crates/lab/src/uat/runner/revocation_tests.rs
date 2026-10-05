//! A lost lease stops the run (review 2026-10-04).

use std::time::Duration;

use tokio::sync::watch;

use super::tests::{request, Fake};
use super::Runner;
use crate::uat::evidence::{RowResult, RunDir};

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
