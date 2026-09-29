//! NPC-vs-NPC combat (#1009) through the real AI tick, on a meshless Castle
//! space: target acquisition by faction reaction with the NPC as the viewer,
//! the witness gate, the refusals and their log rows, the closest-target
//! pick between a player and an NPC, and a mixed player/NPC threat table
//! across an NPC death.
//!
//! Factions: 3 (Praxis) and 10 (Straegis) are mutually HOSTILE in the
//! client's reaction table; 10 and 10 are FRIENDLY. Players are faction 0
//! server-side and react as faction 3.

use super::seed_default_ability;
use cimmeria_entity::cell_entity::AiState;
use cimmeria_entity::stats::HEALTH;
use tokio::sync::mpsc;

use crate::cell::combat::HOSTILE_FACTION;
use crate::cell::space_manager::SpaceManager;
use crate::test_support::{LogCapture, LogCaptureGuard};

const PLAYER: u32 = 1;
const FRIENDLY: u32 = 200_101;
const HOSTILE: u32 = 200_102;
const PRAXIS: u8 = 3;

/// A meshless, non-instanced Castle space with a connected player at
/// `player_pos` (the witness) and nothing else.
fn space(player_pos: [f32; 3]) -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    mgr.parse_spaces_xml(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#,
    )
    .unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#,
    )
    .unwrap();
    mgr.create_entity(PLAYER, "Castle", player_pos, [0.0; 3])
        .unwrap();
    {
        let p = mgr.get_entity_mut(PLAYER).unwrap();
        p.is_player = true;
        p.player_id = Some(PLAYER as i32);
        p.faction = 0;
        let h = p.stats.get_mut(HEALTH).unwrap();
        h.update(0, 100, 100);
        h.clear_dirty();
    }
    mgr.connect_entity(PLAYER);
    mgr
}

/// Spawn an Idle SGWMob of `faction` at `pos` with full health.
fn npc(mgr: &mut SpaceManager, id: u32, faction: u8, pos: [f32; 3]) {
    mgr.spawn_npc(id, "Castle", pos, [0.0; 3]).unwrap();
    let e = mgr.get_entity_mut(id).unwrap();
    e.faction = faction;
    crate::cell::service::npc_ai::force_ai_state(e, AiState::Idle);
    let h = e.stats.get_mut(HEALTH).unwrap();
    h.update(0, 100, 100);
    h.clear_dirty();
}

/// The standard pair: a Praxis friendly at the origin and a NID guard
/// `gap` u east, with the player witness 40 u south of both (outside every
/// aggro radius, inside the 150 u AoI).
fn pair(gap: f32) -> SpaceManager {
    let mut mgr = space([0.0, 0.0, -40.0]);
    npc(&mut mgr, FRIENDLY, PRAXIS, [0.0, 0.0, 0.0]);
    npc(&mut mgr, HOSTILE, HOSTILE_FACTION, [gap, 0.0, 0.0]);
    let _ = mgr.compute_aoi_changes();
    mgr
}

async fn tick(mgr: &mut SpaceManager) {
    let (tx, _rx) = mpsc::channel(1024);
    crate::cell::service::npc_ai::npc_ai_tick(
        &tx,
        mgr,
        &crate::cell::content::EngineEvents(&cimmeria_content_engine::chain::ChainEngine::new()),
    )
    .await;
}

fn fighting(mgr: &SpaceManager, npc: u32, target: u32) -> bool {
    let e = mgr.get_entity(npc).unwrap();
    e.ai_state() == AiState::Fighting && e.threat_list.contains_key(&target)
}

fn rows<'a>(logs: &'a LogCaptureGuard, event: &'a str) -> Vec<crate::test_support::Captured> {
    logs.all()
        .into_iter()
        .filter(|c| c.has_field("event", event))
        .collect()
}

/// **Guard.** A Praxis friendly and a NID guard 10 u apart, watched by a
/// player, engage each other on the next tick: each holds the other on its
/// threat list and is Fighting. Before #1009 both stood Idle forever (the
/// friendly was not even ticked, and the guard only scanned players).
#[tokio::test]
async fn hostile_npcs_in_range_engage_each_other() {
    let mut mgr = pair(10.0);
    tick(&mut mgr).await;
    assert!(fighting(&mgr, FRIENDLY, HOSTILE), "the friendly engages");
    assert!(fighting(&mgr, HOSTILE, FRIENDLY), "the guard engages");
    let player = mgr.get_entity(PLAYER).unwrap();
    assert!(
        player.threatened_mobs.is_empty(),
        "an NPC fight puts no player in combat"
    );
}

