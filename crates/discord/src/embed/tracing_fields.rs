//! Turn a harvested `warn!`/`error!` event's raw fields into embed
//! fields: fold each Rule 6 ID/name pair into one `Name (#id)` field
//! and pull the trace ID out for the footer.
//!
//! Folding matters for the 25-field cap: an event that pairs every ID
//! (Rule 6) carries twice the fields it used to, and posting them raw
//! pushed the tail past the cap. Folded, each pair costs one slot, and
//! the order (Who, then the objects, then the rest) means a cut only
//! ever drops unpaired fields.

use super::naming::{name_with_id, pair_for, PAIR_EXCEPTIONS, TRACE_KEYS};

/// Embed label for the folded player + account identity.
pub(super) const WHO_LABEL: &str = "Who";

/// Fold `fields` into at most `max` embed fields.
///
/// - **Who** first: `player_id`/`player_name` and
///   `account_id`/`account_name` render as one field,
///   `Name (#player_id) · login (#account_id)`, each half degrading to
///   `#id` (or the bare name) on its own.
/// - **Objects** next: every other ID key folds with its name key into
///   `<label>: Name (#id)`, `#id` when the name is missing, in the
///   event's field order.
/// - **The rest** last, unchanged. When the total is over `max`, the
///   rest is cut from the end and the last slot says how many fields
///   were dropped, so the cut is visible.
///
/// `trace_id`/`span_id` never become fields; [`trace_footer`] renders
/// them.
pub(super) fn fold_fields(fields: &[(String, String)], max: usize) -> Vec<(String, String, bool)> {
    let mut slots: Vec<Option<(&str, &str)>> = fields
        .iter()
        .map(|(k, v)| (!TRACE_KEYS.contains(&k.as_str())).then_some((k.as_str(), v.as_str())))
        .collect();

    // Take the first unconsumed field named `key`.
    fn take<'a>(slots: &mut [Option<(&'a str, &'a str)>], key: &str) -> Option<&'a str> {
        let slot = slots
            .iter_mut()
            .find(|s| matches!(s, Some((k, _)) if *k == key))?;
        slot.take().map(|(_, v)| v)
    }

    let mut who: Option<(String, String, bool)> = None;
    {
        let player_id = take(&mut slots, "player_id");
        // `character_name` is the retired spelling of `player_name`
        // (Rule 6); lines not yet swept still use it.
        let player_name =
            take(&mut slots, "player_name").or_else(|| take(&mut slots, "character_name"));
        let account_id = take(&mut slots, "account_id");
        let account_name = take(&mut slots, "account_name");
        let halves: Vec<String> = [
            name_with_id(player_name, player_id),
            name_with_id(account_name, account_id),
        ]
        .into_iter()
        .flatten()
        .collect();
        if !halves.is_empty() {
            who = Some((WHO_LABEL.to_string(), halves.join(" · "), true));
        }
    }

    // Pull the ID fields out first, then hand out names: exception rows
    // in table order (so `item_type_id` claims `item_name` before
    // `item_id` does), then default-rule IDs in field order. The
    // folded fields still render in field order.
    let mut ids: Vec<(usize, &str, String, String)> = Vec::new();
    for (i, slot) in slots.iter_mut().enumerate() {
        let Some((key, _)) = *slot else { continue };
        if let Some((name_key, label)) = pair_for(key) {
            *slot = None;
            ids.push((i, key, name_key, label));
        }
    }
    let rank = |key: &str| {
        PAIR_EXCEPTIONS
            .iter()
            .position(|r| r.id_key == key)
            .unwrap_or(PAIR_EXCEPTIONS.len())
    };
    let mut claim_order: Vec<usize> = (0..ids.len()).collect();
    claim_order.sort_by_key(|&n| (rank(ids[n].1), ids[n].0));
    let mut names: Vec<Option<&str>> = vec![None; ids.len()];
    for n in claim_order {
        names[n] = take(&mut slots, &ids[n].2);
    }
    let objects: Vec<(String, String, bool)> = ids
        .into_iter()
        .zip(names)
        .filter_map(|((i, _, _, label), name)| {
            let id = fields[i].1.as_str();
            name_with_id(name, Some(id)).map(|v| (label, v, true))
        })
        .collect();

    let rest = slots
        .into_iter()
        .flatten()
        .map(|(k, v)| (k.to_string(), v.to_string(), true));

    let mut out: Vec<(String, String, bool)> = who.into_iter().chain(objects).collect();
    let rest: Vec<_> = rest.collect();
    // Room left for unpaired fields once Who and every pair are placed.
    // The overflow marker takes one of those slots, never a pair's:
    // when the pairs alone fill the embed, the unpaired fields are
    // dropped with no marker, and pairs past the cap (which needs 24+
    // objects on one line) are cut by the cap itself.
    let room = max.saturating_sub(out.len());
    if rest.len() <= room {
        out.extend(rest);
    } else if room > 0 {
        let dropped = rest.len() - (room - 1);
        out.extend(rest.into_iter().take(room - 1));
        out.push(("…".into(), format!("+{dropped} more fields"), true));
    } else {
        out.truncate(max);
    }
    out
}

