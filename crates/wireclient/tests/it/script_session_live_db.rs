//! `ScriptSession` against a real server (WC-12): a wire-created Praxis
//! character's arrival dialog reaches the script's log. Run with
//! `pwsh tools/build-lane/live-db-test.ps1 --wireclient script_session`.
//!
//! A fresh character's first login holds entity introductions for up to 16 s
//! behind the intro movie (F13); this test sends no `cancelMovie`, so the
//! script also has to keep the session alive across that hold on its own
//! keep-alives.

use std::time::Duration;

use cimmeria_wireclient::script::ScriptSession;
use cimmeria_wireclient::semantic::S2CEvent;
use cimmeria_wireclient::session::GameSession;

use crate::support::{
    self, cleanup_account, credentials_for, default_visual_choices,
    insert_sentinel_account_with_level, start_server, SHARD,
};

/// Campaign sentinel block 0x7000_9C00..=0x7000_9CFF; WC-12 owns 0x..05.
const ACCOUNT: i32 = 0x7000_9C05;
const ACCOUNT_NAME: &str = "wc12_script";
/// Praxis Commando, male.
const CHAR_DEF_PRAXIS: i32 = 3;
const RECV: Duration = Duration::from_secs(5);
/// The Cellblock arrival dialog.
const ARRIVAL_DIALOG: i32 = 2982;

#[tokio::test]
async fn script_session_live_db_sees_the_arrival_dialog() {
    let pool = cimmeria_test_support::require_db_or_skip!();
    support::init_test_logging();
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
        .create_character_on_wire("Wirepump", CHAR_DEF_PRAXIS, &choices, RECV)
        .await
        .expect("createCharacter accepted");
    let (player_id,): (i32,) =
        sqlx::query_as("SELECT player_id FROM sgw_player WHERE account_id = $1")
            .bind(ACCOUNT)
            .fetch_one(&pool)
            .await
            .expect("the new character");
    session.play(player_id, RECV).await.expect("play");
    let own_id = session.player_entity_id.expect("own entity id");

    let mut script = ScriptSession::new(session).expect("entry bundles decode and apply");
    assert_eq!(script.mirror.own_id(), Some(own_id));
    assert!(
        !script.log().is_empty(),
        "world entry's bundles are in the log"
    );

    let seen = script
        .wait_event("the arrival dialog 2982", Duration::from_secs(30), |o| {
            matches!(&o.event, Some(S2CEvent::DialogDisplay(d)) if d.dialog_id == ARRIVAL_DIALOG)
        })
        .await
        .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(seen.msg.entity_id, Some(own_id), "sent to the player");

    server.orchestrator.stop_all().await;
    cleanup_account(&pool, ACCOUNT).await;
}
