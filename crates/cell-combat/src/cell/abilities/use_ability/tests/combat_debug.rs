//! AB-N1 pipeline guards: a debugged cast sends its debug lines to the
//! client as feedback-channel chat lines, and an undebugged one sends none.
//!
//! The lines come from the notes the fire, the hit and the NVP pipeline
//! leave (`damage_apply::debug_notes`) and go out from the flush where the
//! cast's scope closes (`handle.rs`, `warmup/tick.rs`). Removing a note, the
//! flush, or the routing fails these asserts: with no line, the `[CD #`
//! texts below are missing.

use cimmeria_cell_world::cell::combat_debug::commands::{toggle, toggle_mob, Toggle};

use super::warmup::{after_warmup, warmup_mgr, INSTANT_ABILITY, WARMUP_ABILITY};
use super::*;
use crate::cell::abilities::resolve_warmups;
use crate::test_support::{LogCapture, NoContentEvents};

const PLAYER: u32 = 1;
const NPC: u32 = 2;
/// The NPC's HEALTH in `warmup_mgr`.
const FULL: i32 = 100_000;

/// The NPC's HEALTH now; the hit's damage is `FULL` minus it (the roll is
/// seeded, so it is fixed, but the test reads it rather than pins it).
fn npc_health(mgr: &SpaceManager) -> i32 {
    let hp = cimmeria_entity::stats::HEALTH;
    mgr.get_entity(NPC).unwrap().stats.get(hp).unwrap().cur
}

/// The text of every `onPlayerCommunication` (28) sent to `to`.
fn chat_lines(msgs: &[CellToBaseMsg], to: u32) -> Vec<String> {
    msgs.iter()
        .filter_map(|m| match m {
            CellToBaseMsg::EntityMethodCall {
                entity_id,
                method_index: 28,
                args,
            } if *entity_id == to => {
                let spk = u32::from_le_bytes(args[0..4].try_into().unwrap()) as usize;
                let off = 4 + spk * 2;
                assert_eq!(args[off + 1], 9, "CHAN_FEEDBACK");
                let off = off + 2;
                let n = u32::from_le_bytes(args[off..off + 4].try_into().unwrap()) as usize;
                let units: Vec<u16> = (0..n)
                    .map(|i| u16::from_le_bytes([args[off + 4 + i * 2], args[off + 5 + i * 2]]))
                    .collect();
                Some(String::from_utf16(&units).unwrap())
            }
            _ => None,
        })
        .collect()
}

fn debug_lines(msgs: &[CellToBaseMsg], to: u32) -> Vec<String> {
    chat_lines(msgs, to)
        .into_iter()
        .filter(|t| t.starts_with("[CD"))
        .collect()
}

/// **Regression guard.** With combat debug on, the player's zero-warmup
/// shot sends one simple line: the cast id, the ability, caster and target,
/// the roll and the target's pools (the 5 HP NVP hit).
#[tokio::test]
async fn a_debugged_cast_sends_its_hit_line() {
    let mut mgr = warmup_mgr();
    toggle(&mut mgr, PLAYER, Toggle::Combat).unwrap();
    let (tx, mut rx) = mpsc::channel(256);
    assert!(handle_use_ability(PLAYER, INSTANT_ABILITY, NPC as i32, &tx, &mut mgr).await);
    let lines = debug_lines(&drain(&mut rx), PLAYER);
    assert_eq!(lines.len(), 1, "one simple line: {lines:?}");
    let line = &lines[0];
    let hp = npc_health(&mgr);
    assert!(hp < FULL, "the shot landed");
    assert!(
        line.starts_with("[CD #1] charged (51) entity 1 -> entity 2: hit, roll "),
        "{line}"
    );
    let pools = format!("HP {FULL}->{hp} ({:+}), FP 0->0 (+0)", hp - FULL);
    assert!(line.ends_with(&pools), "{line}");
}

/// **Regression guard.** No one debugging: the same cast sends no debug
/// line at all, and nothing is left open.
#[tokio::test]
async fn an_undebugged_cast_sends_nothing() {
    let mut mgr = warmup_mgr();
    let (tx, mut rx) = mpsc::channel(256);
    assert!(handle_use_ability(PLAYER, INSTANT_ABILITY, NPC as i32, &tx, &mut mgr).await);
    assert!(chat_lines(&drain(&mut rx), PLAYER).is_empty());
    assert!(!mgr.combat_debug.is_active());
}

/// Verbose adds the effect plan and the NVP entry with its pools, and the
/// `abilities.debug` row of each line carries the exact text sent.
#[tokio::test]
async fn verbose_adds_the_plan_and_nvp_lines_and_rows_match() {
    let mut mgr = warmup_mgr();
    toggle(&mut mgr, PLAYER, Toggle::Verbose).unwrap();
    let (tx, mut rx) = mpsc::channel(256);
    let logs = LogCapture::install();
    assert!(handle_use_ability(PLAYER, INSTANT_ABILITY, NPC as i32, &tx, &mut mgr).await);
    let lines = debug_lines(&drain(&mut rx), PLAYER);
    let hp = npc_health(&mgr);
    let dealt = FULL - hp;
    let nvp = format!(
        "[CD #1]  nvp eff 500 -> entity 2: base H5 F0, dealt H{dealt} F0, absorbed 0 \
         (hit_roll); HP {FULL}->{hp} ({:+}), FP 0->0 (+0)",
        -dealt
    );
    let plan = "[CD #1]  plan eff 500 -> entity 2: nvp (hit_roll)".to_string();
    assert_eq!(lines[1..], [plan, nvp], "{lines:#?}");
    let rows: Vec<String> = logs
        .all()
        .into_iter()
        .filter(|c| c.has_field("event", "combat_debug_line"))
        .map(|c| c.fields.get("text").cloned().unwrap_or_default())
        .collect();
    assert_eq!(rows, lines, "the rows are the client text, line for line");
}

/// A warmup cast prints when it fires from the warmup tick, not at launch.
#[tokio::test]
async fn a_warmup_cast_prints_at_its_fire() {
    let mut mgr = warmup_mgr();
    toggle(&mut mgr, PLAYER, Toggle::Combat).unwrap();
    let (tx, mut rx) = mpsc::channel(256);
    assert!(handle_use_ability(PLAYER, WARMUP_ABILITY, NPC as i32, &tx, &mut mgr).await);
    assert!(
        debug_lines(&drain(&mut rx), PLAYER).is_empty(),
        "nothing yet"
    );
    resolve_warmups(after_warmup(), &tx, &mut mgr, &NoContentEvents).await;
    let lines = debug_lines(&drain(&mut rx), PLAYER);
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(lines[0].contains("charged (50)"), "{lines:?}");
}

/// Mob debug prints the selected mob's cast to the GM, with no other
/// toggle on.
#[tokio::test]
async fn mob_debug_prints_the_mobs_cast() {
    let mut mgr = warmup_mgr();
    mgr.get_entity_mut(PLAYER).unwrap().current_target_id = Some(NPC as i32);
    toggle_mob(&mut mgr, PLAYER, 0).unwrap();
    mgr.get_entity_mut(NPC)
        .unwrap()
        .abilities
        .add_ability(INSTANT_ABILITY);
    let (tx, mut rx) = mpsc::channel(256);
    assert!(handle_use_ability(NPC, INSTANT_ABILITY, PLAYER as i32, &tx, &mut mgr).await);
    let lines = debug_lines(&drain(&mut rx), PLAYER);
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(lines[0].contains("entity 2 -> entity 1"), "{lines:?}");
}
