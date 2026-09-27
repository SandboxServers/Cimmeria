//! Tests for [`super::crafting_station_tick`]: the station set a player
//! is told about as it moves, as a station despawns, and across a world
//! change.

use super::*;
use cimmeria_cell_catalog::crafting::{ENTITYFLAG_CRAFT_ALLOYING, ENTITYFLAG_CRAFT_CRAFT};

const PLAYER: u32 = 10;
const PLAYER_ID: i32 = 4410;
const STATION: u32 = 100_001;
/// A station for all four verbs.
const ALL_VERBS: u64 = 2048 | 4096 | 8192 | 16384;

fn world() -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /><Space WorldName="Harset" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /><Space WorldName="Harset" /></Spaces>"#,
    )
    .unwrap();
    mgr
}

fn add_player(mgr: &mut SpaceManager, world: &str, x: f32) {
    mgr.create_entity(PLAYER, world, [x, 0.0, 0.0], [0.0; 3])
        .unwrap();
    mgr.connect_entity(PLAYER);
    mgr.get_entity_mut(PLAYER).unwrap().player_id = Some(PLAYER_ID);
}

fn add_station(mgr: &mut SpaceManager, id: u32, x: f32, flags: u64) {
    mgr.spawn_npc(id, "Castle", [x, 0.0, 0.0], [0.0; 3])
        .unwrap();
    let e = mgr.get_entity_mut(id).unwrap();
    e.is_player = false;
    e.entity_flags = flags;
}

fn move_player(mgr: &mut SpaceManager, x: f32) {
    mgr.update_entity_position(PLAYER, [x, 0.0, 0.0], [0; 3], [0.0; 3]);
}

/// Run one tick and return the station reports it sent.
async fn tick(mgr: &mut SpaceManager) -> Vec<CraftingStations> {
    let (tx, mut rx) = mpsc::channel(16);
    crafting_station_tick(&tx, mgr).await;
    drop(tx);
    let mut out = Vec::new();
    while let Some(msg) = rx.recv().await {
        match msg {
            CellToBaseMsg::CraftingStations(r) => out.push(r),
            other => panic!("unexpected message {other:?}"),
        }
    }
    out
}

fn report(stations: [Option<u32>; 4], cause: StationChangeCause) -> CraftingStations {
    CraftingStations {
        entity_id: PLAYER,
        player_id: PLAYER_ID,
        stations,
        cause,
    }
}

/// Walking into range reports the station, standing still reports nothing,
/// and walking out reports the empty set.
#[tokio::test]
async fn reports_entering_and_leaving_range_only_on_change() {
    let mut mgr = world();
    add_player(&mut mgr, "Castle", 0.0);
    add_station(&mut mgr, STATION, 20.0, ALL_VERBS);

    assert_eq!(
        tick(&mut mgr).await,
        vec![report([None; 4], StationChangeCause::WorldChange)],
        "first tick"
    );
    assert!(tick(&mut mgr).await.is_empty(), "no change, no report");

    move_player(&mut mgr, 16.0);
    assert_eq!(
        tick(&mut mgr).await,
        vec![report([Some(STATION); 4], StationChangeCause::Moved)]
    );
    assert!(tick(&mut mgr).await.is_empty());

    move_player(&mut mgr, 30.0);
    assert_eq!(
        tick(&mut mgr).await,
        vec![report([None; 4], StationChangeCause::Moved)]
    );
}

/// A station that despawns under the player is reported gone.
#[tokio::test]
async fn reports_a_despawned_station() {
    let mut mgr = world();
    add_player(&mut mgr, "Castle", 0.0);
    add_station(&mut mgr, STATION, 2.0, ENTITYFLAG_CRAFT_CRAFT as u64);
    add_station(&mut mgr, STATION + 1, 3.0, ENTITYFLAG_CRAFT_ALLOYING as u64);
    assert_eq!(
        tick(&mut mgr).await,
        vec![report(
            [Some(STATION), None, None, Some(STATION + 1)],
            StationChangeCause::WorldChange
        )]
    );

    mgr.destroy_entity(STATION);
    assert_eq!(
        tick(&mut mgr).await,
        vec![report(
            [None, None, None, Some(STATION + 1)],
            StationChangeCause::StationDespawned
        )]
    );
}

