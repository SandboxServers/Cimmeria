//! Black Market BM-07 — chain-replay guards for the stasis-room auctioneer:
//! spawn 405 (template 305) and chain 5030 in
//! `db/resources/Content/Seed/castle_cellblock_chains.sql`.
//!
//! The spawns are loaded from the seeded database and spawned the way the
//! cell does at startup, and the click goes through [`fire_interact_tag`]
//! with chain 5030 loaded, so the seeded `INT_Auction` bit, the spawn-time
//! `Auctioneer` derivation, the chain and the executor's authority check
//! are all under test together.
//!
//! * The auctioneer's click sends `onBMOpen` naming him, a chat line, and
//!   records the Black Market session.
//! * The same chain fired at another hub NPC (the dialog NPC, spawn 402)
//!   opens nothing: `open_black_market` refuses an NPC whose template has no
//!   `INT_Auction` bit, whatever chain runs it.

use sqlx::PgPool;
use tokio::sync::mpsc;

use super::super::engine_loader::load_single_chain_for_test;
use crate::cell::content::fire_interact_tag;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::{spawn_npcs_from_records, SpaceManager};
use crate::cell::spawner::load_spawns_from_db;
use crate::test_support::require_db_or_skip;
use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_entity::cell_entity::NpcInteractionType;
use cimmeria_wire::cell::chat::{serialize_on_player_communication, CHAN_FEEDBACK};

const PLAYER: u32 = 7501;
const PLAYER_ID: i32 = 43;
const AUCTIONEER_SPAWN: i32 = 405;
const DIALOG_NPC_SPAWN: i32 = 402;
const TAG: &str = "BlackMarket_Auctioneer";

/// Wire indices, spelled out so a change to the constants cannot make an
/// assertion agree with itself.
const ON_BM_OPEN: u16 = 90;
const ON_PLAYER_COMMUNICATION: u16 = 28;
/// The executor's open line, spelled out for the same reason.
const OPENED_LINE: &str =
    "The auctioneer opens the Black Market. (No window? The Black Market needs the Cimmeria client patch.)";

/// Castle_CellBlock with the seeded auctioneer and dialog NPC spawned, and
/// a connected player 2 units from `near_spawn`. Returns the manager and
/// the two NPC entity ids `(auctioneer, dialog_npc)`.
async fn fixture(pool: &PgPool, near_spawn: i32) -> (SpaceManager, u32, u32) {
    let records: Vec<_> = load_spawns_from_db(pool)
        .await
        .expect("load spawnlist")
        .into_iter()
        .filter(|r| r.spawn_id == AUCTIONEER_SPAWN || r.spawn_id == DIALOG_NPC_SPAWN)
        .collect();
    assert_eq!(records.len(), 2, "spawns 405 and 402 are seeded");

    let mut mgr = SpaceManager::new(1);
    mgr.parse_spaces_xml(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" Instanced="false" MinX="-450" MaxX="450" MinY="-450" MaxY="450" /></Spaces>"#,
    )
    .unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" /></Spaces>"#,
    )
    .unwrap();
    assert_eq!(spawn_npcs_from_records(&records, &mut mgr), 2);

    let by_spawn = |mgr: &SpaceManager, spawn: i32| {
        mgr.all_npc_entity_ids()
            .into_iter()
            .find(|&id| mgr.get_entity(id).and_then(|e| e.spawn_id) == Some(spawn))
            .unwrap_or_else(|| panic!("spawn {spawn} must be in the space"))
    };
    let auctioneer = by_spawn(&mgr, AUCTIONEER_SPAWN);
    let dialog_npc = by_spawn(&mgr, DIALOG_NPC_SPAWN);
    let near = if near_spawn == AUCTIONEER_SPAWN {
        auctioneer
    } else {
        dialog_npc
    };
    let p = mgr.get_entity(near).unwrap().position;
    mgr.create_entity(PLAYER, "Castle_CellBlock", [p.x + 2.0, p.y, p.z], [0.0; 3])
        .unwrap();
    let pl = mgr.get_entity_mut(PLAYER).unwrap();
    pl.is_player = true;
    pl.player_id = Some(PLAYER_ID);
    mgr.connect_entity(PLAYER);
    (mgr, auctioneer, dialog_npc)
}

