//! Gate-travel / reanchor / teleport dispatch arms.
//!
//! Each function is the body of one `CellToBaseMsg` arm carved out of the
//! [`super::handle_cell_message`] match, routing world-transition and
//! position-snap traffic into the existing `world_entry` handlers.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::transport::Transport;
use sqlx::PgPool;
use tokio::sync::mpsc;

use crate::cell::messages::{BaseToCellMsg, CellToBaseMsg};

use super::super::super::ConnectedClientState;
use super::super::gate_travel::handle_gate_travel;
use super::super::reanchor_player::handle_reanchor_player;
use super::super::teleport::handle_teleport_player;
use super::DispatchCtx;

/// Route the gate-travel / reanchor / teleport family of `CellToBaseMsg`.
///
/// Called by [`super::handle_cell_message`] once the outer match has narrowed
/// `msg` to this family; any other variant is `unreachable!` by construction.
pub(super) async fn route(msg: CellToBaseMsg, ctx: &DispatchCtx<'_>) {
    match msg {
        CellToBaseMsg::GateTravel {
            entity_id,
            target_world_name,
            position,
            rotation,
            destination_ring_id,
        } => {
            gate_travel(
                entity_id,
                target_world_name,
                position,
                rotation,
                destination_ring_id,
                ctx.transport,
                ctx.connected,
                ctx.entity_to_addr,
                ctx.cell_tx,
                ctx.db_pool,
            )
            .await
        }
        CellToBaseMsg::ReanchorPlayer {
            entity_id,
            space_id,
            position,
            rotation,
        } => {
            reanchor_player(
                entity_id,
                space_id,
                position,
                rotation,
                ctx.transport,
                ctx.connected,
                ctx.entity_to_addr,
            )
            .await
        }
        CellToBaseMsg::TeleportPlayer {
            entity_id,
            space_id,
            position,
            prev_pos,
        } => {
            teleport_player(
                entity_id,
                space_id,
                position,
                prev_pos,
                ctx.transport,
                ctx.connected,
                ctx.entity_to_addr,
                ctx.db_pool,
            )
            .await
        }
        other => unreachable!("gate_teleport_dispatch::route got unexpected variant: {other:?}"),
    }
}

/// `CellToBaseMsg::GateTravel`.
pub(super) async fn gate_travel(
    entity_id: u32,
    target_world_name: String,
    position: [f32; 3],
    rotation: [f32; 3],
    destination_ring_id: Option<i32>,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
    cell_tx: &Option<mpsc::Sender<BaseToCellMsg>>,
    db_pool: &Option<Arc<PgPool>>,
) {
    if let Err(e) = handle_gate_travel(
        entity_id,
        &target_world_name,
        position,
        rotation,
        destination_ring_id,
        transport,
        connected,
        entity_to_addr,
        cell_tx,
        db_pool,
    )
    .await
    {
        tracing::error!(entity_id, world = %target_world_name, "Gate travel failed: {e}");
    }
}

/// `CellToBaseMsg::ReanchorPlayer`.
pub(super) async fn reanchor_player(
    entity_id: u32,
    space_id: u32,
    position: [f32; 3],
    rotation: [f32; 3],
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    if let Err(e) = handle_reanchor_player(
        entity_id,
        space_id,
        position,
        rotation,
        transport,
        connected,
        entity_to_addr,
    )
    .await
    {
        tracing::error!(entity_id, "Reanchor player failed: {e}");
    }
}

/// `CellToBaseMsg::TeleportPlayer`.
pub(super) async fn teleport_player(
    entity_id: u32,
    space_id: u32,
    position: [f32; 3],
    prev_pos: [f32; 3],
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
    db_pool: &Option<Arc<PgPool>>,
) {
    handle_teleport_player(
        entity_id,
        space_id,
        position,
        prev_pos,
        transport,
        connected,
        entity_to_addr,
        db_pool,
    )
    .await;
}
