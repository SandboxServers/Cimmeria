use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::transport::Transport;

use crate::mercury::{
    build_resource_fragment, build_version_info, FRAG_FIRST, FRAG_FIRST_AND_LAST, FRAG_LAST,
    FRAG_MIDDLE,
};

use super::cooked_sync::{self, EnqueueOutcome, SyncJob, VersionReply};
use super::helpers::{drain_acks_and_seq, get_active_entity_id, get_enc_version};
use super::resources::ResourceCache;
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
    let account_id = connected
        .lock()
        .ok()
        .and_then(|c| c.get(&addr).map(|s| s.account_id))
        .unwrap_or(0);

    tracing::info!(
        %addr,
        account_id,
        event = "cooked_data.version_reply",
        outcome = reply.outcome(),
        reason = reply.reason(),
        category_id,
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
                    account_id,
                    category_id,
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

/// Handle `elementDataRequest` (0xC1).
///
/// Client payload: [categoryId: u32][key: u32]
/// Response: fragment the XML data for the requested element.
pub async fn handle_element_data_request(
    transport: &Arc<dyn Transport>,
    addr: SocketAddr,
    key: [u8; 32],
    payload: &[u8],
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    resource_cache: &Option<Arc<ResourceCache>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if payload.len() < 8 {
        tracing::warn!(%addr, "elementDataRequest: payload too short");
        return Ok(());
    }

    let category_id = u32::from_le_bytes([payload[0], payload[1], payload[2], payload[3]]);
    let element_id = u32::from_le_bytes([payload[4], payload[5], payload[6], payload[7]]);

    tracing::debug!(%addr, category_id, element_id, "elementDataRequest");

    let cache = match resource_cache {
        Some(c) => c,
        None => {
            tracing::warn!(%addr, "No resource cache loaded -- cannot serve element data");
            return Ok(());
        }
    };

    let xml_data = match cache.get(category_id, element_id) {
        Some(data) => data,
        None => {
            tracing::warn!(%addr, category_id, element_id, "Element not found in resource cache");
            return Ok(());
        }
    };

    // Override status drives log level below: every patched-XML push is
    // load-bearing for cache consistency, so it deserves INFO. Reads only.
    let is_override = cache.overridden_elements(category_id).contains(&element_id);

    // Allocate a data_id for this transfer
    let data_id = {
        let mut clients = connected.lock().map_err(|_| "connected lock poisoned")?;
        if let Some(c) = clients.get_mut(&addr) {
            let id = c.next_data_id;
            c.next_data_id = c.next_data_id.wrapping_add(1);
            id
        } else {
            return Ok(());
        }
    };

    let chunks: Vec<&[u8]> = xml_data.chunks(MAX_CHUNK).collect();
    let total_chunks = chunks.len();

    // INFO for an overridden element: every patched-XML push is
    // load-bearing for cache consistency, and the per-element byte count
    // is what surfaces a future PAK element crossing a chunk boundary
    // and adding fragments to the cold-cache login burst.
    if is_override {
        tracing::info!(
            %addr, category_id, element_id,
            bytes = xml_data.len(),
            total_chunks,
            data_id,
            "Fragmenting overridden resource data"
        );
    } else {
        tracing::debug!(
            %addr,
            element_id,
            total_size = xml_data.len(),
            total_chunks,
            data_id,
            "Fragmenting resource data"
        );
    }

    let enc_version = get_enc_version(connected, addr);
    for (i, chunk) in chunks.iter().enumerate() {
        let frag_flags = match (i == 0, i == total_chunks - 1) {
            (true, true) => FRAG_FIRST_AND_LAST,
            (true, false) => FRAG_FIRST,
            (false, true) => FRAG_LAST,
            (false, false) => FRAG_MIDDLE,
        };

        // First fragment includes msgType, categoryId, elementId
        let (mt, cat, elem) = if i == 0 {
            (Some(0u8), Some(category_id), Some(element_id))
        } else {
            (None, None, None)
        };

        let (acks, seq) = drain_acks_and_seq(connected, addr)?;
        let pkt = build_resource_fragment(
            &key,
            seq,
            &acks,
            data_id,
            i as u8,
            frag_flags,
            mt,
            cat,
            elem,
            chunk,
            enc_version,
        );
        transport.send_to(&pkt, addr).await?;
        super::helpers::shadow_register_reliable_send(
            connected,
            addr,
            seq,
            cimmeria_mercury::packet::Bytes::copy_from_slice(&pkt),
        );
    }

    tracing::debug!(%addr, element_id, total_chunks, "Resource fragments sent");

    Ok(())
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
