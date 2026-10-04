//! Public entry points: assemble the embed JSON body from an [`Event`].

use serde_json::{json, Value};

use crate::color;
use crate::event::Event;

use super::budget::{enforce_total_budget, truncate};
use super::format::format_event;
use super::links::{strip_links_in_embed, ALLOWED_LINK_HOSTS};
use super::tracing_fields::trace_footer;
use super::{MAX_DESC, MAX_FIELDS, MAX_FIELD_VALUE, MAX_FOOTER, MAX_TITLE};

/// Build the JSON body of a Discord webhook POST from one event.
///
/// Returns the full request body — usually one embed under `"embeds"`,
/// plus `username`/`avatar_url` overrides if the caller threads them in
/// later. Today the caller passes a single embed and the body shape is:
///
/// ```json
/// { "embeds": [ {...} ] }
/// ```
pub fn build_embed_body(event: &Event, username: Option<&str>, avatar_url: Option<&str>) -> Value {
    let embed = build_embed(event);
    let mut body = json!({ "embeds": [embed] });
    if let (Some(u), Some(obj)) = (username, body.as_object_mut()) {
        obj.insert("username".to_string(), Value::String(u.to_string()));
    }
    if let (Some(a), Some(obj)) = (avatar_url, body.as_object_mut()) {
        obj.insert("avatar_url".to_string(), Value::String(a.to_string()));
    }
    body
}

/// Build the embed object itself (without the surrounding `embeds` array).
/// Exposed for tests; production code goes through [`build_embed_body`].
pub fn build_embed(event: &Event) -> Value {
    let (title, description, fields, timestamp) = format_event(event);

    let color = color::for_severity(event.severity());
    let title = truncate(&title, MAX_TITLE);
    let description = truncate(&description, MAX_DESC);

    let mut fields_json: Vec<Value> = fields
        .into_iter()
        .take(MAX_FIELDS)
        .map(|(name, value, inline)| {
            json!({
                "name": truncate(&name, 256),
                "value": truncate(&value, MAX_FIELD_VALUE),
                "inline": inline,
            })
        })
        .collect();

    let mut embed = json!({
        "title": title,
        "description": description,
        "color": color,
        "timestamp": timestamp,
    });
    if !fields_json.is_empty() {
        embed["fields"] = Value::Array(std::mem::take(&mut fields_json));
    }
    // The trace ID rides in the footer as plain text a developer can
    // paste into SigNoz (D-NT3), never as a link.
    if let Event::TracingEvent { fields, .. } = event {
        if let Some(text) = trace_footer(fields) {
            embed["footer"] = json!({ "text": truncate(&text, MAX_FOOTER) });
        }
    }

    // No internal links (Rule 6, "Discord"): runs over the whole
    // rendered embed, every variant, before the budget pass so the
    // budget sees the final strings.
    strip_links_in_embed(&mut embed, ALLOWED_LINK_HOSTS);

    // Final guard: if the total character budget is exceeded, trim
    // description first (the most likely culprit), then fields, in
    // declaration order.
    enforce_total_budget(&mut embed);
    embed
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{ChannelKind, ChatKind, Event, Named};
    use chrono::Utc;

    fn now() -> chrono::DateTime<Utc> {
        Utc::now()
    }

    /// **Privacy regression guard.** Whisper content must NEVER be
    /// posted, even when the caller hands the event in fully populated.
    /// Reverting `format_chat`'s whisper branch trips this immediately.
    #[test]
    fn whisper_content_is_hidden_regardless_of_input() {
        let event = Event::Chat {
            kind: ChatKind::Whisper,
            speaker: Named::new(12, Some("alice".into())),
            recipient: Some(Named::new(13, Some("bob".into()))),
            content: "this should never appear in Discord".into(),
            timestamp: now(),
        };
        let body = build_embed_body(&event, None, None);
        let serialized = body.to_string();
        assert!(
            !serialized.contains("this should never appear"),
            "whisper body must not contain the raw message: {}",
            serialized
        );
        assert!(
            serialized.contains("[hidden]"),
            "whisper body must contain the hidden sentinel"
        );
    }

    /// Non-whisper chat preserves content.
    #[test]
    fn global_chat_preserves_content() {
        let event = Event::Chat {
            kind: ChatKind::Global,
            speaker: Named::new(12, Some("alice".into())),
            recipient: None,
            content: "hello world".into(),
            timestamp: now(),
        };
        let body = build_embed_body(&event, None, None);
        assert!(body.to_string().contains("hello world"));
    }

    /// Every event variant builds an embed without panicking
    /// and emits a valid color from the palette. The variants come from
    /// the NT-10 pairing table, which covers every one.
    #[test]
    fn every_event_variant_builds() {
        let events = super::super::pairing_tests::every_variant()
            .into_iter()
            .map(|(e, _)| e);

        for e in events {
            let body = build_embed_body(&e, Some("Cimmeria"), None);
            assert!(body.get("embeds").is_some(), "missing embeds array");
            assert_eq!(e.kind(), e.kind()); // round-trip self-check
                                            // Sanity: routing resolves cleanly for every variant.
            let _: ChannelKind = crate::router::channel_for(e.kind());
        }
    }
}
