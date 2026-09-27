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