/// The acquisition row names the target's kind and both factions.
#[tokio::test]
async fn the_acquired_row_names_the_npc_target_and_both_factions() {
    let mut mgr = pair(10.0);
    let logs = LogCapture::install();
    tick(&mut mgr).await;
    let acquired: Vec<_> = rows(&logs, "acquired")
        .into_iter()
        .filter(|c| c.target == "npc_ai.aggro")
        .collect();
    let friendly_row = acquired
        .iter()
        .find(|c| c.has_field("npc_id", &FRIENDLY.to_string()))
        .expect("the friendly's acquisition row");
    for (k, v) in [
        ("cause", "proximity"),
        ("target_kind", "npc"),
        ("target_id", HOSTILE.to_string().as_str()),
        ("npc_faction", "3"),
        ("target_faction", "10"),
    ] {
        assert!(friendly_row.has_field(k, v), "{k}={v}: {friendly_row:?}");
    }
}

/// With no player to watch, no NPC fight starts: the NPC scan is gated on a
/// witness so a standoff in an empty zone costs nothing.
#[tokio::test]
async fn no_witness_no_npc_fight() {
    // The player stands 400 u away, outside the 150 u AoI of both NPCs.
    let mut mgr = space([0.0, 0.0, -400.0]);
    npc(&mut mgr, FRIENDLY, PRAXIS, [0.0, 0.0, 0.0]);
    npc(&mut mgr, HOSTILE, HOSTILE_FACTION, [10.0, 0.0, 0.0]);
    let _ = mgr.compute_aoi_changes();
    assert!(mgr.get_witnesses_of(FRIENDLY).is_empty(), "fixture");
    tick(&mut mgr).await;
    for id in [FRIENDLY, HOSTILE] {
        let e = mgr.get_entity(id).unwrap();
        assert_eq!(e.ai_state(), AiState::Idle, "NPC {id}");
        assert!(e.threat_list.is_empty(), "NPC {id}");
    }
}

/// A Praxis friendly never takes a player as a target, even at arm's
/// length, and a same-faction pair never fights. Neither writes an NPC
/// refusal row: only hostile pairs are candidates.
#[tokio::test]
async fn friendly_rows_never_engage_and_log_nothing() {
    let mut mgr = space([3.0, 0.0, 0.0]);
    npc(&mut mgr, FRIENDLY, PRAXIS, [0.0, 0.0, 0.0]);
    npc(&mut mgr, FRIENDLY + 10, PRAXIS, [5.0, 0.0, 0.0]);
    npc(&mut mgr, HOSTILE, HOSTILE_FACTION, [0.0, 0.0, 300.0]);
    npc(&mut mgr, HOSTILE + 10, HOSTILE_FACTION, [5.0, 0.0, 300.0]);
    let _ = mgr.compute_aoi_changes();
    let logs = LogCapture::install();
    tick(&mut mgr).await;
    for id in [FRIENDLY, FRIENDLY + 10, HOSTILE, HOSTILE + 10] {
        let e = mgr.get_entity(id).unwrap();
        assert!(e.threat_list.is_empty(), "NPC {id}: {:?}", e.threat_list);
    }
    assert!(rows(&logs, "npc_candidate_rejected").is_empty());
}

/// **Negative log.** A hostile NPC just outside the aggro radius (default
/// 18 u) is refused with one `npc_candidate_rejected` row carrying both ids,
/// both factions and `reason=out_of_radius`. With the reporter removed, a
/// friendly standing idle next to a guard is unexplainable from SigNoz.
#[tokio::test]
async fn a_hostile_npc_out_of_radius_is_refused_with_a_row() {
    let mut mgr = pair(25.0);
    let logs = LogCapture::install();
    tick(&mut mgr).await;
    assert!(mgr.get_entity(FRIENDLY).unwrap().threat_list.is_empty());
    let refused: Vec<_> = rows(&logs, "npc_candidate_rejected")
        .into_iter()
        .filter(|c| c.has_field("npc_id", &FRIENDLY.to_string()))
        .collect();
    assert_eq!(refused.len(), 1, "{refused:?}");
    let row = &refused[0];
    assert_eq!(row.level, tracing::Level::DEBUG);
    assert_eq!(row.target, "npc_ai.aggro_scan");
    for (k, v) in [
        ("reason", "out_of_radius"),
        ("target_id", HOSTILE.to_string().as_str()),
        ("npc_faction", "3"),
        ("target_faction", "10"),
        ("npc_to_target", "25.0"),
    ] {
        assert!(row.has_field(k, v), "{k}={v}: {row:?}");
    }
}

