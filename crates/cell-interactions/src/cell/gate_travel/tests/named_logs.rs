//! Rule 6 on the gate-travel dial line (NT-23): an accepted dial names the
//! traveller and the gate they dialled, not only the entity slot and the
//! address id.

use cimmeria_names::{NameBook, Table};

use super::*;

#[tokio::test]
async fn an_accepted_dial_names_the_traveller_and_the_destination_gate() {
    let mut book = NameBook::empty();
    book.insert(Table::Stargates, 2, "NT23_Castle_Gate");
    cimmeria_names::global().store(book);
    let capture = crate::test_support::LogCapture::install();
    let mut mgr = make_manager_with_stargates();
    mgr.create_entity(1, "Agnos", [10.0, 0.0, 10.0], [0.0; 3])
        .unwrap();
    mgr.get_entity_mut(1)
        .unwrap()
        .stamp_log_names(Some("Teal'c"), Some("sgc_login"));
    grant_all_addresses(&mut mgr, 1);
    mgr.connect_entity(1);

    let (tx, _rx) = tokio::sync::mpsc::channel(16);
    handle_dial_gate(1, 2, 0, &tx, &mut mgr, &engine()).await;
    cimmeria_names::global().store(NameBook::empty());

    let ev = capture
        .find_message(tracing::Level::INFO, "Gate travel: dial accepted")
        .expect("the accepted dial must log");
    for (key, want) in [
        ("entity_id", "1"),
        ("entity_name", "Teal'c"),
        ("target_address_id", "2"),
        ("target_address_name", "NT23_Castle_Gate"),
    ] {
        assert!(
            ev.has_field(key, want),
            "expected {key}={want}; got {ev:#?}"
        );
    }
}
