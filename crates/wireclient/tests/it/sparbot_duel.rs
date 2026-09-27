//! SS-U2: the `sparbot` duel partner against the server.
//!
//! - [`sparbot_wire_matches_the_server`] (no DB): the bot's duel indices,
//!   texts and payloads decode with the server's own `cimmeria-wire`
//!   decoders, so the bot and the server cannot drift apart silently.
//! - [`sparbot_accepts_a_duel_challenge`] (live DB): a real challenger
//!   session sends `sendDuelChallenge` (0xD9) naming the bot, the bot
//!   answers `onDuelChallenge` with CM 102 = 1, and the challenger is told
//!   the duel was accepted. This is the accept path SS-D2's wireclient test
//!   builds on.
//! - [`sparbot_session_outlives_the_inactivity_reap`] (live DB, ignored:
//!   it runs for over a minute): the bot's keep-alive holds a session past
//!   the server's 60 s client-silence reap, and the bot still accepts a
//!   challenge afterwards.
//!
//! Run the live-DB ones with
//! `cargo test -p cimmeria-wireclient --test it sparbot_duel -- --test-threads=1`
//! (add `--ignored` for the long one) and `DATABASE_URL` set.

use std::time::Duration;

use cimmeria_wireclient::bundle::decode_bundle;
use cimmeria_wireclient::session::GameSession;
use cimmeria_wireclient::sparbot::{
    self, classify, Incoming, Sparbot, SparbotConfig, SparbotReport, StopReason,
};

use crate::support::{
    self, credentials_for, insert_castle_character, insert_sentinel_account, live_db_pool_or_skip,
    start_server, CASTLE_BASE_POS,
};

use cimmeria_wire::cell::client_methods::duel::{
    build_on_duel_challenge, TEXT_DUEL_ABORTED, TEXT_DUEL_ACCEPTED,
};

#[test]
fn sparbot_wire_matches_the_server() {
    use cimmeria_wire::base::duel::{decode_send_duel_challenge, SEND_DUEL_CHALLENGE};
    use cimmeria_wire::cell::cell_methods::player::duel::{
        decode_send_duel_response, DuelResponse,
    };
    use cimmeria_wire::cell::cell_methods::player::{DUEL_FORFEIT, SEND_DUEL_RESPONSE};
    use cimmeria_wire::cell::client_methods::communicator::ON_PLAYER_COMMUNICATION;
    use cimmeria_wire::cell::client_methods::duel::ON_DUEL_CHALLENGE;

    assert_eq!(sparbot::SEND_DUEL_CHALLENGE, SEND_DUEL_CHALLENGE);
    assert_eq!(sparbot::SEND_DUEL_RESPONSE, SEND_DUEL_RESPONSE);
    assert_eq!(sparbot::DUEL_FORFEIT, DUEL_FORFEIT);
    assert_eq!(sparbot::ON_DUEL_CHALLENGE, ON_DUEL_CHALLENGE);
    assert_eq!(sparbot::ON_PLAYER_COMMUNICATION, ON_PLAYER_COMMUNICATION);
    assert_eq!(sparbot::TEXT_DUEL_ABORTED, TEXT_DUEL_ABORTED);

    // The challenge body (after the 3-byte id + word length) is what the
    // base's decoder reads.
    let challenge = sparbot::send_duel_challenge("SparBot", false);
    let call = decode_send_duel_challenge(&challenge[3..]).expect("server decodes it");
    assert_eq!(call.player_name, "SparBot");
    assert!(!call.is_squad());

    // The accept's args (after id, length, entity id and sub-slot byte).
    let accept = sparbot::send_duel_response(7, true);
    assert_eq!(
        decode_send_duel_response(&accept[8..]),
        Ok(DuelResponse::Accept)
    );

    // The server's own onDuelChallenge args classify as a challenge.
    let mut payload = 7u32.to_le_bytes().to_vec();
    payload.push((ON_DUEL_CHALLENGE - 61) as u8);
    payload.extend_from_slice(&build_on_duel_challenge(42, &[]));
    let msg = cimmeria_wireclient::bundle::S2CMessage {
        msg_id: 0xBD,
        entity_id: Some(7),
        class_id: None,
        method_index: Some(ON_DUEL_CHALLENGE),
        payload: payload.into(),
    };
    assert_eq!(
        classify(&msg, 7),
        Some(Incoming::DuelChallenge {
            challenger_entity_id: 42
        })
    );
}

