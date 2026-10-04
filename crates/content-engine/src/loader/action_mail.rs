//! `send_system_mail` row → [`Action::SendSystemMail`] (SS-U3).
//!
//! Every param is in `params`; `target_id` and `target_key` are unused:
//!
//! ```json
//! {"sender": "Gate Mail Clerk", "subject": "...", "body": "...",
//!  "cash": 50, "item_id": 2893, "qty": 5, "cooldown_secs": 600}
//! ```
//!
//! `sender` and `subject` are required, one line, 1-128 characters; `body`
//! is optional, up to 1,000 characters (the D-SS12 limits the mail writer
//! enforces). `cash` is `0..=i32::MAX`. `item_id` is optional; with it,
//! `qty` defaults to 1 (the writer caps it at the item's stack size).
//! `cooldown_secs`, when present, is at least 1.
//!
//! A bad value drops the row with a `warn!` naming the chain, rather than
//! defaulting: the base would refuse the same mail on every firing, and the
//! player would see a refusal for an authoring mistake.

use tracing::warn;

use crate::actions::Action;

use super::DbActionRow;

/// `sgw_gate_mail.sender_name` and `subject` are `varchar(128)`.
const MAX_LINE_CHARS: usize = 128;
/// D-SS12's body limit.
const MAX_BODY_CHARS: usize = 1000;

/// Convert one `send_system_mail` `content_actions` row.
pub(super) fn convert_send_system_mail(row: &DbActionRow) -> Option<Action> {
    let params = &row.params;
    let drop_row = |why: &str| {
        warn!(
            chain_id = row.chain_id,
            chain_name = cimmeria_names::book().chain(row.chain_id),
            ?params,
            "send_system_mail: {why}; dropping the action row"
        );
        None
    };

    let line = |key: &str| -> Result<String, String> {
        let text = params
            .get(key)
            .and_then(|v| v.as_str())
            .map(str::trim)
            .unwrap_or("");
        if text.is_empty() {
            Err(format!("`{key}` is missing or empty"))
        } else if text.chars().count() > MAX_LINE_CHARS || text.contains(['\n', '\r']) {
            Err(format!(
                "`{key}` must be one line of at most {MAX_LINE_CHARS} characters"
            ))
        } else {
            Ok(text.to_string())
        }
    };
    let sender_name = match line("sender") {
        Ok(s) => s,
        Err(why) => return drop_row(&why),
    };
    let subject = match line("subject") {
        Ok(s) => s,
        Err(why) => return drop_row(&why),
    };
    let body = match params.get("body") {
        None => String::new(),
        Some(v) => match v.as_str() {
            Some(b) if b.chars().count() <= MAX_BODY_CHARS => b.to_string(),
            _ => return drop_row("`body` must be a string of at most 1000 characters"),
        },
    };

    let cash = match params.get("cash") {
        None => 0,
        Some(v) => match v.as_i64() {
            Some(c) if (0..=i64::from(i32::MAX)).contains(&c) => c,
            _ => return drop_row("`cash` must be an integer 0 to 2147483647"),
        },
    };

    let item = match params.get("item_id") {
        None => None,
        Some(v) => {
            let Some(type_id) = v.as_i64().and_then(|t| i32::try_from(t).ok()) else {
                return drop_row("`item_id` must be an integer resources.items id");
            };
            let qty = match params.get("qty") {
                None => 1,
                Some(q) => match q.as_i64().and_then(|q| i32::try_from(q).ok()) {
                    Some(q) if q >= 1 => q,
                    _ => return drop_row("`qty` must be an integer of at least 1"),
                },
            };
            Some((type_id, qty))
        }
    };
    if item.is_none() && params.get("qty").is_some() {
        return drop_row("`qty` without `item_id`");
    }

    let cooldown_secs = match params.get("cooldown_secs") {
        None => None,
        Some(v) => match v.as_u64().and_then(|s| u32::try_from(s).ok()) {
            Some(s) if s >= 1 => Some(s),
            _ => return drop_row("`cooldown_secs` must be an integer of at least 1"),
        },
    };

    Some(Action::SendSystemMail {
        sender_name,
        subject,
        body,
        cash,
        item,
        cooldown_secs,
    })
}
