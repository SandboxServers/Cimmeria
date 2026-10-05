//! The base half of the special-ammo reserve round trip (ammo campaign
//! AM-02, issue #1026; D-AM05): `CellToBaseMsg::AmmoReserve`.
//!
//! * A **reload draw** takes `clip_size - clip_before` rounds of the loaded
//!   special type from the bags ([`super::draw`]) and writes
//!   `clip_before + drawn` into the weapon row, in one transaction. An empty
//!   reserve commits nothing and answers `StackEmpty`.
//! * A **switch return** puts the clip's unfired rounds back in the bags
//!   ([`super::return_rounds`]) and writes the weapon row in the same
//!   transaction: 0 rounds of the new type when everything fit; otherwise the
//!   `remainder` stays loaded **as the old type** and the switch does not
//!   happen. No round is deleted and none changes type.
//!
//! The transactions (and their lock order) are in [`super::commits`]; this
//! file is the shell around them.
//!
//! After the commit: the client learns about every touched stack
//! (`onRemoveItem` for an emptied one, `onUpdateItem` for the rest), the
//! catalog event is logged on target `ammo`, and the answer goes straight to
//! the cell channel. It is not routed through the outbox: the database is
//! already consistent, so a lost answer costs the clip display until the
//! next reload, never a round.

use cimmeria_entity::known_names;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_entity::ammo_telemetry::events;
use cimmeria_mercury::transport::Transport;
use sqlx::PgPool;
use tokio::sync::mpsc;

use super::super::core::{send_inventory_item_update_via, send_on_remove_item};
use super::commits::{commit_reload_draw, commit_switch_return};
use super::StackChange;
use crate::base::{session_identity, ConnectedClientState};
use crate::cell::messages::{AmmoReserveAnswer, AmmoReserveRequest, BaseToCellMsg, ReserveRefusal};

/// The session pieces the handler needs to reach the client and the cell.
pub struct ReserveIo<'a> {
    pub db_pool: &'a Option<Arc<PgPool>>,
    pub cell_tx: &'a Option<mpsc::Sender<BaseToCellMsg>>,
    pub transport: &'a Arc<dyn Transport>,
    pub connected: &'a Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    pub entity_to_addr: &'a Arc<Mutex<HashMap<u32, SocketAddr>>>,
}

/// Tell the client about every stack a draw or return touched.
async fn push_stack_changes(
    io: &ReserveIo<'_>,
    pool: &PgPool,
    entity_id: u32,
    player_id: i32,
    changes: &[StackChange],
) {
    for c in changes {
        if c.after == 0 {
            send_on_remove_item(
                entity_id,
                c.instance_id,
                io.transport,
                io.connected,
                io.entity_to_addr,
            )
            .await;
        } else {
            send_inventory_item_update_via(
                entity_id,
                player_id,
                c.instance_id,
                pool,
                io.transport,
                io.connected,
                io.entity_to_addr,
            )
            .await;
        }
    }
}

async fn answer(io: &ReserveIo<'_>, msg: AmmoReserveAnswer) {
    let kind = msg.kind();
    let (entity_id, player_id) = match &msg {
        AmmoReserveAnswer::ReloadDrawn {
            entity_id,
            player_id,
            ..
        }
        | AmmoReserveAnswer::SwitchReturned {
            entity_id,
            player_id,
            ..
        } => (*entity_id, *player_id),
    };
    let sent = match io.cell_tx {
        Some(tx) => tx.send(BaseToCellMsg::AmmoReserve(msg)).await.is_ok(),
        None => false,
    };
    if !sent {
        let player_label = known_names::player_name(player_id);
        tracing::warn!(
            target: "ammo",
            event = "reserve_answer_send_failed",
            reason = "cell_channel_closed",
            entity_id,
            entity_name = player_label,
            player_id,
            player_name = player_label,
            kind,
            "ammo reserve: the answer could not reach the cell; the database is \
             already settled, the clip display catches up on the next reload"
        );
    }
}

