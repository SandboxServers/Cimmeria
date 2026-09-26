//! Live-DB guards for the animation links on
//! `resources.abilities.event_set_id`.
//!
//! `use_ability/handle.rs` sends the Ability_Begin / Ability_End
//! `onSequence` only when the ability carries an event set, so an ability
//! with a NULL `event_set_id` deals damage with no animation. The client has
//! no ability-keyed lookup of its own: `onSequence` carries an opaque
//! sequence id, and the `KIS-abilities_*` packages hold only generic
//! mechanic templates (single-shot, burst, melee, deployable, beam source).
//! The per-ability link lived in CME's server data, which was never
//! released; the seed is Project Giza's reconstruction from the client and
//! recovered only 35 links.
//!
//! Those 35 follow one rule: the ability's weapon family (`item_monikers`)
//! and its melee/ranged flag pick the event set. Melee on any weapon is 300
//! ("Generic melee weapon source"), ranged single-shot weapons are 3, ranged
//! automatic weapons are 15. Families with no recovered auto-attack link are
//! placed by what their items fire in `items_event_sets` (SMG and assault
//! rifle items fire 559 on set 15; ribbon devices fire 712 on set 300) or by
//! delivery (grenade launcher and energy pistol single-shot, flamethrower
//! sustained). Abilities with no weapon moniker are linked one by one; see
//! `docs/reverse-engineering/findings/ability-animation-links.md`.

use crate::cell::spawner::EVENT_ABILITY_END;
use crate::test_support::require_db_or_skip;

/// `ITEM_*` moniker ids from `db/resources/Entities/Seed/monikers.sql` whose
/// melee auto-attack is recovered on event set 300.
const MELEE_FAMILIES: &[i64] = &[
    1_115_110_575, // ITEM_LightMG
    1_383_013_887, // ITEM_Staff
    1_385_654_633, // ITEM_Fists
    2_035_259_765, // ITEM_Shotgun
    2_389_790_449, // ITEM_DartPistol
    2_445_422_768, // ITEM_Pistol
    2_723_649_405, // ITEM_Zat
    2_882_868_408, // ITEM_Rifle
    312_541_303,   // ITEM_Grenade_Launcher
    3_175_425_141, // ITEM_Automatic_Weapon
    3_257_416_555, // ITEM_Dart_Rifle
    4_193_235_610, // ITEM_RibbonDevice
    4_283_851_787, // Item_Flamethrower
    830_336_901,   // ITEM_Blade
];

/// Ranged single-shot families: event set 3.
const SINGLE_SHOT_FAMILIES: &[i64] = &[
    2_445_422_768, // ITEM_Pistol
    2_882_868_408, // ITEM_Rifle
    2_035_259_765, // ITEM_Shotgun
    1_383_013_887, // ITEM_Staff
    2_723_649_405, // ITEM_Zat
    2_389_790_449, // ITEM_DartPistol
    312_541_303,   // ITEM_Grenade_Launcher (delivery guess)
    1_009_441_186, // ITEM_Energy_Pistol (delivery guess)
];

/// Ranged automatic families: event set 15.
const BURST_FAMILIES: &[i64] = &[
    3_175_425_141, // ITEM_Automatic_Weapon
    1_115_110_575, // ITEM_LightMG
    3_257_416_555, // ITEM_Dart_Rifle
    728_213_066,   // ITEM_SMG (items fire 559)
    3_606_086_656, // ITEM_Assault_Rifle (items fire 559)
    4_283_851_787, // Item_Flamethrower (delivery guess)
];

/// Ranged families whose items fire a set-300 attack: the ribbon device
/// (items fire 712, which the seed recovered on 300).
const RANGED_ON_MELEE_SET_FAMILIES: &[i64] = &[
    4_193_235_610, // ITEM_RibbonDevice
];

/// Every active damage ability whose weapon monikers all fall in one family
/// class carries an event set. Revert the seed links and this lists the
/// ~200 abilities that went silent again.
#[tokio::test]
async fn weapon_bound_damage_abilities_carry_an_event_set() {
    let pool = require_db_or_skip!();

    let silent: Vec<(i32, String)> = sqlx::query_as(
        "SELECT ability_id, name FROM resources.abilities \
         WHERE event_set_id IS NULL AND NOT passive_yn \
           AND type_id IN ('ABILITY_TYPE_DD', 'ABILITY_TYPE_DOT') \
           AND cardinality(item_monikers) > 0 \
           AND ((NOT is_ranged AND item_monikers <@ $1::bigint[]) \
             OR (is_ranged AND (item_monikers <@ $2::bigint[] \
                             OR item_monikers <@ $3::bigint[] \
                             OR item_monikers <@ $4::bigint[]))) \
         ORDER BY ability_id",
    )
    .bind(MELEE_FAMILIES)
    .bind(SINGLE_SHOT_FAMILIES)
    .bind(BURST_FAMILIES)
    .bind(RANGED_ON_MELEE_SET_FAMILIES)
    .fetch_all(&pool)
    .await
    .expect("query must succeed");

    assert!(
        silent.is_empty(),
        "{} weapon-bound damage abilities have a NULL event_set_id and will deal \
         damage with no animation: {silent:?}",
        silent.len()
    );
}

