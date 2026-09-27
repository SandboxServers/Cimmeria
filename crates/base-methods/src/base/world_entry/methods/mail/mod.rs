//! Gate mail on the base: the `MailOp` router.
//!
//! The cell resolves the caller's `player_id` from its own entity and
//! forwards `CellToBaseMsg::MailRequest`; everything that touches
//! `sgw_gate_mail` runs here. One file per family of operations:
//!
//! - [`headers`]: the header list, and the one-header refresh;
//! - [`read`]: body, archive and delete (refused while the mail holds an
//!   attachment);
//! - [`send`]: `sendMailMessage`, text or with cash, an item or COD attached;
//! - [`take`], [`cod`], [`return_`]: take cash, take the item, pay a COD,
//!   return to sender (SS-M3), sharing the lock order in [`claim`];
//! - [`system`]: the one writer for server-originated mail (Black Market
//!   payouts, content actions, GM `.mail`), SS-U1;
//! - [`gm`]: the GM `.mail` and `.mailbox` tools (SS-U1);
//! - [`content`]: the content engine's `send_system_mail` action, behind a
//!   per-player cooldown (SS-U3).

use std::collections::HashMap;
use std::net::SocketAddr;
use std::ops::Deref;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use cimmeria_mercury::transport::Transport;
use sqlx::PgPool;

use super::super::super::helpers::send_to_witness_reliable;
use super::super::super::ConnectedClientState;
use crate::cell::messages::MailOp;
use crate::mercury::build_player_entity_method_packet;

mod claim;
mod cod;
mod content;
mod gm;
mod headers;
mod read;
mod return_;
mod send;
pub mod system;
mod take;

pub use content::handle_content_system_mail;
pub use gm::handle_mail_gm;
pub use system::{
    send_system_mail, send_system_mail_tx, SystemEscrow, SystemItem, SystemMail, SystemMailError,
    SystemMailSent, SERVER_HELD_CONTAINERS, SYSTEM_SOURCE_CHARACTER_ID,
};

#[cfg(test)]
mod tests;

/// Who asked, and how to answer them. Everything here is server state:
/// the cell resolved `player_id` from its own entity.
pub(super) struct Caller<'a> {
    pub(super) entity_id: u32,
    pub(super) player_id: i32,
    pub(super) transport: &'a Arc<dyn Transport>,
    pub(super) connected: &'a Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    pub(super) entity_to_addr: &'a Arc<Mutex<HashMap<u32, SocketAddr>>>,
}

impl Caller<'_> {
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

    /// The caller's `account_id` from the session (rule 5 of the
    /// instrumentation discipline), `None` once the client is gone, so the
    /// field is omitted rather than logged as a fake 0.
    pub(super) fn account_id(&self) -> Option<u32> {
        let addr = self.addr()?;
        let clients = match self.connected.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        clients.get(&addr).map(|c| c.account_id)
    }

    /// The caller's client address, if the entity still has one.
    pub(super) fn addr(&self) -> Option<SocketAddr> {
        let guard = match self.entity_to_addr.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        guard.get(&self.entity_id).copied()
    }
}

/// A [`Caller`] plus the database, for the operations that need one.
pub(super) struct MailCtx<'a> {
    pub(super) caller: Caller<'a>,
    pub(super) pool: &'a PgPool,
}

impl<'a> Deref for MailCtx<'a> {
    type Target = Caller<'a>;

    fn deref(&self) -> &Caller<'a> {
        &self.caller
    }
}

/// Handle a mail request from CellService by querying the DB and sending results to the client.
#[tracing::instrument(
    name = "mail.request",
    level = "info",
    skip_all,
    fields(entity_id, player_id, op = %op_name(&op)),
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
    let caller = Caller {
        entity_id,
        player_id,
        transport,
        connected,
        entity_to_addr,
    };
    route(caller, op, db_pool.as_deref(), Instant::now()).await;
}

/// The router on an explicit clock, so the mail-send bucket can be stepped
/// exactly in tests.
pub(super) async fn route(caller: Caller<'_>, op: MailOp, pool: Option<&PgPool>, now: Instant) {
    // A send takes its rate-limit token before anything else, pool or not
    // (D-SS14: "before any SQL runs").
    let op = match op {
        MailOp::Send(send) => return send::send_mail(&caller, Ok(send), pool, now).await,
        MailOp::SendRejected(reject) => {
            return send::send_mail(&caller, Err(reject), pool, now).await;
        }
        read_op => read_op,
    };

    let Some(pool) = pool else {
        tracing::debug!(
            entity_id = caller.entity_id,
            player_id = caller.player_id,
            "Mail request: no DB pool available"
        );
        return;
    };
    let ctx = MailCtx { caller, pool };
    match op {
        MailOp::RequestHeaders { b_archive } => headers::request_headers(&ctx, b_archive).await,
        MailOp::RequestBody { mail_id } => read::request_body(&ctx, mail_id).await,
        MailOp::Delete { mail_id } => read::delete(&ctx, mail_id).await,
        MailOp::Archive { mail_id } => read::archive(&ctx, mail_id).await,
        MailOp::TakeCash { mail_id } => take::take_cash(&ctx, mail_id).await,
        MailOp::TakeItem {
            mail_id,
            container_id,
            slot_id,
        } => take::take_item(&ctx, mail_id, container_id, slot_id).await,
        MailOp::PayCod { mail_id } => cod::pay_cod(&ctx, mail_id).await,
        MailOp::Return { mail_id } => return_::return_mail(&ctx, mail_id).await,
        // Consumed above.
        MailOp::Send(_) | MailOp::SendRejected(_) => {}
    }
}

/// The span's `op` field. A send's `Debug` would carry the whole subject
/// and body into every span, so each op logs its name and its ids only.
fn op_name(op: &MailOp) -> String {
    match op {
        MailOp::RequestHeaders { b_archive } => format!("request_headers b_archive={b_archive}"),
        MailOp::RequestBody { mail_id } => format!("request_body mail_id={mail_id}"),
        MailOp::Delete { mail_id } => format!("delete mail_id={mail_id}"),
        MailOp::Archive { mail_id } => format!("archive mail_id={mail_id}"),
        MailOp::TakeCash { mail_id } => format!("take_cash mail_id={mail_id}"),
        MailOp::TakeItem { mail_id, .. } => format!("take_item mail_id={mail_id}"),
        MailOp::PayCod { mail_id } => format!("pay_cod mail_id={mail_id}"),
        MailOp::Return { mail_id } => format!("return mail_id={mail_id}"),
        MailOp::Send(send) => format!("send recipients={}", send.recipients.len()),
        MailOp::SendRejected(reject) => format!("send_rejected reason={}", reject.reason()),
    }
}
