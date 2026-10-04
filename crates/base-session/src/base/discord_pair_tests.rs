//! `ConnectedClientState::discord_account` / `discord_character` (NT-10):
//! the base's pairs for the account and the character being played.

use cimmeria_discord::Named;

use crate::test_support::test_default_connected_client_state;

#[test]
fn account_pairs_account_id_with_login_name() {
    let mut s = test_default_connected_client_state();
    s.account_id = 6;
    s.account_name = Some("steve".into());
    assert_eq!(s.discord_account(), Named::new(6, Some("steve".into())));
}

/// The character pairs with `active_player_id`, the DB `player_id`, never
/// an entity ID.
#[test]
fn character_pairs_active_player_id_with_name() {
    let mut s = test_default_connected_client_state();
    s.player_entity_id = Some(4711);
    s.active_player_id = Some(12);
    s.player_name = Some("alice".into());
    assert_eq!(
        s.discord_character(),
        Some(Named::new(12, Some("alice".into())))
    );
}

/// At character select neither half is set, and there is no character.
#[test]
fn no_character_at_character_select() {
    let mut s = test_default_connected_client_state();
    s.account_id = 6;
    assert_eq!(s.discord_character(), None);
}

/// With an empty NameBook a world still carries its name.
#[test]
fn world_without_a_book_entry_keeps_its_name() {
    assert_eq!(
        super::discord_world("Nowhere_Test_World"),
        Named::name_only("Nowhere_Test_World")
    );
}
