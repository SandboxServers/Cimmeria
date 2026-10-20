//! Mission 1664 step 5226 (FZ-27): the Marsh and Moh'katan reports, the cue
//! restore on Command Center login and the abandon tidy-up. The chains under
//! test are the ones `harset_1664_chains.sql` seeds (6016, 6018-6019, 6033-6034).
//!
//! Every event goes through the real dispatchers, so the executor arms run.
//! The harness is copied from `harset_1664` rather than shared, so a change to
//! one file cannot move the other's assertions. The chains load as shipped (no
//! force-enable), so `enabled = false` on any of them fails here.

use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_entity::missions::{
    MissionInstance, MissionObjective, MISSION_ACTIVE, MISSION_COMPLETED, STATUS_ACTIVE,
    STATUS_COMPLETED,
};
use sqlx::PgPool;
use tokio::sync::mpsc;

use super::super::engine_loader::load_single_chain_for_test;
use super::super::{fire_interact_tag, fire_player_loaded};
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use crate::cell::spawner::{
    load_dialog_screen_text, load_dialog_set_maps, load_item_containers, load_mission_defs,
    load_monologue_dialog_ids, load_step_objectives, load_world_rows,
};
use crate::test_support::require_db_or_skip;

const ENTITY: u32 = 1;
const PLAYER: i32 = 42;
const TARGET: u32 = 3;
const MISSION: i32 = 1664;
const HARSET: &str = "Harset";
const CMD_CENTER: &str = "Harset_CmdCenter";
const MARSH: &str = "CmdCenter_Marsh";
const MOHKATAN: &str = "CmdCenter_Mohkatan";
/// Wire index of the client method onDialogDisplay.
const ON_DIALOG_DISPLAY: u16 = 105;
/// Wire index of the client method onPlayerCommunication (barks).
const ON_PLAYER_COMMUNICATION: u16 = 28;
/// The abandon tidy-up (6016) and the two debrief reports and cue restores.
const CHAINS: [i32; 5] = [6016, 6018, 6019, 6033, 6034];

// ── Harness ──────────────────────────────────────────────────────────────