/// A guard walking home evades (NA12), and a surrendered one is out of the
/// fight: the friendly engages neither and says why. The friendly's scan runs
/// on its own, because the guard's own Leashing / Submit handler would move
/// it out of that state within a whole tick.
#[tokio::test]
async fn evading_and_surrendered_npcs_are_not_engaged() {
    for (state, reason) in [
        (AiState::Leashing, "target_evading"),
        (AiState::Submit, "target_unavailable"),
    ] {
        let mut mgr = pair(10.0);
        crate::cell::service::npc_ai::force_ai_state(mgr.get_entity_mut(HOSTILE).unwrap(), state);
        let logs = LogCapture::install();
        let (tx, _rx) = mpsc::channel(64);
        let engaged =
            crate::cell::service::npc_ai::npc_idle_aggro_scan_for_test(FRIENDLY, &tx, &mut mgr)
                .await;
        assert!(!engaged, "{state:?}");
        assert!(mgr.get_entity(FRIENDLY).unwrap().threat_list.is_empty());
        assert!(
            rows(&logs, "npc_candidate_rejected")
                .iter()
                .any(|c| c.has_field("target_id", &HOSTILE.to_string())
                    && c.has_field("reason", reason)),
            "{state:?} must log {reason}"
        );
    }
}

/// A guard that sees both a player and a friendly NPC engages the closer
/// one, either way round.
#[tokio::test]
async fn the_closer_of_a_player_and_an_npc_is_engaged() {
    for (player_z, npc_x, want) in [(12.0, 6.0, FRIENDLY), (6.0, 12.0, PLAYER)] {
        let mut mgr = space([0.0, 0.0, player_z]);
        npc(&mut mgr, HOSTILE, HOSTILE_FACTION, [0.0, 0.0, 0.0]);
        npc(&mut mgr, FRIENDLY, PRAXIS, [npc_x, 0.0, 0.0]);
        let _ = mgr.compute_aoi_changes();
        let (tx, _rx) = mpsc::channel(64);
        // Only the guard's scan: run the tick with the friendly disarmed so
        // its own scan cannot put the guard on a list first.
        mgr.get_entity_mut(FRIENDLY).unwrap().aggro.override_level =
            Some(cimmeria_entity::cell_entity::MobAggression::Neutral);
        crate::cell::service::npc_ai::npc_ai_tick(
            &tx,
            &mut mgr,
            &crate::cell::content::EngineEvents(
                &cimmeria_content_engine::chain::ChainEngine::new(),
            ),
        )
        .await;
        let guard = mgr.get_entity(HOSTILE).unwrap();
        let listed: Vec<u32> = guard.threat_list.keys().copied().collect();
        assert_eq!(listed, vec![want], "player at {player_z}, npc at {npc_x}");
    }
}

