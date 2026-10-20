//! Castle return-hub barks (CS-05c): `castle_hub_barks_chains.sql` chains
//! 10200 (Ogilvie, after 1520) and 10201 (Castle Medic, after 1521), both on
//! `enter_region Castle.Infirmary`, pattern `npc_bark.rs`.
//!
//! Each bark goes through the real `fire_enter_region` dispatcher and the
//! executor's `npc_bark` arm, and the decoded `onPlayerCommunication` is what
//! the client would show, so the seeded `dialog_screens` text is pinned on the
//! wire. The `once` gate lives on the cell entity, so the re-entry test reuses
//! one `SpaceManager`. Every test skips cleanly without `DATABASE_URL`.

use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_entity::missions::{MissionInstance, MISSION_COMPLETED};
use sqlx::PgPool;
use tokio::sync::mpsc;

use super::super::fire_enter_region;
use super::mission_1521_entry::{
    completed_704, engine_with, mission_at, mission_of, staged, CASTLE, ENTITY, PLAYER,
};
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use crate::cell::spawner;
use crate::test_support::require_db_or_skip;

const INFIRMARY: &str = "Castle.Infirmary";
const MISSION_1520: i32 = 1520;
const MISSION_1521: i32 = 1521;
/// Chains 1367, 1368 and 1369 are the Infirmary offers this pair must not
/// collide with; 10200 and 10201 are the barks under test.
const OFFERS: [i32; 3] = [1367, 1368, 1369];
const OGILVIE: &str = "Ogilvie";
const MEDIC: &str = "Castle Medic";
const OGILVIE_SCREEN: i32 = 10900;
const MEDIC_SCREEN: i32 = 10901;
const OGILVIE_DIALOG: i32 = 10740;
const MEDIC_DIALOG: i32 = 10741;
const OGILVIE_TEXT: &str = "The Ambernol run bought these people time, and I will not forget who made it. Keep your head down out there.";
const MEDIC_TEXT: &str = "The tank held, and so did the people who depend on it. Rest while you can; the Castle needs every warrior standing.";
/// `onPlayerCommunication`, as in `npc_bark.rs`. Spelled out so a change to
/// the constant cannot make this assertion agree with itself.
const ON_PLAYER_COMMUNICATION: u16 = 28;

/// A decoded bark: speaker, channel, text.
type Bark = (String, u8, String);

/// A bark as the seed asks for it: channel `say` (0), speaker `speaker`.
fn bark(speaker: &str, text: &str) -> Bark {
    (speaker.to_string(), 0, text.to_string())
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

/// Decode a `onPlayerCommunication` payload: speaker WSTRING, SpeakerFlags
/// byte, Channel byte, text WSTRING.
fn decode_bark(args: &[u8]) -> Bark {
    let (speaker, speaker_units) = read_wstring(args, 0);
    let flags_off = 4 + speaker_units * 2;
    let channel = args[flags_off + 1];
    let (text, _) = read_wstring(args, flags_off + 2);
    (speaker, channel, text)
}

/// The barks among `msgs`, in order.
fn barks(msgs: &[CellToBaseMsg]) -> Vec<Bark> {
    msgs.iter()
        .filter_map(|msg| match msg {
            CellToBaseMsg::EntityMethodCall {
                method_index, args, ..
            } if *method_index == ON_PLAYER_COMMUNICATION => Some(decode_bark(args)),
            _ => None,
        })
        .collect()
}

/// Fire the real `enter_region` dispatcher for the Infirmary and drain the
/// cell-to-base traffic it produced.
async fn enter(engine: &ChainEngine, mgr: &mut SpaceManager) -> Vec<CellToBaseMsg> {
    let (tx, mut rx) = mpsc::channel(256);
    fire_enter_region(ENTITY, PLAYER, INFIRMARY, engine, &tx, mgr).await;
    let mut out = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        out.push(msg);
    }
    out
}

