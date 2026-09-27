//! The player-facing lines for send refusals and partial failures.

use cimmeria_entity::organization::{TextField, TextReject};

use super::recipients::{FailReason, FailedRecipient};
use crate::cell::messages::MailSendReject;

/// The feedback line for a refusal the cell's decode produced.
pub(super) fn decode_refusal_text(reject: &MailSendReject) -> &'static str {
    match reject {
        MailSendReject::TooManyRecipients { .. } => {
            "A gate-mail message can have at most 10 recipients. It was not sent."
        }
        MailSendReject::Malformed { .. } => {
            "Your gate-mail message could not be read. It was not sent."
        }
        MailSendReject::Text { field, reject } => match (field, reject) {
            (TextField::MailSubject, TextReject::TooShort { .. }) => {
                "Your gate-mail message needs a subject. It was not sent."
            }
            (TextField::MailSubject, TextReject::TooLong { .. }) => {
                "Your gate-mail subject is too long (128 characters at most). \
                 It was not sent."
            }
            (TextField::MailBody, TextReject::TooLong { .. }) => {
                "Your gate-mail message is too long (1,000 characters at most). \
                 It was not sent."
            }
            (TextField::MailRecipient, _) => {
                "A recipient name is not a valid character name. The message was not sent."
            }
            _ => {
                "Your gate-mail message contains a character that cannot be sent. \
                 It was not sent."
            }
        },
    }
}

/// One line naming every recipient that did not get the mail, and why. A
/// recipient who ignores the sender gets the shared D-SS15 sentence ("X is
/// not accepting your messages."), the same words a tell or a duel
/// challenge gets, after the list of the others.
pub(super) fn failure_line(failed: &[FailedRecipient]) -> Option<String> {
    if failed.is_empty() {
        return None;
    }
    let (ignoring, others): (Vec<_>, Vec<_>) = failed
        .iter()
        .partition(|f| f.reason == FailReason::Ignoring);
    let mut sentences: Vec<String> = Vec::new();
    if !others.is_empty() {
        let parts: Vec<String> = others
            .iter()
            .map(|f| format!("{} ({})", f.typed, f.reason.player_text()))
            .collect();
        sentences.push(format!("Gate-mail not delivered to: {}.", parts.join(", ")));
    }
    sentences.extend(
        ignoring
            .iter()
            .map(|f| crate::base::contact_list::ignore::not_accepting_text(&f.typed)),
    );
    Some(sentences.join(" "))
}

impl FailReason {
    pub(super) fn player_text(self) -> &'static str {
        match self {
            FailReason::Unknown => "no such character",
            FailReason::Ambiguous => "more than one character matches; check the capitals",
            FailReason::MailboxFull => "gate-mail box is full",
            FailReason::Ignoring => "not accepting your messages",
        }
    }
}