/// **Mixed threat table across an NPC death.** A guard fights a friendly NPC
/// (top threat) while a player it also threatens stands by. The guard
/// attacks the NPC; the NPC dies to a third party; the guard drops it at
/// once, keeps the player (still in combat), and its next pass (natural tick
/// or the fast-retry sweep, interleaved) attacks the player.
#[tokio::test]
async fn a_mixed_threat_table_survives_an_npc_death() {
    let mut mgr = space([0.0, 0.0, 12.0]);
    npc(&mut mgr, HOSTILE, HOSTILE_FACTION, [0.0, 0.0, 0.0]);
    npc(&mut mgr, FRIENDLY, PRAXIS, [8.0, 0.0, 0.0]);
    seed_default_ability(&mut mgr, 0, 30);
    let _ = mgr.compute_aoi_changes();
    {
        let g = mgr.get_entity_mut(HOSTILE).unwrap();
        g.threat_list.insert(PLAYER, 5.0);
        g.threat_list.insert(FRIENDLY, 50.0);
        crate::cell::service::npc_ai::force_ai_state(g, AiState::Fighting);
    }
    let _ = crate::cell::combat::enter_player_combat(&mut mgr, PLAYER, HOSTILE);

    let logs = LogCapture::install();
    tick(&mut mgr).await;
    let attacked = |logs: &LogCaptureGuard, target: u32| {
        logs.all().iter().any(|c| {
            c.has_field("decision_outcome", "attack_in_place")
                && c.has_field("npc_id", &HOSTILE.to_string())
                && c.has_field("target_id", &target.to_string())
        })
    };
    assert!(attacked(&logs, FRIENDLY), "top threat is the NPC");
    assert!(!attacked(&logs, PLAYER));
    drop(logs);

    // A third party (another NPC) finishes the friendly.
    let (tx, _rx) = mpsc::channel(1024);
    let killer = HOSTILE + 50;
    npc(&mut mgr, killer, HOSTILE_FACTION, [20.0, 0.0, 0.0]);
    assert!(
        crate::cell::abilities::kill_npc_out_of_band(FRIENDLY, killer, false, true, &tx, &mut mgr)
            .await
    );
    {
        let g = mgr.get_entity(HOSTILE).unwrap();
        assert!(!g.threat_list.contains_key(&FRIENDLY), "dropped at death");
        assert!(g.threat_list.contains_key(&PLAYER), "the player stays");
        assert_eq!(g.ai_state(), AiState::Fighting);
        let p = mgr.get_entity(PLAYER).unwrap();
        assert!(
            p.threatened_mobs.contains(&HOSTILE),
            "player still in combat"
        );
        assert_ne!(p.state_field & crate::cell::combat::BSF_IN_COMBAT, 0);
    }

    // Interleave the fast-retry sweep with the natural tick, as the message
    // loop does; whichever runs, the guard now turns on the player.
    // The first pass put the ability on cooldown; clear it so the next pass
    // can fire rather than hold (`no_ability`).
    {
        let g = mgr.get_entity_mut(HOSTILE).unwrap();
        g.abilities = cimmeria_entity::abilities::AbilityManager::with_abilities(
            &g.abilities.known_ability_ids(),
        );
    }
    let logs = LogCapture::install();
    let events =
        crate::cell::content::EngineEvents(&cimmeria_content_engine::chain::ChainEngine::new());
    mgr.get_entity_mut(HOSTILE).unwrap().ai_retry_at = Some(std::time::Instant::now());
    mgr.pending_ai_retries.insert(HOSTILE);
    crate::cell::service::npc_ai::npc_ai_retry_sweep(&tx, &mut mgr, &events).await;
    tick(&mut mgr).await;
    assert!(attacked(&logs, PLAYER), "the guard turns on the player");
    assert!(!attacked(&logs, FRIENDLY), "never on the corpse");
}

/// **No credit for an NPC-only kill, through the real fight tick.** A
/// friendly kills a tagged guard (the shape of a KillCount target) with a
/// lethal shot: the guard dies, and nothing is paid: no `EntityDeath`
/// content event (so no kill objective moves), no `GrantXP`, no loot on the
/// corpse although its table always drops.
#[tokio::test]
async fn an_npc_only_kill_through_the_tick_pays_nothing() {
    use crate::cell::messages::CellToBaseMsg;
    use crate::test_support::{RecordedContentEvent, RecordingContentEvents};
    use cimmeria_entity::abilities::EffectDef;

    const LOOT_TABLE: i32 = 0x7000_1010;
    const EFFECT_ID: i32 = 0x7000_1011;
    let mut mgr = pair(8.0);
    seed_default_ability(&mut mgr, 0, 30);
    let mut params = std::collections::HashMap::new();
    params.insert("HealthDamage".to_string(), "9999".to_string());
    mgr.effect_defs.insert(
        EFFECT_ID,
        EffectDef {
            effect_id: EFFECT_ID,
            ability_id: crate::cell::combat::NPC_DEFAULT_ABILITY,
            params,
            ..Default::default()
        },
    );
    mgr.ability_defs
        .get_mut(&crate::cell::combat::NPC_DEFAULT_ABILITY)
        .unwrap()
        .effect_ids = vec![EFFECT_ID];
    mgr.loot_tables.insert(
        LOOT_TABLE,
        vec![crate::cell::spawner::LootTableEntry {
            design_id: Some(7),
            min_quantity: 1,
            max_quantity: 1,
            probability: 1.0,
        }],
    );
    {
        let g = mgr.get_entity_mut(HOSTILE).unwrap();
        g.tag = Some("CIMMERIA_TEST_1009_KILLCOUNT_GUARD".to_string());
        g.level = 5;
        g.loot_table_id = Some(LOOT_TABLE);
        // Disarmed so only the friendly fires this tick.
        g.aggro.override_level = Some(cimmeria_entity::cell_entity::MobAggression::Neutral);
    }
    {
        let f = mgr.get_entity_mut(FRIENDLY).unwrap();
        f.threat_list.insert(HOSTILE, 10.0);
        crate::cell::service::npc_ai::force_ai_state(f, AiState::Fighting);
    }

    let recorder = RecordingContentEvents::new();
    let (tx, mut rx) = mpsc::channel(1024);
    for _ in 0..20 {
        crate::cell::service::npc_ai::npc_ai_tick(&tx, &mut mgr, &recorder).await;
        if mgr.get_entity(HOSTILE).unwrap().ai_state() == AiState::Dead {
            break;
        }
        // A miss leaves the ability cooling; clear it and shoot again.
        let f = mgr.get_entity_mut(FRIENDLY).unwrap();
        f.abilities = cimmeria_entity::abilities::AbilityManager::with_abilities(
            &f.abilities.known_ability_ids(),
        );
    }

    let corpse = mgr.get_entity(HOSTILE).unwrap();
    assert_eq!(
        corpse.ai_state(),
        AiState::Dead,
        "fixture: the friendly kills"
    );
    assert!(corpse.loot.is_empty(), "no loot for an NPC-only kill");
    let deaths: Vec<_> = recorder
        .events()
        .into_iter()
        .filter(|e| matches!(e, RecordedContentEvent::EntityDeath { .. }))
        .collect();
    assert!(deaths.is_empty(), "no kill objective moves: {deaths:?}");
    while let Ok(m) = rx.try_recv() {
        assert!(
            !matches!(m, CellToBaseMsg::GrantXP { .. }),
            "no XP for an NPC-only kill: {m:?}"
        );
    }
}

