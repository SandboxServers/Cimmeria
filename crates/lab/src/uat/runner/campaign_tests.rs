//! AP-02 campaign lifecycle guards over the scripted tool router.

use serde_json::json;

use super::tests::Fake;
use super::{LoadedSpec, RunRequest, Runner};
use crate::uat::evidence::{RunDir, RunManifest};
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

fn section(id: &str, sequential: bool, fresh: bool, finish_intro: bool, rows: &str) -> LoadedSpec {
    let character = if fresh { "fresh" } else { "lab" };
    let fresh_table = if fresh {
        format!(
            "\n[section.fresh]\nalignment = \"praxis\"\narchetype = \"Commando\"\ngender = \"male\"\nfinish_intro = {finish_intro}\n"
        )
    } else {
        String::new()
    };
    let text = format!(
        "schema = 1\n[section]\nid = \"{id}\"\nsystem = \"Cellblock\"\nguide = \"guide\"\nledger = \"ledger\"\ncharacter = \"{character}\"\nsequential = {sequential}\n{fresh_table}{rows}"
    );
    LoadedSpec {
        path: format!("{id}.toml"),
        sha256: "test".into(),
        spec: spec::parse(&text).unwrap(),
    }
}

fn request(root: &std::path::Path, sections: Vec<LoadedSpec>) -> RunRequest {
    RunRequest {
        sections,
        root: root.to_path_buf(),
        lab_character: Some("Labone".into()),
        operator: "test".into(),
        no_settle: true,
        ..Default::default()
    }
}

fn fresh_fake() -> Fake {
    let fake = Fake::new(&[(".help", &["Available commands"])]);
    *fake.login.lock().unwrap() = "character_select".into();
    fake.with("lab_characters")
        .with("lab_ensure_character_slot")
        .with("lab_create_character")
        .with("lab_play_character")
        .with("lab_finish_dialog")
}

#[tokio::test]
async fn resume_adopts_a_character_created_before_interruption() {
    let root = tempfile::tempdir().unwrap();
    let spec = section("praxis", true, true, true, HELP);
    let req = request(root.path(), vec![spec.clone()]);
    let interrupted_fake = fresh_fake();
    let first = Runner::new(&interrupted_fake, None, req.clone()).unwrap();
    let run_dir = first.run.root.clone();
    let name = first.fresh["praxis"].clone();
    let manifest: RunManifest = first.run.read_json(&first.run.run_json()).unwrap();
    assert_eq!(
        manifest.fresh_characters["praxis"],
        json!({
            "name": name, "archetype": "Commando"
        })
    );
    drop(first); // The manifest survives an interruption after character creation.

    let fake = fresh_fake()
        .answer(
            "lab_characters",
            vec![json!({
                "characters": [{"name": format!("Uat {name}"), "archetype": "Commando"}]
            })],
        )
        .answer("lab_play_character", vec![json!({ "dialog_open": true })]);
    let mut resumed = req;
    resumed.run_dir = Some(run_dir.clone());
    let out = Runner::new(&fake, None, resumed)
        .unwrap()
        .run_all()
        .await
        .unwrap();
    assert_eq!(out.rows[0].result, "PASS", "{:?}", out.rows[0].reasons);
    let names = fake.names();
    assert!(names.contains(&"lab_play_character".to_string()));
    assert!(!names.contains(&"lab_create_character".to_string()));
    assert!(names.contains(&"lab_finish_dialog".to_string()));
    let rows = RunDir::open(&run_dir).unwrap().rows().unwrap();
    assert_eq!(rows[0].character["name"], name);
}

#[tokio::test]
async fn older_run_without_fresh_map_adopts_its_original_name() {
    let root = tempfile::tempdir().unwrap();
    let spec = section("praxis", true, true, true, HELP);
    let req = request(root.path(), vec![spec]);
    let first_fake = fresh_fake();
    let first = Runner::new(&first_fake, None, req.clone()).unwrap();
    let run_dir = first.run.root.clone();
    let old_name = super::session::legacy_name(&first.manifest.run_id);
    let mut old_manifest = serde_json::to_value(&first.manifest).unwrap();
    old_manifest
        .as_object_mut()
        .unwrap()
        .remove("fresh_characters");
    first
        .run
        .write_json(&first.run.run_json(), &old_manifest)
        .unwrap();
    drop(first);

    let fake = fresh_fake().answer(
        "lab_characters",
        vec![json!({
            "characters": [{"name": format!("Uat {old_name}"), "archetype": "Commando"}]
        })],
    );
    let mut resumed = req;
    resumed.run_dir = Some(run_dir);
    let out = Runner::new(&fake, None, resumed)
        .unwrap()
        .run_all()
        .await
        .unwrap();
    assert_eq!(out.rows[0].result, "PASS", "{:?}", out.rows[0].reasons);
    assert!(!fake.names().contains(&"lab_create_character".to_string()));
}