/// A character in `world` with `archetype` and `missions`, and the startup
/// `dialog_screens` cache the executor resolves bark text from.
async fn stage_hub(
    pool: &PgPool,
    world: &str,
    archetype: i32,
    missions: Vec<MissionInstance>,
) -> SpaceManager {
    let mut mgr = staged(pool, world, Some(archetype), missions).await;
    mgr.dialog_screen_text = spawner::load_dialog_screen_text(pool)
        .await
        .expect("dialog screen text must load");
    mgr
}

/// A completed mission record for `id`, with no active step.
fn completed(id: i32) -> MissionInstance {
    let mut mission = mission_at(id, None, &[]);
    mission.status = MISSION_COMPLETED;
    mission
}

/// A 1520 record that is still in progress. Absent from the record means
/// `not_active`.
fn active_1520() -> MissionInstance {
    mission_at(MISSION_1520, None, &[])
}

#[tokio::test]
async fn live_db_castle_hub_barks_b1_fires_once_for_1520_completed_low_archetype() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &[10200]).await;
    let mut mgr = stage_hub(
        &pool,
        CASTLE,
        3,
        vec![completed_704(), completed(MISSION_1520)],
    )
    .await;

    let msgs = enter(&engine, &mut mgr).await;
    assert_eq!(
        barks(&msgs),
        vec![bark(OGILVIE, OGILVIE_TEXT)],
        "1520 completed and archetype 3 must give exactly chain 10200's bark \
         with the seeded screen {OGILVIE_SCREEN} text",
    );
}

#[tokio::test]
async fn live_db_castle_hub_barks_b1_does_not_repeat_on_reentry() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &[10200]).await;
    let mut mgr = stage_hub(
        &pool,
        CASTLE,
        3,
        vec![completed_704(), completed(MISSION_1520)],
    )
    .await;

    assert_eq!(barks(&enter(&engine, &mut mgr).await).len(), 1);
    assert!(
        barks(&enter(&engine, &mut mgr).await).is_empty(),
        "chain 10200 is once = true; a second entry in the same session must be silent",
    );
}

#[tokio::test]
async fn live_db_castle_hub_barks_b1_gates_hold_on_1520_archetype_and_world() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &[10200]).await;
    let cases = [
        (
            "1520 active",
            CASTLE,
            3,
            vec![completed_704(), active_1520()],
        ),
        ("1520 not_active", CASTLE, 3, vec![completed_704()]),
        (
            "archetype 8",
            CASTLE,
            8,
            vec![completed_704(), completed(MISSION_1520)],
        ),
        (
            "archetype 7, past the lte 6 bound",
            CASTLE,
            7,
            vec![completed_704(), completed(MISSION_1520)],
        ),
        (
            "another world",
            "Castle_CellBlock",
            3,
            vec![completed_704(), completed(MISSION_1520)],
        ),
    ];
    for (what, world, archetype, missions) in cases {
        let mut mgr = stage_hub(&pool, world, archetype, missions).await;
        let msgs = enter(&engine, &mut mgr).await;
        assert!(
            barks(&msgs).is_empty(),
            "chain 10200 must stay silent for {what}; got {:?}",
            barks(&msgs),
        );
    }
}
#[tokio::test]
async fn live_db_castle_hub_barks_b2_fires_once_for_archetype_8_after_1521() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &[10201]).await;
    let mut mgr = stage_hub(
        &pool,
        CASTLE,
        8,
        vec![completed_704(), completed(MISSION_1521)],
    )
    .await;

    let msgs = enter(&engine, &mut mgr).await;
    assert_eq!(
        barks(&msgs),
        vec![bark(MEDIC, MEDIC_TEXT)],
        "1521 completed, 1520 not_active and archetype 8 must give exactly \
         chain 10201's bark with the seeded screen {MEDIC_SCREEN} text",
    );
}

