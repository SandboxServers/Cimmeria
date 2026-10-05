//! Weapon-granted ability resolution: the `items_event_sets` fallback
//! for abilities not in the known set. The archetype-default redirect
//! (592 → the active weapon's RANGED binding) that used to be tested here
//! was removed in CS-07; `weapon_requirement.rs` guards its absence.

use super::*;

/// **Regression guard: weapon-granted abilities fire even when the ability
/// is not in the player's `entity.abilities` known set.**
///
/// Bug shape: the per-weapon ability resolver (`items_event_sets` lookup
/// at the right-click site) is wired, but the `use_ability` gate
/// historically only checked `entity.abilities.has_ability`. Weapon-
/// granted IDs aren't injected into the known set on equip, so the
/// gate alone rejects every weapon fire — fire button effectively
/// dead for any player holding a weapon whose grants only live in
/// `items_event_sets`.
///
/// Reverting the `is_ability_granted_by_active_weapon` fallback in
/// `use_ability/mod.rs` must fail this test.
#[tokio::test]
async fn weapon_granted_ability_commits_even_when_not_in_known_set() {
    use cimmeria_entity::cell_entity::BandolierItem;
    let mut mgr = make_mgr();
    make_player(&mut mgr, 1, [0.0; 3]);
    // Target NPC within range. Hostile so it passes the #444
    // target-validity gate.
    mgr.create_entity(2, "Castle_CellBlock", [3.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    if let Some(t) = mgr.get_entity_mut(2) {
        t.faction = crate::cell::combat::HOSTILE_FACTION;
    }

    if let Some(p) = mgr.get_entity_mut(1) {
        // Pistol (item 55) in active bandolier slot 0. Weapon drawn so
        // the holster queue doesn't intercept and route to the deferred
        // path — this test is about the known-set gate.
        p.weapon_holstered = false;
        p.active_bandolier_slot = 0;
        p.bandolier_items.insert(
            0,
            BandolierItem {
                instance_id: 0,
                item_id: 55,
                clip_size: 30,
                default_ammo_type: 2,
                current_ammo: 30,
                cur_ammo_type: 2,
            },
        );
        // Critically: do NOT call `p.abilities.add_ability(579)` — the
        // whole point is that weapon abilities don't live in the known
        // set. The fallback must consult items_event_sets instead.
        assert!(
            !p.abilities.has_ability(579),
            "test fixture: ability 579 MUST NOT be in entity.abilities — \
             this regression guard's whole shape depends on it"
        );
    }

    // Wire the items_event_sets binding: pistol RANGED (event 7) → 579.
    // Matches the production seed row at
    // `db/resources/Items/Seed/items_event_sets.sql`.
    mgr.item_event_set_abilities.insert((55, 7), 579);

    // Ability def for 579 with a reasonable range. required_ammo=1 so
    // we exercise the ammo path; cooldown 0.5s.
    mgr.ability_defs.insert(579, make_ability(579, 1, 30));

    let (tx, _rx) = mpsc::channel(64);
    let committed = handle_use_ability(1, 579, 2, &tx, &mut mgr).await;
    assert!(
        committed,
        "fire MUST commit when the ability is granted by the active \
         weapon via items_event_sets, even though it's not in \
         entity.abilities. Reverting the weapon-fallback in use_ability \
         must fail this assertion."
    );

    // The cooldown commit is the load-bearing side effect that proves
    // the fire actually executed (not just returned true).
    assert!(
        mgr.get_entity(1).unwrap().abilities.is_on_cooldown(579),
        "successful commit must start the cooldown for ability 579"
    );
}

/// Companion guard: when the active weapon does NOT grant the requested
/// ability via items_event_sets AND the player doesn't know it, the
/// gate must still reject. This pins that the fallback isn't an
/// "accept anything" hole — it only accepts abilities actually bound
/// to the equipped weapon.
#[tokio::test]
async fn ungranted_ability_still_rejected_when_active_weapon_does_not_grant_it() {
    use cimmeria_entity::cell_entity::BandolierItem;
    let mut mgr = make_mgr();
    make_player(&mut mgr, 1, [0.0; 3]);

    if let Some(p) = mgr.get_entity_mut(1) {
        p.weapon_holstered = false;
        p.active_bandolier_slot = 0;
        p.bandolier_items.insert(
            0,
            BandolierItem {
                instance_id: 0,
                item_id: 55,
                clip_size: 30,
                default_ammo_type: 2,
                current_ammo: 30,
                cur_ammo_type: 2,
            },
        );
    }
    // Pistol binds 579 (ranged), not 999. Player attempts 999.
    mgr.item_event_set_abilities.insert((55, 7), 579);
    mgr.ability_defs.insert(999, make_ability(999, 0, 30));
    let (tx, _rx) = mpsc::channel(8);

    let committed = handle_use_ability(1, 999, 0, &tx, &mut mgr).await;
    assert!(
        !committed,
        "ability not in known set and not bound to active weapon must reject"
    );
}
