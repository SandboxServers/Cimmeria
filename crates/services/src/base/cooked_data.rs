use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use tokio::net::UdpSocket;

use crate::mercury::{
    build_resource_fragment, build_version_info,
    FRAG_FIRST, FRAG_FIRST_AND_LAST, FRAG_LAST, FRAG_MIDDLE,
};

use super::ConnectedClientState;
use super::helpers::{drain_acks_and_seq, get_account_entity_id};
use super::resources::{CategoryData, ResourceCache};

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

    // Version negotiation: send back the PAK metadata version.  When the
    // client's version doesn't match (e.g. fresh client sends version 0),
    // set invalidateAll=true so the client reloads from its PAK baseline
    // and fires the "element ready" events that the UI depends on.
    // The C++ Python server does the same via DefMgr.getChangesForVersion()
    // which returns invalidateAll=True when the version gap is too large.
    let (version, num_elements, invalidate_all, should_push) = match resource_cache {
        Some(cache) => match cache.category(category_id) {
            Some(cat) => {
                let invalidate = client_version != cat.metadata;
                let count = cat.elements.len() as u32;
                // Proactively push all elements when the client's cache is stale.
                // The C++ Python server does this for categories 12-20 (its
                // categoryMaps); we do it for ALL categories to ensure the client
                // receives cooked data regardless of its local PAK state.
                (cat.metadata, count, invalidate, invalidate)
            }
            None => (client_version, 0u32, false, false),
        },
        None => (client_version, 0u32, false, false),
    };

    let required_updates = if should_push { num_elements } else { 0 };

    tracing::info!(
        %addr, category_id, client_version, version, required_updates, invalidate_all,
        "Responding to versionInfoRequest"
    );

    let account_eid = get_account_entity_id(connected, addr)?;
    let (acks, seq) = drain_acks_and_seq(connected, addr)?;
    let pkt = build_version_info(&key, seq, &acks, category_id, version, required_updates, invalidate_all, account_eid);
    socket.send_to(&pkt, addr).await?;

    // Proactively push all elements when the client needs to reload.
    if should_push {
        if let Some(cache) = resource_cache {
            if let Some(cat) = cache.category(category_id) {
                send_category_resources(socket, addr, key, category_id, cat, connected).await?;
            }
        }
    }

    Ok(())
}

/// Proactively send all resources for a category as BASEMSG_RESOURCE_FRAGMENT packets.
///
/// Called immediately after onVersionInfo when the client's cache is stale,
/// rather than waiting for individual elementDataRequests.
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
                &key, seq, &acks,
                data_id, i as u8, frag_flags,
                mt, cat_id, elem, chunk,
            );
            socket.send_to(&pkt, addr).await?;
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
            &key, seq, &acks,
            data_id, i as u8, frag_flags,
            mt, cat, elem, chunk,
        );
        socket.send_to(&pkt, addr).await?;
    }

    tracing::debug!(%addr, element_id, total_chunks, "Resource fragments sent");

    Ok(())
}
