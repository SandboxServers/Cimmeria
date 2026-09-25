//! NA12: the leash policy, the reset and the player-side drain, on meshless
//! spaces. The walk home over a real navmesh is in [`super::leash_walk`].
//!
//! Each test names the audit finding it guards (S3, S5, S6, S7, S12) and
//! fails when the NA12 change it pins is reverted.

use std::time::{Duration, Instant};

use super::*;
use crate::cell::combat::{self, AggroCause, BSF_IN_COMBAT};
use crate::cell::messages::CellToBaseMsg;
use crate::test_support::LogCapture;
use cimmeria_common::Vector3;
use tokio::sync::mpsc;

const NPC: u32 = 200;
const PLAYER: u32 = 101;

async fn ai_tick(mgr: &mut SpaceManager) -> Vec<CellToBaseMsg> {
    let (tx, mut rx) = mpsc::channel(4096);
    crate::cell::service::npc_ai::npc_ai_tick(
        &tx,
        mgr,
        &cimmeria_content_engine::chain::ChainEngine::new(),
    )
    .await;
    std::iter::from_fn(|| rx.try_recv().ok()).collect()
}

/// A fighting NPC at `npc_pos` with spawn at the origin, and a connected
/// full-health player at `player_pos` on its threat list and in combat.
fn fight_fixture(npc_pos: [f32; 3], player_pos: [f32; 3]) -> SpaceManager {
    let mut mgr = make_ai_fixture([0.0; 3], npc_pos);
    seed_default_ability(&mut mgr, 0, 30);
    seed_target_with_threat(&mut mgr, NPC, PLAYER, player_pos);
    mgr.connect_entity(PLAYER);
    let _ = mgr.compute_aoi_changes();
    let _ = combat::enter_player_combat(&mut mgr, PLAYER, NPC);
    mgr
}

/// Move every leash timestamp on the NPC `by` into the past: simulated time.
fn age_leash_clocks(mgr: &mut SpaceManager, by: Duration) {
    let npc = mgr.get_entity_mut(NPC).unwrap();
    for t in [
        &mut npc.leash.walk_started_at,
        &mut npc.leash.target_lost_since,
        &mut npc.leash.reaggro_suppressed_until,
    ] {
        *t = t.and_then(|i| i.checked_sub(by));
    }
}

fn transitions_to(logs: &crate::test_support::LogCaptureGuard, to: &str) -> Vec<String> {
    logs.all()
        .into_iter()
        .filter(|c| c.target == "npc_ai.transition" && c.has_field("to", to))
        .map(|c| c.fields.get("reason").cloned().unwrap_or_default())
        .collect()
}

/// S3. The leash is measured on the NPC, not the player. An NPC at its
/// spawn keeps fighting a player at the tutorial staging spot (49.9 u out
/// horizontally, 4 u up: 50.06 u in 3D, which the old spawn-to-target test
/// leashed on) and a player 60 u out.
#[tokio::test]
async fn leash_is_measured_on_the_npc_not_the_player() {
    for player_pos in [[49.9, 4.0, 0.0], [60.0, 0.0, 0.0]] {
        let mut mgr = fight_fixture([0.0; 3], player_pos);
        ai_tick(&mut mgr).await;
        let npc = mgr.get_entity(NPC).unwrap();
        assert_eq!(
            npc.ai_state(),
            AiState::Fighting,
            "an NPC standing at its spawn must not leash on a player at {player_pos:?}"
        );
        assert!(npc.threat_list.contains_key(&PLAYER));
    }

    // Control: the same player, with the NPC itself past the band, leashes.
    let mut mgr = fight_fixture([56.0, 0.0, 0.0], [60.0, 0.0, 0.0]);
    ai_tick(&mut mgr).await;
    assert_eq!(mgr.get_entity(NPC).unwrap().ai_state(), AiState::Leashing);
}

/// A template's `leash_distance` replaces the default radius.
#[tokio::test]
async fn template_leash_distance_overrides_the_default() {
    let mut mgr = fight_fixture([56.0, 0.0, 0.0], [60.0, 0.0, 0.0]);
    mgr.get_entity_mut(NPC).unwrap().leash.distance_override = Some(80.0);
    ai_tick(&mut mgr).await;
    assert_eq!(
        mgr.get_entity(NPC).unwrap().ai_state(),
        AiState::Fighting,
        "56 u out is inside an 80 u leash"
    );
}

