//! Castle return_q2 side story (mission 10321), CS-04c2c2: the lifecycle and
//! refusal replays of chains 10140-10146 that `castle_side_return_q2.rs`
//! does not cover. Relog, abandon and forged dialog events go through the
//! real dispatchers and the real abandon, so a resolve-only check cannot
//! pass a broken executor arm.
//!
//! Each test names its revert proof on its doc comment. The seed shape of the
//! seven chains is in `castle_side_return_q2_chains.rs`. The harness here is
//! shared with `castle_side_return_q2_worlds.rs` (sibling disjointness, the
//! G9 world sweep and the crossing guard).

use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_entity::missions::{
    MissionInstance, MissionObjective, MISSION_COMPLETED, STATUS_ACTIVE,
};
use sqlx::PgPool;
use tokio::sync::mpsc;
use tracing::Level;

use super::super::engine_loader::load_single_chain_for_test;
use super::super::{fire_dialog_choice, fire_dialog_open, fire_interact_tag, fire_player_loaded};
use crate::cell::content::fire_mission_abandoned;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::missions::abandon_mission;
use crate::cell::space_manager::SpaceManager;
use crate::cell::spawner::{
    load_dialog_set_maps, load_item_containers, load_mission_defs, load_monologue_dialog_ids,
    load_step_objectives, load_world_rows,
};
use crate::test_support::{require_db_or_skip, LogCapture};

pub(super) const ENTITY: u32 = 1;
pub(super) const PLAYER: i32 = 42;
/// The Moh'katan NPC, the interaction target of the 10142 click.
const MOHKATAN: u32 = 4;
pub(super) const CASTLE: &str = "Castle";
/// `resources.worlds` row 1; any world other than 8 refuses the accept.
const COMBAT_SIM: &str = "CombatSim";
pub(super) const MOHKATAN_TAG: &str = "Castle_Mohkatan";
pub(super) const MISSION: i32 = 10321;
pub(super) const PREREQ: i32 = 10320;
pub(super) const STEP_A: i32 = 10378;
pub(super) const STEP_B: i32 = 10379;
const OBJ_A: i32 = 10498;
const OBJ_B: i32 = 10499;
/// The Moh'katan offer: dialog-set map 1000014 bound at slot 54.
pub(super) const OFFER_SET: i32 = 1000014;
pub(super) const OFFER_SLOT: i32 = 54;
const DIALOG: i32 = 10725;
/// The accept chain, the only one on the Moh'katan tag that accepts 10321.
const ACCEPT_CHAIN: i64 = 10142;
/// Every chain of the return_q2 side story, 10140-10146.
pub(super) const ALL_CHAINS: [i32; 7] = [10140, 10141, 10142, 10143, 10144, 10145, 10146];
/// The worlds a player is staged in: Castle (world 8), CombatSim (1) and the
/// other worlds that spawn a template 10 or 54 NPC (G9): Castle_CellBlock
/// (12), Harset_CmdCenter (68), Dakara_E1_StoryRm (62), and Harset (57), the
/// origin of the Harset to Castle crossing.
pub(super) const WORLDS: [&str; 6] = [
    CASTLE,
    COMBAT_SIM,
    "Castle_CellBlock",
    "Harset_CmdCenter",
    "Dakara_E1_StoryRm",
    "Harset",
];

// ── Harness ──────────────────────────────────────────────────────────────

pub(super) async fn engine_with(pool: &PgPool, ids: &[i32]) -> ChainEngine {
    let mut engine = ChainEngine::new();
    for &id in ids {
        let chain = load_single_chain_for_test(pool, id)
            .await
            .unwrap_or_else(|e| panic!("DB query for chain {id} must succeed: {e}"))
            .unwrap_or_else(|| panic!("chain {id} must exist in seeded content_chains"));
        engine.register_chain(chain);
    }
    engine
}

