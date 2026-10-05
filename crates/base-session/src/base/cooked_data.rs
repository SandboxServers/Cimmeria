use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::transport::Transport;

use crate::mercury::build_version_info;

use super::cooked_sync::{self, EnqueueOutcome, SyncJob, VersionReply};
use super::helpers::{drain_acks_and_seq, get_active_entity_id, get_enc_version};
use super::resources::{category_name, ResourceCache};
use super::ConnectedClientState;

/// Maximum XML bytes per `BASEMSG_RESOURCE_FRAGMENT` packet.
///
/// Mercury `MAX_BODY_LENGTH` is **1411 bytes**. The first fragment of a
/// resource transfer carries 16 bytes of overhead inside the body — that
/// is, `BASEMSG`(1) plus `WORD_LEN`(2) plus `data_id`(2) plus
/// `chunk_id`(1) plus `frag_flags`(1) plus `msg_type`(1) plus
/// `category_id`(4) plus `element_id`(4). Non-first fragments carry only
/// 7 bytes (`BASEMSG` + `WORD_LEN` + `data_id` + `chunk_id` + `frag_flags`).
///
/// Picking 1390 leaves a 5-byte safety margin against the tighter
/// first-fragment cap (1395) and packs the XML body at 99.6% utilization;
/// the historical value of 1000 wasted ~28% of every packet.
///
/// The fragment-size guard test in `mercury::protocol::tests` pins this
/// against `MercuryEncryption::decrypt(...)` for both first and non-first
/// fragment shapes; bump in tandem if the wire-format overhead changes.
pub(crate) const MAX_CHUNK: usize = 1390;

/// Handle `versionInfoRequest` (0xC0, character select only).
///
/// Client payload: [categoryId: u32][version: u32]
///
/// Shaped by [`VersionReply::decide`]. A matching version, or a category
/// the server does not serve, gets one `onVersionInfo` with no invalidation.
/// A mismatch starts a full resync of the category on the session's resync
/// task ([`cooked_sync`]): the replies and every entry go out from there,
/// paced through the reliable window, so this returns at once and the
/// receive loop never waits on a push.
pub async fn handle_version_info_request(
    transport: &Arc<dyn Transport>,
    addr: SocketAddr,
    key: [u8; 32],
    payload: &[u8],
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    resource_cache: &Option<Arc<ResourceCache>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if payload.len() < 8 {
        tracing::warn!(%addr, "versionInfoRequest: payload too short");
        return Ok(());
    }

    let category_id = u32::from_le_bytes([payload[0], payload[1], payload[2], payload[3]]);
    let client_version = u32::from_le_bytes([payload[4], payload[5], payload[6], payload[7]]);

    let reply = VersionReply::decide(resource_cache.as_deref(), category_id, client_version);
    let who = super::session_identity::identity_for_addr(connected, addr);

    tracing::info!(
        %addr,
        account_id = who.account_id,
        account_name = who.account_name,
        event = "cooked_data.version_reply",
        outcome = reply.outcome(),
        reason = reply.reason(),
        category_id,
        category_name = category_name(category_id),
        client_version,
        server_version = ?reply.server_version(),
        invalidate_all = matches!(reply, VersionReply::FullResync { .. }),
        "Responding to versionInfoRequest"
    );

    let version = match reply {
        VersionReply::NoServerData { client_version } => client_version,
        VersionReply::UpToDate { version } => version,
        VersionReply::FullResync {
            client_version,
            server_version,
            ..
        } => {
            // `decide` only resyncs a category the cache serves.
            let Some(cache) = resource_cache else {
                return Ok(());
            };
            let job = SyncJob {
                category_id,
                client_version,
                server_version,
            };
            let ctx = cooked_sync::context(transport, addr, key, connected, cache);
            let outcome = cooked_sync::start_resync(ctx, job);
            if outcome == EnqueueOutcome::AlreadyPending {
                tracing::debug!(
                    %addr,
                    account_id = who.account_id,
                    account_name = who.account_name,
                    category_id,
                    category_name = category_name(category_id),
                    "versionInfoRequest for a category already being resynced: ignored"
                );
            }
            return Ok(());
        }
    };

    let active_eid = get_active_entity_id(connected, addr)?;
    let (acks, seq) = drain_acks_and_seq(connected, addr)?;
    let enc_version = get_enc_version(connected, addr);
    let pkt = build_version_info(
        &key,
        seq,
        &acks,
        category_id,
        version,
        0,
        false,
        &[],
        active_eid,
        enc_version,
    );
    transport.send_to(&pkt, addr).await?;
    // versionInfo response is one-shot state â€” register for retransmit.
    super::helpers::shadow_register_reliable_send(
        connected,
        addr,
        seq,
        cimmeria_mercury::packet::Bytes::copy_from_slice(&pkt),
    );
    Ok(())
}

