//! `CellToBaseMsg::BlackMarket` routing (BM-01): every nested variant reaches
//! its own base handler. With no DB pool each handler refuses at its first
//! guard (`BMUnavailable`, BM-02) and logs `bm.refused` with its own `op`,
//! which names the handler that ran.

use super::super::*;
use super::empty_maps;
use crate::cell::messages::BlackMarketCellToBase;
use crate::test_support::{LogCapture, TestTransport};
use cimmeria_wire::black_market::BMSearchOptions;

#[tokio::test]
async fn every_black_market_variant_reaches_its_handler() {
    let capture = LogCapture::install();
    let msgs = [
        (
            BlackMarketCellToBase::Search {
                entity_id: 21,
                player_id: 11,
                options: BMSearchOptions::default(),
            },
            "search",
        ),
        (
            BlackMarketCellToBase::CreateAuction {
                entity_id: 21,
                player_id: 11,
                item_id: 7,
                starting_price: 10,
                buyout_price: 20,
                auction_length: 3,
            },
            "create",
        ),
        (
            BlackMarketCellToBase::PlaceBid {
                entity_id: 21,
                player_id: 11,
                sequence_id: 5,
                bid_amount: 10,
            },
            "bid",
        ),
        (
            BlackMarketCellToBase::CancelAuction {
                entity_id: 21,
                player_id: 11,
                sequence_id: 5,
            },
            "cancel",
        ),
    ];
    for (msg, expected) in msgs {
        let typed_transport = Arc::new(TestTransport::new());
        let transport: Arc<dyn Transport> = typed_transport.clone();
        let (connected, entity_to_addr) = empty_maps();
        handle_cell_message(
            CellToBaseMsg::BlackMarket(msg),
            &transport,
            &connected,
            &entity_to_addr,
            &None,
            &None,
            &None,
            "127.0.0.1",
            7777,
        )
        .await;
        assert!(
            capture.all().iter().any(|e| {
                e.message_contains("Black Market request refused")
                    && e.has_field("op", expected)
                    && e.has_field("reason", "bm_unavailable")
            }),
            "missing the {expected:?} refusal: {:#?}",
            capture.all()
        );
        assert!(typed_transport.is_empty(), "{expected}: nothing sent");
    }
}