/// The player and the Moh'katan actor in `world` (one of [`WORLDS`]), holding
/// `missions`, with the production caches loaded from the seed.
pub(super) async fn staged(
    pool: &PgPool,
    world: &str,
    missions: Vec<MissionInstance>,
) -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    let spaces: String = WORLDS
        .iter()
        .map(|w| {
            format!(
                r#"<Space WorldName="{w}" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" />"#
            )
        })
        .collect();
    mgr.parse_spaces_xml(&format!("<Spaces>{spaces}</Spaces>"))
        .unwrap();
    let startup: String = WORLDS
        .iter()
        .map(|w| format!(r#"<Space WorldName="{w}" />"#))
        .collect();
    mgr.create_startup_spaces(&format!("<Spaces>{startup}</Spaces>"))
        .unwrap();
    mgr.stamp_world_rows(&load_world_rows(pool).await.expect("world cache"));
    mgr.mission_defs = load_mission_defs(pool).await.expect("mission cache");
    mgr.step_objectives = load_step_objectives(pool).await.expect("objective cache");
    mgr.item_containers = load_item_containers(pool).await.expect("item cache");
    mgr.dialog_set_maps = load_dialog_set_maps(pool).await.expect("dialog set maps");
    mgr.monologue_dialog_ids = load_monologue_dialog_ids(pool)
        .await
        .expect("monologue ids");
    mgr.create_entity(MOHKATAN, world, [3.0; 3], [0.0; 3])
        .unwrap();
    mgr.create_entity(ENTITY, world, [0.0; 3], [0.0; 3])
        .unwrap();
    let entity = mgr.get_entity_mut(ENTITY).unwrap();
    entity.is_player = true;
    entity.player_id = Some(PLAYER);
    entity.archetype_id = Some(1);
    for mission in missions {
        entity.missions.add_mission(mission);
    }
    mgr
}

/// 10320 (the Marsh return report) as the player holds it.
#[derive(Clone, Copy, Debug)]
pub(super) enum Prereq {
    Absent,
    Active,
    Completed,
}

/// 10321 as the player holds it.
#[derive(Clone, Copy, Debug)]
pub(super) enum Own {
    Absent,
    AtA,
    AtB,
    Done,
}

pub(super) fn missions(prereq: Prereq, own: Own) -> Vec<MissionInstance> {
    let mut out = Vec::new();
    match prereq {
        Prereq::Absent => {}
        Prereq::Active => out.push(MissionInstance::new(PREREQ, 0, Vec::new())),
        Prereq::Completed => out.push(completed_prereq()),
    }
    match own {
        Own::Absent => {}
        Own::AtA => out.push(missions_at(STEP_A, OBJ_A)),
        Own::AtB => out.push(missions_at(STEP_B, OBJ_B)),
        Own::Done => {
            let mut done = missions_at(STEP_B, OBJ_B);
            done.status = MISSION_COMPLETED;
            done.current_step_id = None;
            out.push(done);
        }
    }
    out
}

/// 10320 completed: the prerequisite of the offer.
pub(super) fn completed_prereq() -> MissionInstance {
    let mut mission = MissionInstance::new(PREREQ, 0, Vec::new());
    mission.status = MISSION_COMPLETED;
    mission.current_step_id = None;
    mission
}

/// 10321 active at `step` with `objective`, as the accept leaves it.
fn missions_at(step: i32, objective: i32) -> MissionInstance {
    let mut mission = MissionInstance::new(
        MISSION,
        step,
        vec![MissionObjective {
            objective_id: objective,
            status: STATUS_ACTIVE,
            hidden: false,
            optional: false,
        }],
    );
    mission.current_step_id = Some(step);
    mission
}

pub(super) fn mission_of(mgr: &SpaceManager, mission_id: i32) -> Option<&MissionInstance> {
    mgr.get_entity(ENTITY)
        .unwrap()
        .missions
        .get_mission(mission_id)
}

pub(super) fn step_of(mgr: &SpaceManager) -> Option<i32> {
    mission_of(mgr, MISSION).and_then(|m| m.current_step_id)
}

pub(super) fn drain(rx: &mut mpsc::Receiver<CellToBaseMsg>) -> Vec<CellToBaseMsg> {
    let mut out = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        out.push(msg);
    }
    out
}

/// The dialog-set-map ids bound on the player at `slot`, ascending.
pub(super) fn bound_at(mgr: &SpaceManager, slot: i32) -> Vec<i32> {
    let mut ids: Vec<i32> = mgr
        .get_entity(ENTITY)
        .unwrap()
        .available_interactions
        .get(&slot)
        .map(|v| v.iter().map(|&(id, _, _)| id).collect())
        .unwrap_or_default();
    ids.sort_unstable();
    ids
}

/// Everything the return_q2 story can change on the player, as one value:
/// 10321's status and step, the offer binding and the dialogs on screen.
fn snapshot(mgr: &SpaceManager) -> (Option<(i8, Option<i32>)>, Vec<i32>, Vec<i32>) {
    let entity = mgr.get_entity(ENTITY).unwrap();
    let q2 = entity
        .missions
        .get_mission(MISSION)
        .map(|m| (m.status, m.current_step_id));
    (q2, bound_at(mgr, OFFER_SLOT), entity.offered_dialogs())
}

