//! The NPC-vs-NPC CPU budget (#1009): the target scan is a bounded grid
//! query, and a Castle-sized standoff (10 vs 10) costs the AI tick a bounded
//! amount.
//!
//! - [`the_npc_scan_is_a_bounded_grid_query`] runs in CI: with 400 more
//!   hostile NPCs elsewhere in the space, a standoff NPC's scan still looks at
//!   the standoff and nothing else. It fails if the scan becomes a sweep of
//!   the space (`npc_ids_in_space_of`), which makes the tick O(n^2).
//! - [`bench_npc_vs_npc_tick_cost`] is `#[ignore]`d: it prints the per-tick
//!   cost of the scenarios for the PR. Run it in release:
//!   `cargo nextest run -p cimmeria-cell --release --run-ignored only bench_npc_vs_npc`
//!   (through the build lane) and read the `--no-capture` output.

use std::time::{Duration, Instant};

use cimmeria_entity::cell_entity::AiState;
use cimmeria_entity::stats::HEALTH;
use tokio::sync::mpsc;

use super::seed_default_ability;
use crate::cell::combat::HOSTILE_FACTION;
use crate::cell::space_manager::SpaceManager;

const PLAYER: u32 = 1;
const PRAXIS: u8 = 3;
/// First id of the standoff; ids `BASE..BASE + 20`.
const BASE: u32 = 300_000;
/// First id of the far-away crowd.
const CROWD: u32 = 310_000;

fn space() -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    mgr.parse_spaces_xml(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="-2000" MaxX="2000" MinY="-2000" MaxY="2000" /></Spaces>"#,
    )
    .unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#,
    )
    .unwrap();
    mgr.create_entity(PLAYER, "Castle", [0.0, 0.0, -60.0], [0.0; 3])
        .unwrap();
    {
        let p = mgr.get_entity_mut(PLAYER).unwrap();
        p.is_player = true;
        p.player_id = Some(PLAYER as i32);
    }
    mgr.connect_entity(PLAYER);
    seed_default_ability(&mut mgr, 0, 30);
    mgr
}

fn npc(mgr: &mut SpaceManager, id: u32, faction: u8, pos: [f32; 3]) {
    mgr.spawn_npc(id, "Castle", pos, [0.0; 3]).unwrap();
    let e = mgr.get_entity_mut(id).unwrap();
    e.faction = faction;
    crate::cell::service::npc_ai::force_ai_state(e, AiState::Idle);
    // Enough health that nobody dies while the benchmark runs.
    let h = e.stats.get_mut(HEALTH).unwrap();
    h.update(0, 1_000_000, 1_000_000);
    h.clear_dirty();
}

/// Ten NPCs of `west` two columns wide at x = -6 and ten of `east` at x = 6,
/// 3 u apart in z: every pair across the line is inside the default 18 u
/// aggro radius, the shape of the Castle courtyard standoff. With `crowd`,
/// 400 more Praxis NPCs stand in a grid 300-700 u away.
fn standoff(west: u8, east: u8, crowd: bool) -> SpaceManager {
    let mut mgr = space();
    for i in 0..10u32 {
        let z = (i as f32 - 4.5) * 3.0;
        npc(&mut mgr, BASE + i, west, [-6.0, 0.0, z]);
        npc(&mut mgr, BASE + 10 + i, east, [6.0, 0.0, z]);
    }
    if crowd {
        for i in 0..400u32 {
            let (gx, gz) = ((i % 20) as f32, (i / 20) as f32);
            npc(
                &mut mgr,
                CROWD + i,
                PRAXIS,
                [300.0 + gx * 20.0, 0.0, 300.0 + gz * 20.0],
            );
        }
    }
    let _ = mgr.compute_aoi_changes();
    mgr
}

async fn tick(mgr: &mut SpaceManager) {
    let (tx, mut rx) = mpsc::channel(65_536);
    crate::cell::service::npc_ai::npc_ai_tick(
        &tx,
        mgr,
        &crate::cell::content::EngineEvents(&cimmeria_content_engine::chain::ChainEngine::new()),
    )
    .await;
    while rx.try_recv().is_ok() {}
}

/// **Guard.** A standoff NPC's scan evaluates the other 19 standoff NPCs and
/// none of the 400 in the far crowd. A sweep of the space would hand it 419.
#[test]
fn the_npc_scan_is_a_bounded_grid_query() {
    let mgr = standoff(PRAXIS, HOSTILE_FACTION, true);
    let candidates = crate::cell::service::npc_ai::npc_scan_candidates_for_test(&mgr, BASE);
    assert!(
        candidates.iter().all(|&c| c < CROWD),
        "the far crowd must never be scanned: {} candidates",
        candidates.len()
    );
    assert_eq!(candidates.len(), 19, "the other 19 standoff NPCs");
}

