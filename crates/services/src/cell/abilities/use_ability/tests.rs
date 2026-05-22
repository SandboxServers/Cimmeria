use super::*;
use crate::cell::space_manager::SpaceManager;
use crate::mercury::method_idx;
use cimmeria_entity::abilities::AbilityDef;

fn make_ability(id: i32, required_ammo: i32, max_range: i32) -> AbilityDef {
    AbilityDef {
        ability_id: id,
        name: "test".to_string(),
        cooldown: 0.5,
        warmup: 0.0,
        flags: 0,
        is_ranged: false,
        min_range: 0,
        max_range,
        target_type_id: 0,
        effect_ids: vec![],
        moniker_ids: vec![],
        required_ammo,
        event_set_id: None,
        velocity: 0.0,
    }
}

fn make_mgr() -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" Instanced="true" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(r#"<?xml version="1.0"?><Spaces></Spaces>"#)
        .unwrap();
    mgr
}

fn make_player(mgr: &mut SpaceManager, id: u32, pos: [f32; 3]) {
    mgr.create_entity(id, "Castle_CellBlock", pos, [0.0; 3])
        .unwrap();
    if let Some(p) = mgr.get_entity_mut(id) {
        p.is_player = true;
        p.player_id = Some(100 + id as i32);
    }
}

fn drain(rx: &mut mpsc::Receiver<CellToBaseMsg>) -> Vec<CellToBaseMsg> {
    let mut out = Vec::new();
    while let Ok(m) = rx.try_recv() {
        out.push(m);
    }
    out
}

#[tokio::test]
async fn missing_entity_returns_false_and_emits_no_packets() {
    let mut mgr = make_mgr();
    let (tx, mut rx) = mpsc::channel(8);
    let committed = handle_use_ability(999, 1, 0, &tx, &mut mgr).await;
    assert!(!committed);
    assert!(drain(&mut rx).is_empty());
}

#[tokio::test]
async fn entity_without_ability_returns_false() {
    let mut mgr = make_mgr();
    make_player(&mut mgr, 1, [0.0; 3]);
    // No ability added to the entity.
    let (tx, mut rx) = mpsc::channel(8);
    let committed = handle_use_ability(1, 7, 0, &tx, &mut mgr).await;
    assert!(!committed);
    assert!(drain(&mut rx).is_empty());
}

#[tokio::test]
async fn cooldown_blocks_fire_and_emits_no_packets() {
    let mut mgr = make_mgr();
    make_player(&mut mgr, 1, [0.0; 3]);
    if let Some(p) = mgr.get_entity_mut(1) {
        p.abilities.add_ability(7);
        p.abilities
            .start_ability_cooldown(7, std::time::Duration::from_secs(60));
    }
    mgr.ability_defs.insert(7, make_ability(7, 0, 30));
    let (tx, mut rx) = mpsc::channel(8);

    let committed = handle_use_ability(1, 7, 0, &tx, &mut mgr).await;
    assert!(!committed);
    assert!(
        drain(&mut rx).is_empty(),
        "cooldown rejection must not emit any wire packets"
    );
}

