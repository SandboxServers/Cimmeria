//! `npc_bark` chain-replay guard (DU-03).
//!
//! Like [`super::grant_xp`], this module seeds its own chain: there are
//! **zero** `npc_bark` rows in `db/resources/Content/Seed/` today (packet
//! DU-07 authors the first ones), so a sentinel chain is the only way to
//! guard both halves of the verb before content reaches it.
//!
//! Three halves are actually load-bearing here, and each fails
//! separately:
//!
//! - Removing the `"npc_bark"` arm from
//!   `crates/content-engine/src/loader/action.rs` (or the
//!   `action_bark` module behind it) makes `convert_action` return
//!   `None`, the chain loads with zero actions, and the resolve
//!   assertion fails.
//! - Removing the `Action::NpcBark` arm from
//!   `crates/services/src/cell/content/executor/mod.rs` drops execution
//!   into the `other =>` catch-all and the `EntityMethodCall` assertion
//!   fails.
//! - Removing the `dialog_screen_text` startup cache (or its
//!   `spawner::load_dialog_screen_text` loader) leaves the executor with
//!   no text to resolve; it warns `screen_not_cached` and sends nothing,
//!   and the same assertion fails.
//!
//! The text is loaded from the database rather than stubbed, and the
//! screen id is a real one — dialog 5019 screen 96351, Col. Marsh's
//! "Let's move out!" — so this also pins that the seeded line still says
//! what the bark seed rows assume it says.
//!
//! Sentinel id range: `0x7000_5100`. The sibling reservations in
//! `crates/services` run `0x7000_1000..0x7000_1B00`, `0x7000_2000`,
//! `0x7000_3000`, `0x7000_4000`, `0x7000_4242` and `0x7000_5000`
//! (`grant_xp`); this steps past all of them. Cleanup deletes the exact
//! ids inserted, never a range.

use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_content_engine::context::ExecutionContext;
use cimmeria_content_engine::triggers::{TriggerEvent, TriggerType};
use sqlx::PgPool;
use tokio::sync::mpsc;

use super::super::engine_loader::load_single_chain_for_test;
use super::super::executor::execute_actions;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use crate::cell::spawner;
use crate::test_support::require_db_or_skip;

/// Sentinel `content_chains.chain_id`. Fits in `i32` (the column type).
const TEST_CHAIN_ID: i32 = 0x7000_5100;
/// Sentinel interact tag — namespaced so it can never collide with a
/// real `content_triggers.event_key`.
const TEST_TAG: &str = "CIMMERIA_TEST_NPC_BARK_TAG";

/// Dialog 5019 screen 96351 in `db/resources/Dialogs/Seed/dialog_screens.sql`.
const MARSH_SCREEN: i32 = 96351;
/// The seeded text for that screen. Asserted, not assumed: a bark seed
/// row carries only the id, so a change to this row silently changes
/// what the player hears.
const MARSH_TEXT: &str = "Let's move out!";
const MARSH_SPEAKER: &str = "Col. Marsh";

const PLAYER_EID: u32 = 7301;
const WITNESS_EID: u32 = 7302;
/// `onPlayerCommunication`, the only non-modal text route the client
/// honours. Spelled out rather than imported so a change to the
/// `method_idx` constant cannot make this assertion agree with itself.
const ON_PLAYER_COMMUNICATION: u16 = 28;

async fn seed_sentinel_chain(pool: &PgPool) {
    sqlx::query(
        "INSERT INTO resources.content_chains \
         (chain_id, description, scope_type, scope_id, enabled, priority) \
         VALUES ($1, 'npc_bark chain-replay sentinel', 'space', NULL, true, 0)",
    )
    .bind(TEST_CHAIN_ID)
    .execute(pool)
    .await
    .expect("sentinel content_chains insert must succeed");

    sqlx::query(
        "INSERT INTO resources.content_triggers \
         (chain_id, event_type, event_key, scope, once, sort_order) \
         VALUES ($1, 'interact_tag', $2, 'player', false, 0)",
    )
    .bind(TEST_CHAIN_ID)
    .bind(TEST_TAG)
    .execute(pool)
    .await
    .expect("sentinel content_triggers insert must succeed");

    // The exact param shape DU-07 will author.
    sqlx::query(
        "INSERT INTO resources.content_actions \
         (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) \
         VALUES ($1, 'npc_bark', NULL, NULL, $2::jsonb, 0, 0)",
    )
    .bind(TEST_CHAIN_ID)
    .bind(format!(
        r#"{{"screen_id": {MARSH_SCREEN}, "speaker": "{MARSH_SPEAKER}", "channel": "say"}}"#
    ))
    .execute(pool)
    .await
    .expect("sentinel content_actions insert must succeed");
}

/// Delete by exact chain id, children first (FK order).
async fn cleanup_sentinel_chain(pool: &PgPool) {
    for stmt in [
        "DELETE FROM resources.content_actions WHERE chain_id = $1",
        "DELETE FROM resources.content_triggers WHERE chain_id = $1",
        "DELETE FROM resources.content_chains WHERE chain_id = $1",
    ] {
        sqlx::query(stmt)
            .bind(TEST_CHAIN_ID)
            .execute(pool)
            .await
            .expect("sentinel cleanup must succeed");
    }
}

fn make_space_mgr() -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" Instanced="false" MinX="0" MaxX="100" MinY="0" MaxY="100" /></Spaces>"#;
    let cxml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(cxml).unwrap();
    mgr
}