/// `CellToBaseMsg::AmmoReserve`: run the request, update the client's bag
/// view, log the catalog event and answer the cell.
#[tracing::instrument(name = "ammo.reserve_request", level = "info", skip_all, fields(kind = req.kind()))]
pub async fn handle_ammo_reserve_request(req: AmmoReserveRequest, io: ReserveIo<'_>) {
    match req {
        AmmoReserveRequest::ReloadDraw {
            entity_id,
            player_id,
            slot_id,
            instance_id,
            ammo_type,
            clip_before,
        } => {
            handle_reload_draw(
                &io,
                entity_id,
                player_id,
                slot_id,
                instance_id,
                ammo_type,
                clip_before,
            )
            .await
        }
        AmmoReserveRequest::SwitchReturn {
            entity_id,
            player_id,
            slot_id,
            instance_id,
            from_ammo_type,
            to_ammo_type,
            rounds,
        } => {
            handle_switch_return(
                &io,
                entity_id,
                player_id,
                slot_id,
                instance_id,
                (from_ammo_type, to_ammo_type),
                rounds,
            )
            .await
        }
    }
}

#[tracing::instrument(
    name = "ammo.reload_draw",
    level = "info",
    skip_all,
    fields(entity_id, player_id, ammo_type)
)]
async fn handle_reload_draw(
    io: &ReserveIo<'_>,
    entity_id: u32,
    player_id: i32,
    slot_id: i32,
    instance_id: i32,
    ammo_type: i32,
    clip_before: i32,
) {
    let account_id =
        session_identity::identity_for_entity(io.connected, io.entity_to_addr, entity_id)
            .account_id;
    let outcome = match io.db_pool {
        Some(pool) => {
            match commit_reload_draw(
                pool,
                player_id,
                slot_id,
                instance_id,
                ammo_type,
                clip_before,
            )
            .await
            {
                Ok(o) => o,
                Err(e) => {
                    let player_label = known_names::player_name(player_id);
                    tracing::error!(
                        target: "ammo",
                        event = events::RELOAD_REFUSED,
                        reason = ReserveRefusal::DbError.reason(),
                        account_id,
                        account_name = known_names::account_name(account_id),
                        player_id,
                        player_name = player_label,
                        entity_id,
                        entity_name = player_label,
                        ammo_type,
                        slot_id, // nt:id-only slot index, unnamed
                        instance_id, // nt:id-only weapon instance in slot_id
                        error = %e,
                        "reload draw: database error, nothing drawn"
                    );
                    Err(ReserveRefusal::DbError)
                }
            }
        }
        None => Err(ReserveRefusal::DbError),
    };
    let (drawn, stack_after, result) = match outcome {
        Ok(c) => {
            if let Some(pool) = io.db_pool {
                push_stack_changes(io, pool, entity_id, player_id, &c.draw.changes).await;
            }
            if c.requested > 0 {
                let player_label = known_names::player_name(player_id);
                tracing::debug!(
                    target: "ammo",
                    event = events::RELOAD_DRAW,
                    account_id,
                    account_name = known_names::account_name(account_id),
                    player_id,
                    player_name = player_label,
                    entity_id,
                    entity_name = player_label,
                    item_id = c.draw.item_id,
                    item_name = cimmeria_names::owned::item(c.draw.item_id),
                    ammo_type,
                    slot_id, // nt:id-only slot index, unnamed
                    instance_id, // nt:id-only weapon instance in slot_id
                    requested = c.requested,
                    drawn = c.draw.drawn,
                    clip_before = c.clip_before,
                    clip_after = c.clip_after,
                    stack_before = c.draw.stack_before,
                    stack_after = c.draw.stack_after,
                    "reload drew special rounds from the bags"
                );
            }
            (c.draw.drawn, c.draw.stack_after, Ok(()))
        }
        Err(refusal) => {
            if refusal != ReserveRefusal::DbError {
                let player_label = known_names::player_name(player_id);
                tracing::warn!(
                    target: "ammo",
                    event = events::RELOAD_REFUSED,
                    reason = refusal.reason(),
                    account_id,
                    account_name = known_names::account_name(account_id),
                    player_id,
                    player_name = player_label,
                    entity_id,
                    entity_name = player_label,
                    ammo_type,
                    slot_id, // nt:id-only slot index, unnamed
                    instance_id, // nt:id-only weapon instance in slot_id
                    clip_before,
                    "reload refused: nothing drawn, the clip is unchanged"
                );
            }
            (0, 0, Err(refusal))
        }
    };
    answer(
        io,
        AmmoReserveAnswer::ReloadDrawn {
            entity_id,
            player_id,
            slot_id,
            instance_id,
            ammo_type,
            drawn,
            stack_after,
            result,
        },
    )
    .await;
}