async fn chain_5030(pool: &PgPool) -> ChainEngine {
    let chain = load_single_chain_for_test(pool, 5030)
        .await
        .expect("DB query for chain 5030")
        .expect("chain 5030 must be seeded");
    let mut engine = ChainEngine::new();
    engine.register_chain(chain);
    engine
}

/// Click `target` as if it carried the auctioneer's tag; returns whether a
/// chain claimed the click and every `(entity, method, args)` sent.
async fn click(
    mgr: &mut SpaceManager,
    engine: &ChainEngine,
    target: u32,
) -> (bool, Vec<(u32, u16, Vec<u8>)>) {
    let (tx, mut rx) = mpsc::channel(32);
    let handled = fire_interact_tag(PLAYER, PLAYER_ID, TAG, target, engine, &tx, mgr).await;
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
    (handled, out)
}

/// The seeded auctioneer spawns as an `Auctioneer`, and clicking him runs
/// chain 5030: `onBMOpen` to the player naming the auctioneer, the open
/// line, and a Black Market session at him.
#[tokio::test]
async fn clicking_the_seeded_auctioneer_opens_the_black_market() {
    let pool = require_db_or_skip!();
    let (mut mgr, auctioneer, _) = fixture(&pool, AUCTIONEER_SPAWN).await;
    assert_eq!(
        mgr.get_entity(auctioneer).unwrap().interaction_type,
        Some(NpcInteractionType::Auctioneer),
        "template 305's INT_Auction bit derives Auctioneer at spawn"
    );
    let engine = chain_5030(&pool).await;

    let (handled, sent) = click(&mut mgr, &engine, auctioneer).await;

    assert!(handled, "chain 5030 claims the click");
    let opens: Vec<_> = sent.iter().filter(|c| c.1 == ON_BM_OPEN).collect();
    assert_eq!(opens.len(), 1, "{sent:?}");
    assert_eq!(opens[0].0, PLAYER);
    assert_eq!(opens[0].2, (auctioneer as i32).to_le_bytes().to_vec());
    let line = serialize_on_player_communication("SYSTEM", 0, CHAN_FEEDBACK, OPENED_LINE);
    assert!(
        sent.iter()
            .any(|c| c.0 == PLAYER && c.1 == ON_PLAYER_COMMUNICATION && c.2 == line),
        "the player is told the Black Market opened: {sent:?}"
    );
    assert_eq!(
        mgr.black_market
            .get(PLAYER_ID)
            .and_then(|s| s.auctioneer_id),
        Some(auctioneer)
    );
}

/// The authority half: chain 5030 fired at the hub's dialog NPC (a chain
/// bound to the wrong NPC) sends no `onBMOpen` and records no session; the
/// player is told nobody here runs the Black Market. Fails if
/// `open_black_market` stops checking for an auctioneer.
#[tokio::test]
async fn the_open_chain_at_another_npc_opens_nothing() {
    let pool = require_db_or_skip!();
    let (mut mgr, _, dialog_npc) = fixture(&pool, DIALOG_NPC_SPAWN).await;
    assert_ne!(
        mgr.get_entity(dialog_npc).unwrap().interaction_type,
        Some(NpcInteractionType::Auctioneer)
    );
    let engine = chain_5030(&pool).await;

    let (_, sent) = click(&mut mgr, &engine, dialog_npc).await;

    assert!(
        !sent.iter().any(|c| c.1 == ON_BM_OPEN),
        "no Black Market at a non-auctioneer: {sent:?}"
    );
    let line = serialize_on_player_communication(
        "SYSTEM",
        0,
        CHAN_FEEDBACK,
        "Nobody here runs the Black Market.",
    );
    assert!(sent
        .iter()
        .any(|c| c.1 == ON_PLAYER_COMMUNICATION && c.2 == line));
    assert!(mgr.black_market.get(PLAYER_ID).is_none());
}