/// **Guard: a corpse gets no turn in the tick it died.** Two NPCs fight each
/// other with a lethal shot, so whichever the tick visits first kills the
/// other. The dead one must stay Dead: before #1009 the dispatcher ran every
/// NPC with the state it had at the start of the tick, so the corpse got a
/// Fighting turn, found its (cleared) threat list empty and walked home, from
/// Dead to Leashing. Which of the two dies depends on the tick's order; the
/// assertions hold for either.
#[tokio::test]
async fn an_npc_killed_by_another_npc_gets_no_turn_that_tick() {
    use cimmeria_entity::abilities::EffectDef;

    const EFFECT_ID: i32 = 0x7000_1012;
    let mut mgr = pair(8.0);
    seed_default_ability(&mut mgr, 0, 30);
    let mut params = std::collections::HashMap::new();
    params.insert("HealthDamage".to_string(), "9999".to_string());
    mgr.effect_defs.insert(
        EFFECT_ID,
        EffectDef {
            effect_id: EFFECT_ID,
            ability_id: crate::cell::combat::NPC_DEFAULT_ABILITY,
            params,
            ..Default::default()
        },
    );
    mgr.ability_defs
        .get_mut(&crate::cell::combat::NPC_DEFAULT_ABILITY)
        .unwrap()
        .effect_ids = vec![EFFECT_ID];
    for (me, them) in [(FRIENDLY, HOSTILE), (HOSTILE, FRIENDLY)] {
        let e = mgr.get_entity_mut(me).unwrap();
        e.threat_list.insert(them, 10.0);
        crate::cell::service::npc_ai::force_ai_state(e, AiState::Fighting);
    }

    let (tx, _rx) = mpsc::channel(1024);
    let events =
        crate::cell::content::EngineEvents(&cimmeria_content_engine::chain::ChainEngine::new());
    // A miss is possible; shoot again until someone dies.
    for _ in 0..20 {
        crate::cell::service::npc_ai::npc_ai_tick(&tx, &mut mgr, &events).await;
        let dead = [FRIENDLY, HOSTILE]
            .into_iter()
            .filter(|&id| {
                mgr.get_entity(id)
                    .unwrap()
                    .stats
                    .get(HEALTH)
                    .is_some_and(|h| h.cur <= 0)
            })
            .count();
        if dead > 0 {
            break;
        }
        for id in [FRIENDLY, HOSTILE] {
            let e = mgr.get_entity_mut(id).unwrap();
            e.abilities = cimmeria_entity::abilities::AbilityManager::with_abilities(
                &e.abilities.known_ability_ids(),
            );
        }
    }

    let states: Vec<(u32, AiState, i32)> = [FRIENDLY, HOSTILE]
        .into_iter()
        .map(|id| {
            let e = mgr.get_entity(id).unwrap();
            (id, e.ai_state(), e.stats.get(HEALTH).unwrap().cur)
        })
        .collect();
    let corpses: Vec<_> = states.iter().filter(|(_, _, hp)| *hp <= 0).collect();
    assert_eq!(corpses.len(), 1, "exactly one dies: {states:?}");
    assert_eq!(
        corpses[0].1,
        AiState::Dead,
        "the corpse must stay Dead, not take a turn: {states:?}"
    );
}
