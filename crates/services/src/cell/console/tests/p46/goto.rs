//! `.goto <name>` — move the caller-or-selection to a named player.
//!
//! Legacy `deprecated/python/cell/commands/Player.py:298-318`.

use super::*;

/// Legacy `Player.py:307-309`: an unknown name reports "not available",
/// verbatim, and nothing else happens.
#[tokio::test]
async fn legacy_p46_goto_unknown_name_reports_not_available() {
    let (mut mgr, gm, _npc) = setup_worlds();
    let before = position_of(&mgr, gm);

    let t = run("goto", gm, &["Teal'c"], None, &mut mgr).await;

    assert!(
        t.has_line(NOT_AVAILABLE),
        "unknown name must report legacy's exact wording; got {:?}",
        t.feedback
    );
    assert_no_move(&t);
    assert_eq!(position_of(&mgr, gm), before, "the caller must not move");
}

/// Legacy `Player.py:312-314`: a player whose entity exists but is not in any
/// space's `players` set is "not on any reachable space" — a *different*
/// message from the unknown-name one. Collapsing P44's two failure variants
/// onto one string is the regression this pins.
#[tokio::test]
async fn legacy_p46_goto_in_transition_reports_not_reachable() {
    let (mut mgr, gm, _npc) = setup_worlds();
    // Created but never connected: named, present, unreachable.
    mgr.create_entity(50, AGNOS, [5.0, 0.0, 5.0], [0.0; 3])
        .unwrap();
    mgr.get_entity_mut(50).unwrap().character_name = Some("Daniel".into());

    let t = run("goto", gm, &["Daniel"], None, &mut mgr).await;

    assert!(
        t.has_line(NOT_REACHABLE),
        "an unreachable player must report the not-reachable wording, not the \
         not-available one; got {:?}",
        t.feedback
    );
    assert!(!t.has_line(NOT_AVAILABLE));
    assert_no_move(&t);
}

/// P44's `Ambiguous` is a fourth outcome with no legacy equivalent. It must
/// refuse rather than silently picking one of the duplicates — the whole
/// reason the lookup collects every match before deciding.
#[tokio::test]
async fn legacy_p46_goto_ambiguous_name_refuses_to_guess() {
    let (mut mgr, gm, _npc) = setup_worlds();
    spawn_named_player(&mut mgr, 50, AGNOS, [5.0, 0.0, 5.0], "Daniel");
    spawn_named_player(&mut mgr, 51, CASTLE, [6.0, 0.0, 6.0], "Daniel");

    let t = run("goto", gm, &["Daniel"], None, &mut mgr).await;

    assert!(
        t.mentions("Multiple players named"),
        "a duplicated name must be refused explicitly; got {:?}",
        t.feedback
    );
    assert!(
        !t.has_line(NOT_AVAILABLE) && !t.has_line(NOT_REACHABLE),
        "ambiguity must not be reported as either legacy failure: {:?}",
        t.feedback
    );
    assert_no_move(&t);
}

/// Happy path, same space: the caller snaps to the named player's exact
/// position with no `GateTravel` at all.
///
/// Regression shape: an adapter that always calls the transfer primitive, or
/// that treats `Ok(SameSpace)` as "done", would either bounce the GM through
/// a gratuitous loading screen or report a teleport that never happened.
#[tokio::test]
async fn legacy_p46_goto_same_space_snaps_without_gate_travel() {
    let (mut mgr, gm, _npc) = setup_worlds();
    let caller_before = position_of(&mgr, gm);
    let agnos = mgr.get_entity_space_id(gm).unwrap();
    spawn_named_player(&mut mgr, 50, AGNOS, [20.0, 0.0, 20.0], "Bob");

    let t = run("goto", gm, &["Bob"], None, &mut mgr).await;

    assert!(
        t.gate_travels.is_empty(),
        "a same-space .goto must not enqueue a transfer: {:?}",
        t.gate_travels
    );
    assert_eq!(
        t.teleports,
        vec![(gm, agnos, [20.0, 0.0, 20.0], caller_before)],
        "the caller must be snapped to the target's exact position"
    );
    assert_eq!(position_of(&mgr, gm), [20.0, 0.0, 20.0]);
    assert!(
        t.has_line("Teleporting to player <Bob>"),
        "legacy's exact success wording; got {:?}",
        t.feedback
    );
}

