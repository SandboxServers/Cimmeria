use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use tokio::net::UdpSocket;

use crate::mercury::{
    build_resource_fragment, build_version_info, FRAG_FIRST, FRAG_FIRST_AND_LAST, FRAG_LAST,
    FRAG_MIDDLE,
};

use super::helpers::{drain_acks_and_seq, get_active_entity_id};
use super::resources::{CategoryData, ResourceCache};
use super::ConnectedClientState;

/// Handle `versionInfoRequest` (0xC0).
///
/// Client payload: [categoryId: u32][version: u32]
/// Response: onVersionInfo -- if we have data for this category, tell the client
/// to invalidate and re-fetch; otherwise echo the client's version (cache OK).
pub(crate) async fn handle_version_info_request(
    socket: &Arc<UdpSocket>,
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

    // Three-way response shape, extended from the original C++ behaviour
    // (Account.py:322-352, Def.py:293-319) to add per-key invalidation:
    //
    // 1. Server has no data for this category → echo the client's version,
    //    invalidate_all=false, no keys (client keeps its local cache).
    // 2. Versions match → invalidate_all=false, no keys (client is up to date).
    // 3. Versions differ AND Cimmeria has scoped overrides for this category
    //    → invalidate_all=false, InvalidKeys=<overridden ids>. The client
    //    drops only those entries from its runtime cache and waits for the
    //    server to push the replacements — it does NOT issue
    //    elementDataRequest for InvalidKeys. The proactive push happens in
    //    push_overridden_elements right after the onVersionInfo reply, with
    //    RequiredUpdates set to the InvalidKeys count.
    // 4. Versions differ AND no scoped overrides → invalidate_all=true
    //    (legacy fallback for whole-PAK swaps, e.g. switching QA→Server build).
    let (version, invalidate_all, invalid_keys): (u32, bool, Vec<u32>) = match resource_cache {
        Some(cache) => match cache.category(category_id) {
            Some(cat) => {
                let server_version = cat.metadata;
                if client_version == server_version {
                    (server_version, false, Vec::new())
                } else {
                    let overrides = cache.overridden_elements(category_id);
                    if !overrides.is_empty() {
                        (server_version, false, overrides.to_vec())
                    } else {
                        (server_version, true, Vec::new())
                    }
                }
            }
            None => (client_version, false, Vec::new()),
        },
        None => (client_version, false, Vec::new()),
    };

    tracing::info!(
        %addr, category_id, client_version, version, invalidate_all,
        invalid_key_count = invalid_keys.len(),
        invalid_keys = ?invalid_keys,
        "Responding to versionInfoRequest"
    );

    let active_eid = get_active_entity_id(connected, addr)?;
    let (acks, seq) = drain_acks_and_seq(connected, addr)?;
    // RequiredUpdates tells the client how many resourceFragment packets
    // are about to land — set to the InvalidKeys count when we're shipping
    // overrides so the client doesn't try to lazy-fetch via elementDataRequest
    // (which the runtime cache doesn't actually issue — it just drops the
    // local entry on InvalidKeys and waits for our push). Without this,
    // the client's `Cache.en-US/CookedDataMissions.pak` is left with the
    // entries removed but never replaced — symptom: missions stop being
    // granted on subsequent logins because the catalog row is gone.
    let required_updates = invalid_keys.len() as u32;
    let pkt = build_version_info(
        &key,
        seq,
        &acks,
        category_id,
        version,
        required_updates,
        invalidate_all,
        &invalid_keys,
        active_eid,
    );
    socket.send_to(&pkt, addr).await?;
    // Issue #308: versionInfo response is one-shot state — register for retransmit.
    super::helpers::shadow_register_reliable_send(
        connected,
        addr,
        seq,
        cimmeria_mercury::packet::Bytes::copy_from_slice(&pkt),
    );

    // Push the patched XML proactively for each overridden element. The
    // version-info reply already named these in InvalidKeys; the client
    // drops them locally and expects N follow-up resourceFragment pushes.
    if !invalid_keys.is_empty() {
        if let Some(cache) = resource_cache {
            push_overridden_elements(
                socket,
                addr,
                key,
                category_id,
                &invalid_keys,
                cache.as_ref(),
                connected,
            )
            .await?;
        }
    }

    Ok(())
}

