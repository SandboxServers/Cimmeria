//! The chatter tick over a real `SpaceManager`: who hears a line, the bytes
//! they get, when a scene starts, and the skipped-speaker WARN.

use std::time::{Duration, Instant};

use cimmeria_cell_catalog::cell::spawner::{
    AmbientChatterCatalog, ChatterExchange, ChatterGroup, ChatterLine,
};
use cimmeria_names::{NameBook, Table};
use cimmeria_wire::cell::chat::{serialize_on_player_communication, CHAN_SAY};
use cimmeria_wire::cell::client_methods::communicator::ON_PLAYER_COMMUNICATION;
use cimmeria_wire::state_field::BSF_DEAD;
use tokio::sync::mpsc;
use tracing::Level;

use super::schedule::IDLE_RECHECK;
use super::*;
use crate::test_support::{make_space_manager, LogCapture};

const WORLD_ID: i32 = 1300;
const RA: u32 = 900_001;
const BAAL: u32 = 900_002;
const RA_NAME_ID: i32 = 20205;
const BAAL_NAME_ID: i32 = 8186;
/// Within the 20 m radius of Ra at the origin.
const NEAR: u32 = 1;
/// 30 m from both lords.
const FAR: u32 = 2;

fn line(tag: &str, ms: u64, text: &str) -> ChatterLine {
    ChatterLine {
        speaker_tag: tag.into(),
        delay: Duration::from_millis(ms),
        text: text.into(),
    }
}

fn catalog(world_id: i32) -> AmbientChatterCatalog {
    AmbientChatterCatalog {
        groups: vec![ChatterGroup {
            group_id: 1,
            world_id,
            name: "summit".into(),
            hear_radius: 20.0,
            exchange_gap: Duration::from_secs(30),
            exchanges: vec![
                ChatterExchange {
                    exchange_id: 1,
                    lines: vec![
                        line("Lords_Ra", 0, "The sun is mine."),
                        line("Lords_Baal", 4000, "Paperwork, Ra."),
                    ],
                },
                ChatterExchange {
                    exchange_id: 2,
                    lines: vec![line("Lords_Baal", 0, "Cappuccino?")],
                },
            ],
        }],
    }
}

fn install_names() {
    let mut book = NameBook::empty();
    book.insert(Table::Texts, RA_NAME_ID.into(), "Ra");
    book.insert(Table::Texts, BAAL_NAME_ID.into(), "Ba'al");
    cimmeria_names::global().store(book);
}

/// Agnos as world 1300, Ra at the origin and Ba'al 3 m east, a player 5 m
/// north of Ra and one 30 m south, and the catalog in the resources.
fn world() -> SpaceManager {
    install_names();
    let mut mgr = make_space_manager();
    mgr.worlds.get_mut("Agnos").unwrap().world_id = Some(WORLD_ID);
    for (id, tag, name_id, x) in [
        (RA, "Lords_Ra", RA_NAME_ID, 0.0),
        (BAAL, "Lords_Baal", BAAL_NAME_ID, 3.0),
    ] {
        mgr.create_entity(id, "Agnos", [x, 0.0, 0.0], [0.0; 3])
            .unwrap();
        let e = mgr.get_entity_mut(id).unwrap();
        e.tag = Some(tag.into());
        e.name_id = Some(name_id);
    }
    for (id, z) in [(NEAR, 5.0), (FAR, -30.0)] {
        mgr.create_entity(id, "Agnos", [0.0, 0.0, z], [0.0; 3])
            .unwrap();
        mgr.connect_entity(id);
    }
    mgr.resources.insert(catalog(WORLD_ID));
    mgr
}

fn drain(rx: &mut mpsc::Receiver<CellToBaseMsg>) -> Vec<(u32, Vec<u8>)> {
    let mut out = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        match msg {
            CellToBaseMsg::EntityMethodCall {
                entity_id,
                method_index,
                args,
            } => {
                assert_eq!(method_index, ON_PLAYER_COMMUNICATION);
                out.push((entity_id, args));
            }
            other => panic!("chatter sent something other than a method call: {other:?}"),
        }
    }
    out
}

fn say(speaker: &str, text: &str) -> Vec<u8> {
    serialize_on_player_communication(speaker, 0, CHAN_SAY, text)
}

/// The first line goes out on the first tick, as an `onPlayerCommunication`
/// say line from Ra, byte-identical to the chat serializer's, to the player
/// in earshot only. Revert proof: drop the radius filter in
/// `speak::listeners` and FAR hears it too.
#[tokio::test]
async fn the_first_line_reaches_only_the_player_in_earshot() {
    let mut mgr = world();
    let (tx, mut rx) = mpsc::channel(64);
    let t0 = Instant::now();

    run_at(&tx, &mut mgr, t0).await;

    assert_eq!(drain(&mut rx), vec![(NEAR, say("Ra", "The sun is mine."))]);
}