/// The packet's core acceptance criterion: with two instances of the same
/// instanced world loaded, `.goto` joins the **target's** instance, not "an"
/// instance of the right world.
///
/// Regression shape: passing `None` for the destination space (i.e.
/// re-resolving the target's world name) sends the GM to the *oldest* loaded
/// instance — instance A here — which is an empty copy of the map.
#[tokio::test]
async fn legacy_p46_goto_joins_the_named_players_exact_instance() {
    let (mut mgr, gm, _npc) = setup_worlds();
    let origin = mgr.get_entity_space_id(gm).unwrap();
    let instance_a = spawn_named_player(&mut mgr, 50, INSTANCED, [1.0, 0.0, 1.0], "Ana");
    let instance_b = spawn_named_player(&mut mgr, 51, INSTANCED, [30.0, 0.0, 40.0], "Bob");
    assert_ne!(
        instance_a, instance_b,
        "an instanced world must allocate a distinct space per create — \
         without that this scenario is a mirage"
    );

    let t = run("goto", gm, &["Bob"], None, &mut mgr).await;

    assert_eq!(
        t.only_gate_travel(),
        &(
            gm,
            INSTANCED.to_string(),
            Some(instance_b),
            [30.0, 0.0, 40.0]
        ),
        "the transfer must name Bob's exact instance and position"
    );
    assert!(
        t.teleports.is_empty(),
        "a cross-space move is a transfer, not a snap"
    );
    assert_eq!(
        mgr.get_entity_space_id(gm),
        None,
        "the subject must have been torn out of its origin space"
    );
    assert_ne!(Some(instance_a), Some(origin));
    assert!(t.has_line("Teleporting to player <Bob>"));
}

/// Three loaded instances of the same world, one of them shared by two
/// players: `.goto` still resolves to the instance the *named* player is in.
///
/// The shared instance matters because it is the realistic case — a GM
/// joining a party already running a dungeon — and because it is the one an
/// "allocate a fresh instance" regression would silently pass without: the
/// destination would be a new, empty space that happens to be in the right
/// world.
#[tokio::test]
async fn legacy_p46_goto_picks_the_instance_holding_the_named_player() {
    let (mut mgr, gm, _npc) = setup_worlds();
    // Two decoy instances FIRST, so the shared one is never the default —
    // otherwise a "re-resolve by world name" regression would coincidentally
    // land on the right space and the test would prove nothing.
    let decoy_a = spawn_named_player(&mut mgr, 52, INSTANCED, [3.0, 0.0, 3.0], "Carl");
    let decoy_b = spawn_named_player(&mut mgr, 53, INSTANCED, [4.0, 0.0, 4.0], "Dana");
    let shared = spawn_named_player(&mut mgr, 50, INSTANCED, [1.0, 0.0, 1.0], "Ana");
    spawn_named_player_in_space(&mut mgr, 51, shared, [25.0, 0.0, 26.0], "Bob");
    assert_eq!(
        mgr.get_entity_space_id(51),
        Some(shared),
        "Bob must share Ana's instance"
    );
    assert!(decoy_a != shared && decoy_b != shared && decoy_a != decoy_b);
    assert_ne!(
        mgr.default_space_for_world(INSTANCED),
        Some(shared),
        "the shared instance must not also be the default instance"
    );

    let t = run("goto", gm, &["Bob"], None, &mut mgr).await;

    assert_eq!(
        t.only_gate_travel(),
        &(gm, INSTANCED.to_string(), Some(shared), [25.0, 0.0, 26.0]),
        "the destination must be the instance Bob is in, out of three loaded"
    );
}

