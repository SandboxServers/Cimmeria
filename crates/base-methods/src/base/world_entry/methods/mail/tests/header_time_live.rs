//! Live-DB guard for the header `sentTime` the client gets (Black Market
//! live run, 2026-09-29): the database stores epoch seconds, the wire
//! carries the mail's age. A Black Market payout mail read back through
//! `read_one` (the new-mail push; the header list shares its `to_wire`)
//! must say "just now", not 56 years ago. Sentinels `0x7300_54xx`.

use super::super::headers::read_one;
use super::*;
use crate::base::world_entry::methods::mail::system::{send_system_mail, SystemItem, SystemMail};

const DAY: i32 = 86_400;

fn epoch_now() -> i32 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i32
}

#[tokio::test]
async fn live_db_mail_header_sent_time_is_the_age_not_the_epoch() {
    let pool = require_db_or_skip!();
    let (acct, rcpt) = (0x7300_5400, 0x7300_5401);
    cleanup(&pool, acct).await;
    insert_players(&pool, acct, &[(rcpt, "SentAgeRcpt")]).await;

    // The mail that showed "Thu Jan 1st, 1970": a Black Market payout.
    let sent = send_system_mail(
        &pool,
        &SystemMail {
            sender_name: "Black Market".into(),
            recipient_player_id: rcpt,
            subject: "Auction Won".into(),
            body: "You won a Black Market auction.".into(),
            cash: 0,
            item: SystemItem::None,
        },
    )
    .await
    .expect("system mail");
    let (headers, _) = read_one(&pool, rcpt, sent.mail_id).await.unwrap();
    let header = headers.first().expect("the new mail's header");
    assert!(
        (0.0..=60.0).contains(&header.sent_time),
        "a mail sent just now has an age of seconds, got sentTime {} \
         (an epoch value makes the client show 1970 and expire it Soon)",
        header.sent_time
    );

    // A three-day-old mail (the player send path stores the same epoch
    // column): the client shows 27 days left only if the age is 3 days.
    let old_id: i32 = sqlx::query_scalar(
        "INSERT INTO sgw_gate_mail \
            (character_id, sender_id, sender_name, subject, message, cash, \
             sent_time, read_time, flags) \
         VALUES ($1, NULL, 'Someone', 'Old', 'Old mail', 0, $2, 0, 0) RETURNING mail_id",
    )
    .bind(rcpt)
    .bind(epoch_now() - 3 * DAY)
    .fetch_one(&pool)
    .await
    .unwrap();
    let (headers, _) = read_one(&pool, rcpt, old_id).await.unwrap();
    let age = headers.first().expect("the old mail's header").sent_time;
    assert!(
        (f64::from(age) - f64::from(3 * DAY)).abs() <= 60.0,
        "a three-day-old mail has an age of 259200 s, got sentTime {age}"
    );

    cleanup(&pool, acct).await;
}