/// The scene plays at its own pace: line 2 waits its 4 s, then the 30 s gap,
/// then exchange 2.
#[tokio::test]
async fn the_scene_keeps_its_timing() {
    let mut mgr = world();
    let (tx, mut rx) = mpsc::channel(64);
    let t0 = Instant::now();
    run_at(&tx, &mut mgr, t0).await;
    drain(&mut rx);

    run_at(&tx, &mut mgr, t0 + Duration::from_millis(3900)).await;
    assert!(drain(&mut rx).is_empty(), "line 2 is not due before 4 s");
    let t1 = t0 + Duration::from_millis(4000);
    run_at(&tx, &mut mgr, t1).await;
    assert_eq!(drain(&mut rx), vec![(NEAR, say("Ba'al", "Paperwork, Ra."))]);

    run_at(&tx, &mut mgr, t1 + Duration::from_secs(29)).await;
    assert!(drain(&mut rx).is_empty(), "quiet for the exchange gap");
    run_at(&tx, &mut mgr, t1 + Duration::from_secs(30)).await;
    assert_eq!(drain(&mut rx), vec![(NEAR, say("Ba'al", "Cappuccino?"))]);
}

/// With nobody in earshot the lords stay quiet, and the player who walks up
/// later hears the first scene from its first line. Revert proof: start the
/// exchange without `anyone_in_earshot` and the walk-up hears line 2 or
/// nothing.
#[tokio::test]
async fn an_empty_room_waits_for_a_listener() {
    let mut mgr = world();
    mgr.get_entity_mut(NEAR).unwrap().position.z = 40.0;
    let (tx, mut rx) = mpsc::channel(64);
    let t0 = Instant::now();

    run_at(&tx, &mut mgr, t0).await;
    run_at(&tx, &mut mgr, t0 + Duration::from_secs(10)).await;
    assert!(drain(&mut rx).is_empty());

    mgr.get_entity_mut(NEAR).unwrap().position.z = 5.0;
    let t1 = t0 + Duration::from_secs(10) + IDLE_RECHECK;
    run_at(&tx, &mut mgr, t1).await;
    assert_eq!(drain(&mut rx), vec![(NEAR, say("Ra", "The sun is mine."))]);
}

/// A dead speaker's line is skipped with one WARN naming the tag, and the
/// scene goes on: Ba'al still speaks. A second pass over the same scene does
/// not warn again.
#[tokio::test]
async fn a_dead_speaker_is_skipped_and_warned_once() {
    let capture = LogCapture::install();
    let mut mgr = world();
    mgr.get_entity_mut(RA).unwrap().state_field |= BSF_DEAD;
    let (tx, mut rx) = mpsc::channel(64);
    let t0 = Instant::now();

    run_at(&tx, &mut mgr, t0).await;
    assert!(drain(&mut rx).is_empty(), "Ra is dead: line 1 is skipped");
    let t1 = t0 + Duration::from_secs(4);
    run_at(&tx, &mut mgr, t1).await;
    assert_eq!(drain(&mut rx), vec![(NEAR, say("Ba'al", "Paperwork, Ra."))]);

    let warned = capture
        .find_event(
            Level::WARN,
            "ambient chatter line skipped",
            "no_living_npc_with_tag",
        )
        .expect("a missing speaker warns");
    assert!(warned.has_field("speaker_tag", "Lords_Ra"));

    // Exchange 2, then exchange 1 again: still no second WARN.
    run_at(&tx, &mut mgr, t1 + Duration::from_secs(30)).await;
    run_at(&tx, &mut mgr, t1 + Duration::from_secs(60)).await;
    let warns = capture
        .all()
        .into_iter()
        .filter(|c| c.level == Level::WARN && c.has_field("reason", "no_living_npc_with_tag"))
        .count();
    assert_eq!(warns, 1);
}

/// A group whose world this cell does not load stays silent and says so
/// once; no catalog at all is not an error.
#[tokio::test]
async fn a_group_in_an_unloaded_world_is_silent() {
    let capture = LogCapture::install();
    let mut mgr = world();
    mgr.resources.insert(catalog(9999));
    let (tx, mut rx) = mpsc::channel(64);
    run_at(&tx, &mut mgr, Instant::now()).await;
    assert!(drain(&mut rx).is_empty());
    assert!(capture
        .find_event(Level::WARN, "no shared space", "world_not_loaded")
        .is_some());

    let mut bare = make_space_manager();
    run_at(&tx, &mut bare, Instant::now()).await;
    assert!(drain(&mut rx).is_empty());
}

/// A group with no gap and no line delays still returns from the tick: one
/// exchange starts per group per tick, so the cell loop is never stuck
/// replaying scenes. Revert proof: drop the one-start rule in `run_at` and
/// this test never finishes.
#[tokio::test]
async fn a_zero_gap_group_starts_one_exchange_per_tick() {
    let mut mgr = world();
    let mut cat = catalog(WORLD_ID);
    cat.groups[0].exchange_gap = Duration::ZERO;
    for ex in &mut cat.groups[0].exchanges {
        for l in &mut ex.lines {
            l.delay = Duration::ZERO;
        }
    }
    mgr.resources.insert(cat);
    let (tx, mut rx) = mpsc::channel(256);
    let t0 = Instant::now();
    run_at(&tx, &mut mgr, t0).await;
    // Exchange 1 (two lines) plays in full; exchange 2 waits for the next tick.
    assert_eq!(drain(&mut rx).len(), 2);
    run_at(&tx, &mut mgr, t0).await;
    assert_eq!(drain(&mut rx), vec![(NEAR, say("Ba'al", "Cappuccino?"))]);
}
