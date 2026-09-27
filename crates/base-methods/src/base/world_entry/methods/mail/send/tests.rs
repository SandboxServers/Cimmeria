//! Unit tests for the send path's pure pieces: D-SS13 name resolution, the
//! D-SS07 alias seam and the feedback text.

use super::recipients::{resolve_names, FailReason, FailedRecipient, Resolution};
use super::*;

fn rows(names: &[(i32, &str)]) -> Vec<(i32, String)> {
    names.iter().map(|(id, n)| (*id, n.to_string())).collect()
}

fn typed(names: &[&str]) -> Vec<String> {
    names.iter().map(|n| n.to_string()).collect()
}

#[test]
fn exact_match_wins_over_a_case_fold() {
    let db = rows(&[(1, "Bob"), (2, "bob")]);
    assert_eq!(
        resolve_names(&typed(&["bob", "Bob"]), &db),
        vec![
            Resolution::Found { player_id: 2 },
            Resolution::Found { player_id: 1 }
        ]
    );
}

#[test]
fn unique_fold_resolves_and_ambiguous_fold_is_refused() {
    let db = rows(&[(1, "Solo"), (2, "Twin"), (3, "tWIN")]);
    assert_eq!(
        resolve_names(&typed(&["SOLO", "TWIN", "Ghost"]), &db),
        vec![
            Resolution::Found { player_id: 1 },
            Resolution::Failed(FailReason::Ambiguous),
            Resolution::Failed(FailReason::Unknown),
        ]
    );
}

#[test]
fn recipient_flags_seam_refuses_every_alias() {
    assert_eq!(resolve_recipient_flags(0), Ok(()));
    let vault = resolve_recipient_flags(flags::MAIL_TO_VAULT).unwrap_err();
    assert_eq!(
        (vault.failed_flags, vault.reason),
        (flags::MAIL_TO_VAULT, "vault_alias_unsupported")
    );
    let org = resolve_recipient_flags(flags::MAIL_TO_COMMAND | flags::MAIL_TO_TEAM).unwrap_err();
    assert_eq!(
        (org.failed_flags, org.reason),
        (
            flags::MAIL_TO_COMMAND | flags::MAIL_TO_TEAM,
            "organization_alias_unsupported"
        )
    );
    // MAIL_Archive and MAIL_COD are header flags, never aliases.
    for bits in [flags::MAIL_ARCHIVE, flags::MAIL_COD, 1 << 20] {
        let bad = resolve_recipient_flags(bits).unwrap_err();
        assert_eq!(
            (bad.failed_flags, bad.reason),
            (bits, "unknown_recipient_flags")
        );
    }
}

fn attached(cash: i32, cod: bool, item_id: i32, item_quantity: i32) -> MailSend {
    MailSend {
        recipient_flags: 0,
        recipients: typed(&["Bob"]),
        subject: "S".into(),
        body: String::new(),
        cash,
        cod,
        item_id,
        item_quantity,
    }
}

/// CAT-G-01 / D-SS09: the attachment checks that need no database. Each
/// row is a refusal the shipped client would never send.
#[test]
fn attachment_validation_refuses_malformed_attachments() {
    let cases = [
        (
            attached(-1, false, 0, 0),
            MailResult::NoRecipients,
            "negative_cash",
        ),
        (
            attached(-1, true, 10_500, 1),
            MailResult::NoRecipients,
            "negative_cash",
        ),
        (
            attached(0, false, 0, 3),
            MailResult::NoRecipients,
            "item_quantity_without_item",
        ),
        (
            attached(0, false, 10_500, 0),
            MailResult::ItemNotAvailable,
            "invalid_item_quantity",
        ),
        (
            attached(0, false, 10_500, -2),
            MailResult::ItemNotAvailable,
            "invalid_item_quantity",
        ),
        (
            attached(50, true, 0, 0),
            MailResult::ItemNotAvailable,
            "cod_without_item",
        ),
        (
            attached(0, true, 10_500, 1),
            MailResult::NoRecipients,
            "cod_without_price",
        ),
    ];
    for (send, result, reason) in cases {
        let refusal = attachment::validate(&send).unwrap_err();
        assert_eq!(
            (refusal.result, refusal.reason),
            (result, reason),
            "{send:?}"
        );
        assert!(refusal.text.ends_with("not sent."), "{}", refusal.text);
    }
}

/// D-SS02 / D-SS09: what the sender pays now. Postage on every attachment,
/// the gift cash on top, a COD price never (the recipient pays it).
#[test]
fn attachment_cost_is_postage_plus_gift_cash() {
    use attachment::{Attachment, ItemRequest, POSTAGE};
    assert_eq!(attachment::validate(&attached(0, false, 0, 0)), Ok(None));

    let gift = attachment::validate(&attached(300, false, 0, 0))
        .unwrap()
        .unwrap();
    assert_eq!(gift.sender_cost(), 325);
    assert_eq!(gift.mail_flags(), 0);

    let item = attachment::validate(&attached(0, false, 10_500, 2))
        .unwrap()
        .unwrap();
    assert_eq!(
        item,
        Attachment {
            cash: 0,
            cod: false,
            item: Some(ItemRequest {
                item_id: 10_500,
                quantity: 2
            }),
        }
    );
    assert_eq!(item.sender_cost(), POSTAGE);

    let cod = attachment::validate(&attached(900, true, 10_500, 1))
        .unwrap()
        .unwrap();
    assert_eq!(
        cod.sender_cost(),
        POSTAGE,
        "a COD price is not debited from the sender"
    );
    assert_eq!(cod.mail_flags(), flags::MAIL_COD);

    // i32::MAX gift plus postage does not wrap.
    let max = attachment::validate(&attached(i32::MAX, false, 0, 0))
        .unwrap()
        .unwrap();
    assert_eq!(max.sender_cost(), i64::from(i32::MAX) + 25);
}

#[test]
fn distinct_names_folds_case() {
    assert_eq!(distinct_names(&typed(&["Bob", "bob", "BOB"])), 1);
    assert_eq!(distinct_names(&typed(&["Bob", "Al"])), 2);
    assert_eq!(distinct_names(&[]), 0);
}

#[test]
fn failure_line_names_each_recipient_and_reason() {
    assert_eq!(failure_line(&[]), None);
    let failed = [
        FailedRecipient {
            typed: "Ghost".into(),
            player_id: None,
            reason: FailReason::Unknown,
        },
        FailedRecipient {
            typed: "Full".into(),
            player_id: Some(7),
            reason: FailReason::MailboxFull,
        },
    ];
    assert_eq!(
        failure_line(&failed).unwrap(),
        "Gate-mail not delivered to: Ghost (no such character), Full (gate-mail box is full)."
    );
}

/// A recipient who ignores the sender gets the shared D-SS15 sentence, after
/// the list of the others.
#[test]
fn failure_line_uses_the_shared_not_accepting_sentence() {
    let failed = [
        FailedRecipient {
            typed: "Ghost".into(),
            player_id: None,
            reason: FailReason::Unknown,
        },
        FailedRecipient {
            typed: "Grumpy".into(),
            player_id: Some(9),
            reason: FailReason::Ignoring,
        },
    ];
    assert_eq!(
        failure_line(&failed).unwrap(),
        "Gate-mail not delivered to: Ghost (no such character). Grumpy is not accepting your messages."
    );
}
