//! `send_system_mail` row → [`Action::SendSystemMail`] conversion (SS-U3).
//!
//! The loader is the boundary: a value the mail writer would refuse on every
//! firing drops the row here, with a warn naming the chain, instead of
//! reaching a player as a refusal. No database.

use super::super::action::convert_action;
use super::super::*;
use crate::actions::Action;

fn mail_row(params: serde_json::Value) -> DbActionRow {
    DbActionRow {
        chain_id: 7011,
        action_type: "send_system_mail".to_string(),
        target_id: None,
        target_key: None,
        params,
        delay_ms: 0,
        sort_order: 0,
    }
}

/// The Gate Mail Clerk's row, as seeded.
#[test]
fn convert_send_system_mail_full_row() {
    let action = convert_action(&mail_row(serde_json::json!({
        "sender": "Gate Mail Clerk",
        "subject": "Test mail",
        "body": "Take the naquadah and the slappacks.",
        "cash": 50,
        "item_id": 2893,
        "qty": 5,
        "cooldown_secs": 600
    })))
    .expect("a fully specified send_system_mail row must convert");
    assert_eq!(
        action,
        Action::SendSystemMail {
            sender_name: "Gate Mail Clerk".into(),
            subject: "Test mail".into(),
            body: "Take the naquadah and the slappacks.".into(),
            cash: 50,
            item: Some((2893, 5)),
            cooldown_secs: Some(600),
            quiet_cooldown: false,
        }
    );
}

/// Only `sender` and `subject` are required: no body, no cash, no item and
/// no cooldown is a plain text mail on every firing; `qty` defaults to 1.
#[test]
fn convert_send_system_mail_defaults() {
    let action = convert_action(&mail_row(serde_json::json!({
        "sender": "  Gate Mail Clerk ",
        "subject": "Hello"
    })))
    .expect("a minimal row must convert");
    assert_eq!(
        action,
        Action::SendSystemMail {
            sender_name: "Gate Mail Clerk".into(),
            subject: "Hello".into(),
            body: String::new(),
            cash: 0,
            item: None,
            cooldown_secs: None,
            quiet_cooldown: false,
        }
    );
    let with_item = convert_action(&mail_row(serde_json::json!({
        "sender": "Clerk", "subject": "Hello", "item_id": 2893
    })))
    .expect("an item without qty must convert");
    assert!(matches!(
        with_item,
        Action::SendSystemMail {
            item: Some((2893, 1)),
            ..
        }
    ));
}

/// `quiet_cooldown` (Dakara DK-01): `true` with a window is carried through;
/// an explicit `false` is the default, with or without a window.
#[test]
fn convert_send_system_mail_quiet_cooldown() {
    let quiet = convert_action(&mail_row(serde_json::json!({
        "sender": "Dakara Gate Watch", "subject": "Hello",
        "cooldown_secs": 2_147_483_647_i64, "quiet_cooldown": true
    })))
    .expect("a quiet row with a window must convert");
    assert_eq!(
        quiet,
        Action::SendSystemMail {
            sender_name: "Dakara Gate Watch".into(),
            subject: "Hello".into(),
            body: String::new(),
            cash: 0,
            item: None,
            cooldown_secs: Some(2_147_483_647),
            quiet_cooldown: true,
        }
    );
    for params in [
        serde_json::json!({"sender": "c", "subject": "s", "cooldown_secs": 60, "quiet_cooldown": false}),
        serde_json::json!({"sender": "c", "subject": "s", "quiet_cooldown": false}),
    ] {
        assert!(
            matches!(
                convert_action(&mail_row(params.clone())),
                Some(Action::SendSystemMail {
                    quiet_cooldown: false,
                    ..
                })
            ),
            "row {params} must convert as not quiet"
        );
    }
}

/// Every rejected shape drops the row.
#[test]
fn convert_send_system_mail_rejects_bad_params() {
    let long = "x".repeat(129);
    let long_body = "x".repeat(1001);
    let cases = [
        serde_json::json!({"subject": "s"}),
        serde_json::json!({"sender": "", "subject": "s"}),
        serde_json::json!({"sender": "c"}),
        serde_json::json!({"sender": "c", "subject": long}),
        serde_json::json!({"sender": "two\nlines", "subject": "s"}),
        serde_json::json!({"sender": "c", "subject": "s", "body": long_body}),
        serde_json::json!({"sender": "c", "subject": "s", "body": 5}),
        serde_json::json!({"sender": "c", "subject": "s", "cash": -1}),
        serde_json::json!({"sender": "c", "subject": "s", "cash": 2_147_483_648_i64}),
        serde_json::json!({"sender": "c", "subject": "s", "item_id": "2893"}),
        serde_json::json!({"sender": "c", "subject": "s", "item_id": 2893, "qty": 0}),
        serde_json::json!({"sender": "c", "subject": "s", "qty": 5}),
        serde_json::json!({"sender": "c", "subject": "s", "cooldown_secs": 0}),
        serde_json::json!({"sender": "c", "subject": "s", "cooldown_secs": -5}),
        // `quiet_cooldown`: a boolean, and `true` only with a window.
        serde_json::json!({"sender": "c", "subject": "s", "quiet_cooldown": true}),
        serde_json::json!({"sender": "c", "subject": "s", "cooldown_secs": 60, "quiet_cooldown": 1}),
        serde_json::json!({"sender": "c", "subject": "s", "cooldown_secs": 60, "quiet_cooldown": "true"}),
        serde_json::json!({"sender": "c", "subject": "s", "cooldown_secs": 60, "quiet_cooldown": null}),
    ];
    for params in cases {
        assert_eq!(
            convert_action(&mail_row(params.clone())),
            None,
            "row {params} must be dropped"
        );
    }
}
