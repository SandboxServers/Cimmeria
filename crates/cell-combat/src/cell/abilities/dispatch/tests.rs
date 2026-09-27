//! Tests for the ground-target dispatch (`useAbilityOnGroundTarget`).

use super::*;

/// Synthesize an `AbilityDef` + matching `EffectDef` with a `Radius`
/// NVP and assert `ability_radius` reads it.
#[test]
fn ability_radius_reads_effect_nvp() {
    use cimmeria_entity::abilities::{AbilityDef, EffectDef};
    use std::collections::HashMap;

    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" Instanced="false" MinX="0" MaxX="100" MinY="0" MaxY="100" /></Spaces>"#;
    let cxml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(cxml).unwrap();

    let mut effect_params = HashMap::new();
    effect_params.insert("Radius".to_string(), "12.5".to_string());
    mgr.effect_defs.insert(
        500,
        EffectDef {
            effect_id: 500,
            ability_id: 999,
            delay: 0,
            effect_sequence: 0,
            event_set_id: None,
            script_name: None,
            params: effect_params,
            ..Default::default()
        },
    );

    let ability = AbilityDef {
        ability_id: 999,
        name: "GroundTargetAoE".to_string(),
        cooldown: 1.0,
        warmup: 0.0,
        flags: 0,
        is_ranged: true,
        min_range: 0,
        max_range: 30,
        target_type_id: 0,
        effect_ids: vec![500],
        moniker_ids: vec![],
        required_ammo: 0,
        event_set_id: None,
        velocity: 0.0,
    };

    assert_eq!(ability_radius(&Some(ability), &mgr), 12.5);
}

#[test]
fn ability_radius_falls_back_to_default_when_unset() {
    let mgr = SpaceManager::new(1);
    // No ability def → default
    assert_eq!(ability_radius(&None, &mgr), DEFAULT_GROUND_TARGET_RADIUS);
}

#[test]
fn ability_radius_falls_back_when_effects_have_no_radius() {
    use cimmeria_entity::abilities::{AbilityDef, EffectDef};
    use std::collections::HashMap;

    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" Instanced="false" MinX="0" MaxX="100" MinY="0" MaxY="100" /></Spaces>"#;
    let cxml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(cxml).unwrap();

    // Effect with HealthDamage but no Radius → fall back to default.
    let mut effect_params = HashMap::new();
    effect_params.insert("HealthDamage".to_string(), "20".to_string());
    mgr.effect_defs.insert(
        501,
        EffectDef {
            effect_id: 501,
            ability_id: 998,
            delay: 0,
            effect_sequence: 0,
            event_set_id: None,
            script_name: None,
            params: effect_params,
            ..Default::default()
        },
    );

    let ability = AbilityDef {
        ability_id: 998,
        name: "NoRadius".to_string(),
        cooldown: 1.0,
        warmup: 0.0,
        flags: 0,
        is_ranged: true,
        min_range: 0,
        max_range: 30,
        target_type_id: 0,
        effect_ids: vec![501],
        moniker_ids: vec![],
        required_ammo: 0,
        event_set_id: None,
        velocity: 0.0,
    };

    assert_eq!(
        ability_radius(&Some(ability), &mgr),
        DEFAULT_GROUND_TARGET_RADIUS
    );
}

// ── Ground-target AoE end-to-end tests ───────────────────────────────
//
// These cover the critical correctness behaviors flagged on PR #122:
//   1. damage applies to every hostile NPC in radius (not just nearest)
//   2. secondary damage is suppressed when the primary cast is rejected
//   3. NPCs outside the attacker's space are excluded
//   4. the returned Vec<u32> identifies every alive→dead transition for
//      the caller's `fire_entity_death` loop

use cimmeria_entity::abilities::{AbilityDef, EffectDef};
use cimmeria_entity::cell_entity::BandolierItem;
use cimmeria_entity::stats::{AMMO_SLOT_1, HEALTH};
use std::collections::HashMap;
use tokio::sync::mpsc;

const ATTACKER_EID: u32 = 1;
const GROUND_ABILITY_ID: i32 = 8888;
const GROUND_EFFECT_ID: i32 = 9999;
const HOSTILE: u8 = 10;