/// A world change re-creates the player's cell entity, which forgets its
/// last report: the first tick in the new world reports the (empty) set,
/// so the base cannot keep the old world's station.
#[tokio::test]
async fn a_world_change_reports_the_new_worlds_set() {
    let mut mgr = world();
    add_player(&mut mgr, "Castle", 0.0);
    add_station(&mut mgr, STATION, 1.0, ALL_VERBS);
    assert_eq!(
        tick(&mut mgr).await,
        vec![report([Some(STATION); 4], StationChangeCause::WorldChange)]
    );

    mgr.destroy_entity(PLAYER);
    add_player(&mut mgr, "Harset", 0.0);
    assert_eq!(
        tick(&mut mgr).await,
        vec![report([None; 4], StationChangeCause::WorldChange)]
    );
}

/// A station at the same coordinates in another space is not in reach.
#[tokio::test]
async fn a_station_in_another_space_is_not_in_reach() {
    let mut mgr = world();
    add_player(&mut mgr, "Harset", 0.0);
    add_station(&mut mgr, STATION, 0.5, ALL_VERBS);
    assert_eq!(
        tick(&mut mgr).await,
        vec![report([None; 4], StationChangeCause::WorldChange)]
    );
}

/// Reach is measured in 3-D, as `interact` measures it: a station on the
/// floor above is not in reach.
#[tokio::test]
async fn a_station_on_the_floor_above_is_not_in_reach() {
    let mut mgr = world();
    add_player(&mut mgr, "Castle", 0.0);
    mgr.spawn_npc(STATION, "Castle", [0.0, 6.0, 0.0], [0.0; 3])
        .unwrap();
    mgr.get_entity_mut(STATION).unwrap().entity_flags = ALL_VERBS;
    assert_eq!(
        tick(&mut mgr).await,
        vec![report([None; 4], StationChangeCause::WorldChange)]
    );
}

/// A player still in world entry (no `player_id`) is skipped, and reports
/// once loaded.
#[tokio::test]
async fn a_player_without_player_id_reports_once_loaded() {
    let mut mgr = world();
    add_player(&mut mgr, "Castle", 0.0);
    mgr.get_entity_mut(PLAYER).unwrap().player_id = None;
    assert!(tick(&mut mgr).await.is_empty());

    mgr.get_entity_mut(PLAYER).unwrap().player_id = Some(PLAYER_ID);
    assert_eq!(
        tick(&mut mgr).await,
        vec![report([None; 4], StationChangeCause::WorldChange)]
    );
}

/// A report the base channel refuses is a WARN `forward_failed` carrying the
/// player's identity, not a silent drop.
#[tokio::test]
async fn a_failed_report_send_warns_with_identity() {
    let capture = crate::test_support::LogCapture::install();
    let mut mgr = world();
    add_player(&mut mgr, "Castle", 0.0);
    mgr.get_entity_mut(PLAYER).unwrap().account_id = Some(4409);
    let (tx, rx) = mpsc::channel(1);
    drop(rx);

    crafting_station_tick(&tx, &mut mgr).await;

    let event = capture
        .find_message(
            tracing::Level::WARN,
            "crafting station report could not be queued",
        )
        .expect("the failed send is logged");
    assert_eq!(event.target, "crafting");
    assert!(event.has_field("event", "forward_failed"), "{event:#?}");
    assert!(event.has_field("account_id", "4409"), "{event:#?}");
    assert!(
        event.has_field("player_id", &PLAYER_ID.to_string()),
        "{event:#?}"
    );
    assert!(
        event.has_field("entity_id", &PLAYER.to_string()),
        "{event:#?}"
    );
}
