//! `player_entered_combat` and the one-time tutorials it gates (CS-03).
//!
//! In memory, no database: the base's half of `show_tutorial` is played by
//! hand (a `TutorialRecorded` built in the test), so the tests here pin the
//! cell's decisions and the wire frame it sends.

use std::collections::HashMap;

use tokio::sync::mpsc;

use cimmeria_content_engine::actions::Action;
use cimmeria_content_engine::chain::{Chain, ChainEngine};
use cimmeria_content_engine::conditions::{ComparisonOp, Condition};
use cimmeria_content_engine::triggers::Trigger;

use super::fire_pending_combat_entries;
use crate::cell::messages::{
    CellToBaseMsg, RecordTutorialShown, TutorialRecordOutcome, TutorialRecorded,
};
use crate::cell::space_manager::SpaceManager;
use crate::cell::spawner::WorldRow;

const PLAYER_EID: u32 = 1;
const MOB_A: u32 = 50;
const MOB_B: u32 = 51;
const PLAYER_ID: i32 = 100;
const SOLDIER: i32 = 1;
const COUNTER: &str = "combat_entries";
const EQUIPPING_A_WEAPON: i32 = 5882;
const COMBAT: i32 = 5883;
const COMBAT_CHAIN: i64 = 7101;

fn make_mgr() -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    mgr.parse_spaces_xml(
        r#"<?xml version="1.0"?><Spaces>
        <Space WorldName="Castle_CellBlock" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" />
    </Spaces>"#,
    )
    .unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" /></Spaces>"#,
    )
    .unwrap();
    mgr.stamp_world_rows(&HashMap::from([(
        "Castle_CellBlock".to_string(),
        WorldRow::enforcing(2),
    )]));
    mgr.create_entity(PLAYER_EID, "Castle_CellBlock", [0.0; 3], [0.0; 3])
        .unwrap();
    for mob in [MOB_A, MOB_B] {
        mgr.spawn_npc(mob, "Castle_CellBlock", [5.0, 0.0, 5.0], [0.0; 3])
            .unwrap();
    }
    if let Some(p) = mgr.get_entity_mut(PLAYER_EID) {
        p.is_player = true;
        p.player_id = Some(PLAYER_ID);
        p.archetype_id = Some(SOLDIER);
    }
    mgr.connect_entity(PLAYER_EID);
    mgr
}

fn chain(id: i64, conditions: Vec<Condition>, actions: Vec<Action>) -> Chain {
    Chain {
        id,
        name: format!("test: combat {id}"),
        enabled: true,
        trigger: Trigger::OnPlayerEnteredCombat,
        conditions,
        actions,
        action_delays: Vec::new(),
        priority: 0,
        once: false,
    }
}

/// The seeded 5883 chain's shape (`tutorial_chains.sql`, chain 7101).
fn combat_tutorial_engine() -> ChainEngine {
    let mut engine = ChainEngine::new();
    engine.register_chain(chain(
        COMBAT_CHAIN,
        vec![
            Condition::TutorialShown {
                tutorial_id: EQUIPPING_A_WEAPON,
                operator: ComparisonOp::Eq,
            },
            Condition::TutorialShown {
                tutorial_id: COMBAT,
                operator: ComparisonOp::Neq,
            },
        ],
        vec![Action::ShowTutorial {
            tutorial_id: COMBAT,
        }],
    ));
    engine
}

fn counter(mgr: &SpaceManager) -> i32 {
    mgr.get_entity(PLAYER_EID)
        .and_then(|p| p.counters.get(COUNTER).copied())
        .unwrap_or(0)
}

/// Every `RecordTutorialShown` the cell sent, in order.
fn records(rx: &mut mpsc::Receiver<CellToBaseMsg>) -> Vec<RecordTutorialShown> {
    let mut out = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        if let CellToBaseMsg::RecordTutorialShown(r) = msg {
            out.push(r);
        }
    }
    out
}

/// Every `onDialogDisplay` the cell sent, as its argument bytes.
fn dialog_displays(rx: &mut mpsc::Receiver<CellToBaseMsg>) -> Vec<Vec<u8>> {
    let mut out = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        if let CellToBaseMsg::EntityMethodCall {
            method_index, args, ..
        } = msg
        {
            if method_index == crate::mercury::method_idx::ON_DIALOG_DISPLAY {
                out.push(args);
            }
        }
    }
    out
}