/// Mean wall time of `n` ticks of `mgr`. `before` runs before each timed
/// tick, untimed.
async fn mean_tick(mgr: &mut SpaceManager, n: u32, before: fn(&mut SpaceManager)) -> Duration {
    let mut total = Duration::ZERO;
    for _ in 0..n {
        before(mgr);
        let t = Instant::now();
        tick(mgr).await;
        total += t.elapsed();
    }
    total / n
}

/// Clear every standoff NPC's cooldowns so each tick is a firing tick.
fn clear_cooldowns(mgr: &mut SpaceManager) {
    for i in 0..20 {
        let e = mgr.get_entity_mut(BASE + i).unwrap();
        e.abilities = cimmeria_entity::abilities::AbilityManager::with_abilities(
            &e.abilities.known_ability_ids(),
        );
    }
}

/// Prints the per-tick cost of five scenarios, each without and with a crowd
/// of 400 Praxis NPCs (which fight NPCs) standing 300-700 u away, unwatched:
///
/// - `idle_same_faction`: 20 faction-10 guards and a witness, nobody to
///   fight. What every existing world pays for the NPC scan.
/// - `idle_no_npc_enemies`: 20 faction-1 NPCs, which do not scan at all.
/// - `guards_vs_player`: the 20 guards fighting one player, every guard
///   firing every pass. The pre-#1009 cost of a 20-NPC fight, for scale.
/// - `standoff_acquire`: the first tick of a 10 vs 10 standoff, where all 20
///   acquire a target.
/// - `standoff_fighting`: steady-state ticks of that standoff, every NPC
///   firing every pass (cooldowns cleared between ticks).
#[tokio::test]
#[ignore = "benchmark: run in release with --run-ignored, see the module doc"]
async fn bench_npc_vs_npc_tick_cost() {
    const N: u32 = 200;
    let mut report = format!(
        "npc_vs_npc_tick_cost (mean of {N} ticks)
"
    );
    report += &format!(
        "{:<22}{:>14}{:>18}
",
        "scenario", "20 NPCs", "+400 far crowd"
    );
    let mut rows: Vec<(&str, [Duration; 2])> = Vec::new();
    for (col, crowd) in [false, true].into_iter().enumerate() {
        let mut idle = standoff(HOSTILE_FACTION, HOSTILE_FACTION, crowd);
        let a = mean_tick(&mut idle, N, |_| {}).await;

        let mut inert = standoff(1, 1, crowd);
        let b = mean_tick(&mut inert, N, |_| {}).await;

        let mut guards = standoff(HOSTILE_FACTION, HOSTILE_FACTION, crowd);
        {
            let p = guards.get_entity_mut(PLAYER).unwrap();
            p.position = cimmeria_common::Vector3::new(0.0, 0.0, 8.0);
            let h = p.stats.get_mut(HEALTH).unwrap();
            h.update(0, 1_000_000, 1_000_000);
        }
        for i in 0..20 {
            let e = guards.get_entity_mut(BASE + i).unwrap();
            e.threat_list.insert(PLAYER, 10.0);
            crate::cell::service::npc_ai::force_ai_state(e, AiState::Fighting);
        }
        let c = mean_tick(&mut guards, N, clear_cooldowns).await;

        let mut acquire_total = Duration::ZERO;
        for _ in 0..N {
            let mut mgr = standoff(PRAXIS, HOSTILE_FACTION, crowd);
            let t = Instant::now();
            tick(&mut mgr).await;
            acquire_total += t.elapsed();
        }
        let d = acquire_total / N;

        let mut fight = standoff(PRAXIS, HOSTILE_FACTION, crowd);
        tick(&mut fight).await;
        let engaged = (0..20)
            .filter(|i| fight.get_entity(BASE + i).unwrap().ai_state() == AiState::Fighting)
            .count();
        assert_eq!(engaged, 20, "fixture: the whole standoff is fighting");
        let e = mean_tick(&mut fight, N, clear_cooldowns).await;

        for (k, (name, v)) in [
            ("idle_same_faction", a),
            ("idle_no_npc_enemies", b),
            ("guards_vs_player", c),
            ("standoff_acquire", d),
            ("standoff_fighting", e),
        ]
        .into_iter()
        .enumerate()
        {
            if col == 0 {
                rows.push((name, [v, Duration::ZERO]));
            } else {
                rows[k].1[1] = v;
            }
        }
    }
    for (name, [a, b]) in rows {
        report += &format!(
            "{name:<22}{:>14?}{:>18?}
",
            a, b
        );
    }
    println!("{report}");
}
