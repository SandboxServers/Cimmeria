//! Attacker-side guards: `threatened_mobs`, `BSF_InCombat`, and the
//! re-engage path. The leak these reproduce (H04 worknote request B) is
//! player-side, so every assertion here reads the *player* entity, not
//! the NPC.

use super::*;

// ── The player-side scrub (H04 worknote request B) ─────────────────────

/// A player who is genuinely in combat with the NPC must end up OUT of
/// combat when it surrenders: dropped from `threatened_mobs`,
/// `BSF_InCombat` cleared, and the clear pushed to their client.
///
/// Threat is seeded directly rather than by shooting, so the assertions
/// are about the handler and nothing else. The same claim driven through
/// the real shot-to-surrender path is
/// `health_crossing::a_shot_that_crosses_the_threshold_disengages_both_sides`
/// — that one additionally fails if the damage-path hook is unwired,
/// which this one cannot notice.
///
/// Reverting the `clear_dead_npc_from_all_player_threat` call to an
/// in-place `threat_list.clear()` fails this on the `threatened_mobs`
/// assertion: the NPC's list empties, the player's set does not.
#[tokio::test]
async fn submit_takes_the_attacker_out_of_combat() {
    let mut mgr = make_mgr();
    add_player(&mut mgr, PLAYER_A, 0.0);
    add_npc(&mut mgr, NPC, 5.0);

    // Both sides enter combat.
    let _ = generate_threat(&mut mgr, PLAYER_A, NPC, 50.0);
    assert_ne!(
        mgr.get_entity(PLAYER_A).unwrap().state_field & BSF_IN_COMBAT,
        0,
        "fixture invariant: the attacker must be in combat before the submit"
    );

    content_sets_submit(&mut mgr, NPC);
    let (tx, mut rx) = mpsc::channel(64);
    run_ai_tick(&tx, &mut mgr).await;

    let player = mgr.get_entity(PLAYER_A).unwrap();
    assert!(
        player.threatened_mobs.is_empty(),
        "the surrendered NPC must be dropped from the attacker's threatened_mobs — \
         this set, not the BSF bit, is what gates regen and the weapon posture",
    );
    assert_eq!(
        player.state_field & BSF_IN_COMBAT,
        0,
        "BSF_InCombat must clear once the last threatening mob surrendered",
    );
    assert!(
        mgr.get_entity(NPC).unwrap().threat_list.is_empty(),
        "and the NPC's own list is still drained",
    );

    let updates = state_updates_for(&drain(&mut rx), PLAYER_A);
    assert_eq!(
        updates.len(),
        1,
        "exactly one onStateFieldUpdate to the attacker's own client",
    );
    assert_eq!(
        updates[0] & BSF_IN_COMBAT,
        0,
        "and the broadcast payload must carry the cleared bit",
    );
}

/// Two players on the same NPC: the surrender must scrub BOTH, not just
/// whoever landed the crossing hit. Same multi-attacker shape the death
/// path fans out — a killer-only fix leaves the second player stuck in
/// combat with no mob to blame.
///
/// Fails on the same revert as the single-attacker case, and
/// additionally on any "scrub only the top-threat attacker" narrowing.
#[tokio::test]
async fn submit_clears_every_attacker_not_just_the_last_one() {
    let mut mgr = make_mgr();
    add_player(&mut mgr, PLAYER_A, 0.0);
    add_player(&mut mgr, PLAYER_B, 2.0);
    add_npc(&mut mgr, NPC, 5.0);

    let _ = generate_threat(&mut mgr, PLAYER_A, NPC, 50.0);
    let _ = generate_threat(&mut mgr, PLAYER_B, NPC, 90.0);
    for pid in [PLAYER_A, PLAYER_B] {
        assert_ne!(
            mgr.get_entity(pid).unwrap().state_field & BSF_IN_COMBAT,
            0,
            "fixture invariant: player {pid} must be in combat"
        );
    }

    content_sets_submit(&mut mgr, NPC);
    let (tx, mut rx) = mpsc::channel(64);
    run_ai_tick(&tx, &mut mgr).await;

    let msgs = drain(&mut rx);
    for pid in [PLAYER_A, PLAYER_B] {
        let player = mgr.get_entity(pid).unwrap();
        assert!(
            player.threatened_mobs.is_empty(),
            "player {pid} must be scrubbed too",
        );
        assert_eq!(player.state_field & BSF_IN_COMBAT, 0);
        assert_eq!(
            state_updates_for(&msgs, pid).len(),
            1,
            "player {pid} must get their own onStateFieldUpdate",
        );
    }
}