/// **Guard: the trigger fires once per combat entry and carries the
/// `archetype` param.** A chain gated `archetype eq 1` counts entries for a
/// Soldier: one for the first mob, none for a second mob joining the
/// fight, one more after leaving combat and re-entering. Remove the queue
/// push in `enter_player_combat` or the drain from the tick and the count
/// stays 0; drop the `archetype` param and the gate reads -1 and never
/// passes.
#[tokio::test]
async fn player_entered_combat_fires_once_per_entry_with_archetype() {
    let mut engine = ChainEngine::new();
    engine.register_chain(chain(
        0x7007_0200,
        vec![Condition::Archetype {
            operator: ComparisonOp::Eq,
            archetype_id: SOLDIER,
        }],
        vec![Action::IncrementCounter {
            counter_name: COUNTER.to_string(),
            amount: 1,
        }],
    ));
    let mut mgr = make_mgr();
    let (tx, _rx) = mpsc::channel(64);

    let _ = crate::cell::combat::enter_player_combat(&mut mgr, PLAYER_EID, MOB_A);
    fire_pending_combat_entries(&engine, &tx, &mut mgr).await;
    assert_eq!(counter(&mgr), 1, "the first mob puts the player in combat");

    let _ = crate::cell::combat::enter_player_combat(&mut mgr, PLAYER_EID, MOB_B);
    fire_pending_combat_entries(&engine, &tx, &mut mgr).await;
    assert_eq!(counter(&mgr), 1, "a second mob is not a new combat entry");
    assert!(
        mgr.pending_combat_entries.is_empty(),
        "the drain empties the queue"
    );

    let _ = crate::cell::combat::exit_player_combat(&mut mgr, PLAYER_EID, MOB_A);
    let _ = crate::cell::combat::exit_player_combat(&mut mgr, PLAYER_EID, MOB_B);
    let _ = crate::cell::combat::enter_player_combat(&mut mgr, PLAYER_EID, MOB_A);
    fire_pending_combat_entries(&engine, &tx, &mut mgr).await;
    assert_eq!(
        counter(&mgr),
        2,
        "leaving and re-entering combat fires again"
    );
}

/// The same `archetype eq 1` chain does not fire for another archetype, so
/// the positive test cannot pass on an ungated chain.
#[tokio::test]
async fn player_entered_combat_archetype_gate_discriminates() {
    let mut engine = ChainEngine::new();
    engine.register_chain(chain(
        0x7007_0201,
        vec![Condition::Archetype {
            operator: ComparisonOp::Eq,
            archetype_id: SOLDIER,
        }],
        vec![Action::IncrementCounter {
            counter_name: COUNTER.to_string(),
            amount: 1,
        }],
    ));
    let mut mgr = make_mgr();
    mgr.get_entity_mut(PLAYER_EID).unwrap().archetype_id = Some(7);
    let (tx, _rx) = mpsc::channel(64);

    let _ = crate::cell::combat::enter_player_combat(&mut mgr, PLAYER_EID, MOB_A);
    fire_pending_combat_entries(&engine, &tx, &mut mgr).await;
    assert_eq!(counter(&mgr), 0);
}

/// **Guard: the 5883 chain needs 5882 shown.** A player who has not seen
/// "Equipping a Weapon" enters combat and nothing is asked of the base.
/// Drop the `tutorial_shown 5882 eq` gate (or let the condition fail open
/// on a missing set) and 5883 is requested before 5882.
#[tokio::test]
async fn combat_tutorial_waits_for_the_weapon_tutorial() {
    let engine = combat_tutorial_engine();
    let mut mgr = make_mgr();
    let (tx, mut rx) = mpsc::channel(64);

    let _ = crate::cell::combat::enter_player_combat(&mut mgr, PLAYER_EID, MOB_A);
    fire_pending_combat_entries(&engine, &tx, &mut mgr).await;

    assert!(records(&mut rx).is_empty(), "5883 must wait for 5882");
    assert!(!mgr
        .get_entity(PLAYER_EID)
        .unwrap()
        .shown_tutorials
        .contains(&COMBAT));
}

