//! Pure clause evaluation: comparisons over JSON, the chat new-line
//! diff, and SigNoz attestation grading. No I/O here, so every rule is
//! unit-tested without a client.

use serde_json::{json, Value};

use super::evidence::{clip, Verdict};
use super::spec::{ExpectSpec, Op, Source};

/// Value at a JSON pointer (`/chat_tail/0`); the whole value for `None`
/// or `""`. A lua_eval result is a string; when it parses as JSON the
/// parsed value is compared, so `return 40` compares as the number 40.
///
/// A segment may end in selectors, `[key=value]`, that pick the first
/// array element whose `key` equals `value` (numbers numerically):
/// `/state/stats[stat_id=8]/cur` reads one stat out of
/// `server_ability_state`'s list without knowing its index.
pub fn json_at<'a>(v: &'a Value, pointer: Option<&str>) -> Option<&'a Value> {
    match pointer {
        None | Some("") => Some(v),
        Some(p) if !p.contains('[') => v.pointer(p),
        Some(p) => select_path(v, p),
    }
}

/// [`json_at`] for a pointer with `[key=value]` selectors.
fn select_path<'a>(v: &'a Value, pointer: &str) -> Option<&'a Value> {
    let mut cur = v;
    for seg in pointer.strip_prefix('/')?.split('/') {
        let (name, mut sels) = match seg.find('[') {
            Some(i) => (&seg[..i], &seg[i..]),
            None => (seg, ""),
        };
        if !name.is_empty() {
            let key = name.replace("~1", "/").replace("~0", "~");
            cur = match cur {
                Value::Object(o) => o.get(&key)?,
                Value::Array(a) => a.get(key.parse::<usize>().ok()?)?,
                _ => return None,
            };
        }
        while let Some(rest) = sels.strip_prefix('[') {
            let end = rest.find(']')?;
            let (k, want) = rest[..end].split_once('=')?;
            let want = Value::String(want.to_string());
            cur = cur
                .as_array()?
                .iter()
                .find(|e| e.get(k).is_some_and(|got| loose_eq(got, &want)))?;
            sels = &rest[end + 1..];
        }
        if !sels.is_empty() {
            return None;
        }
    }
    Some(cur)
}

fn as_f64(v: &Value) -> Option<f64> {
    match v {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.trim().parse().ok(),
        Value::Bool(b) => Some(f64::from(u8::from(*b))),
        _ => None,
    }
}

fn as_text(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

fn truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().is_some_and(|f| f != 0.0),
        Value::String(s) => !matches!(s.trim(), "" | "false" | "nil" | "0"),
        Value::Array(a) => !a.is_empty(),
        Value::Object(o) => !o.is_empty(),
    }
}

/// Loose equality: numbers compare numerically across string/number (Lua
/// returns strings), everything else by text.
fn loose_eq(a: &Value, b: &Value) -> bool {
    match (as_f64(a), as_f64(b)) {
        (Some(x), Some(y)) if !matches!(a, Value::Bool(_)) && !matches!(b, Value::Bool(_)) => {
            (x - y).abs() < 1e-9
        }
        _ => as_text(a) == as_text(b),
    }
}

/// Apply `op` (default: `eq` when a value is given, else `truthy`) with no
/// tolerance. Test-only: every runtime path calls [`compare_tol`] with the
/// clause's `tolerance`, so `approx` can never lose it.
#[cfg(test)]
pub fn compare(
    op: Option<Op>,
    observed: Option<&Value>,
    value: Option<&Value>,
) -> Result<bool, String> {
    compare_tol(op, observed, value, None)
}

