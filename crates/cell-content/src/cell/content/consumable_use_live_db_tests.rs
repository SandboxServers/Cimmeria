//! Seed guards for the native consumables: exactly the intended items
//! classify as native, each with the ability its `items_event_sets` row
//! names, and each magnitude NVP equal to the number in its effect's own
//! `effect_desc` (the 2009 rows shipped none, so the seed must not invent
//! one).

use std::collections::{BTreeMap, BTreeSet};

use cimmeria_entity::stats::{FOCUS, HEALTH};

use super::consumable_use::{classify, Classification};
use crate::cell::space_manager::SpaceManager;
use crate::cell::spawner;
use crate::test_support::require_db_or_skip;

/// `(item, ability)` for every health heal, focus heal and stimpack this
/// path is meant to wire. 4735 is a second Health Slappack TC1 row bound
/// to the same ability 648.
const HEALTH_HEALS: [(i32, i32); 12] = [
    (2893, 648),
    (4735, 648),
    (6132, 2246),
    (6239, 2285),
    (6737, 2286),
    (6738, 2287),
    (6739, 2288),
    (6740, 2289),
    (6741, 2290),
    (6742, 2291),
    (6743, 2292),
    (6744, 2293),
];
const FOCUS_HEALS: [(i32, i32); 10] = [
    (6106, 2206),
    (6237, 2276),
    (6243, 2277),
    (6244, 2278),
    (6253, 2279),
    (6255, 2280),
    (6257, 2281),
    (6734, 2282),
    (6735, 2283),
    (6736, 2284),
];
const STIMPACKS: [(i32, i32); 24] = [
    (6677, 2735),
    (6678, 2736),
    (6679, 2737),
    (6680, 2738),
    (6681, 2734),
    (6682, 2739),
    (6697, 2740),
    (6717, 2746),
    (6718, 2752),
    (6719, 2741),
    (6720, 2747),
    (6721, 2753),
    (6722, 2742),
    (6723, 2748),
    (6724, 2754),
    (6725, 2743),
    (6726, 2749),
    (6727, 2755),
    (6728, 2744),
    (6729, 2750),
    (6730, 2756),
    (6731, 2745),
    (6732, 2751),
    (6733, 2757),
];

async fn caches(pool: &sqlx::PgPool) -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    mgr.ability_defs = spawner::load_ability_defs(pool).await.unwrap();
    mgr.effect_defs = spawner::load_effect_defs(pool).await.unwrap();
    mgr.item_event_set_abilities = spawner::load_item_event_set_abilities(pool).await.unwrap();
    mgr
}

/// Every item the seed makes native, and nothing else. A new
/// `HealHealth` / `HealFocus` / `StatBuff` script on an effect some other
/// event-5 ability owns would silently turn that item into a consumable;
/// this fails first.
#[tokio::test]
async fn live_db_exactly_the_intended_items_are_native_consumables() {
    let pool = require_db_or_skip!();
    let mgr = caches(&pool).await;

    let mut native: BTreeMap<i32, (i32, Vec<i32>, bool)> = BTreeMap::new();
    let items: BTreeSet<i32> = mgr
        .item_event_set_abilities
        .keys()
        .filter(|(_, event)| *event == 5)
        .map(|(item, _)| *item)
        .collect();
    let mut placeholders = 0;
    for item in items {
        match classify(item, &mgr) {
            Classification::Native(plan) => {
                native.insert(item, (plan.ability_id, plan.heals, plan.buffs));
            }
            Classification::Placeholder => placeholders += 1,
            _ => {}
        }
    }
    let mut want: BTreeMap<i32, (i32, Vec<i32>, bool)> = BTreeMap::new();
    for (item, ability) in HEALTH_HEALS {
        want.insert(item, (ability, vec![HEALTH], false));
    }
    for (item, ability) in FOCUS_HEALS {
        want.insert(item, (ability, vec![FOCUS], false));
    }
    for (item, ability) in STIMPACKS {
        let effects = mgr.ability_defs[&ability].effect_ids.len();
        assert!(
            (1..=2).contains(&effects),
            "stim {item} ability {ability} has {effects} effects"
        );
        want.insert(item, (ability, vec![], true));
    }
    assert_eq!(native, want, "the native consumable set drifted");
    assert!(
        placeholders > 100,
        "the 597 filler should cover the mission items ({placeholders})"
    );
}

/// The Ambernol vial stays a chain item: its event-5 ability (1374) has no
/// native script, so the native path never applies it.
#[tokio::test]
async fn live_db_the_ambernol_vial_is_not_a_native_consumable() {
    let pool = require_db_or_skip!();
    let mgr = caches(&pool).await;
    assert_eq!(
        classify(19, &mgr),
        Classification::NotNative {
            ability_id: 1374,
            reason: "effect_not_native"
        }
    );
}

/// Each wired magnitude equals the number in its effect's description:
/// `HealAmount` for "Heals N Health" / "Heals N Focus", `<Stat>` for
/// "+N <Stat>". The stat buffs also keep their 3600 s duration and one
/// pulse.
#[tokio::test]
async fn live_db_every_consumable_magnitude_is_its_effect_description() {
    let pool = require_db_or_skip!();
    let mgr = caches(&pool).await;
    let descs: BTreeMap<i32, String> = sqlx::query_as::<_, (i32, Option<String>)>(
        "SELECT effect_id, effect_desc FROM resources.effects",
    )
    .fetch_all(&pool)
    .await
    .unwrap()
    .into_iter()
    .filter_map(|(id, desc)| desc.map(|d| (id, d)))
    .collect();
    let abilities = HEALTH_HEALS
        .iter()
        .chain(FOCUS_HEALS.iter())
        .chain(STIMPACKS.iter())
        .map(|&(_, a)| a)
        .collect::<BTreeSet<_>>();
    let mut checked = 0;
    for ability in abilities {
        for effect_id in &mgr.ability_defs[&ability].effect_ids {
            let effect = &mgr.effect_defs[effect_id];
            let desc = descs[effect_id].trim_end_matches('.').to_string();
            let words: Vec<&str> = desc.split_whitespace().collect();
            match effect.script_name.as_deref() {
                Some("HealHealth") | Some("HealFocus") => {
                    assert_eq!(words.len(), 3, "{effect_id}: {desc}");
                    assert_eq!(words[0], "Heals");
                    assert_eq!(
                        effect.params.get("HealAmount").map(String::as_str),
                        Some(words[1]),
                        "effect {effect_id} ({desc})"
                    );
                }
                Some("StatBuff") => {
                    assert_eq!(words.len(), 2, "{effect_id}: {desc}");
                    let amount = words[0].trim_start_matches('+');
                    assert_eq!(
                        effect.params.get(words[1]).map(String::as_str),
                        Some(amount),
                        "effect {effect_id} ({desc})"
                    );
                    assert_eq!(effect.pulse_duration, 3600.0, "{effect_id}");
                    assert_eq!(effect.pulse_count, 1, "{effect_id}");
                }
                other => panic!("effect {effect_id} has script {other:?}"),
            }
            checked += 1;
        }
    }
    assert_eq!(checked, 11 + 10 + 42, "effects checked");
}