/// **Guard: the combat tutorial is displayed exactly once, byte for byte.**
/// With 5882 already shown, the first combat entry asks the base to record
/// 5883; the base's `First` answer sends one `onDialogDisplay` (method 105:
/// speaker = the player's own entity id, dialog 5883, flags 0, immediate 1,
/// mission 0). A second combat entry asks nothing (the cell's set already
/// holds 5883), and a duplicate `AlreadyShown` answer displays nothing.
/// Display on the request instead of the answer, or on `AlreadyShown`, and
/// the frame count is wrong.
#[tokio::test]
async fn combat_tutorial_dialog_is_sent_exactly_once() {
    let engine = combat_tutorial_engine();
    let mut mgr = make_mgr();
    mgr.get_entity_mut(PLAYER_EID)
        .unwrap()
        .shown_tutorials
        .insert(EQUIPPING_A_WEAPON);
    let (tx, mut rx) = mpsc::channel(64);

    let _ = crate::cell::combat::enter_player_combat(&mut mgr, PLAYER_EID, MOB_A);
    fire_pending_combat_entries(&engine, &tx, &mut mgr).await;
    let sent = records(&mut rx);
    assert_eq!(
        sent,
        vec![RecordTutorialShown {
            entity_id: PLAYER_EID,
            player_id: PLAYER_ID,
            chain_id: COMBAT_CHAIN,
            tutorial_id: COMBAT,
        }]
    );

    let answer = |outcome| TutorialRecorded {
        entity_id: PLAYER_EID,
        player_id: PLAYER_ID,
        chain_id: COMBAT_CHAIN,
        tutorial_id: COMBAT,
        outcome,
    };
    crate::cell::content::apply_tutorial_recorded(
        answer(TutorialRecordOutcome::First),
        &tx,
        &mut mgr,
    )
    .await;
    let frames = dialog_displays(&mut rx);
    let mut expected = Vec::new();
    expected.extend_from_slice(&(PLAYER_EID as i32).to_le_bytes());
    expected.extend_from_slice(&COMBAT.to_le_bytes());
    expected.extend_from_slice(&0i32.to_le_bytes());
    expected.push(1);
    expected.extend_from_slice(&0i32.to_le_bytes());
    assert_eq!(frames, vec![expected], "one onDialogDisplay for 5883");

    // Leave combat, re-enter: nothing more is asked or shown.
    let _ = crate::cell::combat::exit_player_combat(&mut mgr, PLAYER_EID, MOB_A);
    let _ = crate::cell::combat::enter_player_combat(&mut mgr, PLAYER_EID, MOB_B);
    fire_pending_combat_entries(&engine, &tx, &mut mgr).await;
    crate::cell::content::apply_tutorial_recorded(
        answer(TutorialRecordOutcome::AlreadyShown),
        &tx,
        &mut mgr,
    )
    .await;
    let mut rest = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        rest.push(msg);
    }
    assert!(
        rest.iter().all(|m| !matches!(
            m,
            CellToBaseMsg::RecordTutorialShown(_)
                | CellToBaseMsg::EntityMethodCall {
                    method_index: crate::mercury::method_idx::ON_DIALOG_DISPLAY,
                    ..
                }
        )),
        "a second combat entry neither records nor displays 5883"
    );
}

/// **Guard: a refused record shows nothing and can be retried.** The base
/// could not write the row, so the cell displays nothing (a tutorial shown
/// without a row would replay after a relog) and drops its optimistic mark,
/// so the next combat entry asks again.
#[tokio::test]
async fn refused_record_shows_nothing_and_allows_a_retry() {
    let engine = combat_tutorial_engine();
    let mut mgr = make_mgr();
    mgr.get_entity_mut(PLAYER_EID)
        .unwrap()
        .shown_tutorials
        .insert(EQUIPPING_A_WEAPON);
    let (tx, mut rx) = mpsc::channel(64);

    let _ = crate::cell::combat::enter_player_combat(&mut mgr, PLAYER_EID, MOB_A);
    fire_pending_combat_entries(&engine, &tx, &mut mgr).await;
    assert_eq!(records(&mut rx).len(), 1);
    crate::cell::content::apply_tutorial_recorded(
        TutorialRecorded {
            entity_id: PLAYER_EID,
            player_id: PLAYER_ID,
            chain_id: COMBAT_CHAIN,
            tutorial_id: COMBAT,
            outcome: TutorialRecordOutcome::Refused,
        },
        &tx,
        &mut mgr,
    )
    .await;
    assert!(dialog_displays(&mut rx).is_empty(), "nothing is displayed");
    assert!(!mgr
        .get_entity(PLAYER_EID)
        .unwrap()
        .shown_tutorials
        .contains(&COMBAT));

    let _ = crate::cell::combat::exit_player_combat(&mut mgr, PLAYER_EID, MOB_A);
    let _ = crate::cell::combat::enter_player_combat(&mut mgr, PLAYER_EID, MOB_A);
    fire_pending_combat_entries(&engine, &tx, &mut mgr).await;
    assert_eq!(records(&mut rx).len(), 1, "the next entry asks again");
}

/// A world entry that hydrates `shown_tutorials` (a relog, a world change)
/// makes `show_tutorial` a no-op: nothing is asked of the base.
#[tokio::test]
async fn a_hydrated_tutorial_is_never_requested_again() {
    let engine = combat_tutorial_engine();
    let mut mgr = make_mgr();
    mgr.get_entity_mut(PLAYER_EID)
        .unwrap()
        .shown_tutorials
        .extend([EQUIPPING_A_WEAPON, COMBAT]);
    let (tx, mut rx) = mpsc::channel(64);

    let _ = crate::cell::combat::enter_player_combat(&mut mgr, PLAYER_EID, MOB_A);
    fire_pending_combat_entries(&engine, &tx, &mut mgr).await;
    assert!(records(&mut rx).is_empty());
}