/// [`compare`] with the clause's `tolerance`, which `approx` needs.
pub fn compare_tol(
    op: Option<Op>,
    observed: Option<&Value>,
    value: Option<&Value>,
    tolerance: Option<f64>,
) -> Result<bool, String> {
    let op = op.unwrap_or(if value.is_some() { Op::Eq } else { Op::Truthy });
    let need = || value.ok_or_else(|| format!("{op:?} needs a value"));
    let obs = observed.unwrap_or(&Value::Null);
    Ok(match op {
        Op::Exists => observed.is_some_and(|v| !v.is_null()),
        Op::Absent => observed.is_none_or(Value::is_null),
        Op::Truthy => truthy(obs),
        Op::Falsy => !truthy(obs),
        Op::Eq => loose_eq(obs, need()?),
        Op::Ne => !loose_eq(obs, need()?),
        Op::Contains => as_text(obs).contains(&as_text(need()?)),
        Op::NotContains => !as_text(obs).contains(&as_text(need()?)),
        Op::Matches => {
            let re = regex::Regex::new(&as_text(need()?)).map_err(|e| e.to_string())?;
            re.is_match(&as_text(obs))
        }
        Op::Approx => {
            let want = as_f64(need()?).ok_or("approx needs a numeric value")?;
            let tol = tolerance.ok_or("approx needs a tolerance")?;
            // A hair of slack so `15 ± 1` keeps 16.0 after float noise.
            as_f64(obs).is_some_and(|h| (h - want).abs() <= tol + 1e-9)
        }
        Op::Gt | Op::Gte | Op::Lt | Op::Lte | Op::LenGte => {
            let want = as_f64(need()?).ok_or("a numeric op needs a numeric value")?;
            let have = if op == Op::LenGte {
                match obs {
                    Value::Array(a) => a.len() as f64,
                    Value::String(s) => s.chars().count() as f64,
                    Value::Object(o) => o.len() as f64,
                    _ => return Ok(false),
                }
            } else {
                match as_f64(obs) {
                    Some(h) => h,
                    None => return Ok(false),
                }
            };
            match op {
                Op::Gt => have > want,
                Op::Gte | Op::LenGte => have >= want,
                Op::Lt => have < want,
                _ => have <= want,
            }
        }
    })
}

/// The lines in `after` that were not in `before`. Both are tails of the
/// same chat box, so `after` starts with a suffix of `before`: find the
/// longest such overlap. Returns the new lines and whether an overlap was
/// found (false means more new lines arrived than the tail holds, or the
/// box was cleared, e.g. by a relog — every line in `after` counts).
pub fn new_lines(before: &[String], after: &[String]) -> (Vec<String>, bool) {
    if before.is_empty() {
        return (after.to_vec(), true);
    }
    for k in (1..=before.len().min(after.len())).rev() {
        if before[before.len() - k..] == after[..k] {
            return (after[k..].to_vec(), true);
        }
    }
    (after.to_vec(), false)
}

/// Grade a chat clause against the new lines. Returns the verdict, the
/// observation for the bundle, and regex group 1 of the first match
/// (for `capture_var`).
pub fn eval_chat(c: &ExpectSpec, lines: &[String]) -> (Verdict, Value, Option<String>) {
    let re = c.matches.as_deref().map(regex::Regex::new);
    let re = match re {
        Some(Ok(r)) => Some(r),
        Some(Err(e)) => return (Verdict::Fail, json!({ "error": e.to_string() }), None),
        None => None,
    };
    let is_match = |l: &str| {
        c.contains.as_deref().is_none_or(|s| l.contains(s))
            && re.as_ref().is_none_or(|r| r.is_match(l))
    };
    let hits: Vec<&String> = lines.iter().filter(|l| is_match(l)).collect();
    let captured = re.as_ref().and_then(|r| {
        hits.first()
            .and_then(|l| r.captures(l))
            .and_then(|cap| cap.get(1))
            .map(|m| m.as_str().to_string())
    });
    let ok = if c.absent {
        hits.is_empty()
    } else if let Some(n) = c.count {
        hits.len() == n as usize
    } else {
        !hits.is_empty()
    };
    let observed = json!({
        "matching_lines": hits,
        "match_count": hits.len(),
        "new_lines": lines.len(),
    });
    (
        if ok { Verdict::Pass } else { Verdict::Fail },
        observed,
        captured,
    )
}

