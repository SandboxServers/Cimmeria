//! Castle defense_q1 sentry (mission 10310), CS-04b1c1: the seeded shape of
//! chains 10100-10106, the slot and tag bindings, and the placement of the
//! giver (template 419, spawn 13808). The replay of the click and the region
//! entry is in `castle_side_defense_q1`.

use cimmeria_content_engine::actions::Action;
use cimmeria_content_engine::conditions::{
    ComparisonOp, Condition, MissionStatusValue, StepStatusValue,
};

use super::super::engine_loader::load_single_chain_for_test;
use crate::cell::spawner::load_spawns_from_db;
use crate::test_support::require_db_or_skip;

const PREREQ: i32 = 10300;
const MISSION: i32 = 10310;
const STEP_A: i32 = 10354;
const STEP_B: i32 = 10355;
const OBJ_B: i32 = 10475;
/// The sentry offer: dialog-set map 1000006 bound at slot 419.
const OFFER_SET: i32 = 1000006;
const OFFER_SLOT: i32 = 419;
const DIALOG: i32 = 10709;
/// The giver: template 419, spawn 13808, tag `Castle_DefenseQ1_Sentry`.
const GIVER_TEMPLATE: i32 = 419;
const GIVER_SPAWN: i32 = 13808;
const GIVER_TAG: &str = "Castle_DefenseQ1_Sentry";
/// `Castle.Infirmary` (point set 2051): the box step B completes in.
const INFIRMARY_X: (f32, f32) = (344.9, 371.0);
const INFIRMARY_Z: (f32, f32) = (880.2, 904.6);
/// Mirrors `cimmeria_cell_world::cell::combat::aggression::{DEFAULT_AGGRO_RADIUS,
/// AGGRO_VERTICAL_BAND}`, which this crate cannot depend on.
const DEFAULT_AGGRO_RADIUS: f32 = 18.0;
const AGGRO_VERTICAL_BAND: f32 = 4.0;
/// The friendlies rule of `live_db_castle_population`: stand this far outside
/// a hostile's aggro radius.
const FRIENDLY_CLEARANCE: f32 = 5.0;
/// Debug text of a `world = 8` condition.
fn world_8() -> String {
    format!(
        "{:?}",
        Condition::World {
            operator: ComparisonOp::Eq,
            world_id: 8
        }
    )
}

/// Debug text of a `mission_status` condition.
fn mission_is(mission_id: i32, expected_status: MissionStatusValue) -> String {
    format!(
        "{:?}",
        Condition::MissionStatus {
            mission_id,
            operator: ComparisonOp::Eq,
            expected_status
        }
    )
}

/// Debug text of a `step_status = active` condition on 10310.
fn step_active(step_id: i32) -> String {
    format!(
        "{:?}",
        Condition::StepStatus {
            mission_id: MISSION,
            step_id,
            operator: ComparisonOp::Eq,
            expected_status: StepStatusValue::Active
        }
    )
}

fn offer_bind() -> Action {
    Action::AddDialogSet {
        dialog_set_id: OFFER_SET,
        slot: OFFER_SLOT,
        mission_id: None,
    }
}

