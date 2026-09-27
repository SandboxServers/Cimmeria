//! Two real wire clients trade an item by COD gate mail, end to end
//! (social-systems SS-M3, TESTING.md type 11).
//!
//! A mails B an item with a 300-naquadah COD. B opens the mailbox, pays the
//! COD and takes the item. A opens the mailbox and takes the payment. Every
//! step is a real client method over the real Mercury UDP session against a
//! spawned `Orchestrator` (auth + base + cell), and every answer is read off
//! the wire: `sendMailResult`, `onMailHeaderInfo`, `onCashChanged`. The
//! database is read only to confirm where the item and the money ended up.
//!
//! Live-DB only, and like the other two-client modules not wired into CI
//! (see `docs/architecture/wireclient.md`), so it never replaces the
//! CI-run guards in `cimmeria-base-methods`' `mail::tests`. Run locally:
//! ```text
//! bash tools/build-lane/reload-db.sh        # prints DATABASE_URL
//! DATABASE_URL=... bash tools/build-lane/lane.sh \
//!   cargo test -p cimmeria-wireclient --test it two_client_mail_cod -- --test-threads=1
//! ```

use std::time::Duration;

use sqlx::PgPool;

use crate::support::{
    self, credentials_for, insert_castle_character, insert_sentinel_account, live_db_pool_or_skip,
    start_server, wait_for, CASTLE_BASE_POS,
};

use cimmeria_wireclient::bundle::S2CMessage;
use cimmeria_wireclient::session::GameSession;

// Sentinels (social-systems block 0x7300_1Bxx).
const ACCOUNT_A: i32 = 0x7300_1B01;
const ACCOUNT_B: i32 = 0x7300_1B02;
const PLAYER_A: i32 = 0x7300_1B11;
const PLAYER_B: i32 = 0x7300_1B12;
const ITEM: i32 = 0x7300_1B21;
const NAME_B: &str = "SsmThreeWireB";

// SGWMailManager cell methods (`SGWMailManager.def`, cell indices 43-51).
const CM_REQUEST_MAIL_HEADERS: u16 = 43;
const CM_SEND_MAIL_MESSAGE: u16 = 44;
const CM_TAKE_CASH: u16 = 49;
const CM_TAKE_ITEM: u16 = 50;
const CM_PAY_COD: u16 = 51;
// Client methods (`crates/wire/src/mercury/mod.rs` `method_idx`).
const ON_CASH_CHANGED: u16 = 75;
const ON_MAIL_HEADER_INFO: u16 = 76;
const SEND_MAIL_RESULT: u16 = 79;
// `EMailFlags::MAIL_COD` (`enumerations.xml`).
const MAIL_COD: i32 = 2;

async fn cleanup(pool: &PgPool) {
    for player in [PLAYER_A, PLAYER_B] {
        let _ = sqlx::query("DELETE FROM sgw_gate_mail WHERE character_id = $1")
            .bind(player)
            .execute(pool)
            .await;
        let _ = sqlx::query("DELETE FROM sgw_inventory WHERE character_id = $1")
            .bind(player)
            .execute(pool)
            .await;
        let _ = sqlx::query("DELETE FROM sgw_player WHERE player_id = $1")
            .bind(player)
            .execute(pool)
            .await;
    }
    for account in [ACCOUNT_A, ACCOUNT_B] {
        let _ = sqlx::query("DELETE FROM account WHERE account_id = $1")
            .bind(account)
            .execute(pool)
            .await;
    }
}

fn wstring(out: &mut Vec<u8>, s: &str) {
    let units: Vec<u16> = s.encode_utf16().collect();
    out.extend_from_slice(&(units.len() as u32).to_le_bytes());
    for u in units {
        out.extend_from_slice(&u.to_le_bytes());
    }
}

