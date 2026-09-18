//! Dispatcher tests for the lifecycle events, concentrated on
//! `EntityHealthBelow` (Harset H04).
//!
//! Two layers, both needed:
//!
//! - **Dispatcher level** — drives [`fire_health_below_for_hit`] with
//!   exact health values, so the crossing semantics ("fires once on the
//!   crossing", "a killing blow never fires it") are pinned against real
//!   numbers rather than against whatever damage the QR roll happened to
//!   produce.
//! - **Damage-path level** — drives
//!   [`crate::cell::abilities::handle_use_ability_with_kill_credit`] end
//!   to end, so removing the hook from the combat caller fails a test.
//!   A dispatcher-level test cannot catch an unwired hook.

use cimmeria_content_engine::actions::Action;
use cimmeria_content_engine::chain::{Chain, ChainEngine};
use cimmeria_content_engine::triggers::Trigger;
use cimmeria_entity::stats::HEALTH;
use tokio::sync::mpsc;

use super::*;
use crate::cell::combat;

const PLAYER_EID: u32 = 1;
const PLAYER_ID: i32 = 100;
const NPC_EID: u32 = 50;
const DUEL_TAG: &str = "Rinla_Malac";
const WOUND_COUNTER: &str = "rinla_submitted";
const DEATH_COUNTER: &str = "rinla_killed";