/// S7. A leash drains the player's combat state for this NPC: the NPC leaves
/// `threatened_mobs`, `BSF_InCombat` clears, the player is told, and regen
/// resumes on the next regen tick.
#[tokio::test]
async fn leash_drains_player_combat_and_regen_resumes() {
    let mut mgr = fight_fixture([56.0, 0.0, 0.0], [60.0, 0.0, 0.0]);
    if let Some(p) = mgr.get_entity_mut(PLAYER) {
        let h = p.stats.get_mut(HEALTH).unwrap();
        h.update(0, 50, 100);
        h.clear_dirty();
    }
    assert!(
        mgr.get_entity(PLAYER).unwrap().state_field & BSF_IN_COMBAT != 0,
        "precondition: the player is in combat with the NPC"
    );

    let msgs = ai_tick(&mut mgr).await;

    assert_eq!(mgr.get_entity(NPC).unwrap().ai_state(), AiState::Leashing);
    let player = mgr.get_entity(PLAYER).unwrap();
    assert!(
        player.threatened_mobs.is_empty(),
        "the leashing NPC must leave the player's threatened_mobs"
    );
    assert_eq!(player.state_field & BSF_IN_COMBAT, 0, "BSF_InCombat clears");
    assert!(
        msgs.iter().any(|m| matches!(
            m,
            CellToBaseMsg::EntityMethodCall { entity_id: PLAYER, method_index, .. }
                if *method_index == crate::mercury::method_idx::ON_STATE_FIELD_UPDATE
        )),
        "the player's client must be sent the cleared state field"
    );

    let (tx, _rx) = mpsc::channel(64);
    crate::cell::service::ticks::regen_tick(&tx, &mut mgr).await;
    assert!(
        mgr.get_entity(PLAYER)
            .unwrap()
            .stats
            .get(HEALTH)
            .unwrap()
            .cur
            > 50,
        "out of combat, the player regenerates"
    );
}

/// S6 + S7. The target dies: the NPC drops it, drains the player's combat
/// state, and heads home (Leashing) in the same tick instead of going Idle
/// where it stands on the next one.
#[tokio::test]
async fn target_death_sends_the_npc_home_and_drains_the_player() {
    let mut mgr = fight_fixture([20.0, 0.0, 0.0], [25.0, 0.0, 0.0]);
    if let Some(p) = mgr.get_entity_mut(PLAYER) {
        p.stats.get_mut(HEALTH).unwrap().update(0, 0, 100);
    }
    let logs = LogCapture::install();

    ai_tick(&mut mgr).await;

    let npc = mgr.get_entity(NPC).unwrap();
    assert_eq!(npc.ai_state(), AiState::Leashing);
    assert!(npc.threat_list.is_empty());
    assert!(mgr.get_entity(PLAYER).unwrap().threatened_mobs.is_empty());
    assert_eq!(transitions_to(&logs, "leashing"), ["target_lost"]);
}

/// The target has been beyond the NPC's AoI for the grace period: dropped,
/// and the NPC goes home. Inside the grace period the clock only starts.
#[tokio::test]
async fn target_out_of_aoi_for_the_grace_period_is_lost() {
    let mut mgr = fight_fixture([0.0; 3], [150.0, 0.0, 0.0]);
    ai_tick(&mut mgr).await;
    let npc = mgr.get_entity(NPC).unwrap();
    assert_eq!(npc.ai_state(), AiState::Fighting, "control: grace running");
    assert!(npc.leash.target_lost_since.is_some(), "the clock started");

    mgr.get_entity_mut(NPC).unwrap().leash.target_lost_since =
        Some(Instant::now() - Duration::from_secs(6));
    ai_tick(&mut mgr).await;
    let npc = mgr.get_entity(NPC).unwrap();
    assert_eq!(npc.ai_state(), AiState::Leashing);
    assert!(npc.threat_list.is_empty());
    assert!(mgr.get_entity(PLAYER).unwrap().threatened_mobs.is_empty());
}

/// Snap fallback: no navmesh means no route home, so the leash tick snaps
/// the NPC to spawn, heals it and goes Idle under `leash_snap_fallback`,
/// with no route left behind.
#[tokio::test]
async fn no_route_home_snaps_heals_and_idles() {
    let mut mgr = fight_fixture([60.0, 0.0, 0.0], [70.0, 0.0, 0.0]);
    mgr.get_entity_mut(NPC)
        .unwrap()
        .stats
        .get_mut(HEALTH)
        .unwrap()
        .update(0, 30, 100);
    let logs = LogCapture::install();

    ai_tick(&mut mgr).await;
    assert_eq!(mgr.get_entity(NPC).unwrap().ai_state(), AiState::Leashing);
    ai_tick(&mut mgr).await;

    let npc = mgr.get_entity(NPC).unwrap();
    assert_eq!(npc.ai_state(), AiState::Idle);
    assert_eq!(npc.position, Vector3::new(0.0, 0.0, 0.0));
    assert!(npc.nav_path.is_empty());
    assert_eq!(npc.velocity, [0.0; 3]);
    assert_eq!(npc.stats.get(HEALTH).unwrap().cur, 100);
    assert!(npc.leash.reaggro_suppressed_until.is_some());
    assert_eq!(transitions_to(&logs, "idle"), ["leash_snap_fallback"]);
}

