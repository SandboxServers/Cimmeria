//! Auto-cycle commit-time arm and clear paths: arming the loop on first
//! commit, the AF_DEACTIVATE_AUTO_CYCLE one-shot exit, and the
//! manual-override (different-ability) vs same-ability continuation.

use super::*;

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
