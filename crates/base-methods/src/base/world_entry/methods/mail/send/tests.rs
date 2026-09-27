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
