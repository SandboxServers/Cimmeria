//! Rule 6 key pairing: which name field goes with which ID field.
//!
//! Mirrors the "Which key gets which name" section of
//! `docs/architecture/instrumentation-discipline.md` (Rule 6). The
//! exceptions table below is that section's table, row for row; the
//! default rule (`<p>_id` → `<p>_name`) and the bare entity keys are
//! the prose above it. Change the doc and this file together.
//!
//! Kept as plain `&'static` data with no crate dependencies so it can
//! move to a shared crate later: NT-03's unpaired-ID scanner in
//! `crates/server` carries the same table today.

/// One row of Rule 6's exceptions table: an ID key whose name key (or
/// display label) differs from the default rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PairRule {
    /// The ID field's key, e.g. `space_id`.
    pub id_key: &'static str,
    /// The name field's key, e.g. `world`.
    pub name_key: &'static str,
    /// The embed field label the folded pair renders under, e.g. `space`.
    pub label: &'static str,
}

const fn rule(id_key: &'static str, name_key: &'static str, label: &'static str) -> PairRule {
    PairRule {
        id_key,
        name_key,
        label,
    }
}

/// Rule 6's exceptions table. Rows whose name key follows the default
/// rule (`template_id`, `ability_id`, ...) are left out; the default
/// rule covers them.
///
/// `item_type_id` sits before `item_id` on purpose: both pair with
/// `item_name`, and the name is the type's, so the type ID claims it
/// when a line carries both.
pub(crate) const PAIR_EXCEPTIONS: &[PairRule] = &[
    rule("target", "target_name", "target"),
    rule("attacker", "attacker_name", "attacker"),
    rule("entity", "entity_name", "entity"),
    rule("witness", "witness_name", "witness"),
    rule("item_type_id", "item_name", "item_type"),
    rule("item_id", "item_name", "item"),
    rule("space_id", "world", "space"),
    rule("world_id", "world", "world"),
    rule("archetype", "archetype_name", "archetype"),
    rule("error_code", "error_name", "error"),
    rule("opcode", "msg_name", "opcode"),
    rule("msg_id", "msg_name", "msg"),
    rule("method_id", "method_name", "method"),
    rule("method_index", "method_name", "method"),
];

/// Keys that hold an ID but are not paired with a name: they are
/// correlation tokens (Rule 6's `nt:id-only` class). The Discord embed
/// moves the trace pair to the footer as plain text (D-NT3).
pub(crate) const TRACE_KEYS: &[&str] = &["trace_id", "span_id"];

/// The pairing for `id_key`, or `None` when the key is not an ID key.
///
/// Exceptions first, then the default rule: `<p>_id` pairs with
/// `<p>_name` and renders under `<p>`.
/// Returns `(name_key, label)`.
pub(crate) fn pair_for(id_key: &str) -> Option<(String, String)> {
    if let Some(r) = PAIR_EXCEPTIONS.iter().find(|r| r.id_key == id_key) {
        return Some((r.name_key.to_string(), r.label.to_string()));
    }
    if TRACE_KEYS.contains(&id_key) {
        return None;
    }
    let prefix = id_key.strip_suffix("_id").filter(|p| !p.is_empty())?;
    Some((format!("{prefix}_name"), prefix.to_string()))
}

/// Render an object as `Name (#id)`, `#id` when the name is missing,
/// or the bare name when only the name is known. `None` when neither
/// is known.
///
/// An empty name counts as missing: a handful of call sites still log
/// `unwrap_or("")` (Rule 6 forbids it, and the sweeps remove them).
pub(crate) fn name_with_id(name: Option<&str>, id: Option<&str>) -> Option<String> {
    let name = name.filter(|n| !n.is_empty());
    match (name, id) {
        (Some(n), Some(i)) => Some(format!("{n} (#{i})")),
        (None, Some(i)) => Some(format!("#{i}")),
        (Some(n), None) => Some(n.to_string()),
        (None, None) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_rule_pairs_prefix_name() {
        assert_eq!(
            pair_for("ability_id"),
            Some(("ability_name".into(), "ability".into()))
        );
        assert_eq!(
            pair_for("subject_player_id"),
            Some(("subject_player_name".into(), "subject_player".into()))
        );
    }

    #[test]
    fn exceptions_override_the_default_rule() {
        assert_eq!(pair_for("space_id").unwrap().0, "world");
        assert_eq!(pair_for("item_type_id").unwrap().0, "item_name");
        assert_eq!(pair_for("method_index").unwrap().0, "method_name");
        assert_eq!(pair_for("target").unwrap().0, "target_name");
    }

    #[test]
    fn non_id_and_trace_keys_do_not_pair() {
        assert_eq!(pair_for("reason"), None);
        assert_eq!(pair_for("_id"), None);
        assert_eq!(pair_for("trace_id"), None);
        assert_eq!(pair_for("span_id"), None);
    }

    #[test]
    fn name_with_id_degrades_like_account_value() {
        assert_eq!(
            name_with_id(Some("Staff Blast"), Some("880")).as_deref(),
            Some("Staff Blast (#880)")
        );
        assert_eq!(name_with_id(None, Some("880")).as_deref(), Some("#880"));
        assert_eq!(name_with_id(Some(""), Some("880")).as_deref(), Some("#880"));
        assert_eq!(name_with_id(Some("steve"), None).as_deref(), Some("steve"));
        assert_eq!(name_with_id(None, None), None);
    }
}
