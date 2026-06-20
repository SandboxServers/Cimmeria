use super::*;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use crate::mercury::method_idx;
use cimmeria_entity::abilities::AbilityDef;
use tokio::sync::mpsc;

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

// ── #444 single-target faction / target-validity gate ─────────────────

/// **#444 friendly-fire guard.** A single-target ability against a
/// non-hostile NPC (vendor / quest giver / neutral, faction != 10) must
/// be rejected before the damage pipeline — no commit, no cooldown.
/// Pre-fix the path only checked alive + range, so a forged
/// `useAbility(weapon_id, vendor_eid)` killed vendors and quest NPCs.
#[tokio::test]
async fn non_hostile_npc_target_blocks_fire() {
    let mut mgr = make_mgr();
    make_player(&mut mgr, 1, [0.0; 3]);
    // Neutral NPC (default faction 0) within range — a vendor / quest giver.
    mgr.create_entity(2, "Castle_CellBlock", [3.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    if let Some(p) = mgr.get_entity_mut(1) {
        p.abilities.add_ability(7);
    }
    mgr.ability_defs.insert(7, make_ability(7, 0, 30));
    let (tx, _rx) = mpsc::channel(8);

    let committed = handle_use_ability(1, 7, 2, &tx, &mut mgr).await;
    assert!(
        !committed,
        "a single-target ability must not commit against a non-hostile NPC"
    );
    assert!(
        !mgr.get_entity(1).unwrap().abilities.is_on_cooldown(7),
        "rejecting a non-hostile target must not start the cooldown"
    );
}

/// **#444 PvP-forgery guard.** A forged single-target ability against
/// another PLAYER must be rejected — players are never legitimate
/// single-target ability targets in today's PvE-only design, and the
/// wire path doesn't otherwise stop `target_id = other_player_eid`.
#[tokio::test]
async fn player_target_blocks_fire() {
    let mut mgr = make_mgr();
    make_player(&mut mgr, 1, [0.0; 3]);
    make_player(&mut mgr, 2, [3.0, 0.0, 0.0]); // another player in range
    if let Some(p) = mgr.get_entity_mut(1) {
        p.abilities.add_ability(7);
    }
    mgr.ability_defs.insert(7, make_ability(7, 0, 30));
    let (tx, _rx) = mpsc::channel(8);

    let committed = handle_use_ability(1, 7, 2, &tx, &mut mgr).await;
    assert!(
        !committed,
        "a single-target ability must not commit against another player"
    );
    assert!(!mgr.get_entity(1).unwrap().abilities.is_on_cooldown(7));
}

/// **#444 regression guard (positive).** A hostile NPC target still
/// passes the gate and the ability commits — proves the new check
/// doesn't over-block legitimate combat. A revert that drops the gate
/// keeps this green, but the two negative tests above flip to failing,
/// which is the intended fail-shape.
#[tokio::test]
async fn hostile_npc_target_commits() {
    let mut mgr = make_mgr();
    make_player(&mut mgr, 1, [0.0; 3]);
    mgr.create_entity(2, "Castle_CellBlock", [3.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    if let Some(t) = mgr.get_entity_mut(2) {
        t.faction = crate::cell::combat::HOSTILE_FACTION;
    }
    if let Some(p) = mgr.get_entity_mut(1) {
        p.abilities.add_ability(7);
    }
    mgr.ability_defs.insert(7, make_ability(7, 0, 30));
    let (tx, _rx) = mpsc::channel(8);

    let committed = handle_use_ability(1, 7, 2, &tx, &mut mgr).await;
    assert!(
        committed,
        "a hostile NPC target must still commit — the gate must not over-block combat"
    );
    assert!(
        mgr.get_entity(1).unwrap().abilities.is_on_cooldown(7),
        "a committed fire against a hostile target starts the cooldown"
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

// ── Auto-cycle commit-time arm and clear paths ─────────────────────

/// First commit while `auto_cycle == true` arms the loop: stashes the
/// ability id, sets `BSF_AUTO_CYCLING`, and broadcasts
/// `onStateFieldUpdate` so the client highlights the gun-icon button.
///
/// Bug shape: a refactor that calls `arm_auto_cycle` but skips the
/// broadcast leaves the server thinking the loop is running while the
/// client never lights the button.
#[tokio::test]
async fn auto_cycle_first_commit_arms_loop_and_broadcasts_state_field() {
    use crate::cell::combat::BSF_AUTO_CYCLING;
    let mut mgr = make_mgr();
    make_player(&mut mgr, 1, [0.0; 3]);
    if let Some(p) = mgr.get_entity_mut(1) {
        p.abilities.add_ability(7);
        p.abilities.auto_cycle = true; // button pressed earlier
        p.weapon_holstered = false; // skip the unholster queue
    }
    mgr.ability_defs.insert(7, make_ability(7, 0, 30));
    let (tx, mut rx) = mpsc::channel(64);

    let committed = handle_use_ability(1, 7, 0, &tx, &mut mgr).await;
    assert!(committed);

    let e = mgr.get_entity(1).unwrap();
    assert_eq!(
        e.abilities.auto_cycle_ability_id,
        Some(7),
        "ability id must be stashed for the driver tick to re-fire",
    );
    assert_ne!(
        e.state_field & BSF_AUTO_CYCLING,
        0,
        "BSF_AUTO_CYCLING must be set so the client highlights the gun-icon button",
    );

    let msgs = drain(&mut rx);
    let saw_state_field_update = msgs.iter().any(|m| {
        matches!(
            m,
            CellToBaseMsg::EntityMethodCall {
                entity_id: 1,
                method_index,
                ..
            } if *method_index == method_idx::ON_STATE_FIELD_UPDATE
        )
    });
    assert!(
        saw_state_field_update,
        "first commit must emit onStateFieldUpdate so the client lights the button"
    );
}

/// AF_DEACTIVATE_AUTO_CYCLE-flagged abilities CANCEL the loop after
/// commit. The cast still goes through (cooldown started, ammo
/// consumed) — the flag just stops the re-fire chain so one-shot
/// abilities don't get repeated by the driver. Mirrors python
/// `AbilityManager` behavior for flag mask `0x400`.
///
/// Bug shape: a refactor that arms unconditionally after commit
/// makes one-shot specials (signature moves) auto-repeat forever.
#[tokio::test]
async fn af_deactivate_auto_cycle_clears_loop_on_commit() {
    use crate::cell::combat::BSF_AUTO_CYCLING;
    use cimmeria_entity::abilities::AF_DEACTIVATE_AUTO_CYCLE;
    let mut mgr = make_mgr();
    make_player(&mut mgr, 1, [0.0; 3]);
    if let Some(p) = mgr.get_entity_mut(1) {
        p.abilities.add_ability(7);
        // Pre-armed loop with the SAME ability stashed (so the
        // manual-override gate doesn't trip on entry — this test is
        // about the AF_DEACTIVATE-flag exit path, not the override
        // exit path).
        p.abilities.auto_cycle = true;
        p.abilities.auto_cycle_ability_id = Some(7);
        p.set_state_flag(BSF_AUTO_CYCLING);
        p.weapon_holstered = false;
    }
    let mut deactivating_ability = make_ability(7, 0, 30);
    deactivating_ability.flags = AF_DEACTIVATE_AUTO_CYCLE;
    mgr.ability_defs.insert(7, deactivating_ability);
    let (tx, _rx) = mpsc::channel(64);

    let committed = handle_use_ability(1, 7, 0, &tx, &mut mgr).await;
    assert!(committed, "the cast itself must still commit");

    let e = mgr.get_entity(1).unwrap();
    assert!(
        !e.abilities.auto_cycle,
        "AF_DEACTIVATE_AUTO_CYCLE must clear the auto_cycle flag",
    );
    assert!(e.abilities.auto_cycle_ability_id.is_none());
    assert_eq!(
        e.state_field & BSF_AUTO_CYCLING,
        0,
        "BSF_AUTO_CYCLING must be cleared so the client un-highlights the button",
    );
}

/// Manual fire of a different ability while auto-cycle is armed
/// cancels the loop on entry — the player's intent was to switch
/// abilities. The new manual cast still commits normally.
///
/// Mirrors python `AbilityManager.useAbility` (line 1019:
/// `self.autoCycle = False`). Tick-driven re-fires invoke with the
/// stashed ability id, so this gate never trips for loop shots —
/// only manual override clicks.
///
/// Bug shape: dropping the override gate turns every ability swap
/// into "fire the new ability AND keep looping the old one".
#[tokio::test]
async fn manual_fire_of_different_ability_cancels_auto_cycle() {
    use crate::cell::combat::BSF_AUTO_CYCLING;
    let mut mgr = make_mgr();
    make_player(&mut mgr, 1, [0.0; 3]);
    if let Some(p) = mgr.get_entity_mut(1) {
        p.abilities.add_ability(7);
        p.abilities.add_ability(8);
        p.abilities.auto_cycle = true;
        p.abilities.auto_cycle_ability_id = Some(7);
        p.set_state_flag(BSF_AUTO_CYCLING);
        p.weapon_holstered = false;
    }
    mgr.ability_defs.insert(7, make_ability(7, 0, 30));
    mgr.ability_defs.insert(8, make_ability(8, 0, 30));
    let (tx, _rx) = mpsc::channel(64);

    // Manual fire of ability 8 (different from stashed 7).
    let committed = handle_use_ability(1, 8, 0, &tx, &mut mgr).await;
    assert!(committed, "the new ability still fires");

    let e = mgr.get_entity(1).unwrap();
    assert!(
        !e.abilities.auto_cycle,
        "different-ability manual fire must clear the auto_cycle flag",
    );
    assert!(e.abilities.auto_cycle_ability_id.is_none());
    assert_eq!(
        e.state_field & BSF_AUTO_CYCLING,
        0,
        "BSF must clear so the client un-highlights the button",
    );
}

/// Same-ability manual fire does NOT cancel the loop. Right-clicking
/// the same weapon at a new enemy continues the loop; Phase 2's
/// `current_target_id` (live cursor target, written by `setTargetID`)
/// handles target redirect for subsequent tick-driven re-fires.
///
/// Bug shape: conflating "any manual fire" with "different-ability
/// fire" turns every right-click into a loop reset.
#[tokio::test]
async fn same_ability_manual_fire_does_not_break_loop() {
    use crate::cell::combat::BSF_AUTO_CYCLING;
    let mut mgr = make_mgr();
    make_player(&mut mgr, 1, [0.0; 3]);
    if let Some(p) = mgr.get_entity_mut(1) {
        p.abilities.add_ability(7);
        p.abilities.auto_cycle = true;
        p.abilities.auto_cycle_ability_id = Some(7);
        p.set_state_flag(BSF_AUTO_CYCLING);
        p.weapon_holstered = false;
    }
    mgr.ability_defs.insert(7, make_ability(7, 0, 30));
    let (tx, _rx) = mpsc::channel(64);

    let committed = handle_use_ability(1, 7, 0, &tx, &mut mgr).await;
    assert!(committed);

    let e = mgr.get_entity(1).unwrap();
    assert!(
        e.abilities.auto_cycle,
        "same-ability fire must NOT break the loop",
    );
    assert_eq!(
        e.abilities.auto_cycle_ability_id,
        Some(7),
        "ability id must be stable",
    );
    assert_ne!(
        e.state_field & BSF_AUTO_CYCLING,
        0,
        "BSF must remain set — the loop continues",
    );
}

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
