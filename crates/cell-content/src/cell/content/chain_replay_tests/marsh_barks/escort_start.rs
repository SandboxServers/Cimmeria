//! Chain 1176 — "Let's move out!" on the ring ride to the topside route.
//!
//! This module also owns the packet's single executor-level test. A
//! resolve-only test cannot tell a wired `Action::NpcBark` arm from the
//! `other =>` catch-all, and it cannot see the line's text at all: the
//! seed row carries only a `screen_id`, and the executor resolves the
//! text server-side from the cell's `dialog_screens` startup cache. The
//! `SpaceManager` fixture and the wire reader therefore live here beside
//! their only caller rather than in [`super`].

use cimmeria_content_engine::chain::ChainEngine;
use tokio::sync::mpsc;

use super::super::super::executor::execute_actions;
use super::{
    assert_refused, assert_single_bark, engine_with, load, resolve_teleport_in, CHAIN_MOVE_OUT,
    CHAN_SAY, MARSH_SPEAKER, SCREEN_MOVE_OUT,
};
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use crate::cell::spawner;
use crate::test_support::require_db_or_skip;

/// The seeded text of screen 96351, asserted rather than assumed: the
/// seed row carries only the id, so an edit to that `dialog_screens` row
/// silently changes what a player hears.
const MOVE_OUT_TEXT: &str = "Let's move out!";

/// `onPlayerCommunication`. Spelled out rather than imported from
/// `method_idx` so a change to the constant cannot make the assertion
/// agree with itself.
const ON_PLAYER_COMMUNICATION: u16 = 28;

const PLAYER_EID: u32 = 7601;
const PLAYER_ID: i32 = 42;

/// Happy path: ringing up to the topside route while mission 680 is still
/// on step 2344 speaks the departure line.
#[tokio::test]
async fn chain_1176_ring_ride_while_step_2344_active_barks_the_departure_line() {
    let pool = require_db_or_skip!();
    let engine = engine_with(load(&pool, CHAIN_MOVE_OUT).await);

    let resolved = resolve_teleport_in(&engine, 3, &[("mission_680_step_2344_status", "active")]);
    assert_single_bark(&resolved, CHAIN_MOVE_OUT as i64, SCREEN_MOVE_OUT);
}

/// Adjacent wrong state — phase already passed. Step 2344 retires into
/// `completed_steps` the moment chain 1072 advances 680 to 2345 on this
/// same edge, so a second ring ride (die topside, respawn at a
/// Preparation-room respawner, walk back to ring switch 2) must not
/// re-speak the line.
///
/// This is the case that forced the gate off `mission_status 680 eq
/// active`: mission 680 stays active all the way to Region9, so a mission
/// gate would still be open on that second ride.
#[tokio::test]
async fn chain_1176_does_not_re_bark_on_a_second_ring_ride() {
    let pool = require_db_or_skip!();
    let engine = engine_with(load(&pool, CHAIN_MOVE_OUT).await);

    let resolved = resolve_teleport_in(
        &engine,
        3,
        &[
            ("mission_680_status", "active"),
            ("mission_680_step_2344_status", "completed"),
            ("mission_680_step_2345_status", "active"),
        ],
    );
    assert_refused(
        &resolved,
        CHAIN_MOVE_OUT as i64,
        "step 2344 has already been advanced past (mission 680 is still \
         active, which is exactly why a mission-status gate would not have \
         closed here)",
    );
}

/// Adjacent wrong state — wrong ring. Ring 2 is the downstairs hop chain
/// 1044 owns; only ring 3 is the topside arrival.
#[tokio::test]
async fn chain_1176_does_not_fire_on_the_other_ring_transport() {
    let pool = require_db_or_skip!();
    let engine = engine_with(load(&pool, CHAIN_MOVE_OUT).await);

    let resolved = resolve_teleport_in(&engine, 2, &[("mission_680_step_2344_status", "active")]);
    assert_refused(
        &resolved,
        CHAIN_MOVE_OUT as i64,
        "the player teleported into ring region 2, not the topside ring 3",
    );
}

