//! Mission 1664 "Debriefing" from the Castle grant to the Command Center door
//! (FZ-22), on the whole seeded engine. Unlike `harset_1664` and `castle_1664`,
//! which load only their own chains, every test here builds the engine from
//! the database, so another seeded chain on `mission_completed 708`,
//! `player_loaded 'Harset'`, `interact_tag` or `mission_abandoned` that also
//! matches shows up as an extra trace label or an extra bind.
//!
//! The harness is copied from `harset_1664` and `castle_1664` rather than
//! shared, so a change to one file cannot move another file's assertions.

use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_entity::missions::{
    MissionInstance, MissionObjective, MISSION_COMPLETED, STATUS_ACTIVE,
};
use sqlx::PgPool;
use tokio::sync::mpsc;

use super::super::engine_loader::build_engine;
use super::super::event_dispatch::fire_mission_completed;
use super::super::{fire_interact_tag, fire_player_loaded};
use crate::cell::content::fire_mission_abandoned;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::missions::abandon_mission;
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
const CASTLE: &str = "Castle";
const HARSET: &str = "Harset";
const CMD_CENTER: &str = "Harset_CmdCenter";
const KAERLIA: &str = "Harset_Kaerlia";
const DOOR: &str = "Harset_CmdDoor";
/// Wire index of the client method onDialogDisplay.
const ON_DIALOG_DISPLAY: u16 = 105;
/// Wire index of the client method onPlayerCommunication.
const ON_PLAYER_COMMUNICATION: u16 = 28;
/// The Kaerlia cue (dialog set 7016) and the door cue (dialog set map 120301),
/// and the slots they sit in.
const KAERLIA_SLOT: i32 = 260;
const DOOR_SLOT: i32 = 261;

// ── Harness ──────────────────────────────────────────────────────────────

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

fn mission_of(mgr: &SpaceManager) -> Option<&MissionInstance> {
    mgr.get_entity(ENTITY)
        .unwrap()
        .missions
        .get_mission(MISSION)
}

