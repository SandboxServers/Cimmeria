//! `useItem` on an item a base plugin takes over (#962 step 5): a crafting
//! item (a Blueprint item or a Racial Paradigm Guide), or a crafting item
//! that is no longer this player's. The plugin (`cimmeria-base-crafting`)
//! decides and commits the use through an [`ItemUseHookPoint`] hook; this
//! module brings the client's inventory up to date afterwards.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_base_session::base::plugin::{
    entity_plugins, BaseCtx, ItemUseCall, ItemUseHookPoint, ItemUseOutcome,
};
use cimmeria_mercury::transport::Transport;
use sqlx::PgPool;
use tokio::sync::mpsc;

use super::{send_full_inventory_update, send_on_remove_item};
use crate::base::outbox;
use crate::base::ConnectedClientState;
use crate::cell::messages::BaseToCellMsg;

/// Offer `useItem` of instance `item_id` to the base plugins at `point`.
/// On a consumed item the instance leaves the client (`onRemoveItem` when
/// its last one went, then the inventory update) and the cell is told it is
/// gone; a refusal sends nothing more, since the plugin already told the
/// player and nothing in the inventory changed. Returns `false` when no
/// plugin took the use, so the caller runs its own line.
///
/// `OnItemUse` is deliberately not fired: the plugin's use is the only
/// consumer of these items, so a content chain cannot remove one a second
/// time.
pub(super) async fn offer_item_use(
    point: ItemUseHookPoint,
    entity_id: u32,
    player_id: i32,
    item_id: i32,
    pool: &Arc<PgPool>,
    db_pool: &Option<Arc<PgPool>>,
    cell_tx: &Option<mpsc::Sender<BaseToCellMsg>>,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) -> bool {
    let plugins = entity_plugins(connected, entity_to_addr, entity_id);
    let call = ItemUseCall {
        entity_id,
        player_id,
        item_id,
        pool,
        ctx: BaseCtx {
            db_pool,
            cell_tx,
            transport,
            connected,
            entity_to_addr,
        },
    };
    let consumed = match plugins.run_item_use_hook(point, call).await {
        ItemUseOutcome::NotHandled => return false,
        ItemUseOutcome::Refused => return true,
        ItemUseOutcome::Consumed(consumed) => consumed,
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
    true
}
