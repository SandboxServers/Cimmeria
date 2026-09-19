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
///
/// The counter assertion alone is indistinguishable from the untagged
/// case, the no-chain case, or an outright dropped event, so this also
/// pins the negative log that makes the skip visible in production. Per
/// [the negative-logging convention](../../../../../../../docs/architecture/negative-logging-convention.md)
/// this seam warns rather than failing silently, because the function is
/// only reachable from player-driven paths — reaching it without a
/// `player_id` means a caller wired it up wrong.
#[tokio::test]
async fn an_attacker_without_a_player_id_fires_nothing_and_warns() {
    use crate::test_support::LogCapture;
    use tracing::Level;

    let capture = LogCapture::install();

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
    assert!(
        capture
            .find_message(Level::WARN, "attacker has no player_id")
            .is_some(),
        "the skip must be visible in the log, not silent — deleting the \
         warn! leaves an unexplained missing mission advance",
    );
}

// There is deliberately no "empty engine emits nothing" test. The
// `chains_for_trigger(...) == 0` fast path at the top of
// `fire_health_below_for_hit` is a per-hit cost optimisation with no
// observable behaviour: delete it and an empty `ChainEngine` still
// resolves nothing, so any black-box assertion passes either way. A test
// named for that guard would claim coverage it cannot have.

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
///
/// Note what this does and does not guard. It pins the *contract*, and
/// the enforcing layer is the **dispatcher** — `fire_health_below_for_hit`
/// returns early on both `is_dead_state` and `pct_after <= 0`. Hoisting
/// the call in `kill_credit.rs` out of its `if !just_died` branch would
/// leave this test green, because the corpse trips those guards anyway.
/// The branch placement is defence-in-depth, not the enforcement; the
/// two dispatcher-level tests above are what guard the enforcement.
#[tokio::test]
async fn a_killing_hit_fires_death_and_the_dispatcher_suppresses_health_below() {
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

// ─── The non-single-target damage paths (PR #662 review, finding 1) ──
//
// H04 sampled `pct_before` inside the single-target kill-credit wrapper,
// so every other damage path was invisible to the trigger — and because
// the predicate is a stateless downward band, a crossing made on one of
// those paths is lost forever rather than merely late (every later hit
// arrives with `pct_before <= threshold`). The sample now lives at the
// health-application seams; these are the guards for the paths that had
// none.

/// Register a pulsing DoT on the NPC, invoked by the player, already due
/// to fire. `dmg` is the per-pulse HealthDamage.
async fn arm_dot_on_npc(mgr: &mut SpaceManager, tx: &mpsc::Sender<CellToBaseMsg>, dmg: i32) {
    use cimmeria_entity::abilities::EffectDef;
    use std::time::{Duration, Instant};

    let mut params = std::collections::HashMap::new();
    params.insert("HealthDamage".to_string(), dmg.to_string());
    let effect = EffectDef {
        effect_id: 7777,
        ability_id: 1234,
        pulse_count: 5,
        pulse_duration: 1.0,
        params,
        ..Default::default()
    };
    mgr.effect_defs.insert(effect.effect_id, effect.clone());

    let past = Instant::now() - Duration::from_secs(2);
    crate::cell::effects::register_active_effect(mgr, NPC_EID, PLAYER_EID, &effect, past, tx).await;
    if let Some(inst) = mgr
        .get_entity_mut(NPC_EID)
        .and_then(|t| t.active_effects.first_mut())
    {
        inst.next_pulse_at = past;
    }
}

/// **The headline gap.** A DoT tick that drags the NPC from 35% to 25%
/// crosses `:30` and must fire exactly once. Before the fix `fire_pulse`
/// sampled nothing, so this counter stayed at zero — and, worse, every
/// subsequent direct hit arrived with `pct_before <= 30`, permanently
/// disarming the chain.
#[tokio::test]
async fn a_dot_pulse_crossing_fires_health_below_once() {
    let mut mgr = make_duel_mgr();
    let engine = duel_engine(30);
    let (tx, _rx) = mpsc::channel(128);

    damage_npc_to(&mut mgr, 35);
    arm_dot_on_npc(&mut mgr, &tx, 10).await;

    crate::cell::effects::effect_pulse_tick(&engine, &tx, &mut mgr).await;

    let hp = mgr
        .get_entity(NPC_EID)
        .and_then(|e| e.stats.get(HEALTH))
        .map(|s| s.cur)
        .expect("NPC must still exist with a HEALTH stat");
    assert_eq!(hp, 25, "test fixture: the pulse must land 35 -> 25");
    assert_eq!(
        counter(&mgr, WOUND_COUNTER),
        1,
        "a DoT tick crossing the threshold must fire entity_health_below \
         exactly once — zero means the pulse path never samples pct_before",
    );
    assert_eq!(counter(&mgr, DEATH_COUNTER), 0, "nothing died");
}

/// A second pulse landing below the threshold must not re-fire, the same
/// way a second shot below it does not. Pins that the per-pulse drain
/// hands over one sample per pulse rather than re-using a stale one.
#[tokio::test]
async fn a_second_dot_pulse_below_the_threshold_fires_nothing_more() {
    let mut mgr = make_duel_mgr();
    let engine = duel_engine(30);
    let (tx, _rx) = mpsc::channel(128);

    damage_npc_to(&mut mgr, 35);
    arm_dot_on_npc(&mut mgr, &tx, 10).await;
    crate::cell::effects::effect_pulse_tick(&engine, &tx, &mut mgr).await;

    // Make the next pulse due and tick again: 25% -> 15%, already below.
    if let Some(inst) = mgr
        .get_entity_mut(NPC_EID)
        .and_then(|t| t.active_effects.first_mut())
    {
        inst.next_pulse_at = std::time::Instant::now() - std::time::Duration::from_secs(2);
    }
    crate::cell::effects::effect_pulse_tick(&engine, &tx, &mut mgr).await;

    assert_eq!(
        counter(&mgr, WOUND_COUNTER),
        1,
        "the second pulse starts below the threshold, so there is no \
         downward crossing left to fire",
    );
}

/// The exclusivity contract on the pulse path: a lethal tick fires
/// `entity_dead_tag` and **not** the threshold chain, even though the
/// kill crosses the threshold on its way to zero.
///
/// This also guards the death transition itself. Before the fix a DoT
/// kill produced no corpse at all — the mob sat at zero health, never
/// flipped `BSF_DEAD`, and a kill-count mission stalled whenever the
/// killing blow happened to be a tick rather than a shot.
#[tokio::test]
async fn a_killing_dot_pulse_fires_death_and_not_the_threshold() {
    let mut mgr = make_duel_mgr();
    let engine = duel_engine(30);
    let (tx, _rx) = mpsc::channel(128);

    damage_npc_to(&mut mgr, 35);
    arm_dot_on_npc(&mut mgr, &tx, 40).await;

    crate::cell::effects::effect_pulse_tick(&engine, &tx, &mut mgr).await;

    let dead = mgr
        .get_entity(NPC_EID)
        .map(|e| combat::is_dead_state(e.state_field))
        .expect("NPC entity must survive as a corpse");
    assert!(
        dead,
        "a lethal pulse must run the canonical death transition"
    );
    assert_eq!(
        counter(&mgr, DEATH_COUNTER),
        1,
        "a DoT kill must credit entity_dead_tag to the effect's invoker",
    );
    assert_eq!(
        counter(&mgr, WOUND_COUNTER),
        0,
        "a lethal pulse must not also fire the threshold chain",
    );
}

/// The AoE/cone gap: a secondary target dragged through its threshold by
/// a ground cast must fire, not just the primary. The sample lives in
/// `apply_damage_to_target`, which every secondary goes through; before
/// the fix only the single-target wrapper sampled at all.
#[tokio::test]
async fn an_aoe_secondary_crossing_fires_health_below() {
    let mut mgr = make_duel_mgr();
    let engine = duel_engine(30);
    let (tx, _rx) = mpsc::channel(256);

    // The tagged duel NPC is the *secondary*: a second, untagged hostile
    // sits closer to the impact point and takes the primary slot.
    mgr.spawn_npc(NPC_EID + 1, "Castle", [4.5, 0.0, 0.0], [0.0; 3])
        .unwrap();
    if let Some(other) = mgr.get_entity_mut(NPC_EID + 1) {
        other.faction = combat::HOSTILE_FACTION;
        if let Some(stat) = other.stats.get_mut(HEALTH) {
            stat.update(0, 10_000, 10_000);
            stat.clear_dirty();
        }
    }
    arm_player_with_ability(&mut mgr, 10);
    damage_npc_to(&mut mgr, 35);

    // Drive the real cell-method dispatch rather than
    // `handle_use_ability_on_ground` directly: the drain lives in the
    // handler, so calling the ability helper would guard nothing.
    let mut args = Vec::with_capacity(16);
    args.extend_from_slice(&7i32.to_le_bytes()); // ability_id
    args.extend_from_slice(&5.0f32.to_le_bytes()); // x
    args.extend_from_slice(&0.0f32.to_le_bytes()); // y
    args.extend_from_slice(&0.0f32.to_le_bytes()); // z
    crate::cell::cell_methods::player::combat::dispatch(
        PLAYER_EID,
        crate::cell::cell_methods::player::USE_ABILITY_ON_GROUND,
        &args,
        &tx,
        &mut mgr,
        &engine,
    )
    .await;

    let hp = mgr
        .get_entity(NPC_EID)
        .and_then(|e| e.stats.get(HEALTH))
        .map(|s| s.cur)
        .expect("the tagged NPC must survive the blast");
    assert!(
        hp < 35 && hp > 0,
        "test fixture: the secondary must be wounded without dying \
         (health = {hp})",
    );
    assert_eq!(
        counter(&mgr, WOUND_COUNTER),
        1,
        "an AoE secondary crossing the threshold must fire \
         entity_health_below — zero means the ground path never drains",
    );
}