/// Handle `elementDataRequest` (0xC1 at character select; in-world the same
/// request arrives as SGWPlayer `0xD5` and takes the same path).
///
/// Client payload: [categoryId: u32][key: u32] (`ClientCache.def`: two
/// INT32s). The entry goes out as a `resourceFragment` transfer on the
/// session's resync task, ahead of any background stream
/// ([`cooked_sync::serve_miss`]).
pub async fn handle_element_data_request(
    transport: &Arc<dyn Transport>,
    addr: SocketAddr,
    key: [u8; 32],
    payload: &[u8],
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    resource_cache: &Option<Arc<ResourceCache>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if payload.len() < 8 {
        tracing::warn!(
            %addr,
            payload_len = payload.len(),
            reason = "payload_too_short",
            "elementDataRequest: payload too short"
        );
        return Ok(());
    }
    let category_id = u32::from_le_bytes([payload[0], payload[1], payload[2], payload[3]]);
    let element_id = u32::from_le_bytes([payload[4], payload[5], payload[6], payload[7]]);
    let Some(cache) = resource_cache else {
        tracing::warn!(
            %addr,
            category_id,
            category_name = category_name(category_id),
            element_id,
            element_name = element_name(category_id, element_id),
            reason = "no_resource_cache",
            "elementDataRequest: no resource cache loaded"
        );
        return Ok(());
    };
    cooked_sync::serve_miss(
        cooked_sync::context(transport, addr, key, connected, cache),
        category_id,
        element_id,
    );
    Ok(())
}

/// The name of one cooked-data element for logs (Rule 6), where the
/// category's element IDs are content IDs the NameBook names: abilities,
/// missions, items, dialogs, effects, worlds, stargates and containers.
/// `None` for the other categories and for an unnamed ID. Owned, because
/// the NameBook guard can't outlive the call: log branches only.
pub(crate) fn element_name(category_id: u32, element_id: u32) -> Option<String> {
    let book = cimmeria_names::book();
    match category_id {
        2 => book.ability(element_id),
        3 => book.mission(element_id),
        4 => book.item(element_id),
        5 => book.dialog(element_id),
        9 => book.effect(element_id),
        12 => book.world(element_id),
        13 => book.stargate(element_id),
        14 => book.container(element_id),
        _ => None,
    }
    .map(str::to_owned)
}

/// Compile-time lower-bound guard for `MAX_CHUNK`. The wire-level tests in
/// `mercury::protocol::tests` decrypt a resource fragment at the current
/// `MAX_CHUNK` and assert it fits Mercury's body cap — but they'd happily
/// pass at `MAX_CHUNK = 1000` too, since 1000-byte chunks also fit. This
/// const assertion pins the lower bound at compile time: a future revert
/// that drops `MAX_CHUNK` below 1390 fails to build instead of silently
/// restoring the wasted ~28% per-packet headroom and re-widening the
/// cold-cache login burst against the 32-slot reliable TX window.
///
/// Bump in tandem if a deliberate retreat from 1390 is ever needed (and
/// document the reason in `MAX_CHUNK`'s docstring above).
const _: () = assert!(
    MAX_CHUNK >= 1390,
    "MAX_CHUNK dropped below the 1390 floor — see the doc comment above \
     for the cold-cache burst rationale; update the docstring + this \
     guard together if the retreat is intentional."
);
