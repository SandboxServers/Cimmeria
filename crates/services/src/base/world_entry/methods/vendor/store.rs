use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::transport::Transport;
use sqlx::PgPool;

use super::super::super::super::helpers::send_to_witness_reliable;
use super::super::super::super::ConnectedClientState;
use super::data::{
    load_store_buy_items, load_vendor_buyback_prices, load_vendor_recharge_prices,
    load_vendor_repair_prices, load_vendor_sell_prices,
};
use super::serializers::{
    serialize_empty_store_open, serialize_store_open, serialize_store_update, StoreItemCostUpdate,
};
use crate::cell::client_methods::player::{ON_STORE_OPEN, ON_STORE_UPDATE};
use crate::mercury::build_player_entity_method_packet;

#[derive(sqlx::FromRow)]
pub struct VendorTemplateLists {
    pub buy_item_list: Option<i32>,
    pub sell_item_list: Option<i32>,
    pub repair_item_list: Option<i32>,
    pub recharge_item_list: Option<i32>,
}

/// Open a vendor store for a player, loading all item lists and pricing.
#[tracing::instrument(
    name = "vendor.open_store",
    level = "info",
    skip_all,
    fields(entity_id, player_id, vendor_entity_id, ?vendor_template_id),
)]
pub async fn handle_open_vendor_store(
    entity_id: u32,
    player_id: i32,
    vendor_entity_id: i32,
    vendor_template_id: Option<i32>,
    db_pool: &Option<Arc<PgPool>>,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    let Some(pool) = db_pool else {
        send_store_open_to_client(
            entity_id,
            vendor_entity_id,
            serialize_empty_store_open(vendor_entity_id),
            transport,
            connected,
            entity_to_addr,
        )
        .await;
        tracing::debug!(entity_id, vendor_entity_id, "OpenVendorStore: no DB pool");
        return;
    };

    let Some(template_id) = vendor_template_id else {
        send_store_open_to_client(
            entity_id,
            vendor_entity_id,
            serialize_empty_store_open(vendor_entity_id),
            transport,
            connected,
            entity_to_addr,
        )
        .await;
        tracing::debug!(
            entity_id,
            vendor_entity_id,
            "OpenVendorStore: vendor template missing"
        );
        return;
    };

    let template = match sqlx::query_as::<_, VendorTemplateLists>(
        "SELECT buy_item_list, sell_item_list, repair_item_list, recharge_item_list \
         FROM resources.entity_templates WHERE template_id = $1",
    )
    .bind(template_id)
    .fetch_optional(pool.as_ref())
    .await
    {
        Ok(Some(row)) => row,
        Ok(None) => {
            tracing::warn!(template_id, "OpenVendorStore: template not found");
            VendorTemplateLists {
                buy_item_list: None,
                sell_item_list: None,
                repair_item_list: None,
                recharge_item_list: None,
            }
        }
        Err(e) => {
            tracing::error!(template_id, "OpenVendorStore: template query failed: {e}");
            VendorTemplateLists {
                buy_item_list: None,
                sell_item_list: None,
                repair_item_list: None,
                recharge_item_list: None,
            }
        }
    };

    let buy_items = load_store_buy_items(pool, template.buy_item_list).await;
    let sell_prices = load_vendor_sell_prices(pool, player_id, template.sell_item_list).await;
    let buyback_prices = load_vendor_buyback_prices(pool, player_id).await;
    let repair_prices = load_vendor_repair_prices(pool, player_id, template.repair_item_list).await;
    let recharge_prices =
        load_vendor_recharge_prices(pool, player_id, template.recharge_item_list).await;

    let args = serialize_store_open(
        vendor_entity_id,
        &buy_items,
        &sell_prices,
        &buyback_prices,
        &repair_prices,
        &recharge_prices,
    );

    let buy_count = buy_items.len();
    let sell_count = sell_prices.len();
    let buyback_count = buyback_prices.len();
    let repair_count = repair_prices.len();
    let recharge_count = recharge_prices.len();

    send_store_open_to_client(
        entity_id,
        vendor_entity_id,
        args,
        transport,
        connected,
        entity_to_addr,
    )
    .await;
    tracing::debug!(
        entity_id,
        player_id,
        vendor_entity_id,
        template_id,
        buy_count,
        sell_count,
        buyback_count,
        repair_count,
        recharge_count,
        "OpenVendorStore: sent"
    );
}

