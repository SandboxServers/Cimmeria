//! Two-player runner tests (AB-L6) over two fake clients: a `players = 2`
//! row brings p2 in world, runs `client = "p2"` steps and clauses on p2
//! only, and `@target_player` targets the other client's character by
//! name. Without a second instance the row is BLOCKED with the reason.

use serde_json::Value;

use super::tests::{request, Fake};
use super::{Runner, SecondPlayer};
use crate::uat::evidence::{RowEvidence, RowResult, RunDir, Verdict};

const DUEL_ROW: &str = r#"
[[row]]
id = "M1-2"
title = "Readouts describe the selected player"
expected = "p1 targets p2; p2 targets p1 and reads its own state."
players = 2
step = [
  { tool = "@target_player", label = "p1-targets" },
  { tool = "@target_player", client = "p2", label = "p2-targets" },
  { chat = ".info", client = "p2" },
]
[[row.expect]]
id = "p2-state"
text = "p2 reads its own player state"
source = "tool"
client = "p2"
tool = "@player_state"
pointer = "/name"
value = "Labtwo"
[[row.expect]]
id = "p1-state"
text = "p1 still reads its own"
source = "tool"
tool = "@player_state"
pointer = "/name"
value = "Labone"
"#;

fn p2_of(fake: &Fake) -> Result<SecondPlayer<'_, Fake>, String> {
    Ok(SecondPlayer {
        inv: fake,
        instance: "p2".into(),
        account: Some("lab2".into()),
        character: "Labtwo".into(),
    })
}

async fn run(p1: &Fake, p2: Result<SecondPlayer<'_, Fake>, String>, rows: &str) -> RowEvidence {
    let tmp = tempfile::tempdir().unwrap().keep();
    let out = Runner::new(p1, None, request(&tmp, rows))
        .unwrap()
        .with_p2(p2)
        .run_all()
        .await
        .unwrap();
    RunDir::open(std::path::Path::new(&out.run_dir))
        .unwrap()
        .rows()
        .unwrap()
        .remove(0)
}

fn targeted(fake: &Fake) -> Vec<Value> {
    fake.log
        .lock()
        .unwrap()
        .iter()
        .filter(|(n, _)| n == "client_target")
        .map(|(_, a)| a["name"].clone())
        .collect()
}

#[tokio::test]
async fn a_two_player_row_drives_p2_and_targets_the_other_player() {
    let p1 = Fake::new(&[])
        .with("client_target")
        .with("client_player_state");
    let p2 = Fake::new(&[])
        .with("client_target")
        .with("client_player_state")
        .stopped("Labtwo");
    let row = run(&p1, p2_of(&p2), DUEL_ROW).await;
    assert_eq!(row.result, RowResult::Pass, "{:?}", row.reasons);

    // p2 was started, logged in and played as its own character.
    let p2_calls = p2.names();
    for flow in ["lab_client_start", "lab_login", "lab_play_character"] {
        assert!(
            p2_calls.iter().any(|c| c == flow),
            "p2 never ran {flow}: {p2_calls:?}"
        );
    }
    let play = p2
        .log
        .lock()
        .unwrap()
        .iter()
        .find(|(n, _)| n == "lab_play_character")
        .map(|(_, a)| a.clone())
        .unwrap();
    assert_eq!(play["name"], "Labtwo");

    // Each side targets the other by name, with real input.
    assert_eq!(targeted(&p1), vec![Value::from("Labtwo")]);
    assert_eq!(targeted(&p2), vec![Value::from("Labone")]);
    let step = row
        .actions
        .iter()
        .find(|a| a.label.as_deref() == Some("p2-targets"))
        .unwrap();
    assert_eq!(step.client.as_deref(), Some("p2"));
    assert_eq!(step.tool.as_deref(), Some("client_target"));
    assert_eq!(step.tier.map(|t| t.as_str()), Some("N1"));

    // p2's chat line went to p2's client, not p1's.
    assert!(p2_calls.iter().any(|c| c == "client_type_text"));
    // The p2 clause read p2; the p1 clause read p1.
    let p2_clause = row.clauses.iter().find(|c| c.id == "p2-state").unwrap();
    assert_eq!(p2_clause.verdict, Verdict::Pass);
    assert_eq!(p2_clause.client.as_deref(), Some("p2"));
    assert_eq!(row.vars["p2_character"], "Labtwo");
    assert!(row.attachments.iter().any(|a| a.name == "final-p2"));
}

/// No second instance: the row is BLOCKED with the reason and nothing is
/// driven on either client.
#[tokio::test]
async fn without_a_second_instance_the_row_is_blocked_with_the_reason() {
    let p1 = Fake::new(&[])
        .with("client_target")
        .with("client_player_state");
    let reason = "players = 2 needs a second lab instance: no lab-account.p2.json";
    let row = run(&p1, Err(reason.into()), DUEL_ROW).await;
    assert_eq!(row.result, RowResult::Blocked);
    assert!(row.reasons.iter().any(|r| r == reason), "{:?}", row.reasons);
    assert!(p1.names().is_empty(), "a blocked row drives nothing");

    // The default (never configured) reason names what to set up.
    let tmp = tempfile::tempdir().unwrap().keep();
    let out = Runner::new(&p1, None, request(&tmp, DUEL_ROW))
        .unwrap()
        .run_all()
        .await
        .unwrap();
    assert_eq!(out.rows[0].result, "BLOCKED");
    assert!(out.rows[0].reasons[0].contains("lab-account.p2.json"));
}