/// An engine holding `ids`, loaded as shipped.
async fn engine_with(pool: &PgPool, ids: &[i32]) -> ChainEngine {
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

/// `world` with player `ENTITY` holding `missions`, with the dialog-set,
/// monologue and dialog-screen caches loaded from the seed.
async fn staged(pool: &PgPool, world: &str, missions: Vec<MissionInstance>) -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    let space = format!(
        r#"<Space WorldName="{world}" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" />"#
    );
    mgr.parse_spaces_xml(&format!("<Spaces>{space}</Spaces>"))
        .unwrap();
    mgr.create_startup_spaces(&format!(
        r#"<Spaces><Space WorldName="{world}" /></Spaces>"#
    ))
    .unwrap();
    mgr.stamp_world_rows(&load_world_rows(pool).await.expect("world cache"));
    mgr.mission_defs = load_mission_defs(pool).await.expect("mission cache");
    mgr.step_objectives = load_step_objectives(pool).await.expect("objective cache");
    mgr.item_containers = load_item_containers(pool).await.expect("item cache");
    mgr.dialog_set_maps = load_dialog_set_maps(pool).await.expect("dialog set maps");
    mgr.dialog_screen_text = load_dialog_screen_text(pool)
        .await
        .expect("dialog screen text");
    mgr.monologue_dialog_ids = load_monologue_dialog_ids(pool)
        .await
        .expect("monologue ids");
    mgr.create_entity(TARGET, world, [3.0; 3], [0.0; 3])
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

/// A completed mission.
fn completed(mission_id: i32) -> MissionInstance {
    let mut mission = MissionInstance::new(mission_id, 0, Vec::new());
    mission.status = MISSION_COMPLETED;
    mission.current_step_id = None;
    mission
}

/// 1664 active at `step` with `objective` active.
fn mission_1664(step: i32, objective: i32) -> MissionInstance {
    let objective = MissionObjective {
        objective_id: objective,
        status: STATUS_ACTIVE,
        hidden: false,
        optional: false,
    };
    let mut mission = MissionInstance::new(MISSION, step, vec![objective]);
    mission.current_step_id = Some(step);
    mission
}

/// 1664 at step 5226 with both report objectives (6061 Marsh, 6062 Moh'katan).
/// An objective in `done` is completed, the other stays active.
fn debrief(done: &[i32]) -> MissionInstance {
    let objectives: Vec<MissionObjective> = [6061, 6062]
        .into_iter()
        .map(|objective_id| MissionObjective {
            objective_id,
            status: if done.contains(&objective_id) {
                STATUS_COMPLETED
            } else {
                STATUS_ACTIVE
            },
            hidden: false,
            optional: false,
        })
        .collect();
    let mut mission = MissionInstance::new(MISSION, 5226, objectives);
    mission.current_step_id = Some(5226);
    mission
}

/// A completed 1664.
fn completed_1664() -> MissionInstance {
    completed(MISSION)
}

/// The states where neither report may answer: no 1664, 5224, 5225 and a
/// completed 1664. Built fresh on each call because a mission is not cloned.
fn off_steps() -> [(&'static str, Vec<MissionInstance>); 4] {
    [
        ("no 1664", Vec::new()),
        ("5224", vec![mission_1664(5224, 6063)]),
        ("5225", vec![mission_1664(5225, 6060)]),
        ("completed", vec![completed_1664()]),
    ]
}

/// Every bind on the player as `(slot, dialog-set-map ids)`, ascending.
fn all_binds(mgr: &SpaceManager) -> Vec<(i32, Vec<i32>)> {
    let player = mgr.get_entity(ENTITY).unwrap();
    let mut binds: Vec<(i32, Vec<i32>)> = player
        .available_interactions
        .iter()
        .filter(|(_, v)| !v.is_empty())
        .map(|(&slot, v)| {
            let mut ids: Vec<i32> = v.iter().map(|&(id, _, _)| id).collect();
            ids.sort_unstable();
            (slot, ids)
        })
        .collect();
    binds.sort_unstable();
    binds
}

/// The mission 1664 instance the player holds, if any.
fn mission_of(mgr: &SpaceManager) -> Option<&MissionInstance> {
    mgr.get_entity(ENTITY)
        .unwrap()
        .missions
        .get_mission(MISSION)
}

/// Status and current step of 1664. Panics if the player no longer holds it.
fn status_of(mgr: &SpaceManager) -> (i8, Option<i32>) {
    let mission = mission_of(mgr).expect("the player must still hold 1664");
    (mission.status, mission.current_step_id)
}

/// True when 1664 has `objective` completed, active-but-completed or in the
/// completed list.
fn objective_done(mgr: &SpaceManager, objective: i32) -> bool {
    let mission = mission_of(mgr).expect("the player must still hold 1664");
    mission.completed_objectives.contains(&objective)
        || mission
            .active_objectives
            .iter()
            .any(|o| o.objective_id == objective && o.status == STATUS_COMPLETED)
}

/// Read a WSTRING at `offset`: a `u32` UTF-16 code-unit count followed by
/// that many little-endian `u16`s. Returns the decoded string and the
/// code-unit count, as in `npc_bark.rs`.
fn read_wstring(args: &[u8], offset: usize) -> (String, usize) {
    let units = u32::from_le_bytes(args[offset..offset + 4].try_into().unwrap()) as usize;
    let (pairs, _) = args[offset + 4..offset + 4 + units * 2].as_chunks::<2>();
    let s = char::decode_utf16(pairs.iter().copied().map(u16::from_le_bytes))
        .map(|r| r.expect("wire text must be valid UTF-16"))
        .collect();
    (s, units)
}

/// The text of an `onPlayerCommunication` payload: speaker WSTRING, SpeakerFlags
/// byte, Channel byte, then the text WSTRING.
fn decode_bark_text(args: &[u8]) -> String {
    let (_, speaker_units) = read_wstring(args, 0);
    let flags_off = 4 + speaker_units * 2;
    let (text, _) = read_wstring(args, flags_off + 2);
    text
}

/// The text of dialog screen `id`, which the bark tests compare against.
async fn screen(pool: &PgPool, id: i32) -> String {
    let text: String =
        sqlx::query_scalar("SELECT text FROM resources.dialog_screens WHERE screen_id = $1")
            .bind(id)
            .fetch_one(pool)
            .await
            .expect("dialog screen row");
    assert!(!text.is_empty(), "screen {id} must carry text");
    text
}

/// Fire one event against the stage and return everything sent to the
/// cell-to-base channel, as a compact trace in send order. A report click's
/// trace is exactly its bark: `bark TEXT`, `dialog N` or `travel W`.
async fn traffic(fire: impl AsyncFnOnce(&mpsc::Sender<CellToBaseMsg>)) -> Vec<String> {
    let (tx, mut rx) = mpsc::channel(256);
    fire(&tx).await;
    let mut trace: Vec<String> = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        let label = match msg {
            CellToBaseMsg::EntityMethodCall {
                entity_id: ENTITY,
                method_index: ON_DIALOG_DISPLAY,
                args,
            } => {
                format!(
                    "dialog {}",
                    i32::from_le_bytes(args[4..8].try_into().unwrap())
                )
            }
            CellToBaseMsg::EntityMethodCall {
                method_index: ON_PLAYER_COMMUNICATION,
                args,
                ..
            } => {
                format!("bark {}", decode_bark_text(&args))
            }
            CellToBaseMsg::GateTravel {
                target_world_name, ..
            } => {
                format!("travel {target_world_name}")
            }
            _ => continue,
        };
        trace.push(label);
    }
    trace
}

async fn log_in(engine: &ChainEngine, mgr: &mut SpaceManager, world: &str) -> Vec<String> {
    traffic(async |tx| fire_player_loaded(ENTITY, PLAYER, world, engine, tx, mgr).await).await
}

async fn click(engine: &ChainEngine, mgr: &mut SpaceManager, tag: &str) -> (bool, Vec<String>) {
    let mut handled = false;
    let trace = traffic(async |tx| {
        handled = fire_interact_tag(ENTITY, PLAYER, tag, TARGET, engine, tx, mgr).await;
    })
    .await;
    (handled, trace)
}

/// Remove `mission` from the player and fire its abandon through the real
/// dispatcher, returning the trace the abandon sends.
async fn abandon(engine: &ChainEngine, mgr: &mut SpaceManager, mission: i32) -> Vec<String> {
    traffic(async |tx| {
        super::super::event_dispatch::fire_mission_abandoned(
            ENTITY, PLAYER, mission, engine, tx, mgr,
        )
        .await
    })
    .await
}

// ── Debrief cues on Command Center login ─────────────────────────────────

/// **Command Center login restores the debrief cues, and only the open ones.**
/// At 5226 with neither report credited, Marsh's cue (slot 10, dsm 120302) and
/// Moh'katan's (slot 54, dsm 120303) both come back. Each credited report drops
/// its own cue. Outside 5226, and in Harset, nothing is bound. Revert proof:
/// delete chain 6033 and the first case loses slot 10.
#[tokio::test]
async fn live_db_mission_1664_debrief_cues_restore_on_command_center_login() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &CHAINS).await;

    let mut both = staged(&pool, CMD_CENTER, vec![debrief(&[])]).await;
    log_in(&engine, &mut both, CMD_CENTER).await;
    assert_eq!(
        all_binds(&both),
        vec![(10, vec![120302]), (54, vec![120303])],
        "5226 with no report credited binds both debrief cues"
    );

    let mut marsh_done = staged(&pool, CMD_CENTER, vec![debrief(&[6061])]).await;
    log_in(&engine, &mut marsh_done, CMD_CENTER).await;
    assert_eq!(
        all_binds(&marsh_done),
        vec![(54, vec![120303])],
        "Marsh credited: only Moh'katan's cue returns"
    );

    let mut mohkatan_done = staged(&pool, CMD_CENTER, vec![debrief(&[6062])]).await;
    log_in(&engine, &mut mohkatan_done, CMD_CENTER).await;
    assert_eq!(
        all_binds(&mohkatan_done),
        vec![(10, vec![120302])],
        "Moh'katan credited: only Marsh's cue returns"
    );

    for missions in [
        Vec::new(),
        vec![mission_1664(5224, 6063)],
        vec![mission_1664(5225, 6060)],
        vec![completed_1664()],
    ] {
        let mut mgr = staged(&pool, CMD_CENTER, missions).await;
        log_in(&engine, &mut mgr, CMD_CENTER).await;
        assert!(
            all_binds(&mgr).is_empty(),
            "no debrief cue outside 5226; got {:?}",
            all_binds(&mgr)
        );
    }

    let mut harset = staged(&pool, HARSET, vec![debrief(&[])]).await;
    log_in(&engine, &mut harset, HARSET).await;
    assert!(
        all_binds(&harset).is_empty(),
        "the debrief cues are Command Center binds, not Harset ones"
    );
}