/// The links follow the family rule, not just "some event set": a melee
/// special that pointed at a ranged set would play a gunshot for a staff
/// swing. 1033 (Aimed Burst's charged set) is the one recovered exception.
#[tokio::test]
async fn weapon_bound_links_match_the_family_rule() {
    let pool = require_db_or_skip!();

    let wrong: Vec<(i32, String, bool, i32)> = sqlx::query_as(
        "SELECT ability_id, name, is_ranged, event_set_id FROM resources.abilities \
         WHERE event_set_id IS NOT NULL AND event_set_id <> 1033 \
           AND cardinality(item_monikers) > 0 AND ( \
             (NOT is_ranged AND item_monikers <@ $1::bigint[] AND event_set_id <> 300) \
          OR (is_ranged AND item_monikers <@ $2::bigint[] AND event_set_id <> 3) \
          OR (is_ranged AND item_monikers <@ $3::bigint[] AND event_set_id <> 15) \
          OR (is_ranged AND item_monikers <@ $4::bigint[] AND event_set_id <> 300)) \
         ORDER BY ability_id",
    )
    .bind(MELEE_FAMILIES)
    .bind(SINGLE_SHOT_FAMILIES)
    .bind(BURST_FAMILIES)
    .bind(RANGED_ON_MELEE_SET_FAMILIES)
    .fetch_all(&pool)
    .await
    .expect("query must succeed");

    assert!(
        wrong.is_empty(),
        "links that break the weapon-family rule: {wrong:?}"
    );
}

/// Every event set an ability points at has an Ability_End sequence, the
/// one `handle.rs` needs to animate the hit. A link to a set without one
/// is as silent as NULL.
#[tokio::test]
async fn every_ability_event_set_has_an_ability_end_sequence() {
    let pool = require_db_or_skip!();

    let dangling: Vec<(i32, i32)> = sqlx::query_as(
        "SELECT a.ability_id, a.event_set_id FROM resources.abilities a \
         WHERE a.event_set_id IS NOT NULL AND NOT EXISTS ( \
             SELECT 1 FROM resources.event_sets_sequences ess \
             JOIN resources.sequences s ON s.sequence_id = ess.sequence_id \
             WHERE ess.event_set_id = a.event_set_id AND s.event_id = $1) \
         ORDER BY a.ability_id",
    )
    .bind(EVENT_ABILITY_END)
    .fetch_all(&pool)
    .await
    .expect("query must succeed");

    assert!(
        dangling.is_empty(),
        "abilities linked to an event set with no Ability_End sequence: {dangling:?}"
    );
}

/// The NPC kits that motivated the restore animate, including the abilities
/// with no weapon moniker that are linked one by one.
#[tokio::test]
async fn npc_special_kits_are_linked() {
    let pool = require_db_or_skip!();

    for (ability_id, want) in [
        (598, 15),    // Quick Burst
        (718, 15),    // Selective Fire
        (891, 15),    // Suppression Shot
        (720, 15),    // Cover Fire (assault rifle)
        (653, 3),     // Blast
        (2024, 3),    // Charged Blast
        (1984, 300),  // Staff Swing
        (2025, 300),  // Whirlwind
        (1613, 300),  // Ashrak Dagger: Back Slash
        (1624, 300),  // Ribbon Device: Fear
        (523, 296),   // Concussive Grenade
        (854, 296),   // Smoke Grenade
        (861, 296),   // High-Explosive Grenade
        (868, 296),   // Flashbang Grenade
        (660, 300),   // Bash
        (983, 300),   // Bite
        (1076, 300),  // Lok'nel
        (1077, 300),  // Lok'nel Kei
        (1433, 300),  // Mob Strike
        (1186, 300),  // Drone Strike
        (2146, 300),  // Takedown
        (1174, 15),   // Drone Shot
        (1156, 1499), // Straegis: Disengage
        (1240, 1507), // Straegis Explode
        (2847, 1497), // Straegis: Dissonance
    ] {
        let got: Option<i32> = sqlx::query_scalar(
            "SELECT event_set_id FROM resources.abilities WHERE ability_id = $1",
        )
        .bind(ability_id)
        .fetch_one(&pool)
        .await
        .expect("ability must exist");
        assert_eq!(got, Some(want), "ability {ability_id}");
    }
}
