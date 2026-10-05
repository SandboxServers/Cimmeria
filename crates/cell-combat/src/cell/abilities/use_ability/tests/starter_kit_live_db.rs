//! The debug kit against the real seed: what a debug-kit character (the
//! seeded playtest characters, Class Start v6 lock L2) holds at spawn works
//! once the pistol is reloaded.
//!
//! The kit comes from the seed, not from constants here: the abilities from
//! `char_creation_debug_kit_abilities`, the weapon from
//! `char_creation_debug_kit_items`, and the weapon's RANGED binding from
//! `items_event_sets`. Since CS-02 a normal character starts with no kit, and
//! the pistol is created empty (OD-CS13 amendment): the tests load the
//! magazine the way the player's one free reload does.

use cimmeria_entity::cell_entity::BandolierItem;
use cimmeria_entity::stats::{FOCUS, HEALTH};

use super::*;
use crate::cell::spawner::{
    load_ability_defs, load_ammo_catalog, load_effect_defs, EVENT_ITEM_RANGED,
};
use crate::test_support::require_db_or_skip;

const PISTOL_SHOT: i32 = 592;
const HEAL_FOCUS: i32 = 597;
const HEALTH_HEAL: i32 = 1646;
const RECUPERATION: i32 = 1218;
const PLAYER: u32 = 1;
const MOB: u32 = 2;
const MAX: i32 = 1000;
const START_HEALTH: i32 = 500;

/// The seed's debug kit.
struct Kit {
    abilities: Vec<i32>,
    weapon: BandolierItem,
    /// What Pistol Shot fires with the weapon drawn.
    ranged_ability: i32,
}

async fn seeded_kit(pool: &sqlx::PgPool) -> Kit {
    let abilities: Vec<i32> =
        sqlx::query_scalar("SELECT ability_id FROM resources.char_creation_debug_kit_abilities")
            .fetch_all(pool)
            .await
            .expect("starter abilities");
    let (item_id, clip_size, default_ammo_type): (i32, i32, i32) = sqlx::query_as(
        "SELECT ri.item_id, ri.clip_size, \
                COALESCE(array_position(enum_range(NULL::resources.\"EAmmoType\"), \
                                        ri.default_ammo_type) - 1, 0) \
         FROM resources.char_creation_debug_kit_items ci \
         JOIN resources.items ri ON ri.item_id = ci.item_id \
         WHERE 3 = ANY(ri.container_sets) \
         ORDER BY ri.item_id LIMIT 1",
    )
    .fetch_one(pool)
    .await
    .expect("the debug kit has a bandolier weapon");
    let ranged_ability: i32 = sqlx::query_scalar(
        "SELECT ability_id FROM resources.items_event_sets WHERE item_id = $1 AND event_id = $2",
    )
    .bind(item_id)
    .bind(EVENT_ITEM_RANGED)
    .fetch_one(pool)
    .await
    .expect("the starter weapon has a RANGED binding");
    Kit {
        abilities,
        weapon: BandolierItem {
            instance_id: 1,
            item_id,
            clip_size,
            default_ammo_type,
            // Created empty (OD-CS13); this is after the one free reload.
            current_ammo: clip_size,
            cur_ammo_type: default_ammo_type,
        },
        ranged_ability,
    }
}