#[tokio::test]
async fn live_db_castle_hub_barks_b2_gates_hold_on_archetype_1521_1520_and_world() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &[10201]).await;
    let cases = [
        (
            "archetype 3",
            CASTLE,
            3,
            vec![completed_704(), completed(MISSION_1521)],
        ),
        (
            "1521 active",
            CASTLE,
            8,
            vec![completed_704(), mission_at(MISSION_1521, None, &[])],
        ),
        (
            "1520 completed",
            CASTLE,
            8,
            vec![
                completed_704(),
                completed(MISSION_1521),
                completed(MISSION_1520),
            ],
        ),
        (
            "1520 active",
            CASTLE,
            8,
            vec![completed_704(), completed(MISSION_1521), active_1520()],
        ),
        (
            "another world",
            "Castle_CellBlock",
            8,
            vec![completed_704(), completed(MISSION_1521)],
        ),
    ];
    for (what, world, archetype, missions) in cases {
        let mut mgr = stage_hub(&pool, world, archetype, missions).await;
        let msgs = enter(&engine, &mut mgr).await;
        assert!(
            barks(&msgs).is_empty(),
            "chain 10201 must stay silent for {what}; got {:?}",
            barks(&msgs),
        );
    }
}
#[tokio::test]
async fn live_db_castle_hub_barks_are_disjoint_from_the_infirmary_offers() {
    let pool = require_db_or_skip!();
    let mut ids = OFFERS.to_vec();
    ids.extend([10200, 10201]);
    let engine = engine_with(&pool, &ids).await;

    // 1520 completed, archetype 3: only 10200 speaks, and no offer accepts.
    let mut mgr = stage_hub(
        &pool,
        CASTLE,
        3,
        vec![completed_704(), completed(MISSION_1520)],
    )
    .await;
    let msgs = enter(&engine, &mut mgr).await;
    assert_eq!(barks(&msgs), vec![bark(OGILVIE, OGILVIE_TEXT)]);
    assert!(
        mission_of(&mgr, MISSION_1521).is_none(),
        "no offer may accept 1521 next to the 1520 bark",
    );

    // 1521 completed, 1520 not_active, archetype 8: only 10201 speaks.
    let mut mgr = stage_hub(
        &pool,
        CASTLE,
        8,
        vec![completed_704(), completed(MISSION_1521)],
    )
    .await;
    let msgs = enter(&engine, &mut mgr).await;
    assert_eq!(barks(&msgs), vec![bark(MEDIC, MEDIC_TEXT)]);
    assert!(
        mission_of(&mgr, MISSION_1520).is_none(),
        "no offer may accept 1520 next to the 1521 bark",
    );
}

#[tokio::test]
async fn live_db_castle_hub_barks_seed_screens_match_the_frozen_text() {
    let pool = require_db_or_skip!();
    let screens = spawner::load_dialog_screen_text(&pool)
        .await
        .expect("dialog screen text must load");
    assert_eq!(
        screens.get(&OGILVIE_SCREEN).map(String::as_str),
        Some(OGILVIE_TEXT),
        "screen {OGILVIE_SCREEN} must carry the frozen Ogilvie line",
    );
    assert_eq!(
        screens.get(&MEDIC_SCREEN).map(String::as_str),
        Some(MEDIC_TEXT),
        "screen {MEDIC_SCREEN} must carry the frozen Castle Medic line",
    );

    for (screen, dialog) in [
        (OGILVIE_SCREEN, OGILVIE_DIALOG),
        (MEDIC_SCREEN, MEDIC_DIALOG),
    ] {
        let found: i32 = sqlx::query_scalar(
            "SELECT dialog_id FROM resources.dialog_screens WHERE screen_id = $1",
        )
        .bind(screen)
        .fetch_one(&pool)
        .await
        .expect("screen row must exist");
        assert_eq!(
            found, dialog,
            "screen {screen} must sit under dialog {dialog}"
        );
    }
}
