//! Resource / precondition gating: entity existence, ability ownership,
//! cooldown, range, reload-in-flight, ammo, dead-target, and the
//! self-cast commit path.

use super::*;

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
    // Target far away beyond max_range=10. Hostile so it passes the
    // #444 target-validity gate and reaches the range check this test
    // is about.
    mgr.create_entity(2, "Castle_CellBlock", [100.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    if let Some(t) = mgr.get_entity_mut(2) {
        t.faction = crate::cell::combat::HOSTILE_FACTION;
    }
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
        p.reload_complete_at = Some(std::time::Instant::now() - std::time::Duration::from_secs(1));
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
                instance_id: 0,
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
