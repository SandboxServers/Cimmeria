//! `useItem` on a crafting item (a Blueprint item or a Racial Paradigm
//! Guide): the crafting subsystem decides and commits the use, and this
//! module brings the client's inventory up to date afterwards.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::transport::Transport;
use sqlx::PgPool;
use tokio::sync::mpsc;

use super::{send_full_inventory_update, send_on_remove_item};
use crate::base::crafting::item_use::handle_crafting_item_use;
use crate::base::crafting::request::CraftCtx;
use crate::base::outbox;
use crate::base::ConnectedClientState;
use crate::cell::messages::BaseToCellMsg;

/// Use crafting item instance `item_id`. On a committed use the consumed
/// instance leaves the client (`onRemoveItem` when its last one went, then
/// the inventory update) and the cell is told it is gone; a refusal sends
/// only the crafting refusal line, since nothing in the inventory changed.
///
/// `OnItemUse` is deliberately not fired: the crafting use is the only
/// consumer of these items, so a content chain cannot remove one a second
/// time.
pub(super) async fn use_crafting_item(
    entity_id: u32,
    player_id: i32,
    item_id: i32,
    pool: &Arc<PgPool>,
    db_pool: &Option<Arc<PgPool>>,
    cell_tx: &Option<mpsc::Sender<BaseToCellMsg>>,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    let ctx = CraftCtx {
        db_pool,
        cell_tx,
        transport,
        connected,
        entity_to_addr,
    };
    let Some(consumed) = handle_crafting_item_use(entity_id, player_id, item_id, &ctx).await else {
        return;
    };
    if consumed.removed_all {
        send_on_remove_item(entity_id, item_id, transport, connected, entity_to_addr).await;
    }
    send_full_inventory_update(
        entity_id,
        player_id,
        pool,
        transport,
        connected,
        entity_to_addr,
    )
    .await;
    if let (Some((outbox_id, payload)), Some(cell_tx)) = (consumed.outbox, cell_tx) {
        outbox::try_dispatch_now(pool.as_ref(), cell_tx, outbox_id, entity_id, payload).await;
    }
}
