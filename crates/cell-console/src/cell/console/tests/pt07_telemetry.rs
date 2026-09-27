//! Pets campaign PT-07: the `.pet` / `.giveability` log lines (TESTING.md
//! type 12, negative-log).
//!
//! Filter prefix: `pt07_`.
//!
//! The bar: "GM X did Y at T and it failed" is answerable from SigNoz alone.
//! So every refusal must carry `reason` and the caller's `account_id` /
//! `player_id` at the right level, every accept its correlators, and a line
//! about someone else's pet or character the subject's `subject_player_id`.
//! Each test pins level, target, `decision_outcome` and `reason`, so a
//! revert that drops a field or demotes an event fails.

use cimmeria_content_engine::chain::ChainEngine;
use tokio::sync::mpsc;
use tracing::Level;

use super::pt07_giveability::{console, say, world, CALLER};
use super::pt07_pet::{only_pet_of, pet, pet_world, GM, OTHER, PET_TEMPLATE};
use crate::cell::console::{exec, handle_console_command};
use crate::test_support::{Captured, LogCapture};

/// The one event on `target` with `decision_outcome` and (if given)
/// `reason`; panics listing what was captured otherwise.
fn event(
    all: &[Captured],
    level: Level,
    target: &str,
    outcome: &str,
    reason: Option<&str>,
) -> Captured {
    let found: Vec<&Captured> = all
        .iter()
        .filter(|c| {
            c.level == level
                && c.target == target
                && c.has_field("decision_outcome", outcome)
                && reason.is_none_or(|r| c.has_field("reason", r))
        })
        .collect();
    assert_eq!(
        found.len(),
        1,
        "want one {level} {target} {outcome}/{reason:?}, got {found:#?}\nall: {all:#?}"
    );
    found[0].clone()
}

/// The caller's identity, as `pet_world` / `world` stamp it (Rule 5).
fn assert_caller_identity(e: &Captured) {
    assert!(
        e.has_field("account_id", "601") && e.has_field("player_id", "71"),
        "caller identity missing: {e:#?}"
    );
}

const PETS_COMMAND: &str = "pets.command";
const GIVEABILITY_TARGET: &str = "cimmeria_cell_console::cell::console::give_ability";
const DISPATCH_TARGET: &str = "cimmeria_cell_console::cell::console::dispatch";

#[tokio::test]
async fn pt07_pet_refusals_log_debug_with_reason_and_caller_identity() {
    for (args, reason) in [
        (&["summon", "99999"][..], "unknown_id"),
        (&["summon", "x"], "bad_args"),
        (&["stance", "7"], "bad_stance"),
        (&["stance", "1"], "no_pet"),
        (&["dismiss"], "no_pet"),
        (&["info"], "no_pet"),
        (&["frobnicate"], "unknown_verb"),
    ] {
        let mut mgr = pet_world();
        let capture = LogCapture::install();
        pet(&mut mgr, GM, None, args).await;
        let e = event(
            &capture.all(),
            Level::DEBUG,
            PETS_COMMAND,
            "gm_refused",
            Some(reason),
        );
        assert_caller_identity(&e);
    }
}

#[tokio::test]
async fn pt07_pet_summon_and_stance_log_info_with_correlators() {
    let mut mgr = pet_world();
    let capture = LogCapture::install();
    pet(&mut mgr, GM, None, &["summon", "2826"]).await;
    let pet_id = only_pet_of(&mgr, GM).to_string();
    pet(&mut mgr, GM, None, &["stance", "0"]).await;
    let all = capture.all();

    let summoned = event(&all, Level::INFO, PETS_COMMAND, "gm_summoned", None);
    assert_caller_identity(&summoned);
    for (k, v) in [
        ("pet_id", pet_id.as_str()),
        ("owner_id", "1"),
        ("template_id", "350"),
        ("ability_id", "2826"),
    ] {
        assert!(summoned.has_field(k, v), "{k}={v}: {summoned:#?}");
    }
    let stance = event(&all, Level::INFO, PETS_COMMAND, "gm_stance_set", None);
    assert_caller_identity(&stance);
    assert!(
        stance.has_field("pet_id", &pet_id)
            && stance.has_field("from", "defensive")
            && stance.has_field("stance", "passive")
            && stance.has_field("event", "stance_changed"),
        "{stance:#?}"
    );
}

/// Inspecting another owner's pet names that owner as the subject.
#[tokio::test]
async fn pt07_pet_info_on_another_owners_pet_logs_the_subject() {
    let mut mgr = pet_world();
    let theirs = mgr.spawn_pet_from_template(OTHER, PET_TEMPLATE, 0).unwrap();
    let capture = LogCapture::install();
    pet(&mut mgr, GM, Some(theirs), &["info"]).await;
    let e = event(
        &capture.all(),
        Level::DEBUG,
        PETS_COMMAND,
        "gm_inspected",
        None,
    );
    assert_caller_identity(&e);
    assert!(
        e.has_field("subject_player_id", "72") && e.has_field("owner_id", "2"),
        "{e:#?}"
    );
}

