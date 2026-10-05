//! AB-N1: the crafted-caller combat-debug cells 2 (`toggleCombatDebug`), 3
//! (`toggleCombatVerboseDebug`) and 6 (`toggleHealDebug`) through the full
//! router.
//!
//! The stock client never sends them, but they are exposed, so a crafted
//! packet reaches the router. For each: a player (access level 0) gets only
//! the gate's `onErrorCode` and no state; a GM's call flips the toggle,
//! answers with exactly one feedback line, and writes exactly one
//! `gm_command` row. Removing an index from `requires_gm` fails the player
//! half; reverting the handler to its old no-op fails the GM half.

use tokio::sync::mpsc;

use super::super::messages::CellToBaseMsg;
use super::super::space_manager::SpaceManager;
use super::constants::{
    CM_TOGGLE_COMBAT_DEBUG, CM_TOGGLE_COMBAT_VERBOSE_DEBUG, CM_TOGGLE_HEAL_DEBUG,
};
use super::*;
use crate::test_support::LogCapture;

const CALLER: u32 = 1;
/// `onErrorCode`.
const ON_ERROR_CODE: u16 = 121;
/// `onPlayerCommunication`.
const ON_PLAYER_COMMUNICATION: u16 = 28;

/// `(index, cmd, which flag)`: 0 combat, 1 verbose, 2 heal.
const CELLS: [(u16, &str, usize); 3] = [
    (CM_TOGGLE_COMBAT_DEBUG, "toggleCombatDebug", 0),
    (
        CM_TOGGLE_COMBAT_VERBOSE_DEBUG,
        "toggleCombatVerboseDebug",
        1,
    ),
    (CM_TOGGLE_HEAL_DEBUG, "toggleHealDebug", 2),
];

fn world(access_level: u32) -> SpaceManager {
    let mut mgr = crate::test_support::make_space_manager();
    mgr.create_entity(CALLER, "Agnos", [0.0; 3], [0.0; 3])
        .unwrap();
    let p = mgr.get_entity_mut(CALLER).unwrap();
    p.is_player = true;
    p.player_id = Some(100);
    p.account_id = Some(9100);
    p.access_level = access_level;
    mgr
}

fn flag(mgr: &SpaceManager, which: usize) -> bool {
    mgr.combat_debug
        .settings(CALLER)
        .is_some_and(|s| [s.combat, s.verbose, s.heal][which])
}

async fn route(mgr: &mut SpaceManager, index: u16) -> Vec<CellToBaseMsg> {
    let engine = cimmeria_content_engine::chain::ChainEngine::new();
    let (tx, mut rx) = mpsc::channel(32);
    dispatch_cell_method(CALLER, index, &[], &tx, mgr, &engine, None).await;
    std::iter::from_fn(|| rx.try_recv().ok()).collect()
}

fn count(msgs: &[CellToBaseMsg], method: u16) -> usize {
    msgs.iter()
        .filter(|m| {
            matches!(
                m,
                CellToBaseMsg::EntityMethodCall { method_index, .. } if *method_index == method
            )
        })
        .count()
}

#[tokio::test]
async fn crafted_debug_toggles_are_refused_for_a_player_through_the_router() {
    for (index, cmd, which) in CELLS {
        let mut mgr = world(0);
        let msgs = route(&mut mgr, index).await;
        assert_eq!(count(&msgs, ON_ERROR_CODE), 1, "{cmd}: {msgs:?}");
        assert_eq!(msgs.len(), 1, "{cmd}: only onErrorCode: {msgs:?}");
        assert!(!flag(&mgr, which), "{cmd}: no state for a player");
        assert!(!mgr.combat_debug.is_active(), "{cmd}");
    }
}

#[tokio::test]
async fn crafted_debug_toggles_flip_answer_and_log_once_for_a_gm() {
    for (index, cmd, which) in CELLS {
        let mut mgr = world(2);
        let logs = LogCapture::install();
        let msgs = route(&mut mgr, index).await;
        assert!(flag(&mgr, which), "{cmd}: the toggle is on");
        assert_eq!(count(&msgs, ON_ERROR_CODE), 0, "{cmd}: {msgs:?}");
        assert_eq!(
            count(&msgs, ON_PLAYER_COMMUNICATION),
            1,
            "{cmd}: one feedback line: {msgs:?}"
        );
        let rows: Vec<_> = logs
            .all()
            .into_iter()
            .filter(|c| c.has_field("event", "gm_command") && c.has_field("cmd", cmd))
            .collect();
        assert_eq!(rows.len(), 1, "{cmd}: one gm_command row: {rows:#?}");
        assert!(rows[0].has_field("decision_outcome", "applied"), "{rows:?}");
        assert!(rows[0].has_field("account_id", "9100"), "{rows:?}");
    }
}
