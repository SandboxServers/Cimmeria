//! UO-26: the fixture every Omega hub service chain shares (63850-63869).
//! Callers: UO-32 (auctioneer), UO-33 (medic), UO-35 (guides), UO-36 (inn
//! and battlemaster).
//!
//! [`click`] is the same path the cell takes on a real click:
//! [`fire_interact_tag`] with the chain engine the caller built, and the
//! `(entity, method, args)` list is every `EntityMethodCall` it sent.
//!
//! The W9 modules use these helpers; a helper unused today must not fail
//! clippy, so dead code is allowed here.
#![allow(dead_code)]

use std::collections::BTreeMap;

use sqlx::PgPool;
use tokio::sync::mpsc;

use super::super::super::engine_loader::load_single_chain_for_test;
use crate::cell::content::fire_interact_tag;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::{spawn_npcs_from_records, SpaceManager};
use crate::cell::spawner::{self, load_spawns_from_db};
use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_wire::cell::chat::{serialize_on_player_communication, CHAN_SAY};

pub(super) const PLAYER: u32 = 7601;
/// Kept at `i32` to match `fire_interact_tag`'s `player_id` parameter.
pub(super) const PLAYER_ID: i32 = 61;
/// Wire indices, spelled out so a change to the constants cannot make an
/// assertion agree with itself.
pub(super) const ON_PLAYER_COMMUNICATION: u16 = 28;
pub(super) const ON_BM_OPEN: u16 = 90;
pub(super) const HUB: &str = "Omega_Site";

/// Seeds world 18 with the given spawns (by spawn_id), loads the dialog-screen
/// text cache, and stands a connected player 2 m east of spawn `near`.
/// Returns the manager and a map spawn_id -> entity id.
pub(super) async fn hub_with(
    pool: &PgPool,
    spawns: &[i32],
    near: i32,
) -> (SpaceManager, BTreeMap<i32, u32>) {
    let records: Vec<_> = load_spawns_from_db(pool)
        .await
        .expect("load spawnlist")
        .into_iter()
        .filter(|r| spawns.contains(&r.spawn_id))
        .collect();
    assert_eq!(
        records.len(),
        spawns.len(),
        "every spawn in {spawns:?} must be seeded in world 18"
    );

    let mut mgr = SpaceManager::new(1);
    mgr.parse_spaces_xml(&format!(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="{HUB}" Instanced="false" MinX="-1000" MaxX="1000" MinY="-1000" MaxY="1000" /></Spaces>"#
    ))
    .unwrap();
    mgr.create_startup_spaces(&format!(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="{HUB}" /></Spaces>"#
    ))
    .unwrap();
    assert_eq!(
        spawn_npcs_from_records(&records, &mut mgr),
        records.len(),
        "every spawn in {spawns:?} must spawn into {HUB}"
    );

    let ids: BTreeMap<i32, u32> = mgr
        .all_npc_entity_ids()
        .into_iter()
        .filter_map(|id| {
            mgr.get_entity(id)
                .and_then(|e| e.spawn_id)
                .map(|spawn| (spawn, id))
        })
        .collect();
    for spawn in spawns {
        assert!(
            ids.contains_key(spawn),
            "spawn {spawn} must map to an entity in {HUB}"
        );
    }
    let near_id = *ids
        .get(&near)
        .unwrap_or_else(|| panic!("near spawn {near} must be in {HUB}"));
    let p = mgr.get_entity(near_id).unwrap().position;
    mgr.create_entity(PLAYER, HUB, [p.x + 2.0, p.y, p.z], [0.0; 3])
        .unwrap();
    let pl = mgr.get_entity_mut(PLAYER).unwrap();
    pl.is_player = true;
    pl.player_id = Some(PLAYER_ID);
    mgr.connect_entity(PLAYER);

    // Without the cache every `npc_bark` logs `screen_not_cached` and sends
    // nothing.
    mgr.dialog_screen_text = spawner::load_dialog_screen_text(pool)
        .await
        .expect("dialog screen text");
    (mgr, ids)
}

/// A chain engine holding exactly these chains (each must be seeded).
pub(super) async fn engine_with(pool: &PgPool, chains: &[i32]) -> ChainEngine {
    let mut engine = ChainEngine::new();
    for &chain_id in chains {
        let chain = load_single_chain_for_test(pool, chain_id)
            .await
            .unwrap_or_else(|e| panic!("DB query for chain {chain_id}: {e}"))
            .unwrap_or_else(|| panic!("chain {chain_id} must be seeded"));
        engine.register_chain(chain);
    }
    engine
}

/// One click on `target` as if it carried `tag`: whether a chain claimed it,
/// and every `(entity, method, args)` the cell sent.
pub(super) async fn click(
    mgr: &mut SpaceManager,
    engine: &ChainEngine,
    tag: &str,
    target: u32,
) -> (bool, Vec<(u32, u16, Vec<u8>)>) {
    let (tx, mut rx) = mpsc::channel(32);
    let handled = fire_interact_tag(PLAYER, PLAYER_ID, tag, target, engine, &tx, mgr).await;
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

/// The exact bytes `npc_bark` sends on the say channel: speaker, the
/// `SPEAKER_None` flags byte (0), `CHAN_SAY`, then the text.
pub(super) fn bark_bytes(speaker: &str, text: &str) -> Vec<u8> {
    serialize_on_player_communication(speaker, 0, CHAN_SAY, text)
}

/// `dialog_screens.text` of one screen, from the live DB.
pub(super) async fn screen_text(pool: &PgPool, screen_id: i32) -> String {
    sqlx::query_scalar("SELECT text FROM resources.dialog_screens WHERE screen_id = $1")
        .bind(screen_id)
        .fetch_one(pool)
        .await
        .unwrap_or_else(|e| panic!("dialog screen {screen_id} must be seeded: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::require_db_or_skip;

    /// The banker (spawn 63820) is spawned into the hub, and no chain keys
    /// on `UnionOmega_Banker`, so a click with no chains loaded is not handled.
    #[tokio::test]
    async fn live_db_services_hub_spawns_the_banker() {
        let pool = require_db_or_skip!();
        let (mut mgr, ids) = hub_with(&pool, &[63820], 63820).await;
        let banker = *ids
            .get(&63820)
            .expect("spawn 63820 (UnionOmega_Banker) must map to an entity");
        let engine = engine_with(&pool, &[]).await;
        let (handled, _sent) = click(&mut mgr, &engine, "UnionOmega_Banker", banker).await;
        assert!(
            !handled,
            "a click on spawn 63820 (UnionOmega_Banker) with no chains must not be handled"
        );
    }
}