pub(super) async fn log_in(engine: &ChainEngine, mgr: &mut SpaceManager, world: &str) {
    let (tx, _rx) = mpsc::channel(256);
    fire_player_loaded(ENTITY, PLAYER, world, engine, &tx, mgr).await;
}

/// Click Moh'katan; returns whether a chain handled the click and everything
/// sent to the cell-to-base channel.
async fn click_mohkatan(
    engine: &ChainEngine,
    mgr: &mut SpaceManager,
) -> (bool, Vec<CellToBaseMsg>) {
    let (tx, mut rx) = mpsc::channel(256);
    let handled = fire_interact_tag(ENTITY, PLAYER, MOHKATAN_TAG, MOHKATAN, engine, &tx, mgr).await;
    (handled, drain(&mut rx))
}

// ── Relog and abandon ────────────────────────────────────────────────────

/// **Each login restores exactly one of the return_q2 owners.** With 10320
/// active nothing binds or heals; with 10320 completed and 10321 not_active
/// the offer binds at slot 54; at A the login heals to B (10145) and does
/// not bind; at B it needs nothing; with both completed it binds nothing.
/// Revert proof: remove 10145 and the at-A login stays at A.
#[tokio::test]
async fn live_db_castle_side_return_q2_relog_each_state() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &ALL_CHAINS).await;
    let cases: [(&str, Prereq, Own, Vec<i32>, Option<i32>); 5] = [
        ("10320 active", Prereq::Active, Own::Absent, vec![], None),
        (
            "10320 completed, 10321 not_active",
            Prereq::Completed,
            Own::Absent,
            vec![OFFER_SET],
            None,
        ),
        (
            "10321 at A",
            Prereq::Completed,
            Own::AtA,
            vec![],
            Some(STEP_B),
        ),
        (
            "10321 at B",
            Prereq::Completed,
            Own::AtB,
            vec![],
            Some(STEP_B),
        ),
        ("both completed", Prereq::Completed, Own::Done, vec![], None),
    ];
    for (label, prereq, own, bound, step) in cases {
        let mut mgr = staged(&pool, CASTLE, missions(prereq, own)).await;
        log_in(&engine, &mut mgr, CASTLE).await;
        assert_eq!(bound_at(&mgr, OFFER_SLOT), bound, "{label}: login bind");
        assert_eq!(step_of(&mgr), step, "{label}: login step");
    }
}

/// **An abandon at A or at B rebinds the Moh'katan offer.** The real abandon
/// removes 10321 first, then `mission_abandoned` fires 10146, which binds the
/// offer again at slot 54 because 10320 is completed and 10321 is not_active.
/// Revert proof: remove 10146 and the offer stays unbound after the abandon.
#[tokio::test]
async fn live_db_castle_side_return_q2_abandon_rebinds() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &ALL_CHAINS).await;

    for (label, own) in [("at A", Own::AtA), ("at B", Own::AtB)] {
        let mut mgr = staged(&pool, CASTLE, missions(Prereq::Completed, own)).await;
        let (tx, _rx) = mpsc::channel(256);
        assert!(
            abandon_mission(ENTITY, MISSION, &tx, &mut mgr).await,
            "{label}: the abandon must remove 10321"
        );
        fire_mission_abandoned(ENTITY, PLAYER, MISSION, &engine, &tx, &mut mgr).await;
        assert!(
            mission_of(&mgr, MISSION).is_none(),
            "{label}: 10321 must be gone after the abandon"
        );
        assert_eq!(
            bound_at(&mgr, OFFER_SLOT),
            vec![OFFER_SET],
            "{label}: mission_abandoned must rebind the offer (10146)"
        );
    }
}

// ── Refusals ─────────────────────────────────────────────────────────────