/// S12 / evade. Damage on a leashing NPC adds no threat and does not put the
/// attacker in combat, and says so at DEBUG.
#[tokio::test]
async fn leashing_npc_evades_threat_and_logs_it() {
    let mut mgr = fight_fixture([0.0; 3], [10.0, 0.0, 0.0]);
    let _ = combat::exit_player_combat(&mut mgr, PLAYER, NPC);
    if let Some(npc) = mgr.get_entity_mut(NPC) {
        crate::cell::service::npc_ai::force_ai_state(npc, AiState::Leashing);
        npc.threat_list.clear();
    }
    let logs = LogCapture::install();

    let sent = combat::generate_threat(&mut mgr, PLAYER, NPC, 40.0, AggroCause::Damage);

    assert_eq!(sent, None);
    let npc = mgr.get_entity(NPC).unwrap();
    assert_eq!(npc.ai_state(), AiState::Leashing);
    assert!(npc.threat_list.is_empty(), "no threat while evading");
    assert!(
        mgr.get_entity(PLAYER).unwrap().threatened_mobs.is_empty(),
        "shooting an evading NPC must not put the player in combat"
    );
    let row = logs
        .all()
        .into_iter()
        .find(|c| c.target == "npc_ai.leash" && c.has_field("event", "damage_ignored"))
        .expect("npc_ai.leash event=damage_ignored");
    assert_eq!(row.level, tracing::Level::DEBUG);
}

/// Post-reset suppression: inside the window an aggressive Idle NPC ignores
/// the player in its AoI; once it closes, it aggroes.
#[tokio::test]
async fn reaggro_is_suppressed_just_after_a_reset() {
    let mut mgr = make_aggression_fixture(NPC, 10, PLAYER, [10.0, 0.0, 0.0]);
    if let Some(npc) = mgr.get_entity_mut(NPC) {
        npc.aggro.override_level = Some(cimmeria_entity::cell_entity::MobAggression::Hostile);
        npc.leash.reaggro_suppressed_until = Some(Instant::now() + Duration::from_secs(5));
    }
    ai_tick(&mut mgr).await;
    assert_eq!(mgr.get_entity(NPC).unwrap().ai_state(), AiState::Idle);

    mgr.get_entity_mut(NPC)
        .unwrap()
        .leash
        .reaggro_suppressed_until = Some(Instant::now() - Duration::from_millis(1));
    ai_tick(&mut mgr).await;
    assert_eq!(
        mgr.get_entity(NPC).unwrap().ai_state(),
        AiState::Fighting,
        "control: with the window closed the same NPC aggroes"
    );
}

/// S5. The aggro/leash loop is gone. An aggressive NPC with a player 60 u
/// from its spawn, inside its AoI, is run for 60 simulated seconds (30 AI
/// ticks at the 2 s cadence, with the 100 ms movement tick between). Before
/// NA12 it aggroed, leashed on the player's distance, snapped home and
/// re-aggroed every cycle; now it leashes at most once.
#[tokio::test]
async fn aggro_leash_loop_is_gone_over_sixty_seconds() {
    let mut mgr = make_aggression_fixture(NPC, 10, PLAYER, [60.0, 0.0, 0.0]);
    seed_default_ability(&mut mgr, 0, 30);
    // Faction 10 is hostile on its own (NA13). The 60 u player is outside
    // the 18 u default aggro radius, which on its own would stop the loop;
    // widen it to the AoI so this still tests the leash, as before NA13.
    mgr.get_entity_mut(NPC).unwrap().aggro.radius_override = Some(100.0);
    let logs = LogCapture::install();

    for _ in 0..30 {
        ai_tick(&mut mgr).await;
        for _ in 0..20 {
            crate::cell::service::ticks::npc_movement_tick(&mut mgr);
        }
        // The loop runs in milliseconds of wall time, but the leash clocks
        // read `Instant::now()`. Age them by the 2 s this iteration stands
        // for, so the 5 s re-aggro window closes as it would in play and
        // cannot hide a loop.
        age_leash_clocks(&mut mgr, Duration::from_secs(2));
    }

    let leashes = transitions_to(&logs, "leashing").len();
    let aggros = transitions_to(&logs, "fighting").len();
    assert!(
        leashes <= 1,
        "at most one leash in 60 s, got {leashes} (and {aggros} aggros)"
    );
    assert!(aggros >= 1, "control: the NPC did aggro on the player");
}
