//! Regression guards for the `onMailHeaderInfo` `sentTime` fix
//! (owner playtest 2026-09-28: a fresh mail's Read Message window showed
//! "Sent: Wed Dec 31st, 1969 @ 7:0 pm" — Unix epoch 0 in US Eastern — and
//! its inbox row showed "Expires: Soon").
//!
//! Root cause (`docs/reverse-engineering/findings/mail-wire-formats.md`
//! M-Q3): the client's header-record constructor (`FUN_00eb5ab0`,
//! `ghidra://SGW.exe@0x00eb5ab0`) treats the wire `sentTime` float as
//! **seconds elapsed since the mail was sent**, not a Unix epoch
//! timestamp. It rounds the field to a 64-bit integer and subtracts
//! `value * 10_000_000` (100ns FILETIME ticks) from the client's own
//! `GetSystemTime()` (`FUN_00eb5a10`, `ghidra://SGW.exe@0x00eb5a10`) to
//! get the "Sent: <date>" display, and divides the same value by 3600 for
//! `ExpiresHours = 720 - hours` (`ghidra://SGW.exe@0x00eb5c0d`..`0x00eb5c19`).
//! The pre-fix code sent the raw Unix epoch (~1.7-1.8 billion in 2026),
//! which the client read as an almost-56-year-old age: `now - 56yr` lands
//! the date near Unix epoch 0, and `720 - (1.7e9/3600)` underflows to a
//! huge negative `ExpiresHours`, which `GateMail.lua:138`
//! (`ExpiresHours < 2`) renders as "Soon" no matter how fresh the mail is.
//!
//! **Why the existing suite never caught this**: `insert_mail`'s fixture
//! always stamps `sent_time = 0`. Under both the buggy code (raw epoch)
//! and the fix (age), a stored `0` produces the wire value `0`, which
//! looks fresh either way — the two interpretations only diverge for a
//! non-zero `sent_time`, which is what [`request_headers_sends_the_age_not_the_epoch`]
//! stores.

use std::time::Instant;

use super::headers::sent_time_age_secs;
use super::packets::{Client, Received};
use super::*;

const BASE: i32 = 0x7300_2400;

/// Unit: the conversion is `(now - sent_time_unix).max(0)`, not a pass
/// through of the stored epoch value.
#[test]
fn sent_time_age_secs_is_now_minus_sent_time_clamped_at_zero() {
    assert_eq!(sent_time_age_secs(1_700_002_000, 1_700_000_000), 2_000.0);
    // A mail sent this instant: age 0, not the epoch value itself.
    assert_eq!(sent_time_age_secs(1_700_000_000, 1_700_000_000), 0.0);
    // Clock skew (sent_time in the future): clamp, never a negative age.
    assert_eq!(sent_time_age_secs(1_700_000_000, 1_700_000_500), 0.0);
}

/// Byte-exact: a header with a known `sent_time` and a known "now" carries
/// the age (not the epoch) at the `sentTime` byte offset on the wire.
/// Reverting to `sent_time_unix as f32` fails this: 1_700_000_000.0's LE
/// bytes are `00 00 CA D3` (mantissa rounds a value this large to the
/// nearest 128), not `00 00 20 44` (500.0).
#[test]
fn header_wire_bytes_carry_the_age_not_the_epoch() {
    let now = 1_700_000_500;
    let sent_time_unix = 1_700_000_000; // sent 500 s ago
    let age = sent_time_age_secs(now, sent_time_unix);
    assert_eq!(age, 500.0);

    let headers = vec![cimmeria_wire::cell::mail::MailHeader {
        id: 7,
        from_text: "Al".to_string(),
        from_id: 1,
        subject_text: "Hi".to_string(),
        cash: 0,
        sent_time: age,
        read_time: 0.0,
        flags: 0,
    }];
    let args = cimmeria_wire::cell::mail::serialize_on_mail_header_info(false, 0, &headers, &[]);

    // ResetCategory(1) + bArchive(1) + count(4) + id(4) + fromText(4+2+2)
    // + fromId(4) + subjectText(4+2+2) + subjectId(4) + cash(4) = 33, then sentTime.
    let sent_time_offset = 1 + 1 + 4 + 4 + (4 + 4) + 4 + (4 + 4) + 4 + 4;
    let bytes: [u8; 4] = args[sent_time_offset..sent_time_offset + 4]
        .try_into()
        .unwrap();
    assert_eq!(
        f32::from_le_bytes(bytes),
        500.0,
        "sentTime bytes must be the age (500.0), not the epoch value"
    );
    assert_ne!(
        bytes,
        (sent_time_unix as f32).to_le_bytes(),
        "must not be the raw epoch value's bytes"
    );
}

