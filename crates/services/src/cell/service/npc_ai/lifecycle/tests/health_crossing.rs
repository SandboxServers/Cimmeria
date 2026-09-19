//! **The H08 acceptance case, driven end to end through the real damage
//! seam.**
//!
//! Every other guard in this directory reaches Submit by writing
//! `ai_state` directly ([`content_sets_submit`]). That is the right
//! shape for asserting what the handler does, and the wrong shape for
//! asserting that the handler is *reached* — a test that hand-sets the
//! state cannot notice that the path from "a shot crossed the duel
//! threshold" to "the surrender handler runs" is broken anywhere along
//! its four hops:
//!
//! ```text
//!   handle_use_ability_with_kill_credit   (the player's shot)
//!     -> combat::note_pre_damage_health   (pct_before sample)
//!     -> content::fire_pending_health_below / fire_health_below_for_hit
//!     -> ChainEngine resolve + execute_actions
//!     -> executor::world::set_npc_ai_state  (ai_state = Submit)
//!   npc_ai_tick -> npc_ai_submit           (the cleanup)
//! ```
//!
//! PR #662's review fix R1 moved the `entity_health_below` sample off the
//! single-target path and onto the shared health-application seam, so the
//! hop that H04 wired is not the hop that runs today. This module drives
//! whatever the current seam is, which is the point: it fails if the seam
//! moves again and nobody re-wires it.
//!
//! No live DB. The chain is registered in memory rather than seeded,
//! because what is under test is the runtime path, not the loader — the
//! loader's `entity_health_below` arm has its own live-DB guard in
//! `cell::content::chain_replay_tests::entity_health_below`.

use super::*;

use cimmeria_content_engine::actions::{Action, NpcAiStateAction};
use cimmeria_content_engine::chain::{Chain, ChainEngine};
use cimmeria_content_engine::triggers::Trigger;

/// The duel NPC's content tag. Namespaced so it cannot collide with a
/// real seed row if these fixtures are ever pointed at a loaded engine.
const DUEL_TAG: &str = "CIMMERIA_TEST_H08_DUEL_NPC";

/// Threshold for the submit chain. 99 so the assertion holds for any
/// non-zero damage the QR roll produces: this module is about the wiring,
/// not the damage numbers (those are pinned at the dispatcher level in
/// `cell::content::event_dispatch::lifecycle::tests`).
const SUBMIT_PCT: i32 = 99;

/// A player mid-fight with a tagged duel NPC: threat on both sides,
/// `BSF_InCombat` lit, an armed auto-fire loop, and an ability that
/// actually deals damage. The state a duelist is actually in when the
/// ritual ends.
///
/// The shared [`install_ability_def`] fixture is deliberately damage-free
/// (the loop tests are about loop semantics), so this module attaches a
/// 5-point `HealthDamage` effect to the same ability id: 5 of 100 is a
/// crossing of the 99% threshold that cannot be lethal on any QR roll.
fn make_duel_in_progress() -> SpaceManager {
    let mut mgr = make_mgr();
    add_player(&mut mgr, PLAYER_A, 0.0);
    add_npc(&mut mgr, NPC, 5.0);
    install_ability_def(&mut mgr);

    let mut params = HashMap::new();
    params.insert("HealthDamage".to_string(), "5".to_string());
    mgr.effect_defs.insert(
        0x7008,
        EffectDef {
            effect_id: 0x7008,
            ability_id: ABILITY,
            params,
            ..Default::default()
        },
    );
    if let Some(def) = mgr.ability_defs.get_mut(&ABILITY) {
        def.effect_ids = vec![0x7008];
    }

    if let Some(npc) = mgr.get_entity_mut(NPC) {
        npc.tag = Some(DUEL_TAG.to_string());
    }
    arm_loop_at(&mut mgr, PLAYER_A, NPC);
    mgr
}

/// The chain H21 will author for mission 1325, minus its `step_status`
/// gate (that gate is pre-existing engine code with its own coverage;
/// carrying it here would only add a way for this test to fail for a
/// reason unrelated to H08).
fn submit_on_crossing_engine() -> ChainEngine {
    let mut engine = ChainEngine::new();
    engine.register_chain(Chain {
        action_delays: Vec::new(),
        id: 0x7008_0001,
        name: "test: duel NPC surrenders on health crossing".to_string(),
        enabled: true,
        trigger: Trigger::OnEntityHealthBelow {
            entity_tag: DUEL_TAG.to_string(),
            pct: SUBMIT_PCT,
        },
        conditions: vec![],
        actions: vec![Action::SetNpcAiState {
            entity_tag: DUEL_TAG.to_string(),
            state: NpcAiStateAction::Submit,
        }],
        priority: 0,
    });
    engine
}

