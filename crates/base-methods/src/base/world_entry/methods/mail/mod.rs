//! Gate mail on the base: the `MailOp` router.
//!
//! The cell resolves the caller's `player_id` from its own entity and
//! forwards `CellToBaseMsg::MailRequest`; everything that touches
//! `sgw_gate_mail` runs here. One file per family of operations:
//!
//! - [`read`]: headers, body, archive and delete.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::transport::Transport;
use sqlx::PgPool;

use super::super::super::helpers::send_to_witness_reliable;
use super::super::super::ConnectedClientState;
use crate::cell::messages::MailOp;
use crate::mercury::build_player_entity_method_packet;

mod read;

#[cfg(test)]
mod tests;

/// Everything one mail operation needs: who asked, and how to answer them.
pub(super) struct MailCtx<'a> {
    pub(super) entity_id: u32,
    pub(super) player_id: i32,
    pub(super) transport: &'a Arc<dyn Transport>,
    pub(super) connected: &'a Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    pub(super) entity_to_addr: &'a Arc<Mutex<HashMap<u32, SocketAddr>>>,
    pub(super) pool: &'a PgPool,
}

impl MailCtx<'_> {
    /// Send one client method to the caller's own client, reliably.
    pub(super) async fn send_to_caller(&self, method_index: u16, args: &[u8]) {
        let entity_id = self.entity_id;
        send_to_witness_reliable(
            self.transport,
            self.connected,
            self.entity_to_addr,
            entity_id,
            |key, version, seq, acks| {
                build_player_entity_method_packet(
                    key,
                    seq,
                    acks,
                    entity_id,
                    method_index,
                    args,
                    version,
                )
            },
        )
        .await;
    }
}

/// Handle a mail request from CellService by querying the DB and sending results to the client.
#[tracing::instrument(
    name = "mail.request",
    level = "info",
    skip_all,
    fields(entity_id, player_id, op = ?op),
)]
pub async fn handle_mail_request(
    entity_id: u32,
    player_id: i32,
    op: MailOp,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
    db_pool: &Option<Arc<PgPool>>,
) {
    let pool = match db_pool {
        Some(p) => p,
        None => {
            tracing::debug!(entity_id, player_id, "Mail request: no DB pool available");
            return;
        }
    };
    let ctx = MailCtx {
        entity_id,
        player_id,
        transport,
        connected,
        entity_to_addr,
        pool,
    };

    match op {
        MailOp::RequestHeaders { b_archive } => read::request_headers(&ctx, b_archive).await,
        MailOp::RequestBody { mail_id } => read::request_body(&ctx, mail_id).await,
        MailOp::Delete { mail_id } => read::delete(&ctx, mail_id).await,
        MailOp::Archive { mail_id } => read::archive(&ctx, mail_id).await,
    }
}