/// The client's own `ExpiresHours = 720 - floor(age_secs / 3600)` formula
/// (`ghidra://SGW.exe@0x00eb5c06`..`0x00eb5c19`, `__aulldiv` by 0xe10):
/// a mail sent "now" (age 0) yields exactly 720 (the client's `Never`
/// branch is a distinct sentinel, not modeled here — `GateMail.lua:137`).
/// A mail 100 hours old yields 620. Both would be wildly different (deep
/// negative, `GateMail.lua`'s "Soon" branch) if `age_secs` were instead
/// the raw Unix epoch.
fn client_expires_hours(age_secs: i64) -> i64 {
    720 - age_secs / 3600
}

#[test]
fn fresh_mail_expires_hours_matches_the_client_formula() {
    let now = 2_000_000_000;
    let age = sent_time_age_secs(now, now) as i64;
    assert_eq!(client_expires_hours(age), 720, "freshly sent mail: 30 days");

    let hundred_hours_ago = now - 100 * 3600;
    let age = sent_time_age_secs(now, hundred_hours_ago) as i64;
    assert_eq!(client_expires_hours(age), 620);
}

/// Live-DB fan-out byte guard: a mail stored with `sent_time` 100 s in the
/// past must arrive over the wire as `sentTime` ~= 100.0, not as the raw
/// stored epoch (~2 billion). Reverting `headers::to_wire`'s conversion
/// back to `r.sent_time as f32` fails this — the decoded value lands in
/// the billions, far outside the assertion window.
#[tokio::test]
async fn request_headers_sends_the_age_not_the_epoch() {
    let pool = require_db_or_skip!();
    let (acct, owner) = (BASE, BASE + 1);
    cleanup(&pool, acct).await;
    insert_players(&pool, acct, &[(owner, "SsmSentAge")]).await;

    let before = unix_now();
    let mail_id = insert_mail(&pool, owner, "aged").await;
    sqlx::query("UPDATE sgw_gate_mail SET sent_time = $2 WHERE mail_id = $1")
        .bind(mail_id)
        .bind(before - 100)
        .execute(&pool)
        .await
        .expect("stamp sent_time 100s in the past");

    let c = Client::new(BASE as u32 + 0x40, owner, 55_140, "SsmSentAge");
    c.op(
        MailOp::RequestHeaders { b_archive: 0 },
        Some(&pool),
        Instant::now(),
    )
    .await;
    match c.take().as_slice() {
        [Received::HeaderInfo {
            headers, sent_time, ..
        }] => {
            assert_eq!(headers, &vec![(mail_id, 0)]);
            assert_eq!(sent_time.len(), 1);
            // ~100s of age, with slack for the wall-clock time this test
            // itself takes; the pre-fix value would be ~2_000_000_000.0.
            assert!(
                (90.0..=130.0).contains(&sent_time[0]),
                "sentTime must be a small age in seconds, got {}",
                sent_time[0]
            );
        }
        other => panic!("expected one onMailHeaderInfo, got {other:?}"),
    }

    cleanup(&pool, acct).await;
}
