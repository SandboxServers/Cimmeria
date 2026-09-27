//! The base's entry point for `CellToBaseMsg::Crafting`.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::transport::Transport;
use sqlx::PgPool;
use tokio::sync::mpsc;

use super::feedback::{reject, CraftReject};
use super::spend::handle_spend;
use crate::base::ConnectedClientState;
use crate::cell::messages::{BaseToCellMsg, CraftRequest, CraftVerb};

/// Everything a crafting verb may need from the base dispatcher, so later
/// verbs add handlers without touching the dispatch arm.
pub struct CraftCtx<'a> {
    pub db_pool: &'a Option<Arc<PgPool>>,
    pub cell_tx: &'a Option<mpsc::Sender<BaseToCellMsg>>,
    pub transport: &'a Arc<dyn Transport>,
    pub connected: &'a Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    pub entity_to_addr: &'a Arc<Mutex<HashMap<u32, SocketAddr>>>,
}

/// Log the request at target `crafting` (`event = "request"`) and answer it.
///
/// `Spend` is decided by [`super::spend`]. Every other verb is answered with
/// a "not available yet" line until its packet lands: a press is never
/// silent (D-CR14).
pub async fn handle_craft_request(request: CraftRequest, ctx: &CraftCtx<'_>) {
    let CraftRequest {
        entity_id,
        player_id,
        verb,
        allowed,
    } = request;
    tracing::info!(
        target: "crafting",
        event = "request",
        entity_id,
        player_id,
        method = verb.method_name(),
        allowed,
        args = ?verb,
        "crafting request"
    );
    if let CraftVerb::Spend { discipline_id } = verb {
        handle_spend(entity_id, player_id, discipline_id, ctx).await;
        return;
    }
    // TODO(CR-07..CR-10): each verb's packet replaces this with its handler
    // (`craft.rs`, `research.rs`, `reverse_engineer.rs`, `alloy.rs`,
    // `respec.rs`).
    reject(
        entity_id,
        player_id,
        &CraftReject::not_available(&verb),
        ctx.transport,
        ctx.connected,
        ctx.entity_to_addr,
    )
    .await;
}
