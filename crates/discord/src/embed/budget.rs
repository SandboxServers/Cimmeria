//! Truncation and the Discord 6000-char total-budget enforcement.

use serde_json::Value;

use super::{ELLIPSIS, MAX_TOTAL};

/// Truncate a string to at most `max` characters (Unicode-aware count).
/// Appends an ellipsis if truncated. `max` MUST be ≥ 1 (the ellipsis
/// itself); callers pass Discord's per-field caps which are all ≥ 256.
pub(super) fn truncate(s: &str, max: usize) -> String {
    debug_assert!(max >= ELLIPSIS.chars().count());
    let count = s.chars().count();
    if count <= max {
        return s.to_string();
    }
    let keep = max - ELLIPSIS.chars().count();
    let mut out: String = s.chars().take(keep).collect();
    out.push_str(ELLIPSIS);
    out
}

/// Final pass: enforce Discord's 6000-character total budget. Trims
/// `description` first, then `fields[].value` back-to-front (least
/// important last → trimmed first), looping until the total fits or
/// nothing further can be reclaimed. Title and footer are kept full
/// because they're already individually capped at 256 and 2048.
///
/// **Postcondition**: on return, `total_chars(embed) <= MAX_TOTAL`
/// *unless* the immutable parts (title + footer + field names) by
/// themselves already exceed the budget — a configuration error that
/// would need to be diagnosed by the caller. Discord would then 400
/// the request and the failure counter would increment.
pub(super) fn enforce_total_budget(embed: &mut Value) {
    // Helper: trim one string field down by up to `over` chars,
    // returning the number of chars actually reclaimed. Inserts an
    // ellipsis so the visible total is `target_len = cur - over`
    // when `cur > over + ell`; otherwise reclaims as much as
    // possible while leaving room for the ellipsis.
    fn trim_value_in_place(slot: &mut Value, over: usize) -> usize {
        let Some(s) = slot.as_str().map(str::to_string) else {
            return 0;
        };
        let cur = s.chars().count();
        let ell = ELLIPSIS.chars().count();
        if cur <= ell + 1 {
            return 0; // can't usefully shrink
        }
        let keep = if cur > over + ell {
            cur - over - ell
        } else {
            // Reclaim everything except a 1-char prefix + ellipsis.
            1
        };
        let trimmed: String = s.chars().take(keep).collect::<String>() + ELLIPSIS;
        let new_len = trimmed.chars().count();
        *slot = Value::String(trimmed);
        cur.saturating_sub(new_len)
    }

    // Loop guard: each iteration must make progress, otherwise we'd
    // spin forever in the pathological "title + footer alone already
    // exceed budget" case. Bail out if a pass reclaims zero chars.
    loop {
        let total = total_chars(embed);
        if total <= MAX_TOTAL {
            return;
        }
        let mut over = total - MAX_TOTAL;
        let mut reclaimed_this_pass = 0;

        // Description first — usually the longest single string.
        if let Some(desc_slot) = embed.get_mut("description") {
            let r = trim_value_in_place(desc_slot, over);
            reclaimed_this_pass += r;
            over = over.saturating_sub(r);
        }

        // Then fields back-to-front so the last-added (least important)
        // field shrinks before earlier ones.
        if over > 0 {
            if let Some(fields) = embed.get_mut("fields").and_then(Value::as_array_mut) {
                for f in fields.iter_mut().rev() {
                    if over == 0 {
                        break;
                    }
                    if let Some(val_slot) = f.get_mut("value") {
                        let r = trim_value_in_place(val_slot, over);
                        reclaimed_this_pass += r;
                        over = over.saturating_sub(r);
                    }
                }
            }
        }

        if reclaimed_this_pass == 0 {
            // No progress possible — title+footer+field-names alone
            // exceed the budget. Discord will reject, the failure
            // counter will increment, and the operator will see it.
            return;
        }
    }
}