// ── Reports ──────────────────────────────────────────────────────────────

/// **Marsh then Moh'katan completes 1664.** Marsh's report barks his screen,
/// drops his cue and credits 6061: 1664 stays active at 5226 with only
/// Moh'katan's cue left. Moh'katan's report credits 6062, which auto-completes
/// 1664 and drops the last cue. Revert proof: delete 6018's `complete_objective`
/// row and the Marsh click leaves 6061 open, failing the first assertion.
#[tokio::test]
async fn live_db_mission_1664_marsh_then_mohkatan_completes_the_mission() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &CHAINS).await;
    let marsh = screen(&pool, 200180).await;
    let mohkatan = screen(&pool, 200181).await;

    let mut mgr = staged(&pool, CMD_CENTER, vec![debrief(&[])]).await;
    log_in(&engine, &mut mgr, CMD_CENTER).await;
    let (handled, trace) = click(&engine, &mut mgr, MARSH).await;
    assert!(handled, "Marsh's report must be answered at 5226");
    assert_eq!(trace, vec![format!("bark {marsh}")]);
    assert!(objective_done(&mgr, 6061), "Marsh's report credits 6061");
    assert_eq!(status_of(&mgr), (MISSION_ACTIVE, Some(5226)));
    assert_eq!(all_binds(&mgr), vec![(54, vec![120303])]);

    let (handled, trace) = click(&engine, &mut mgr, MOHKATAN).await;
    assert!(handled, "Moh'katan's report must be answered after Marsh's");
    assert_eq!(trace, vec![format!("bark {mohkatan}")]);
    assert_eq!(status_of(&mgr), (MISSION_COMPLETED, None));
    assert!(
        all_binds(&mgr).is_empty(),
        "both cues are gone once 1664 completes; got {:?}",
        all_binds(&mgr)
    );
}

