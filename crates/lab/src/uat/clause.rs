//! Pure clause evaluation: comparisons over JSON, the chat new-line
//! diff, and SigNoz attestation grading. No I/O here, so every rule is
//! unit-tested without a client.

use serde_json::{json, Value};

use super::evidence::Verdict;
use super::spec::{ExpectSpec, Op, Source};

/// Value at a JSON pointer (`/chat_tail/0`); the whole value for `None`
/// or `""`. A lua_eval result is a string; when it parses as JSON the
/// parsed value is compared, so `return 40` compares as the number 40.
pub fn json_at<'a>(v: &'a Value, pointer: Option<&str>) -> Option<&'a Value> {
    match pointer {
        None | Some("") => Some(v),
        Some(p) => v.pointer(p),
    }
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

/// Apply `op` (default: `eq` when a value is given, else `truthy`).
pub fn compare(
    op: Option<Op>,
    observed: Option<&Value>,
    value: Option<&Value>,
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
/// key row when given.
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
            if !compare(c.op, v, c.value.as_ref())? {
                return Ok(Verdict::Fail);
            }
        }
    }
    Ok(Verdict::Pass)
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
            }
            s
        }
        Source::Human => c.question.clone().unwrap_or_default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn clause(src: &str) -> ExpectSpec {
        let t = format!("id = \"c\"\ntext = \"t\"\n{src}");
        toml::from_str(&t).unwrap()
    }

    fn s(v: &[&str]) -> Vec<String> {
        v.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn lua_strings_compare_as_numbers() {
        assert!(compare(Some(Op::Eq), Some(&json!("40")), Some(&json!(40))).unwrap());
        assert!(compare(Some(Op::Gte), Some(&json!("6")), Some(&json!(6))).unwrap());
        assert!(!compare(Some(Op::Gt), Some(&json!("n/a")), Some(&json!(1))).unwrap());
        assert!(compare(None, Some(&json!("true")), None).unwrap());
        assert!(!compare(None, Some(&json!("false")), None).unwrap());
        assert!(compare(Some(Op::Absent), None, None).unwrap());
        assert!(compare(Some(Op::LenGte), Some(&json!([1, 2])), Some(&json!(2))).unwrap());
    }

    #[test]
    fn new_lines_follow_a_sliding_tail() {
        let before = s(&["a", "b", "c"]);
        assert_eq!(
            new_lines(&before, &s(&["b", "c", "d", "e"])),
            (s(&["d", "e"]), true)
        );
        // Repeated text: the longest overlap wins, so a second "c" is new.
        assert_eq!(
            new_lines(&before, &s(&["a", "b", "c", "c"])),
            (s(&["c"]), true)
        );
        // Cleared box (relog): everything is new, flagged.
        assert_eq!(new_lines(&before, &s(&["x"])), (s(&["x"]), false));
        assert_eq!(new_lines(&[], &s(&["x"])), (s(&["x"]), true));
    }

    #[test]
    fn chat_count_catches_a_double_echo() {
        let c = clause("source = \"chat\"\ncontains = \"hi\"\ncount = 1");
        let (v, obs, _) = eval_chat(&c, &s(&["[Say] Labone: hi", "[Say] Labone: hi"]));
        assert_eq!(v, Verdict::Fail);
        assert_eq!(obs["match_count"], 2);
        assert_eq!(eval_chat(&c, &s(&["[Say] Labone: hi"])).0, Verdict::Pass);
    }

    #[test]
    fn chat_capture_takes_group_one() {
        let c = clause("source = \"chat\"\nmatches = 'Bookmark (\\d+) recorded'");
        let (v, _, cap) = eval_chat(&c, &s(&["Bookmark 1790650000123 recorded: 3 of 3"]));
        assert_eq!(v, Verdict::Pass);
        assert_eq!(cap.as_deref(), Some("1790650000123"));
    }

    #[test]
    fn absent_passes_only_with_no_match() {
        let c = clause("source = \"chat\"\ncontains = \"error\"\nabsent = true");
        assert_eq!(eval_chat(&c, &s(&["fine"])).0, Verdict::Pass);
        assert_eq!(eval_chat(&c, &s(&["an error"])).0, Verdict::Fail);
    }

    #[test]
    fn signoz_rows_and_fields_grade() {
        let c = clause("source = \"signoz\"\nfilter = \"x\"");
        assert_eq!(grade_signoz(&c, 0, &[]).unwrap(), Verdict::Fail);
        assert_eq!(grade_signoz(&c, 2, &[]).unwrap(), Verdict::Pass);
        let c = clause(
            "source = \"signoz\"\nfilter = \"x\"\nfield = \"outcome\"\nop = \"eq\"\nvalue = \"up_to_date\"",
        );
        let ok = [json!({"outcome": "up_to_date"})];
        let bad = [json!({"outcome": "full_resync"})];
        assert_eq!(grade_signoz(&c, 1, &ok).unwrap(), Verdict::Pass);
        assert_eq!(grade_signoz(&c, 1, &bad).unwrap(), Verdict::Fail);
        assert!(grade_signoz(&c, 1, &[]).is_err());
        let none = clause("source = \"signoz\"\nfilter = \"x\"\nmax_rows = 0");
        assert_eq!(grade_signoz(&none, 0, &[]).unwrap(), Verdict::Pass);
        assert_eq!(grade_signoz(&none, 1, &[]).unwrap(), Verdict::Fail);
    }
}
