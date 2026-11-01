//! AX-12: the runner reads the `evidence` verdict of an action's result.

use serde_json::{json, Value};

use super::super::tests::{request, Fake};
use super::super::Runner;
use crate::uat::evidence::{RowEvidence, RowResult, RunDir};

const TOOL: &str = "client_world_click";

fn evidence(verdict: &str, why: &str) -> Value {
    json!({ "evidence": {
        "verdict": verdict, "why": why, "hint": "see the hint", "checks": [],
    } })
}

const CLAUSE: &str = r#"
[[row.expect]]
id = "c"
text = "t"
source = "wait"
lua_condition = "true"
"#;

/// A passing clause that is not required: it proves nothing for the rule.
const OPTIONAL_CLAUSE: &str = r#"
[[row.expect]]
id = "o"
text = "t"
source = "wait"
lua_condition = "true"
required = false
"#;

fn row(with_clause: bool, teardown: bool) -> String {
    format!(
        "\n[[row]]\nid = \"V1\"\ntitle = \"click\"\nexpected = \"it clicks\"\n\
         step = [{{ tool = \"{TOOL}\", args = {{}} }}]\n{}{}",
        if teardown {
            format!("teardown = [{{ tool = \"{TOOL}\", args = {{}} }}]\n")
        } else {
            String::new()
        },
        if with_clause { CLAUSE } else { OPTIONAL_CLAUSE }
    )
}

async fn run(reply: Value, rows: &str) -> RowEvidence {
    let fake = Fake::new(&[]).with(TOOL).answer(TOOL, vec![reply]);
    let tmp = tempfile::tempdir().unwrap().keep();
    let runner = Runner::new(&fake, None, request(&tmp, rows)).unwrap();
    let out = runner.run_all().await.unwrap();
    let run = RunDir::open(std::path::Path::new(&out.run_dir)).unwrap();
    run.rows().unwrap().remove(0)
}

fn flagged(r: &RowEvidence) -> bool {
    r.flags.iter().any(|f| f.starts_with("inconclusive:"))
}

#[tokio::test]
async fn a_denied_step_fails_its_action() {
    let r = run(evidence("denied", "no_hover"), &row(true, false)).await;
    assert_eq!(r.result, RowResult::Fail, "{:?}", r.reasons);
    let step = r
        .actions
        .iter()
        .find(|a| a.tool.as_deref() == Some(TOOL))
        .unwrap();
    assert!(!step.ok);
    assert!(
        step.error.as_deref().unwrap().contains("denied: no_hover"),
        "{:?}",
        step.error
    );
}

#[tokio::test]
async fn an_inconclusive_step_without_a_passing_clause_is_unverified() {
    let r = run(evidence("inconclusive", "no_post_read"), &row(false, false)).await;
    assert_eq!(r.result, RowResult::Unverified, "{:?}", r.reasons);
    assert!(
        r.flags.contains(&format!(
            "inconclusive: {TOOL}: inconclusive (no_post_read)"
        )),
        "{:?}",
        r.flags
    );
}

#[tokio::test]
async fn an_inconclusive_step_with_a_passing_required_clause_keeps_pass() {
    let r = run(evidence("inconclusive", "no_post_read"), &row(true, false)).await;
    assert_eq!(r.result, RowResult::Pass, "{:?}", r.reasons);
    assert!(flagged(&r), "{:?}", r.flags);
}

#[tokio::test]
async fn an_unjudged_step_is_inconclusive_when_strict() {
    let r = run(evidence("unjudged", "not_yet_judged"), &row(false, false)).await;
    assert_eq!(r.result, RowResult::Unverified, "{:?}", r.reasons);
    assert!(
        r.flags.contains(&format!("inconclusive: {TOOL}: unjudged")),
        "{:?}",
        r.flags
    );
}

#[tokio::test]
async fn teardown_inconclusive_is_ignored() {
    // The step answers nothing judged; only the teardown call would carry
    // the verdict, and the fake answers every call to the tool alike, so
    // script the step's reply first and the teardown's second.
    let fake = Fake::new(&[]).with(TOOL).answer(
        TOOL,
        vec![json!({}), evidence("inconclusive", "no_post_read")],
    );
    let tmp = tempfile::tempdir().unwrap().keep();
    let runner = Runner::new(&fake, None, request(&tmp, &row(true, true))).unwrap();
    let out = runner.run_all().await.unwrap();
    let r = RunDir::open(std::path::Path::new(&out.run_dir))
        .unwrap()
        .rows()
        .unwrap()
        .remove(0);
    assert_eq!(r.result, RowResult::Pass, "{:?}", r.reasons);
    assert!(!flagged(&r), "{:?}", r.flags);
}