/// **Moh'katan then Marsh completes 1664, in the other order.** The two credits
/// commute: the second report completes the mission whichever report it is.
/// Revert proof: delete 6019's `complete_objective` row and the Moh'katan click
/// leaves 6062 open, failing the first assertion.
#[tokio::test]
async fn live_db_mission_1664_mohkatan_then_marsh_completes_the_mission() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &CHAINS).await;
    let marsh = screen(&pool, 200180).await;
    let mohkatan = screen(&pool, 200181).await;

    let mut mgr = staged(&pool, CMD_CENTER, vec![debrief(&[])]).await;
    log_in(&engine, &mut mgr, CMD_CENTER).await;
    let (handled, trace) = click(&engine, &mut mgr, MOHKATAN).await;
    assert!(handled, "Moh'katan's report must be answered at 5226");
    assert_eq!(trace, vec![format!("bark {mohkatan}")]);
    assert!(
        objective_done(&mgr, 6062),
        "Moh'katan's report credits 6062"
    );
    assert_eq!(status_of(&mgr), (MISSION_ACTIVE, Some(5226)));
    assert_eq!(all_binds(&mgr), vec![(10, vec![120302])]);

    let (handled, trace) = click(&engine, &mut mgr, MARSH).await;
    assert!(handled, "Marsh's report must be answered after Moh'katan's");
    assert_eq!(trace, vec![format!("bark {marsh}")]);
    assert_eq!(status_of(&mgr), (MISSION_COMPLETED, None));
    assert!(
        all_binds(&mgr).is_empty(),
        "both cues are gone once 1664 completes; got {:?}",
        all_binds(&mgr)
    );
}

