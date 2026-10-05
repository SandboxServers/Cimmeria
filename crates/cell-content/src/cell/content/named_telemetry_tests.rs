//! Rule 6 capture guards for the content sweep's most-read events (named
//! telemetry NT-21): mission accept / step advance / complete, chain fire,
//! and dialog display / choice. Each asserts the name next to every ID the
//! line carries, by value, so dropping one name field trips its guard.
//!
//! The names come from a NameBook installed for the test and from the
//! player's `character_name`, so a regression that resolves the wrong table
//! (a dialog-set map read as a dialog set, say) fails on the value too.

use std::collections::HashMap;

use cimmeria_content_engine::actions::Action;
use cimmeria_content_engine::chain::{Chain, ChainEngine, ResolvedActions};
use cimmeria_content_engine::triggers::Trigger;
use cimmeria_names::{NameBook, Table};
use tokio::sync::mpsc;
use tracing::Level;

use crate::cell::space_manager::SpaceManager;
use crate::test_support::{Captured, LogCapture, LogCaptureGuard};

const PLAYER: u32 = 1;
const PLAYER_ID: i32 = 42;
const PLAYER_NAME: &str = "Teal'c";
const NPC: u32 = 100;
const NPC_NAME_ID: i64 = 70_001;
const NPC_NAME: &str = "Gerschon";

const MISSION: i32 = 701;
const MISSION_NAME: &str = "Gerschon's Request";
const STEP: i32 = 2113;
const STEP_NAME: &str = "Talk to Gerschon";
const CHAIN: i64 = 1202;
const CHAIN_NAME: &str = "701 - Gerschon interact (Human): offer dialog 2573";
const DIALOG: i32 = 2573;
const DIALOG_NAME: &str = "What do you need?";

/// Installs the test's NameBook; the empty book comes back on drop, so a
/// test that panics doesn't leave its names behind for the next one.
struct Names;

impl Names {
    fn install() -> Self {
        let mut book = NameBook::empty();
        book.insert(Table::Missions, MISSION.into(), MISSION_NAME);
        book.insert(Table::MissionSteps, STEP.into(), STEP_NAME);
        book.insert(Table::Chains, CHAIN, CHAIN_NAME);
        book.insert(Table::Dialogs, DIALOG.into(), DIALOG_NAME);
        book.insert(Table::Texts, NPC_NAME_ID, NPC_NAME);
        cimmeria_names::global().store(book);
        Names
    }
}

impl Drop for Names {
    fn drop(&mut self) {
        cimmeria_names::global().store(NameBook::empty());
    }
}

/// A space with the named player and the named NPC in it.
fn world() -> SpaceManager {
    let mut mgr = crate::test_support::make_space_manager();
    mgr.create_entity(PLAYER, "Agnos", [0.0; 3], [0.0; 3])
        .unwrap();
    let p = mgr.get_entity_mut(PLAYER).unwrap();
    p.is_player = true;
    p.player_id = Some(PLAYER_ID);
    p.character_name = Some(PLAYER_NAME.to_string());
    p.stamp_log_names(Some(PLAYER_NAME), None);
    mgr.create_entity(NPC, "Agnos", [1.0; 3], [0.0; 3]).unwrap();
    mgr.get_entity_mut(NPC).unwrap().name_id = Some(NPC_NAME_ID as i32);
    mgr
}

async fn execute(
    mgr: &mut SpaceManager,
    actions: Vec<Action>,
    params: HashMap<String, serde_json::Value>,
) {
    let (tx, _rx) = mpsc::channel(256);
    let engine = ChainEngine::new();
    let resolved = ResolvedActions {
        action_delays: Vec::new(),
        params,
        actions: actions.into_iter().map(|a| (CHAIN, a)).collect(),
    };
    super::execute_actions(resolved, PLAYER, PLAYER_ID, &tx, mgr, &engine).await;
}

fn event(capture: &LogCaptureGuard, level: Level, msg: &str) -> Captured {
    capture
        .find_message(level, msg)
        .unwrap_or_else(|| panic!("no {level} event {msg:?}"))
}

/// Assert `key` carries `want`. Captured fields are `Debug`-formatted, so a
/// string value arrives quoted.
fn assert_name(e: &Captured, key: &str, want: &str) {
    let got = e.fields.get(key);
    assert_eq!(
        got.map(|v| v.trim_matches('"')),
        Some(want),
        "{key} must name its ID on {:?}; fields: {:?}",
        e.message,
        e.fields
    );
}

#[tokio::test]
async fn mission_accept_advance_and_complete_name_mission_step_chain_and_player() {
    let _names = Names::install();
    let capture = LogCapture::install();
    let mut mgr = world();

    execute(
        &mut mgr,
        vec![
            Action::AcceptMission {
                mission_id: MISSION,
            },
            Action::AdvanceStep {
                mission_id: MISSION,
                step_id: STEP,
            },
            Action::CompleteMission {
                mission_id: MISSION,
            },
        ],
        HashMap::new(),
    )
    .await;

    for msg in [
        "Content: accepting mission",
        "Content: advancing step",
        "Content: completing mission",
    ] {
        let e = event(&capture, Level::INFO, msg);
        assert_name(&e, "entity_name", PLAYER_NAME);
        assert_name(&e, "mission_name", MISSION_NAME);
        assert_name(&e, "chain_name", CHAIN_NAME);
    }
    let advance = event(&capture, Level::INFO, "Content: advancing step");
    assert_name(&advance, "step_name", STEP_NAME);
}

#[tokio::test]
async fn dialog_display_names_dialog_player_npc_and_chain() {
    let _names = Names::install();
    let capture = LogCapture::install();
    let mut mgr = world();
    let mut params = HashMap::new();
    params.insert("target_entity_id".to_string(), serde_json::json!(NPC));

    execute(
        &mut mgr,
        vec![Action::DisplayDialog { dialog_id: DIALOG }],
        params,
    )
    .await;

    let e = event(&capture, Level::INFO, "Content: displaying dialog");
    assert_name(&e, "entity_name", PLAYER_NAME);
    assert_name(&e, "dialog_name", DIALOG_NAME);
    assert_name(&e, "npc_entity_name", NPC_NAME);
    assert_name(&e, "chain_name", CHAIN_NAME);
}

#[tokio::test]
async fn dialog_choice_and_the_chain_it_fires_are_named() {
    let _names = Names::install();
    let capture = LogCapture::install();
    let mut mgr = world();
    let mut engine = ChainEngine::new();
    engine.register_chain(Chain {
        id: CHAIN,
        // What the loader stores: the `content_chains.description`.
        name: CHAIN_NAME.to_string(),
        enabled: true,
        trigger: Trigger::OnDialogChoice { dialog_id: DIALOG },
        conditions: Vec::new(),
        actions: vec![Action::IncrementCounter {
            counter_name: "chose".to_string(),
            amount: 1,
        }],
        action_delays: Vec::new(),
        priority: 0,
        once: false,
    });
    let (tx, _rx) = mpsc::channel(64);

    super::fire_dialog_choice(PLAYER, PLAYER_ID, DIALOG, 0, &engine, &tx, &mut mgr).await;

    let choice = event(&capture, Level::INFO, "fire_dialog_choice: matched");
    assert_name(&choice, "entity_name", PLAYER_NAME);
    assert_name(&choice, "player_name", PLAYER_NAME);
    assert_name(&choice, "dialog_name", DIALOG_NAME);

    let fired = event(&capture, Level::DEBUG, "resolve_event: chain matched");
    assert_name(&fired, "chain_name", CHAIN_NAME);
}
