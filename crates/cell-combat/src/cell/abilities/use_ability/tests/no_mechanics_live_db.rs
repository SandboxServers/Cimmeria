//! AB-12 against the real seed: how many abilities have a mechanic today,
//! and the abilities that work today must never be refused.
//!
//! The count moves as the generator packets land (AB-03 damage numbers,
//! AB-04 stat effects, AB-10 shields and purges): update [`HAS_MECHANICS_TODAY`] after each merge.
//! The test prints the count it sees, so the new value is in the output.

use cimmeria_entity::abilities::{ability_effects_have_mechanics, AbilityDef};

use super::super::is_owner_pet_ability;
use super::super::no_mechanics::ability_has_mechanics;
use crate::cell::cell_methods::player::world::reload::ABILITY_RELOAD_WEAPON;
use crate::cell::cover::COVER_STANCE_ABILITY;
use crate::cell::space_manager::SpaceManager;
use crate::cell::spawner::{
    load_ability_defs, load_ammo_catalog, load_deployables, load_effect_defs, load_pet_summons,
};
use crate::test_support::require_db_or_skip;

/// Seeded abilities with a mechanic on `main` after AB-01, AB-02, AB-03, AB-04, AB-06,
/// AB-07, AB-08, AB-09 and AB-10 (AB-09's crowd-control scripts land on abilities
/// that all had a damage effect already; AB-10 adds Personal Shield 1013, the mitigation
/// toggles 1016, 1017, 1018, 1235 and the purges 2027, 2099, 2865). The one
/// number to update when a packet lights up more abilities.
const HAS_MECHANICS_TODAY: usize = 284;

/// The archetype starters every new character holds: Pistol Shot, Strike,
/// Heal Focus, Health Heal, Recuperation.
const STARTERS: [i32; 5] = [592, 594, 597, 1646, 1218];

async fn seeded_manager(pool: &sqlx::PgPool) -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    mgr.ability_defs = load_ability_defs(pool).await.expect("ability defs load");
    mgr.effect_defs = load_effect_defs(pool).await.expect("effect defs load");
    mgr.pet_summons = load_pet_summons(pool).await.expect("pet summons load");
    mgr.deployable_specs = load_deployables(pool).await.expect("deployables load");
    mgr.ammo_catalog = load_ammo_catalog(pool).await.expect("ammo catalog loads");
    // The cell installs the registry at startup; the predicate reads it.
    crate::test_support::install_effect_scripts(&mut mgr);
    mgr
}

async fn ids(pool: &sqlx::PgPool, sql: &'static str) -> Vec<i32> {
    sqlx::query_scalar::<_, i32>(sql)
        .fetch_all(pool)
        .await
        .unwrap_or_else(|e| panic!("{sql}: {e}"))
}

/// Pins today's count of seeded abilities with a mechanic.
#[tokio::test]
async fn seeded_has_mechanics_count_live_db() {
    let pool = require_db_or_skip!();
    let mgr = seeded_manager(&pool).await;

    let mut with: Vec<&AbilityDef> = mgr
        .ability_defs
        .values()
        .filter(|d| ability_has_mechanics(&mgr, d))
        .collect();
    with.sort_by_key(|d| d.ability_id);
    let animation_only = mgr
        .ability_defs
        .values()
        .filter(|d| !ability_has_mechanics(&mgr, d) && d.event_set_id.is_some())
        .count();
    let scripts = mgr.effect_scripts().clone();
    let by_effects = |d: &AbilityDef| {
        ability_effects_have_mechanics(d, &mgr.effect_defs, |s| scripts.contains(s))
    };
    let via_effects = with.iter().filter(|d| by_effects(d)).count();
    let weapon_shot_only: Vec<i32> = with
        .iter()
        .filter(|d| !by_effects(d) && d.required_ammo > 0)
        .map(|d| d.ability_id)
        .collect();
    println!(
        "AB-12: {} of {} seeded abilities have a mechanic ({via_effects} through their \
         effects, {} only as weapon shots: {weapon_shot_only:?}); {animation_only} more \
         animate only",
        with.len(),
        mgr.ability_defs.len(),
        weapon_shot_only.len(),
    );
    assert_eq!(
        with.len(),
        HAS_MECHANICS_TODAY,
        "the has-mechanics count moved; if a packet just gave abilities their \
         numbers or scripts, update HAS_MECHANICS_TODAY to the printed count"
    );
}

/// Never refuse what works today: the five starters, Cover Stance, Reload, every
/// ammo toggle, every pet summon, every deployable and every owner-pet
/// ability. The lists come from the seed tables, so a new row is guarded
/// too.
#[tokio::test]
async fn abilities_that_work_today_have_mechanics_live_db() {
    let pool = require_db_or_skip!();
    let mgr = seeded_manager(&pool).await;

    let toggles = ids(
        &pool,
        "SELECT DISTINCT toggle_ability_id FROM resources.ammo_modifiers",
    )
    .await;
    let summons = ids(&pool, "SELECT ability_id FROM resources.pet_summons").await;
    let deployables = ids(&pool, "SELECT ability_id FROM resources.deployables").await;
    let owner_pet: Vec<i32> = mgr
        .ability_defs
        .keys()
        .copied()
        .filter(|&id| is_owner_pet_ability(&mgr, id))
        .collect();
    assert!(!toggles.is_empty(), "ammo toggles are seeded");
    assert!(!summons.is_empty(), "pet summons are seeded");
    assert!(!deployables.is_empty(), "deployables are seeded");
    assert!(!owner_pet.is_empty(), "owner-pet abilities are seeded");

    let groups: [(&str, Vec<i32>); 7] = [
        ("starter", STARTERS.to_vec()),
        ("cover stance", vec![COVER_STANCE_ABILITY]),
        ("reload", vec![ABILITY_RELOAD_WEAPON]),
        ("ammo toggle", toggles),
        ("pet summon", summons),
        ("deployable", deployables),
        ("owner-pet ability", owner_pet),
    ];
    for (group, list) in groups {
        for id in list {
            let def = mgr
                .ability_defs
                .get(&id)
                .unwrap_or_else(|| panic!("{group} {id} is seeded"));
            assert!(
                ability_has_mechanics(&mgr, def),
                "{group} {id} ({}) works today and must never draw the no-effect refusal",
                def.name
            );
        }
    }
}