/// **A reported NPC answers nothing new.** After Marsh's report, a second click
/// on him is refused: 6061 is complete, so 6018 no longer matches, and no bark
/// or credit follows. Revert proof: drop 6018's `objective_status` row and the
/// second click barks again.
#[tokio::test]
async fn live_db_mission_1664_a_reported_npc_answers_nothing_new() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &CHAINS).await;

    let mut mgr = staged(&pool, CMD_CENTER, vec![debrief(&[])]).await;
    log_in(&engine, &mut mgr, CMD_CENTER).await;
    let (handled, _) = click(&engine, &mut mgr, MARSH).await;
    assert!(handled, "the first report must be answered");

    let (handled, trace) = click(&engine, &mut mgr, MARSH).await;
    assert!(!handled, "a reported NPC must not answer again");
    assert!(trace.is_empty(), "no bark on a repeat; got {trace:?}");
    assert_eq!(status_of(&mgr), (MISSION_ACTIVE, Some(5226)));
}

// ── Gates ────────────────────────────────────────────────────────────────

/// **The debrief clicks are gated.** Neither report answers outside 5226 (no
/// 1664, 5224, 5225 or completed), outside Command Center (`world 68`), or while
/// the Praxis (1361 at 4694) or Jaffa (1324 at 3954) guards are live. Each refusal
/// is silent: not handled, empty trace. Revert proof: drop 6018's `world` row and
/// the Harset click credits.
#[tokio::test]
async fn live_db_mission_1664_debrief_clicks_are_gated() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &CHAINS).await;

    for tag in [MARSH, MOHKATAN] {
        for (state, missions) in off_steps() {
            let mut mgr = staged(&pool, CMD_CENTER, missions).await;
            let (handled, trace) = click(&engine, &mut mgr, tag).await;
            assert!(!handled, "{tag} must be refused with {state}");
            assert!(trace.is_empty(), "{tag} with {state} sends nothing");
        }

        let mut harset = staged(&pool, HARSET, vec![debrief(&[])]).await;
        let (handled, trace) = click(&engine, &mut harset, tag).await;
        assert!(!handled, "{tag} must be refused outside Command Center");
        assert!(trace.is_empty(), "{tag} in Harset sends nothing");
    }

    let mut praxis = MissionInstance::new(1361, 4694, Vec::new());
    praxis.current_step_id = Some(4694);
    let mut mgr = staged(&pool, CMD_CENTER, vec![debrief(&[]), praxis]).await;
    let (handled, trace) = click(&engine, &mut mgr, MARSH).await;
    assert!(!handled, "the Praxis step 4694 blocks Marsh's report");
    assert!(trace.is_empty(), "the Praxis block sends nothing");

    let mut jaffa = MissionInstance::new(1324, 3954, Vec::new());
    jaffa.current_step_id = Some(3954);
    let mut mgr = staged(&pool, CMD_CENTER, vec![debrief(&[]), jaffa]).await;
    let (handled, trace) = click(&engine, &mut mgr, MOHKATAN).await;
    assert!(!handled, "the Jaffa step 3954 blocks Moh'katan's report");
    assert!(trace.is_empty(), "the Jaffa block sends nothing");
}

// ── Abandon ──────────────────────────────────────────────────────────────

/// **Abandoning 1664 at 5226 drops both debrief cues.** The login has bound
/// them both; the abandon fires 6016, which removes slot 10 and slot 54 along
/// with the earlier cues. Revert proof: drop 6016's 120302 row and slot 10
/// survives the abandon.
#[tokio::test]
async fn live_db_mission_1664_abandon_at_5226_drops_both_debrief_cues() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &CHAINS).await;

    let mut mgr = staged(&pool, CMD_CENTER, vec![debrief(&[])]).await;
    log_in(&engine, &mut mgr, CMD_CENTER).await;
    assert_eq!(
        all_binds(&mgr),
        vec![(10, vec![120302]), (54, vec![120303])],
        "both debrief cues are bound before the abandon"
    );
    mgr.get_entity_mut(ENTITY)
        .unwrap()
        .missions
        .abandon_mission(MISSION);
    abandon(&engine, &mut mgr, MISSION).await;
    assert!(
        all_binds(&mgr).is_empty(),
        "the abandon must drop both debrief cues; got {:?}",
        all_binds(&mgr)
    );
}