/// A player still threatened by a second, unrelated mob stays in combat
/// when the first one surrenders. Pins that the surrender routes through
/// the shared per-mob scrub rather than nuking `threatened_mobs`
/// wholesale — the failure shape would be a player who walks away from a
/// live fight with no combat HUD and full regen.
#[tokio::test]
async fn submit_leaves_a_player_in_combat_with_another_live_mob() {
    let mut mgr = make_mgr();
    add_player(&mut mgr, PLAYER_A, 0.0);
    add_npc(&mut mgr, NPC, 5.0);
    add_npc(&mut mgr, NPC + 1, 8.0);

    let _ = generate_threat(&mut mgr, PLAYER_A, NPC, 50.0);
    let _ = generate_threat(&mut mgr, PLAYER_A, NPC + 1, 50.0);

    content_sets_submit(&mut mgr, NPC);
    let (tx, _rx) = mpsc::channel(64);
    run_ai_tick(&tx, &mut mgr).await;

    let player = mgr.get_entity(PLAYER_A).unwrap();
    assert_eq!(
        player.threatened_mobs.len(),
        1,
        "only the surrendered mob leaves the set",
    );
    assert!(player.threatened_mobs.contains(&(NPC + 1)));
    assert_ne!(
        player.state_field & BSF_IN_COMBAT,
        0,
        "the other mob is still fighting, so the attacker stays in combat",
    );
}

/// Re-engaging a surrendered NPC re-runs the cleanup on the next AI pass.
///
/// This is the leak the probe's `!threat_list.is_empty()` term exists for:
/// `generate_threat` writes the attacker into a submitted NPC's
/// `threat_list` and calls `enter_player_combat` *outside* the
/// state-preemption guard, so every stray bullet at a surrendered NPC
/// puts the shooter back into permanent combat. Submit has no death or
/// leash exit to undo it — this handler is the only scrub. Replacing the
/// probe with a one-shot latch fails this test.
///
/// Explicit attacks themselves are unchanged: the shot lands, damage
/// applies, and the NPC can still be killed.
#[tokio::test]
async fn re_engaging_a_surrendered_npc_scrubs_the_attacker_again() {
    let mut mgr = make_mgr();
    add_player(&mut mgr, PLAYER_A, 0.0);
    add_npc(&mut mgr, NPC, 5.0);
    let _ = generate_threat(&mut mgr, PLAYER_A, NPC, 50.0);

    content_sets_submit(&mut mgr, NPC);
    let (tx, _rx) = mpsc::channel(64);
    run_ai_tick(&tx, &mut mgr).await;
    assert!(mgr.get_entity(PLAYER_A).unwrap().threatened_mobs.is_empty());

    // Player shoots the surrendered NPC anyway — allowed, and it re-arms
    // both sides' combat state.
    let _ = generate_threat(&mut mgr, PLAYER_A, NPC, 25.0);
    assert_ne!(
        mgr.get_entity(PLAYER_A).unwrap().state_field & BSF_IN_COMBAT,
        0,
        "fixture invariant: the stray shot must put the player back in combat"
    );
    assert_eq!(
        mgr.get_entity(NPC).unwrap().ai_state,
        AiState::Submit,
        "and the NPC must NOT be preempted back into Fighting",
    );

    run_ai_tick(&tx, &mut mgr).await;

    let player = mgr.get_entity(PLAYER_A).unwrap();
    assert!(
        player.threatened_mobs.is_empty(),
        "the next AI pass must scrub the attacker again — otherwise one stray \
         shot at a surrendered NPC denies that player regen for the session",
    );
    assert_eq!(player.state_field & BSF_IN_COMBAT, 0);
}