#[tokio::test]
async fn fail_blocks_following_sequential_rows_and_dependencies() {
    let rows = r#"
[[row]]
id = "W01"
title = "fail"
expected = "The tool succeeds."
anchor = false
step = [{ tool = "lab_fail", tier = "N1" }]
[[row.expect]]
id = "e1"
text = "help replies"
source = "chat"
contains = "Available commands"
[[row]]
id = "W02"
title = "next"
expected = "A reply appears."
anchor = false
step = [{ chat = ".help" }]
[[row.expect]]
id = "e2"
text = "help replies"
source = "chat"
contains = "Available commands"
[[row]]
id = "W03"
depends_on = ["W01"]
title = "dependent"
expected = "A reply appears."
anchor = false
step = [{ chat = ".help" }]
[[row.expect]]
id = "e3"
text = "help replies"
source = "chat"
contains = "Available commands"
"#;
    let root = tempfile::tempdir().unwrap();
    let fake = Fake::new(&[(".help", &["Available commands"])]).with("lab_fail");
    let out = Runner::new(
        &fake,
        None,
        request(root.path(), vec![section("p", true, false, true, rows)]),
    )
    .unwrap()
    .run_all()
    .await
    .unwrap();
    assert_eq!(out.rows[0].result, "FAIL");
    for row in &out.rows[1..] {
        assert_eq!(row.result, "BLOCKED", "{:?}", row.reasons);
        assert!(row.reasons.iter().any(|r| r == "prior row W01 = FAIL"));
    }
    assert!(!fake.names().contains(&"client_type_text".to_string()));
}

#[tokio::test]
async fn needs_human_keeps_a_sequential_section_moving() {
    let rows = format!(
        "{}{}",
        HELP.replace(
            "source = \"chat\"\ncontains = \"Available commands\"",
            "source = \"human\"\nquestion = \"Did it show?\""
        ),
        HELP.replace("W01", "W02")
    );
    let root = tempfile::tempdir().unwrap();
    let fake = Fake::new(&[(".help", &["Available commands"])]);
    let out = Runner::new(
        &fake,
        None,
        request(root.path(), vec![section("p", true, false, true, &rows)]),
    )
    .unwrap()
    .run_all()
    .await
    .unwrap();
    assert_eq!(
        out.rows[0].result, "NEEDS_HUMAN",
        "{:?}",
        out.rows[0].reasons
    );
    assert_eq!(out.rows[1].result, "PASS", "{:?}", out.rows[1].reasons);
}

#[tokio::test]
async fn finish_intro_false_leaves_the_dialog_for_the_first_row() {
    let root = tempfile::tempdir().unwrap();
    let fake = fresh_fake();
    let out = Runner::new(
        &fake,
        None,
        request(root.path(), vec![section("p", true, true, false, HELP)]),
    )
    .unwrap()
    .run_all()
    .await
    .unwrap();
    assert_eq!(out.rows[0].result, "PASS", "{:?}", out.rows[0].reasons);
    let names = fake.names();
    assert!(names.contains(&"lab_create_character".to_string()));
    assert!(!names.contains(&"lab_finish_dialog".to_string()));
}

#[tokio::test]
async fn fresh_intro_arriving_after_play_is_finished_before_the_first_row() {
    let root = tempfile::tempdir().unwrap();
    let fake = fresh_fake()
        .answer("lab_play_character", vec![json!({ "dialog_open": false })])
        .answer("client_wait_for", vec![json!({ "met": true })]);
    let out = Runner::new(
        &fake,
        None,
        request(root.path(), vec![section("p", true, true, true, HELP)]),
    )
    .unwrap()
    .run_all()
    .await
    .unwrap();
    assert_eq!(out.rows[0].result, "PASS", "{:?}", out.rows[0].reasons);
    let names = fake.names();
    assert!(names.contains(&"client_wait_for".to_string()));
    assert!(names.contains(&"lab_finish_dialog".to_string()));
    let waits: Vec<_> = fake
        .log
        .lock()
        .unwrap()
        .iter()
        .filter(|(tool, _)| tool == "client_wait_for")
        .map(|(_, args)| args.clone())
        .collect();
    assert_eq!(waits.len(), 2);
    assert_eq!(waits[0]["timeout_ms"], 15000);
    assert_eq!(waits[1]["timeout_ms"], 20000);
    assert_eq!(waits[1]["lua_condition"], "not ((LetterboxedWin and LetterboxedWin:isVisible()) or (MoviePlayerWin and MoviePlayerWin:isVisible()))");
}

