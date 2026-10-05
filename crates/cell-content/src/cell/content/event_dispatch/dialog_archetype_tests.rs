//! The `archetype` contract on the dialog dispatchers (Class Start v6,
//! CS-01b): `fire_dialog_open` and `fire_dialog_choice` set the `archetype`
//! param from the acting player, as every other player-scoped dispatcher
//! does.
//!
//! Without it `Condition::Archetype` reads -1: an `eq` gate never matches
//! and a `neq` gate always passes, so the SGC starter dialogs gated
//! `archetype neq 7` (OD-CS09) would still pull a visiting Free Jaffa in.
//! In memory, no database: each chain only bumps a counter on the player.

use tokio::sync::mpsc;

use cimmeria_content_engine::actions::Action;
use cimmeria_content_engine::chain::{Chain, ChainEngine};
use cimmeria_content_engine::conditions::{ComparisonOp, Condition};
use cimmeria_content_engine::triggers::Trigger;

use super::{fire_dialog_choice, fire_dialog_open};
use crate::cell::space_manager::SpaceManager;

const PLAYER_EID: u32 = 1;
const PLAYER_ID: i32 = 100;
const DIALOG_ID: i32 = 9_271;
const COUNTER: &str = "archetype_gate_fired";

/// `EArchetype` wire values (`db/resources/Archetypes/Types/EArchetype.sql`).
const SOLDIER: i32 = 1;
const SHOLVA: i32 = 7;

#[derive(Clone, Copy, Debug)]
enum Dispatcher {
    Open,
    Choice,
}

/// A connected player in Castle with `archetype`.
fn make_mgr(archetype: i32) -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    mgr.parse_spaces_xml(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#,
    )
    .unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#,
    )
    .unwrap();
    mgr.create_entity(PLAYER_EID, "Castle", [0.0; 3], [0.0; 3])
        .unwrap();
    if let Some(p) = mgr.get_entity_mut(PLAYER_EID) {
        p.is_player = true;
        p.player_id = Some(PLAYER_ID);
        p.archetype_id = Some(archetype);
    }
    mgr.connect_entity(PLAYER_EID);
    mgr
}

/// One chain on `dispatcher`'s trigger for `DIALOG_ID`, gated
/// `archetype <op> 7`.
fn gated_engine(dispatcher: Dispatcher, operator: ComparisonOp) -> ChainEngine {
    let trigger = match dispatcher {
        Dispatcher::Open => Trigger::OnDialogOpen {
            dialog_id: DIALOG_ID,
        },
        Dispatcher::Choice => Trigger::OnDialogChoice {
            dialog_id: DIALOG_ID,
        },
    };
    let mut engine = ChainEngine::new();
    engine.register_chain(Chain {
        id: 9_271,
        name: "test: archetype-gated dialog".to_string(),
        enabled: true,
        trigger,
        conditions: vec![Condition::Archetype {
            operator,
            archetype_id: SHOLVA,
        }],
        actions: vec![Action::IncrementCounter {
            counter_name: COUNTER.to_string(),
            amount: 1,
        }],
        action_delays: Vec::new(),
        priority: 0,
        once: false,
    });
    engine
}

/// Fire `dispatcher` for a player of `archetype` against a chain gated
/// `archetype <op> 7`; true when the chain ran.
async fn fires(dispatcher: Dispatcher, operator: ComparisonOp, archetype: i32) -> bool {
    let mut mgr = make_mgr(archetype);
    let engine = gated_engine(dispatcher, operator);
    let (tx, _rx) = mpsc::channel(64);
    match dispatcher {
        Dispatcher::Open => {
            fire_dialog_open(PLAYER_EID, PLAYER_ID, DIALOG_ID, &engine, &tx, &mut mgr).await
        }
        Dispatcher::Choice => {
            fire_dialog_choice(PLAYER_EID, PLAYER_ID, DIALOG_ID, 1, &engine, &tx, &mut mgr).await
        }
    }
    mgr.get_entity(PLAYER_EID)
        .expect("player entity must still exist")
        .counters
        .contains_key(COUNTER)
}

/// `archetype eq 7` fires for a Shol'va and not for a Soldier, on both
/// dialog triggers. The positive half is the guard: with the param unset
/// the condition reads -1 and never matches.
#[tokio::test]
async fn dialog_archetype_eq_matches_only_that_archetype() {
    for dispatcher in [Dispatcher::Open, Dispatcher::Choice] {
        assert!(
            fires(dispatcher, ComparisonOp::Eq, SHOLVA).await,
            "{dispatcher:?}: `archetype eq 7` must fire for a Shol'va player",
        );
        assert!(
            !fires(dispatcher, ComparisonOp::Eq, SOLDIER).await,
            "{dispatcher:?}: `archetype eq 7` must not fire for a Soldier",
        );
    }
}

/// `archetype neq 7` keeps a Shol'va out and lets a Soldier in. The first
/// half is the guard: with the param unset `-1 != 7` and the chain fires.
#[tokio::test]
async fn dialog_archetype_neq_excludes_that_archetype() {
    for dispatcher in [Dispatcher::Open, Dispatcher::Choice] {
        assert!(
            !fires(dispatcher, ComparisonOp::Neq, SHOLVA).await,
            "{dispatcher:?}: `archetype neq 7` must not fire for a Shol'va player",
        );
        assert!(
            fires(dispatcher, ComparisonOp::Neq, SOLDIER).await,
            "{dispatcher:?}: `archetype neq 7` must fire for a Soldier",
        );
    }
}
