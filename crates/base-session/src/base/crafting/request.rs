//! The base's entry point for `CellToBaseMsg::Crafting`.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::transport::Transport;
use sqlx::PgPool;
use tokio::sync::mpsc;

use super::alloy::{handle_alloy, AlloyRequest};
use super::feedback::{reject, CraftReject};
use super::research::handle_research;
use super::reverse_engineer::handle_reverse_engineer;
use super::spend::handle_spend;
use super::sync::CraftClient;
use super::telemetry::account_id_of;
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

impl CraftCtx<'_> {
    /// The player-client half, for the pushes and the refusal line.
    pub fn client(&self) -> CraftClient<'_> {
        CraftClient {
            transport: self.transport,
            connected: self.connected,
            entity_to_addr: self.entity_to_addr,
        }
    }
}

/// Log the request at target `crafting` (`event = "request"`), apply the
/// station gate ([`super::gate`]) and answer it, inside one
/// `crafting.request` span per request.
///
/// `Spend` is decided by [`super::spend`], `Research` by
/// [`super::research`], `ReverseEngineer` by [`super::reverse_engineer`]
/// and `Alloy` by [`super::alloy`]. Every other verb that passes the gate
/// is answered with a "not available yet" line until it is implemented,
/// so a press is never silent.
#[tracing::instrument(
    name = "crafting.request",
    level = "info",
    skip_all,
    fields(verb = request.verb.method_name())
)]
pub async fn handle_craft_request(request: CraftRequest, ctx: &CraftCtx<'_>) {
    let gate = super::gate::check(&request, ctx).await;
    let CraftRequest {
        entity_id,
        player_id,
        verb,
        allowed,
    } = request;
    let method = verb.method_name();
    tracing::info!(
        target: "crafting",
        event = "request",
        verb = method,
        account_id = account_id_of(entity_id, ctx.connected, ctx.entity_to_addr),
        player_id,
        entity_id,
        method,
        allowed,
        args = ?verb,
        "crafting request"
    );
    if let Err(why) = gate {
        reject(method, entity_id, player_id, &why, ctx.client()).await;
        return;
    }
    match verb {
        CraftVerb::Spend { discipline_id } => {
            handle_spend(entity_id, player_id, discipline_id, ctx).await;
            return;
        }
        CraftVerb::Research { item_id, kickers } => {
            handle_research(entity_id, player_id, item_id, kickers, ctx).await;
            return;
        }
        CraftVerb::ReverseEngineer { item_id } => {
            handle_reverse_engineer(entity_id, player_id, item_id, ctx).await;
            return;
        }
        CraftVerb::Alloy {
            blueprint_id,
            current_tier_item_id,
            ref lower_tier_items,
        } => {
            let request = AlloyRequest {
                blueprint_id,
                current_tier_item_id,
                lower_tier_items,
            };
            handle_alloy(entity_id, player_id, request, ctx).await;
            return;
        }
        _ => {}
    }
    reject(
        method,
        entity_id,
        player_id,
        &CraftReject::not_available(&verb),
        ctx.client(),
    )
    .await;
}
