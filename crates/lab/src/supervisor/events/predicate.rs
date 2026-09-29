//! What `client_wait_event` waits for: a conjunction of optional clauses
//! over one stored event. Pure, so every clause is unit-tested off Windows.
//!
//! All string comparisons are case-insensitive globs (`*` any run, `?` one
//! character): CME event names are long and the caller rarely wants to spell
//! `Event_NetIn_` every time (`*onEffectResults` is enough).

use serde_json::{Map, Value};

use super::store::StoredEvent;

/// Field names that carry an event's *name*, in the order they are tried.
/// `cme.event` has `event`, `net.out` has `method`, `combat.hit` has
/// `ability_name`, the entity events and hooks have `name`.
pub const NAME_FIELDS: [&str; 5] = ["event", "method", "name", "ability_name", "channel_name"];

/// Field names that carry an entity id. `combat.hit` has no ids (the stock
/// UI exposes unit slots, not ids), so an `entity_id` clause never matches
/// one; use `fields: {target: "..."}` there.
pub const ENTITY_FIELDS: [&str; 4] = ["entity_id", "source_id", "target_id", "id"];

/// Field names searched by the `text` clause before falling back to the
/// whole field set.
pub const TEXT_FIELDS: [&str; 3] = ["text", "message", "line"];

/// The predicate. Every clause that is present must hold; an empty
/// predicate matches any event.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct EventPredicate {
    pub kind: Option<String>,
    pub name: Option<String>,
    pub entity_id: Option<i64>,
    pub text: Option<String>,
    pub fields: Map<String, Value>,
}

impl EventPredicate {
    pub fn is_empty(&self) -> bool {
        self.kind.is_none()
            && self.name.is_none()
            && self.entity_id.is_none()
            && self.text.is_none()
            && self.fields.is_empty()
    }

    pub fn matches(&self, ev: &StoredEvent) -> bool {
        if let Some(k) = &self.kind {
            if !glob(k, &ev.kind) {
                return false;
            }
        }
        if let Some(n) = &self.name {
            let hit = NAME_FIELDS
                .iter()
                .filter_map(|f| ev.fields.get(*f))
                .any(|v| glob(n, &scalar_text(v)));
            if !hit {
                return false;
            }
        }
        if let Some(id) = self.entity_id {
            let hit = ENTITY_FIELDS
                .iter()
                .filter_map(|f| ev.fields.get(*f))
                .any(|v| as_i64(v) == Some(id));
            if !hit {
                return false;
            }
        }
        if let Some(t) = &self.text {
            if !text_matches(t, &ev.fields) {
                return false;
            }
        }
        self.fields
            .iter()
            .all(|(k, want)| ev.fields.get(k).is_some_and(|got| value_matches(want, got)))
    }
}

/// Case-insensitive glob: `*` matches any run (empty included), `?` one
/// character. Iterative with single-star backtracking, so a pathological
/// pattern cannot blow the stack.
pub fn glob(pattern: &str, text: &str) -> bool {
    let p: Vec<char> = pattern.to_lowercase().chars().collect();
    let t: Vec<char> = text.to_lowercase().chars().collect();
    let (mut pi, mut ti) = (0usize, 0usize);
    let mut star: Option<(usize, usize)> = None;
    while ti < t.len() {
        if pi < p.len() && (p[pi] == '?' || p[pi] == t[ti]) {
            pi += 1;
            ti += 1;
        } else if pi < p.len() && p[pi] == '*' {
            star = Some((pi, ti));
            pi += 1;
        } else if let Some((sp, st)) = star {
            pi = sp + 1;
            ti = st + 1;
            star = Some((sp, st + 1));
        } else {
            return false;
        }
    }
    p[pi..].iter().all(|c| *c == '*')
}

fn has_wildcards(s: &str) -> bool {
    s.contains('*') || s.contains('?')
}

