//! Every row under an ability target names its `event`.
//!
//! The `cast_id` forensics query ("Reading one cast" in
//! `docs/gameplay/ability-system.md`) and the SigNoz ability views group and
//! filter by `event`. A row with none shows up as a blank line in the middle
//! of a cast: the 2026-10-04 colo smoke test (Heal Focus 597, cast 1) had
//! one, the `onStatUpdate` fan-out's no-witnesses row, and 41 more sat in the
//! source unseen. [`every_ability_row_names_an_event`] scans every crate the
//! server links for a `<level>!(target: "<ability target>", …)` or
//! `event!(target: …)` call and fails on one with no `event` field.
//!
//! [`every_effect_row_names_its_cast_and_players`] is stricter for the
//! effect scripts and the effect runtime, whose rows all run inside a cast
//! scope: each must carry `cast_id`, the caster's `player_id` and, when it
//! names a `target_id`, the `target_player_id` (the same smoke test's
//! `effect_script_dispatch` and `heal_focus` rows had no `cast_id`).
//!
//! The ability targets are AB-T7's ([`super::abilities_target_tests`]):
//! `abilities`, `abilities.*`, `vitals`, `base.entity_method`.

use std::collections::BTreeSet;

use super::target_scan_tests::{
    crates_dir, is_test_path, rs_files, strip_test_module, IN_PROCESS_CRATES,
};

/// The targets the forensics query reads.
fn is_ability_target(target: &str) -> bool {
    target == "abilities"
        || target.starts_with("abilities.")
        || target == "vitals"
        || target == "base.entity_method"
}

const MACROS: [&str; 6] = ["trace", "debug", "info", "warn", "error", "event"];

/// The macro call's arguments from just after `!(` to its closing paren,
/// with every string literal's contents blanked so a message that says
/// "event =" cannot pass for a field. `None` if the parens never close.
fn call_body(src: &str) -> Option<String> {
    let mut out = String::new();
    let mut depth = 1usize;
    let mut chars = src.chars();
    while let Some(c) = chars.next() {
        match c {
            '"' => {
                out.push('"');
                loop {
                    match chars.next()? {
                        '\\' => {
                            chars.next()?;
                        }
                        '"' => break,
                        _ => {}
                    }
                }
                out.push('"');
                continue;
            }
            // A char literal: '"' or '(' must not count.
            '\'' => {
                let rest = chars.as_str();
                let lit = rest.find('\'').filter(|&n| n <= 4);
                if let Some(n) = lit {
                    chars = rest[n + 1..].chars();
                    out.push_str("' '");
                    continue;
                }
            }
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(out);
                }
            }
            _ => {}
        }
        out.push(c);
    }
    None
}

/// The literal target the body opens with, if any.
fn literal_target(body_src: &str) -> Option<&str> {
    let rest = body_src.trim_start().strip_prefix("target:")?;
    let rest = rest.trim_start().strip_prefix('"')?;
    Some(&rest[..rest.find('"')?])
}

/// `true` if `code` (string contents blanked) has a field named `name`:
/// `name = …` or the shorthand `name,`.
fn has_field(code: &str, name: &str) -> bool {
    let mut from = 0;
    while let Some(i) = code[from..].find(name) {
        let at = from + i;
        from = at + name.len();
        let before = code[..at].chars().next_back();
        if before.is_some_and(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.') {
            continue;
        }
        let after = code[from..].trim_start();
        if (after.starts_with('=') && !after.starts_with("=="))
            || after.starts_with(',')
            || after.starts_with(')')
        {
            return true;
        }
    }
    false
}

fn has_event_field(code: &str) -> bool {
    has_field(code, "event")
}

/// `(line, target)` for every ability-target call in `src` with no `event`.
fn bare_rows(src: &str) -> Vec<(usize, String)> {
    ability_rows(src)
        .into_iter()
        .filter(|(_, _, code)| !has_event_field(code))
        .map(|(line, target, _)| (line, target))
        .collect()
}

/// `(line, target, code)` for every ability-target call in `src`, `code`
/// being its arguments with string contents blanked.
fn ability_rows(src: &str) -> Vec<(usize, String, String)> {
    let mut out = Vec::new();
    let mut from = 0;
    while let Some(i) = src[from..].find("!(") {
        let bang = from + i;
        from = bang + 2;
        let name_start = src[..bang]
            .rfind(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
            .map_or(0, |p| p + 1);
        if !MACROS.contains(&&src[name_start..bang]) {
            continue;
        }
        let Some(target) = literal_target(&src[from..]) else {
            continue;
        };
        if !is_ability_target(target) {
            continue;
        }
        let Some(code) = call_body(&src[from..]) else {
            continue;
        };
        let line = src[..bang].matches('\n').count() + 1;
        out.push((line, target.to_string(), code));
    }
    out
}

/// The core fields `code` lacks for an effect-script row: `cast_id`, the
/// caster's `player_id`, and `target_player_id` when it names a `target_id`.
fn missing_core_fields(code: &str) -> Vec<&'static str> {
    let mut out = Vec::new();
    if !has_field(code, "cast_id") {
        out.push("cast_id");
    }
    if !has_field(code, "player_id") {
        out.push("player_id");
    }
    if has_field(code, "target_id") && !has_field(code, "target_player_id") {
        out.push("target_player_id");
    }
    out
}