/// Chain 1176 end to end: resolve the real seed row, run it through
/// [`execute_actions`], and assert the emitted method-28 call carries
/// Marsh's actual 2009 line, resolved from `resources.dialog_screens`.
///
/// Three separate things fail this and nothing else catches them:
/// the executor losing its `Action::NpcBark` arm (zero calls, the action
/// falls into the `other =>` catch-all), the `dialog_screen_text` startup
/// cache going away (zero calls, `screen_not_cached`), and the seed row
/// naming a screen whose text has changed (wrong bytes on the wire).
#[tokio::test]
async fn chain_1176_executes_the_seeded_line_onto_the_wire() {
    let pool = require_db_or_skip!();
    let engine = engine_with(load(&pool, CHAIN_MOVE_OUT).await);

    let resolved = resolve_teleport_in(&engine, 3, &[("mission_680_step_2344_status", "active")]);
    assert_single_bark(&resolved, CHAIN_MOVE_OUT as i64, SCREEN_MOVE_OUT);

    let mut mgr = make_space_mgr();
    stage_player(&mut mgr, PLAYER_EID);
    // The real startup loader against the real seed — what a running cell
    // has in the cache.
    mgr.dialog_screen_text = spawner::load_dialog_screen_text(&pool)
        .await
        .expect("dialog screen text must load");

    let (tx, mut rx) = mpsc::channel(32);
    let exec_engine = ChainEngine::new();
    execute_actions(resolved, PLAYER_EID, PLAYER_ID, &tx, &mut mgr, &exec_engine).await;

    let mut calls = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        if let CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index,
            args,
        } = msg
        {
            calls.push((entity_id, method_index, args));
        }
    }
    assert_eq!(
        calls.len(),
        1,
        "chain 1176 must emit exactly one EntityMethodCall; got {:?}. Zero \
         means the executor has no Action::NpcBark arm, or the screen-text \
         cache did not load",
        calls.iter().map(|(e, m, _)| (*e, *m)).collect::<Vec<_>>(),
    );
    let (target, method_index, args) = &calls[0];
    assert_eq!(
        *target, PLAYER_EID,
        "the bark is addressed to the ringing player"
    );
    assert_eq!(*method_index, ON_PLAYER_COMMUNICATION);

    let (speaker, speaker_units) = read_wstring(args, 0);
    assert_eq!(speaker, MARSH_SPEAKER, "speaker comes from the seed param");
    let flags_off = 4 + speaker_units * 2;
    assert_eq!(args[flags_off], 0, "SpeakerFlags must be SPEAKER_None");
    assert_eq!(args[flags_off + 1], CHAN_SAY, "Channel must be CHAN_say");

    let (text, _) = read_wstring(args, flags_off + 2);
    assert_eq!(
        text, MOVE_OUT_TEXT,
        "the line on the wire must be the seeded dialog 5019 screen \
         {SCREEN_MOVE_OUT} text, resolved server-side — the seed row carries \
         only the id, so this is the only assertion that pins what the player \
         actually hears"
    );
}

fn make_space_mgr() -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" Instanced="false" MinX="-450" MaxX="450" MinY="-450" MaxY="450" /></Spaces>"#;
    let cxml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(cxml).unwrap();
    mgr
}

fn stage_player(mgr: &mut SpaceManager, eid: u32) {
    mgr.create_entity(
        eid,
        "Castle_CellBlock",
        [-91.689, 45.188, -161.533],
        [0.0; 3],
    )
    .expect("Castle_CellBlock startup space must accept the player entity");
    let p = mgr
        .get_entity_mut(eid)
        .expect("player entity must exist immediately after create_entity");
    p.is_player = true;
    p.player_id = Some(PLAYER_ID);
    mgr.connect_entity(eid);
}

/// Read a WSTRING at `offset`: a `u32` UTF-16 code-unit count followed by
/// that many little-endian `u16`s. Returns the decoded string and the
/// **code-unit count** (not the byte length), so the caller keeps doing
/// its offset arithmetic in the wire's own terms.
fn read_wstring(args: &[u8], offset: usize) -> (String, usize) {
    let units = u32::from_le_bytes(args[offset..offset + 4].try_into().unwrap()) as usize;
    let (pairs, _) = args[offset + 4..offset + 4 + units * 2].as_chunks::<2>();
    let s = char::decode_utf16(pairs.iter().copied().map(u16::from_le_bytes))
        .map(|r| r.expect("wire text must be valid UTF-16"))
        .collect();
    (s, units)
}