/// Out-of-range hits the dedicated error-code branch — emits exactly
/// one onErrorCode (method 121) carrying CONDITION_FEEDBACK_OutsideWeaponRange=42.
/// Pin the byte layout (SystemID:u8 + InstanceID:i32 + ErrorCodeID:u16).
#[tokio::test]
async fn out_of_range_emits_on_error_code_with_condition_42() {
    let mut mgr = make_mgr();
    make_player(&mut mgr, 1, [0.0; 3]);
    // Target far away beyond max_range=10.
    mgr.create_entity(2, "Castle_CellBlock", [100.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    if let Some(p) = mgr.get_entity_mut(1) {
        p.abilities.add_ability(7);
    }
    mgr.ability_defs.insert(7, make_ability(7, 0, 10));
    let (tx, mut rx) = mpsc::channel(8);

    let committed = handle_use_ability(1, 7, 2, &tx, &mut mgr).await;
    assert!(!committed);

    let msgs = drain(&mut rx);
    let err = msgs
        .iter()
        .find_map(|m| match m {
            CellToBaseMsg::EntityMethodCall {
                entity_id: 1,
                method_index,
                args,
            } if *method_index == method_idx::ON_ERROR_CODE => Some(args.clone()),
            _ => None,
        })
        .expect("out-of-range must emit onErrorCode");
    // Layout: u8 SystemID + i32 InstanceID + u16 ErrorCodeID = 7 bytes.
    assert_eq!(err.len(), 7);
    assert_eq!(err[0], 0, "SystemID should be ERRORCODE_SYSTEM_Ability=0");
    assert_eq!(
        i32::from_le_bytes([err[1], err[2], err[3], err[4]]),
        7,
        "InstanceID should echo the ability_id"
    );
    assert_eq!(
        u16::from_le_bytes([err[5], err[6]]),
        42,
        "ErrorCodeID should be CONDITION_FEEDBACK_OutsideWeaponRange=42"
    );
}

/// Reload-in-flight blocks fire even when the deadline elapsed by clock.
/// Regression guard: the gate is `is_some()`, not `now < deadline`.
/// Setting `reload_complete_at` to a past instant must still block,
/// because only the 100ms reload-completion tick clears it.
#[tokio::test]
async fn reload_in_flight_blocks_fire_even_with_past_deadline() {
    let mut mgr = make_mgr();
    make_player(&mut mgr, 1, [0.0; 3]);
    if let Some(p) = mgr.get_entity_mut(1) {
        // Weapon drawn so the attack-while-holstered queue doesn't
        // intercept — this test is about the reload-in-flight gate.
        p.weapon_holstered = false;
        p.abilities.add_ability(7);
        p.reload_complete_at =
            Some(std::time::Instant::now() - std::time::Duration::from_secs(1));
    }
    mgr.ability_defs.insert(7, make_ability(7, 1, 30));
    let (tx, mut rx) = mpsc::channel(8);

    let committed = handle_use_ability(1, 7, 0, &tx, &mut mgr).await;
    assert!(
        !committed,
        "fire must be blocked while reload_complete_at is_some(), regardless of wall-clock"
    );
    assert!(drain(&mut rx).is_empty());
}

#[tokio::test]
async fn no_ammo_for_player_blocks_fire() {
    use cimmeria_entity::cell_entity::BandolierItem;
    let mut mgr = make_mgr();
    make_player(&mut mgr, 1, [0.0; 3]);
    if let Some(p) = mgr.get_entity_mut(1) {
        // Weapon drawn so the attack-while-holstered queue doesn't
        // intercept — this test is about the no-ammo gate.
        p.weapon_holstered = false;
        p.abilities.add_ability(7);
        // Active slot 0, ammo 0 of 30.
        p.bandolier_items.insert(
            0,
            BandolierItem {
                item_id: 1,
                clip_size: 30,
                default_ammo_type: 2,
                current_ammo: 0,
                cur_ammo_type: 2,
            },
        );
    }
    mgr.ability_defs.insert(7, make_ability(7, 1, 30));
    let (tx, mut rx) = mpsc::channel(8);

    let committed = handle_use_ability(1, 7, 0, &tx, &mut mgr).await;
    assert!(!committed);
    assert!(drain(&mut rx).is_empty());
}

/// Cast against a dead target rejects without consuming ammo or starting
/// the cooldown. Regression guard for the dead-target branch.
#[tokio::test]
async fn dead_target_blocks_fire_without_consuming_resources() {
    let mut mgr = make_mgr();
    make_player(&mut mgr, 1, [0.0; 3]);
    mgr.create_entity(2, "Castle_CellBlock", [0.0; 3], [0.0; 3])
        .unwrap();
    if let Some(p) = mgr.get_entity_mut(1) {
        p.abilities.add_ability(7);
    }
    if let Some(t) = mgr.get_entity_mut(2) {
        t.set_state_flag(crate::cell::combat::BSF_DEAD);
    }
    mgr.ability_defs.insert(7, make_ability(7, 0, 30));
    let (tx, _rx) = mpsc::channel(8);

    let committed = handle_use_ability(1, 7, 2, &tx, &mut mgr).await;
    assert!(!committed);
    // Cooldown must not have been started — a follow-up cast with a
    // live target should be allowed.
    assert!(
        !mgr.get_entity(1).unwrap().abilities.is_on_cooldown(7),
        "rejecting against a dead target must not start the cooldown"
    );
}

/// `use_ability` does not touch BSF_InCombat (bit 3). The bit is
/// derived from `threatened_mobs` and flips on via
/// `combat::generate_threat` → `enter_player_combat` from
/// `damage_apply::apply_damage_to_target` when this attack actually
/// hits a surviving NPC. A self-cast (target_id == 0) commits
/// cooldown + ammo but produces no threat, so the bit stays
/// unchanged — pinned here so a regression that re-introduces a raw
/// `state_field |= BSF_IN_COMBAT` setter on this path doesn't slip
/// through (stuck-bit hazard for target-less casts).
#[tokio::test]
async fn commit_leaves_bsf_in_combat_alone_on_self_cast() {
    let mut mgr = make_mgr();
    make_player(&mut mgr, 1, [0.0; 3]);
    if let Some(p) = mgr.get_entity_mut(1) {
        p.abilities.add_ability(7);
    }
    mgr.ability_defs.insert(7, make_ability(7, 0, 30));
    let (tx, _rx) = mpsc::channel(64);

    let committed = handle_use_ability(1, 7, 0, &tx, &mut mgr).await;
    assert!(committed);

    let s = mgr.get_entity(1).unwrap().state_field;
    assert_eq!(
        s & (1 << 3),
        0,
        "BSF_InCombat must NOT be set by use_ability — \
         it's now derived from threatened_mobs via enter_player_combat"
    );
}

/// Target-less / no-target cast (target_id == 0) must not set
/// BSF_InCombat on the attacker. Stuck-bit regression guard: the
/// previous raw `state_field |= BSF_IN_COMBAT` here ran before the
/// `if target_id <= 0` early-return downstream, so a self-cast
/// would flip the in-combat HUD forever (no NPC death ever runs
/// the clear path).
#[tokio::test]
async fn no_target_cast_does_not_set_bsf_in_combat() {
    let mut mgr = make_mgr();
    make_player(&mut mgr, 1, [0.0; 3]);
    if let Some(p) = mgr.get_entity_mut(1) {
        p.abilities.add_ability(7);
    }
    mgr.ability_defs.insert(7, make_ability(7, 0, 30));
    let (tx, _rx) = mpsc::channel(64);

    let committed = handle_use_ability(1, 7, 0, &tx, &mut mgr).await;
    assert!(committed, "self-cast still commits (cooldown + ammo)");

    let s = mgr.get_entity(1).unwrap().state_field;
    assert_eq!(
        s & (1 << 3),
        0,
        "no-target cast must not strand BSF_InCombat — no NPC death \
         would ever run the clear path"
    );
    // threatened_mobs must also stay empty so the regen tick (which
    // gates on the set) is free to fire.
    assert!(
        mgr.get_entity(1).unwrap().threatened_mobs.is_empty(),
        "no-target cast must leave threatened_mobs empty"
    );
}

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
    let (tx, _rx) = mpsc::channel(64);

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

/// Self-target (target_id == 0) commits cooldown and ammo consume but
/// skips combat resolution — the function returns true. Pin so a
/// regression that re-routes self-cast through damage_apply (and would
/// then bail on missing target) doesn't go silently.
#[tokio::test]
async fn self_target_commits_returns_true() {
    let mut mgr = make_mgr();
    make_player(&mut mgr, 1, [0.0; 3]);
    if let Some(p) = mgr.get_entity_mut(1) {
        p.abilities.add_ability(7);
    }
    mgr.ability_defs.insert(7, make_ability(7, 0, 30));
    let (tx, _rx) = mpsc::channel(64);

    let committed = handle_use_ability(1, 7, 0, &tx, &mut mgr).await;
    assert!(committed);
    assert!(mgr.get_entity(1).unwrap().abilities.is_on_cooldown(7));
}
