//! Castle_CellBlock stasis-room debug hub — chain-replay guards for the five
//! chains in `db/resources/Content/Seed/debug_hub_chains.sql`.
//!
//! * 7001-7003: the dialog round trip. Click the NPC, dialog 60100 opens;
//!   its button fires 7002, which opens 60101; closing 60101 (-1) fires
//!   7003, which speaks a confirmation line into chat.
//! * 7004-7005: the Livewire round trip. Click the terminal, Livewire
//!   starts; the win fires 7005, which speaks a confirmation line.
//!
//! Every chain is loaded from the seeded database, resolved, and then pushed
//! through [`execute_actions`], asserting the `CellToBaseMsg` traffic. A
//! resolve-only test cannot tell a wired executor arm from the `other =>`
//! catch-all, and for the dialogs the executor is where the speaker is
//! resolved: 7002 has no NPC in its event, so it only reaches the client
//! through the player's `last_interaction_target` pin.
//!
//! The chains carry no conditions, so there is no gated negative to test;
//! the negatives here are "the wrong key fires nothing".

use cimmeria_content_engine::actions::Action;
use cimmeria_content_engine::chain::{ChainEngine, ResolvedActions};
use cimmeria_content_engine::context::ExecutionContext;
use cimmeria_content_engine::triggers::{TriggerEvent, TriggerType};
use sqlx::PgPool;
use tokio::sync::mpsc;

use super::super::engine_loader::load_single_chain_for_test;
use super::super::executor::execute_actions;
use super::livewire_pairs::{execute_and_drain_starts, resolve_interact};
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use crate::cell::spawner;
use crate::test_support::require_db_or_skip;

const PLAYER_EID: u32 = 7401;
const PLAYER_ID: i32 = 42;
/// The dialog NPC. Any id the manager hands out is fine; this one is fixed
/// so the wire assertions can name it.
const NPC_EID: u32 = 100_700;

/// Wire indices, spelled out so a change to the `method_idx` constants
/// cannot make an assertion agree with itself.
const ON_DIALOG_DISPLAY: u16 = 105;
const ON_PLAYER_COMMUNICATION: u16 = 28;

/// A Castle_CellBlock space with the player and the dialog NPC staged, and
/// the two dialog caches the executor reads loaded from the real seed.
async fn staged_space(pool: &PgPool) -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" Instanced="false" MinX="-1000" MaxX="1000" MinY="-1000" MaxY="1000" /></Spaces>"#;
    let cxml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(cxml).unwrap();
    mgr.create_entity(
        PLAYER_EID,
        "Castle_CellBlock",
        [-338.0, 73.472, -229.0],
        [0.0; 3],
    )
    .expect("the player must stage");
    let p = mgr.get_entity_mut(PLAYER_EID).unwrap();
    p.is_player = true;
    p.player_id = Some(PLAYER_ID);
    mgr.connect_entity(PLAYER_EID);
    mgr.spawn_npc(
        NPC_EID,
        "Castle_CellBlock",
        [-338.63, 73.472, -231.35],
        [0.0; 3],
    )
    .expect("the dialog NPC must stage");
    mgr.get_entity_mut(NPC_EID).unwrap().tag = Some("DebugHub_DialogNpc".into());

    mgr.monologue_dialog_ids = spawner::load_monologue_dialog_ids(pool)
        .await
        .expect("monologue dialog ids must load");
    mgr.dialog_screen_text = spawner::load_dialog_screen_text(pool)
        .await
        .expect("dialog screen text must load");
    mgr
}

async fn engine_with(pool: &PgPool, chain_id: i32) -> ChainEngine {
    let chain = load_single_chain_for_test(pool, chain_id)
        .await
        .unwrap_or_else(|e| panic!("DB query for chain {chain_id} must succeed: {e}"))
        .unwrap_or_else(|| panic!("chain {chain_id} must exist in seeded content_chains"));
    let mut engine = ChainEngine::new();
    engine.register_chain(chain);
    engine
}

/// Fire `dialog_choice` with the context `fire_dialog_choice` populates.
fn choose(engine: &ChainEngine, dialog_id: i32, button_id: i32) -> ResolvedActions {
    let mut ctx = ExecutionContext::new();
    ctx.set_param("dialog_id".to_string(), serde_json::json!(dialog_id));
    ctx.set_param("button_id".to_string(), serde_json::json!(button_id));
    let event = TriggerEvent {
        trigger_type: TriggerType::DialogChoice,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };
    engine.resolve_event(&event, &ctx)
}

