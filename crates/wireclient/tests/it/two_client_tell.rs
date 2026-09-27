//! Two real wire clients exchange a tell (SS-C1, TESTING.md type 11).
//!
//! Both characters enter Castle through the real auth, Mercury handshake
//! and world-entry sequence against a spawned `Orchestrator`. Then:
//!
//! 1. A sends `sendPlayerCommunication(10, "<B>", text)` (0xC2 on the tell
//!    channel). B's client receives `onPlayerCommunication` (28) spoken by A
//!    on channel 10, and A's receives `onTellSent` (30) naming B.
//! 2. B answers; the same holds the other way round.
//! 3. A tells a name nobody is playing and gets "Player X is not online."
//!    on the feedback channel.
//!
//! This covers the wire path the base-dispatch fan-out tests
//! (`cimmeria-base` `dispatch::tests::tell`) cannot: real encode/decode and
//! the online index as world entry really fills it. Like the other modules
//! here it is live-DB only and is not run in CI (audit A-60):
//!
//! ```text
//! bash tools/build-lane/reload-db.sh    # prints DATABASE_URL
//! DATABASE_URL=... cargo test -p cimmeria-wireclient --test it two_client_tell -- --test-threads=1
//! ```

use std::time::Duration;

use crate::support::{
    credentials_for, enter_castle, insert_castle_character, insert_sentinel_account,
    live_db_pool_or_skip, start_server, wait_for, CASTLE_BASE_POS,
};

use cimmeria_wireclient::bundle::S2CMessage;
use cimmeria_wireclient::session::GameSession;

const TELL_CHANNEL: u8 = 10;
const FEEDBACK_CHANNEL: u8 = 9;
const SEND_PLAYER_COMMUNICATION: u8 = 0xC2;
const ON_PLAYER_COMMUNICATION: u16 = 28;
const ON_TELL_SENT: u16 = 30;

fn wstr(buf: &mut Vec<u8>, s: &str) {
    let units: Vec<u16> = s.encode_utf16().collect();
    buf.extend_from_slice(&(units.len() as u32).to_le_bytes());
    for u in units {
        buf.extend_from_slice(&u.to_le_bytes());
    }
}

fn read_wstr(b: &[u8], o: &mut usize) -> String {
    let n = u32::from_le_bytes(b[*o..*o + 4].try_into().unwrap()) as usize;
    *o += 4;
    let units: Vec<u16> = (0..n)
        .map(|i| u16::from_le_bytes([b[*o + i * 2], b[*o + i * 2 + 1]]))
        .collect();
    *o += n * 2;
    String::from_utf16(&units).unwrap()
}

/// `onPlayerCommunication` args (after the 4-byte entity id) as
/// `(speaker, channel, text)`.
fn player_comm(m: &S2CMessage) -> (String, u8, String) {
    let args = &m.payload[4..];
    let mut o = 0;
    let speaker = read_wstr(args, &mut o);
    let channel = args[o + 1];
    o += 2;
    (speaker, channel, read_wstr(args, &mut o))
}

/// `onTellSent` args as `(target, text)`.
fn tell_sent(m: &S2CMessage) -> (String, String) {
    let args = &m.payload[4..];
    let mut o = 0;
    let target = read_wstr(args, &mut o);
    (target, read_wstr(args, &mut o))
}

async fn tell(from: &GameSession, target: &str, text: &str) {
    let mut args = vec![TELL_CHANNEL];
    wstr(&mut args, target);
    wstr(&mut args, text);
    from.send_bundle(
        &GameSession::base_method(SEND_PLAYER_COMMUNICATION, &args),
        true,
    )
    .await
    .expect("send sendPlayerCommunication");
}

#[tokio::test]
async fn two_clients_exchange_a_tell() {
    let pool = match live_db_pool_or_skip().await {
        Some(p) => p,
        None => return,
    };
    let server = start_server(&std::env::var("DATABASE_URL").unwrap()).await;

    const ACCOUNT_A: i32 = 900_311;
    const ACCOUNT_B: i32 = 900_312;
    const PLAYER_A: i32 = 900_313;
    const PLAYER_B: i32 = 900_314;
    const NAME_A: &str = "SsC1TellA";
    const NAME_B: &str = "SsC1TellB";
    let cleanup = || async {
        for pid in [PLAYER_A, PLAYER_B] {
            let _ = sqlx::query("DELETE FROM sgw_player WHERE player_id = $1")
                .bind(pid)
                .execute(&pool)
                .await;
        }
        for acc in [ACCOUNT_A, ACCOUNT_B] {
            let _ = sqlx::query("DELETE FROM account WHERE account_id = $1")
                .bind(acc)
                .execute(&pool)
                .await;
        }
    };
    cleanup().await;
    insert_sentinel_account(&pool, ACCOUNT_A, "ssc1_tell_a").await;
    insert_sentinel_account(&pool, ACCOUNT_B, "ssc1_tell_b").await;
    insert_castle_character(&pool, ACCOUNT_A, PLAYER_A, NAME_A, CASTLE_BASE_POS).await;
    let pos_b = [
        CASTLE_BASE_POS[0] + 20.0,
        CASTLE_BASE_POS[1],
        CASTLE_BASE_POS[2],
    ];
    insert_castle_character(&pool, ACCOUNT_B, PLAYER_B, NAME_B, pos_b).await;

    let a = enter_castle(
        &server.auth_url,
        &credentials_for("ssc1_tell_a"),
        PLAYER_A,
        1,
    )
    .await;
    let b = enter_castle(
        &server.auth_url,
        &credentials_for("ssc1_tell_b"),
        PLAYER_B,
        2,
    )
    .await;
    let (a_id, b_id) = (a.player_entity_id.unwrap(), b.player_entity_id.unwrap());

    for (from, from_id, from_name, to, to_id, to_name, text) in [
        (&a, a_id, NAME_A, &b, b_id, NAME_B, "psst, B"),
        (&b, b_id, NAME_B, &a, a_id, NAME_A, "hi A"),
    ] {
        // The recipient's name is typed in lower case: D-SS13 folds it.
        tell(from, &to_name.to_lowercase(), text).await;

        let got = wait_for(to, Duration::from_secs(5), |m| {
            m.method_index == Some(ON_PLAYER_COMMUNICATION)
                && m.entity_id == Some(to_id)
                && player_comm(m).1 == TELL_CHANNEL
        })
        .await
        .unwrap_or_else(|| panic!("{to_name} never received the tell from {from_name}"));
        assert_eq!(
            player_comm(&got),
            (from_name.to_string(), TELL_CHANNEL, text.to_string())
        );

        let sent = wait_for(from, Duration::from_secs(5), |m| {
            m.method_index == Some(ON_TELL_SENT) && m.entity_id == Some(from_id)
        })
        .await
        .unwrap_or_else(|| panic!("{from_name} never received onTellSent"));
        assert_eq!(tell_sent(&sent), (to_name.to_string(), text.to_string()));
    }

    tell(&a, "SsC1NobodyHome", "anyone?").await;
    let fb = wait_for(&a, Duration::from_secs(5), |m| {
        m.method_index == Some(ON_PLAYER_COMMUNICATION) && player_comm(m).1 == FEEDBACK_CHANNEL
    })
    .await
    .expect("a tell to an offline name answers with a feedback line");
    assert_eq!(player_comm(&fb).2, "Player SsC1NobodyHome is not online.");

    for s in [&a, &b] {
        let _ = s.send_bundle(&GameSession::disconnect(0), true).await;
    }
    server.orchestrator.stop_all().await;
    cleanup().await;
}
