//! Beneficial darts through the whole shot (ammo campaign AM-11c, D-AM07,
//! and AM-11d's ally targeting): `handle_use_ability` fires a dart loaded
//! with Stim, and the on-hit effect lands on an ally, never on a hostile.
//!
//! This is an integration test, not a module under `damage_apply`, because
//! AM-11c owns no file in this crate. It drives only public entry points:
//! `handle_use_ability` and the `SpaceManager` it mutates.
//!
//! `ammo.finite_special` is switched on for the process and never off. Every
//! test here wants it on.

use std::collections::HashMap;

use cimmeria_cell_catalog::cell::spawner::{AmmoCatalog, AmmoModifier};
use cimmeria_cell_combat::cell::abilities::handle_use_ability;
use cimmeria_cell_world::cell::combat::faction_reaction::HOSTILE_FACTION;
use cimmeria_cell_world::cell::effects::ammo_dart_support::DART_SUPPORT_DAMAGE_MULT;
use cimmeria_cell_world::cell::space_manager::SpaceManager;
use cimmeria_entity::abilities::{AbilityDef, EffectDef};
use cimmeria_entity::ammo_type::{DART_DEFAULT, DART_STIM};
use cimmeria_entity::cell_entity::BandolierItem;
use cimmeria_entity::stats::{FOCUS, HEALTH};
use tokio::sync::mpsc;

const SHOT: i32 = 7;
const SHOT_EFFECT: i32 = 100;
/// The seeded Stim on-hit effect: `HealFocus`, 10%.
const STIM_EFFECT: i32 = 9160;
const TARGET_HEALTH: i32 = 100_000;
const TARGET_FOCUS_MAX: i32 = 1000;

/// Player 1 with a dart pistol loaded with `ammo_type`, a weapon shot
/// (`required_ammo = 1`, 1000 HealthDamage) it knows, the seeded Stim row
/// and its on-hit effect, and entity 2 at full health and empty Focus.
/// Entity 2 is a hostile NPC, or a second player (an ally) when
/// `target_is_player`.
fn world(ammo_type: i32, target_is_player: bool) -> SpaceManager {
    cimmeria_entity::ammo_feature::set_finite_special(true);
    let mut mgr = SpaceManager::new(1);
    mgr.parse_spaces_xml(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#,
    )
    .unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#,
    )
    .unwrap();
    mgr.create_entity(1, "Castle", [0.0; 3], [0.0; 3]).unwrap();
    mgr.create_entity(2, "Castle", [1.0, 0.0, 0.0], [0.0; 3])
        .unwrap();

    mgr.ability_defs.insert(
        SHOT,
        AbilityDef {
            ability_id: SHOT,
            name: "Dart Pistol Auto Attack".to_string(),
            cooldown: 0.0,
            warmup: 0.0,
            flags: 0,
            is_ranged: true,
            min_range: 0.0,
            max_range: 30.0,
            target_type_id: 2,
            effect_ids: vec![SHOT_EFFECT],
            moniker_ids: vec![],
            required_ammo: 1,
            event_set_id: None,
            velocity: 0.0,
        },
    );
    mgr.effect_defs.insert(
        SHOT_EFFECT,
        EffectDef {
            effect_id: SHOT_EFFECT,
            ability_id: SHOT,
            params: HashMap::from([("HealthDamage".to_string(), "1000".to_string())]),
            ..Default::default()
        },
    );
    mgr.effect_defs.insert(
        STIM_EFFECT,
        EffectDef {
            effect_id: STIM_EFFECT,
            ability_id: 992,
            script_name: Some("HealFocus".to_string()),
            params: HashMap::from([("HealPercentage".to_string(), "10".to_string())]),
            ..Default::default()
        },
    );
    mgr.ammo_catalog = AmmoCatalog::from_rows(
        [AmmoModifier {
            ammo_type: DART_STIM,
            damage_mult: DART_SUPPORT_DAMAGE_MULT,
            penetration_mult: 1.0,
            damage_type: None,
            on_hit_effect_id: Some(STIM_EFFECT),
            toggle_ability_id: 992,
            beneficial: true,
        }],
        [(DART_STIM, 9010)],
    );

    let p = mgr.get_entity_mut(1).unwrap();
    p.is_player = true;
    p.player_id = Some(100);
    p.abilities.add_ability(SHOT);
    // Drawn, so the shot fires now instead of queueing an unholster.
    p.set_weapon_holstered(false);
    p.active_bandolier_slot = 0;
    p.bandolier_items.insert(
        0,
        BandolierItem {
            instance_id: 1,
            item_id: 1520,
            clip_size: 100,
            default_ammo_type: DART_DEFAULT,
            current_ammo: 100,
            cur_ammo_type: ammo_type,
        },
    );

    let t = mgr.get_entity_mut(2).unwrap();
    if target_is_player {
        t.is_player = true;
        t.player_id = Some(200);
    } else {
        t.faction = HOSTILE_FACTION;
    }
    t.stats
        .get_mut(HEALTH)
        .unwrap()
        .update(0, TARGET_HEALTH, TARGET_HEALTH);
    t.stats
        .get_mut(FOCUS)
        .unwrap()
        .update(0, 0, TARGET_FOCUS_MAX);
    mgr.connect_entity(1);
    if target_is_player {
        mgr.connect_entity(2);
    }
    let _ = mgr.compute_aoi_changes();
    mgr
}