/// A spawned character holding the kit, weapon drawn in slot 0, at half
/// Health and no Focus, next to a hostile mob. The seeded warmups are the
/// warmup tests' business, so they are zeroed here.
async fn spawned(pool: &sqlx::PgPool, kit: &Kit) -> SpaceManager {
    let mut mgr = make_mgr();
    let abilities = load_ability_defs(pool).await.expect("ability defs load");
    mgr.effect_defs = load_effect_defs(pool).await.expect("effect defs load");
    mgr.ammo_catalog = load_ammo_catalog(pool).await.expect("ammo catalog loads");
    crate::test_support::install_effect_scripts(&mut mgr);
    for id in kit.abilities.iter().copied().chain([kit.ranged_ability]) {
        let def = abilities
            .get(&id)
            .unwrap_or_else(|| panic!("ability {id} is seeded"));
        mgr.ability_defs.insert(
            id,
            AbilityDef {
                warmup: 0.0,
                ..def.clone()
            },
        );
    }
    mgr.item_event_set_abilities
        .insert((kit.weapon.item_id, EVENT_ITEM_RANGED), kit.ranged_ability);

    make_player(&mut mgr, PLAYER, [0.0; 3]);
    mgr.create_entity(MOB, "Castle_CellBlock", [3.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    mgr.get_entity_mut(MOB).unwrap().faction = crate::cell::combat::HOSTILE_FACTION;
    let p = mgr.get_entity_mut(PLAYER).unwrap();
    for &id in &kit.abilities {
        p.abilities.add_ability(id);
    }
    p.weapon_holstered = false;
    p.active_bandolier_slot = 0;
    p.bandolier_items.insert(0, kit.weapon.clone());
    p.stats
        .get_mut(HEALTH)
        .unwrap()
        .update(0, START_HEALTH, MAX);
    p.stats.get_mut(FOCUS).unwrap().update(0, 0, MAX);
    p.stats.clear_dirty();
    mgr
}

fn pool_of(mgr: &SpaceManager, stat: i32) -> i32 {
    mgr.get_entity(PLAYER).unwrap().stats.get(stat).unwrap().cur
}

/// **Regression guard.** Pistol Shot pressed at a mob fires the starter
/// pistol's ranged ability and spends a round: no NoAmmo refusal. With the
/// magazine left empty (the kit before the fix) the cast is refused.
#[tokio::test]
async fn pistol_shot_fires_the_starter_pistol_at_spawn_live_db() {
    let pool = require_db_or_skip!();
    let kit = seeded_kit(&pool).await;
    assert!(
        kit.abilities.contains(&PISTOL_SHOT),
        "Pistol Shot is a starter"
    );
    assert!(
        kit.weapon.clip_size > 0,
        "the starter weapon has a magazine"
    );

    let mut mgr = spawned(&pool, &kit).await;
    let (tx, _rx) = mpsc::channel(256);
    assert!(
        handle_use_ability(PLAYER, PISTOL_SHOT, MOB as i32, &tx, &mut mgr).await,
        "Pistol Shot with the loaded starter pistol must fire, not draw NoAmmo"
    );
    let p = mgr.get_entity(PLAYER).unwrap();
    assert!(
        p.abilities.is_on_cooldown(kit.ranged_ability),
        "592 resolves to the pistol's RANGED binding {} and that is what fired",
        kit.ranged_ability
    );
    assert_eq!(
        p.active_ammo(),
        kit.weapon.clip_size - 1,
        "the shot spends one round"
    );

    // The bug shape: the same press with an empty magazine is refused.
    let mut empty = spawned(&pool, &kit).await;
    empty
        .get_entity_mut(PLAYER)
        .unwrap()
        .bandolier_items
        .get_mut(&0)
        .unwrap()
        .current_ammo = 0;
    assert!(
        !handle_use_ability(PLAYER, PISTOL_SHOT, MOB as i32, &tx, &mut empty).await,
        "an empty magazine is refused (NoAmmo): the starter kit must ship loaded"
    );
}

/// The three starter heals, pressed with nothing selected at spawn, land on
/// the caster (AB-01): Heal Focus restores Focus, Health Heal and the first
/// Recuperation pulse restore Health.
#[tokio::test]
async fn starter_heals_land_on_the_caster_at_spawn_live_db() {
    let pool = require_db_or_skip!();
    let kit = seeded_kit(&pool).await;
    for id in [HEAL_FOCUS, HEALTH_HEAL, RECUPERATION] {
        assert!(kit.abilities.contains(&id), "{id} is a starter");
    }
    let (tx, _rx) = mpsc::channel(256);

    let mut mgr = spawned(&pool, &kit).await;
    assert!(handle_use_ability(PLAYER, HEAL_FOCUS, 0, &tx, &mut mgr).await);
    assert!(
        pool_of(&mgr, FOCUS) > 0,
        "Heal Focus restores the caster's Focus"
    );

    let mut mgr = spawned(&pool, &kit).await;
    assert!(handle_use_ability(PLAYER, HEALTH_HEAL, 0, &tx, &mut mgr).await);
    assert!(
        pool_of(&mgr, HEALTH) > START_HEALTH,
        "Health Heal restores the caster's Health"
    );

    let mut mgr = spawned(&pool, &kit).await;
    assert!(handle_use_ability(PLAYER, RECUPERATION, 0, &tx, &mut mgr).await);
    assert!(
        pool_of(&mgr, HEALTH) > START_HEALTH,
        "Recuperation's first pulse lands on the caster at once"
    );
}