/// Run `resolved` for the staged player and return every
/// `EntityMethodCall` as `(entity, method, args)`.
async fn execute(resolved: ResolvedActions, mgr: &mut SpaceManager) -> Vec<(u32, u16, Vec<u8>)> {
    let (tx, mut rx) = mpsc::channel(64);
    let exec_engine = ChainEngine::new();
    execute_actions(resolved, PLAYER_EID, PLAYER_ID, &tx, mgr, &exec_engine).await;
    let mut out = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        if let CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index,
            args,
        } = msg
        {
            out.push((entity_id, method_index, args));
        }
    }
    out
}

/// `(npc_entity_id, dialog_id)` from an `onDialogDisplay` payload.
fn dialog_display(call: &(u32, u16, Vec<u8>)) -> (i32, i32) {
    let (entity, method, args) = call;
    assert_eq!(
        *entity, PLAYER_EID,
        "the dialog goes to the clicking player"
    );
    assert_eq!(*method, ON_DIALOG_DISPLAY);
    (
        i32::from_le_bytes(args[0..4].try_into().unwrap()),
        i32::from_le_bytes(args[4..8].try_into().unwrap()),
    )
}

/// The chat text of an `onPlayerCommunication` payload: speaker WSTRING,
/// two flag bytes, text WSTRING.
fn bark_text(call: &(u32, u16, Vec<u8>)) -> (String, String) {
    let (entity, method, args) = call;
    assert_eq!(
        *entity, PLAYER_EID,
        "the bark goes to the triggering player"
    );
    assert_eq!(*method, ON_PLAYER_COMMUNICATION);
    let wstring = |offset: usize| {
        let units = u32::from_le_bytes(args[offset..offset + 4].try_into().unwrap()) as usize;
        let (pairs, _) = args[offset + 4..offset + 4 + units * 2].as_chunks::<2>();
        let s: String = char::decode_utf16(pairs.iter().copied().map(u16::from_le_bytes))
            .map(|r| r.unwrap())
            .collect();
        (s, units)
    };
    let (speaker, units) = wstring(0);
    let (text, _) = wstring(4 + units * 2 + 2);
    (speaker, text)
}

/// Chain 7001: clicking the dialog NPC opens 60100, spoken by that NPC.
/// The click's `target_entity_id` names the speaker, so the frame's
/// EntityId must be the NPC, not the player.
#[tokio::test]
async fn debug_hub_dialog_npc_click_opens_dialog_60100_as_the_npc() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, 7001).await;

    let mut ctx = ExecutionContext::new();
    ctx.set_param(
        "entity_tag".to_string(),
        serde_json::json!("DebugHub_DialogNpc"),
    );
    ctx.set_param("target_entity_id".to_string(), serde_json::json!(NPC_EID));
    let event = TriggerEvent {
        trigger_type: TriggerType::InteractTag,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };
    let resolved = engine.resolve_event(&event, &ctx);
    assert_eq!(
        resolved.actions,
        vec![(7001, Action::DisplayDialog { dialog_id: 60100 })],
        "chain 7001 must resolve exactly one display_dialog 60100",
    );

    let mut mgr = staged_space(&pool).await;
    let calls = execute(resolved, &mut mgr).await;
    assert_eq!(calls.len(), 1, "one onDialogDisplay; got {calls:?}");
    assert_eq!(dialog_display(&calls[0]), (NPC_EID as i32, 60100));

    // Another NPC's tag fires nothing.
    let mut wrong = ExecutionContext::new();
    wrong.set_param(
        "entity_tag".to_string(),
        serde_json::json!("DebugHub_Vendor"),
    );
    let event = TriggerEvent {
        trigger_type: TriggerType::InteractTag,
        source_entity: None,
        target_entity: None,
        params: wrong.params.clone(),
    };
    assert!(engine.resolve_event(&event, &wrong).actions.is_empty());
}