/// **The packet's acceptance test.** A real shot crosses the duel
/// threshold, the chain flips the NPC to Submit through the live content
/// path, and the next AI tick must leave the attacker out of combat and
/// not auto-attacking.
///
/// Three independent reverts fail this, which is why it is worth its
/// length:
///
/// - Deleting the `fire_pending_health_below` / `fire_health_below_for_hit`
///   hook from the damage path fails the `ai_state == Submit` fixture
///   assertion — the surrender is never reached at all.
/// - Reverting `clear_dead_npc_from_all_player_threat` to an in-place
///   `threat_list.clear()` fails the `threatened_mobs` assertion.
/// - Reverting the `clear_auto_cycle_for_target` sweep fails the
///   `auto_cycle` assertion.
#[tokio::test]
async fn a_shot_that_crosses_the_threshold_disengages_both_sides() {
    let mut mgr = make_duel_in_progress();
    let engine = submit_on_crossing_engine();
    let (tx, mut rx) = mpsc::channel(128);

    crate::cell::abilities::handle_use_ability_with_kill_credit(
        PLAYER_A, ABILITY, NPC as i32, &engine, &tx, &mut mgr,
    )
    .await;

    // Fixture sanity before the real assertions: a QR miss or a lethal
    // roll would make everything below fail for a reason that has
    // nothing to do with the surrender path.
    let hp = mgr
        .get_entity(NPC)
        .and_then(|e| e.stats.get(HEALTH))
        .map(|s| s.cur)
        .expect("the duel NPC must still exist with a HEALTH stat");
    assert!(
        hp < 100 && hp > 0,
        "test fixture: the shot must wound without killing (health = {hp})",
    );
    assert_eq!(
        mgr.get_entity(NPC).unwrap().ai_state,
        AiState::Submit,
        "the crossing must reach the content engine and run \
         `set_npc_ai_state submit` — a Fighting NPC here means the \
         damage-path health-below hook is gone, not that the surrender \
         handler is broken",
    );
    assert_ne!(
        mgr.get_entity(PLAYER_A).unwrap().state_field & BSF_IN_COMBAT,
        0,
        "fixture: the shot put the attacker in combat, so the assertions \
         below are testing a clear and not a no-op",
    );
    let _ = drain(&mut rx);

    // The cleanup pass.
    run_ai_tick(&tx, &mut mgr).await;

    let player = mgr.get_entity(PLAYER_A).expect("the attacker must exist");
    assert!(
        player.threatened_mobs.is_empty(),
        "the surrendered NPC must be dropped from the attacker's \
         threatened_mobs — that set, not the BSF bit, is what gates \
         `regen_tick` and the weapon-drawn posture",
    );
    assert_eq!(
        player.state_field & BSF_IN_COMBAT,
        0,
        "and BSF_InCombat clears with it",
    );
    assert!(
        !player.abilities.auto_cycle,
        "the auto-fire loop must stop — leaving it armed is what kills \
         the surrendered NPC seconds later",
    );
    assert_eq!(
        player.state_field & BSF_AUTO_CYCLING,
        0,
        "and the client is told the loop is off",
    );

    let updates = state_updates_for(&drain(&mut rx), PLAYER_A);
    assert!(
        !updates.is_empty(),
        "the attacker's client must be told about the state change; a \
         server-only clear leaves the HUD in combat",
    );
    assert!(
        updates.iter().all(|s| s & BSF_IN_COMBAT == 0),
        "every broadcast payload must carry the cleared combat bit, got \
         {updates:?}",
    );
}

/// The kill window, closed at the 100 ms clock rather than the ~2 s one.
///
/// Same real crossing as above, but the AI tick is deliberately *not*
/// run: this is the state the world is actually in for up to two seconds
/// after the surrender, and it is the window in which the armed loop
/// would otherwise land several more shots. `auto_cycle_tick` must refuse
/// the target on its own, without help from the AI handler.
///
/// Reverting `is_auto_cycle_target_valid` to a bare `!is_dead_state`
/// check fails this: the tick re-fires, the ability goes on cooldown and
/// the surrendered NPC takes more damage.
#[tokio::test]
async fn the_auto_cycle_tick_closes_the_window_before_the_ai_tick_runs() {
    let mut mgr = make_duel_in_progress();
    let engine = submit_on_crossing_engine();
    let (tx, _rx) = mpsc::channel(128);

    crate::cell::abilities::handle_use_ability_with_kill_credit(
        PLAYER_A, ABILITY, NPC as i32, &engine, &tx, &mut mgr,
    )
    .await;
    assert_eq!(
        mgr.get_entity(NPC).unwrap().ai_state,
        AiState::Submit,
        "fixture: the crossing must have reached the surrender action",
    );

    // Clear the cooldown the crossing shot started, so a refusal below
    // is attributable to the target gate rather than to rate limiting.
    if let Some(p) = mgr.get_entity_mut(PLAYER_A) {
        p.abilities.clear_all_cooldowns();
    }
    let hp_after_crossing = mgr.get_entity(NPC).unwrap().stats.get(HEALTH).unwrap().cur;

    // No `run_ai_tick` here — that is the whole point.
    crate::cell::service::ticks::auto_cycle_tick(&tx, &mut mgr, &engine).await;

    assert_eq!(
        mgr.get_entity(NPC).unwrap().stats.get(HEALTH).unwrap().cur,
        hp_after_crossing,
        "the surrendered NPC must take no further auto-fire damage in \
         the window before the AI tick processes the surrender",
    );
    assert!(
        !mgr.get_entity(PLAYER_A).unwrap().abilities.auto_cycle,
        "and the loop is cleared outright, not merely skipped for a tick",
    );
}
