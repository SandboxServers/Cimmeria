//! Atomic execute path. Placeholder for commit 2 — replaced by the
//! sqlx-tx implementation in the next commit. The signature is locked
//! in here so the cell-side hand-off compiles in the same step.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::transport::Transport;
use sqlx::PgPool;

use crate::base::ConnectedClientState;

/// Stub — see commit 3 for the actual atomic swap implementation.
#[allow(clippy::too_many_arguments)]
pub async fn handle_execute_trade(
    entity_id: u32,
    player_id: i32,
    partner_entity_id: u32,
    partner_player_id: i32,
    p1_item_instance_ids: Vec<i32>,
    p1_cash: i32,
    p2_item_instance_ids: Vec<i32>,
    p2_cash: i32,
    _db_pool: &Option<Arc<PgPool>>,
    _transport: &Arc<dyn Transport>,
    _connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    _entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    tracing::warn!(
        entity_id,
        player_id,
        partner_entity_id,
        partner_player_id,
        p1_items = p1_item_instance_ids.len(),
        p1_cash,
        p2_items = p2_item_instance_ids.len(),
        p2_cash,
        "STUB: handle_execute_trade — atomic commit lands in next commit"
    );
}
