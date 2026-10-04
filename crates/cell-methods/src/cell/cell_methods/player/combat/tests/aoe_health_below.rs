//! An AoE secondary target crossing its health threshold fires
//! `entity_health_below` (Harset H04, PR #662 review finding 1).
//!
//! Cut from `cimmeria-cell-content`'s `event_dispatch::lifecycle::tests` in
//! wave C3 of the services crate split
//! (docs/architecture/services-crate-split.md): it drives the real
//! `useAbilityOnGround` cell-method dispatch, so it waited in
//! `cimmeria-services` until wave C5a moved the dispatcher here. The duel
//! fixtures are copies of that file's.

use cimmeria_content_engine::actions::Action;
use cimmeria_content_engine::chain::{Chain, ChainEngine};
use cimmeria_content_engine::triggers::Trigger;
use cimmeria_entity::stats::HEALTH;
use tokio::sync::mpsc;

use crate::cell::combat;
use crate::cell::space_manager::SpaceManager;

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
        once: false,
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
        once: false,
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
            min_range: 0.0,
            max_range: 30.0,
            target_type_id: 0,
            effect_ids: vec![100],
            moniker_ids: vec![],
            required_ammo: 0,
            event_set_id: None,
            velocity: 0.0,
            type_id: Default::default(),
            passive: false,
        },
    );
    if let Some(p) = mgr.get_entity_mut(PLAYER_EID) {
        p.abilities.add_ability(7);
        p.weapon_holstered = false;
    }
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
    // 20 base damage survives QR and defense with enough left to cross 30%
    // from 35% without killing.
    arm_player_with_ability(&mut mgr, 20);
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
