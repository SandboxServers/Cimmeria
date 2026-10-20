//! CB-N04a — the Cellblock ambient cast (cells): spawn rows tagged
//! `Cellblock_Ambient_*` and bark chains 11010.. in
//! `castle_cellblock_chains.sql`. Placement, gates and copy are sealed in
//! `docs/analysis/cellblock-continuation/worknotes/cb-n04-seal.md`.
//!
//! - The bark tests pin each chain's phase, the two adjacent wrong states
//!   its gate must refuse, and `once` (the H52 step-activation replay
//!   re-fires mission-gated `enter_region` chains; `once` stops a second
//!   line in one visit).
//! - The placement test pins world, template and tag for every cast row,
//!   the static clearances the seal measured from every world-12 hostile
//!   (templates 4, 15 and 24) and every protected anchor, and that no cast
//!   row lies inside `Castle_Cellblock.Region1` (D-CC12).

use cimmeria_content_engine::actions::Action;
use cimmeria_content_engine::chain::{ChainEngine, ResolvedActions};
use cimmeria_content_engine::context::ExecutionContext;
use cimmeria_content_engine::triggers::{TriggerEvent, TriggerType};
use sqlx::PgPool;

use super::super::engine_loader::load_single_chain_for_test;
use super::assert_no_deferred_actions;
use crate::cell::spawner::region_contains_xz;
use crate::test_support::require_db_or_skip;

type Params = &'static [(&'static str, &'static str)];

struct Bark {
    chain_id: i32,
    region_key: &'static str,
    screen_id: i32,
    fires: Params,
    refuses: &'static [(&'static str, Params)],
}

/// Seal § Values. Region keys are byte-identical to `point_sets.sql`.
const BARKS: &[Bark] = &[Bark {
    chain_id: 11010,
    region_key: "Castle_Cellblock.Region2",
    screen_id: 200160,
    fires: &[("mission_639_step_2117_status", "active")],
    refuses: &[
        (
            "the phase is not reached",
            &[("mission_639_step_2117_status", "not_active")],
        ),
        (
            "the phase has passed",
            &[("mission_639_step_2117_status", "completed")],
        ),
    ],
}];

const SPEAKER: &str = "Op-CORE Soldier";
/// `CHAN_say`; the loader accepts no other channel.
const CHAN_SAY: u8 = 0;

/// Seal § Values: (tag, spawn id, template id).
const CAST: &[(&str, i32, i32)] = &[
    ("Cellblock_Ambient_Prisoner1", 520, 176),
    ("Cellblock_Ambient_Prisoner2", 521, 176),
];
/// Template 24 (`Barracks_Guard*`, assist radius 26): the measured minimum.
const GUARD_CLEARANCE_M: f64 = 117.0;
/// Templates 15 and 4: aggro radius 18 u plus 5 m.
const HOSTILE_CLEARANCE_M: f64 = 23.0;
const PROTECTED_CLEARANCE_M: f64 = 22.0;
const PROTECTED: &[(&str, [f64; 3])] = &[
    ("WakePod", [-334.23, 73.47, -228.03]),
    ("AmbernolVial", [-234.04, 66.52, -124.70]),
    ("Preparation_RingSwitch", [-193.92, 56.32, -152.16]),
    ("HackTheRings_Switch", [-218.08, 67.04, -122.72]),
    ("Cellblock_ArmoryRingSwitch", [-54.88, 26.08, -163.84]),
    ("Cellblock_UnusedRingSwitch", [-87.04, 46.24, -160.32]),
    ("Hallway05_Guard1", [-100.16, 24.67, -43.90]),
    ("Hallway05_Guard2", [-101.50, 24.67, -51.30]),
    ("MessHall_Guard1", [-96.25, 34.59, -91.59]),
    ("MessHall_Guard2", [-95.89, 34.59, -98.81]),
    ("Prisoner_329", [-289.81, 65.47, -113.14]),
    ("329_CellDoorButton", [-287.29, 67.26, -115.04]),
    ("ArmYourself_FrostBody", [-328.30, 73.47, -210.27]),
    ("ArmYourself_GuardBody", [-322.51, 73.47, -209.83]),
];

async fn engine_for(pool: &PgPool, chain_id: i32) -> ChainEngine {
    let chain = load_single_chain_for_test(pool, chain_id)
        .await
        .unwrap_or_else(|e| panic!("DB query for chain {chain_id} must succeed: {e}"))
        .unwrap_or_else(|| panic!("chain {chain_id} must exist and assemble"));
    let mut engine = ChainEngine::new();
    engine.register_chain(chain);
    engine
}

fn resolve_region_enter(engine: &ChainEngine, region_key: &str, params: Params) -> ResolvedActions {
    let mut ctx = ExecutionContext::new();
    ctx.set_param("region_key".to_string(), serde_json::json!(region_key));
    for (k, v) in params {
        ctx.set_param((*k).to_string(), serde_json::json!(*v));
    }
    let event = TriggerEvent {
        trigger_type: TriggerType::RegionEnter,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };
    engine.resolve_event(&event, &ctx)
}