/// The client method's arguments: past the entity id, and past the
/// sub-index byte for the extended (`0xBD`) encoding.
fn args(m: &S2CMessage) -> &[u8] {
    if m.msg_id == 0xBD {
        &m.payload[5..]
    } else {
        &m.payload[4..]
    }
}

fn i32_at(b: &[u8], at: usize) -> i32 {
    i32::from_le_bytes(b[at..at + 4].try_into().unwrap())
}

/// Send one cell method to the session's own player entity.
async fn call(session: &GameSession, method: u16, a: &[u8]) {
    let own = session.player_entity_id.expect("in world");
    session
        .send_bundle(&GameSession::cell_method(method, own, a), true)
        .await
        .expect("send cell method");
}

/// Wait for client method `method` on the session's own entity whose
/// arguments satisfy `pred`. Earlier replies of the same method (the
/// postage `onCashChanged`, the header refresh after the payment) are
/// skipped rather than mistaken for the one this step waits for.
async fn expect(
    session: &GameSession,
    method: u16,
    what: &str,
    pred: impl Fn(&[u8]) -> bool,
) -> S2CMessage {
    let own = session.player_entity_id.unwrap();
    wait_for(session, Duration::from_secs(10), |m| {
        m.method_index == Some(method) && m.entity_id == Some(own) && pred(args(m))
    })
    .await
    .unwrap_or_else(|| panic!("never received {what} (method {method})"))
}

/// An `onMailHeaderInfo`'s headers as `(id, cash, flags)`, and its
/// attachment count.
fn headers(a: &[u8]) -> (Vec<(i32, i32, i32)>, u32) {
    let u32_at = |at: usize| u32::from_le_bytes(a[at..at + 4].try_into().unwrap());
    // ResetCategory, bArchive, then the header array.
    let mut at = 2;
    let n = u32_at(at);
    at += 4;
    let mut out = Vec::new();
    for _ in 0..n {
        let id = i32_at(a, at);
        at += 4;
        at += 4 + 2 * u32_at(at) as usize; // fromText
        at += 4; // fromId
        at += 4 + 2 * u32_at(at) as usize; // subjectText
        at += 4; // subjectId
        let cash = i32_at(a, at);
        at += 12; // cash, sentTime, readTime
        out.push((id, cash, i32_at(a, at)));
        at += 4;
    }
    (out, u32_at(at))
}