/// **The Moh'katan accept fires only in world 8, with 10320 completed and
/// 10321 not_active.** A positive control (world 8, 10320 completed, 10321
/// not started) is handled by 10142, so the refusals cannot pass vacuously.
/// Then the same click in CombatSim, with 10320 active, with 10321 at A and
/// with 10321 completed is handled by nothing, sends nothing, leaves 10321
/// unstarted or unchanged, and logs a `condition_failed` row for 10142 naming
/// the failing condition (0 = world, 1 = `mission_status 10320`, 2 =
/// `mission_status 10321`). Revert proof: drop 10142's `world` condition and
/// the CombatSim case is handled.
#[tokio::test]
async fn live_db_castle_side_return_q2_refusals_fire_nothing() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &ALL_CHAINS).await;

    let mut control = staged(&pool, CASTLE, vec![completed_prereq()]).await;
    let (handled, out) = click_mohkatan(&engine, &mut control).await;
    assert!(handled, "control: chain 10142 must accept in world 8");
    assert!(
        out.iter().any(|m| matches!(
            m,
            CellToBaseMsg::MissionUpdate {
                mission_id: MISSION,
                ..
            }
        )),
        "control: the accept must persist 10321"
    );

    // (label, world, missions, index of the condition that must fail)
    let refused: [(&str, &str, Vec<MissionInstance>, usize); 4] = [
        ("wrong world", COMBAT_SIM, vec![completed_prereq()], 0),
        (
            "10320 active",
            CASTLE,
            vec![MissionInstance::new(PREREQ, 0, Vec::new())],
            1,
        ),
        (
            "10321 at A",
            CASTLE,
            missions(Prereq::Completed, Own::AtA),
            2,
        ),
        (
            "10321 completed",
            CASTLE,
            missions(Prereq::Completed, Own::Done),
            2,
        ),
    ];
    for (label, world, held, failed_index) in refused {
        let capture = LogCapture::install();
        let mut mgr = staged(&pool, world, held).await;
        let before = snapshot(&mgr);
        let (handled, out) = click_mohkatan(&engine, &mut mgr).await;
        assert!(!handled, "{label}: no chain may handle the Moh'katan click");
        assert!(
            !out.iter().any(|m| matches!(
                m,
                CellToBaseMsg::MissionUpdate { .. } | CellToBaseMsg::EntityMethodCall { .. }
            )),
            "{label}: no mission or dialog traffic; got {out:?}"
        );
        assert_eq!(snapshot(&mgr), before, "{label}: the refusal moved state");
        let rows: Vec<_> = capture
            .all()
            .into_iter()
            .filter(|c| {
                c.level == Level::DEBUG
                    && c.target == "content.resolve"
                    && c.has_field("chain_id", &ACCEPT_CHAIN.to_string())
            })
            .collect();
        assert_eq!(rows.len(), 1, "{label}: {rows:#?}");
        assert!(rows[0].has_field("reason", "condition_failed"), "{rows:#?}");
        assert!(
            rows[0].has_field("failed_condition_index", &failed_index.to_string()),
            "{label}: condition {failed_index} must be the one that fails; {rows:#?}"
        );
    }
}

// ── Forged close ─────────────────────────────────────────────────────────

/// **A forged close, button or open of dialog 10725 moves nothing.** No chain
/// keys on dialog 10725, nor on any dialog event under scope 10321, so the
/// real dispatchers send nothing and move no state in any return_q2 state.
/// Revert proof: add a `dialog_choice` trigger for 10725 to any chain of the
/// story and the seed query finds it.
#[tokio::test]
async fn live_db_castle_side_return_q2_forged_close_is_inert() {
    let pool = require_db_or_skip!();
    let keyed: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM resources.content_triggers t \
         JOIN resources.content_chains c ON c.chain_id = t.chain_id \
         WHERE (t.event_type IN ('dialog_choice', 'dialog_open') AND t.event_key = $1) \
            OR (c.scope_type = 'mission' AND c.scope_id = $2 \
                AND t.event_type IN ('dialog_choice', 'dialog_open', 'dialog_set_open'))",
    )
    .bind(DIALOG.to_string())
    .bind(MISSION)
    .fetch_one(&pool)
    .await
    .expect("dialog trigger query must succeed");
    assert_eq!(
        keyed, 0,
        "no chain may key on dialog {DIALOG} or on a dialog event under scope {MISSION}"
    );

    let engine = engine_with(&pool, &ALL_CHAINS).await;
    for own in [Own::Absent, Own::AtA, Own::AtB, Own::Done] {
        let mut mgr = staged(&pool, CASTLE, missions(Prereq::Completed, own)).await;
        let before = snapshot(&mgr);
        let (tx, mut rx) = mpsc::channel(256);
        for button in [-1, 0, 1] {
            fire_dialog_choice(ENTITY, PLAYER, DIALOG, button, &engine, &tx, &mut mgr).await;
        }
        fire_dialog_open(ENTITY, PLAYER, DIALOG, &engine, &tx, &mut mgr).await;
        let out = drain(&mut rx);
        assert!(
            out.is_empty(),
            "{own:?}: forged dialog events must send nothing; got {out:?}"
        );
        assert_eq!(
            snapshot(&mgr),
            before,
            "{own:?}: forged dialog events moved state"
        );
    }
}
