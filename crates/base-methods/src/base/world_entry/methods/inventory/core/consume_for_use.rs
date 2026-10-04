//! `handle_consume_item_for_use` — the base half of the native consumable
//! round trip (`CellToBaseMsg::ConsumeItemForUse`).
//!
//! The cell has decided a use of a native consumable (an item whose
//! `items_event_sets` event-5 ability it applies itself: a slappack heal, a
//! stimpack buff) is allowed. This takes exactly one unit off the clicked
//! instance, through the same locked transaction as `removeItem`
//! ([`super::remove_instance`]), held to the design id the cell resolved,
//! and only once that commits tells the cell to apply the ability
//! (`BaseToCellMsg::ItemUseConsumed`).
//!
//! That order is what makes a use consume once and apply once. The row lock
//! serialises two consumes of one stack; the second finds the row gone (or
//! one unit lower) and each committed unit gets exactly one answer. A
//! double-click on the last unit therefore heals once, and an `ItemUsed`
//! the outbox redelivers after a crash finds nothing to consume.
//!
//! The answer goes straight to the cell channel, not through the outbox:
//! at most once. If the channel is gone the unit is spent and the effect is
//! lost (logged as an ERROR); a replayed outbox row could apply the effect
//! twice for one unit, which is the outcome this path exists to prevent.

use cimmeria_entity::known_names;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::transport::Transport;
use sqlx::PgPool;
use tokio::sync::mpsc;

use super::access::AccessOp;
use super::remove_instance::{remove_instance, RemoveInstance};
use crate::base::{session_identity, ConnectedClientState};
use crate::cell::messages::{BaseToCellMsg, ConsumeItemForUse, ItemUseConsumed};

/// Consume one unit of `req.instance_id` (of design `req.type_id`) and, on
/// commit, ask the cell to apply the item's ability.
#[tracing::instrument(
    name = "inventory.consume_for_use",
    level = "info",
    skip_all,
    fields(
        entity_id = req.entity_id,
        player_id = req.player_id,
        instance_id = req.instance_id,
        type_id = req.type_id
    )
)]
pub async fn handle_consume_item_for_use(
    req: ConsumeItemForUse,
    db_pool: &Option<Arc<PgPool>>,
    cell_tx: &Option<mpsc::Sender<BaseToCellMsg>>,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    let ConsumeItemForUse {
        entity_id,
        player_id,
        instance_id,
        type_id,
        vault,
    } = req;
    let account_id =
        session_identity::identity_for_entity(connected, entity_to_addr, entity_id).account_id;
    let committed = remove_instance(
        RemoveInstance {
            entity_id,
            player_id,
            item_id: instance_id,
            quantity: 1,
            notify_gm: false,
            vault,
            expected_type_id: Some(type_id),
            op: AccessOp::Use,
        },
        db_pool,
        cell_tx,
        transport,
        connected,
        entity_to_addr,
    )
    .await;
    if !committed {
        // `remove_instance` logged the cause (row gone, another type, an
        // inaccessible container, a database error). This row ties it to
        // the use: nothing was consumed, so nothing will be applied.
        tracing::info!(
            event = "consumable_consume_refused",
            decision_outcome = "not_consumed",
            reason = "not_removed",
            entity_id,
            entity_name = known_names::player_name(player_id),
            account_id,
            account_name = known_names::account_name(account_id),
            player_id,
            player_name = known_names::player_name(player_id),
            instance_id, // nt:id-only instance row of item_type_id
            item_type_id = type_id,
            item_name = cimmeria_names::book().item(type_id),
            "item use: no unit was consumed, so its effect is not applied"
        );
        return;
    }
    let answer = BaseToCellMsg::ItemUseConsumed(ItemUseConsumed {
        entity_id,
        player_id,
        instance_id,
        type_id,
    });
    let sent = match cell_tx {
        Some(tx) => tx.send(answer).await.is_ok(),
        None => false,
    };
    if sent {
        tracing::debug!(
            event = "consumable_consumed",
            decision_outcome = "consumed",
            entity_id,
            entity_name = known_names::player_name(player_id),
            account_id,
            account_name = known_names::account_name(account_id),
            player_id,
            player_name = known_names::player_name(player_id),
            instance_id, // nt:id-only instance row of item_type_id
            item_type_id = type_id,
            item_name = cimmeria_names::book().item(type_id),
            "item use: one unit consumed; the cell applies the effect"
        );
    } else {
        tracing::error!(
            event = "consumable_apply_send_failed",
            reason = "cell_channel_closed",
            entity_id,
            entity_name = known_names::player_name(player_id),
            account_id,
            account_name = known_names::account_name(account_id),
            player_id,
            player_name = known_names::player_name(player_id),
            instance_id, // nt:id-only instance row of item_type_id
            item_type_id = type_id,
            item_name = cimmeria_names::book().item(type_id),
            "item use: one unit consumed but ItemUseConsumed could not reach the \
             cell; the player lost the unit without its effect"
        );
    }
}
