//! NA32 (D-NA15a): cover as a per-node damage reduction, driven through
//! `apply_damage_to_target`.
//!
//! The NPC stands at a cover node at the origin facing +X. The player shoots
//! from +X (in front: covered) or from -X (behind: flanked). Every case uses
//! the same ability, effect sequence and so the same beta-roll seed; cover
//! no longer moves the QR, so the only thing that changes the damage is the
//! reduction.

use super::tests::{make_ability, make_mgr_player_vs_npc};
use super::*;
use crate::cell::cover::{Cover, CoverHeight, CoverNode, CoverQuality, CoverSlotKey};
use crate::test_support::LogCapture;
use cimmeria_common::{EntityId, Vector3};
use cimmeria_entity::abilities::EffectDef;
use cimmeria_entity::stats::{StatList, COVER_ACCURACY, COVER_DEFENSE, COVER_QR_MODIFIER};

const PLAYER: u32 = 1;
const NPC: u32 = 2;
const ABILITY: i32 = 7;
const EFFECT: i32 = 100;
const SEQ: u32 = 1;
/// Large enough that rounding does not blur the percentages.
const HEALTH_DAMAGE: i32 = 200;
/// World id the fixture's cover node carries (Castle_CellBlock).
const WORLD: i32 = 12;
const SLOT: CoverSlotKey = CoverSlotKey {
    chunk_id: 50,
    node_id: 0,
};
/// The seeds' typical guard slot (13 of the 16 spawn-held slots).
const GUARD_SLOT: (CoverQuality, CoverHeight) = (CoverQuality::Better, CoverHeight::Mid);

fn set_stat(stats: &mut StatList, id: i32, v: i32) {
    let s = stats.get_mut(id).unwrap();
    s.update(-1000, v, v.max(100_000));
    s.clear_dirty();
}

