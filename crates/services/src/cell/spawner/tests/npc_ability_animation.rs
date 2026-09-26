//! Live-DB seed linter for NPC attack animations (NA43; handoff §14, §26
//! tests 18 and 19).
//!
//! An NPC attack animates only through this chain: the ability's
//! `event_set_id` → `event_sets_sequences` → the one `sequences` row with
//! event 1001 (Ability_End), whose `sequence_id` goes out in `onSequence`.
//! Break any link and `handle_use_ability` still deals the damage, with no
//! animation on any client. 1851 of the 1886 seeded abilities have no event
//! set, so the check is scoped to what NPCs actually fire: the effective
//! abilities (the template's set, or `NPC_DEFAULT_ABILITY` when it has none)
//! of every template that is spawned hostile or carries an ability set.
//!
//! The third test is the §14 weapon check: a hostile template that names a
//! `weapon_item_id` must fire that weapon's ranged auto attack
//! (`items_event_sets` event 7), not the pistol fallback. Before NA43 the
//! Castle SMG guards 148, 169 and 170 failed it.

use std::collections::{BTreeMap, BTreeSet};

use crate::cell::combat::NPC_DEFAULT_ABILITY;
use crate::cell::spawner::{
    load_ability_defs, load_event_set_sequences, load_spawn_templates, EVENT_ABILITY_END,
};
use crate::test_support::require_db_or_skip;

/// Abilities allowed to reach an NPC without a resolvable Ability_End. Each
/// entry needs a reason. Empty on purpose: every NPC combat ability animates
/// today, and a new exception should be argued in review.
const ANIMATION_ALLOWLIST: &[(i32, &str)] = &[];

/// `items_event_sets.event_id` for a weapon's ranged auto attack (6 is
/// melee). Item 21's pair is (595, 6) and (559, 7).
const WEAPON_RANGED_EVENT: i32 = 7;

/// Templates an NPC combat ability can come from: spawned hostile (faction
/// 10, or any spawn row carrying an aggression override), or carrying an
/// ability set at all.
const SCOPE_SQL: &str = "\
    SELECT t.template_id, t.template_name \
      FROM resources.entity_templates t \
     WHERE t.ability_set_id IS NOT NULL \
        OR EXISTS (SELECT 1 FROM resources.spawnlist s \
                    WHERE s.template_id = t.template_id \
                      AND (t.faction = 10 OR s.aggression_override IS NOT NULL)) \
     ORDER BY t.template_id";

