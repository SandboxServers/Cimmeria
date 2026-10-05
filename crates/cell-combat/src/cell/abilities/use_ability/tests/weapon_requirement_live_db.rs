//! The Class Start v6 starter cases (CS-07, OD-CS11) against the real seed:
//! the abilities' `item_monikers` and the weapons' `moniker_ids` as loaded
//! by the server's own loaders.
//!
//! Each press is judged on the weapon rule alone: a passing case must not
//! draw `onErrorCode(0, ability, 63)`, the refused case must draw exactly
//! that. Whether the ability then has a mechanic or ammo is other gates'
//! business.

use cimmeria_entity::cell_entity::BandolierItem;

use super::super::weapon_requirement::WRONG_WEAPON_TYPE_ERROR_CODE;
use super::warmup::calls;
use super::*;
use crate::cell::spawner::{load_ability_defs, load_effect_defs, load_weapon_monikers};
use crate::test_support::require_db_or_skip;

const PLAYER: u32 = 1;
const MOB: u32 = 2;

/// Press `ability` with seeded `item` active; return whether the press drew
/// WrongWeaponType.
async fn wrong_weapon_live_db(pool: &sqlx::PgPool, ability: i32, item: i32) -> bool {
    let mut mgr = make_mgr();
    let defs = load_ability_defs(pool).await.expect("ability defs load");
    mgr.effect_defs = load_effect_defs(pool).await.expect("effect defs load");
    mgr.item_monikers = load_weapon_monikers(pool)
        .await
        .expect("item monikers load");
    let def = defs
        .get(&ability)
        .unwrap_or_else(|| panic!("ability {ability} is seeded"));
    mgr.ability_defs.insert(
        ability,
        AbilityDef {
            warmup: 0.0,
            ..def.clone()
        },
    );
    make_player(&mut mgr, PLAYER, [0.0; 3]);
    mgr.create_entity(MOB, "Castle_CellBlock", [3.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    mgr.get_entity_mut(MOB).unwrap().faction = crate::cell::combat::HOSTILE_FACTION;
    let p = mgr.get_entity_mut(PLAYER).unwrap();
    p.abilities.add_ability(ability);
    p.weapon_holstered = false;
    p.active_bandolier_slot = 0;
    p.bandolier_items.insert(
        0,
        BandolierItem {
            instance_id: 1,
            item_id: item,
            clip_size: 30,
            default_ammo_type: 0,
            current_ammo: 30,
            cur_ammo_type: 0,
        },
    );
    let (tx, mut rx) = mpsc::channel(256);
    handle_use_ability(PLAYER, ability, MOB as i32, &tx, &mut mgr).await;
    let mut err = vec![0u8];
    err.extend_from_slice(&ability.to_le_bytes());
    err.extend_from_slice(&WRONG_WEAPON_TYPE_ERROR_CODE.to_le_bytes());
    calls(&drain(&mut rx))
        .iter()
        .any(|(e, m, a)| *e == PLAYER && *m == method_idx::ON_ERROR_CODE && *a == err)
}

/// **Regression guard (OD-CS11 on the seed).** 592 + 55, 598 + 21,
/// 1984 + 2797 and 1639 + 4565 pass the weapon rule; 598 + 3260 (SK37 LMG,
/// ITEM_LightMG only) is refused with WrongWeaponType. Without the loader
/// change `item_monikers` is empty and 598 + 3260 passes.
#[tokio::test]
async fn class_start_starter_cases_hold_on_the_seed_live_db() {
    let pool = require_db_or_skip!();
    for (ability, item, refused) in [
        (592, 55, false),
        (598, 21, false),
        (598, 3260, true),
        (1984, 2797, false),
        (1639, 4565, false),
    ] {
        assert_eq!(
            wrong_weapon_live_db(&pool, ability, item).await,
            refused,
            "ability {ability} with item {item}: WrongWeaponType expected = {refused}"
        );
    }
}
