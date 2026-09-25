//! `.summon <name>` — bring a named player to the caller.
//!
//! Legacy `deprecated/python/cell/commands/Player.py:321-341` is `.goto` with
//! the roles swapped, anchored on `entity = target or player`. The anchor rule
//! is the one deliberate departure (owner decision 2026-09-20): the caller is
//! always the anchor and a selection is ignored — see `summon`'s doc comment.
//! Everything else, wording included, is still legacy's.

use super::*;

/// Same failure wording as `.goto` — legacy `Player.py:330-332`.
#[tokio::test]
async fn legacy_p46_summon_unknown_name_reports_not_available() {
    let (mut mgr, gm, _npc) = setup_worlds();

    let t = run("summon", gm, &["Teal'c"], None, &mut mgr).await;

    assert!(
        t.has_line(NOT_AVAILABLE),
        "unknown name must report legacy's exact wording; got {:?}",
        t.feedback
    );
    assert_no_move(&t);
}

/// The named player is pulled into the **caller's exact instance**, not the
/// default instance of the caller's world — the mirror of `.goto`'s
/// instance-exactness criterion, and the case a GM running an instanced
/// dungeon actually hits.
#[tokio::test]
async fn p46_summon_pulls_the_player_into_the_callers_exact_instance() {
    let (mut mgr, _gm, _npc) = setup_worlds();
    let instance_a = spawn_named_player(&mut mgr, 50, INSTANCED, [11.0, 0.0, 12.0], "Ana");
    let instance_b = spawn_named_player(&mut mgr, 51, INSTANCED, [30.0, 0.0, 40.0], "Bob");
    assert_ne!(instance_a, instance_b);

    // Caller = Ana, in instance A. Subject = Bob.
    let t = run("summon", 50, &["Bob"], None, &mut mgr).await;

    assert_eq!(
        t.only_gate_travel(),
        &(
            51,
            INSTANCED.to_string(),
            Some(instance_a),
            [11.0, 0.0, 12.0]
        ),
        "Bob must be transferred into the caller's exact instance, at her position"
    );
    assert_eq!(
        mgr.get_entity_space_id(51),
        None,
        "the summoned player must be torn out of their origin instance"
    );
    assert_eq!(
        mgr.get_entity_space_id(50),
        Some(instance_a),
        "the caller must not move"
    );
    assert!(
        t.has_line("Summoning player <Bob>"),
        "legacy's exact success wording; got {:?}",
        t.feedback
    );
}

/// The caller is the anchor, and the caller itself is never the entity that
/// moves.
#[tokio::test]
async fn p46_summon_brings_the_player_to_the_caller() {
    let (mut mgr, gm, _npc) = setup_worlds();
    let caller_pos = position_of(&mgr, gm);
    let caller_space = mgr.get_entity_space_id(gm).unwrap();
    spawn_named_player(&mut mgr, 50, CASTLE, [30.0, 0.0, 30.0], "Bob");

    let t = run("summon", gm, &["Bob"], None, &mut mgr).await;

    assert_eq!(
        t.only_gate_travel(),
        &(50, AGNOS.to_string(), Some(caller_space), caller_pos),
        "Bob must be transferred to the caller's own space and position"
    );
    assert_eq!(
        mgr.get_entity_space_id(gm),
        Some(caller_space),
        "the caller must stay put — .summon moves the named player, not the GM"
    );
    assert_eq!(position_of(&mgr, gm), caller_pos);
}

/// Anchor and subject already share a space: the named player snaps in place,
/// and the `TeleportPlayer` names **the summoned player**, never the caller.
#[tokio::test]
async fn legacy_p46_summon_same_space_snaps_the_named_player() {
    let (mut mgr, gm, _npc) = setup_worlds();
    let caller_pos = position_of(&mgr, gm);
    let agnos = mgr.get_entity_space_id(gm).unwrap();
    spawn_named_player(&mut mgr, 50, AGNOS, [42.0, 0.0, 43.0], "Bob");

    let t = run("summon", gm, &["Bob"], None, &mut mgr).await;

    assert!(
        t.gate_travels.is_empty(),
        "a same-space summon must not enqueue a transfer: {:?}",
        t.gate_travels
    );
    assert_eq!(
        t.teleports,
        vec![(50, agnos, caller_pos, [42.0, 0.0, 43.0])],
        "the snap must name the summoned player and carry their prior position"
    );
    assert_eq!(position_of(&mgr, 50), caller_pos);
    assert_eq!(
        position_of(&mgr, gm),
        caller_pos,
        "the caller must not move"
    );
    assert!(t.has_line("Summoning player <Bob>"));
}

/// The 2026-09-20 colo repro. The GM still had an NPC selected — clicked 19
/// minutes earlier and 216 m behind them — and legacy's `target or player`
/// anchor dropped the summoned player on that NPC instead of beside the GM.
/// Reverting `summon` to `target.unwrap_or(caller_id)` lands Bob on the NPC's
/// position and fails the first assertion.
#[tokio::test]
async fn p46_summon_ignores_a_selected_npc() {
    let (mut mgr, gm, npc) = setup_worlds();
    let caller_pos = position_of(&mgr, gm);
    let caller_space = mgr.get_entity_space_id(gm).unwrap();
    assert_ne!(
        position_of(&mgr, npc),
        caller_pos,
        "fixture: the selection must stand somewhere the caller is not"
    );
    spawn_named_player(&mut mgr, 50, CASTLE, [30.0, 0.0, 30.0], "Bob");

    let t = run("summon", gm, &["Bob"], Some(npc), &mut mgr).await;

    assert_eq!(
        t.only_gate_travel(),
        &(50, AGNOS.to_string(), Some(caller_space), caller_pos),
        "the player must come to the caller, not to the caller's selection"
    );
}

/// Same rule for a selected **player** in another instance: a GM who has a
/// party member targeted and summons a third player gets them at their own
/// feet, not inside the party member's instance.
#[tokio::test]
async fn p46_summon_ignores_a_selected_player_in_another_instance() {
    let (mut mgr, gm, _npc) = setup_worlds();
    let caller_pos = position_of(&mgr, gm);
    let caller_space = mgr.get_entity_space_id(gm).unwrap();
    let anas_instance = spawn_named_player(&mut mgr, 50, INSTANCED, [11.0, 0.0, 12.0], "Ana");
    spawn_named_player(&mut mgr, 51, CASTLE, [30.0, 0.0, 30.0], "Bob");
    assert_ne!(anas_instance, caller_space);

    let t = run("summon", gm, &["Bob"], Some(50), &mut mgr).await;

    assert_eq!(
        t.only_gate_travel(),
        &(51, AGNOS.to_string(), Some(caller_space), caller_pos),
        "Bob must land beside the caller, not in the selected player's instance"
    );
    assert_eq!(
        mgr.get_entity_space_id(50),
        Some(anas_instance),
        "the selected player is not part of the command and must not move"
    );
}