pub(super) fn total_chars(embed: &Value) -> usize {
    let mut total = 0;
    if let Some(s) = embed.get("title").and_then(Value::as_str) {
        total += s.chars().count();
    }
    if let Some(s) = embed.get("description").and_then(Value::as_str) {
        total += s.chars().count();
    }
    if let Some(fields) = embed.get("fields").and_then(Value::as_array) {
        for f in fields {
            if let Some(s) = f.get("name").and_then(Value::as_str) {
                total += s.chars().count();
            }
            if let Some(s) = f.get("value").and_then(Value::as_str) {
                total += s.chars().count();
            }
        }
    }
    if let Some(s) = embed
        .get("footer")
        .and_then(|x| x.get("text"))
        .and_then(Value::as_str)
    {
        total += s.chars().count();
    }
    total
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::builder::build_embed;
    use super::super::{MAX_DESC, MAX_TITLE};
    use crate::event::{Event, TracingEventKind};
    use chrono::Utc;

    fn now() -> chrono::DateTime<Utc> {
        Utc::now()
    }

    /// Title truncation. A 500-char title fits Discord's 256 cap with an
    /// ellipsis.
    #[test]
    fn long_title_truncates_with_ellipsis() {
        let long = "x".repeat(500);
        let truncated = truncate(&long, MAX_TITLE);
        assert_eq!(truncated.chars().count(), MAX_TITLE);
        assert!(truncated.ends_with(ELLIPSIS));
    }

    /// Description truncation.
    #[test]
    fn long_description_truncates_with_ellipsis() {
        let long = "x".repeat(8000);
        let truncated = truncate(&long, MAX_DESC);
        assert_eq!(truncated.chars().count(), MAX_DESC);
        assert!(truncated.ends_with(ELLIPSIS));
    }

    /// Unicode boundaries — truncation must not split a code point.
    #[test]
    fn truncation_respects_unicode_boundaries() {
        let s = "😀".repeat(300);
        let t = truncate(&s, 100);
        assert_eq!(t.chars().count(), 100);
        // Confirm valid UTF-8 by re-decoding: panicking here means we
        // split a multi-byte char.
        let _ = t.chars().collect::<Vec<_>>();
    }

    /// Unicode-safe budget enforcement: trimming a multi-byte UTF-8
    /// description must not split a code point and must keep the
    /// resulting total under MAX_TOTAL.
    ///
    /// Reverting `enforce_total_budget` to the pre-fix "subtract `over`
    /// chars then append ellipsis" formulation trips this immediately
    /// because that path could exit with `over > 0` unchanged.
    #[test]
    fn total_budget_enforced_with_multibyte_utf8() {
        // Each "😀" is 1 char but 4 bytes — sized to exceed MAX_TOTAL
        // strictly in character count.
        let big = "😀".repeat(MAX_DESC + 500);
        let event = Event::TracingEvent {
            kind: TracingEventKind::Error,
            target: "unicode-test".into(),
            message: big,
            fields: (0..5)
                .map(|i| (format!("k{}", i), "🎯".repeat(300)))
                .collect(),
            timestamp: now(),
        };
        let embed = build_embed(&event);
        let actual = total_chars(&embed);
        assert!(
            actual <= MAX_TOTAL,
            "multibyte content must respect the 6000-char budget, got {}",
            actual
        );
        // Reserialise to verify the JSON is still valid UTF-8 (any
        // mid-code-point split would corrupt it).
        let serialized = serde_json::to_string(&embed).expect("embed must round-trip as JSON");
        // Sanity: the ellipsis sentinel must show up somewhere (proof
        // that at least one trim happened).
        assert!(
            serialized.contains(ELLIPSIS),
            "expected at least one truncation marker"
        );
    }

    /// Multi-field trim path: description alone isn't enough to bring
    /// the embed under budget, so the enforcer must walk the fields
    /// back-to-front and trim multiple of them. The pre-fix code only
    /// trimmed one field and could exit over-budget.
    #[test]
    fn total_budget_enforced_when_description_alone_insufficient() {
        let event = Event::TracingEvent {
            kind: TracingEventKind::Error,
            target: "x".into(),
            // Short description.
            message: "short".into(),
            // Many bulky fields → must trim more than one to fit.
            fields: (0..20)
                .map(|i| (format!("k{}", i), "z".repeat(900)))
                .collect(),
            timestamp: now(),
        };
        let embed = build_embed(&event);
        let actual = total_chars(&embed);
        assert!(
            actual <= MAX_TOTAL,
            "multi-field trim must reach ≤ {} chars, got {}",
            MAX_TOTAL,
            actual
        );
    }

    /// 6000-char total budget enforcement.
    #[test]
    fn total_budget_enforced_by_description_trim() {
        // Construct an event whose description, after building, exceeds
        // 6000 chars combined. The TracingEvent variant is the natural
        // path: message takes the desc slot, fields add the rest.
        let event = Event::TracingEvent {
            kind: TracingEventKind::Error,
            target: "x".repeat(200),
            message: "y".repeat(MAX_DESC),
            fields: (0..20)
                .map(|i| (format!("k{}", i), "z".repeat(200)))
                .collect(),
            timestamp: now(),
        };
        let mut embed = build_embed(&event);
        // Pre-budget: pretend the description+fields blew past 6000.
        // Validate the actual budget enforcer keeps us under.
        let actual = total_chars(&embed);
        assert!(
            actual <= MAX_TOTAL,
            "expected ≤ {} chars after enforcement, got {}",
            MAX_TOTAL,
            actual
        );
        // Sanity that we didn't trim it to zero
        assert!(actual > 1000);

        // Also force a second pass to confirm idempotence.
        let before = embed.clone();
        enforce_total_budget(&mut embed);
        assert_eq!(before, embed, "enforce_total_budget must be idempotent");
    }
}
