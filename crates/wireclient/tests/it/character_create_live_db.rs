//! A character created on the wire, as a new player makes one (D-WC11).
//!
//! A wire-created character carries `first_login = 1`, the real start
//! profile and the real starter kit, which a row the tests insert directly
//! does not (docs/analysis/wireclient-ci F13). Run with
//! `pwsh tools/build-lane/live-db-test.ps1 --wireclient character_create_live_db`.

use std::time::Duration;

use cimmeria_wireclient::bundle::decode_bundle_strict;
use cimmeria_wireclient::error::Error;
use cimmeria_wireclient::session::GameSession;

use crate::support::{
    self, cleanup_account, credentials_for, default_visual_choices,
    insert_sentinel_account_with_level, start_server, SHARD,
};

/// Campaign sentinel block 0x7000_9C00..=0x7000_9CFF; WC-10 owns 0x..01.
const ACCOUNT: i32 = 0x7000_9C01;
const ACCOUNT_NAME: &str = "wc10_praxis";
/// Praxis Commando, male.
const CHAR_DEF_PRAXIS: i32 = 3;
const RECV: Duration = Duration::from_secs(5);
/// `onUpdateItem` on the player (idbase 61): the starter kit at entry.
const ON_UPDATE_ITEM: u16 = 72;
/// The server's refusal code for an unknown char def (`fail_code.rs`).
const INVALID_CHARACTER_TYPE: i32 = 10001;

#[tokio::test]
async fn wire_created_praxis_character_enters_castle_cellblock_live_db() {
    let pool = cimmeria_test_support::require_db_or_skip!();
    cleanup_account(&pool, ACCOUNT).await;
    insert_sentinel_account_with_level(&pool, ACCOUNT, ACCOUNT_NAME, 0).await;
    let server = start_server(&support::db_url()).await;

    let mut session =
        GameSession::connect(&server.auth_url, &credentials_for(ACCOUNT_NAME), SHARD, 1)
            .await
            .expect("connect");
    session
        .open_character_list(RECV)
        .await
        .expect("character list");
    let choices = default_visual_choices(&pool, CHAR_DEF_PRAXIS).await;
    session
        .create_character_on_wire("Wirepraxis", CHAR_DEF_PRAXIS, &choices, RECV)
        .await
        .expect("createCharacter accepted");

    let rows: Vec<(i32, i32, String)> = sqlx::query_as(
        "SELECT player_id, first_login, world_location FROM sgw_player WHERE account_id = $1",
    )
    .bind(ACCOUNT)
    .fetch_all(&pool)
    .await
    .expect("read the new character");
    assert_eq!(rows.len(), 1, "one character for the account: {rows:?}");
    let (player_id, first_login, world_location) = rows[0].clone();
    assert_eq!(first_login, 1, "a new character's first login is pending");
    assert_eq!(world_location, "Castle_CellBlock");

    // Only world entry records bundles: the character list, then play's.
    assert_eq!(
        session.entry_bundles.len(),
        1,
        "only the character list so far"
    );
    session.play(player_id, RECV).await.expect("play");
    assert!(session.player_entity_id.is_some(), "own entity id set");
    let entry = &session.entry_bundles[1..];
    assert!(
        entry.len() >= 3,
        "world entry recorded {} bundles",
        entry.len()
    );
    let kit_sent = entry.iter().any(|b| {
        decode_bundle_strict(b)
            .expect("entry bundle decodes to its last byte")
            .iter()
            .any(|m| m.method_index_for(61) == Some(ON_UPDATE_ITEM))
    });
    assert!(kit_sent, "no onUpdateItem in the world-entry bundles");

    server.orchestrator.stop_all().await;
    cleanup_account(&pool, ACCOUNT).await;
}

#[tokio::test]
async fn create_character_refusal_is_an_error_live_db() {
    const REFUSED_ACCOUNT: i32 = 0x7000_9C04;
    const REFUSED_NAME: &str = "wc10_refused";
    let pool = cimmeria_test_support::require_db_or_skip!();
    cleanup_account(&pool, REFUSED_ACCOUNT).await;
    insert_sentinel_account_with_level(&pool, REFUSED_ACCOUNT, REFUSED_NAME, 0).await;
    let server = start_server(&support::db_url()).await;

    let mut session =
        GameSession::connect(&server.auth_url, &credentials_for(REFUSED_NAME), SHARD, 1)
            .await
            .expect("connect");
    session
        .open_character_list(RECV)
        .await
        .expect("character list");
    let result = session
        .create_character_on_wire("Wirerefused", -1, &[], RECV)
        .await;
    assert!(
        matches!(
            result,
            Err(Error::CharacterCreateFailed(INVALID_CHARACTER_TYPE))
        ),
        "char def -1 must be refused, got {result:?}"
    );

    server.orchestrator.stop_all().await;
    cleanup_account(&pool, REFUSED_ACCOUNT).await;
}
