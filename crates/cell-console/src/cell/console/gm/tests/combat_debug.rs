//! AB-N1: the native combat-debug GM commands (`gmDebugAbility` 169,
//! `gmDebugCombat` 170, `gmDebugCombatVerbose` 171, `gmDebugHeal` 172,
//! `gmDebugAbilityOnMob` 176).
//!
//! Each index is refused for a non-GM before any handler runs; a GM's press
//! flips the state and is answered on the first press; a refusal answers
//! too; each writes one `gm_command` row.

use super::*;
use crate::cell::dispatch::gm_gate::{enforce_gm_gate, requires_gm};
use crate::test_support::LogCapture;

const GM: u32 = 1;
const MOB: u32 = 50;
const ABILITY: i32 = 592;

const AB_N1: [u16; 5] = [
    GM_DEBUG_ABILITY,
    GM_DEBUG_COMBAT,
    GM_DEBUG_COMBAT_VERBOSE,
    GM_DEBUG_HEAL,
    GM_DEBUG_ABILITY_ON_MOB,
];

fn fixture() -> SpaceManager {
    let mut mgr = mgr_with_player(GM, "Castle");
    mgr.spawn_npc(MOB, "Castle", [2.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    cimmeria_cell_world::test_fixtures::seed_ability_defs(&mut mgr, &[ABILITY]);
    mgr
}

async fn call(mgr: &mut SpaceManager, index: u16, args: &[u8]) -> Vec<CellToBaseMsg> {
    let (tx, mut rx) = mpsc::channel(32);
    assert!(
        dispatch(GM, index, args, &tx, mgr, &test_engine()).await,
        "index {index} must be handled, not fall through"
    );
    drain(&mut rx)
}

fn settings(mgr: &SpaceManager) -> cimmeria_cell_world::cell::combat_debug::DebugSettings {
    mgr.combat_debug.settings(GM).cloned().unwrap_or_default()
}

/// **Server authority.** Every AB-N1 index sits in the GM tail, and the
/// gate refuses it for a player with an `onErrorCode`, before a handler.
#[tokio::test]
async fn ab_n1_indices_are_gated_for_non_gms() {
    for idx in AB_N1 {
        assert!(requires_gm(idx), "{idx} must be GM-gated");
        let mut mgr = fixture();
        mgr.get_entity_mut(GM).unwrap().access_level = 0;
        let (tx, mut rx) = mpsc::channel(8);
        assert!(!enforce_gm_gate(GM, idx, &tx, &mgr).await, "{idx}: refused");
        let msgs = drain(&mut rx);
        assert!(
            matches!(
                msgs.as_slice(),
                [CellToBaseMsg::EntityMethodCall {
                    method_index: 121,
                    ..
                }]
            ),
            "{idx}: only onErrorCode: {msgs:?}"
        );
        assert!(mgr.combat_debug.settings(GM).is_none(), "{idx}: no state");
        mgr.get_entity_mut(GM).unwrap().access_level = 2;
        assert!(
            enforce_gm_gate(GM, idx, &tx, &mgr).await,
            "{idx}: GM passes"
        );
    }
}

/// 170, 171, 172 flip their toggle and answer on the first press, on and
/// off, each with one `gm_command` row.
#[tokio::test]
async fn the_toggles_flip_and_answer_every_press() {
    for (idx, cmd, flag) in [
        (GM_DEBUG_COMBAT, "gmDebugCombat", 0),
        (GM_DEBUG_COMBAT_VERBOSE, "gmDebugCombatVerbose", 1),
        (GM_DEBUG_HEAL, "gmDebugHeal", 2),
    ] {
        let mut mgr = fixture();
        let read = |m: &SpaceManager| {
            let s = settings(m);
            [s.combat, s.verbose, s.heal][flag]
        };
        let logs = LogCapture::install();
        let on = call(&mut mgr, idx, &[]).await;
        assert!(read(&mgr), "{cmd}: on after the first press");
        let text = feedback_text(&on, GM).expect("first press answered");
        assert!(
            text.starts_with(&format!("{cmd}: ")) && text.contains(" on"),
            "{text}"
        );
        let off = call(&mut mgr, idx, &[]).await;
        assert!(!read(&mgr), "{cmd}: off after the second");
        assert!(feedback_text(&off, GM).unwrap().ends_with("off"));
        let rows = logs
            .all()
            .into_iter()
            .filter(|c| c.has_field("event", "gm_command") && c.has_field("cmd", cmd))
            .count();
        assert_eq!(rows, 2, "{cmd}: one row per press");
        assert!(
            mgr.combat_debug.settings(GM).is_none(),
            "all off: forgotten"
        );
    }
}

/// 169 lists the ability, refuses an unknown id, and 0 clears.
#[tokio::test]
async fn gm_debug_ability_toggles_the_list() {
    let mut mgr = fixture();
    let msgs = call(&mut mgr, GM_DEBUG_ABILITY, &ABILITY.to_le_bytes()).await;
    assert_eq!(settings(&mgr).abilities, vec![ABILITY]);
    assert!(feedback_text(&msgs, GM)
        .unwrap()
        .contains("TestAbility592 (592)"));

    let msgs = call(&mut mgr, GM_DEBUG_ABILITY, &777_777i32.to_le_bytes()).await;
    assert!(feedback_text(&msgs, GM)
        .unwrap()
        .contains("no ability 777777"));
    assert_eq!(
        settings(&mgr).abilities,
        vec![ABILITY],
        "refusal changes nothing"
    );

    let msgs = call(&mut mgr, GM_DEBUG_ABILITY, &0i32.to_le_bytes()).await;
    assert!(feedback_text(&msgs, GM).unwrap().contains("cleared"));
    assert!(mgr.combat_debug.settings(GM).is_none());

    let msgs = call(&mut mgr, GM_DEBUG_ABILITY, &[1, 2]).await;
    assert!(feedback_text(&msgs, GM)
        .unwrap()
        .contains("missing INT32 aAbilityId"));
}

/// 176 needs a selected mob; with one, it adds the mob.
#[tokio::test]
async fn gm_debug_ability_on_mob_needs_a_selected_mob() {
    let mut mgr = fixture();
    let msgs = call(&mut mgr, GM_DEBUG_ABILITY_ON_MOB, &0i32.to_le_bytes()).await;
    assert!(feedback_text(&msgs, GM)
        .unwrap()
        .contains("select a mob first"));
    mgr.get_entity_mut(GM).unwrap().current_target_id = Some(MOB as i32);
    let msgs = call(&mut mgr, GM_DEBUG_ABILITY_ON_MOB, &0i32.to_le_bytes()).await;
    let text = feedback_text(&msgs, GM).unwrap();
    assert!(
        text.contains("Mob debug on") && text.contains("all its abilities"),
        "{text}"
    );
    assert_eq!(settings(&mgr).mobs, vec![(MOB, 0)]);
}