#[tracing::instrument(
    name = "ammo.switch_return",
    level = "info",
    skip_all,
    fields(entity_id, player_id)
)]
async fn handle_switch_return(
    io: &ReserveIo<'_>,
    entity_id: u32,
    player_id: i32,
    slot_id: i32,
    instance_id: i32,
    (from_ammo_type, to_ammo_type): (i32, i32),
    rounds: i32,
) {
    let account_id =
        session_identity::identity_for_entity(io.connected, io.entity_to_addr, entity_id)
            .account_id;
    let outcome = match io.db_pool {
        Some(pool) => match commit_switch_return(
            pool,
            player_id,
            slot_id,
            instance_id,
            from_ammo_type,
            to_ammo_type,
            rounds,
        )
        .await
        {
            Ok(o) => o,
            Err(e) => {
                let player_label = known_names::player_name(player_id);
                tracing::error!(
                    target: "ammo",
                    event = events::AMMO_SWITCH_RETURN,
                    reason = ReserveRefusal::DbError.reason(),
                    account_id,
                    account_name = known_names::account_name(account_id),
                    player_id,
                    player_name = player_label,
                    entity_id,
                    entity_name = player_label,
                    ammo_type = from_ammo_type,
                    to_ammo_type, rounds,
                    error = %e,
                    "switch return: database error, nothing moved"
                );
                Err(ReserveRefusal::DbError)
            }
        },
        None => Err(ReserveRefusal::DbError),
    };
    let (rounds, returned, remainder, result) = match outcome {
        Ok(c) => {
            if let Some(pool) = io.db_pool {
                push_stack_changes(io, pool, entity_id, player_id, &c.ret.changes).await;
            }
            let player_label = known_names::player_name(player_id);
            tracing::debug!(
                target: "ammo",
                event = events::AMMO_SWITCH_RETURN,
                account_id,
                account_name = known_names::account_name(account_id),
                player_id,
                player_name = player_label,
                entity_id,
                entity_name = player_label,
                item_id = c.ret.item_id,
                item_name = cimmeria_names::owned::item(c.ret.item_id),
                ammo_type = from_ammo_type,
                to_ammo_type,
                slot_id, // nt:id-only slot index, unnamed
                instance_id, // nt:id-only weapon instance in slot_id
                rounds = c.rounds,
                returned = c.ret.returned,
                remainder = c.ret.remainder,
                stack_before = c.ret.stack_before,
                stack_after = c.ret.stack_after,
                switched = c.switched,
                "ammo switch returned unfired special rounds to the bags"
            );
            (c.rounds, c.ret.returned, c.ret.remainder, Ok(()))
        }
        Err(refusal) => {
            if refusal != ReserveRefusal::DbError {
                let player_label = known_names::player_name(player_id);
                tracing::warn!(
                    target: "ammo",
                    event = events::AMMO_SWITCH_RETURN,
                    reason = refusal.reason(),
                    account_id,
                    account_name = known_names::account_name(account_id),
                    player_id,
                    player_name = player_label,
                    entity_id,
                    entity_name = player_label,
                    ammo_type = from_ammo_type,
                    to_ammo_type,
                    slot_id, // nt:id-only slot index, unnamed
                    instance_id, // nt:id-only weapon instance in slot_id
                    rounds,
                    "switch return refused: nothing moved"
                );
            }
            (rounds, 0, 0, Err(refusal))
        }
    };
    answer(
        io,
        AmmoReserveAnswer::SwitchReturned {
            entity_id,
            player_id,
            slot_id,
            instance_id,
            from_ammo_type,
            to_ammo_type,
            rounds,
            returned,
            remainder,
            result,
        },
    )
    .await;
}