/// Standard AoE scenario: 1 player attacker at origin in
/// `Castle_CellBlock`, an ability with required_ammo=1, max_range=30,
/// and a single effect carrying HealthDamage=30 + Radius=10.0.
/// Returns `(SpaceManager, tx, rx)` for end-to-end fire tests.
fn make_aoe_scenario() -> (
    SpaceManager,
    mpsc::Sender<CellToBaseMsg>,
    mpsc::Receiver<CellToBaseMsg>,
) {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" Instanced="false" MinX="-100" MaxX="100" MinY="-100" MaxY="100" /><Space WorldName="OtherWorld" Instanced="false" MinX="-100" MaxX="100" MinY="-100" MaxY="100" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    let cxml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" /><Space WorldName="OtherWorld" /></Spaces>"#;
    mgr.create_startup_spaces(cxml).unwrap();

    // Attacker — player at origin with full bandolier.
    mgr.create_entity(ATTACKER_EID, "Castle_CellBlock", [0.0; 3], [0.0; 3])
        .unwrap();
    if let Some(e) = mgr.get_entity_mut(ATTACKER_EID) {
        e.is_player = true;
        e.player_id = Some(100);
        // Weapon already drawn so the attack-while-holstered queue
        // doesn't intercept — these tests are about AoE damage
        // resolution, not the unholster choreography.
        e.weapon_holstered = false;
        e.abilities.add_ability(GROUND_ABILITY_ID);
        e.bandolier_items.insert(
            0,
            BandolierItem {
                instance_id: 0,
                item_id: 1,
                clip_size: 30,
                default_ammo_type: 2,
                current_ammo: 30,
                cur_ammo_type: 2,
            },
        );
        if let Some(stat) = e.stats.get_mut(AMMO_SLOT_1) {
            stat.update(0, 30, 30);
            stat.clear_dirty();
        }
    }

    // Effect: HealthDamage 30, Radius 10.
    let mut effect_params = HashMap::new();
    effect_params.insert("HealthDamage".to_string(), "30".to_string());
    effect_params.insert("Radius".to_string(), "10.0".to_string());
    mgr.effect_defs.insert(
        GROUND_EFFECT_ID,
        EffectDef {
            effect_id: GROUND_EFFECT_ID,
            ability_id: GROUND_ABILITY_ID,
            delay: 0,
            effect_sequence: 0,
            event_set_id: None,
            script_name: None,
            params: effect_params,
            ..Default::default()
        },
    );

    // Ability: ranged, required_ammo=1, max_range=30, no warmup.
    mgr.ability_defs.insert(
        GROUND_ABILITY_ID,
        AbilityDef {
            ability_id: GROUND_ABILITY_ID,
            name: "GroundAoE".to_string(),
            cooldown: 0.5,
            warmup: 0.0,
            flags: 0,
            is_ranged: true,
            min_range: 0,
            max_range: 30,
            target_type_id: 0,
            effect_ids: vec![GROUND_EFFECT_ID],
            moniker_ids: vec![],
            required_ammo: 1,
            event_set_id: None,
            velocity: 0.0,
        },
    );

    let (tx, rx) = mpsc::channel(256);
    (mgr, tx, rx)
}

/// Spawn a hostile NPC at `(x, y, z)` in `world` with `hp` HEALTH.
fn add_hostile_npc(mgr: &mut SpaceManager, eid: u32, world: &str, pos: [f32; 3], hp: i32) {
    mgr.spawn_npc(eid, world, pos, [0.0; 3]).unwrap();
    if let Some(npc) = mgr.get_entity_mut(eid) {
        npc.faction = HOSTILE;
        if let Some(stat) = npc.stats.get_mut(HEALTH) {
            stat.update(0, hp, hp);
            stat.clear_dirty();
        }
    }
}

#[tokio::test]
async fn aoe_damages_all_hostiles_in_radius_excluding_other_spaces_and_outside_radius() {
    // Regression for Copilot 3177444056 + 3177444060: AoE must apply to
    // every in-radius hostile (not just the nearest), and must NOT
    // touch NPCs in different spaces or outside the configured radius.
    let (mut mgr, tx, _rx) = make_aoe_scenario();

    // Three hostile NPCs in the attacker's space within the 10.0 radius.
    add_hostile_npc(&mut mgr, 100, "Castle_CellBlock", [2.0, 0.0, 0.0], 200);
    add_hostile_npc(&mut mgr, 101, "Castle_CellBlock", [0.0, 0.0, 5.0], 200);
    add_hostile_npc(&mut mgr, 102, "Castle_CellBlock", [-3.0, 0.0, 4.0], 200);

    // One in the attacker's space but OUTSIDE the radius (15 > 10).
    add_hostile_npc(&mut mgr, 103, "Castle_CellBlock", [15.0, 0.0, 0.0], 200);

    // One in a DIFFERENT space at the same coords as a primary target
    // (regression for the cross-space leak Copilot 3177444060 flagged).
    add_hostile_npc(&mut mgr, 104, "OtherWorld", [2.0, 0.0, 0.0], 200);

    // Fire ground ability at origin.
    let deaths = handle_use_ability_on_ground(
        ATTACKER_EID,
        GROUND_ABILITY_ID,
        [0.0, 0.0, 0.0],
        &tx,
        &mut mgr,
    )
    .await;

    // No one died (200 HP - 60 damage = 140 left, all alive).
    assert!(deaths.is_empty(), "no kills expected — all NPCs had 200 HP");

    // All three in-radius same-space NPCs should have taken damage.
    for eid in [100u32, 101, 102] {
        let hp = mgr.get_entity(eid).unwrap().stats.get(HEALTH).unwrap().cur;
        assert!(
            hp < 200,
            "hostile {eid} in radius should have taken damage; hp={hp}"
        );
    }

    // The out-of-radius NPC must be untouched.
    assert_eq!(
        mgr.get_entity(103).unwrap().stats.get(HEALTH).unwrap().cur,
        200,
        "NPC 103 (outside 10.0 radius) must not be damaged",
    );

    // The cross-space NPC must be untouched.
    assert_eq!(
        mgr.get_entity(104).unwrap().stats.get(HEALTH).unwrap().cur,
        200,
        "NPC 104 (in OtherWorld at same coords as 100) must not be damaged",
    );

    // Ammo should have decremented exactly once (primary consumed; AoE
    // secondaries pass needs_ammo_stat_send=false).
    let ammo = mgr.get_entity(ATTACKER_EID).unwrap().bandolier_items[&0].current_ammo;
    assert_eq!(
        ammo, 29,
        "ammo should decrement once per AoE invocation, got {ammo}"
    );
}

