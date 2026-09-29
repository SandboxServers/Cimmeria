//! `.craftkit` and `.learnblueprint`: a GM's line forwards one
//! `GmCraftGrant` for the targeted player; a malformed argument is answered
//! on the cell and forwards nothing; a non-GM's line is ordinary chat.

use cimmeria_content_engine::chain::ChainEngine;
use tokio::sync::mpsc;

use super::super::chat::{handle_chat_message, CHAN_SAY};
use super::setup;
use crate::cell::messages::{CellToBaseMsg, GmCraftGrant, GmCraftGrantKind};
use crate::cell::space_manager::SpaceManager;

const TARGET: u32 = 2;
const TARGET_PLAYER_ID: i32 = 4302;

/// The console fixture plus a second player, targeted by the caller.
fn with_player_target(caller_access_level: u32) -> (SpaceManager, u32) {
    let (mut mgr, gm, _npc) = setup();
    mgr.create_entity(TARGET, "Agnos", [11.0, 0.0, 10.0], [0.0; 3])
        .unwrap();
    mgr.connect_entity(TARGET);
    mgr.get_entity_mut(TARGET).unwrap().player_id = Some(TARGET_PLAYER_ID);
    let caller = mgr.get_entity_mut(gm).unwrap();
    caller.access_level = caller_access_level;
    caller.current_target_id = Some(TARGET as i32);
    caller
        .witnesses
        .insert(cimmeria_common::EntityId(TARGET as i32));
    (mgr, gm)
}

async fn say(mgr: &mut SpaceManager, speaker: u32, text: &str) -> Vec<CellToBaseMsg> {
    let (tx, mut rx) = mpsc::channel(64);
    handle_chat_message(
        speaker,
        "Speaker",
        0,
        CHAN_SAY,
        text,
        &tx,
        mgr,
        &ChainEngine::new(),
    )
    .await;
    drop(tx);
    let mut out = Vec::new();
    while let Some(msg) = rx.recv().await {
        out.push(msg);
    }
    out
}

fn grants(msgs: &[CellToBaseMsg]) -> Vec<&GmCraftGrant> {
    msgs.iter()
        .filter_map(|m| m.plugin_payload::<GmCraftGrant>())
        .collect()
}

fn grant_for(gm: u32, grant: GmCraftGrantKind) -> GmCraftGrant {
    GmCraftGrant {
        entity_id: TARGET,
        player_id: TARGET_PLAYER_ID,
        gm_entity_id: gm,
        grant,
    }
}

#[tokio::test]
async fn gm_craftkit_forwards_the_blueprint_and_count() {
    let (mut mgr, gm) = with_player_target(2);
    let msgs = say(&mut mgr, gm, ".craftkit 25 3").await;
    assert_eq!(
        grants(&msgs),
        vec![&grant_for(
            gm,
            GmCraftGrantKind::Kit {
                blueprint_id: 25,
                count: 3
            }
        )]
    );
}

/// Without a count the kit is for one craft.
#[tokio::test]
async fn gm_craftkit_count_defaults_to_one() {
    let (mut mgr, gm) = with_player_target(2);
    let msgs = say(&mut mgr, gm, ".craftkit 412").await;
    assert_eq!(
        grants(&msgs),
        vec![&grant_for(
            gm,
            GmCraftGrantKind::Kit {
                blueprint_id: 412,
                count: 1
            }
        )]
    );
}

#[tokio::test]
async fn gm_learnblueprint_forwards_the_blueprint() {
    let (mut mgr, gm) = with_player_target(2);
    let msgs = say(&mut mgr, gm, ".learnblueprint 25").await;
    assert_eq!(
        grants(&msgs),
        vec![&grant_for(
            gm,
            GmCraftGrantKind::LearnBlueprint { blueprint_id: 25 }
        )]
    );
}

/// A non-integer argument is answered on the cell and never reaches the
/// base.
#[tokio::test]
async fn malformed_arguments_forward_nothing() {
    let (mut mgr, gm) = with_player_target(2);
    for line in [".craftkit steel", ".craftkit 25 many", ".learnblueprint x"] {
        let msgs = say(&mut mgr, gm, line).await;
        assert!(grants(&msgs).is_empty(), "{line}: {msgs:?}");
        assert!(!msgs.is_empty(), "{line}: the GM gets a line");
    }
}

/// The guard: a player's `.craftkit` or `.learnblueprint` is chat, not a
/// grant.
#[tokio::test]
async fn non_gm_grants_forward_nothing() {
    let (mut mgr, player) = with_player_target(0);
    for line in [".craftkit 25", ".learnblueprint 25"] {
        let msgs = say(&mut mgr, player, line).await;
        assert!(grants(&msgs).is_empty(), "{line}: {msgs:?}");
    }
}

/// With the base channel closed, the grant cannot be queued: a WARN
/// `forward_failed` carrying the target's `account_id`, `player_id` and
/// `entity_id`, and the GM in `gm_entity_id`.
#[tokio::test]
async fn a_grant_the_base_cannot_receive_warns_with_the_canonical_identity() {
    let capture = crate::test_support::LogCapture::install();
    let (mut mgr, gm) = with_player_target(2);
    mgr.get_entity_mut(TARGET).unwrap().account_id = Some(4303);
    let (tx, rx) = mpsc::channel(64);
    drop(rx);
    handle_chat_message(
        gm,
        "Speaker",
        0,
        CHAN_SAY,
        ".craftkit 25",
        &tx,
        &mut mgr,
        &ChainEngine::new(),
    )
    .await;

    let e = capture
        .find_message(
            tracing::Level::WARN,
            "GM crafting grant could not be queued",
        )
        .expect("forward_failed WARN");
    assert_eq!(e.target, "crafting");
    for (field, value) in [
        ("event", "forward_failed".to_string()),
        ("kind", "gm_craft_grant".to_string()),
        ("account_id", "4303".to_string()),
        ("player_id", TARGET_PLAYER_ID.to_string()),
        ("entity_id", TARGET.to_string()),
        ("gm_entity_id", gm.to_string()),
    ] {
        assert!(e.has_field(field, &value), "{field}: {e:#?}");
    }
}

/// `.help craftkit` and `.help learnblueprint` list their arguments, the
/// count marked optional.
#[tokio::test]
async fn help_lists_the_grant_arguments() {
    for (command, want) in [
        (
            "craftkit",
            vec![
                "    blueprintId (int): Blueprint whose component set 1 the target gets",
                "    [count] (int): Crafts' worth to grant, 1-10 (default 1)",
            ],
        ),
        (
            "learnblueprint",
            vec!["    blueprintId (int): Blueprint to teach the target"],
        ),
    ] {
        let (mut mgr, gm, _npc) = setup();
        let (tx, mut rx) = mpsc::channel(64);
        crate::cell::console::handle_console_command(
            gm,
            &format!(".help {command}"),
            &tx,
            &mut mgr,
            &ChainEngine::new(),
        )
        .await;
        let lines: Vec<String> = std::iter::from_fn(|| rx.try_recv().ok())
            .filter_map(|m| super::decode_feedback(&m))
            .collect();
        assert!(
            lines
                .iter()
                .any(|l| l.starts_with(&format!(".{command}: "))),
            "{lines:?}"
        );
        for line in want {
            assert!(
                lines.iter().any(|l| l == line),
                "{command}: {line:?} in {lines:?}"
            );
        }
    }
}