/// Footer text carrying the trace ID as plain text (D-NT3), or `None`
/// when the event has none. A trace ID that arrived as a link keeps
/// only its hex ID: the link points at SigNoz, which most Discord
/// readers can't reach.
pub(super) fn trace_footer(fields: &[(String, String)]) -> Option<String> {
    let parts: Vec<String> = TRACE_KEYS
        .iter()
        .filter_map(|key| {
            let value = fields.iter().find(|(k, _)| k == key)?.1.as_str();
            Some(format!("{key} {}", plain_trace_id(key, value)?))
        })
        .collect();
    (!parts.is_empty()).then(|| parts.join(" · "))
}

/// The ID for `key` in `value`: the value itself when it is plain.
/// When it is a URL, the W3C-sized hex ID for that key: 32 digits for
/// `trace_id`, 16 for `span_id`. A SigNoz link carries both
/// (`/trace/<32>?spanId=<16>`), so `span_id` reads its query parameter
/// first and never takes the trace's run. `None` when no ID of the
/// right size is there.
fn plain_trace_id(key: &str, value: &str) -> Option<String> {
    let value = value.trim();
    if value.is_empty() {
        return None;
    }
    if !value.contains("://") {
        return Some(value.to_string());
    }
    let is_hex = |s: &str, len: usize| s.len() == len && s.bytes().all(|b| b.is_ascii_hexdigit());
    let len = if key == "span_id" { 16 } else { 32 };
    if key == "span_id" {
        let query = value.split_once('?').map_or("", |(_, q)| q);
        let from_query = query.split(['&', '#']).find_map(|kv| {
            let (k, v) = kv.split_once('=')?;
            (k.eq_ignore_ascii_case("spanid") || k.eq_ignore_ascii_case("span_id"))
                .then_some(v)
                .filter(|v| is_hex(v, len))
        });
        if let Some(v) = from_query {
            return Some(v.to_string());
        }
    }
    value
        .split(|c: char| !c.is_ascii_hexdigit())
        .find(|run| is_hex(run, len))
        .map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::embed::build_embed;
    use crate::event::{Event, TracingEventKind};
    use serde_json::Value;

    fn tracing_event(pairs: &[(&str, &str)]) -> Event {
        Event::TracingEvent {
            kind: TracingEventKind::Warn,
            target: "abilities".into(),
            message: "ability rejected".into(),
            fields: f(pairs),
            timestamp: chrono::Utc::now(),
        }
    }

    /// `(name, value)` of every field in the built embed.
    fn embed_fields(event: &Event) -> Vec<(String, String)> {
        build_embed(event)["fields"]
            .as_array()
            .map(|a| {
                a.iter()
                    .map(|f| {
                        let s = |k: &str| f[k].as_str().unwrap_or_default().to_string();
                        (s("name"), s("value"))
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    fn owned(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
        f(pairs)
    }

    /// NT-11 fold: three ID/name pairs render as three fields, not six.
    /// Reverting the fold posts every raw key (`ability_id`,
    /// `ability_name`, ...) as its own field.
    #[test]
    fn three_pairs_render_three_fields() {
        let event = tracing_event(&[
            ("ability_id", "880"),
            ("ability_name", "Staff Blast"),
            ("target", "4123"),
            ("target_name", "Jaffa Guard"),
            ("space_id", "12"),
            ("world", "Castle_CellBlock"),
        ]);
        assert_eq!(
            embed_fields(&event),
            owned(&[
                ("ability", "Staff Blast (#880)"),
                ("target", "Jaffa Guard (#4123)"),
                ("space", "Castle_CellBlock (#12)"),
                ("Log target", "abilities"),
            ])
        );
    }

    /// NT-11 budget: 30 raw fields, the five pairs listed LAST. Every
    /// pair survives and only the unpaired tail is cut. Reverting the
    /// ordering (or the fold) cuts the pairs, which sit past the cap in
    /// the raw order.
    #[test]
    fn thirty_fields_keep_every_pair_and_drop_only_the_unpaired_tail() {
        let mut raw: Vec<(String, String)> = (0..20)
            .map(|i| (format!("k{i:02}"), format!("v{i}")))
            .collect();
        for (p, id, name) in [
            ("ability", "880", "Staff Blast"),
            ("effect", "77", "Bleed"),
            ("mission", "1562", "Castle_Cellblock_1"),
            ("template", "300", "Jaffa Guard"),
            ("dialog", "4242", "Intro"),
        ] {
            raw.push((format!("{p}_id"), id.into()));
            raw.push((format!("{p}_name"), name.into()));
        }
        assert_eq!(raw.len(), 30);
        let event = Event::TracingEvent {
            kind: TracingEventKind::Error,
            target: "cell".into(),
            message: "m".into(),
            fields: raw,
            timestamp: chrono::Utc::now(),
        };
        let fields = embed_fields(&event);
        assert!(fields.len() <= super::super::MAX_FIELDS);
        for (label, value) in [
            ("ability", "Staff Blast (#880)"),
            ("effect", "Bleed (#77)"),
            ("mission", "Castle_Cellblock_1 (#1562)"),
            ("template", "Jaffa Guard (#300)"),
            ("dialog", "Intro (#4242)"),
        ] {
            assert!(
                fields.iter().any(|(k, v)| k == label && v == value),
                "pair `{label}` dropped: {fields:?}"
            );
        }
        // 24 slots before the log target: 5 pairs + 18 unpaired + the
        // overflow marker. k18 and k19 are the cut tail.
        let kept: Vec<&str> = fields
            .iter()
            .filter(|(k, _)| k.starts_with('k'))
            .map(|(k, _)| k.as_str())
            .collect();
        assert_eq!(kept.len(), 18, "{fields:?}");
        assert!(!kept.contains(&"k18") && !kept.contains(&"k19"));
        assert!(fields
            .iter()
            .any(|(k, v)| k == "…" && v == "+2 more fields"));
        assert_eq!(fields.last().map(|(k, _)| k.as_str()), Some("Log target"));
    }

    /// The Who fold renders both halves in one field, first, and leaves
    /// no raw identity keys behind.
    #[test]
    fn who_fold_renders_player_and_account() {
        let event = tracing_event(&[
            ("reason", "out_of_range"),
            ("account_id", "6"),
            ("account_name", "steve"),
            ("player_id", "100"),
            ("player_name", "Alice"),
        ]);
        let fields = embed_fields(&event);
        assert_eq!(
            fields[0],
            ("Who".to_string(), "Alice (#100) · steve (#6)".to_string())
        );
        assert!(
            !fields
                .iter()
                .any(|(k, _)| k.starts_with("player_") || k.starts_with("account_")),
            "{fields:?}"
        );
    }

    /// A missing name renders as the ID alone, `#id` (Rule 6 "Discord").
    #[test]
    fn missing_name_renders_hash_id() {
        let fields = embed_fields(&tracing_event(&[("ability_id", "880")]));
        assert_eq!(fields[0], ("ability".to_string(), "#880".to_string()));
    }

    /// The trace ID lands in the footer as plain text, never a field.
    #[test]
    fn trace_id_goes_to_the_footer() {
        let id = "4bf92f3577b34da6a3ce929d0e0e4736";
        let embed = build_embed(&tracing_event(&[("trace_id", id), ("reason", "x")]));
        assert_eq!(
            embed["footer"]["text"],
            Value::String(format!("trace_id {id}"))
        );
        assert!(!embed["fields"].to_string().contains("trace_id"));
    }

    fn f(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    fn get<'a>(out: &'a [(String, String, bool)], label: &str) -> Option<&'a str> {
        out.iter()
            .find(|(k, _, _)| k == label)
            .map(|(_, v, _)| v.as_str())
    }

    #[test]
    fn who_renders_each_half_independently() {
        let out = fold_fields(&f(&[("account_id", "6"), ("player_id", "100")]), 24);
        assert_eq!(get(&out, WHO_LABEL), Some("#100 · #6"));

        let out = fold_fields(&f(&[("account_name", "steve"), ("account_id", "6")]), 24);
        assert_eq!(get(&out, WHO_LABEL), Some("steve (#6)"));

        let out = fold_fields(&f(&[("character_name", "Alice"), ("player_id", "100")]), 24);
        assert_eq!(get(&out, WHO_LABEL), Some("Alice (#100)"));
    }

    #[test]
    fn item_type_id_claims_item_name_over_item_id() {
        let out = fold_fields(
            &f(&[
                ("item_id", "900001"),
                ("item_type_id", "5168"),
                ("item_name", "Staff"),
            ]),
            24,
        );
        assert_eq!(get(&out, "item_type"), Some("Staff (#5168)"));
        assert_eq!(get(&out, "item"), Some("#900001"));
    }

    #[test]
    fn overflow_marker_reports_dropped_count() {
        let fields: Vec<(String, String)> =
            (0..10).map(|i| (format!("k{i}"), "v".into())).collect();
        let out = fold_fields(&fields, 4);
        assert_eq!(out.len(), 4);
        assert_eq!(out[3], ("…".into(), "+7 more fields".into(), true));
    }

    /// Review #1199: when the pairs alone fill the budget, the marker
    /// must not displace one of them.
    #[test]
    fn pairs_that_fill_the_budget_are_not_displaced_by_the_marker() {
        let mut raw: Vec<(String, String)> = (0..24)
            .map(|i| (format!("o{i:02}_id"), i.to_string()))
            .collect();
        raw.extend((0..3).map(|i| (format!("k{i}"), "v".into())));
        let out = fold_fields(&raw, 24);
        assert_eq!(out.len(), 24);
        assert!(
            out.iter()
                .all(|(k, v, _)| k.starts_with('o') && v.starts_with('#')),
            "{out:?}"
        );

        // One slot of room: the marker takes it, every pair stays.
        let out = fold_fields(&raw[1..], 24);
        assert_eq!(out.len(), 24);
        assert_eq!(
            out.iter().filter(|(k, _, _)| k.starts_with('o')).count(),
            23
        );
        assert_eq!(out[23], ("…".into(), "+3 more fields".into(), true));
    }

    #[test]
    fn trace_footer_is_plain_text() {
        let id = "4bf92f3577b34da6a3ce929d0e0e4736";
        let fields = f(&[(
            "trace_id",
            &format!("https://signoz.internal:3301/trace/{id}?spanId=00f067aa0ba902b7"),
        )]);
        assert_eq!(trace_footer(&fields), Some(format!("trace_id {id}")));
        assert!(
            fold_fields(&fields, 24).is_empty(),
            "trace_id is never a field"
        );

        let fields = f(&[("trace_id", id), ("span_id", "00f067aa0ba902b7")]);
        assert_eq!(
            trace_footer(&fields),
            Some(format!("trace_id {id} · span_id 00f067aa0ba902b7"))
        );
        assert_eq!(trace_footer(&f(&[("reason", "x")])), None);

        // Review #1199: one SigNoz URL in both keys yields each key's own
        // ID, not the longest hex run for both.
        let link = format!("https://signoz.internal:3301/trace/{id}?spanId=00f067aa0ba902b7");
        let fields = f(&[("trace_id", &link), ("span_id", &link)]);
        assert_eq!(
            trace_footer(&fields),
            Some(format!("trace_id {id} · span_id 00f067aa0ba902b7"))
        );
    }
}