/// Chain 7002: 60100's button opens 60101. The choice event carries no
/// NPC, so the speaker comes from the player's `last_interaction_target`
/// pin; with it the frame names the NPC. If 60101 ever lost its speaker
/// (every screen speaker 0) it would become a monologue and bind the
/// player instead, and this fails.
#[tokio::test]
async fn debug_hub_dialog_button_opens_dialog_60101_through_the_pin() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, 7002).await;

    let resolved = choose(&engine, 60100, 8);
    assert_eq!(
        resolved.actions,
        vec![(7002, Action::DisplayDialog { dialog_id: 60101 })],
        "chain 7002 must resolve exactly one display_dialog 60101",
    );
    assert!(
        choose(&engine, 60101, 8).actions.is_empty(),
        "chain 7002 keys on dialog 60100 only"
    );

    let mut mgr = staged_space(&pool).await;
    mgr.get_entity_mut(PLAYER_EID)
        .unwrap()
        .last_interaction_target = Some(NPC_EID);
    let calls = execute(resolved, &mut mgr).await;
    assert_eq!(calls.len(), 1, "one onDialogDisplay; got {calls:?}");
    assert_eq!(dialog_display(&calls[0]), (NPC_EID as i32, 60101));
}

/// Chain 7003: closing 60101 (the client sends -1 for a button-less
/// dialog) speaks the confirmation line from screen 200003 to the player.
/// The line is read from the seeded `dialog_screens` row, so this also
/// pins that the row says what the hub doc promises.
#[tokio::test]
async fn debug_hub_dialog_close_confirms_in_chat() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, 7003).await;

    let resolved = choose(&engine, 60101, -1);
    assert_eq!(resolved.actions.len(), 1, "chain 7003: one npc_bark");
    assert!(
        choose(&engine, 60100, -1).actions.is_empty(),
        "chain 7003 keys on dialog 60101 only"
    );

    let mut mgr = staged_space(&pool).await;
    let calls = execute(resolved, &mut mgr).await;
    assert_eq!(calls.len(), 1, "one onPlayerCommunication; got {calls:?}");
    assert_eq!(
        bark_text(&calls[0]),
        (
            "Airman Lance".to_string(),
            "Dialog round trip complete: the button and the close both reached the server."
                .to_string()
        ),
    );
}

/// Chains 7004/7005: clicking the terminal starts exactly one Livewire
/// session whose win fires only 7005, at the default difficulty, with no
/// gate; and 7005 speaks the win line from screen 200004.
#[tokio::test]
async fn debug_hub_livewire_terminal_round_trip() {
    let pool = require_db_or_skip!();

    let actions = resolve_interact(&pool, 7004, "DebugHub_LivewireTerminal", &[]).await;
    assert_eq!(
        actions,
        vec![(
            7004,
            Action::StartMinigame {
                minigame_type: "Livewire".into(),
                difficulty: 1,
                on_victory_chains: vec![7005],
            }
        )],
        "chain 7004 must resolve exactly one StartMinigame(Livewire, [7005])",
    );
    assert!(
        resolve_interact(&pool, 7004, "DebugHub_DialogNpc", &[])
            .await
            .is_empty(),
        "chain 7004 keys on the terminal's tag only"
    );

    let starts = execute_and_drain_starts(actions).await;
    assert_eq!(
        starts.len(),
        1,
        "one CellToBaseMsg::StartMinigame; got {starts:?}"
    );
    let (_, game, difficulty, chains) = &starts[0];
    assert_eq!(
        (game.as_str(), *difficulty, chains.as_slice()),
        ("Livewire", 1, &[7005i64][..]),
    );

    // The victory chain has no trigger row; `fire_chain_by_id` runs its
    // actions directly, so read them off the loaded chain.
    let victory = load_single_chain_for_test(&pool, 7005)
        .await
        .expect("DB query for chain 7005 must succeed")
        .expect("chain 7005 must exist in seeded content_chains");
    let resolved = ResolvedActions {
        action_delays: Vec::new(),
        params: std::collections::HashMap::new(),
        actions: victory.actions.iter().cloned().map(|a| (7005, a)).collect(),
    };
    let mut mgr = staged_space(&pool).await;
    let calls = execute(resolved, &mut mgr).await;
    assert_eq!(calls.len(), 1, "one onPlayerCommunication; got {calls:?}");
    assert_eq!(
        bark_text(&calls[0]),
        (
            "Terminal".to_string(),
            "Livewire round trip complete: the minigame reported your win to the server."
                .to_string()
        ),
    );
}