#[tokio::test]
async fn two_fresh_sections_have_distinct_persisted_names_and_protected_slots() {
    let root = tempfile::tempdir().unwrap();
    let fake = fresh_fake();
    let runner = Runner::new(
        &fake,
        None,
        request(
            root.path(),
            vec![
                section("praxis", true, true, false, HELP),
                section("jaffa", true, true, false, HELP),
            ],
        ),
    )
    .unwrap();
    assert_ne!(runner.fresh["praxis"], runner.fresh["jaffa"]);
    assert_eq!(
        runner.manifest.fresh_characters["praxis"]["archetype"],
        "Commando"
    );
    assert_eq!(runner.manifest.fresh_characters.len(), 2);
    let first_name = runner.fresh["praxis"].clone();
    let mut old_manifest = serde_json::to_value(&runner.manifest).unwrap();
    old_manifest
        .as_object_mut()
        .unwrap()
        .remove("fresh_characters");
    let old: RunManifest = serde_json::from_value(old_manifest).unwrap();
    assert!(
        old.fresh_characters.is_empty(),
        "older run.json stays readable"
    );
    let out = runner.run_all().await.unwrap();
    assert!(
        out.rows.iter().all(|r| r.result == "PASS"),
        "{:?}",
        out.rows
    );
    let slot_calls: Vec<_> = fake
        .log
        .lock()
        .unwrap()
        .iter()
        .filter(|(tool, _)| tool == "lab_ensure_character_slot")
        .map(|(_, args)| args.clone())
        .collect();
    assert_eq!(slot_calls.len(), 2);
    assert!(slot_calls[1]["protect"]
        .as_array()
        .unwrap()
        .contains(&json!(format!("Uat {first_name}"))));
}

#[tokio::test]
async fn explicit_dependency_blocks_only_its_row() {
    let rows = format!(
        r#"
[[row]]
id = "W01"
title = "blocked"
expected = "Unavailable."
blocked = "waiting for a world tool"
[[row]]
id = "W02"
depends_on = ["W01"]
title = "dependent"
expected = "A reply appears."
anchor = false
step = [{{ chat = ".help" }}]
[[row.expect]]
id = "e2"
text = "help replies"
source = "chat"
contains = "Available commands"
{}"#,
        HELP.replace("W01", "W03")
    );
    let root = tempfile::tempdir().unwrap();
    let fake = Fake::new(&[(".help", &["Available commands"])]);
    let out = Runner::new(
        &fake,
        None,
        request(root.path(), vec![section("p", false, false, true, &rows)]),
    )
    .unwrap()
    .run_all()
    .await
    .unwrap();
    assert_eq!(out.rows[0].result, "BLOCKED");
    assert_eq!(out.rows[1].result, "BLOCKED");
    assert!(out.rows[1]
        .reasons
        .iter()
        .any(|r| r == "prior row W01 = BLOCKED"));
    assert_eq!(out.rows[2].result, "PASS", "{:?}", out.rows[2].reasons);
}

#[test]
fn dependency_must_name_an_earlier_row() {
    let malformed = HELP.replace(
        "title = \"help\"",
        "depends_on = [\"W02\"]\ntitle = \"help\"",
    );
    let text = format!("schema = 1\n[section]\nid = \"p\"\nsystem = \"p\"\nguide = \"g\"\nledger = \"l\"\n{malformed}");
    assert!(spec::parse(&text)
        .unwrap_err()
        .contains("must name an earlier row"));
}

#[test]
fn concurrent_runs_reserve_distinct_bundles_and_fresh_names() {
    use std::collections::HashSet;

    let root = tempfile::tempdir().unwrap();
    let results = std::thread::scope(|scope| {
        let workers: Vec<_> = (0..20)
            .map(|_| {
                scope.spawn(|| {
                    // Fixed decisecond reproduces the old collision even
                    // when the scheduler starts workers at different times.
                    for _ in 0..10 {
                        let id = super::new_run_id(42);
                        match RunDir::create_exclusive(root.path(), "2026-10-13", None, &id) {
                            Ok(dir) => {
                                let name = super::session::fresh_name(&id, "praxis", 0);
                                return (id, dir.root, name);
                            }
                            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                            Err(e) => panic!("reserve bundle: {e}"),
                        }
                    }
                    panic!("ten generated run ids collided");
                })
            })
            .collect();
        workers
            .into_iter()
            .map(|w| w.join().unwrap())
            .collect::<Vec<_>>()
    });
    assert_eq!(
        results.iter().map(|r| &r.0).collect::<HashSet<_>>().len(),
        20
    );
    assert_eq!(
        results.iter().map(|r| &r.1).collect::<HashSet<_>>().len(),
        20
    );
    assert_eq!(
        results.iter().map(|r| &r.2).collect::<HashSet<_>>().len(),
        20
    );
    assert!(results.iter().all(|r| r.2.len() <= 16));
}