fn step_of(mgr: &SpaceManager) -> Option<i32> {
    mission_of(mgr).and_then(|m| m.current_step_id)
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

/// The dialog-set-map ids bound in `slot`, ascending.
fn slot_ids(mgr: &SpaceManager, slot: i32) -> Vec<i32> {
    all_binds(mgr)
        .into_iter()
        .filter(|(s, _)| *s == slot)
        .flat_map(|(_, ids)| ids)
        .collect()
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

/// The text of dialog screen `id`, which the abandon test compares against.
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
/// cell-to-base channel, as a compact trace in send order.
async fn traffic(fire: impl AsyncFnOnce(&mpsc::Sender<CellToBaseMsg>)) -> Vec<String> {
    let (tx, mut rx) = mpsc::channel(256);
    fire(&tx).await;
    let mut trace: Vec<String> = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        let label = match msg {
            CellToBaseMsg::MissionUpdate {
                mission_id: MISSION,
                current_step_id: Some(step),
                ..
            } => {
                format!("step {step}")
            }
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
        // Only the step echo may repeat (one advance can update twice); a
        // doubled dialog, bark or travel must stay visible in the trace.
        if label.starts_with("step ") && trace.last() == Some(&label) {
            continue;
        }
        trace.push(label);
    }
    trace
}

async fn complete_708(engine: &ChainEngine, mgr: &mut SpaceManager) -> Vec<String> {
    traffic(async |tx| fire_mission_completed(ENTITY, PLAYER, 708, engine, tx, mgr).await).await
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

/// Remove 1664 from the player (the real abandon), then fire the
/// `mission_abandoned` chains.
async fn abandon(engine: &ChainEngine, mgr: &mut SpaceManager) -> Vec<String> {
    let (tx, _rx) = mpsc::channel(256);
    assert!(
        abandon_mission(ENTITY, MISSION, &tx, mgr).await,
        "the abandon must remove 1664"
    );
    traffic(async |tx| fire_mission_abandoned(ENTITY, PLAYER, MISSION, engine, tx, mgr).await).await
}

// ── Walk ─────────────────────────────────────────────────────────────────

/// **1664 walks from the Castle grant to the Command Center, one step at a
/// time.** 708 completes in Castle and holds 1664 at 5224; the granted mission
/// carried into Harset binds only the Kaerlia cue; the Kaerlia click advances
/// to 5225 and shows 6169, moving the cue to the door; the door click advances
/// to 5226 and travels. Any longer trace means another seeded chain matched.
#[tokio::test]
async fn live_db_mission_1664_walks_from_castle_to_the_command_center() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;

    let mut castle = staged(&pool, CASTLE, vec![completed(708)]).await;
    complete_708(&engine, &mut castle).await;
    assert_eq!(
        step_of(&castle),
        Some(5224),
        "708 in Castle must hold 1664 at 5224"
    );
    let granted = mission_of(&castle)
        .expect("1664 must be granted in Castle")
        .clone();

    let mut harset = staged(&pool, HARSET, vec![completed(708), granted]).await;
    log_in(&engine, &mut harset, HARSET).await;
    assert_eq!(
        slot_ids(&harset, KAERLIA_SLOT),
        vec![7016],
        "the login at 5224 binds the Kaerlia cue"
    );

    let (handled, trace) = click(&engine, &mut harset, KAERLIA).await;
    assert!(handled, "a K chain must answer Harset_Kaerlia");
    assert_eq!(
        trace,
        vec!["step 5225".to_string(), "dialog 6169".to_string()],
        "the Kaerlia click trace"
    );
    assert!(
        slot_ids(&harset, KAERLIA_SLOT).is_empty(),
        "the Kaerlia cue must leave slot {KAERLIA_SLOT}"
    );
    assert_eq!(
        slot_ids(&harset, DOOR_SLOT),
        vec![120301],
        "the door cue must take slot {DOOR_SLOT}"
    );

    let (handled, trace) = click(&engine, &mut harset, DOOR).await;
    assert!(handled, "a door chain must answer Harset_CmdDoor");
    assert_eq!(
        trace,
        vec!["step 5226".to_string(), format!("travel {CMD_CENTER}"),],
        "the door click trace"
    );
}

/// **Relog and abandon hold on the whole engine.** A Harset login at 5225
/// restores only the door cue; at 5224 an abandon drops both cues and the
/// stock line answers the next Kaerlia click; a login at 5226 binds nothing,
/// in Harset or at the Command Center.
#[tokio::test]
async fn live_db_mission_1664_relog_and_abandon_on_the_whole_engine() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;

    let mut relog = staged(
        &pool,
        HARSET,
        vec![completed(708), mission_1664(5225, 6060)],
    )
    .await;
    log_in(&engine, &mut relog, HARSET).await;
    assert!(
        slot_ids(&relog, KAERLIA_SLOT).is_empty(),
        "the 5225 login must not bind the Kaerlia cue"
    );
    assert_eq!(
        slot_ids(&relog, DOOR_SLOT),
        vec![120301],
        "the 5225 login restores the door cue"
    );

    let mut gone = staged(
        &pool,
        HARSET,
        vec![completed(708), mission_1664(5224, 6063)],
    )
    .await;
    log_in(&engine, &mut gone, HARSET).await;
    assert_eq!(
        slot_ids(&gone, KAERLIA_SLOT),
        vec![7016],
        "the 5224 login binds the Kaerlia cue"
    );
    abandon(&engine, &mut gone).await;
    assert!(
        mission_of(&gone).is_none(),
        "the abandon must remove 1664 from the player"
    );
    assert!(
        slot_ids(&gone, KAERLIA_SLOT).is_empty(),
        "the abandon must drop the Kaerlia cue"
    );
    assert!(
        slot_ids(&gone, DOOR_SLOT).is_empty(),
        "the abandon must leave slot {DOOR_SLOT} empty"
    );
    let (handled, trace) = click(&engine, &mut gone, KAERLIA).await;
    assert!(
        handled,
        "a K chain must answer Harset_Kaerlia after the abandon"
    );
    assert_eq!(
        trace,
        vec![format!("bark {}", screen(&pool, 117158).await)],
        "the Kaerlia click after the abandon gets the stock line"
    );

    for world in [HARSET, CMD_CENTER] {
        let mut mgr = staged(&pool, world, vec![completed(708), mission_1664(5226, 6061)]).await;
        log_in(&engine, &mut mgr, world).await;
        assert!(
            slot_ids(&mgr, KAERLIA_SLOT).is_empty(),
            "the 5226 login in {world} must leave slot {KAERLIA_SLOT} empty"
        );
        assert!(
            slot_ids(&mgr, DOOR_SLOT).is_empty(),
            "the 5226 login in {world} must leave slot {DOOR_SLOT} empty"
        );
    }
}

// ── Grant and door ───────────────────────────────────────────────────────

/// **1664 is granted by exactly one chain and never re-granted.** The grant
/// query returns chain 1398 alone, and no chain 1399 is seeded.
#[tokio::test]
async fn live_db_mission_1664_has_one_grant_and_no_regrant() {
    let pool = require_db_or_skip!();

    let grants: Vec<(i32,)> = sqlx::query_as(
        "SELECT a.chain_id FROM resources.content_actions a \
         JOIN resources.content_chains c USING (chain_id) \
         WHERE a.action_type = 'accept_mission' AND a.target_id = 1664 AND c.enabled",
    )
    .fetch_all(&pool)
    .await
    .expect("grant query");
    assert_eq!(
        grants,
        vec![(1398,)],
        "O18-02 (option P0): 1664 has one grant and no re-grant"
    );

    let (retired,): (i64,) =
        sqlx::query_as("SELECT count(*) FROM resources.content_chains WHERE chain_id = 1399")
            .fetch_one(&pool)
            .await
            .expect("retired chain query");
    assert_eq!(
        retired, 0,
        "O18-02 (option P0): 1664 has one grant and no re-grant"
    );
}

/// **The walk-in door 6006 teleports once.** Its only `cross_world_teleport`
/// row goes to Harset_CmdCenter, so a second teleport row would double the
/// travel (the D-PU4 soft-lock guard, FZ-25).
#[tokio::test]
async fn live_db_walk_in_door_6006_still_teleports_once() {
    let pool = require_db_or_skip!();

    let rows: Vec<(Option<String>,)> = sqlx::query_as(
        "SELECT target_key FROM resources.content_actions \
         WHERE chain_id = 6006 AND action_type = 'cross_world_teleport'",
    )
    .fetch_all(&pool)
    .await
    .expect("teleport rows");
    assert_eq!(
        rows,
        vec![(Some(CMD_CENTER.to_string()),)],
        "chain 6006 must teleport exactly once, to Harset_CmdCenter"
    );
}
