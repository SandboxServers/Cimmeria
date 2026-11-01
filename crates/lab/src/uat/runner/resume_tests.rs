//! AX-16 guards: `resume_from` picks the start row and skips the rest.

use super::{start_index, validate};
use crate::uat::evidence::{RowResult, RunDir};
use crate::uat::runner::tests::{request, Fake};
use crate::uat::runner::{RunRequest, Runner};

fn keys(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| s.to_string()).collect()
}

#[test]
fn auto_starts_at_first_missing_row() {
    let order = keys(&["a/1", "a/2", "a/3"]);
    assert_eq!(start_index(&order, &keys(&["a/1"]), "auto"), Ok(1));
}

#[test]
fn auto_with_all_done_runs_nothing() {
    let order = keys(&["a/1", "a/2", "a/3"]);
    let done = keys(&["a/1", "a/2", "a/3"]);
    assert_eq!(start_index(&order, &done, "auto"), Ok(3));
}

#[test]
fn explicit_row_and_unknown_row() {
    let order = keys(&["a/1", "a/2", "a/3"]);
    assert_eq!(start_index(&order, &[], "a/2"), Ok(1));
    assert_eq!(
        start_index(&order, &[], "a/9"),
        Err("resume_from: no row a/9 in the selected sections".to_string())
    );
}

#[test]
fn resume_needs_run_dir_and_excludes_rows() {
    let mut req = RunRequest {
        resume_from: Some("auto".into()),
        ..Default::default()
    };
    assert_eq!(validate(&req), Err("resume_from needs run_dir".to_string()));
    req.run_dir = Some("x".into());
    req.rows = Some(vec!["R1".into()]);
    assert_eq!(
        validate(&req),
        Err("resume_from and rows are exclusive".to_string())
    );
    req.rows = None;
    assert_eq!(validate(&req), Ok(()));
}

fn row(id: &str, line: &str, reply: &str) -> String {
    format!(
        r#"
[[row]]
id = "{id}"
title = "{id}"
expected = "A reply appears."
anchor = false
step = [{{ chat = "{line}" }}]
[[row.expect]]
id = "reply"
text = "replies"
source = "chat"
contains = "{reply}"
"#
    )
}

#[test]
fn new_rejects_resume_without_run_dir_before_creating_a_bundle() {
    let root = tempfile::tempdir().unwrap();
    let fake = Fake::new(&[]);
    let mut req = request(root.path(), &row("R1", ".one", "ONE"));
    req.resume_from = Some("auto".into());
    let err = Runner::new(&fake, None, req).err().unwrap();
    assert_eq!(err, "resume_from needs run_dir");
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
}

#[tokio::test]
async fn a_resumed_run_skips_earlier_rows() {
    let rows = format!("{}{}", row("R1", ".one", "ONE"), row("R2", ".two", "TWO"));
    let root = tempfile::tempdir().unwrap();
    // First pass: the server answers only row 1, so row 2 fails.
    let first_fake = Fake::new(&[(".one", &["ONE"])]);
    let out = Runner::new(&first_fake, None, request(root.path(), &rows))
        .unwrap()
        .run_all()
        .await
        .unwrap();
    assert_eq!(out.rows[0].result, "PASS", "{:?}", out.rows[0].reasons);
    assert_ne!(out.rows[1].result, "PASS");
    let run_dir = std::path::PathBuf::from(&out.run_dir);
    let before =
        std::fs::read_to_string(RunDir::open(&run_dir).unwrap().row_json("gm-parity", "R1"))
            .unwrap();

    // Second pass: resume at row 2, now answered.
    let fake = Fake::new(&[(".two", &["TWO"])]);
    let mut req = request(root.path(), &rows);
    req.run_dir = Some(run_dir.clone());
    req.resume_from = Some("gm-parity/R2".into());
    let out = Runner::new(&fake, None, req)
        .unwrap()
        .run_all()
        .await
        .unwrap();
    assert_eq!(out.rows.len(), 1);
    assert_eq!(out.rows[0].row, "R2");
    assert_eq!(out.rows[0].result, "PASS", "{:?}", out.rows[0].reasons);
    let typed: Vec<String> = fake
        .log
        .lock()
        .unwrap()
        .iter()
        .filter(|(n, _)| n == "client_type_text")
        .map(|(_, a)| a["text"].as_str().unwrap_or_default().to_string())
        .collect();
    assert!(typed.contains(&".two".to_string()), "{typed:?}");
    assert!(
        !typed.contains(&".one".to_string()),
        "row 1 re-ran: {typed:?}"
    );
    let run = RunDir::open(&run_dir).unwrap();
    let after = std::fs::read_to_string(run.row_json("gm-parity", "R1")).unwrap();
    assert_eq!(before, after, "row 1 evidence must be untouched");
    let rows = run.rows().unwrap();
    assert!(rows
        .iter()
        .all(|r| r.result == RowResult::Pass || r.row_id != "R1"));
}