/// D15: the entity being *moved* across spaces must be a player. An NPC
/// selection is refused, and refused *before* anything is torn down.
#[tokio::test]
async fn legacy_p46_goto_npc_selection_cross_space_is_rejected() {
    let (mut mgr, gm, npc) = setup_worlds();
    let npc_before = position_of(&mgr, npc);
    let npc_space = mgr.get_entity_space_id(npc);
    spawn_named_player(&mut mgr, 50, CASTLE, [30.0, 0.0, 30.0], "Bob");

    let t = run("goto", gm, &["Bob"], Some(npc), &mut mgr).await;

    assert_no_move(&t);
    assert!(
        t.mentions("not a player"),
        "an NPC subject must be refused with a clear reason (D15); got {:?}",
        t.feedback
    );
    assert!(!t.mentions("Teleporting"), "must not claim success");
    assert_eq!(position_of(&mgr, npc), npc_before);
    assert_eq!(
        mgr.get_entity_space_id(npc),
        npc_space,
        "a refused transfer must not tear the NPC out of its space"
    );
}

/// ...but a *same-space* `.goto` with an NPC selected still works: D15
/// restricts only the cross-space legs, and this is the same in-place snap
/// `.gotoxyz` already performs on NPCs.
#[tokio::test]
async fn legacy_p46_goto_same_space_moves_a_selected_npc() {
    let (mut mgr, gm, npc) = setup_worlds();
    spawn_named_player(&mut mgr, 50, AGNOS, [20.0, 0.0, 20.0], "Bob");

    let t = run("goto", gm, &["Bob"], Some(npc), &mut mgr).await;

    assert_eq!(
        position_of(&mgr, npc),
        [20.0, 0.0, 20.0],
        "the NPC must be moved to the named player's position"
    );
    assert!(
        t.teleports.is_empty(),
        "an NPC has no client, so no forced-position push: {:?}",
        t.teleports
    );
    assert!(t.gate_travels.is_empty());
    assert!(t.has_line("Teleporting to player <Bob>"));
}

/// Legacy's `entity = target or player`: the *selection* is what moves, and
/// the caller stays exactly where they are (D03 — the caller only receives
/// the feedback line).
#[tokio::test]
async fn legacy_p46_goto_moves_the_selection_not_the_caller() {
    let (mut mgr, gm, _npc) = setup_worlds();
    let caller_before = position_of(&mgr, gm);
    let caller_space = mgr.get_entity_space_id(gm);
    spawn_named_player(&mut mgr, 50, CASTLE, [30.0, 0.0, 30.0], "Bob");
    spawn_named_player(&mut mgr, 51, AGNOS, [4.0, 0.0, 4.0], "Carl");

    let t = run("goto", gm, &["Bob"], Some(51), &mut mgr).await;

    assert_eq!(
        t.only_gate_travel().0,
        51,
        "the selected player is the one transferred"
    );
    assert_eq!(position_of(&mgr, gm), caller_before);
    assert_eq!(
        mgr.get_entity_space_id(gm),
        caller_space,
        "the caller must stay in their own space"
    );
}

/// A closed base channel must leave the subject exactly where it was rather
/// than tearing it out with no transfer in flight — P45's ordering contract,
/// surfaced to the GM instead of being swallowed.
#[tokio::test]
async fn legacy_p46_goto_closed_base_channel_leaves_the_subject_in_place() {
    let (mut mgr, gm, _npc) = setup_worlds();
    let caller_before = position_of(&mgr, gm);
    let caller_space = mgr.get_entity_space_id(gm);
    spawn_named_player(&mut mgr, 50, CASTLE, [30.0, 0.0, 30.0], "Bob");

    let engine = ChainEngine::new();
    let (tx, rx) = mpsc::channel(16);
    drop(rx); // base channel closed

    exec("goto", gm, &["Bob"], None, &tx, &mut mgr, &engine).await; // must not panic

    assert_eq!(
        mgr.get_entity_space_id(gm),
        caller_space,
        "a failed enqueue must not remove the subject from its space"
    );
    assert_eq!(position_of(&mgr, gm), caller_before);
}
