//! Recipient resolution for a gate-mail send: D-SS13 name matching against
//! `sgw_player`, and the Ignore seam (D-SS15).

use std::collections::HashSet;

use sqlx::PgConnection;

/// Why one recipient did not get the mail.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum FailReason {
    /// No character has this name, exactly or case-folded.
    Unknown,
    /// No exact match, and more than one character matches case-folded.
    Ambiguous,
    /// The recipient already holds `MAILBOX_CAP` open messages (D-SS03).
    MailboxFull,
    /// The recipient ignores the sender (D-SS15).
    Ignoring,
}

impl FailReason {
    /// Stable `reason` log value.
    pub(super) fn reason(self) -> &'static str {
        match self {
            FailReason::Unknown => "unknown_recipient",
            FailReason::Ambiguous => "ambiguous_recipient",
            FailReason::MailboxFull => "mailbox_full",
            FailReason::Ignoring => "recipient_ignoring_sender",
        }
    }
}

/// One recipient that did not get the mail: the name as the sender typed
/// it (echoed back in `FailedRecipients`), and the character it resolved
/// to, if any.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct FailedRecipient {
    pub(super) typed: String,
    pub(super) player_id: Option<i32>,
    pub(super) reason: FailReason,
}

/// What one typed name resolved to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Resolution {
    Found { player_id: i32 },
    Failed(FailReason),
}

/// Resolve every typed name against candidate `(player_id, player_name)`
/// rows (D-SS13): an exact match first; failing that, a case-insensitive
/// match only if exactly one character has it. `sgw_player.player_name` is
/// `UNIQUE` but case-sensitive, so "Bob" and "bob" can both exist, and an
/// ambiguous fold is refused rather than guessed.
pub(super) fn resolve_names(typed: &[String], rows: &[(i32, String)]) -> Vec<Resolution> {
    typed
        .iter()
        .map(|name| {
            if let Some((id, _)) = rows.iter().find(|(_, n)| n == name) {
                return Resolution::Found { player_id: *id };
            }
            let folded = name.to_lowercase();
            let mut hits = rows.iter().filter(|(_, n)| n.to_lowercase() == folded);
            match (hits.next(), hits.next()) {
                (None, _) => Resolution::Failed(FailReason::Unknown),
                (Some((id, _)), None) => Resolution::Found { player_id: *id },
                (Some(_), Some(_)) => Resolution::Failed(FailReason::Ambiguous),
            }
        })
        .collect()
}

/// Every `sgw_player` row that could match one of `typed`, exactly or
/// case-folded. At most `MAX_MAIL_RECIPIENTS` names reach here, so this is
/// one small query.
pub(super) async fn candidate_rows(
    conn: &mut PgConnection,
    typed: &[String],
) -> Result<Vec<(i32, String)>, sqlx::Error> {
    let folded: Vec<String> = typed.iter().map(|n| n.to_lowercase()).collect();
    sqlx::query_as::<_, (i32, String)>(
        "SELECT player_id, player_name FROM sgw_player \
         WHERE player_name = ANY($1) OR lower(player_name) = ANY($2)",
    )
    .bind(typed)
    .bind(&folded)
    .fetch_all(conn)
    .await
}

/// The recipients among `recipient_ids` who ignore `sender_id` (D-SS15:
/// mail to someone who ignores you is refused, one way).
///
/// A seam until SS-C1 lands the Ignore list: it answers "nobody", and the
/// send path already reports [`FailReason::Ignoring`] for anyone it returns.
pub(super) async fn ignoring_sender(
    _conn: &mut PgConnection,
    _sender_id: i32,
    _recipient_ids: &[i32],
) -> Result<HashSet<i32>, sqlx::Error> {
    // TODO(SS-C1): read the recipients' Ignore lists (contact-list flags 301).
    Ok(HashSet::new())
}
