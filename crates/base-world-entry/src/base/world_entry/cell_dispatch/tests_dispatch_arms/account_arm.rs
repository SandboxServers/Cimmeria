//! `CellToBaseMsg::Account` routing (client-tls-oidc C10): the arm runs the
//! GM `.account` handler and answers the GM. With no pool the answer is the
//! `db_unavailable` line, which is enough to prove the arm reaches
//! `account_admin::handle`; the reads are tested live in
//! `cimmeria-base-session` `base::account_admin::tests_live_db`.

use super::super::*;
use super::one_session;
use crate::cell::messages::AccountCellToBase;
use crate::test_support::{LogCapture, TestTransport};
use cimmeria_base_session::base::world_entry::gm_only_worlds::GM_ACCESS_LEVEL;

#[tokio::test]
async fn account_arm_answers_the_gm() {
    let capture = LogCapture::install();
    let (addr, connected, entity_to_addr) = one_session(4242, false);
    {
        let mut clients = connected.lock().unwrap();
        let client = clients.get_mut(&addr).unwrap();
        client.player_entity_id = Some(4242);
        client.access_level = GM_ACCESS_LEVEL;
    }
    let typed_transport = Arc::new(TestTransport::new());
    let transport: Arc<dyn Transport> = typed_transport.clone();

    handle_cell_message(
        CellToBaseMsg::Account(AccountCellToBase::Identities {
            entity_id: 4242,
            account_id: Some(6),
            account_name: "Player_1234".into(),
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
        .find_message(
            tracing::Level::WARN,
            "account command could not reach the identity tables",
        )
        .expect("the arm logs gm_action");
    assert!(row.has_field("event", "gm_action"));
    assert!(row.has_field("reason", "db_unavailable"));
    assert!(row.has_field("entity_id", "4242"));
}

#[tokio::test]
async fn account_arm_refuses_a_player_before_the_handler() {
    let capture = LogCapture::install();
    let (addr, connected, entity_to_addr) = one_session(4242, false);
    let typed_transport = Arc::new(TestTransport::new());
    let transport: Arc<dyn Transport> = typed_transport.clone();

    handle_cell_message(
        CellToBaseMsg::Account(AccountCellToBase::Identities {
            entity_id: 4242,
            account_id: Some(6),
            account_name: "Player_1234".into(),
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

    assert!(
        typed_transport.filter_to(addr).is_empty(),
        "a refused command gets no answer"
    );
    let row = capture
        .find_message(tracing::Level::WARN, "account command refused by the base")
        .expect("the arm logs the refusal");
    assert!(row.has_field("event", "gm_action"));
    assert!(row.has_field("reason", "not_gm"));
}