fn pools(mgr: &SpaceManager) -> (i32, i32) {
    let s = &mgr.get_entity(2).unwrap().stats;
    (s.get(HEALTH).unwrap().cur, s.get(FOCUS).unwrap().cur)
}

/// One shot at entity 2, with the cooldown the last one started cleared.
async fn fire(mgr: &mut SpaceManager) -> bool {
    mgr.get_entity_mut(1)
        .unwrap()
        .abilities
        .clear_all_cooldowns();
    let (tx, _rx) = mpsc::channel(4096);
    handle_use_ability(1, SHOT, 2, &tx, mgr).await
}

/// A Stim dart on an ally (AM-11d): the shot deals no damage and the on-hit
/// HealFocus restores 10% of the ally's Focus. The same shot at the ally
/// with default darts is refused by the #444 gate, so the heal is the Stim
/// row's doing.
#[tokio::test]
async fn a_stim_dart_heals_an_ally_and_deals_no_damage() {
    let mut mgr = world(DART_STIM, true);
    assert!(fire(&mut mgr).await, "a support shot at an ally commits");
    let (health, focus) = pools(&mgr);
    assert_eq!(health, TARGET_HEALTH, "a support dart deals no damage");
    assert_eq!(focus, TARGET_FOCUS_MAX / 10, "one shot restores 10% Focus");

    let mut control = world(DART_DEFAULT, true);
    assert!(
        !fire(&mut control).await,
        "default darts at an ally are refused"
    );
    assert_eq!(pools(&control), (TARGET_HEALTH, 0));
}

/// A Stim dart never helps a hostile NPC (AM-11d): the launch is refused,
/// so there is no damage, no heal and no ammo spent. The same shot with
/// default darts commits, so the refusal is the beneficial row's doing.
#[tokio::test]
async fn a_stim_dart_at_a_hostile_npc_does_nothing() {
    let mut mgr = world(DART_STIM, false);
    for _ in 0..5 {
        assert!(
            !fire(&mut mgr).await,
            "a support shot at a hostile is refused"
        );
    }
    assert_eq!(pools(&mgr), (TARGET_HEALTH, 0));
    assert_eq!(
        mgr.get_entity(1).unwrap().active_ammo(),
        100,
        "a refused support shot spends no ammo"
    );

    let mut control = world(DART_DEFAULT, false);
    assert!(
        fire(&mut control).await,
        "default darts at a hostile commit"
    );
}
