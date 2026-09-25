//! Auto-fire loop guards (H04 worknote request C). Two mechanisms on
//! two clocks: the one-shot `clear_auto_cycle_for_target` sweep the
//! submit handler runs on the ~2 s AI cadence, and the per-tick
//! `is_auto_cycle_target_valid` gate `auto_cycle_tick` applies every
//! 100 ms. The gate is what actually closes the kill window; the sweep
//! is what un-highlights the client's button promptly.

use super::*;
use crate::cell::combat::AggroCause;

// ── The auto-cycle stop (H04 worknote request C) ───────────────────────

/// Submit must stop the attacker's auto-fire loop: `auto_cycle` off,
/// `BSF_AUTO_CYCLING` cleared, and the un-highlight pushed to the client.
///
/// Reverting the `clear_auto_cycle_for_target` call fails this — the loop
/// stays armed and the tick keeps re-firing until the NPC dies.
#[tokio::test]
async fn submit_stops_the_attackers_auto_fire_loop() {
    let mut mgr = make_mgr();
    add_player(&mut mgr, PLAYER_A, 0.0);
    add_npc(&mut mgr, NPC, 5.0);

    let _ = generate_threat(&mut mgr, PLAYER_A, NPC, 50.0, AggroCause::Damage);
    arm_loop_at(&mut mgr, PLAYER_A, NPC);

    content_sets_submit(&mut mgr, NPC);
    let (tx, mut rx) = mpsc::channel(64);
    run_ai_tick(&tx, &mut mgr).await;

    let player = mgr.get_entity(PLAYER_A).unwrap();
    assert!(
        !player.abilities.auto_cycle,
        "the loop must be disarmed, not just skipped for a tick",
    );
    assert_eq!(player.abilities.auto_cycle_ability_id, None);
    assert_eq!(
        player.state_field & BSF_AUTO_CYCLING,
        0,
        "BSF_AutoCycling must clear so the client un-highlights the button",
    );
    assert!(
        !state_updates_for(&drain(&mut rx), PLAYER_A).is_empty(),
        "the attacker must be told about the state change",
    );
}

/// A player auto-firing at a DIFFERENT mob keeps their loop when this one
/// surrenders. Pins the live-`current_target_id` filter — the failure
/// shape is a surrender that silently cancels a bystander's attack.
#[tokio::test]
async fn submit_leaves_a_loop_aimed_at_another_mob_alone() {
    let mut mgr = make_mgr();
    add_player(&mut mgr, PLAYER_A, 0.0);
    add_npc(&mut mgr, NPC, 5.0);
    add_npc(&mut mgr, NPC + 1, 8.0);

    let _ = generate_threat(&mut mgr, PLAYER_A, NPC, 50.0, AggroCause::Damage);
    arm_loop_at(&mut mgr, PLAYER_A, NPC + 1);

    content_sets_submit(&mut mgr, NPC);
    let (tx, _rx) = mpsc::channel(64);
    run_ai_tick(&tx, &mut mgr).await;

    let player = mgr.get_entity(PLAYER_A).unwrap();
    assert!(
        player.abilities.auto_cycle,
        "the loop is aimed elsewhere and must survive",
    );
    assert_ne!(player.state_field & BSF_AUTO_CYCLING, 0);
}

/// The durable half of the auto-cycle stop, and the one that actually
/// makes a post-surrender kill impossible: `auto_cycle_tick` runs every
/// 100 ms while the AI handler above runs every ~2 s, so without this
/// gate the loop lands several more shots before the surrender is
/// processed.
///
/// Reverting `is_auto_cycle_target_valid` to a bare `!is_dead_state`
/// check fails this — the tick takes the re-fire branch, starts the
/// ability cooldown, and the surrendered NPC takes another hit.
#[tokio::test]
async fn auto_cycle_tick_refuses_to_re_fire_at_a_surrendered_npc() {
    let mut mgr = make_mgr();
    add_player(&mut mgr, PLAYER_A, 0.0);
    add_npc(&mut mgr, NPC, 5.0);
    install_ability_def(&mut mgr);
    arm_loop_at(&mut mgr, PLAYER_A, NPC);
    content_sets_submit(&mut mgr, NPC);

    let (tx, _rx) = mpsc::channel(64);
    let npc_health_before = mgr.get_entity(NPC).unwrap().stats.get(HEALTH).unwrap().cur;

    crate::cell::service::ticks::auto_cycle_tick(
        &tx,
        &mut mgr,
        &cimmeria_content_engine::chain::ChainEngine::new(),
    )
    .await;

    let player = mgr.get_entity(PLAYER_A).unwrap();
    assert!(
        !player.abilities.auto_cycle,
        "a surrendered target must clear the loop, not merely skip a tick",
    );
    assert!(
        !player.abilities.is_on_cooldown(ABILITY),
        "and no shot may be fired — a started cooldown means the tick re-fired",
    );
    assert_eq!(
        mgr.get_entity(NPC).unwrap().stats.get(HEALTH).unwrap().cur,
        npc_health_before,
        "the surrendered NPC must take no further auto-fire damage",
    );
}

/// Control for the test above: the same fixture with the NPC left
/// Fighting DOES re-fire. Without this, a gate that rejected every target
/// would look identical to a correct one.
#[tokio::test]
async fn auto_cycle_tick_still_re_fires_at_a_fighting_npc() {
    let mut mgr = make_mgr();
    add_player(&mut mgr, PLAYER_A, 0.0);
    add_npc(&mut mgr, NPC, 5.0);
    install_ability_def(&mut mgr);
    arm_loop_at(&mut mgr, PLAYER_A, NPC);

    let (tx, _rx) = mpsc::channel(64);
    crate::cell::service::ticks::auto_cycle_tick(
        &tx,
        &mut mgr,
        &cimmeria_content_engine::chain::ChainEngine::new(),
    )
    .await;

    let player = mgr.get_entity(PLAYER_A).unwrap();
    assert!(player.abilities.auto_cycle, "a live target keeps the loop");
    assert!(
        player.abilities.is_on_cooldown(ABILITY),
        "and the tick fires — proving the surrender case above is the gate, \
         not a fixture that never fires at all",
    );
}
