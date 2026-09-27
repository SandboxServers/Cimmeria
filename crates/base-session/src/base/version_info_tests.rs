//! The category-12 (`CookedWorldInfo`) version handshake, end to end on the
//! wire, against the real committed PAKs.
//!
//! A client holding the shipped world table must be told about exactly the
//! seven historical CellBlock worlds, per key, and then be sent each entry:
//! `onVersionInfo(invalidate_all = false, RequiredUpdates = 7,
//! InvalidKeys = [1201..1207])` followed by seven single-fragment
//! `resourceFragment` transfers. `invalidate_all = true` here would empty the
//! client's whole world table (the 2026-09-20 Kismet sequence wipe, on
//! category 1).

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::encryption::MercuryEncryption;
use cimmeria_mercury::transport::Transport;

use super::cooked_data::handle_version_info_request;
use super::resources::ResourceCache;
use crate::mercury::FRAG_FIRST_AND_LAST;
use crate::test_support::{test_default_connected_client_state, TestTransport};

const CATEGORY_WORLD_INFO: u32 = 12;
/// `MetaData` of the `CookedWorldInfo.pak` clients ship with.
const CLIENT_SHIPPED_WORLD_INFO_VERSION: u32 = 5959;
const HISTORICAL_CELLBLOCK_IDS: [u32; 7] = [1201, 1202, 1203, 1204, 1205, 1206, 1207];
/// Wire ids (crate-private in `cimmeria-wire`).
const BASEMSG_ON_VERSION_INFO: u8 = 0x80;
const BASEMSG_RESOURCE_FRAGMENT: u8 = 0x36;
/// The fixture session's key (`test_default_connected_client_state`).
const KEY: [u8; 32] = [0u8; 32];

fn committed_cache() -> Arc<ResourceCache> {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../data/cache");
    Arc::new(ResourceCache::load_all(dir).expect("committed PAKs load"))
}

/// Answer one `versionInfoRequest` for category 12 at `client_version` and
/// return every decrypted plaintext sent to the client, in order.
async fn request_world_info(cache: &Arc<ResourceCache>, client_version: u32) -> Vec<Vec<u8>> {
    let addr: SocketAddr = "127.0.0.1:5812".parse().unwrap();
    let connected = Arc::new(Mutex::new(HashMap::from([(
        addr,
        test_default_connected_client_state(),
    )])));
    let sent = Arc::new(TestTransport::new());
    let transport: Arc<dyn Transport> = sent.clone();

    let mut payload = CATEGORY_WORLD_INFO.to_le_bytes().to_vec();
    payload.extend_from_slice(&client_version.to_le_bytes());
    handle_version_info_request(
        &transport,
        addr,
        KEY,
        &payload,
        &connected,
        &Some(Arc::clone(cache)),
    )
    .await
    .expect("versionInfoRequest handled");

    let enc = MercuryEncryption::from_session_key(KEY);
    sent.filter_to(addr)
        .iter()
        .map(|pkt| enc.decrypt(pkt).expect("decrypt"))
        .collect()
}

fn u32_at(b: &[u8], off: usize) -> u32 {
    u32::from_le_bytes(b[off..off + 4].try_into().unwrap())
}

/// `onVersionInfo` fields: (category, version, RequiredUpdates,
/// invalidate_all, InvalidKeys).
fn parse_version_info(pt: &[u8]) -> (u32, u32, u32, bool, Vec<u32>) {
    // [flags][msg_id][u16 len] then account_eid, category, version,
    // required_updates, invalidate_all u8, InvalidKeys ARRAY<u32>.
    assert_eq!(pt[1], BASEMSG_ON_VERSION_INFO);
    let p = &pt[4..];
    let count = u32_at(p, 17) as usize;
    let keys = (0..count).map(|i| u32_at(p, 21 + i * 4)).collect();
    (u32_at(p, 4), u32_at(p, 8), u32_at(p, 12), p[16] != 0, keys)
}

#[tokio::test]
async fn shipped_client_is_sent_the_seven_historical_worlds_per_key() {
    let cache = committed_cache();
    let served = cache.category(CATEGORY_WORLD_INFO).unwrap().metadata;
    let packets = request_world_info(&cache, CLIENT_SHIPPED_WORLD_INFO_VERSION).await;

    assert_eq!(
        packets.len(),
        1 + HISTORICAL_CELLBLOCK_IDS.len(),
        "one onVersionInfo, then one resourceFragment per world"
    );
    let (category, version, required, invalidate_all, keys) = parse_version_info(&packets[0]);
    assert_eq!(category, CATEGORY_WORLD_INFO);
    assert_eq!(version, served);
    assert!(
        !invalidate_all,
        "invalidate_all would wipe the client's whole world table"
    );
    assert_eq!(required, 7, "RequiredUpdates");
    assert_eq!(keys, HISTORICAL_CELLBLOCK_IDS);

    // Each push: [flags][0x36][u16 len] data_id(2) chunk_id(1) frag_flags(1)
    // msg_type(1) category(4) element(4) xml.
    for (pt, &world_id) in packets[1..].iter().zip(&HISTORICAL_CELLBLOCK_IDS) {
        assert_eq!(pt[1], BASEMSG_RESOURCE_FRAGMENT);
        let len = u16::from_le_bytes([pt[2], pt[3]]) as usize;
        let p = &pt[4..4 + len];
        assert_eq!(p[2], 0, "chunk_id: the whole entry is one fragment");
        assert_eq!(p[3], FRAG_FIRST_AND_LAST);
        assert_eq!(u32_at(p, 5), CATEGORY_WORLD_INFO);
        assert_eq!(u32_at(p, 9), world_id);
        assert_eq!(
            Some(&p[13..].to_vec()),
            cache.get(CATEGORY_WORLD_INFO, world_id),
            "world {world_id} must be pushed verbatim"
        );
    }
}

/// A client that already holds the served version gets no invalidation and
/// no pushes, so every later login is a no-op for this category.
#[tokio::test]
async fn up_to_date_client_is_sent_nothing_more() {
    let cache = committed_cache();
    let served = cache.category(CATEGORY_WORLD_INFO).unwrap().metadata;
    let packets = request_world_info(&cache, served).await;

    assert_eq!(packets.len(), 1, "onVersionInfo only");
    let (_, version, required, invalidate_all, keys) = parse_version_info(&packets[0]);
    assert_eq!(version, served);
    assert_eq!(required, 0);
    assert!(!invalidate_all);
    assert!(keys.is_empty());
}