/// **Chains 10100-10106 load with the sealed trigger, every condition in
/// order and the exact action list.** Each chain is compared whole: trigger,
/// condition list (type, ids, operator, status), action list (type, ids,
/// slot, step) in sort order, plus enabled, priority 0, not `once` and no
/// delays. Revert proof: delete 10103 and the load fails; drop 10104's
/// `step_status 10354` condition or swap its action order and the comparison
/// fails.
#[tokio::test]
async fn live_db_castle_side_defense_q1_chains_load() {
    let pool = require_db_or_skip!();
    let offer_gate = || {
        vec![
            world_8(),
            mission_is(PREREQ, MissionStatusValue::Completed),
            mission_is(MISSION, MissionStatusValue::NotActive),
        ]
    };
    let active_at = |step| {
        vec![
            world_8(),
            mission_is(MISSION, MissionStatusValue::Active),
            step_active(step),
        ]
    };
    // (chain, trigger Debug, conditions in order, actions in order)
    let sealed: [(i32, &str, Vec<String>, Vec<Action>); 7] = [
        (
            10100,
            "OnMissionCompleted { mission_id: 10300 }",
            offer_gate(),
            vec![offer_bind()],
        ),
        (
            10101,
            "OnPlayerLoaded { world_name: Some(\"Castle\") }",
            offer_gate(),
            vec![offer_bind()],
        ),
        (
            10102,
            "OnMissionAbandoned { mission_id: 10310 }",
            offer_gate(),
            vec![offer_bind()],
        ),
        (
            10103,
            "OnInteractTag { entity_tag: \"Castle_DefenseQ1_Sentry\" }",
            offer_gate(),
            vec![Action::AcceptMission {
                mission_id: MISSION,
            }],
        ),
        (
            10104,
            "OnMissionAccepted { mission_id: 10310 }",
            vec![world_8(), step_active(STEP_A)],
            vec![
                Action::AdvanceStep {
                    mission_id: MISSION,
                    step_id: STEP_B,
                },
                Action::RemoveDialogSet {
                    dialog_set_id: OFFER_SET,
                    slot: OFFER_SLOT,
                },
                Action::DisplayDialog { dialog_id: DIALOG },
            ],
        ),
        (
            10105,
            "OnRegionEnter { region_key: \"Castle.Infirmary\" }",
            active_at(STEP_B),
            vec![Action::CompleteObjective {
                mission_id: MISSION,
                objective_id: OBJ_B,
            }],
        ),
        (
            10106,
            "OnPlayerLoaded { world_name: Some(\"Castle\") }",
            active_at(STEP_A),
            vec![Action::AdvanceStep {
                mission_id: MISSION,
                step_id: STEP_B,
            }],
        ),
    ];
    for (id, trigger, conditions, actions) in sealed {
        let chain = load_single_chain_for_test(&pool, id)
            .await
            .unwrap_or_else(|e| panic!("DB query for chain {id} must succeed: {e}"))
            .unwrap_or_else(|| panic!("chain {id} must exist in seeded content_chains"));
        assert!(chain.enabled, "chain {id} must be enabled");
        assert_eq!(chain.priority, 0, "chain {id} priority");
        assert!(!chain.once, "chain {id} must not be once");
        assert_eq!(
            format!("{:?}", chain.trigger),
            trigger,
            "chain {id} trigger"
        );
        let got: Vec<String> = chain.conditions.iter().map(|c| format!("{c:?}")).collect();
        assert_eq!(got, conditions, "chain {id} conditions in sort order");
        assert_eq!(chain.actions, actions, "chain {id} actions in sort order");
        assert!(
            chain.action_delays.iter().all(|&d| d == 0),
            "chain {id} must have no delays"
        );
    }
}

// ── Slot and tag binding ─────────────────────────────────────────────────

/// **Only the defense_q1 chains bind slot 419 or the sentry tag.** Against
/// the seeded tables: the chains whose actions name slot 419 are 10100,
/// 10101, 10102 (add) and 10104 (remove), and the only chain on the tag is
/// 10103. No other chain or map is pinned here. Revert proof: add a second
/// `add_dialog_set` on slot 419, or a second `interact_tag` row on the tag,
/// and the list gains an id.
#[tokio::test]
async fn live_db_castle_side_defense_q1_slot_419_only_bound_here() {
    let pool = require_db_or_skip!();
    let slot_chains: Vec<i32> = sqlx::query_scalar(
        "SELECT DISTINCT chain_id FROM resources.content_actions \
         WHERE params::jsonb->>'slot' = '419' ORDER BY chain_id",
    )
    .fetch_all(&pool)
    .await
    .expect("slot 419 query must succeed");
    assert_eq!(
        slot_chains,
        vec![10100, 10101, 10102, 10104],
        "only the defense_q1 offer, rebind and remove chains may name slot 419"
    );
    let tag_chains: Vec<i32> = sqlx::query_scalar(
        "SELECT chain_id FROM resources.content_triggers \
         WHERE event_key = 'Castle_DefenseQ1_Sentry' ORDER BY chain_id",
    )
    .fetch_all(&pool)
    .await
    .expect("tag query must succeed");
    assert_eq!(
        tag_chains,
        vec![10103],
        "only the accept chain may listen on the sentry tag"
    );
}

