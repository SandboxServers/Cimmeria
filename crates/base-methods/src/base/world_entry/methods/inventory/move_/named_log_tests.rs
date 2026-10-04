//! NT-22 (Rule 6): `move_accepted`, the vault move's success line, names
//! every ID it carries: the account, the character, the item and both
//! containers. No database: the line is written from the committed move.
//!
//! Sentinels: account and player `0x7000_D200..=0x7000_D201`, entity
//! `0x7000_D2E0`. Item and container IDs are the stock ones, named here by a
//! test NameBook so the line does not depend on the seed's spelling.

use cimmeria_entity::cell_entity::VaultScope;
use cimmeria_entity::known_names;
use cimmeria_names::{NameBook, Table};
use cimmeria_wire::cell::vault::VaultAccess;
use tracing::Level;

use super::bank_rules::{log_move_accepted, AcceptedVaultMove, VaultOwner};
use crate::test_support::LogCapture;

const ACCOUNT: i32 = 0x7000_D200;
const PLAYER: i32 = 0x7000_D201;
const ENTITY: u32 = 0x7000_D2E0;
const SLAPPACK: i32 = 2893;

fn name_the_world() {
    let mut book = NameBook::empty();
    book.insert(Table::Items, i64::from(SLAPPACK), "Slappack TC1");
    book.insert(Table::Containers, 1, "MAIN");
    book.insert(Table::Containers, 17, "BANK");
    cimmeria_names::global().store(book);
    known_names::remember_player(PLAYER, "Vala Mal Doran");
    known_names::remember_account(ACCOUNT, "nt22_login");
}

/// A deposit from the main bag into the vault names the player, the item
/// and both ends. Fails if any of the name fields is dropped from the line.
#[test]
fn move_accepted_names_the_player_the_item_and_both_containers() {
    name_the_world();
    let capture = LogCapture::install();
    let vault = VaultAccess::Open {
        scope: VaultScope::Personal,
        org_id: None,
        banker_id: Some(0x7000_D2D0),
        distance: Some(2.0),
    };

    log_move_accepted(
        &AcceptedVaultMove {
            owner: VaultOwner {
                account_id: ACCOUNT,
                bank_slots: 40,
            },
            entity_id: ENTITY,
            player_id: PLAYER,
            item_id: 0x7000_D2A0,
            type_id: SLAPPACK,
            quantity: 1,
            source_container_id: 1,
            source_slot_id: 4,
            target_container_id: 17,
            target_slot_id: 0,
            kind: "deposit",
            source_stack_before: 1,
            source_stack_after: 0,
            target_stack_before: 0,
            target_stack_after: 1,
        },
        &vault,
    );

    let line = capture
        .find_message(Level::DEBUG, "move_accepted")
        .expect("move_accepted");
    for (k, v) in [
        ("account_id", ACCOUNT.to_string()),
        ("account_name", "nt22_login".to_string()),
        ("player_id", PLAYER.to_string()),
        ("player_name", "Vala Mal Doran".to_string()),
        ("entity_id", ENTITY.to_string()),
        ("entity_name", "Vala Mal Doran".to_string()),
        ("item_type_id", SLAPPACK.to_string()),
        ("item_name", "Slappack TC1".to_string()),
        ("source_container_name", "MAIN".to_string()),
        ("target_container_name", "BANK".to_string()),
    ] {
        assert!(line.has_field(k, &v), "{k}={v}: {line:#?}");
    }
}

/// An ID the server has no name for leaves the name off the line rather
/// than writing a placeholder (Rule 6: absent when unresolved).
#[test]
fn move_accepted_leaves_an_unknown_name_off_the_line() {
    name_the_world();
    let capture = LogCapture::install();

    log_move_accepted(
        &AcceptedVaultMove {
            owner: VaultOwner {
                account_id: ACCOUNT,
                bank_slots: 40,
            },
            entity_id: ENTITY,
            player_id: PLAYER + 0x10,
            item_id: 0x7000_D2A1,
            type_id: 0x7000_D2F0,
            quantity: 1,
            source_container_id: 1,
            source_slot_id: 4,
            target_container_id: 17,
            target_slot_id: 0,
            kind: "deposit",
            source_stack_before: 1,
            source_stack_after: 0,
            target_stack_before: 0,
            target_stack_after: 1,
        },
        &VaultAccess::NO_SESSION,
    );

    let line = capture
        .find_message(Level::DEBUG, "move_accepted")
        .expect("move_accepted");
    for absent in ["player_name", "entity_name", "item_name"] {
        assert!(!line.fields.contains_key(absent), "{absent}: {line:#?}");
    }
    assert!(line.has_field("target_container_name", "BANK"));
}