/// §26 test 18. Every effective ability of every in-scope template has a
/// non-NULL event set with exactly one Ability_End sequence, and the runtime
/// `sequence_map` built by the production loader resolves it.
///
/// Fails when a seed change gives a spawned hostile an ability with no
/// event set (`handle.rs` would skip the whole `onSequence`), or an event set
/// with zero or two Ability_End rows (the loader's last-write-wins insert
/// would pick one arbitrarily).
#[tokio::test]
async fn every_npc_combat_ability_resolves_one_ability_end_sequence() {
    let pool = require_db_or_skip!();

    let scope: Vec<(i32, String)> = sqlx::query_as(SCOPE_SQL)
        .fetch_all(&pool)
        .await
        .expect("scope query");
    assert!(
        scope.len() >= 20,
        "control: the scope query must see the seeded combat templates, got {}",
        scope.len()
    );
    let templates = load_spawn_templates(&pool).await.expect("templates load");
    let defs = load_ability_defs(&pool).await.expect("ability defs load");
    let sequence_map = load_event_set_sequences(&pool)
        .await
        .expect("sequence map loads");

    // ability → the templates that fire it, for the failure message.
    let mut fired_by: BTreeMap<i32, Vec<i32>> = BTreeMap::new();
    for (template_id, name) in &scope {
        let proto = templates
            .get(template_id)
            .unwrap_or_else(|| panic!("template {template_id} ({name}) must load"));
        let effective = if proto.ability_ids.is_empty() {
            vec![NPC_DEFAULT_ABILITY]
        } else {
            proto.ability_ids.clone()
        };
        for ability_id in effective {
            fired_by.entry(ability_id).or_default().push(*template_id);
        }
    }

    let mut failures = Vec::new();
    for (ability_id, templates) in &fired_by {
        if ANIMATION_ALLOWLIST.iter().any(|(id, _)| id == ability_id) {
            continue;
        }
        let Some(def) = defs.get(ability_id) else {
            failures.push(format!(
                "{ability_id}: no resources.abilities row ({templates:?})"
            ));
            continue;
        };
        let Some(es) = def.event_set_id else {
            failures.push(format!(
                "{ability_id} {}: NULL event_set_id ({templates:?})",
                def.name
            ));
            continue;
        };
        let end_rows: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM resources.event_sets_sequences ess \
               JOIN resources.sequences s ON s.sequence_id = ess.sequence_id \
              WHERE ess.event_set_id = $1 AND s.event_id = $2",
        )
        .bind(es)
        .bind(EVENT_ABILITY_END)
        .fetch_one(&pool)
        .await
        .expect("Ability_End count");
        if end_rows != 1 {
            failures.push(format!(
                "{ability_id} {}: event set {es} has {end_rows} Ability_End rows, want 1 \
                 ({templates:?})",
                def.name
            ));
        } else if !sequence_map.contains_key(&(es, EVENT_ABILITY_END)) {
            failures.push(format!(
                "{ability_id} {}: event set {es} Ability_End missing from the runtime map",
                def.name
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "NPC combat abilities that would deal damage with no attack animation:\n{}",
        failures.join("\n")
    );
}

/// §26 test 19. 559 Automatic Weapon Auto Attack, the NID Guards' SMG
/// burst, resolves event set 15 → Ability_End sequence 15
/// (`KIS-abilities_human.KIS-SA_Burst_Source`) through the production loaders.
#[tokio::test]
async fn automatic_weapon_auto_attack_resolves_the_burst_ability_end() {
    let pool = require_db_or_skip!();

    let defs = load_ability_defs(&pool).await.expect("ability defs load");
    let sequence_map = load_event_set_sequences(&pool)
        .await
        .expect("sequence map loads");

    let def = defs.get(&559).expect("ability 559 is seeded");
    assert_eq!(def.event_set_id, Some(15), "559's event set");
    assert_eq!(
        sequence_map.get(&(15, EVENT_ABILITY_END)),
        Some(&15),
        "event set 15's Ability_End is sequence 15"
    );
    let script: String = sqlx::query_scalar(
        "SELECT kismet_script_name FROM resources.sequences WHERE sequence_id = 15",
    )
    .fetch_one(&pool)
    .await
    .expect("sequence 15 is seeded");
    assert_eq!(script, "KIS-abilities_human.KIS-SA_Burst_Source");
}

/// §14 weapon/ability mismatch. A hostile spawned template that names a
/// `weapon_item_id` carries that item's ranged auto attack in its effective
/// ability set. Without it the NPC fires the Pistol Shot fallback (592) and
/// plays the pistol animation holding, say, an SMG.
///
/// Fails on the pre-NA43 seed: 148, 169 and 170 hold item 21 (SMG, ranged
/// 559) with no ability set, so their effective set is `[592]`.
#[tokio::test]
async fn a_hostile_template_fires_its_weapons_ranged_auto_attack() {
    let pool = require_db_or_skip!();

    let armed: Vec<(i32, String, i32)> = sqlx::query_as(
        "SELECT t.template_id, t.template_name, t.weapon_item_id \
           FROM resources.entity_templates t \
          WHERE t.weapon_item_id IS NOT NULL \
            AND t.faction = 10 \
            AND EXISTS (SELECT 1 FROM resources.spawnlist s WHERE s.template_id = t.template_id) \
          ORDER BY t.template_id",
    )
    .fetch_all(&pool)
    .await
    .expect("armed-template query");
    let armed_ids: BTreeSet<i32> = armed.iter().map(|(id, _, _)| *id).collect();
    assert!(
        armed_ids.is_superset(&BTreeSet::from([15, 24, 148, 169, 170])),
        "control: the seeded armed hostiles must be in scope, got {armed_ids:?}"
    );
    let templates = load_spawn_templates(&pool).await.expect("templates load");

    let mut failures = Vec::new();
    for (template_id, name, item_id) in &armed {
        let ranged: Vec<i32> = sqlx::query_scalar(
            "SELECT ability_id FROM resources.items_event_sets \
              WHERE item_id = $1 AND event_id = $2 ORDER BY ability_id",
        )
        .bind(item_id)
        .bind(WEAPON_RANGED_EVENT)
        .fetch_all(&pool)
        .await
        .expect("items_event_sets query");
        if ranged.is_empty() {
            // A melee-only or cosmetic item binds no ranged attack; nothing
            // to match.
            continue;
        }
        let proto = &templates[template_id];
        let effective = if proto.ability_ids.is_empty() {
            vec![NPC_DEFAULT_ABILITY]
        } else {
            proto.ability_ids.clone()
        };
        if !ranged.iter().any(|a| effective.contains(a)) {
            failures.push(format!(
                "template {template_id} ({name}) holds item {item_id} (ranged {ranged:?}) \
                 but fires {effective:?}"
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "hostile NPCs whose attack does not match the weapon they hold:\n{}",
        failures.join("\n")
    );
}
