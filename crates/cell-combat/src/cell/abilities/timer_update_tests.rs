//! `onTimerUpdate` routing guards: owner only, never an NPC's witnesses.

use tracing::Level;

use super::*;
use crate::cell::abilities::{send_entity_method, send_entity_method_to_witnesses};
use crate::test_support::LogCapture;

const PLAYER: u32 = 1;
const OTHER_PLAYER: u32 = 2;
const NPC: u32 = 3;

/// Players 1 and 2 and NPC 3 together in one space, AoI computed: both
/// players witness the NPC and each other.
fn scene() -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    mgr.parse_spaces_xml(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#,
    )
    .unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#,
    )
    .unwrap();
    for id in [PLAYER, OTHER_PLAYER, NPC] {
        mgr.create_entity(id, "Castle", [0.0; 3], [0.0; 3]).unwrap();
    }
    for id in [PLAYER, OTHER_PLAYER] {
        let p = mgr.get_entity_mut(id).unwrap();
        p.is_player = true;
        p.player_id = Some(100 + id as i32);
        mgr.connect_entity(id);
    }
    let _ = mgr.compute_aoi_changes();
    assert!(
        mgr.get_witnesses_of(NPC).contains(&PLAYER),
        "fixture: the player must witness the NPC"
    );
    mgr
}

fn drain(rx: &mut mpsc::Receiver<CellToBaseMsg>) -> Vec<CellToBaseMsg> {
    let mut out = Vec::new();
    while let Ok(m) = rx.try_recv() {
        out.push(m);
    }
    out
}

fn timer_args() -> Vec<u8> {
    cimmeria_entity::abilities::serialize_timer_update(
        559,
        cimmeria_entity::abilities::TIMER_ABILITY_COOLDOWN,
        NPC as i32,
        0,
        1.5,
        100.0,
    )
}

/// A player's timer goes to their own client, unchanged, and to nobody else.
#[tokio::test]
async fn a_players_timer_goes_to_their_own_client_only() {
    let mgr = scene();
    let (tx, mut rx) = mpsc::channel(16);
    let args = timer_args();

    let route = send_timer_update(PLAYER, args.clone(), &tx, &mgr).await;

    assert_eq!(route, TimerRoute::Owner);
    let msgs = drain(&mut rx);
    assert_eq!(msgs.len(), 1, "{msgs:#?}");
    match &msgs[0] {
        CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index,
            args: sent,
        } => {
            assert_eq!(*entity_id, PLAYER);
            assert_eq!(*method_index, ON_TIMER_UPDATE);
            assert_eq!(sent, &args, "the 21 bytes pass through untouched");
        }
        other => panic!("expected the owner's EntityMethodCall, got {other:?}"),
    }
}

/// The regression guard for the colo 2026-09-29 drops. An NPC's timer is
/// sent to nobody: before the fix it went to every witness, whose client
/// has no `onTimerUpdate` handler for `SGWMob` and drops it.
#[tokio::test]
async fn an_npc_timer_is_sent_to_nobody() {
    let mgr = scene();
    let (tx, mut rx) = mpsc::channel(16);
    let logs = LogCapture::install();

    let route = send_timer_update(NPC, timer_args(), &tx, &mgr).await;

    assert_eq!(route, TimerRoute::NotPlayer);
    assert!(drain(&mut rx).is_empty(), "no witness may receive it");
    let skip = logs
        .find_event(Level::DEBUG, "onTimerUpdate not sent", "not_player")
        .expect("the skip is logged at DEBUG with reason=not_player");
    assert!(skip.has_field("entity_id", "3"), "{skip:?}");
}

/// The fallback guard: a caller that bypasses `send_timer_update` and hands
/// method 12 about an NPC to `send_entity_method` sends nothing and WARNs
/// with `reason = no_client_binding`, so the bypass shows up in SigNoz.
#[tokio::test]
async fn send_entity_method_refuses_an_npc_timer_and_warns() {
    let mgr = scene();
    let (tx, mut rx) = mpsc::channel(16);
    let logs = LogCapture::install();

    send_entity_method(NPC, ON_TIMER_UPDATE, timer_args(), &tx, &mgr).await;

    assert!(drain(&mut rx).is_empty(), "nothing fans out to witnesses");
    let warn = logs
        .find_event(Level::WARN, "not fanned out", "no_client_binding")
        .expect("the refused fanout WARNs with reason=no_client_binding");
    assert!(warn.has_field("via", "send_entity_method"), "{warn:?}");
}

/// Same guard on the strict witness helper.
#[tokio::test]
async fn witness_fanout_refuses_an_npc_timer() {
    let mgr = scene();
    let (tx, mut rx) = mpsc::channel(16);

    let count =
        send_entity_method_to_witnesses(NPC, ON_TIMER_UPDATE, timer_args(), &tx, &mgr).await;

    assert_eq!(count, 0);
    assert!(drain(&mut rx).is_empty());
}

/// The guard is scoped: other NPC methods still fan out (here
/// `onStateFieldUpdate`, 19), and a player's method 12 to other players is
/// not refused, since `SGWPlayer` does bind `onTimerUpdate`.
#[tokio::test]
async fn the_guard_does_not_block_other_methods_or_player_entities() {
    let mgr = scene();
    let (tx, mut rx) = mpsc::channel(16);

    send_entity_method(NPC, 19, vec![0; 4], &tx, &mgr).await;
    let npc_fanout = drain(&mut rx);
    assert!(
        npc_fanout.iter().any(|m| matches!(
            m,
            CellToBaseMsg::WitnessEntityMethod {
                witness_id: PLAYER,
                entity_id: NPC,
                method_index: 19,
                ..
            }
        )),
        "{npc_fanout:#?}"
    );

    let count =
        send_entity_method_to_witnesses(PLAYER, ON_TIMER_UPDATE, timer_args(), &tx, &mgr).await;
    assert_eq!(count, 1, "player 2 witnesses player 1");
}