/// Player at `player_pos`, NPC at the origin with plenty of HP, one cover
/// node of `node` rating at the origin facing +X.
fn fixture(
    player_pos: [f32; 3],
    node: (CoverQuality, CoverHeight),
    npc_holds_slot: bool,
    npc_cover_defense: i32,
) -> SpaceManager {
    let mut mgr = make_mgr_player_vs_npc();
    mgr.stamp_world_rows(&std::collections::HashMap::from([(
        "Castle".to_string(),
        crate::cell::spawner::WorldRow::enforcing(WORLD),
    )]));
    mgr.get_entity_mut(PLAYER).unwrap().position =
        Vector3::new(player_pos[0], player_pos[1], player_pos[2]);
    let npc = mgr.get_entity_mut(NPC).unwrap();
    set_stat(&mut npc.stats, HEALTH, 100_000);
    set_stat(&mut npc.stats, COVER_DEFENSE, npc_cover_defense);
    mgr.cover = Cover::from_loaded(
        Vec::new(),
        vec![CoverNode {
            chunk_id: SLOT.chunk_id,
            node_id: SLOT.node_id,
            world_id: WORLD,
            pos: Vector3::new(0.0, 0.0, 0.0),
            orient: 0.0,
            height: node.1,
            quality: node.0,
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
    params.insert("HealthDamage".to_string(), HEALTH_DAMAGE.to_string());
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

/// The damage the pipeline gives for this fixture's hit with `final_pct`
/// taken off, computed straight from the combat functions.
fn expected(mgr: &SpaceManager, attacker: u32, target: u32, base: i32, final_pct: f64) -> i32 {
    let att = mgr.get_entity(attacker).unwrap().stats.clone();
    let mut def = mgr.get_entity(target).unwrap().stats.clone();
    let qr = combat::calculate_qr(&att, &def, false);
    let seed = super::super::rng::pseudo_random_seed(attacker, ABILITY, SEQ);
    let roll = combat::calculate_result(qr, seed);
    let scale = 1.0 - final_pct / 100.0;
    combat::calculate_damage_scaled(&roll, base, scale, DT_PHYSICAL, HEALTH, &att, &mut def).1
}

fn cover_row(logs: &crate::test_support::LogCaptureGuard) -> crate::test_support::Captured {
    logs.all()
        .into_iter()
        .find(|c| c.target == "abilities.qr" && c.has_field("event", "cover_resolved"))
        .expect("a hit on an NPC at a cover node writes the cover row")
}

/// The typical guard slot with Cover Stance takes 35% off: 25% for a
/// `Mid`/`Better` node plus 10 for +100 `coverDefense`. Revert the wiring
/// and the covered hit does the exposed damage; change the table or the
/// stance scale and it misses `expected`.
#[tokio::test]
async fn a_guard_in_its_slot_takes_the_rated_reduction() {
    let mut open = fixture([20.0, 0.0, 0.0], GUARD_SLOT, false, 0);
    let exposed = damage(&mut open, PLAYER, NPC).await;

    let mut mgr = fixture([20.0, 0.0, 0.0], GUARD_SLOT, true, 100);
    // Player damage is doubled upstream (the temporary 2x in `mod.rs`).
    let want = expected(&mgr, PLAYER, NPC, HEALTH_DAMAGE * 2, 35.0);
    let logs = LogCapture::install();
    let covered = damage(&mut mgr, PLAYER, NPC).await;

    assert!(exposed > 0, "{exposed}");
    assert_eq!(covered, want, "25% node + 10% stance");
    let ratio = covered as f64 / exposed as f64;
    assert!((ratio - 0.65).abs() < 0.01, "{covered} / {exposed}");
    let row = cover_row(&logs);
    for (k, v) in [
        ("defender_cover", "in_cover"),
        ("flanked", "false"),
        ("cover_quality", "QUALITY_Better"),
        ("cover_height", "HEIGHT_Mid"),
        ("base_pct", "25.0"),
        ("stance_pct", "10.0"),
        ("penetration_pct", "0.0"),
        ("final_pct", "35.0"),
    ] {
        assert!(row.has_field(k, v), "{k}={v}: {row:?}");
    }
}

/// A wall (tall, best quality) with the stance hits the 60% cap; a lunch
/// table (low, good quality) without it gives 15%.
#[tokio::test]
async fn a_wall_beats_a_lunch_table() {
    let mut open = fixture([20.0, 0.0, 0.0], GUARD_SLOT, false, 0);
    let exposed = damage(&mut open, PLAYER, NPC).await as f64;
    for (node, stance, pct) in [
        ((CoverQuality::Best, CoverHeight::High), 100, 60.0),
        ((CoverQuality::Good, CoverHeight::Low), 0, 15.0),
    ] {
        let mut mgr = fixture([20.0, 0.0, 0.0], node, true, stance);
        let got = damage(&mut mgr, PLAYER, NPC).await as f64;
        assert!(
            (got / exposed - (1.0 - pct / 100.0)).abs() < 0.01,
            "{node:?}: {got} vs {exposed}, expected {pct}% off"
        );
    }
}

/// Shot from behind, the same NPC with the same stance takes the exposed
/// hit, and the row says why.
#[tokio::test]
async fn flanked_npc_gets_no_cover() {
    let mut open = fixture([-20.0, 0.0, 0.0], GUARD_SLOT, false, 0);
    let exposed = damage(&mut open, PLAYER, NPC).await;
    let mut mgr = fixture([-20.0, 0.0, 0.0], GUARD_SLOT, true, 100);
    let logs = LogCapture::install();
    let flanked = damage(&mut mgr, PLAYER, NPC).await;
    assert_eq!(flanked, exposed, "flanked cover is no cover");
    let row = cover_row(&logs);
    assert!(row.has_field("flanked", "true"), "{row:?}");
    assert!(row.has_field("final_pct", "0.0"), "{row:?}");
}

/// The raised stat without the slot (an NPC walking to it, or a buff out of
/// cover) changes nothing.
#[tokio::test]
async fn cover_defense_away_from_cover_changes_nothing() {
    let mut open = fixture([20.0, 0.0, 0.0], GUARD_SLOT, false, 0);
    let exposed = damage(&mut open, PLAYER, NPC).await;
    let mut mgr = fixture([20.0, 0.0, 0.0], GUARD_SLOT, false, 100);
    assert_eq!(damage(&mut mgr, PLAYER, NPC).await, exposed);
}

/// The owner's rule: never a bullet sponge. Every node rating, with the
/// heaviest defensive stack, lets at least 40% of the hit through; and the
/// most penetration still leaves the 10% floor.
#[tokio::test]
async fn a_covered_guard_always_takes_at_least_forty_percent() {
    let mut open = fixture([20.0, 0.0, 0.0], GUARD_SLOT, false, 0);
    let exposed = damage(&mut open, PLAYER, NPC).await as f64;
    for q in [
        CoverQuality::None_,
        CoverQuality::Good,
        CoverQuality::Better,
        CoverQuality::Best,
    ] {
        for h in [
            CoverHeight::Low,
            CoverHeight::Mid,
            CoverHeight::High,
            CoverHeight::Los,
        ] {
            let mut mgr = fixture([20.0, 0.0, 0.0], (q, h), true, 900);
            set_stat(
                &mut mgr.get_entity_mut(NPC).unwrap().stats,
                COVER_QR_MODIFIER,
                5,
            );
            let got = damage(&mut mgr, PLAYER, NPC).await as f64;
            assert!(got >= 0.4 * exposed - 1.0, "{q:?} {h:?}: {got} / {exposed}");

            let mut pen = fixture([20.0, 0.0, 0.0], (q, h), true, 0);
            set_stat(
                &mut pen.get_entity_mut(PLAYER).unwrap().stats,
                COVER_ACCURACY,
                1000,
            );
            let got = damage(&mut pen, PLAYER, NPC).await as f64;
            assert!(
                (got / exposed - 0.9).abs() < 0.01,
                "{q:?} {h:?}: penetration floors at 10%: {got} / {exposed}"
            );
        }
    }
}

/// A player standing at a node with a cover-defense buff is covered against
/// an NPC in front of the node, with no slot to hold.
#[tokio::test]
async fn covered_player_takes_less_from_an_npc() {
    let build = |def: i32| {
        let mut mgr = fixture([0.5, 0.0, 0.0], GUARD_SLOT, false, 0);
        mgr.get_entity_mut(NPC).unwrap().position = Vector3::new(20.0, 0.0, 0.0);
        let p = mgr.get_entity_mut(PLAYER).unwrap();
        set_stat(&mut p.stats, HEALTH, 100_000);
        set_stat(&mut p.stats, COVER_DEFENSE, def);
        mgr
    };
    let mut open = build(0);
    open.cover = Cover::empty();
    let exposed = damage(&mut open, NPC, PLAYER).await;
    let mut mgr = build(100);
    let want = expected(&mgr, NPC, PLAYER, HEALTH_DAMAGE, 35.0);
    let covered = damage(&mut mgr, NPC, PLAYER).await;
    assert!(covered < exposed, "{covered} vs {exposed}");
    assert_eq!(covered, want, "the nearest node's 25% plus the buff's 10%");
}