/// Sentinel ids for this module: accounts and players 900_4xx.
const BOT_ACCOUNT: i32 = 900_401;
const CHALLENGER_ACCOUNT: i32 = 900_402;
const BOT_PLAYER: i32 = 900_403;
const CHALLENGER_PLAYER: i32 = 900_404;
const BOT_NAME: &str = "SsuSparBot";

/// Both characters in Castle, 10 units apart (inside the 20-unit
/// challenge range, D-SS19).
async fn seed(pool: &sqlx::PgPool) {
    insert_sentinel_account(pool, BOT_ACCOUNT, "ssu2_sparbot").await;
    insert_sentinel_account(pool, CHALLENGER_ACCOUNT, "ssu2_challenger").await;
    insert_castle_character(pool, BOT_ACCOUNT, BOT_PLAYER, BOT_NAME, CASTLE_BASE_POS).await;
    let near = [
        CASTLE_BASE_POS[0] + 10.0,
        CASTLE_BASE_POS[1],
        CASTLE_BASE_POS[2],
    ];
    insert_castle_character(
        pool,
        CHALLENGER_ACCOUNT,
        CHALLENGER_PLAYER,
        "SsuChallenger",
        near,
    )
    .await;
}

async fn cleanup(pool: &sqlx::PgPool) {
    for pid in [BOT_PLAYER, CHALLENGER_PLAYER] {
        let _ = sqlx::query("DELETE FROM sgw_player WHERE player_id = $1")
            .bind(pid)
            .execute(pool)
            .await;
    }
    for aid in [BOT_ACCOUNT, CHALLENGER_ACCOUNT] {
        let _ = sqlx::query("DELETE FROM account WHERE account_id = $1")
            .bind(aid)
            .execute(pool)
            .await;
    }
}

/// The challenger's side: wait `delay`, challenge the bot, then collect
/// every line sent to the challenger until `TEXT_DUEL_ACCEPTED` or
/// `deadline`.
async fn challenge_and_listen(
    challenger: &GameSession,
    delay: Duration,
    deadline: Duration,
) -> Vec<String> {
    tokio::time::sleep(delay).await;
    challenger
        .send_bundle(&sparbot::send_duel_challenge(BOT_NAME, false), true)
        .await
        .expect("send sendDuelChallenge");
    let own = challenger.player_entity_id.unwrap();
    let start = tokio::time::Instant::now();
    let mut lines = Vec::new();
    while start.elapsed() < deadline {
        for b in challenger.recv_bundles(1, Duration::from_millis(200)).await {
            for msg in decode_bundle(&b) {
                if let Some(Incoming::Line { text, .. }) = classify(&msg, own) {
                    let done = text == TEXT_DUEL_ACCEPTED;
                    lines.push(text);
                    if done {
                        return lines;
                    }
                }
            }
        }
        // The challenger's own keep-alive and acks.
        let _ = challenger
            .send_bundle(&GameSession::authenticate(), false)
            .await;
    }
    lines
}

/// Server and bot logs on the test writer (`RUST_LOG` overrides), so a
/// `--nocapture` run shows the duel rows and the forfeit stub line.
fn init_logs() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "sparbot=info,duel=debug".to_string()),
        )
        .with_test_writer()
        .try_init();
}

fn assert_accepted(report: &SparbotReport, challenger_lines: &[String]) {
    assert_eq!(
        report.challenges_accepted, 1,
        "the bot saw one onDuelChallenge: {report:?}"
    );
    assert!(
        challenger_lines.iter().any(|l| l == TEXT_DUEL_ACCEPTED),
        "the challenger was told the duel was accepted: {challenger_lines:?}"
    );
    assert!(
        report.lines.iter().any(|l| l == TEXT_DUEL_ACCEPTED),
        "the bot was told too: {report:?}"
    );
}