/// Where effect scripts and the effect runtime log, relative to `crates/`.
/// Every row there runs inside a cast scope (the fire, a pulse tick, a
/// channel cancel or an expiry of an effect the cast registered), so each
/// can name its cast.
const EFFECT_SOURCE_DIRS: &[&str] = &["cell-effect-scripts/src", "cell-world/src/cell/effects"];

/// Effect-dir files whose rows are about an entity, not a cast: the AB-T5
/// state snapshot is written on a `.bug` bookmark, a death or a disconnect.
const NOT_CAST_ROWS: &[&str] = &["ability_snapshot.rs"];

/// **The guard.** A failure names the file, line and target: give the row a
/// stable `event = "<snake_case>"` (and the core fields, per
/// `docs/architecture/instrumentation-discipline.md`).
#[test]
fn every_ability_row_names_an_event() {
    let root = crates_dir();
    let mut bad = BTreeSet::new();
    let mut seen = 0;
    for krate in IN_PROCESS_CRATES {
        let mut files = Vec::new();
        rs_files(&root.join(krate).join("src"), &mut files);
        for f in files {
            let rel = f.strip_prefix(&root).unwrap().to_path_buf();
            if is_test_path(&rel) {
                continue;
            }
            let src = std::fs::read_to_string(&f).unwrap();
            let src = strip_test_module(&src);
            seen += src.matches("target: \"abilities").count();
            for (line, target) in bare_rows(src) {
                bad.insert(format!("{}:{line} ({target})", rel.display()));
            }
        }
    }
    assert!(
        seen > 150,
        "the scan saw only {seen} ability-target calls: moved?"
    );
    assert!(
        bad.is_empty(),
        "ability rows with no `event` field:\n{}",
        bad.into_iter().collect::<Vec<_>>().join("\n")
    );
}

/// **The effect-row guard (colo smoke test, 2026-10-04).** The Heal Focus
/// cast's `effect_script_dispatch` and `heal_focus` rows carried no
/// `cast_id`, and `heal_focus` not even the caster's ids, so a cast's
/// forensics query skipped exactly the rows that say what the heal did.
/// Every ability-target row an effect script or the effect runtime writes
/// names its cast, the caster's player and (when it names a target) the
/// target's player. Give a new row `ctx.row_ids()`'s fields
/// (`EffectContext::row_ids`).
#[test]
fn every_effect_row_names_its_cast_and_players() {
    let root = crates_dir();
    let mut bad = BTreeSet::new();
    let mut seen = 0;
    for dir in EFFECT_SOURCE_DIRS {
        let mut files = Vec::new();
        rs_files(&root.join(dir), &mut files);
        assert!(!files.is_empty(), "{dir} holds no sources: moved?");
        for f in files {
            let rel = f.strip_prefix(&root).unwrap().to_path_buf();
            let name = rel.file_name().unwrap().to_string_lossy();
            if is_test_path(&rel) || name.contains("test") || NOT_CAST_ROWS.contains(&&*name) {
                continue;
            }
            let src = std::fs::read_to_string(&f).unwrap();
            for (line, target, code) in ability_rows(strip_test_module(&src)) {
                seen += 1;
                let missing = missing_core_fields(&code);
                if !missing.is_empty() {
                    bad.insert(format!(
                        "{}:{line} ({target}) lacks {}",
                        rel.display(),
                        missing.join(", ")
                    ));
                }
            }
        }
    }
    assert!(seen > 40, "the scan saw only {seen} effect rows: moved?");
    assert!(
        bad.is_empty(),
        "effect rows that cannot be joined to their cast:\n{}",
        bad.into_iter().collect::<Vec<_>>().join("\n")
    );
}

#[test]
fn missing_core_fields_reads_the_field_names() {
    assert_eq!(
        missing_core_fields("target: \"abilities\", event = \"\", target_id = 1, x"),
        ["cast_id", "player_id", "target_player_id"]
    );
    assert!(missing_core_fields(
        "target: \"abilities\", cast_id, player_id = p, target_id, target_player_id = t"
    )
    .is_empty());
    // `ids.cast_id` as a value is not a `cast_id` field.
    assert_eq!(
        missing_core_fields("target: \"abilities\", x = ids.cast_id, player_id = p"),
        ["cast_id"]
    );
}

/// The scanner itself: the forms the source uses, and the traps.
#[test]
fn bare_rows_finds_only_rows_without_an_event() {
    let src = r#"
        tracing::debug!(target: "abilities", event = "a", "named");
        tracing::debug!(target: "abilities.wire", event = EVENT_X, x = 1, "const");
        tracing::event!(target: "abilities", $level, event, "shorthand");
        tracing::debug!(target: "abilities", x = f(1, (2)), "no event = here");
        tracing::warn!(
            target: "vitals",
            prevent = 1,
            "an ident that ends in event"
        );
        tracing::debug!(target: "other", "not an ability target");
        tracing::info!(entity_id, "module path");
        tracing::debug!(target: "base.entity_method", c = '"', event = "q", "char literal");
    "#;
    let got: Vec<_> = bare_rows(src).into_iter().map(|(_, t)| t).collect();
    assert_eq!(got, ["abilities", "vitals"]);
}
