//! `.summon <name>` — move a named player to the caller-or-selection.
//!
//! Legacy `deprecated/python/cell/commands/Player.py:321-341`. Same move as
//! `.goto` with the roles swapped: the *named* player is the subject, and
//! `entity = target or player` is only the destination anchor.

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

/// The named player is pulled into the **anchor's exact instance**, not the
/// default instance of the anchor's world — the mirror of `.goto`'s
/// instance-exactness criterion, and the case a GM running an instanced
/// dungeon actually hits.
#[tokio::test]
async fn legacy_p46_summon_pulls_the_player_into_the_anchors_exact_instance() {
    let (mut mgr, gm, _npc) = setup_worlds();
    let instance_a = spawn_named_player(&mut mgr, 50, INSTANCED, [11.0, 0.0, 12.0], "Ana");
    let instance_b = spawn_named_player(&mut mgr, 51, INSTANCED, [30.0, 0.0, 40.0], "Bob");
    assert_ne!(instance_a, instance_b);

    // Anchor = the selected player Ana, in instance A. Subject = Bob.
    let t = run("summon", gm, &["Bob"], Some(50), &mut mgr).await;

    assert_eq!(
        t.only_gate_travel(),
        &(
            51,
            INSTANCED.to_string(),
            Some(instance_a),
            [11.0, 0.0, 12.0]
        ),
        "Bob must be transferred into Ana's exact instance, at Ana's position"
    );
    assert_eq!(
        mgr.get_entity_space_id(51),
        None,
        "the summoned player must be torn out of their origin instance"
    );
    assert_eq!(
        mgr.get_entity_space_id(50),
        Some(instance_a),
        "the anchor must not move"
    );
    assert!(
        t.has_line("Summoning player <Bob>"),
        "legacy's exact success wording; got {:?}",
        t.feedback
    );
}

/// No selection: the anchor falls back to the caller (`entity = target or
/// player`), and the caller itself is never the entity that moves.
#[tokio::test]
async fn legacy_p46_summon_falls_back_to_the_caller_as_anchor() {
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

/// The anchor may be an NPC — D15's players-only rule constrains the entity
/// being *moved*, which for `.summon` is always the named player. Refusing
/// here would be over-applying the restriction.
#[tokio::test]
async fn legacy_p46_summon_accepts_an_npc_anchor() {
    let (mut mgr, gm, npc) = setup_worlds();
    let npc_pos = position_of(&mgr, npc);
    let npc_space = mgr.get_entity_space_id(npc).unwrap();
    spawn_named_player(&mut mgr, 50, CASTLE, [30.0, 0.0, 30.0], "Bob");

    let t = run("summon", gm, &["Bob"], Some(npc), &mut mgr).await;

    assert_eq!(
        t.only_gate_travel(),
        &(50, AGNOS.to_string(), Some(npc_space), npc_pos),
        "the player must be transferred to the NPC anchor's space and position"
    );
    assert!(
        !t.mentions("not a player"),
        "the anchor's kind must not trip D15's subject check; got {:?}",
        t.feedback
    );
}
