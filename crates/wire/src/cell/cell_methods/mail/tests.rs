//! `sendMailMessage` (CM 44) decoder tests: byte-built payloads.

use super::*;
use crate::cell::messages::{MailSend, MailSendReject};
use crate::mercury::write_wstring;
use cimmeria_entity::organization::{TextField, TextReject};

/// Build a CM 44 payload in `.def` order.
fn payload(flags: i32, names: &[&str], subject: &str, body: &str, tail: [i32; 4]) -> Vec<u8> {
    let mut p = Vec::new();
    p.extend_from_slice(&flags.to_le_bytes());
    p.extend_from_slice(&(names.len() as u32).to_le_bytes());
    for n in names {
        write_wstring(&mut p, n);
    }
    write_wstring(&mut p, subject);
    write_wstring(&mut p, body);
    let [cash, cod, item_id, qty] = tail;
    p.extend_from_slice(&cash.to_le_bytes());
    p.push(cod as u8);
    p.extend_from_slice(&item_id.to_le_bytes());
    p.extend_from_slice(&qty.to_le_bytes());
    p
}

#[test]
fn decodes_every_field_in_def_order() {
    let p = payload(
        0,
        &["Bob", "Al"],
        "Hi",
        "line 1\nline 2",
        [25, 1, 10_042, 3],
    );
    assert_eq!(
        decode_send_mail_message(&p),
        Ok(MailSend {
            recipient_flags: 0,
            recipients: vec!["Bob".into(), "Al".into()],
            subject: "Hi".into(),
            body: "line 1\nline 2".into(),
            cash: 25,
            cod: true,
            item_id: 10_042,
            item_quantity: 3,
        })
    );
}

/// D-SS05: ten names decode; eleven are refused from the count alone.
#[test]
fn recipient_cap_is_ten() {
    let ten: Vec<String> = (0..10).map(|i| format!("n{i}")).collect();
    let ten: Vec<&str> = ten.iter().map(String::as_str).collect();
    let ok = decode_send_mail_message(&payload(0, &ten, "s", "", [0; 4])).unwrap();
    assert_eq!(ok.recipients.len(), 10);

    let eleven: Vec<String> = (0..11).map(|i| format!("n{i}")).collect();
    let eleven: Vec<&str> = eleven.iter().map(String::as_str).collect();
    assert_eq!(
        decode_send_mail_message(&payload(0, &eleven, "s", "", [0; 4])),
        Err(MailSendReject::TooManyRecipients { declared: 11 })
    );
}

/// A forged count is refused before allocation: the payload holds no names
/// at all, so reaching the name loop would report `truncated` instead.
#[test]
fn forged_recipient_count_is_refused_before_reading_names() {
    let mut p = 0i32.to_le_bytes().to_vec();
    p.extend_from_slice(&u32::MAX.to_le_bytes());
    assert_eq!(
        decode_send_mail_message(&p),
        Err(MailSendReject::TooManyRecipients { declared: u32::MAX })
    );
}

#[test]
fn forged_wstring_length_is_malformed_not_an_allocation() {
    let mut p = 0i32.to_le_bytes().to_vec();
    p.extend_from_slice(&1u32.to_le_bytes());
    p.extend_from_slice(&0x7FFF_FFFFu32.to_le_bytes()); // name length
    assert_eq!(
        decode_send_mail_message(&p),
        Err(MailSendReject::Malformed {
            reason: "truncated"
        })
    );
}

#[test]
fn truncated_and_trailing_payloads_are_malformed() {
    let full = payload(0, &["Bob"], "s", "b", [0; 4]);
    assert_eq!(
        decode_send_mail_message(&full[..full.len() - 1]),
        Err(MailSendReject::Malformed {
            reason: "truncated"
        })
    );
    let mut long = full.clone();
    long.push(0);
    assert_eq!(
        decode_send_mail_message(&long),
        Err(MailSendReject::Malformed {
            reason: "trailing_bytes"
        })
    );
}

/// D-SS12: subject 1-128, body up to 1,000 (newline allowed), recipients
/// one line; refused, never truncated.
#[test]
fn text_rules_apply_to_every_string() {
    let cases: [(Vec<u8>, TextField, TextReject); 5] = [
        (
            payload(0, &["Bob"], "", "", [0; 4]),
            TextField::MailSubject,
            TextReject::TooShort { units: 0, min: 1 },
        ),
        (
            payload(0, &["Bob"], &"s".repeat(129), "", [0; 4]),
            TextField::MailSubject,
            TextReject::TooLong {
                units: 129,
                max: 128,
            },
        ),
        (
            payload(0, &["Bob"], "s", &"b".repeat(1_001), [0; 4]),
            TextField::MailBody,
            TextReject::TooLong {
                units: 1_001,
                max: 1_000,
            },
        ),
        (
            payload(0, &["Bob"], "s\nx", "", [0; 4]),
            TextField::MailSubject,
            TextReject::Control('\n'),
        ),
        (
            payload(0, &["B\u{202E}ob"], "s", "", [0; 4]),
            TextField::MailRecipient,
            TextReject::Bidi('\u{202E}'),
        ),
    ];
    for (p, field, reject) in cases {
        assert_eq!(
            decode_send_mail_message(&p),
            Err(MailSendReject::Text { field, reject }),
            "{field:?}"
        );
    }
    // The body alone may span lines.
    assert!(decode_send_mail_message(&payload(0, &["Bob"], "s", "a\nb", [0; 4])).is_ok());
}

#[test]
fn lone_surrogate_is_a_text_rejection_on_its_field() {
    let mut p = 0i32.to_le_bytes().to_vec();
    p.extend_from_slice(&1u32.to_le_bytes());
    write_wstring(&mut p, "Bob");
    // Subject: one unpaired high surrogate.
    p.extend_from_slice(&1u32.to_le_bytes());
    p.extend_from_slice(&0xD800u16.to_le_bytes());
    write_wstring(&mut p, "");
    p.extend_from_slice(&[0; 13]);
    assert_eq!(
        decode_send_mail_message(&p),
        Err(MailSendReject::Text {
            field: TextField::MailSubject,
            reject: TextReject::LoneSurrogate
        })
    );
}

#[test]
fn has_attachment_covers_every_attachment_field() {
    let plain = decode_send_mail_message(&payload(0, &["Bob"], "s", "", [0; 4])).unwrap();
    assert!(!plain.has_attachment());
    for tail in [
        [5, 0, 0, 0],
        [-5, 0, 0, 0],
        [0, 1, 0, 0],
        [0, 0, 7, 0],
        [0, 0, 0, 1],
    ] {
        let m = decode_send_mail_message(&payload(0, &["Bob"], "s", "", tail)).unwrap();
        assert!(m.has_attachment(), "{tail:?}");
    }
}
