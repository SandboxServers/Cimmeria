//! `CellToBaseMsg::BlackMarket` routing (BM-01): every nested variant reaches
//! its own base handler. With no DB pool each handler returns at its first
//! guard and logs `"<op>: no DB pool"`, which names the handler that ran.

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
            "search: no DB pool",
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
            "createAuction: no DB pool",
        ),
        (
            BlackMarketCellToBase::PlaceBid {
                entity_id: 21,
                player_id: 11,
                sequence_id: 5,
                bid_amount: 10,
            },
            "placeBid: no DB pool",
        ),
        (
            BlackMarketCellToBase::CancelAuction {
                entity_id: 21,
                player_id: 11,
                sequence_id: 5,
            },
            "cancelAuction: no DB pool",
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
            capture
                .find_message(tracing::Level::DEBUG, expected)
                .is_some(),
            "missing {expected:?}: {:#?}",
            capture.all()
        );
        assert!(typed_transport.is_empty(), "{expected}: nothing sent");
    }
}
