//! `CellToBaseMsg::GrantItem`: loot pickups go to the loot-aware grant,
//! which hands a refused item back to its corpse; every other caller goes
//! to the plain grant.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::transport::Transport;
use sqlx::PgPool;
use tokio::sync::mpsc;

use super::super::super::ConnectedClientState;
use super::super::methods::{handle_grant_item, handle_loot_grant};
use crate::cell::messages::{BaseToCellMsg, LootGrantSource};

/// `CellToBaseMsg::GrantItem`.
pub(super) async fn grant_item(
    entity_id: u32,
    player_id: i32,
    item_id: i32,
    container_id: i32,
    count: i32,
    notify_gm: bool,
    loot: Option<LootGrantSource>,
    db_pool: &Option<Arc<PgPool>>,
    cell_tx: &Option<mpsc::Sender<BaseToCellMsg>>,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    if let Some(source) = loot {
        return handle_loot_grant(
            entity_id,
            player_id,
            item_id,
            container_id,
            count,
            source,
            db_pool,
            cell_tx,
            transport,
            connected,
            entity_to_addr,
        )
        .await;
    }
    handle_grant_item(
        entity_id,
        player_id,
        item_id,
        container_id,
        count,
        notify_gm,
        db_pool,
        cell_tx,
        transport,
        connected,
        entity_to_addr,
    )
    .await;
}