#[tokio::test]
async fn live_db_cellblock_ambience_barks_fire_once_in_their_phase() {
    let pool = require_db_or_skip!();
    for bark in BARKS {
        let id = i64::from(bark.chain_id);
        let engine = engine_for(&pool, bark.chain_id).await;
        assert!(
            engine.is_once(id),
            "chain {id} must be once = true (H52 replay)"
        );
        let resolved = resolve_region_enter(&engine, bark.region_key, bark.fires);
        let barks: Vec<&Action> = resolved
            .actions
            .iter()
            .filter(|(c, a)| *c == id && matches!(a, Action::NpcBark { .. }))
            .map(|(_, a)| a)
            .collect();
        assert_eq!(
            barks.len(),
            1,
            "chain {id} must resolve one NpcBark: {:?}",
            resolved.actions
        );
        match barks[0] {
            Action::NpcBark {
                screen_id,
                speaker,
                channel,
            } => {
                assert_eq!(
                    *screen_id, bark.screen_id,
                    "chain {id} speaks the wrong screen"
                );
                assert_eq!(speaker, SPEAKER, "chain {id} has the wrong chat prefix");
                assert_eq!(*channel, CHAN_SAY, "chain {id} must ride CHAN_say");
            }
            other => panic!("filtered for NpcBark but got {other:?}"),
        }
        assert_no_deferred_actions(&resolved, id);
    }
}

#[tokio::test]
async fn live_db_cellblock_ambience_barks_refuse_adjacent_states() {
    let pool = require_db_or_skip!();
    for bark in BARKS {
        let id = i64::from(bark.chain_id);
        let engine = engine_for(&pool, bark.chain_id).await;
        for (why, params) in bark.refuses {
            let resolved = resolve_region_enter(&engine, bark.region_key, params);
            assert!(
                !resolved.actions.iter().any(|(c, _)| *c == id),
                "chain {id} must not resolve when {why}: {:?}",
                resolved.actions
            );
        }
    }
}

#[tokio::test]
async fn live_db_cellblock_ambience_cast_rows_are_placed_safely() {
    let pool = require_db_or_skip!();
    let rows: Vec<(i32, String, i32, i32, bool, f64, f64, f64)> = sqlx::query_as(
        "SELECT spawn_id::int4, tag, template_id::int4, world_id::int4, \
         COALESCE(is_stationary, false), \
         x::float8, y::float8, z::float8 FROM resources.spawnlist \
         WHERE tag LIKE 'Cellblock\\_Ambient\\_%' ORDER BY spawn_id",
    )
    .fetch_all(&pool)
    .await
    .expect("spawnlist scan must succeed");
    assert_eq!(
        rows.len(),
        CAST.len(),
        "exactly the sealed cast rows: {rows:?}"
    );

    // Hostiles 15 (spawn 20) and 4 (spawn 10) are the nearest; 24 is the
    // Barracks guard set. Each row is checked against its own floor.
    let hostiles: Vec<(i32, f64, f64, f64)> = sqlx::query_as(
        "SELECT template_id::int4, x::float8, y::float8, z::float8 FROM resources.spawnlist \
         WHERE world_id = 12 AND template_id IN (4, 15, 24)",
    )
    .fetch_all(&pool)
    .await
    .expect("hostile scan must succeed");
    for template in [4, 15, 24] {
        assert!(
            hostiles.iter().any(|h| h.0 == template),
            "no world-12 template {template} hostile: the seed did not load"
        );
    }

    let region1: Vec<[f32; 3]> = sqlx::query_as::<_, (f32, f32, f32)>(
        "SELECT x, y, z FROM resources.point_set_points \
         WHERE set_id = 2032 ORDER BY point_id",
    )
    .fetch_all(&pool)
    .await
    .expect("Region1 (set 2032) must load")
    .into_iter()
    .map(|(x, y, z)| [x, y, z])
    .collect();
    assert_eq!(region1.len(), 4, "Region1 is the four-corner set 2032");

    let dist = |a: [f64; 3], b: [f64; 3]| {
        ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
    };
    for ((spawn_id, tag, template, world, stationary, x, y, z), (want_tag, want_id, want_tpl)) in
        rows.iter().zip(CAST)
    {
        assert_eq!(
            (spawn_id, tag.as_str(), template, world),
            (want_id, *want_tag, want_tpl, &12)
        );
        assert!(
            *stationary,
            "{tag} must be is_stationary (D-CC12: no path, no cover)"
        );
        let at = [*x, *y, *z];
        for (template, hx, hy, hz) in &hostiles {
            let floor = if *template == 24 {
                GUARD_CLEARANCE_M
            } else {
                HOSTILE_CLEARANCE_M
            };
            let d = dist(at, [*hx, *hy, *hz]);
            assert!(
                d >= floor,
                "{tag} is {d:.1} m from template {template} (floor {floor} m)"
            );
        }
        assert!(
            !region_contains_xz(&region1, *x as f32, *z as f32),
            "{tag} lies inside Castle_Cellblock.Region1 (D-CC12)"
        );
        for (name, anchor) in PROTECTED {
            let d = dist(at, *anchor);
            assert!(d >= PROTECTED_CLEARANCE_M, "{tag} is {d:.1} m from {name}");
        }
    }
}
