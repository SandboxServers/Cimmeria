//! Rule 6's pairing contract as code: which keys are IDs, which name key each
//! one needs, and the `// nt:id-only <reason>` exemption.
//!
//! The rule is [instrumentation-discipline.md § Rule 6]. A key is ID-shaped
//! when its last dotted component ends in `_id`, is a bare entity key
//! (`target`, `attacker`, `entity`, `witness`), or is one of the exceptions
//! table's non-`_id` keys (`opcode`, `method_index`, `archetype`,
//! `error_code`). Its name key keeps the same prefix, dotted path included.
//!
//! [instrumentation-discipline.md § Rule 6]: ../../../../../docs/architecture/instrumentation-discipline.md

use std::collections::BTreeSet;

use super::calls::Call;

/// The exemption marker. The reason after it is mandatory.
pub(super) const MARKER: &str = "nt:id-only";

/// Bare keys that hold an entity ID and pair with `<key>_name`.
const BARE_ENTITY_KEYS: &[&str] = &["target", "attacker", "entity", "witness"];

/// Rule 6's exceptions, as `(ID suffix, name suffix)`. A key matches when it
/// is the suffix or ends in `_` + the suffix, and the prefix carries over:
/// `dest_space_id` pairs with `dest_world`. Checked in order, longest first
/// where one suffix ends another (`item_type_id` before `type_id`).
const EXCEPTIONS: &[(&str, &str)] = &[
    ("item_type_id", "item_name"),
    ("item_id", "item_name"),
    ("space_id", "world"),
    ("world_id", "world"),
    ("opcode", "msg_name"),
    ("msg_id", "msg_name"),
    ("method_id", "method_name"),
    ("method_index", "method_name"),
    ("error_code", "error_name"),
    ("archetype", "archetype_name"),
];

/// Keys that don't say what they identify. Rule 6 has a sweep rename them to
/// their domain key; until then no name pairs them, so they always count.
const GENERIC_KEYS: &[&str] = &["type_id", "design_id"];

/// What the scan concluded about one ID-shaped field.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum Verdict {
    Paired,
    Exempt,
    /// No pair and no exemption: counts against the file's baseline.
    Unpaired,
    /// A marker with no reason: fails the build outright, baseline or not.
    MarkerWithoutReason,
}

fn ends_with_part(key: &str, suffix: &str) -> Option<usize> {
    if key == suffix {
        return Some(0);
    }
    let cut = key.len().checked_sub(suffix.len() + 1)?;
    (key.ends_with(suffix) && key.as_bytes()[cut] == b'_').then_some(cut + 1)
}

/// The name key `key` must be paired with; `None` when `key` is not
/// ID-shaped. A generic key returns `Some(None)`: an ID no name can pair.
pub(super) fn name_key_for(key: &str) -> Option<Option<String>> {
    let (path, last) = key.rsplit_once('.').map_or(("", key), |(p, l)| (p, l));
    let dotted = |name: String| {
        if path.is_empty() {
            name
        } else {
            format!("{path}.{name}")
        }
    };
    if GENERIC_KEYS.contains(&last) {
        return Some(None);
    }
    if BARE_ENTITY_KEYS.contains(&last) {
        return Some(Some(dotted(format!("{last}_name"))));
    }
    for (id_suffix, name_suffix) in EXCEPTIONS {
        if let Some(at) = ends_with_part(last, id_suffix) {
            return Some(Some(dotted(format!("{}{name_suffix}", &last[..at]))));
        }
    }
    let prefix = last.strip_suffix("_id").filter(|p| !p.is_empty())?;
    Some(Some(dotted(format!("{prefix}_name"))))
}

/// The marker on `line`, if any: `Some(true)` with a reason, `Some(false)`
/// without one.
fn marker_on(line_comments: &[(usize, String)], line: usize) -> Option<bool> {
    line_comments
        .iter()
        .filter(|(l, _)| *l == line)
        .find_map(|(_, text)| {
            let at = text.find(MARKER)?;
            Some(!text[at + MARKER.len()..].trim().is_empty())
        })
}

/// Every ID-shaped field of `call` with its verdict, as `(line, key, verdict)`.
pub(super) fn judge(
    call: &Call,
    line_comments: &[(usize, String)],
) -> Vec<(usize, String, Verdict)> {
    let keys: BTreeSet<&str> = call.fields.iter().map(|f| f.key.as_str()).collect();
    let mut out = Vec::new();
    for field in &call.fields {
        let Some(name_key) = name_key_for(&field.key) else {
            continue;
        };
        let verdict = match marker_on(line_comments, field.line) {
            Some(true) => Verdict::Exempt,
            Some(false) => Verdict::MarkerWithoutReason,
            None if name_key.is_some_and(|n| keys.contains(n.as_str())) => Verdict::Paired,
            None => Verdict::Unpaired,
        };
        out.push((field.line, field.key.clone(), verdict));
    }
    out
}