#[tokio::test]
async fn aoe_skips_secondaries_when_primary_cast_rejected() {
    // Regression for Copilot 3177444056 + CodeRabbit 3177444353 (the
    // critical issue): if handle_use_ability rejects the primary
    // (e.g., ability on cooldown), AoE secondaries must NOT fire —
    // otherwise the player gets free damage to bystanders without
    // paying the cooldown/ammo.
    let (mut mgr, tx, _rx) = make_aoe_scenario();

    // Two hostile NPCs in radius.
    add_hostile_npc(&mut mgr, 200, "Castle_CellBlock", [2.0, 0.0, 0.0], 200);
    add_hostile_npc(&mut mgr, 201, "Castle_CellBlock", [-3.0, 0.0, 4.0], 200);

    // Force the ability onto cooldown so handle_use_ability rejects.
    if let Some(e) = mgr.get_entity_mut(ATTACKER_EID) {
        e.abilities
            .start_ability_cooldown(GROUND_ABILITY_ID, std::time::Duration::from_secs(60));
    }

    let deaths = handle_use_ability_on_ground(
        ATTACKER_EID,
        GROUND_ABILITY_ID,
        [0.0, 0.0, 0.0],
        &tx,
        &mut mgr,
    )
    .await;
    assert!(
        deaths.is_empty(),
        "no deaths expected when primary rejected"
    );

    // BOTH NPCs must be untouched — the primary was rejected so the
    // secondary loop must not have run.
    for eid in [200u32, 201] {
        let hp = mgr.get_entity(eid).unwrap().stats.get(HEALTH).unwrap().cur;
        assert_eq!(
            hp, 200,
            "NPC {eid} must not take damage when primary cast rejected; hp={hp}"
        );
    }

    // Ammo must NOT have decremented (cooldown rejection happens
    // before consume).
    let ammo = mgr.get_entity(ATTACKER_EID).unwrap().bandolier_items[&0].current_ammo;
    assert_eq!(
        ammo, 30,
        "ammo must not decrement when primary cast rejected; ammo={ammo}"
    );
}

#[tokio::test]
async fn aoe_returns_dead_target_ids_for_caller_to_fire_entity_death() {
    // Regression for Copilot 3177444062: AoE secondary kills must
    // surface in the return value so the caller fires entity_death
    // for each (kill-count missions, death-trigger chains).
    let (mut mgr, tx, _rx) = make_aoe_scenario();

    // Three NPCs at very low HP — should die from the 60-damage
    // (HealthDamage 30 × 2 player-bonus) AoE hit.
    add_hostile_npc(&mut mgr, 300, "Castle_CellBlock", [2.0, 0.0, 0.0], 5);
    add_hostile_npc(&mut mgr, 301, "Castle_CellBlock", [0.0, 0.0, 4.0], 5);
    add_hostile_npc(&mut mgr, 302, "Castle_CellBlock", [-3.0, 0.0, 4.0], 5);
    // One survivor (high HP) so we can confirm survivors aren't in the list.
    add_hostile_npc(&mut mgr, 303, "Castle_CellBlock", [1.0, 0.0, 1.0], 1000);

    let deaths = handle_use_ability_on_ground(
        ATTACKER_EID,
        GROUND_ABILITY_ID,
        [0.0, 0.0, 0.0],
        &tx,
        &mut mgr,
    )
    .await;

    // All three low-HP NPCs should be in the deaths Vec; survivor not.
    assert_eq!(deaths.len(), 3, "expected 3 AoE kills, got {deaths:?}");
    for eid in [300u32, 301, 302] {
        assert!(
            deaths.contains(&eid),
            "expected kill {eid} in deaths Vec {deaths:?}"
        );
    }
    assert!(
        !deaths.contains(&303),
        "high-HP survivor must not be in deaths Vec"
    );
}