/// A JSON scalar as text (strings unquoted).
fn scalar_text(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

fn as_i64(v: &Value) -> Option<i64> {
    match v {
        Value::Number(n) => n.as_i64().or_else(|| n.as_f64().map(|f| f as i64)),
        Value::String(s) => s.trim().parse().ok(),
        _ => None,
    }
}

/// The `text` clause: a substring (case-insensitive), or a glob when the
/// pattern has wildcards, over the text-carrying fields; when the event has
/// none of them, over the whole field set serialized.
fn text_matches(pattern: &str, fields: &Value) -> bool {
    let candidates: Vec<String> = TEXT_FIELDS
        .iter()
        .filter_map(|f| fields.get(*f))
        .map(scalar_text)
        .collect();
    let haystacks = if candidates.is_empty() {
        vec![fields.to_string()]
    } else {
        candidates
    };
    let needle = pattern.to_lowercase();
    haystacks.iter().any(|h| {
        if has_wildcards(pattern) {
            glob(pattern, h)
        } else {
            h.to_lowercase().contains(&needle)
        }
    })
}

/// One `fields` clause: strings glob against the field's text, numbers
/// compare numerically (a string field holding a number counts), anything
/// else must be equal.
fn value_matches(want: &Value, got: &Value) -> bool {
    match want {
        Value::String(p) => glob(p, &scalar_text(got)),
        Value::Number(n) => match (n.as_f64(), got) {
            (Some(w), Value::Number(g)) => g.as_f64() == Some(w),
            (Some(w), Value::String(s)) => s.trim().parse::<f64>().ok() == Some(w),
            _ => false,
        },
        other => other == got,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn ev(kind: &str, fields: Value) -> StoredEvent {
        StoredEvent {
            seq: 1,
            kind: kind.into(),
            ts_ms: 0,
            fields,
        }
    }

    #[test]
    fn glob_handles_stars_questions_and_case() {
        assert!(glob("*onEffectResults", "Event_NetIn_onEffectResults"));
        assert!(glob("event_netin_*", "Event_NetIn_onDialogDisplay"));
        assert!(glob("a?c", "abc"));
        assert!(!glob("a?c", "ac"));
        assert!(glob("*", ""));
        assert!(glob("a*b*c", "aXXbYYc"));
        assert!(!glob("a*b*c", "aXXbYY"));
        assert!(glob("exact", "EXACT"));
        assert!(!glob("exact", "exactly"));
        // Backtracking past a false start.
        assert!(glob("*ab", "aab"));
    }

    #[test]
    fn empty_predicate_matches_everything() {
        let p = EventPredicate::default();
        assert!(p.is_empty());
        assert!(p.matches(&ev("anything", json!({}))));
    }

    #[test]
    fn kind_and_name_clauses_read_the_name_fields() {
        let cme = ev(
            "cme.event",
            json!({ "event": "Event_NetIn_onTimerUpdate", "kind": "net_in" }),
        );
        let out = ev("net.out", json!({ "method": "useAbility", "entity_id": 7 }));
        let p = EventPredicate {
            kind: Some("cme.*".into()),
            name: Some("*onTimerUpdate".into()),
            ..Default::default()
        };
        assert!(p.matches(&cme));
        assert!(!p.matches(&out), "kind clause excludes net.out");
        let by_method = EventPredicate {
            name: Some("useability".into()),
            ..Default::default()
        };
        assert!(by_method.matches(&out));
        assert!(!by_method.matches(&cme));
    }

    #[test]
    fn entity_id_matches_numbers_and_numeric_strings() {
        let p = EventPredicate {
            entity_id: Some(5400),
            ..Default::default()
        };
        assert!(p.matches(&ev("entity.enter", json!({ "entity_id": 5400 }))));
        assert!(p.matches(&ev("x", json!({ "target_id": "5400" }))));
        assert!(!p.matches(&ev("x", json!({ "entity_id": 5401 }))));
        // An event without any id field never matches an id clause.
        assert!(!p.matches(&ev("combat.hit", json!({ "target": "Guard" }))));
    }

    #[test]
    fn text_is_a_substring_or_a_glob_over_text_fields() {
        let chat = ev(
            "chat.line",
            json!({ "text": "You are too far away from your target", "channel": 9 }),
        );
        let sub = EventPredicate {
            text: Some("TOO FAR".into()),
            ..Default::default()
        };
        assert!(sub.matches(&chat));
        let g = EventPredicate {
            text: Some("you are*target".into()),
            ..Default::default()
        };
        assert!(g.matches(&chat));
        let miss = EventPredicate {
            text: Some("cooldown".into()),
            ..Default::default()
        };
        assert!(!miss.matches(&chat));
        // No text field: the whole field set is searched.
        let cegui = ev("hook.hit", json!({ "addr": "0x1234" }));
        let any = EventPredicate {
            text: Some("0x1234".into()),
            ..Default::default()
        };
        assert!(any.matches(&cegui));
    }

    #[test]
    fn field_clauses_glob_strings_and_compare_numbers() {
        let hit = ev(
            "combat.hit",
            json!({ "hit_name": "Critical", "ability_id": 1234, "mortal": false,
                    "target": "Cellblock Guard" }),
        );
        let mut fields = Map::new();
        fields.insert("hit_name".into(), json!("crit*"));
        fields.insert("ability_id".into(), json!(1234));
        fields.insert("mortal".into(), json!(false));
        let p = EventPredicate {
            fields,
            ..Default::default()
        };
        assert!(p.matches(&hit));
        let mut wrong = Map::new();
        wrong.insert("ability_id".into(), json!(99));
        assert!(!EventPredicate {
            fields: wrong,
            ..Default::default()
        }
        .matches(&hit));
        let mut missing = Map::new();
        missing.insert("absent".into(), json!("x"));
        assert!(!EventPredicate {
            fields: missing,
            ..Default::default()
        }
        .matches(&hit));
    }

    #[test]
    fn every_clause_must_hold() {
        let e = ev("net.out", json!({ "method": "useAbility", "entity_id": 3 }));
        let p = EventPredicate {
            kind: Some("net.out".into()),
            name: Some("useAbility".into()),
            entity_id: Some(4),
            ..Default::default()
        };
        assert!(!p.matches(&e));
    }
}
