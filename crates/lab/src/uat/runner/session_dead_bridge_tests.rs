//! DR-07k: a runner that starts with no record of who is in world must not
//! log out a client that already holds the section's character (the logout
//! crashes SGW.exe, #1680), and a dead bridge is restarted, never logged
//! out through.

use serde_json::{json, Value};

use super::tests::Fake;
use super::{LoadedSpec, RunRequest, Runner};
use crate::uat::spec;

const HELP: &str = r#"
[[row]]
id = "W01"
title = "help"
expected = "A reply appears."
anchor = false
step = [{ chat = ".help" }]
[[row.expect]]
id = "reply"
text = "help replies"
source = "chat"
contains = "Available commands"
"#;

fn fresh_section() -> LoadedSpec {
    let text = format!(
        "schema = 1\n[section]\nid = \"praxis\"\nsystem = \"Cellblock\"\nguide = \"guide\"\n\
         ledger = \"ledger\"\ncharacter = \"fresh\"\nsequential = true\n\
         [section.fresh]\nalignment = \"praxis\"\narchetype = \"Commando\"\n\
         gender = \"male\"\nfinish_intro = false\n{HELP}"
    );
    LoadedSpec {
        path: "praxis.toml".into(),
        sha256: "test".into(),
        spec: spec::parse(&text).unwrap(),
    }
}

fn request(root: &std::path::Path) -> RunRequest {
    RunRequest {
        sections: vec![fresh_section()],
        root: root.to_path_buf(),
        lab_character: Some("Labone".into()),
        operator: "test".into(),
        no_settle: true,
        ..Default::default()
    }
}

fn base() -> Fake {
    Fake::new(&[(".help", &["Available commands"])])
        .with("lab_characters")
        .with("lab_ensure_character_slot")
        .with("lab_create_character")
        .with("lab_play_character")
        .with("lab_client_restart")
        .with("lab_login")
        .with("client_player_state")
}

/// The fresh character's name and the run dir that keeps it, from a first
/// runner that is then dropped: the next `lab_uat_run` call resumes it.
fn first_runner(req: &RunRequest) -> (String, std::path::PathBuf) {
    let probe = base();
    let first = Runner::new(&probe, None, req.clone()).unwrap();
    (first.fresh["praxis"].clone(), first.run.root.clone())
}

fn index(names: &[String], tool: &str) -> Option<usize> {
    names.iter().position(|n| n == tool)
}

fn status(login: &str, heartbeat: &str) -> Value {
    json!({ "running": true, "login_state": login, "heartbeat": { "state": heartbeat } })
}

#[tokio::test]
async fn a_new_runner_keeps_the_fresh_character_already_in_world() {
    let root = tempfile::tempdir().unwrap();
    let req = request(root.path());
    let (name, run_dir) = first_runner(&req);
    // The client is in world as the section's fresh character (listed as
    // `<first> <last>`), but this runner has no `in_world_as` record.
    let fake = base().named(&format!("Uat {name}"));
    let mut resumed = req;
    resumed.run_dir = Some(run_dir);
    let out = Runner::new(&fake, None, resumed)
        .unwrap()
        .run_all()
        .await
        .unwrap();
    assert_eq!(out.rows[0].result, "PASS", "{:?}", out.rows[0].reasons);
    let names = fake.names();
    // Only the row's own setup counts: the end-of-run logout is legitimate.
    let row_calls: Vec<_> = names
        .iter()
        .take_while(|n| *n != "client_type_text")
        .collect();
    assert!(
        !row_calls.iter().any(|n| *n == "lab_logout"),
        "logged out a client that already held {name}: {names:?}"
    );
    assert!(
        !names.contains(&"lab_play_character".to_string()),
        "{names:?}"
    );
}

#[tokio::test]
async fn a_different_character_in_world_is_still_logged_out() {
    let root = tempfile::tempdir().unwrap();
    let req = request(root.path());
    let (_, run_dir) = first_runner(&req);
    let fake = base().named("Somebodyelse");
    let mut resumed = req;
    resumed.run_dir = Some(run_dir);
    Runner::new(&fake, None, resumed)
        .unwrap()
        .run_all()
        .await
        .unwrap();
    let names = fake.names();
    let logout = index(&names, "lab_logout").expect("{names:?}");
    let play = index(&names, "lab_play_character").expect("{names:?}");
    assert!(logout < play, "{names:?}");
}

#[tokio::test]
async fn an_unreachable_bridge_is_restarted_and_never_logged_out() {
    let root = tempfile::tempdir().unwrap();
    let req = request(root.path());
    let (_, run_dir) = first_runner(&req);
    // The cached login state says in_world while the heartbeat is dead; the
    // later status reads (after the restart) are healthy.
    let fake = base().answer(
        "lab_client_status",
        vec![
            status("in_world", "unreachable"),
            status("character_select", "alive"),
        ],
    );
    let mut resumed = req;
    resumed.run_dir = Some(run_dir);
    Runner::new(&fake, None, resumed)
        .unwrap()
        .run_all()
        .await
        .unwrap();
    let names = fake.names();
    assert_eq!(
        names.iter().filter(|n| *n == "lab_client_restart").count(),
        1,
        "{names:?}"
    );
    let restart = index(&names, "lab_client_restart").unwrap();
    let play = index(&names, "lab_play_character").expect("{names:?}");
    assert!(restart < play, "{names:?}");
    assert!(
        !names[..play].iter().any(|n| n == "lab_logout"),
        "logout before the play: {names:?}"
    );
}

#[tokio::test]
async fn a_client_still_booting_with_a_dead_heartbeat_is_logged_in_not_restarted() {
    let root = tempfile::tempdir().unwrap();
    let req = request(root.path());
    let (_, run_dir) = first_runner(&req);
    // Launched earlier, the bridge is not up yet: no screens, no restart.
    let fake = base().answer(
        "lab_client_status",
        vec![status("not_started", "unreachable")],
    );
    let mut resumed = req;
    resumed.run_dir = Some(run_dir);
    Runner::new(&fake, None, resumed)
        .unwrap()
        .run_all()
        .await
        .unwrap();
    let names = fake.names();
    assert!(
        !names.iter().any(|n| n == "lab_client_restart"),
        "{names:?}"
    );
    let login = index(&names, "lab_login").expect("{names:?}");
    let play = index(&names, "lab_play_character").expect("{names:?}");
    assert!(login < play, "{names:?}");
}

#[tokio::test]
async fn a_bridge_that_dies_on_the_name_read_is_restarted() {
    let root = tempfile::tempdir().unwrap();
    let req = request(root.path());
    let (_, run_dir) = first_runner(&req);
    let fake = base().answer(
        "client_player_state",
        vec![json!({ "__error": "connect to bridge at 127.0.0.1:8770: refused" })],
    );
    let mut resumed = req;
    resumed.run_dir = Some(run_dir);
    Runner::new(&fake, None, resumed)
        .unwrap()
        .run_all()
        .await
        .unwrap();
    let names = fake.names();
    let restart = index(&names, "lab_client_restart").expect("{names:?}");
    let play = index(&names, "lab_play_character").expect("{names:?}");
    assert!(restart < play, "{names:?}");
    assert!(
        !names[..play].iter().any(|n| n == "lab_logout"),
        "logout before the play: {names:?}"
    );
}