/// Grade a SigNoz clause from attested rows: `min_rows`/`max_rows` on the
/// count (default: at least one row), and `field` `op` `value` on every
/// key row when given. Packet clauses grade their matching rows the same
/// way ([`grade_packet`]).
pub fn grade_signoz(c: &ExpectSpec, row_count: u64, rows: &[Value]) -> Result<Verdict, String> {
    let min = c
        .min_rows
        .unwrap_or(if c.max_rows.is_some() { 0 } else { 1 });
    if row_count < min || c.max_rows.is_some_and(|m| row_count > m) {
        return Ok(Verdict::Fail);
    }
    if let Some(field) = &c.field {
        if rows.is_empty() && row_count > 0 {
            return Err(format!("field {field} needs the key rows attested"));
        }
        for r in rows {
            let v = r.get(field.as_str()).or_else(|| r.pointer(field));
            if !compare_tol(c.op, v, c.value.as_ref(), c.tolerance)? {
                return Ok(Verdict::Fail);
            }
        }
    }
    Ok(Verdict::Pass)
}

/// A tap row's direction for a clause's `direction`.
fn tap_dir(direction: Option<&str>) -> &'static str {
    match direction {
        Some("to_server") => "in",
        _ => "out",
    }
}

/// One tapped message flattened for grading: the decoded fields at the
/// top level (so `field = "cooldown"` reads the message's own argument),
/// with the tap's columns (`ts_ms`, `dir`, `msg_name`, `target_entity_id`,
/// `args_len`, `args_hex`) beside them. A decoded field never shadows a
/// tap column; `/decoded/...` pointers still reach the original.
pub fn packet_row(m: &Value) -> Value {
    let mut out = match m.get("decoded") {
        Some(Value::Object(d)) => d.clone(),
        _ => serde_json::Map::new(),
    };
    if let Value::Object(cols) = m {
        for (k, v) in cols {
            out.insert(k.clone(), v.clone());
        }
    }
    Value::Object(out)
}

/// Whether a tapped message is one the clause names.
pub fn packet_matches(c: &ExpectSpec, entity: Option<u64>, m: &Value) -> bool {
    let name_ok = m
        .get("msg_name")
        .and_then(Value::as_str)
        .zip(c.message.as_deref())
        .is_some_and(|(have, want)| have.eq_ignore_ascii_case(want));
    let dir_ok = m.get("dir").and_then(Value::as_str) == Some(tap_dir(c.direction.as_deref()));
    let entity_ok =
        entity.is_none_or(|e| m.get("target_entity_id").and_then(Value::as_u64) == Some(e));
    name_ok && dir_ok && entity_ok
}

/// Grade a packet clause against one `server_packet_tap_read` result
/// (`{messages, dropped, ...}`). Returns the verdict, the observation for
/// the bundle and a detail line. A ring that dropped messages cannot
/// prove an upper bound or "every row", so such a PASS is UNVERIFIED.
/// A read without a `messages` array or a numeric `dropped` is not an
/// empty, lossless tap: it proves nothing, so it is UNVERIFIED too.
pub fn grade_packet(
    c: &ExpectSpec,
    entity: Option<u64>,
    tap: &Value,
) -> (Verdict, Value, Option<String>) {
    let (Some(all), Some(dropped)) = (
        tap.get("messages").and_then(Value::as_array),
        tap.get("dropped").and_then(Value::as_u64),
    ) else {
        return (
            Verdict::Unverified,
            clip(tap, 300),
            Some("the tap read has no messages array or no numeric dropped count".into()),
        );
    };
    let rows: Vec<Value> = all
        .iter()
        .filter(|m| packet_matches(c, entity, m))
        .map(packet_row)
        .collect();
    let observed = json!({
        "matching_rows": rows.len(),
        "tapped_rows": all.len(),
        "dropped": dropped,
        "rows": clip(&json!(rows.iter().take(5).collect::<Vec<_>>()), 1500),
    });
    let verdict = match grade_signoz(c, rows.len() as u64, &rows) {
        Ok(v) => v,
        Err(e) => return (Verdict::Unverified, observed, Some(e)),
    };
    if verdict == Verdict::Pass && dropped > 0 && (c.max_rows.is_some() || c.field.is_some()) {
        let why = format!(
            "the tap ring dropped {dropped} message(s), so an upper bound or an every-row check cannot be proven"
        );
        return (Verdict::Unverified, observed, Some(why));
    }
    (verdict, observed, None)
}

