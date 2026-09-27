//! Base-side GM feedback line: the definitive (post-commit) confirmation to
//! the GM's own client.
//!
//! This is the counterpart to the cell-side `gm::feedback::send_gm_feedback`.
//! The base-round-trip GM commands (give/crafting/spawn) used to feed back
//! *optimistically* from the cell with a "requested" line, which lied whenever
//! the base dropped the DB write. The handlers in this layer call this helper
//! only after the write actually commits, so the GM sees confirmation that
//! reflects reality ("trust, but verify").
//!
//! The wire send is the shared single-recipient feedback line in
//! [`super::feedback`]; this module only resolves the GM's entity to its
//! client address.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::transport::Transport;

use super::feedback::{send_feedback_to_entity, FeedbackCtx};
use super::ConnectedClientState;

/// Send a single definitive GM-feedback line to the entity's own client.
///
/// Speaker is `"SYSTEM"`, flags `0`, channel `CHAN_FEEDBACK`, addressed to
/// `entity_id` and sent reliably.
///
/// If the entity has no connected client (e.g. it disconnected between the
/// write committing and this send), the line is dropped with a WARN — the DB
/// write already committed, so dropping the feedback line is harmless.
pub async fn send_gm_feedback_to_client(
    entity_id: u32,
    text: &str,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    let addr = entity_to_addr.lock().unwrap().get(&entity_id).copied();
    let Some(addr) = addr else {
        tracing::warn!(
            entity_id,
            reason = "entity_to_addr_miss",
            "GM feedback: no client addr for entity -- line dropped"
        );
        return;
    };
    let ctx = FeedbackCtx {
        transport,
        connected,
    };
    send_feedback_to_entity(&ctx, addr, entity_id, text).await;
}