/// Send `BASEMSG_RESOURCE_FRAGMENT` packets for the named overridden elements,
/// in the order they appear in `element_ids`.
///
/// Mirrors the chunking + frag-flag logic of `handle_element_data_request`,
/// but iterates a fixed list so we can ship just the overridden subset
/// without the client having to ask. Used by the InvalidKeys handshake to
/// keep the client's writable runtime cache (`Cache.en-US/`) consistent
/// with the server's in-memory overrides.
async fn push_overridden_elements(
    socket: &Arc<UdpSocket>,
    addr: SocketAddr,
    key: [u8; 32],
    category_id: u32,
    element_ids: &[u32],
    cache: &ResourceCache,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    const MAX_CHUNK: usize = 1000;

    for &element_id in element_ids {
        let xml_data = match cache.get(category_id, element_id) {
            Some(data) => data,
            None => {
                tracing::warn!(
                    %addr, category_id, element_id,
                    "push_overridden_elements: element missing from cache, skipping",
                );
                continue;
            }
        };

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

        tracing::info!(
            %addr, category_id, element_id,
            bytes = xml_data.len(),
            total_chunks,
            data_id,
            "Pushing overridden element fragments"
        );

        for (i, chunk) in chunks.iter().enumerate() {
            let frag_flags = match (i == 0, i == total_chunks - 1) {
                (true, true) => FRAG_FIRST_AND_LAST,
                (true, false) => FRAG_FIRST,
                (false, true) => FRAG_LAST,
                (false, false) => FRAG_MIDDLE,
            };
            let (mt, cat, elem) = if i == 0 {
                (Some(0u8), Some(category_id), Some(element_id))
            } else {
                (None, None, None)
            };
            let (acks, seq) = drain_acks_and_seq(connected, addr)?;
            let pkt = build_resource_fragment(
                &key, seq, &acks, data_id, i as u8, frag_flags, mt, cat, elem, chunk,
            );
            socket.send_to(&pkt, addr).await?;
            super::helpers::shadow_register_reliable_send(
                connected,
                addr,
                seq,
                cimmeria_mercury::packet::Bytes::copy_from_slice(&pkt),
            );
        }
    }

    Ok(())
}

/// Proactively send all resources for a category as BASEMSG_RESOURCE_FRAGMENT packets.
///
/// Not currently used.  The C++ server only pushes resources for certain
/// "categoryMaps" categories (12-20) when invalidateAll=true and the
/// client version is stale.  Category 7 (char_creation) is NOT in that
/// map — the client loads it from its local PAK.  Kept for future use.
#[allow(dead_code)]
pub(crate) async fn send_category_resources(
    socket: &Arc<UdpSocket>,
    addr: SocketAddr,
    key: [u8; 32],
    category_id: u32,
    cat: &CategoryData,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    const MAX_CHUNK: usize = 1000;

    // Sort element IDs for deterministic ordering.
    let mut element_ids: Vec<u32> = cat.elements.keys().copied().collect();
    element_ids.sort();

    for &element_id in &element_ids {
        let xml_data = match cat.elements.get(&element_id) {
            Some(data) => data,
            None => continue,
        };

        // Allocate a data_id for this transfer.
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

        for (i, chunk) in chunks.iter().enumerate() {
            let frag_flags = match (i == 0, i == total_chunks - 1) {
                (true, true) => FRAG_FIRST_AND_LAST,
                (true, false) => FRAG_FIRST,
                (false, true) => FRAG_LAST,
                (false, false) => FRAG_MIDDLE,
            };

            let (mt, cat_id, elem) = if i == 0 {
                (Some(0u8), Some(category_id), Some(element_id))
            } else {
                (None, None, None)
            };

            let (acks, seq) = drain_acks_and_seq(connected, addr)?;
            let pkt = build_resource_fragment(
                &key, seq, &acks, data_id, i as u8, frag_flags, mt, cat_id, elem, chunk,
            );
            socket.send_to(&pkt, addr).await?;
            super::helpers::shadow_register_reliable_send(
                connected,
                addr,
                seq,
                cimmeria_mercury::packet::Bytes::copy_from_slice(&pkt),
            );
        }
    }

    tracing::debug!(
        %addr, category_id, count = element_ids.len(),
        "Proactively sent all resource fragments"
    );

    Ok(())
}

/// Handle `elementDataRequest` (0xC1).
///
/// Client payload: [categoryId: u32][key: u32]
/// Response: fragment the XML data for the requested element.
pub(crate) async fn handle_element_data_request(
    socket: &Arc<UdpSocket>,
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

    // Promote the log to INFO when we're shipping a Cimmeria-overridden
    // element — operationally the load-bearing question is "did the
    // patched mission XML actually go out to this client", and that
    // answer is too important to bury at debug level on a noisy DB.
    let is_override = cache.overridden_elements(category_id).contains(&element_id);
    if is_override {
        tracing::info!(
            %addr, category_id, element_id,
            bytes = xml_data.len(),
            "Shipping overridden element to client"
        );
    }

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

    // Fragment into 1000-byte chunks
    const MAX_CHUNK: usize = 1000;
    let chunks: Vec<&[u8]> = xml_data.chunks(MAX_CHUNK).collect();
    let total_chunks = chunks.len();

    tracing::debug!(
        %addr,
        element_id,
        total_size = xml_data.len(),
        total_chunks,
        data_id,
        "Fragmenting resource data"
    );

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
            &key, seq, &acks, data_id, i as u8, frag_flags, mt, cat, elem, chunk,
        );
        socket.send_to(&pkt, addr).await?;
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