#[tokio::test]
async fn sparbot_accepts_a_duel_challenge() {
    init_logs();
    let pool = match live_db_pool_or_skip().await {
        Some(p) => p,
        None => return,
    };
    let server = start_server(&std::env::var("DATABASE_URL").unwrap()).await;
    cleanup(&pool).await;
    seed(&pool).await;

    let bot_session = support::enter_castle(
        &server.auth_url,
        &credentials_for("ssu2_sparbot"),
        BOT_PLAYER,
        41,
    )
    .await;
    let challenger = support::enter_castle(
        &server.auth_url,
        &credentials_for("ssu2_challenger"),
        CHALLENGER_PLAYER,
        42,
    )
    .await;

    // Forfeit one second after the accept, inside SS-D1's five-second
    // countdown, so the forfeit is sent while the duel is still on.
    let mut bot = Sparbot::new(
        bot_session.player_entity_id.unwrap(),
        SparbotConfig {
            forfeit_after: Some(Duration::from_secs(1)),
        },
    );
    let ((), lines) = tokio::join!(
        sparbot::run(&bot_session, &mut bot, Some(Duration::from_secs(4))),
        challenge_and_listen(
            &challenger,
            Duration::from_millis(800),
            Duration::from_secs(3)
        ),
    );

    assert_accepted(&bot.report, &lines);
    assert_eq!(bot.report.forfeits_sent, 1, "{:?}", bot.report);
    assert_eq!(bot.report.stopped, Some(StopReason::RunTimeElapsed));

    for s in [&bot_session, &challenger] {
        let _ = s.send_bundle(&GameSession::disconnect(0), true).await;
    }
    cleanup(&pool).await;
    server.orchestrator.stop_all().await;
}

/// Longer than the server's 60 s client-silence reap
/// (`base-session` `tick_sync.rs`).
const PAST_THE_REAP: Duration = Duration::from_secs(70);

#[tokio::test]
#[ignore = "runs for over a minute; run with --ignored to re-prove the keep-alive"]
async fn sparbot_session_outlives_the_inactivity_reap() {
    init_logs();
    let pool = match live_db_pool_or_skip().await {
        Some(p) => p,
        None => return,
    };
    let server = start_server(&std::env::var("DATABASE_URL").unwrap()).await;
    cleanup(&pool).await;
    seed(&pool).await;

    let bot_session = support::enter_castle(
        &server.auth_url,
        &credentials_for("ssu2_sparbot"),
        BOT_PLAYER,
        43,
    )
    .await;
    let challenger = support::enter_castle(
        &server.auth_url,
        &credentials_for("ssu2_challenger"),
        CHALLENGER_PLAYER,
        44,
    )
    .await;

    let mut bot = Sparbot::new(
        bot_session.player_entity_id.unwrap(),
        SparbotConfig {
            forfeit_after: None,
        },
    );
    // The challenger stays quiet except for its own keep-alive until the
    // reap time has passed, then challenges.
    let challenger_side = async {
        let start = tokio::time::Instant::now();
        while start.elapsed() < PAST_THE_REAP {
            let _ = challenger
                .send_bundle(&GameSession::authenticate(), false)
                .await;
            challenger
                .recv_bundles(64, Duration::from_millis(250))
                .await;
        }
        challenge_and_listen(&challenger, Duration::ZERO, Duration::from_secs(5)).await
    };
    let ((), lines) = tokio::join!(
        sparbot::run(
            &bot_session,
            &mut bot,
            Some(PAST_THE_REAP + Duration::from_secs(6))
        ),
        challenger_side,
    );

    assert_eq!(bot.report.stopped, Some(StopReason::RunTimeElapsed));
    assert_accepted(&bot.report, &lines);

    for s in [&bot_session, &challenger] {
        let _ = s.send_bundle(&GameSession::disconnect(0), true).await;
    }
    cleanup(&pool).await;
    server.orchestrator.stop_all().await;
}