/// **The giver is one stationary faction-1 NPC, clear of the Infirmary box and
/// of every faction-10 hostile.** Spawn 13808 is outside `FRIENDLY_SPAWNS`, so
/// `live_db_castle_population` does not see it; this test carries the
/// clearance (aggro radius + 5 u, inside the 4 u height band, spawn and every
/// tenth of a patrol leg). Exactly one spawn of template 419 exists and the
/// tag is unique in `spawnlist`. Navmesh membership is
/// `crates/entity/tests/castle_navmesh.rs`; the component (not 468 or 116) is
/// an end-UAT row, not a test here.
/// Revert proof: add a second 419 spawn or reuse the tag and the counts fail;
/// move the spawn into the Infirmary box or beside a guard and the placement
/// assertions fail.
#[tokio::test]
async fn live_db_castle_side_defense_q1_giver() {
    let pool = require_db_or_skip!();
    let spawns = load_spawns_from_db(&pool)
        .await
        .expect("load_spawns_from_db must succeed");
    let of_template: Vec<_> = spawns
        .iter()
        .filter(|r| r.template_id == GIVER_TEMPLATE)
        .collect();
    assert_eq!(of_template.len(), 1, "exactly one spawn of template 419");
    let giver = of_template[0];
    assert_eq!(giver.spawn_id, GIVER_SPAWN);
    assert_eq!(
        spawns
            .iter()
            .filter(|r| r.tag.as_deref() == Some(GIVER_TAG))
            .count(),
        1,
        "the sentry tag must be unique in spawnlist"
    );
    assert_eq!(giver.tag.as_deref(), Some(GIVER_TAG));
    assert_eq!(giver.world_name, "Castle", "the giver is in World 8");
    assert_eq!(giver.faction, Some(1), "the giver is faction 1");
    assert!(giver.is_stationary, "the giver must not wander");
    let in_box = (INFIRMARY_X.0..=INFIRMARY_X.1).contains(&giver.x)
        && (INFIRMARY_Z.0..=INFIRMARY_Z.1).contains(&giver.z);
    assert!(
        !in_box,
        "the giver must stand outside Castle.Infirmary, or arrival completes step B"
    );
    for hostile in spawns
        .iter()
        .filter(|r| r.world_name == "Castle" && r.faction == Some(10))
    {
        let need = hostile.aggro_radius.unwrap_or(DEFAULT_AGGRO_RADIUS) + FRIENDLY_CLEARANCE;
        let mut idle = vec![[hostile.x, hostile.y, hostile.z]];
        let path = &hostile.patrol_path;
        for i in 0..path.len() {
            let (a, b) = (path[i], path[(i + 1) % path.len()]);
            for k in 0..=10 {
                let t = k as f32 / 10.0;
                idle.push([
                    a.x + (b.x - a.x) * t,
                    a.y + (b.y - a.y) * t,
                    a.z + (b.z - a.z) * t,
                ]);
            }
        }
        for p in idle {
            if (giver.y - p[1]).abs() > AGGRO_VERTICAL_BAND {
                continue;
            }
            let d = ((giver.x - p[0]).powi(2) + (giver.z - p[2]).powi(2)).sqrt();
            assert!(
                d >= need,
                "hostile spawn {} ({}) is {d:.1} u from the giver; need {need:.1}",
                hostile.spawn_id,
                hostile.template_name
            );
        }
    }
}
