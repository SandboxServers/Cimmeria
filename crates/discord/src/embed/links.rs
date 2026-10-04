//! No internal links in Discord (instrumentation-discipline.md Rule 6,
//! "Discord"): the last pass over every rendered embed removes any
//! `http://` or `https://` URL whose host is not on
//! [`ALLOWED_LINK_HOSTS`].
//!
//! Most of the team reading Discord can't reach SigNoz, the admin API
//! or any other VPN-only host, so a link there is dead text at best and
//! a leaked internal hostname at worst. Removing the link (rather than
//! refusing to post) keeps the rest of the message.

use serde_json::Value;

/// Hosts a Discord embed may link to: public, team-reachable sites
/// only. Empty today. A host matches itself and its subdomains.
pub(super) const ALLOWED_LINK_HOSTS: &[&str] = &[];

/// What a removed link leaves behind, so the reader sees one was there.
pub(super) const LINK_REMOVED: &str = "[link removed]";

/// Remove every non-allowlisted link from `embed`, recursively: string
/// values lose the URL, and URL-typed keys (`url`, `icon_url`, ...)
/// are dropped outright, since Discord rejects a non-URL there.
pub(super) fn strip_links_in_embed(embed: &mut Value, allowed: &[&str]) {
    match embed {
        Value::String(s) => {
            if let Some(clean) = strip_links(s, allowed) {
                *s = clean;
            }
        }
        Value::Array(items) => {
            for item in items {
                strip_links_in_embed(item, allowed);
            }
        }
        Value::Object(map) => {
            map.retain(|key, value| {
                !(key.ends_with("url")
                    && value
                        .as_str()
                        .is_some_and(|s| strip_links(s, allowed).is_some()))
            });
            for value in map.values_mut() {
                strip_links_in_embed(value, allowed);
            }
        }
        _ => {}
    }
}

/// `s` with every non-allowlisted URL replaced by [`LINK_REMOVED`], or
/// `None` when `s` has none (the common case, no allocation).
pub(super) fn strip_links(s: &str, allowed: &[&str]) -> Option<String> {
    let mut out = String::new();
    let mut rest = s;
    let mut changed = false;
    while let Some(start) = find_scheme(rest) {
        let (before, from_scheme) = rest.split_at(start);
        let end = from_scheme
            .find(|c: char| {
                c.is_whitespace() || matches!(c, '<' | '>' | '"' | '\'' | '`' | ')' | ']' | '|')
            })
            .unwrap_or(from_scheme.len());
        let (url, after) = from_scheme.split_at(end);
        out.push_str(before);
        if host_allowed(url, allowed) {
            out.push_str(url);
        } else {
            out.push_str(LINK_REMOVED);
            changed = true;
        }
        rest = after;
    }
    out.push_str(rest);
    changed.then_some(out)
}

/// Byte offset of the next `http://` or `https://`, case-insensitive.
fn find_scheme(s: &str) -> Option<usize> {
    let lower = s.to_ascii_lowercase();
    [lower.find("http://"), lower.find("https://")]
        .into_iter()
        .flatten()
        .min()
}

fn host_allowed(url: &str, allowed: &[&str]) -> bool {
    let Some((_, after_scheme)) = url.split_once("://") else {
        return false;
    };
    let authority = after_scheme
        .split(['/', '?', '#'])
        .next()
        .unwrap_or_default();
    // Drop userinfo and port: `user@host:port`.
    let host = authority.rsplit('@').next().unwrap_or_default();
    let host = host
        .split(':')
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase();
    !host.is_empty()
        && allowed.iter().any(|a| {
            let a = a.to_ascii_lowercase();
            host == a || host.ends_with(&format!(".{a}"))
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn strips_every_scheme_and_keeps_surrounding_text() {
        let s = "see HTTPS://signoz.internal:3301/trace/abc (and http://10.0.0.5/x) done";
        assert_eq!(
            strip_links(s, &[]).as_deref(),
            Some("see [link removed] (and [link removed]) done")
        );
        assert_eq!(strip_links("no links here", &[]), None);
    }

    #[test]
    fn allowlist_keeps_listed_hosts_and_subdomains_only() {
        let allowed = ["example.org"];
        assert_eq!(strip_links("https://example.org/a", &allowed), None);
        assert_eq!(
            strip_links("https://docs.example.org:443/a", &allowed),
            None
        );
        assert_eq!(
            strip_links("https://example.org.evil.test/a", &allowed).as_deref(),
            Some(LINK_REMOVED)
        );
        assert_eq!(
            strip_links("https://user@signoz.internal/", &allowed).as_deref(),
            Some(LINK_REMOVED)
        );
    }

    /// NT-11 no-links guard, end to end: a SigNoz URL in a harvested
    /// field value, the message and the trace ID renders without the
    /// link anywhere in the embed. Reverting the guard in `build_embed`
    /// posts the URL.
    #[test]
    fn signoz_url_in_a_field_value_renders_without_it() {
        use crate::embed::build_embed;
        use crate::event::{Event, TracingEventKind};

        let url = "https://signoz.internal:3301/trace/4bf92f3577b34da6a3ce929d0e0e4736";
        let event = Event::TracingEvent {
            kind: TracingEventKind::Error,
            target: "cell".into(),
            message: format!("stalled, see {url}"),
            fields: vec![
                ("reason".into(), format!("tick_stall see {url}")),
                ("trace_id".into(), url.into()),
            ],
            timestamp: chrono::Utc::now(),
        };
        let s = build_embed(&event).to_string();
        assert!(!s.contains("signoz.internal"), "{s}");
        assert!(!s.contains("://"), "{s}");
        assert!(s.contains("tick_stall see [link removed]"), "{s}");
        assert!(
            s.contains("trace_id 4bf92f3577b34da6a3ce929d0e0e4736"),
            "{s}"
        );
    }

    #[test]
    fn url_keys_are_dropped_and_nested_strings_cleaned() {
        let mut embed = json!({
            "title": "x",
            "url": "https://signoz.internal/trace/1",
            "footer": { "text": "trace https://signoz.internal/t", "icon_url": "http://admin.internal/i.png" },
            "fields": [{ "name": "reason", "value": "http://admin.internal/api", "inline": true }],
        });
        strip_links_in_embed(&mut embed, ALLOWED_LINK_HOSTS);
        let s = embed.to_string();
        assert!(!s.contains("://"), "a link survived: {s}");
        assert!(embed.get("url").is_none());
        assert!(embed["footer"].get("icon_url").is_none());
        assert_eq!(embed["footer"]["text"], "trace [link removed]");
    }
}
