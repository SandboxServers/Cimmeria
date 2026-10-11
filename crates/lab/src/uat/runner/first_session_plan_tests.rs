//! SV-13: the committed first-session section plans against today's tools.
//! Its interaction rows stand at tagged poses and its Play rows wait on the
//! server's hold-release line, so without the server lab MCP exactly those
//! rows are BLOCKED naming it, and with one every row plans as ready. Fails
//! if a migrated row needs a tool nobody routes, or loses its pose or wait.

use super::test_server::FakeServer;
use super::tests::{Fake, MAIN_TOOLS};
use super::{RunOutcome, RunRequest, Runner};
use crate::uat::invoke::ServerInvoker;

/// The rows SV-13 moved onto poses or `wait_server`, sorted. FS-P5 equips
/// by name: no pose, no wait.
const SERVER_ROWS: [&str; 8] = [
    "FS-P2", "FS-P3", "FS-P4", "FS-S2", "FS-S3", "FS-S4", "FS-S5", "FS-S6",
];

async fn plan(fake: &Fake, server: Option<&FakeServer>) -> RunOutcome {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/guides/uat-specs");
    let mut sections = crate::uat::load_sections(&dir, None).unwrap();
    sections.retain(|s| s.spec.section.id == "first-session");
    assert_eq!(sections.len(), 1, "first-session.toml is committed");
    let req = RunRequest {
        sections,
        root: tempfile::tempdir().unwrap().keep(),
        lab_character: Some("Labone".into()),
        plan_only: true,
        no_settle: true,
        ..Default::default()
    };
    Runner::new(fake, server.map(|s| s as &dyn ServerInvoker), req)
        .unwrap()
        .run_all()
        .await
        .unwrap()
}

fn main_tools() -> Fake {
    let mut fake = Fake::new(&[]);
    fake.tools = MAIN_TOOLS.split_whitespace().map(str::to_string).collect();
    fake
}

#[tokio::test]
async fn first_session_needs_the_server_for_poses_and_waits() {
    // The spec itself says which rows carry a pose or a server wait.
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/guides/uat-specs");
    let sections = crate::uat::load_sections(&dir, None).unwrap();
    let spec = &sections
        .iter()
        .find(|s| s.spec.section.id == "first-session")
        .unwrap()
        .spec;
    let mut needs_server: Vec<&str> = spec
        .rows
        .iter()
        .filter(|r| {
            !r.pose.is_empty()
                || r.setup
                    .iter()
                    .chain(&r.steps)
                    .chain(&r.teardown)
                    .any(|a| a.wait_server.is_some())
        })
        .map(|r| r.id.as_str())
        .collect();
    needs_server.sort_unstable();
    assert_eq!(needs_server, SERVER_ROWS);

    let fake = main_tools();
    let out = plan(&fake, None).await;
    assert_eq!(out.rows.len(), 14, "the section lost or gained rows");
    for r in &out.rows {
        if needs_server.contains(&r.row.as_str()) {
            assert_eq!(r.result, "BLOCKED", "{}: {:?}", r.row, r.reasons);
            assert!(
                r.reasons.iter().any(|x| x.contains("server lab MCP")),
                "{}: {:?}",
                r.row,
                r.reasons
            );
        } else {
            assert_eq!(r.result, "SKIPPED", "{}: {:?}", r.row, r.reasons);
        }
    }
    assert!(fake.calls.lock().unwrap().is_empty());
}

#[tokio::test]
async fn first_session_plans_ready_with_a_server() {
    let fake = main_tools();
    let server = FakeServer::new(serde_json::json!({}));
    let out = plan(&fake, Some(&server)).await;
    assert_eq!(out.rows.len(), 14, "the section lost or gained rows");
    // The server rows and every other: nothing needs an unrouted tool.
    for r in &out.rows {
        assert_eq!(r.result, "SKIPPED", "{}: {:?}", r.row, r.reasons);
    }
    assert!(fake.calls.lock().unwrap().is_empty());
}