/// Send vendor store open to client.
///
/// `onStoreOpen` is SGWPlayer flat index 109 (`SGWPlayer.def`), which is
/// above `IDBASE_SGW_PLAYER` (61) and therefore rides the extended
/// encoding: `[0xBD][word_len: u16][entity_id: u32][109 - 61 = 48][args]`.
pub async fn send_store_open_to_client(
    entity_id: u32,
    vendor_entity_id: i32,
    args: Vec<u8>,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    send_to_witness_reliable(
        transport,
        connected,
        entity_to_addr,
        entity_id,
        |key, version, seq, acks| {
            build_player_entity_method_packet(
                key,
                seq,
                acks,
                entity_id,
                ON_STORE_OPEN,
                &args,
                version,
            )
        },
    )
    .await;
    tracing::trace!(entity_id, vendor_entity_id, "Sent onStoreOpen");
}

/// Send vendor store price updates to client.
///
/// `onStoreUpdate` is SGWPlayer flat index 110 — extended encoding, sub-slot
/// byte `110 - 61 = 49`.
pub async fn send_store_update_to_client(
    entity_id: u32,
    updates: &[StoreItemCostUpdate],
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    if updates.is_empty() {
        return;
    }

    let args = serialize_store_update(updates);
    send_to_witness_reliable(
        transport,
        connected,
        entity_to_addr,
        entity_id,
        |key, version, seq, acks| {
            build_player_entity_method_packet(
                key,
                seq,
                acks,
                entity_id,
                ON_STORE_UPDATE,
                &args,
                version,
            )
        },
    )
    .await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{test_default_connected_client_state, TestTransport};
    use cimmeria_mercury::encryption::MercuryEncryption;

    /// `test_default_connected_client_state` builds its cipher from this key,
    /// so the test can decrypt what the handler put on the wire.
    const SESSION_KEY: [u8; 32] = [0u8; 32];

    /// Extended-encoding marker byte emitted for any method index at or above
    /// `IDBASE_SGW_PLAYER`.
    const EXTENDED_MARKER: u8 = 0xBD;

    fn make_client(
        entity_id: u32,
        addr: SocketAddr,
    ) -> (
        Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
        Arc<Mutex<HashMap<u32, SocketAddr>>>,
    ) {
        let connected = Arc::new(Mutex::new(HashMap::from([(
            addr,
            test_default_connected_client_state(),
        )])));
        let entity_to_addr = Arc::new(Mutex::new(HashMap::from([(entity_id, addr)])));
        (connected, entity_to_addr)
    }

    /// Decrypt one recorded packet and return its Mercury body (flags byte
    /// stripped from the front, 4-byte seq footer stripped from the back —
    /// `pending_acks` is empty in the test client state, so `FLAG_HAS_ACKS`
    /// is clear and the seq is the only footer).
    fn decrypt_body(packet: &[u8]) -> Vec<u8> {
        let enc = MercuryEncryption::from_session_key(SESSION_KEY);
        let pt = enc.decrypt(packet).expect("decrypt vendor packet");
        pt[1..pt.len() - 4].to_vec()
    }

    /// Wire-format regression guard: the vendor store-open payload must be
    /// addressed to SGWPlayer flat method **109** (`onStoreOpen`).
    ///
    /// Bug shape: the emit path used a `method_idx` constant that claimed a
    /// nonexistent "SGWVendorStore interface" at 80/81. Index 80 is
    /// `onMissionUpdate` (Missionary interface), so the entire store payload
    /// was delivered to the client's mission handler — a silent
    /// wrong-recipient dispatch that no length or DB assertion can see.
    ///
    /// The sub-slot byte is asserted as a **literal** `0x30`, not computed
    /// from the constant, so this fails if the emit path is pointed back at
    /// 80 (whose sub-slot byte is `80 - 61 = 19 = 0x13`). Computing it from
    /// `ON_STORE_OPEN` would make the assertion tautological.
    #[tokio::test]
    async fn store_open_is_emitted_on_method_109_not_a_mission_index() {
        let transport = Arc::new(TestTransport::new());
        let dyn_transport: Arc<dyn Transport> = transport.clone();

        let entity_id = 0x0000_1234u32;
        let vendor_entity_id = 777i32;
        let addr: SocketAddr = "127.0.0.1:41090".parse().unwrap();
        let (connected, entity_to_addr) = make_client(entity_id, addr);

        // A short, recognisable arg blob: the byte layout of the store
        // payload is pinned by the serializer's own tests; this test is
        // about which method index carries it.
        let args = vec![0xAAu8, 0xBB, 0xCC, 0xDD];

        send_store_open_to_client(
            entity_id,
            vendor_entity_id,
            args.clone(),
            &dyn_transport,
            &connected,
            &entity_to_addr,
        )
        .await;

        let sent = transport.drain();
        assert_eq!(sent.len(), 1, "store open is one packet to the owner");
        assert_eq!(sent[0].0, addr, "store open goes to the vendor user's addr");

        let body = decrypt_body(&sent[0].1);

        // Extended encoding: [0xBD][word_len: u16 LE][entity_id: u32 LE][sub_index: u8][args]
        let expected_len = u16::try_from(4 + 1 + args.len()).unwrap();
        let mut expected = vec![EXTENDED_MARKER];
        expected.extend_from_slice(&expected_len.to_le_bytes());
        expected.extend_from_slice(&entity_id.to_le_bytes());
        expected.push(0x30); // 109 - IDBASE_SGW_PLAYER(61) = 48 = 0x30
        expected.extend_from_slice(&args);

        assert_eq!(
            body, expected,
            "onStoreOpen must ride method 109 (sub-slot byte 0x30). A sub-slot \
             byte of 0x13 means the emit path regressed to index 80, which is \
             onMissionUpdate — the client would route the store payload into \
             its mission handler"
        );
    }

    /// Same guard for the price-update emit: `onStoreUpdate` is flat method
    /// **110**, sub-slot byte `110 - 61 = 49 = 0x31`. The regression value to
    /// trip on is `0x14` (index 81 = `onStepUpdate`).
    #[tokio::test]
    async fn store_update_is_emitted_on_method_110_not_a_mission_index() {
        let transport = Arc::new(TestTransport::new());
        let dyn_transport: Arc<dyn Transport> = transport.clone();

        let entity_id = 0x0000_5678u32;
        let addr: SocketAddr = "127.0.0.1:41091".parse().unwrap();
        let (connected, entity_to_addr) = make_client(entity_id, addr);

        let updates = vec![StoreItemCostUpdate {
            item_id: 4242,
            sell_price: 10,
            repair_price: 20,
            recharge_price: 30,
        }];

        send_store_update_to_client(
            entity_id,
            &updates,
            &dyn_transport,
            &connected,
            &entity_to_addr,
        )
        .await;

        let sent = transport.drain();
        assert_eq!(sent.len(), 1, "store update is one packet to the owner");
        assert_eq!(sent[0].0, addr);

        let body = decrypt_body(&sent[0].1);
        let args = serialize_store_update(&updates);

        let expected_len = u16::try_from(4 + 1 + args.len()).unwrap();
        let mut expected = vec![EXTENDED_MARKER];
        expected.extend_from_slice(&expected_len.to_le_bytes());
        expected.extend_from_slice(&entity_id.to_le_bytes());
        expected.push(0x31); // 110 - IDBASE_SGW_PLAYER(61) = 49 = 0x31
        expected.extend_from_slice(&args);

        assert_eq!(
            body, expected,
            "onStoreUpdate must ride method 110 (sub-slot byte 0x31). A \
             sub-slot byte of 0x14 means the emit path regressed to index 81, \
             which is onStepUpdate"
        );
    }

    /// An empty update list must not put anything on the wire — the early
    /// return exists so the client isn't handed a zero-length array to
    /// re-render the store from.
    #[tokio::test]
    async fn store_update_with_no_rows_sends_nothing() {
        let transport = Arc::new(TestTransport::new());
        let dyn_transport: Arc<dyn Transport> = transport.clone();

        let entity_id = 0x0000_9ABCu32;
        let addr: SocketAddr = "127.0.0.1:41092".parse().unwrap();
        let (connected, entity_to_addr) = make_client(entity_id, addr);

        send_store_update_to_client(entity_id, &[], &dyn_transport, &connected, &entity_to_addr)
            .await;

        assert!(
            transport.is_empty(),
            "empty update list must short-circuit before the send"
        );
    }
}
