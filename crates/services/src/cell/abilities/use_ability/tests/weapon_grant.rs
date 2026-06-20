//! Weapon-granted ability resolution: the `items_event_sets` fallback
//! for abilities not in the known set, and the archetype-default weapon
//! redirect (592 → the active weapon's RANGED binding).

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

// ── Archetype-default weapon redirect (592 → weapon's RANGED binding) ──

/// **The P90 case.** Player has Pistol Shot (592) bound to an
/// action-bar slot from when their archetype was Soldier; that slot
/// stays bound to 592 forever client-side. Player equips a P90,
/// presses the slot → wire arrives as `useAbility(592)`. The redirect
/// must rewrite this to 559 (P90's items_event_sets RANGED binding)
/// before validation runs, so the SMG actually fires.
///
/// Bug shape pre-fix: 592 was in the known set (archetype starter),
/// validation passed, server fired Pistol Shot — wrong animation,
/// wrong damage profile, wrong ammo pool, with a P90 visibly equipped.
/// Reverting the redirect block trips this test on the cooldown
/// assertion (Pistol Shot's cooldown gets stamped instead of SMG's).
#[tokio::test]
async fn use_ability_redirects_pistol_shot_to_smg_when_p90_equipped() {
    use cimmeria_entity::cell_entity::BandolierItem;

    let mut mgr = make_mgr();
    make_player(&mut mgr, 1, [0.0; 3]);
    // NPC target so the post-redirect range check has something to find.
    mgr.spawn_npc(50, "Castle_CellBlock", [3.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    if let Some(t) = mgr.get_entity_mut(50) {
        t.faction = crate::cell::combat::HOSTILE_FACTION;
    }

    if let Some(p) = mgr.get_entity_mut(1) {
        // Pistol Shot (592) — archetype starter. The player only
        // knows this id; SMG Auto Attack (559) is NOT in the known
        // set (it's not auto-granted by PR #494 in this fixture).
        // The redirect must let the call land on 559 anyway.
        p.abilities.add_ability(592);
        // P90 in slot 0 — item id 21 per items_event_sets seed.
        p.bandolier_items.insert(
            0,
            BandolierItem {
                instance_id: 0,
                item_id: 21,
                clip_size: 30,
                default_ammo_type: 2,
                current_ammo: 30,
                cur_ammo_type: 2,
            },
        );
        p.active_bandolier_slot = 0;
    }
    // P90 (21) binds 559 to RANGED (event 7).
    mgr.item_event_set_abilities.insert((21, 7), 559);
    // ability_defs entries for both — the redirect re-looks-up the
    // def after rewriting ability_id, so 559's def must be in the
    // map for the post-redirect validation to find a max_range and
    // cooldown.
    mgr.ability_defs.insert(592, make_ability(592, 0, 30));
    mgr.ability_defs.insert(559, make_ability(559, 0, 30));

    let (tx, _rx) = mpsc::channel(64);
    let committed = handle_use_ability(1, 592, 50, &tx, &mut mgr).await;

    assert!(
        committed,
        "redirect must succeed end-to-end — the SMG fire commits via 559's path"
    );
    let entity = mgr.get_entity(1).unwrap();
    assert!(
        entity.abilities.is_on_cooldown(559),
        "the WEAPON ability (559 SMG Auto Attack) must be the one that started \
         cooldown — proves the redirect ran and the fire took place against 559",
    );
    assert!(
        !entity.abilities.is_on_cooldown(592),
        "Pistol Shot (592) must NOT have entered cooldown — the redirect \
         rewrites ability_id BEFORE the cooldown is stamped, so 592 stays \
         untouched even though the client sent it on the wire",
    );
}

/// Companion: when the player has NO weapon active (empty slot 0), the
/// redirect must NOT fire — 592 fires as itself. Pins the off-switch.
#[tokio::test]
async fn use_ability_does_not_redirect_pistol_shot_when_no_weapon_equipped() {
    let mut mgr = make_mgr();
    make_player(&mut mgr, 1, [0.0; 3]);
    mgr.spawn_npc(50, "Castle_CellBlock", [3.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    if let Some(t) = mgr.get_entity_mut(50) {
        t.faction = crate::cell::combat::HOSTILE_FACTION;
    }

    if let Some(p) = mgr.get_entity_mut(1) {
        p.abilities.add_ability(592);
        // Slot 0 left empty — no bandolier_items insert.
        p.active_bandolier_slot = 0;
    }
    // items_event_sets unchanged — would never be consulted in the
    // empty-slot case, but pin the negative-space assertion below
    // regardless.
    mgr.item_event_set_abilities.insert((21, 7), 559);
    mgr.ability_defs.insert(592, make_ability(592, 0, 30));
    mgr.ability_defs.insert(559, make_ability(559, 0, 30));

    let (tx, _rx) = mpsc::channel(64);
    let committed = handle_use_ability(1, 592, 50, &tx, &mut mgr).await;

    assert!(committed, "Pistol Shot must still fire bare-handed");
    let entity = mgr.get_entity(1).unwrap();
    assert!(
        entity.abilities.is_on_cooldown(592),
        "no weapon active → no redirect → 592 stamps its own cooldown"
    );
    assert!(
        !entity.abilities.is_on_cooldown(559),
        "redirect target (559) must not have been touched"
    );
}

/// Companion: when the player fires the weapon's binding (559)
/// DIRECTLY — e.g. they manually bound 559 to a slot — the redirect
/// must be a no-op. Pins that the redirect only triggers on the
/// archetype-default id, not on any in-bound weapon ability.
#[tokio::test]
async fn use_ability_does_not_redirect_when_called_ability_is_already_weapon_binding() {
    use cimmeria_entity::cell_entity::BandolierItem;

    let mut mgr = make_mgr();
    make_player(&mut mgr, 1, [0.0; 3]);
    mgr.spawn_npc(50, "Castle_CellBlock", [3.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    if let Some(t) = mgr.get_entity_mut(50) {
        t.faction = crate::cell::combat::HOSTILE_FACTION;
    }

    if let Some(p) = mgr.get_entity_mut(1) {
        p.abilities.add_ability(559);
        p.bandolier_items.insert(
            0,
            BandolierItem {
                instance_id: 0,
                item_id: 21,
                clip_size: 30,
                default_ammo_type: 2,
                current_ammo: 30,
                cur_ammo_type: 2,
            },
        );
        p.active_bandolier_slot = 0;
    }
    mgr.item_event_set_abilities.insert((21, 7), 559);
    mgr.ability_defs.insert(559, make_ability(559, 0, 30));

    let (tx, _rx) = mpsc::channel(64);
    let committed = handle_use_ability(1, 559, 50, &tx, &mut mgr).await;
    assert!(committed);
    // Single cooldown — no double-fire, no redirect loop.
    assert!(mgr.get_entity(1).unwrap().abilities.is_on_cooldown(559));
}