#[tokio::test]
async fn a_p2_tool_missing_on_p2_blocks_and_three_players_always_block() {
    let p1 = Fake::new(&[])
        .with("client_target")
        .with("client_player_state");
    // p2's lab lacks client_player_state: the p2 clause's reader is missing.
    let p2 = Fake::new(&[]).with("client_target");
    let row = run(&p1, p2_of(&p2), DUEL_ROW).await;
    assert_eq!(row.result, RowResult::Blocked);
    assert!(
        row.reasons
            .iter()
            .any(|r| r.contains("client_player_state") && r.contains("on p2")),
        "{:?}",
        row.reasons
    );

    let three = DUEL_ROW.replace("players = 2", "players = 3");
    let p2 = Fake::new(&[])
        .with("client_target")
        .with("client_player_state");
    let row = run(&p1, p2_of(&p2), &three).await;
    assert_eq!(row.result, RowResult::Blocked);
    assert!(row.reasons[0].contains("at most two"), "{:?}", row.reasons);
}

/// p2 playing p1's own character would kick p1 (duplicate login).
#[tokio::test]
async fn p2_on_p1s_character_is_blocked() {
    let p1 = Fake::new(&[])
        .with("client_target")
        .with("client_player_state");
    let p2 = Fake::new(&[])
        .with("client_target")
        .with("client_player_state");
    let mut same = p2_of(&p2).unwrap();
    same.character = "labone".into();
    let row = run(&p1, Ok(same), DUEL_ROW).await;
    assert_eq!(row.result, RowResult::Blocked);
    assert!(
        row.reasons[0].contains("same character"),
        "{:?}",
        row.reasons
    );
}

/// The p2 supervisor outlives a run: a p2 already in world as someone
/// else is logged out and re-selected; one in world as its own character
/// is reused.
#[tokio::test]
async fn a_p2_in_world_as_someone_else_is_reselected() {
    let p1 = Fake::new(&[])
        .with("client_target")
        .with("client_player_state");
    let stranger = Fake::new(&[])
        .with("client_target")
        .with("client_player_state")
        .named("Labthree");
    let row = run(&p1, p2_of(&stranger), DUEL_ROW).await;
    let calls = stranger.names();
    assert!(calls.iter().any(|c| c == "lab_logout"), "{calls:?}");
    assert!(calls.iter().any(|c| c == "lab_play_character"), "{calls:?}");
    assert!(row
        .actions
        .iter()
        .any(|a| a.requested == "lab_play_character"));

    let own = Fake::new(&[])
        .with("client_target")
        .with("client_player_state")
        .named("Labtwo");
    run(&p1, p2_of(&own), DUEL_ROW).await;
    let calls = own.names();
    assert!(
        !calls
            .iter()
            .any(|c| c == "lab_logout" || c == "lab_play_character"),
        "{calls:?}"
    );
}

/// p2 on p1's account would evict p1 mid-row: BLOCKED before either
/// client is driven.
#[tokio::test]
async fn p2_on_p1s_account_is_blocked_before_anything_runs() {
    let p1 = Fake::new(&[])
        .with("client_target")
        .with("client_player_state");
    let p2 = Fake::new(&[])
        .with("client_target")
        .with("client_player_state");
    let mut same = p2_of(&p2).unwrap();
    same.account = Some("LAB".into());
    let tmp = tempfile::tempdir().unwrap().keep();
    let mut req = request(&tmp, DUEL_ROW);
    req.account_name = Some("lab".into());
    let out = Runner::new(&p1, None, req)
        .unwrap()
        .with_p2(Ok(same))
        .run_all()
        .await
        .unwrap();
    assert_eq!(out.rows[0].result, "BLOCKED");
    assert!(
        out.rows[0].reasons[0].contains("p1's own account"),
        "{:?}",
        out.rows[0].reasons
    );
    assert!(p1.names().is_empty() && p2.names().is_empty());
}

/// The committed gm-parity M1-2 row puts both players in one space before
/// targeting: p1 types `.summon <p2>` in setup, then checks it sees p2.
#[tokio::test]
async fn m1_2_summons_p2_before_targeting() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/guides/uat-specs");
    let mut sections = crate::uat::load_sections(&dir, Some(&["gm-parity".to_string()])).unwrap();
    let p1 = Fake::new(&[])
        .with("client_target")
        .with("client_player_state")
        .with("client_entity_find");
    let p2 = Fake::new(&[])
        .with("client_target")
        .with("client_player_state");
    let tmp = tempfile::tempdir().unwrap().keep();
    let req = super::RunRequest {
        sections: std::mem::take(&mut sections),
        rows: Some(vec!["M1-2".into()]),
        root: tmp,
        lab_character: Some("Labone".into()),
        no_settle: true,
        ..Default::default()
    };
    Runner::new(&p1, None, req)
        .unwrap()
        .with_p2(p2_of(&p2))
        .run_all()
        .await
        .unwrap();
    let log = p1.log.lock().unwrap().clone();
    let summon = log
        .iter()
        .position(|(n, a)| n == "client_type_text" && a["text"] == ".summon Labtwo")
        .expect("p1 summons p2");
    let find = log
        .iter()
        .position(|(n, a)| n == "client_entity_find" && a["name"] == "Labtwo")
        .expect("p1 looks for p2");
    let target = log
        .iter()
        .position(|(n, _)| n == "client_target")
        .expect("p1 targets p2");
    assert!(summon < find && find < target, "{summon} {find} {target}");
}