/// A one-line statement of what the clause wants, for the bundle.
pub fn describe(c: &ExpectSpec) -> String {
    let opv = |op: Option<Op>, v: &Option<Value>| {
        format!(
            "{}{}",
            op.map(|o| format!("{o:?} ").to_lowercase())
                .unwrap_or_default(),
            v.as_ref().map(as_text).unwrap_or_default()
        )
    };
    match c.source {
        Source::Chat => {
            let what = c
                .contains
                .as_ref()
                .map(|s| format!("contains {s:?}"))
                .or_else(|| c.matches.as_ref().map(|m| format!("matches /{m}/")))
                .unwrap_or_default();
            if c.absent {
                format!("no new chat line {what}")
            } else if let Some(n) = c.count {
                format!("exactly {n} new chat line(s) {what}")
            } else {
                format!("a new chat line {what}")
            }
        }
        Source::Tool | Source::Server => format!(
            "{}{} {}",
            c.tool.as_deref().unwrap_or("?"),
            c.pointer.as_deref().unwrap_or(""),
            opv(c.op, &c.value)
        ),
        Source::Lua => format!("lua result {}", opv(c.op, &c.value)),
        Source::Wait => format!(
            "within {} ms: {}",
            c.timeout_ms.unwrap_or(10_000),
            c.lua_condition.as_deref().unwrap_or("")
        ),
        Source::Timing => format!(
            "action {} took at most {} ms",
            c.action.as_deref().unwrap_or("?"),
            c.max_ms.unwrap_or(0)
        ),
        Source::Signoz => {
            let mut s = format!("SigNoz: {}", c.filter.as_deref().unwrap_or(""));
            if let Some(m) = c.min_rows {
                s.push_str(&format!(" (>= {m} rows)"));
            }
            if let Some(m) = c.max_rows {
                s.push_str(&format!(" (<= {m} rows)"));
            }
            if let Some(f) = &c.field {
                s.push_str(&format!("; every row {f} {}", opv(c.op, &c.value)));
                if let Some(t) = c.tolerance {
                    s.push_str(&format!(" ± {t}"));
                }
            }
            s
        }
        Source::Packet => {
            let mut s = format!(
                "packet {} {}",
                c.direction.as_deref().unwrap_or("?"),
                c.message.as_deref().unwrap_or("?")
            );
            if let Some(e) = &c.entity {
                s.push_str(&format!(" for entity {}", as_text(e)));
            }
            if let Some(m) = c.min_rows {
                s.push_str(&format!(" (>= {m} rows)"));
            }
            if let Some(m) = c.max_rows {
                s.push_str(&format!(" (<= {m} rows)"));
            }
            if let Some(f) = &c.field {
                s.push_str(&format!("; every row {f} {}", opv(c.op, &c.value)));
                if let Some(t) = c.tolerance {
                    s.push_str(&format!(" ± {t}"));
                }
            }
            s
        }
        Source::ClientEvent => {
            let mut s = format!("client event {}", c.event.as_deref().unwrap_or("?"));
            if let Some(m) = &c.match_fields {
                s.push_str(&format!(" where {}", Value::Object(m.clone())));
            }
            if let Some(l) = &c.since {
                s.push_str(&format!(" since {l}"));
            }
            if let Some(m) = c.min_rows {
                s.push_str(&format!(" (>= {m} events)"));
            }
            if let Some(m) = c.max_rows {
                s.push_str(&format!(" (<= {m} events)"));
            }
            if let Some(f) = &c.field {
                s.push_str(&format!("; every event {f} {}", opv(c.op, &c.value)));
                if let Some(t) = c.tolerance {
                    s.push_str(&format!(" ± {t}"));
                }
            }
            s
        }
        Source::Human => c.question.clone().unwrap_or_default(),
    }
}

#[cfg(test)]
#[path = "clause_tests.rs"]
mod tests;