/// A player's `.pet` refusal is a DEBUG on `pets.command` with the player's
/// identity; `.giveability` uses the console's module-path target. Never a
/// WARN: a player can type these at will.
#[tokio::test]
async fn pt07_non_gm_refusals_log_debug_not_gm_with_identity() {
    for (line, target, outcome) in [
        (".pet summon 2826", PETS_COMMAND, "gm_refused"),
        (".giveability 2826", DISPATCH_TARGET, "refused"),
    ] {
        let (mut mgr, _npc) = world(0);
        let capture = LogCapture::install();
        say(&mut mgr, line).await;
        let all = capture.all();
        let e = event(&all, Level::DEBUG, target, outcome, Some("not_gm"));
        assert_caller_identity(&e);
        assert!(
            !all.iter().any(|c| c.level <= Level::WARN),
            "{line}: a player-triggerable refusal must not WARN: {all:#?}"
        );
    }
}

#[tokio::test]
async fn pt07_giveability_refusals_log_debug_with_reason_and_caller_identity() {
    for (line, reason, known) in [
        (".giveability x", "bad_args", false),
        (".giveability 99999", "unknown_ability", false),
        (".giveability 2826", "already_known", true),
    ] {
        let (mut mgr, _npc) = world(2);
        if known {
            mgr.get_entity_mut(CALLER)
                .unwrap()
                .abilities
                .add_ability(2826);
        }
        let capture = LogCapture::install();
        console(&mut mgr, None, line).await;
        let e = event(
            &capture.all(),
            Level::DEBUG,
            GIVEABILITY_TARGET,
            "refused",
            Some(reason),
        );
        assert_caller_identity(&e);
    }
}

/// The accept names the GM, the subject and the ability, and says the cell
/// has not persisted anything (the base's line does).
#[tokio::test]
async fn pt07_giveability_forward_logs_info_with_the_subject() {
    let (mut mgr, _npc) = world(2);
    let capture = LogCapture::install();
    console(&mut mgr, Some(2), ".giveability 2826").await;
    let e = event(
        &capture.all(),
        Level::INFO,
        GIVEABILITY_TARGET,
        "forwarded",
        None,
    );
    assert_caller_identity(&e);
    assert!(
        e.has_field("subject_player_id", "72")
            && e.has_field("ability_id", "2826")
            && e.has_field("persisted", "false"),
        "{e:#?}"
    );
}

/// A sender whose receiver is gone, so every send fails.
fn closed_tx() -> mpsc::Sender<crate::cell::messages::CellToBaseMsg> {
    let (tx, rx) = mpsc::channel(8);
    drop(rx);
    tx
}

/// Span fields are not copied onto OTLP log records, so the `.pet stance`
/// send failure must carry the caller's identity itself (Copilot, #908).
#[tokio::test]
async fn pt07_pet_stance_send_failure_logs_warn_with_caller_identity() {
    let mut mgr = pet_world();
    pet(&mut mgr, GM, None, &["summon", "2826"]).await;
    let pet_id = only_pet_of(&mgr, GM).to_string();
    let capture = LogCapture::install();
    exec(
        "pet",
        GM,
        &["stance", "0"],
        None,
        &closed_tx(),
        &mut mgr,
        &ChainEngine::new(),
    )
    .await;
    let e = event(
        &capture.all(),
        Level::WARN,
        PETS_COMMAND,
        "send_failed",
        Some("cell_to_base_closed"),
    );
    assert_caller_identity(&e);
    assert!(
        e.has_field("entity_id", "1") && e.has_field("pet_id", &pet_id),
        "{e:#?}"
    );
}

/// The `.giveability` send failure carries the same fields as `forwarded`.
#[tokio::test]
async fn pt07_giveability_send_failure_logs_warn_with_caller_identity() {
    let (mut mgr, _npc) = world(2);
    mgr.get_entity_mut(CALLER).unwrap().current_target_id = Some(2);
    let capture = LogCapture::install();
    handle_console_command(
        CALLER,
        ".giveability 2826",
        &closed_tx(),
        &mut mgr,
        &ChainEngine::new(),
    )
    .await;
    let e = event(
        &capture.all(),
        Level::WARN,
        GIVEABILITY_TARGET,
        "send_failed",
        Some("cell_to_base_closed"),
    );
    assert_caller_identity(&e);
    assert!(
        e.has_field("entity_id", "1")
            && e.has_field("subject_entity_id", "2")
            && e.has_field("subject_player_id", "72")
            && e.has_field("ability_id", "2826"),
        "{e:#?}"
    );
}

/// `.help pet` and `.help giveability` print the argument detail lines
/// (Copilot, #908).
#[tokio::test]
async fn pt07_help_shows_pet_and_giveability_argument_detail() {
    let mut mgr = pet_world();
    for (cmd, want) in [
        (
            "pet",
            vec![
                "    verb (str): summon | dismiss",
                "    [id] (int): For summon",
            ],
        ),
        (
            "giveability",
            vec!["    abilityId (int): The ability to grant"],
        ),
    ] {
        let (tx, mut rx) = mpsc::channel(64);
        handle_console_command(
            GM,
            &format!(".help {cmd}"),
            &tx,
            &mut mgr,
            &ChainEngine::new(),
        )
        .await;
        let lines: Vec<String> = std::iter::from_fn(|| rx.try_recv().ok())
            .filter_map(|m| super::decode_feedback(&m))
            .collect();
        assert!(
            lines.iter().any(|l| l.starts_with(&format!(".{cmd}: "))),
            "{cmd} summary: {lines:?}"
        );
        for w in want {
            assert!(
                lines.iter().any(|l| l.starts_with(w)),
                "{cmd} {w:?}: {lines:?}"
            );
        }
    }
}