/// A sends B a COD item; B pays and takes it; A takes the payment.
#[tokio::test]
async fn cod_item_round_trip_between_two_clients() {
    let pool = match live_db_pool_or_skip().await {
        Some(p) => p,
        None => return,
    };
    cleanup(&pool).await;
    let server = start_server(&std::env::var("DATABASE_URL").unwrap()).await;

    insert_sentinel_account(&pool, ACCOUNT_A, "ssm3_mail_a").await;
    insert_sentinel_account(&pool, ACCOUNT_B, "ssm3_mail_b").await;
    insert_castle_character(&pool, ACCOUNT_A, PLAYER_A, "SsmThreeWireA", CASTLE_BASE_POS).await;
    insert_castle_character(&pool, ACCOUNT_B, PLAYER_B, NAME_B, CASTLE_BASE_POS).await;
    for player in [PLAYER_A, PLAYER_B] {
        sqlx::query("UPDATE sgw_player SET naquadah = 1000 WHERE player_id = $1")
            .bind(player)
            .execute(&pool)
            .await
            .unwrap();
    }
    let type_id: i32 =
        sqlx::query_scalar("SELECT item_id FROM resources.items ORDER BY item_id LIMIT 1")
            .fetch_one(&pool)
            .await
            .unwrap();
    sqlx::query(
        "INSERT INTO sgw_inventory \
            (item_id, character_id, type_id, stack_size, container_id, slot_id) \
         VALUES ($1, $2, $3, 1, 1, 0)",
    )
    .bind(ITEM)
    .bind(PLAYER_A)
    .bind(type_id)
    .execute(&pool)
    .await
    .unwrap();

    let a = support::enter_castle(
        &server.auth_url,
        &credentials_for("ssm3_mail_a"),
        PLAYER_A,
        1,
    )
    .await;
    let b = support::enter_castle(
        &server.auth_url,
        &credentials_for("ssm3_mail_b"),
        PLAYER_B,
        2,
    )
    .await;

    // A: sendMailMessage(RecipientFlags, [B], Subject, Body, Cash 300,
    // bCOD 1, ItemId, ItemQuantity 1).
    let mut send = 0i32.to_le_bytes().to_vec();
    send.extend_from_slice(&1u32.to_le_bytes());
    wstring(&mut send, NAME_B);
    wstring(&mut send, "Sword for sale");
    wstring(&mut send, "Pay on delivery.");
    send.extend_from_slice(&300i32.to_le_bytes());
    send.push(1);
    send.extend_from_slice(&ITEM.to_le_bytes());
    send.extend_from_slice(&1i32.to_le_bytes());
    call(&a, CM_SEND_MAIL_MESSAGE, &send).await;
    let result = expect(&a, SEND_MAIL_RESULT, "sendMailResult", |_| true).await;
    assert_eq!(args(&result)[0], 0, "MAILRESULT_Sent");

    // B: the COD mail is in the inbox, flagged COD at 300.
    call(&b, CM_REQUEST_MAIL_HEADERS, &[0]).await;
    let inbox = expect(&b, ON_MAIL_HEADER_INFO, "B's inbox", |a| {
        !headers(a).0.is_empty()
    })
    .await;
    let (rows, attachments) = headers(args(&inbox));
    let (mail_id, cash, flags) = rows[0];
    assert_eq!((cash, flags & MAIL_COD, attachments), (300, MAIL_COD, 1));

    // B pays: 1000 - 300.
    call(&b, CM_PAY_COD, &mail_id.to_le_bytes()).await;
    expect(&b, ON_CASH_CHANGED, "B's onCashChanged 700", |a| {
        i32_at(a, 0) == 700
    })
    .await;

    // B takes the item. The client's ContainerId/SlotId are garbage on the
    // shipped client (SS-E1 M-Q5); send garbage.
    let mut take = mail_id.to_le_bytes().to_vec();
    take.extend_from_slice(&0x5A5A_5A5Ai32.to_le_bytes());
    take.extend_from_slice(&(-7i32).to_le_bytes());
    call(&b, CM_TAKE_ITEM, &take).await;
    // The header comes back with no COD, no cash and, once the item is
    // taken, no attachment.
    expect(
        &b,
        ON_MAIL_HEADER_INFO,
        "B's header without the item",
        |a| headers(a) == (vec![(mail_id, 0, 0)], 0),
    )
    .await;
    let owner: (i32, i32) =
        sqlx::query_as("SELECT character_id, container_id FROM sgw_inventory WHERE item_id = $1")
            .bind(ITEM)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(owner, (PLAYER_B, 1), "the item is in B's backpack");

    // A: the payment mail, then take its cash: 1000 - 25 postage + 300.
    call(&a, CM_REQUEST_MAIL_HEADERS, &[0]).await;
    let inbox = expect(&a, ON_MAIL_HEADER_INFO, "A's inbox with the payment", |a| {
        headers(a).0.iter().any(|&(_, cash, _)| cash == 300)
    })
    .await;
    let payment_id = headers(args(&inbox))
        .0
        .into_iter()
        .find(|&(_, cash, _)| cash == 300)
        .unwrap()
        .0;
    call(&a, CM_TAKE_CASH, &payment_id.to_le_bytes()).await;
    expect(&a, ON_CASH_CHANGED, "A's onCashChanged 1275", |a| {
        i32_at(a, 0) == 1_275
    })
    .await;

    for s in [&a, &b] {
        let _ = s.send_bundle(&GameSession::disconnect(0), true).await;
    }
    server.orchestrator.stop_all().await;
    cleanup(&pool).await;
}
