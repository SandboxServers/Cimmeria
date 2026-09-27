//! `CellToBaseMsg::Bank(GmDump)` routing (bank-vault BV-04): the arm runs
//! the dump and answers the GM. With no pool the answer is the
//! `db_unavailable` line, which is enough to prove the arm reaches
//! `bank_dump::handle_gm_dump`; the read itself is tested live in
//! `cimmeria-base-session` `base::bank_dump::tests`.

use super::super::*;
use super::one_session;
use crate::cell::messages::{BankCellToBase, BankSubject};
use crate::test_support::{LogCapture, TestTransport};

#[tokio::test]
async fn bank_gm_dump_arm_answers_the_gm() {
    let capture = LogCapture::install();
    let (addr, connected, entity_to_addr) = one_session(4242, false);
    connected
        .lock()
        .unwrap()
        .get_mut(&addr)
        .unwrap()
        .player_entity_id = Some(4242);
    let typed_transport = Arc::new(TestTransport::new());
    let transport: Arc<dyn Transport> = typed_transport.clone();

    handle_cell_message(
        CellToBaseMsg::Bank(BankCellToBase::GmDump {
            entity_id: 4242,
            account_id: Some(6),
            player_id: Some(5),
            subject: BankSubject::Name("Kasuf".into()),
        }),
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

    assert_eq!(
        typed_transport.filter_to(addr).len(),
        1,
        "one line to the GM"
    );
    let row = capture
        .find_message(tracing::Level::WARN, "bankdump could not read the vault")
        .expect("the arm logs gm_action");
    assert!(row.has_field("event", "gm_action"));
    assert!(row.has_field("reason", "db_unavailable"));
    assert!(row.has_field("entity_id", "4242"));
}

/// `Bank(Expand)` and `Bank(ExpansionQuote)` routing (BV-05): with no pool
/// each arm reaches its `bank_expand` handler, which logs its
/// `db_unavailable` row; the purchase also sends the player one line. The
/// handlers are tested live in `cimmeria-base-session`
/// `base::bank_expand::{tests, quote_tests}`.
#[tokio::test]
async fn bank_expand_arms_reach_the_expand_handlers() {
    use cimmeria_wire::cell::vault::VaultAccess;

    let capture = LogCapture::install();
    let (addr, connected, entity_to_addr) = one_session(4243, false);
    connected
        .lock()
        .unwrap()
        .get_mut(&addr)
        .unwrap()
        .player_entity_id = Some(4243);
    let typed_transport = Arc::new(TestTransport::new());
    let transport: Arc<dyn Transport> = typed_transport.clone();

    for msg in [
        BankCellToBase::Expand {
            entity_id: 4243,
            account_id: Some(6),
            player_id: 5,
            from_slots: Some(40),
            vault: VaultAccess::NO_SESSION,
        },
        BankCellToBase::ExpansionQuote {
            entity_id: 4243,
            account_id: Some(6),
            player_id: 5,
            speaker_id: 99,
        },
    ] {
        handle_cell_message(
            CellToBaseMsg::Bank(msg),
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
    }

    assert_eq!(typed_transport.filter_to(addr).len(), 1, "one refusal line");
    for event in ["expand_rejected", "expand_quote"] {
        let row = capture
            .all()
            .into_iter()
            .find(|c| c.target == "bank" && c.has_field("event", event))
            .unwrap_or_else(|| panic!("the arm logs {event}"));
        assert!(row.has_field("reason", "db_unavailable"), "{row:#?}");
        assert!(row.has_field("entity_id", "4243"), "{row:#?}");
    }
}
