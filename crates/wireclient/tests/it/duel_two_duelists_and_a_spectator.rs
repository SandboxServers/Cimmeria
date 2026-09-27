//! A duel over the real wire: two duelists and a spectator (SS-D2, TESTING.md
//! type 11).
//!
//! Three real `cimmeria-wireclient` sessions enter Castle within a few units
//! of each other, against a spawned `Orchestrator`. A challenges B by name
//! (base method 0xD9), B accepts (cell method 102), and the test checks what
//! each client receives on the wire:
//!
//! - B: `onDuelChallenge` [143] naming A's entity;
//! - A and B: the countdown, `onTimerUpdate` with `Type = DuelTimer (14)` on
//!   their own entity, then, when it runs out, `onDuelEntitiesSet` [151]
//!   with exactly the two duelists' entity ids;
//! - C, the spectator: `onEntityProperty(GENERICPROPERTY_PvPFlag = 4, 1)` on
//!   A's and on B's entity, and never a 151 of its own.
//!
//! Then B forfeits (cell method 103, SS-D3): both duelists get
//! `onDuelEntitiesClear` [153], the spectator sees both flags go back to 0,
//! and A hears "You won the duel" (879).
//!
//! Like the other modules here it needs a live database and does not run in
//! CI (audit A-60), so it backs, and never replaces, the in-process guards in
//! `crates/cell-world/src/cell/duel/tests/engage.rs`. Run it with
//! `bash tools/build-lane/live-db-test.sh` for the database, then
//! `cargo test -p cimmeria-wireclient --test it duel_two_duelists -- --test-threads=1`
//! with that `DATABASE_URL`.

use std::collections::HashSet;
use std::time::Duration;

use crate::support::{
    self, assert_never, credentials_for, insert_castle_character, insert_sentinel_account,
    live_db_pool_or_skip, start_server, wait_for, CASTLE_BASE_POS,
};

use cimmeria_wireclient::session::GameSession;

const ACCOUNTS: [i32; 3] = [900_601, 900_602, 900_603];
const PLAYERS: [i32; 3] = [900_611, 900_612, 900_613];
const NAMES: [&str; 3] = ["SSD2Alpha", "SSD2Bravo", "SSD2Watch"];
const LOGINS: [&str; 3] = ["ssd2_alpha", "ssd2_bravo", "ssd2_watch"];

/// `sendDuelChallenge(WSTRING aPlayerName, INT8 aSquadDuel)`: a `u32` UTF-16
/// unit count, the units, then the squad byte (0: a one-on-one duel).
fn challenge_args(name: &str) -> Vec<u8> {
    let units: Vec<u16> = name.encode_utf16().collect();
    let mut out = (units.len() as u32).to_le_bytes().to_vec();
    for u in units {
        out.extend_from_slice(&u.to_le_bytes());
    }
    out.push(0);
    out
}

/// The args of a direct-encoded entity method (`[entity_id:4][args]`).
fn direct_args(payload: &[u8]) -> &[u8] {
    &payload[4..]
}

/// The args of an extended-encoded one (`[entity_id:4][sub_index:1][args]`).
fn extended_args(payload: &[u8]) -> &[u8] {
    &payload[5..]
}

