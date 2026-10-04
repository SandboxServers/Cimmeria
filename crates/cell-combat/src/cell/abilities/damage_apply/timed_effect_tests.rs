//! AB-04: a hostile single-pulse timed effect (a debuff) reaches the timed
//! effect ledger through `damage_apply`: it lands on a hit with its duration
//! icon in the same resolution, and a miss lands nothing.
//!
//! The effect is Call Target's 903 as the `stat` family writes it
//! (`TimedStat`, `Defense -100`, 15 s, flags 20 = ClearOnDeath | DontUseQR),
//! or the same row without `EF_DontUseQR` so the roll can miss.

use super::single_damage_path_tests::seq_rolling;
use super::tests::{drain, make_ability, make_mgr_player_vs_npc};
use super::*;
use crate::cell::client_methods::being::ON_TIMER_UPDATE;
use crate::cell::space_manager::SpaceManager;
use cimmeria_entity::abilities::{EffectDef, EF_CLEAR_ON_DEATH, EF_DONT_USE_QR};
use cimmeria_entity::stats::DEFENSE;

const CALL_TARGET: i32 = 847;
const CALL_TARGET_EFFECT: i32 = 903;
const PLAYER: u32 = 1;
const NPC: u32 = 2;

fn call_target(flags: u32) -> EffectDef {
    EffectDef {
        effect_id: CALL_TARGET_EFFECT,
        ability_id: CALL_TARGET,
        script_name: Some("TimedStat".to_string()),
        flags,
        pulse_count: 1,
        pulse_duration: 15.0,
        params: [("Defense".to_string(), "-100".to_string())].into(),
        ..Default::default()
    }
}

fn fixture(flags: u32) -> (SpaceManager, AbilityDef) {
    let mut mgr = make_mgr_player_vs_npc();
    let ability = make_ability(CALL_TARGET, vec![CALL_TARGET_EFFECT]);
    mgr.ability_defs.insert(CALL_TARGET, ability.clone());
    mgr.effect_defs
        .insert(CALL_TARGET_EFFECT, call_target(flags));
    (mgr, ability)
}

fn defense(mgr: &SpaceManager, eid: u32) -> i32 {
    mgr.get_entity(eid).unwrap().stats.get(DEFENSE).unwrap().cur
}

/// The `onTimerUpdate` args `eid`'s own client was sent.
fn timers_to(msgs: &[CellToBaseMsg], eid: u32) -> Vec<Vec<u8>> {
    msgs.iter()
        .filter_map(|m| match m {
            CellToBaseMsg::EntityMethodCall {
                entity_id,
                method_index,
                args,
            } if *entity_id == eid && *method_index == ON_TIMER_UPDATE => Some(args.clone()),
            _ => None,
        })
        .collect()
}

/// **Regression guard (AB-04 hand-off).** An NPC's Call Target on a player
/// lowers the player's Defense by 100 and, in the same resolution, sends the
/// player's client the duration icon: timer id 903, `TIMER_DURATION_EFFECT`,
/// the NPC as source, SecondaryId 903, 15 s. On revert of the
/// `flush_stat_buff_timers` call in `damage_apply` the icon waits for the
/// next stat-buff tick (`one duration icon` fails); on revert of the
/// `stat` family's `TimedStat` script Defense stays 0.
#[tokio::test]
async fn a_debuff_lands_on_a_hit_with_its_icon() {
    let (mut mgr, ability) = fixture(EF_CLEAR_ON_DEATH | EF_DONT_USE_QR);
    let (tx, mut rx) = mpsc::channel(256);

    apply_damage_to_target(
        NPC,
        PLAYER,
        CALL_TARGET,
        &Some(ability),
        1,
        false,
        &tx,
        &mut mgr,
    )
    .await;
    let msgs = drain(&mut rx);

    assert_eq!(defense(&mgr, PLAYER), -100, "Defense -100 for 15 s");
    let entry = &mgr.get_entity(PLAYER).unwrap().stat_buffs.entries[0];
    assert_eq!(
        entry.key(),
        (CALL_TARGET_EFFECT, NPC),
        "keyed by (effect, invoker)"
    );
    let t = timers_to(&msgs, PLAYER);
    assert_eq!(t.len(), 1, "one duration icon: {msgs:?}");
    assert_eq!(
        &t[0][..17],
        &[
            0x87, 0x03, 0x00, 0x00, // id = 903
            0x05, // TIMER_DURATION_EFFECT
            0x02, 0x00, 0x00, 0x00, // source = the NPC
            0x87, 0x03, 0x00, 0x00, // SecondaryId = 903
            0x00, 0x00, 0x70, 0x41, // TotalTime 15.0f32
        ][..]
    );
}

/// **Guard (miss gate).** A QR-rolled debuff whose roll misses lands
/// nothing: no Defense change, no ledger entry, no icon.
#[tokio::test]
async fn a_missed_debuff_lands_nothing() {
    let (mut mgr, ability) = fixture(EF_CLEAR_ON_DEATH);
    let seq = seq_rolling(&mgr, (NPC, PLAYER), CALL_TARGET, true);
    let (tx, mut rx) = mpsc::channel(256);

    apply_damage_to_target(
        NPC,
        PLAYER,
        CALL_TARGET,
        &Some(ability),
        seq,
        false,
        &tx,
        &mut mgr,
    )
    .await;
    let msgs = drain(&mut rx);

    assert_eq!(defense(&mgr, PLAYER), 0);
    assert!(mgr.get_entity(PLAYER).unwrap().stat_buffs.is_idle());
    assert!(timers_to(&msgs, PLAYER).is_empty());
}

/// The same row on a hit (the seed that does not miss) lands, so the miss
/// test above is the roll's doing, not the fixture's.
#[tokio::test]
async fn the_same_qr_rolled_debuff_lands_on_a_hit() {
    let (mut mgr, ability) = fixture(EF_CLEAR_ON_DEATH);
    let seq = seq_rolling(&mgr, (NPC, PLAYER), CALL_TARGET, false);
    let (tx, _rx) = mpsc::channel(256);

    apply_damage_to_target(
        NPC,
        PLAYER,
        CALL_TARGET,
        &Some(ability),
        seq,
        false,
        &tx,
        &mut mgr,
    )
    .await;

    assert_eq!(defense(&mgr, PLAYER), -100);
}
