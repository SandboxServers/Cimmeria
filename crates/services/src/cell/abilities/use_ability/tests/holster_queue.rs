//! Attack-while-holstered unholster queue: deferring the first weapon
//! attack behind the draw animation, rejecting spam during the draw
//! window, and the non-weapon-ability bypasses.

use super::*;

/// Attack-while-holstered: pressing fire on a weapon
/// attack while OOC + holstered must defer the ability — draw
/// weapon, fire `Item_Equip`, stash the call on
/// `pending_attack_*`, and return false WITHOUT committing
/// cooldown or consuming ammo. The `pending_attack_tick`
/// re-invokes after `UNHOLSTER_DRAW_DURATION` to fire for real.
///
/// Bug shape this catches: a refactor removes the queue and the
/// first attack on a holstered weapon fires with no animation
/// (the playtest symptom that drove this fix).
#[tokio::test]
async fn attack_while_holstered_queues_and_draws_without_committing() {
    use cimmeria_entity::cell_entity::BandolierItem;
    let mut mgr = make_mgr();
    make_player(&mut mgr, 1, [0.0; 3]);
    if let Some(p) = mgr.get_entity_mut(1) {
        p.archetype_id = Some(1);
        p.weapon_visual = Some("WP-Human.WP_Pistol_1A".into());
        p.weapon_holstered = true; // OOC + holstered
        p.abilities.add_ability(7);
        p.bandolier_items.insert(
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
    }
    // required_ammo=1 → triggers the weapon-attack queue.
    mgr.ability_defs.insert(7, make_ability(7, 1, 30));
    let (tx, mut rx) = mpsc::channel(64);

    let committed = handle_use_ability(1, 7, 0, &tx, &mut mgr).await;
    assert!(
        !committed,
        "attack-while-holstered must NOT commit on the first press — \
         the queue defers the ability until the draw animation finishes",
    );

    let e = mgr.get_entity(1).unwrap();
    assert!(!e.weapon_holstered, "Phase A draws the weapon");
    assert!(
        e.combat_exit_at.is_some(),
        "OOC re-holster timer must arm so the weapon goes away post-fight",
    );
    assert!(
        e.pending_attack_at.is_some(),
        "pending_attack_at must stamp so pending_attack_tick can fire the queued ability",
    );
    assert_eq!(
        e.pending_attack_ability_id,
        Some(7),
        "ability_id must be stashed for Phase B dispatch",
    );
    assert!(
        !e.abilities.is_on_cooldown(7),
        "Phase A must NOT start the cooldown — cooldown commits in Phase B",
    );
    assert_eq!(
        e.bandolier_items[&0].current_ammo, 30,
        "Phase A must NOT consume ammo — ammo check happens in Phase B",
    );

    // Pin the runtime rebroadcast: Phase A must dispatch exactly one
    // `RefreshAppearance(holstered=false)` so the base-side handler
    // unwraps the weapon mesh in the `ComponentList` and rebroadcasts
    // `BeingAppearance` to self + AoI witnesses. Without this assertion
    // the test would pass even if a refactor dropped the
    // `request_appearance_refresh` call — the bug shape this guards
    // against (server state mutates correctly but other players still
    // see the player in the holstered pose).
    let refreshes: Vec<_> = drain(&mut rx)
        .into_iter()
        .filter_map(|m| match m {
            CellToBaseMsg::RefreshAppearance {
                entity_id,
                holstered,
                ..
            } => Some((entity_id, holstered)),
            _ => None,
        })
        .collect();
    assert_eq!(
        refreshes.len(),
        1,
        "Phase A must dispatch exactly one RefreshAppearance — \
         dropping it leaves AoI witnesses stuck on the holstered pose \
         while the attacker animates an invisible draw",
    );
    assert_eq!(
        refreshes[0],
        (1, false),
        "RefreshAppearance must target the attacker with holstered=false",
    );
}

/// Attack inputs DURING the draw window are rejected so the first
/// press locks in the queue. Spamming clicks must not change the
/// queued ability/target or restart the draw timer.
#[tokio::test]
async fn attack_while_queued_is_rejected_input() {
    use cimmeria_entity::cell_entity::BandolierItem;
    let mut mgr = make_mgr();
    make_player(&mut mgr, 1, [0.0; 3]);
    let queued_stamp = std::time::Instant::now();
    if let Some(p) = mgr.get_entity_mut(1) {
        p.archetype_id = Some(1);
        p.weapon_holstered = true;
        p.abilities.add_ability(7);
        p.bandolier_items.insert(
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
        // Already queued from a previous press.
        p.pending_attack_at = Some(queued_stamp);
        p.pending_attack_ability_id = Some(99);
        p.pending_attack_target_id = Some(42);
    }
    mgr.ability_defs.insert(7, make_ability(7, 1, 30));
    let (tx, _rx) = mpsc::channel(64);

    let committed = handle_use_ability(1, 7, 0, &tx, &mut mgr).await;
    assert!(
        !committed,
        "second press during draw window must be rejected"
    );

    let e = mgr.get_entity(1).unwrap();
    assert_eq!(
        e.pending_attack_ability_id,
        Some(99),
        "the existing queued ability must NOT be overwritten by the second press",
    );
    assert_eq!(
        e.pending_attack_target_id,
        Some(42),
        "the existing queued target must NOT be overwritten",
    );
    assert_eq!(
        e.pending_attack_at,
        Some(queued_stamp),
        "the draw timer must NOT be restarted by spamming clicks",
    );
}

/// Non-weapon abilities (required_ammo == 0 — heals, buffs,
/// self-casts) must NOT trigger the unholster queue. Pin the gate
/// so a refactor that drops the `required_ammo > 0` check doesn't
/// turn every self-cast on a holstered player into a 1s-delayed
/// queued cast.
#[tokio::test]
async fn non_weapon_ability_skips_unholster_queue_when_holstered() {
    let mut mgr = make_mgr();
    make_player(&mut mgr, 1, [0.0; 3]);
    if let Some(p) = mgr.get_entity_mut(1) {
        p.weapon_holstered = true;
        p.abilities.add_ability(7);
    }
    // required_ammo=0 → non-weapon ability (heal, buff, self-cast).
    mgr.ability_defs.insert(7, make_ability(7, 0, 30));
    let (tx, _rx) = mpsc::channel(64);

    let committed = handle_use_ability(1, 7, 0, &tx, &mut mgr).await;
    assert!(
        committed,
        "non-weapon ability on a holstered player must fire immediately, \
         not queue — the weapon isn't being used",
    );
    assert!(
        mgr.get_entity(1).unwrap().pending_attack_at.is_none(),
        "non-weapon ability must NOT set pending_attack_at",
    );
}

/// Non-weapon abilities (heals, buffs, self-casts) must STILL fire
/// even when a weapon attack is queued behind the unholster
/// animation. The queue is about the unholster choreography, not
/// a global ability lockout — a player mid-draw should still be
/// able to heal themselves.
///
/// Bug shape: a refactor that gates the `queued_attack_already_pending`
/// early reject without also checking `is_weapon_attack` regresses
/// to "queue blocks ALL abilities" — heals get silently dropped
/// the moment a weapon attack is queued.
#[tokio::test]
async fn non_weapon_ability_fires_even_when_weapon_attack_queued() {
    let mut mgr = make_mgr();
    make_player(&mut mgr, 1, [0.0; 3]);
    let queued_stamp = std::time::Instant::now() + std::time::Duration::from_secs(1);
    if let Some(p) = mgr.get_entity_mut(1) {
        p.weapon_holstered = false; // weapon drawn (just queued an attack)
        p.abilities.add_ability(7);
        // Simulate a queued weapon attack from a prior press.
        p.pending_attack_at = Some(queued_stamp);
        p.pending_attack_ability_id = Some(99);
        p.pending_attack_target_id = Some(42);
    }
    // Ability 7 is non-weapon (required_ammo=0 — heal/buff).
    mgr.ability_defs.insert(7, make_ability(7, 0, 30));
    let (tx, _rx) = mpsc::channel(64);

    let committed = handle_use_ability(1, 7, 0, &tx, &mut mgr).await;
    assert!(
        committed,
        "non-weapon ability must commit even while a weapon attack \
         is queued — the queue is animation-state, not a global \
         ability lockout",
    );

    let e = mgr.get_entity(1).unwrap();
    assert_eq!(
        e.pending_attack_at,
        Some(queued_stamp),
        "queued weapon attack must NOT be cleared by a non-weapon \
         ability firing through the queue",
    );
    assert_eq!(
        e.pending_attack_ability_id,
        Some(99),
        "queued ability id must be untouched",
    );
}