#[tokio::test]
async fn duel_engages_for_two_duelists_and_flags_both_for_a_spectator() {
    let pool = match live_db_pool_or_skip().await {
        Some(p) => p,
        None => return,
    };
    let server = start_server(&std::env::var("DATABASE_URL").unwrap()).await;
    for i in 0..3 {
        insert_sentinel_account(&pool, ACCOUNTS[i], LOGINS[i]).await;
        let pos = [
            CASTLE_BASE_POS[0] + 3.0 * i as f32,
            CASTLE_BASE_POS[1],
            CASTLE_BASE_POS[2],
        ];
        insert_castle_character(&pool, ACCOUNTS[i], PLAYERS[i], NAMES[i], pos).await;
    }

    let mut sessions = Vec::new();
    for i in 0..3 {
        let creds = credentials_for(LOGINS[i]);
        sessions
            .push(support::enter_castle(&server.auth_url, &creds, PLAYERS[i], 10 + i as u32).await);
    }
    let (a, b, c) = (&sessions[0], &sessions[1], &sessions[2]);
    let a_id = a.player_entity_id.unwrap();
    let b_id = b.player_entity_id.unwrap();

    // The spectator has both duelists in view before the duel starts.
    let mut seen = HashSet::new();
    wait_for(c, Duration::from_secs(10), |m| {
        if m.is_create() && (m.entity_id == Some(a_id) || m.entity_id == Some(b_id)) {
            seen.insert(m.entity_id.unwrap());
        }
        seen.len() == 2
    })
    .await
    .expect("the spectator never saw both duelists created");

    // A challenges B by name; B gets the prompt naming A.
    a.send_bundle(
        &GameSession::base_method(0xD9, &challenge_args(NAMES[1])),
        true,
    )
    .await
    .expect("send sendDuelChallenge");
    let prompt = wait_for(b, Duration::from_secs(5), |m| m.method_index == Some(143))
        .await
        .expect("B never received onDuelChallenge [143]");
    assert_eq!(
        &extended_args(&prompt.payload)[..4],
        &(a_id as i32).to_le_bytes(),
        "the prompt names the challenger's entity"
    );

    // B accepts; both duelists get the DuelTimer countdown.
    b.send_bundle(&GameSession::cell_method(102, b_id, &[1]), true)
        .await
        .expect("send sendDuelResponse");
    for (s, id) in [(a, a_id), (b, b_id)] {
        let timer = wait_for(s, Duration::from_secs(5), |m| {
            m.method_index == Some(12)
                && m.entity_id == Some(id)
                && direct_args(&m.payload).get(4) == Some(&14)
        })
        .await
        .unwrap_or_else(|| panic!("{id} never received the DuelTimer countdown"));
        assert_eq!(direct_args(&timer.payload).len(), 21);
    }

    // The countdown runs out: both get onDuelEntitiesSet with the pair.
    let mut pair = Vec::new();
    pair.extend_from_slice(&2u32.to_le_bytes());
    pair.extend_from_slice(&(a_id as i32).to_le_bytes());
    pair.extend_from_slice(&(b_id as i32).to_le_bytes());
    for (s, id) in [(a, a_id), (b, b_id)] {
        let set = wait_for(s, Duration::from_secs(10), |m| m.method_index == Some(151))
            .await
            .unwrap_or_else(|| panic!("{id} never received onDuelEntitiesSet [151]"));
        assert_eq!(extended_args(&set.payload), &pair[..], "151 to {id}");
    }

    // The spectator sees both duelists flagged.
    let flag_on: [u8; 8] = [4, 0, 0, 0, 1, 0, 0, 0];
    let mut flagged = HashSet::new();
    wait_for(c, Duration::from_secs(10), |m| {
        if m.method_index == Some(7) && direct_args(&m.payload) == flag_on {
            flagged.extend(m.entity_id);
        }
        flagged.contains(&a_id) && flagged.contains(&b_id)
    })
    .await
    .unwrap_or_else(|| panic!("the spectator saw flags only for {flagged:?}"));
    assert_never(
        c,
        Duration::from_secs(1),
        |m| m.method_index == Some(151),
        "the spectator is not a duelist",
    )
    .await;

    // B forfeits; A wins. One stateful wait per client: `wait_for` drops
    // the rest of a bundle once its predicate matches, and 153 and the
    // result line can share one.
    b.send_bundle(&GameSession::cell_method(103, b_id, &[]), true)
        .await
        .expect("send duelForfeit");
    let won: Vec<u8> = "You won the duel"
        .encode_utf16()
        .flat_map(u16::to_le_bytes)
        .collect();
    for (s, id) in [(a, a_id), (b, b_id)] {
        let (mut clear, mut line) = (false, id != a_id);
        wait_for(s, Duration::from_secs(5), |m| {
            clear |= m.method_index == Some(153);
            line |= m.method_index == Some(28)
                && m.payload.windows(won.len()).any(|w| w == won.as_slice());
            clear && line
        })
        .await
        .unwrap_or_else(|| panic!("{id}: 153 seen {clear}, the winner's 879 line seen {line}"));
    }
    let flag_off: [u8; 8] = [4, 0, 0, 0, 0, 0, 0, 0];
    let mut cleared = HashSet::new();
    wait_for(c, Duration::from_secs(5), |m| {
        if m.method_index == Some(7) && direct_args(&m.payload) == flag_off {
            cleared.extend(m.entity_id);
        }
        cleared.contains(&a_id) && cleared.contains(&b_id)
    })
    .await
    .unwrap_or_else(|| panic!("the spectator saw flags cleared only for {cleared:?}"));

    for s in &sessions {
        let _ = s.send_bundle(&GameSession::disconnect(0), true).await;
    }
    for i in 0..3 {
        let _ = sqlx::query("DELETE FROM sgw_player WHERE player_id = $1")
            .bind(PLAYERS[i])
            .execute(&pool)
            .await;
        let _ = sqlx::query("DELETE FROM account WHERE account_id = $1")
            .bind(ACCOUNTS[i])
            .execute(&pool)
            .await;
    }
    server.orchestrator.stop_all().await;
}
