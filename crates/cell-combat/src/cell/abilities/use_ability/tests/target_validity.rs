//! #444 single-target faction / target-validity gate, plus the
//! BSF_InCombat non-mutation guards for self / no-target casts.

use super::*;

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
