//! NA32: cover in the hit roll, driven through `apply_damage_to_target`.
//!
//! The NPC stands at a cover node at the origin facing +X. The player shoots
//! from +X (in front: covered) or from -X (behind: flanked). Every case uses
//! the same ability, effect sequence and so the same beta-roll seed, so the
//! only thing that moves the damage is the cover term.

use super::tests::{make_ability, make_mgr_player_vs_npc};
use super::*;
use crate::cell::cover::{Cover, CoverHeight, CoverNode, CoverQuality, CoverSlotKey};
use crate::test_support::LogCapture;
use cimmeria_common::{EntityId, Vector3};
use cimmeria_entity::abilities::EffectDef;
use cimmeria_entity::stats::{StatList, ACCURACY, COVER_DEFENSE};

const PLAYER: u32 = 1;
const NPC: u32 = 2;
const ABILITY: i32 = 7;
const EFFECT: i32 = 100;
const SEQ: u32 = 1;
/// World id the fixture's cover node carries (Castle_CellBlock).
const WORLD: i32 = 12;
const SLOT: CoverSlotKey = CoverSlotKey {
    chunk_id: 50,
    node_id: 0,
};

fn set_stat(stats: &mut StatList, id: i32, v: i32) {
    let s = stats.get_mut(id).unwrap();
    s.update(0, v, v.max(1000));
    s.clear_dirty();
}

/// Player at `player_pos`, NPC at the origin with 1000 HP, one cover node at
/// the origin facing +X. The player has 150 accuracy (+1.5 QR) so the
/// covered hit still does damage and the magnitude is visible.
fn fixture(player_pos: [f32; 3], npc_holds_slot: bool, npc_cover_defense: i32) -> SpaceManager {
    let mut mgr = make_mgr_player_vs_npc();
    mgr.stamp_world_rows(&std::collections::HashMap::from([(
        "Castle".to_string(),
        crate::cell::spawner::WorldRow::enforcing(WORLD),
    )]));
    mgr.get_entity_mut(PLAYER).unwrap().position =
        Vector3::new(player_pos[0], player_pos[1], player_pos[2]);
    set_stat(
        &mut mgr.get_entity_mut(PLAYER).unwrap().stats,
        ACCURACY,
        150,
    );
    let npc = mgr.get_entity_mut(NPC).unwrap();
    set_stat(&mut npc.stats, HEALTH, 1000);
    set_stat(&mut npc.stats, COVER_DEFENSE, npc_cover_defense);
    mgr.cover = Cover::from_loaded(
        Vec::new(),
        vec![CoverNode {
            chunk_id: SLOT.chunk_id,
            node_id: SLOT.node_id,
            world_id: WORLD,
            pos: Vector3::new(0.0, 0.0, 0.0),
            orient: 0.0,
            height: CoverHeight::Mid,
            quality: CoverQuality::Best,
            width: 1.0,
            tail: [0; 4],
        }],
    );
    if npc_holds_slot {
        mgr.cover
            .reservations
            .lock()
            .unwrap()
            .reserve_for_entity(EntityId(NPC as i32), SLOT)
            .unwrap();
    }
    let mut params = std::collections::HashMap::new();
    params.insert("HealthDamage".to_string(), "20".to_string());
    mgr.effect_defs.insert(
        EFFECT,
        EffectDef {
            effect_id: EFFECT,
            params,
            ..Default::default()
        },
    );
    mgr.ability_defs
        .insert(ABILITY, make_ability(ABILITY, vec![EFFECT]));
    mgr
}

/// HEALTH lost by `target` to one hit from `attacker`.
async fn damage(mgr: &mut SpaceManager, attacker: u32, target: u32) -> i32 {
    let before = mgr
        .get_entity(target)
        .unwrap()
        .stats
        .get(HEALTH)
        .unwrap()
        .cur;
    let (tx, _rx) = mpsc::channel(256);
    let def = mgr.ability_defs.get(&ABILITY).cloned();
    apply_damage_to_target(attacker, target, ABILITY, &def, SEQ, false, &tx, mgr).await;
    before
        - mgr
            .get_entity(target)
            .unwrap()
            .stats
            .get(HEALTH)
            .unwrap()
            .cur
}