fn stage_player(mgr: &mut SpaceManager, eid: u32, player_id: i32) {
    mgr.create_entity(eid, "Agnos", [10.0, 0.0, 10.0], [0.0; 3])
        .expect("Agnos startup space must accept the player entity");
    let p = mgr
        .get_entity_mut(eid)
        .expect("player entity must exist immediately after create_entity");
    p.is_player = true;
    p.player_id = Some(player_id);
    mgr.connect_entity(eid);
}

/// Read a WSTRING at `offset`: a `u32` UTF-16 code-unit count followed by
/// that many little-endian `u16`s. Returns the decoded string and the
/// **code-unit count** (not the byte length) so the caller keeps doing
/// its own offset arithmetic in the wire's own terms.
fn read_wstring(args: &[u8], offset: usize) -> (String, usize) {
    let units = u32::from_le_bytes(args[offset..offset + 4].try_into().unwrap()) as usize;
    let (pairs, _) = args[offset + 4..offset + 4 + units * 2].as_chunks::<2>();
    let s = char::decode_utf16(pairs.iter().copied().map(u16::from_le_bytes))
        .map(|r| r.expect("wire text must be valid UTF-16"))
        .collect();
    (s, units)
}

/// A seeded `npc_bark` row must survive the loader, reach the executor,
/// resolve its text from the startup `dialog_screens` cache, and emit
/// exactly one `onPlayerCommunication` addressed to the firing player —
/// with the seeded 2009 line on the wire, not a retyped copy.
///
/// A witness is staged in the same space and pinned into the firing
/// player's witness set on purpose: `npc_bark` must not take the
/// say-chat broadcast path, which would send the line to that witness
/// too.
#[tokio::test]
async fn npc_bark_row_speaks_the_seeded_line_to_the_firing_player_only() {
    let pool = require_db_or_skip!();

    // Start from a clean slate in case a previous panicking run leaked.
    cleanup_sentinel_chain(&pool).await;
    seed_sentinel_chain(&pool).await;

    let loaded = load_single_chain_for_test(&pool, TEST_CHAIN_ID).await;

    // Drop the sentinel rows before asserting so a failure can't leave a
    // live chain registered in the shared test database.
    cleanup_sentinel_chain(&pool).await;

    let chain = loaded
        .expect("DB query for the sentinel chain must succeed")
        .expect("sentinel chain must load — a None here means the trigger row was rejected");

    let mut engine = ChainEngine::new();
    engine.register_chain(chain);

    let mut ctx = ExecutionContext::new();
    ctx.set_param("entity_tag".to_string(), serde_json::json!(TEST_TAG));
    let event = TriggerEvent {
        trigger_type: TriggerType::InteractTag,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };
    let resolved = engine.resolve_event(&event, &ctx);
    assert_eq!(
        resolved.actions.len(),
        1,
        "the sentinel chain's single npc_bark row must survive \
         convert_action — zero actions means the loader has no \"npc_bark\" \
         arm and the row was dropped with an \"Unknown action_type\" warn",
    );

    let mut mgr = make_space_mgr();
    stage_player(&mut mgr, PLAYER_EID, 42);
    stage_player(&mut mgr, WITNESS_EID, 43);
    mgr.get_entity_mut(PLAYER_EID)
        .expect("firing player must exist")
        .witnesses
        .insert(cimmeria_common::EntityId(WITNESS_EID as i32));

    // The real startup loader, against the real seed — this is what a
    // running cell has in the cache.
    mgr.dialog_screen_text = spawner::load_dialog_screen_text(&pool)
        .await
        .expect("dialog screen text must load");
    assert_eq!(
        mgr.dialog_screen_text
            .get(&MARSH_SCREEN)
            .map(String::as_str),
        Some(MARSH_TEXT),
        "dialog 5019 screen {MARSH_SCREEN} must still carry the escort line \
         the bark seed rows name by id",
    );

    let (tx, mut rx) = mpsc::channel(16);
    let exec_engine = ChainEngine::new();
    execute_actions(resolved, PLAYER_EID, 42, &tx, &mut mgr, &exec_engine).await;

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
        "one npc_bark action must produce exactly one EntityMethodCall; got \
         {:?} (zero means the executor has no Action::NpcBark arm and the \
         action fell through the `other =>` catch-all, or the screen text \
         cache was empty; two means it took the say-chat broadcast path)",
        calls.iter().map(|(e, m, _)| (*e, *m)).collect::<Vec<_>>(),
    );
    let (target, method_index, args) = &calls[0];
    assert_eq!(
        *target, PLAYER_EID,
        "the bark must be addressed to the triggering player, not the witness"
    );
    assert_eq!(*method_index, ON_PLAYER_COMMUNICATION);

    // Decode the payload rather than compare against the serializer:
    // this asserts the seeded TEXT reached the wire, which is the whole
    // point of resolving from `dialog_screens` instead of a param.
    let (speaker, speaker_units) = read_wstring(args, 0);
    assert_eq!(speaker, MARSH_SPEAKER, "speaker comes from the seed param");

    let flags_off = 4 + speaker_units * 2;
    assert_eq!(args[flags_off], 0, "SpeakerFlags must be SPEAKER_None");
    assert_eq!(args[flags_off + 1], 0, "Channel must be CHAN_say (0)");

    let text_off = flags_off + 2;
    let (text, text_units) = read_wstring(args, text_off);
    assert_eq!(
        text, MARSH_TEXT,
        "the line on the wire must be the seeded dialog_screens text, \
         resolved server-side from screen_id {MARSH_SCREEN}"
    );
    assert_eq!(
        args.len(),
        text_off + 4 + text_units * 2,
        "no trailing bytes after the text WSTRING"
    );
}