/// Player at the origin, one tagged hostile NPC five units away at full
/// health out of 100 — so "health points" and "percent" are the same
/// number and every assertion below reads as a percentage.
fn make_duel_mgr() -> SpaceManager {
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
    mgr.spawn_npc(NPC_EID, "Castle", [5.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    if let Some(p) = mgr.get_entity_mut(PLAYER_EID) {
        p.is_player = true;
        p.player_id = Some(PLAYER_ID);
    }
    if let Some(npc) = mgr.get_entity_mut(NPC_EID) {
        npc.faction = combat::HOSTILE_FACTION;
        npc.tag = Some(DUEL_TAG.to_string());
        if let Some(stat) = npc.stats.get_mut(HEALTH) {
            stat.update(0, 100, 100);
            stat.clear_dirty();
        }
    }
    mgr.connect_entity(PLAYER_EID);
    let _ = mgr.compute_aoi_changes();
    mgr
}

/// An engine holding the pair of chains a duel beat would seed: submit
/// on the threshold crossing, and (per the advisory on mission 1325) a
/// death-path fallback on the same tag. Having both registered in every
/// test is deliberate — it makes the "exactly one of the two fires per
/// hit" contract observable in both directions.
fn duel_engine(pct: i32) -> ChainEngine {
    let mut engine = ChainEngine::new();
    engine.register_chain(Chain {
        action_delays: Vec::new(),
        id: 0x7004_0001,
        name: "test: Rin'la submit on health crossing".to_string(),
        enabled: true,
        trigger: Trigger::OnEntityHealthBelow {
            entity_tag: DUEL_TAG.to_string(),
            pct,
        },
        conditions: vec![],
        actions: vec![Action::IncrementCounter {
            counter_name: WOUND_COUNTER.to_string(),
            amount: 1,
        }],
        priority: 0,
    });
    engine.register_chain(Chain {
        action_delays: Vec::new(),
        id: 0x7004_0002,
        name: "test: Rin'la death fallback".to_string(),
        enabled: true,
        trigger: Trigger::OnEntityDeath {
            entity_type: None,
            entity_tag: Some(DUEL_TAG.to_string()),
        },
        conditions: vec![],
        actions: vec![Action::IncrementCounter {
            counter_name: DEATH_COUNTER.to_string(),
            amount: 1,
        }],
        priority: 0,
    });
    engine
}

fn counter(mgr: &SpaceManager, name: &str) -> i32 {
    mgr.get_entity(PLAYER_EID)
        .and_then(|p| p.counters.get(name).copied())
        .unwrap_or(0)
}

/// Set the NPC's current health and hand back the percentage it was at
/// *before* the change — i.e. what the damage path snapshots.
fn damage_npc_to(mgr: &mut SpaceManager, new_cur: i32) -> Option<combat::HealthPct> {
    let before = mgr.get_entity(NPC_EID).and_then(combat::health_pct);
    if let Some(stat) = mgr
        .get_entity_mut(NPC_EID)
        .and_then(|e| e.stats.get_mut(HEALTH))
    {
        stat.cur = new_cur;
    }
    before
}

// ─── Dispatcher level ───────────────────────────────────────────────

/// The headline acceptance case: a hit taking the duel NPC from 60% to
/// 40% fires the `:50` chain exactly once, and a second hit that lands
/// while it is already below fires nothing more.
#[tokio::test]
async fn crossing_fires_once_and_a_second_hit_below_fires_nothing() {
    let mut mgr = make_duel_mgr();
    let engine = duel_engine(50);
    let (tx, _rx) = mpsc::channel(32);

    damage_npc_to(&mut mgr, 60);

    // 60% → 40%: crosses 50.
    let before = damage_npc_to(&mut mgr, 40);
    fire_health_below_for_hit(PLAYER_EID, NPC_EID, before, &engine, &tx, &mut mgr).await;
    assert_eq!(
        counter(&mgr, WOUND_COUNTER),
        1,
        "a hit from 60% to 40% must fire the 50% chain exactly once",
    );

    // 40% → 25%: already below, must not re-fire. This is the half of
    // the predicate (`pct_before > pct`) that keeps a duel from
    // re-advancing the mission step on every follow-up shot.
    let before = damage_npc_to(&mut mgr, 25);
    fire_health_below_for_hit(PLAYER_EID, NPC_EID, before, &engine, &tx, &mut mgr).await;
    assert_eq!(
        counter(&mgr, WOUND_COUNTER),
        1,
        "a second hit landing below the threshold must not re-fire the chain",
    );

    assert_eq!(
        counter(&mgr, DEATH_COUNTER),
        0,
        "nothing died — the death chain must stay untouched",
    );
}

/// Healed back above the threshold, then crossed again → fires again.
/// The crossing is a property of the hit, not a latch.
#[tokio::test]
async fn a_second_genuine_crossing_after_a_heal_fires_again() {
    let mut mgr = make_duel_mgr();
    let engine = duel_engine(50);
    let (tx, _rx) = mpsc::channel(32);

    damage_npc_to(&mut mgr, 60);
    let before = damage_npc_to(&mut mgr, 40);
    fire_health_below_for_hit(PLAYER_EID, NPC_EID, before, &engine, &tx, &mut mgr).await;

    // Healed back to 80% — an upward move must itself fire nothing.
    let before = damage_npc_to(&mut mgr, 80);
    fire_health_below_for_hit(PLAYER_EID, NPC_EID, before, &engine, &tx, &mut mgr).await;
    assert_eq!(
        counter(&mgr, WOUND_COUNTER),
        1,
        "an upward health move is not a downward crossing",
    );

    // …and crossed a second time.
    let before = damage_npc_to(&mut mgr, 45);
    fire_health_below_for_hit(PLAYER_EID, NPC_EID, before, &engine, &tx, &mut mgr).await;
    assert_eq!(
        counter(&mgr, WOUND_COUNTER),
        2,
        "a genuine second crossing must fire again",
    );
}

/// Two thresholds on the same tag: one big hit spanning both fires both.
/// Pinned so an author staging a fight knows a single crit can collapse
/// two beats into one frame.
#[tokio::test]
async fn one_hit_spanning_two_thresholds_fires_both_chains() {
    let mut mgr = make_duel_mgr();
    let mut engine = duel_engine(50);
    engine.register_chain(Chain {
        action_delays: Vec::new(),
        id: 0x7004_0003,
        name: "test: Rin'la taunt at 30%".to_string(),
        enabled: true,
        trigger: Trigger::OnEntityHealthBelow {
            entity_tag: DUEL_TAG.to_string(),
            pct: 30,
        },
        conditions: vec![],
        actions: vec![Action::IncrementCounter {
            counter_name: "rinla_taunt".to_string(),
            amount: 1,
        }],
        priority: 0,
    });
    let (tx, _rx) = mpsc::channel(32);

    damage_npc_to(&mut mgr, 80);
    let before = damage_npc_to(&mut mgr, 20);
    fire_health_below_for_hit(PLAYER_EID, NPC_EID, before, &engine, &tx, &mut mgr).await;

    assert_eq!(counter(&mgr, WOUND_COUNTER), 1, "80% → 20% crosses 50");
    assert_eq!(counter(&mgr, "rinla_taunt"), 1, "80% → 20% also crosses 30");
}

/// A hit that takes the target to zero must not fire a threshold chain
/// even though the band predicate (`31 > 30 && 0 <= 30`) would match.
/// The suppression is the dispatcher's job.
#[tokio::test]
async fn a_hit_that_reaches_zero_health_fires_no_threshold_chain() {
    let mut mgr = make_duel_mgr();
    let engine = duel_engine(30);
    let (tx, _rx) = mpsc::channel(32);

    damage_npc_to(&mut mgr, 31);
    let before = damage_npc_to(&mut mgr, 0);
    fire_health_below_for_hit(PLAYER_EID, NPC_EID, before, &engine, &tx, &mut mgr).await;

    assert_eq!(
        counter(&mgr, WOUND_COUNTER),
        0,
        "a 31% → 0% hit satisfies the band predicate; only the \
         dispatcher's lethal-hit suppression stops it firing",
    );
}

/// Corpse guard, read from `BSF_DEAD` rather than from health. An effect
/// script can heal a target *after* the death transition has stamped the
/// dead bit, leaving a corpse at positive health — a health-only check
/// would fire a threshold chain on it.
#[tokio::test]
async fn a_dead_target_at_positive_health_fires_no_threshold_chain() {
    let mut mgr = make_duel_mgr();
    let engine = duel_engine(50);
    let (tx, _rx) = mpsc::channel(32);

    damage_npc_to(&mut mgr, 60);
    let before = damage_npc_to(&mut mgr, 40);
    // Post-death heal script: dead bit set, health back above zero.
    if let Some(npc) = mgr.get_entity_mut(NPC_EID) {
        npc.set_state_flag(combat::BSF_DEAD);
    }
    fire_health_below_for_hit(PLAYER_EID, NPC_EID, before, &engine, &tx, &mut mgr).await;

    assert_eq!(
        counter(&mgr, WOUND_COUNTER),
        0,
        "a target carrying BSF_DEAD belongs to entity_dead_tag no matter \
         what its health reads",
    );
}

/// An untagged NPC can't be addressed by a chain, so the dispatcher
/// drops the event before building any context.
#[tokio::test]
async fn an_untagged_target_fires_nothing() {
    let mut mgr = make_duel_mgr();
    if let Some(npc) = mgr.get_entity_mut(NPC_EID) {
        npc.tag = None;
    }
    let engine = duel_engine(50);
    let (tx, _rx) = mpsc::channel(32);

    damage_npc_to(&mut mgr, 60);
    let before = damage_npc_to(&mut mgr, 40);
    fire_health_below_for_hit(PLAYER_EID, NPC_EID, before, &engine, &tx, &mut mgr).await;

    assert_eq!(counter(&mgr, WOUND_COUNTER), 0);
}

/// An attacker with no `player_id` (NPC-versus-NPC damage, or a caller
/// that wired this up from a non-player path) must not resolve a chain —
/// there is no mission to advance.
#[tokio::test]
async fn an_attacker_without_a_player_id_fires_nothing() {
    let mut mgr = make_duel_mgr();
    if let Some(p) = mgr.get_entity_mut(PLAYER_EID) {
        p.player_id = None;
    }
    let engine = duel_engine(50);
    let (tx, _rx) = mpsc::channel(32);

    damage_npc_to(&mut mgr, 60);
    let before = damage_npc_to(&mut mgr, 40);
    fire_health_below_for_hit(PLAYER_EID, NPC_EID, before, &engine, &tx, &mut mgr).await;

    assert_eq!(counter(&mgr, WOUND_COUNTER), 0);
}

/// With no chain registered for the trigger the dispatcher must be a
/// complete no-op — this runs on every damaging hit in the game.
#[tokio::test]
async fn an_engine_with_no_health_chains_emits_nothing() {
    let mut mgr = make_duel_mgr();
    let engine = ChainEngine::new();
    let (tx, mut rx) = mpsc::channel(32);

    damage_npc_to(&mut mgr, 60);
    let before = damage_npc_to(&mut mgr, 40);
    fire_health_below_for_hit(PLAYER_EID, NPC_EID, before, &engine, &tx, &mut mgr).await;

    assert!(
        rx.try_recv().is_err(),
        "unseeded server must produce no wire traffic from the hook",
    );
}

// ─── Damage path (end to end through the kill-credit wrapper) ───────

/// Install a single-effect ability on the player. `health_damage` picks
/// whether the shot wounds or kills.
fn arm_player_with_ability(mgr: &mut SpaceManager, health_damage: i32) {
    use cimmeria_entity::abilities::{AbilityDef, EffectDef};

    let mut params = std::collections::HashMap::new();
    params.insert("HealthDamage".to_string(), health_damage.to_string());
    mgr.effect_defs.insert(
        100,
        EffectDef {
            effect_id: 100,
            ability_id: 7,
            delay: 0,
            effect_sequence: 0,
            event_set_id: None,
            script_name: None,
            params,
            ..Default::default()
        },
    );
    mgr.ability_defs.insert(
        7,
        AbilityDef {
            ability_id: 7,
            name: "test".to_string(),
            cooldown: 0.5,
            warmup: 0.0,
            flags: 0,
            is_ranged: false,
            min_range: 0,
            max_range: 30,
            target_type_id: 0,
            effect_ids: vec![100],
            moniker_ids: vec![],
            required_ammo: 0,
            event_set_id: None,
            velocity: 0.0,
        },
    );
    if let Some(p) = mgr.get_entity_mut(PLAYER_EID) {
        p.abilities.add_ability(7);
        p.weapon_holstered = false;
    }
}

/// **Regression guard for the damage-path hook.** A wounding hit that
/// crosses the threshold must fire the health-below chain and not the
/// death chain. Deleting the `fire_health_below_for_hit` call from
/// `abilities/use_ability/kill_credit.rs` leaves the counter at zero.
///
/// The threshold is 99% so the assertion holds for any non-zero damage
/// the QR roll produces — this test is about the wiring, not the damage
/// numbers (those are pinned at the dispatcher level above).
#[tokio::test]
async fn a_wounding_hit_through_the_damage_path_fires_health_below() {
    let mut mgr = make_duel_mgr();
    arm_player_with_ability(&mut mgr, 5);
    let engine = duel_engine(99);
    let (tx, _rx) = mpsc::channel(128);

    crate::cell::abilities::handle_use_ability_with_kill_credit(
        PLAYER_EID,
        7,
        NPC_EID as i32,
        &engine,
        &tx,
        &mut mgr,
    )
    .await;

    // Fixture sanity first: if the QR roll missed, the trigger
    // assertion below would fail for a reason that has nothing to do
    // with the hook.
    let hp = mgr
        .get_entity(NPC_EID)
        .and_then(|e| e.stats.get(HEALTH))
        .map(|s| s.cur)
        .expect("NPC must still exist with a HEALTH stat");
    assert!(
        hp < 100 && hp > 0,
        "test fixture: the shot must wound without killing (health = {hp}); \
         a QR miss would leave this at 100",
    );

    assert_eq!(
        counter(&mgr, WOUND_COUNTER),
        1,
        "a wounding hit must reach fire_health_below_for_hit from the \
         kill-credit wrapper — zero here means the damage-path hook is gone",
    );
    assert_eq!(
        counter(&mgr, DEATH_COUNTER),
        0,
        "nothing died, so entity_dead_tag must not fire",
    );
}

/// The exclusivity contract, end to end: a killing blow fires
/// `entity_dead_tag` and **not** the threshold chain, even though the
/// kill crosses the threshold on its way down.
#[tokio::test]
async fn a_killing_hit_through_the_damage_path_fires_death_not_health_below() {
    let mut mgr = make_duel_mgr();
    arm_player_with_ability(&mut mgr, 9999);
    let engine = duel_engine(99);
    let (tx, _rx) = mpsc::channel(128);

    crate::cell::abilities::handle_use_ability_with_kill_credit(
        PLAYER_EID,
        7,
        NPC_EID as i32,
        &engine,
        &tx,
        &mut mgr,
    )
    .await;

    let dead = mgr
        .get_entity(NPC_EID)
        .map(|e| combat::is_dead_state(e.state_field))
        .expect("NPC entity must survive as a corpse");
    assert!(dead, "test fixture: 9999 damage must kill the NPC");

    assert_eq!(
        counter(&mgr, DEATH_COUNTER),
        1,
        "a killing blow must fire entity_dead_tag",
    );
    assert_eq!(
        counter(&mgr, WOUND_COUNTER),
        0,
        "a killing blow must NOT also fire the health-threshold chain — \
         the two are mutually exclusive per hit",
    );
}