/// The damage the pipeline gives for this fixture's hit at `qr_shift`,
/// computed straight from the combat functions.
fn expected(mgr: &SpaceManager, attacker: u32, target: u32, base: i32, qr_shift: f64) -> i32 {
    let att = mgr.get_entity(attacker).unwrap().stats.clone();
    let mut def = mgr.get_entity(target).unwrap().stats.clone();
    let qr = combat::calculate_qr(&att, &def, false) + qr_shift;
    let seed = super::super::rng::pseudo_random_seed(attacker, ABILITY, SEQ);
    let roll = combat::calculate_result(qr, seed);
    combat::calculate_damage(&roll, base, DT_PHYSICAL, HEALTH, &att, &mut def).1
}

/// Cover Stance's +100 in cover facing the shooter is exactly -1.0 QR
/// (`alias.xml:235`, 0.01 per point). Revert the wiring and the covered hit
/// does the uncovered damage; change the scale and it misses `expected`.
#[tokio::test]
async fn covered_npc_takes_the_minus_one_qr_hit() {
    let mut open = fixture([20.0, 0.0, 0.0], false, 0);
    let exposed = damage(&mut open, PLAYER, NPC).await;

    let mut mgr = fixture([20.0, 0.0, 0.0], true, 100);
    // Player damage is doubled upstream (the temporary 2x in `mod.rs`).
    let want = expected(&mgr, PLAYER, NPC, 40, -1.0);
    let logs = LogCapture::install();
    let covered = damage(&mut mgr, PLAYER, NPC).await;

    assert!(exposed > 0 && covered > 0, "{exposed} / {covered}");
    assert!(
        covered < exposed,
        "cover must reduce the hit: {covered} vs {exposed}"
    );
    assert_eq!(covered, want, "the cover term is -0.01 QR per point");
    let row = logs
        .all()
        .into_iter()
        .find(|c| c.target == "abilities.qr")
        .expect("a hit on a covered NPC writes the abilities.qr row");
    assert!(row.has_field("defender_cover", "in_cover"), "{row:?}");
    assert!(row.has_field("flanked", "false"), "{row:?}");
    assert!(row.has_field("cover_defense_applied", "1.0"), "{row:?}");
}

/// Shot from behind, the same NPC with the same +100 takes the uncovered
/// hit: a flanked defender gets nothing.
#[tokio::test]
async fn flanked_npc_gets_no_cover() {
    let mut open = fixture([-20.0, 0.0, 0.0], false, 0);
    let exposed = damage(&mut open, PLAYER, NPC).await;
    let mut mgr = fixture([-20.0, 0.0, 0.0], true, 100);
    let logs = LogCapture::install();
    let flanked = damage(&mut mgr, PLAYER, NPC).await;
    assert_eq!(flanked, exposed, "flanked cover is no cover");
    let row = logs
        .all()
        .into_iter()
        .find(|c| c.target == "abilities.qr")
        .expect("the flank is reported too");
    assert!(row.has_field("flanked", "true"), "{row:?}");
    assert!(row.has_field("cover_defense_applied", "0.0"), "{row:?}");
}

/// The raised stat without the slot (an NPC that left cover before the
/// stance came off, or a buff out of cover) changes nothing.
#[tokio::test]
async fn cover_defense_away_from_cover_changes_nothing() {
    let mut open = fixture([20.0, 0.0, 0.0], false, 0);
    let exposed = damage(&mut open, PLAYER, NPC).await;
    let mut mgr = fixture([20.0, 0.0, 0.0], false, 100);
    assert_eq!(damage(&mut mgr, PLAYER, NPC).await, exposed);
}

/// A player standing at a node with a cover-defense buff is covered against
/// an NPC in front of the node, with no slot to hold.
#[tokio::test]
async fn covered_player_takes_less_from_an_npc() {
    // Player at the node, the NPC shooting from +X.
    let build = |def: i32| {
        let mut mgr = fixture([0.5, 0.0, 0.0], false, 0);
        mgr.get_entity_mut(NPC).unwrap().position = Vector3::new(20.0, 0.0, 0.0);
        let p = mgr.get_entity_mut(PLAYER).unwrap();
        set_stat(&mut p.stats, HEALTH, 1000);
        set_stat(&mut p.stats, COVER_DEFENSE, def);
        set_stat(&mut mgr.get_entity_mut(NPC).unwrap().stats, ACCURACY, 150);
        mgr
    };
    let mut open = build(0);
    let exposed = damage(&mut open, NPC, PLAYER).await;
    let mut mgr = build(100);
    let want = expected(&mgr, NPC, PLAYER, 20, -1.0);
    let covered = damage(&mut mgr, NPC, PLAYER).await;
    assert!(covered < exposed, "{covered} vs {exposed}");
    assert_eq!(covered, want);
}
