//! The `lab record` argument checks that need no daemon (`Test-RecordArguments`):
//! a usage error exits 2 before anything is asked of the daemon. The message
//! texts are the PowerShell command's, word for word, flag spellings included.

use std::sync::OnceLock;

use regex::Regex;

use super::Args;

/// The longest recording, in minutes.
pub const MAX_MINUTES: u32 = 30;
/// A section id.
pub const SECTION_PATTERN: &str = "^[a-z0-9][a-z0-9-]{0,47}$";
/// What `--character-expr` may hold.
pub const CHARACTER_EXPR_PATTERN: &str = "^[A-Za-z0-9 _${}]{1,64}$";
/// An instance name; it becomes part of the state file's name.
pub const INSTANCE_PATTERN: &str = "^[A-Za-z0-9_-]{1,32}$";

/// The usage text: four lines, in the PowerShell spelling.
pub const USAGE: &str = "usage: lab record start <section> [-Instance p2] [-Character <name>] [-MaxMinutes 30] [-SpecsDir <dir>] [-Json]\n       lab record status [-Instance p2] [-Json]\n       lab record stop [-Instance p2] [-CharacterExpr 'Px${run_id}'] [-Profile <name>] [-NoDraft] [-RecordingId <id>] [-Json]\n       lab record transform <recording id or folder> [-Section <id>] [-CharacterExpr ...] [-Profile ...] [-SpecsDir <dir>] [-Json]";

fn matches(cell: &'static OnceLock<Regex>, pattern: &str, text: &str) -> bool {
    cell.get_or_init(|| Regex::new(pattern).expect("record pattern"))
        .is_match(text)
}

/// True when `text` is a section id (case sensitive).
pub fn is_section(text: &str) -> bool {
    static RE: OnceLock<Regex> = OnceLock::new();
    matches(&RE, SECTION_PATTERN, text)
}

/// True when `text` is an instance name (case sensitive).
pub fn is_instance(text: &str) -> bool {
    static RE: OnceLock<Regex> = OnceLock::new();
    matches(&RE, INSTANCE_PATTERN, text)
}

/// True when `text` is a character expression (case sensitive).
pub fn is_character_expr(text: &str) -> bool {
    static RE: OnceLock<Regex> = OnceLock::new();
    matches(&RE, CHARACTER_EXPR_PATTERN, text)
}

/// `Some(text)` when an option was given: not absent and not an empty string.
pub fn given(value: &Option<String>) -> Option<&str> {
    value.as_deref().filter(|text| !text.is_empty())
}

/// The first problem with `args` that needs no daemon, or `None`.
pub fn check_arguments(args: &Args) -> Option<String> {
    let action = args.action.as_deref().unwrap_or("");
    if !["start", "status", "stop", "transform"].contains(&action) {
        return Some(USAGE.to_string());
    }
    let target = given(&args.target).unwrap_or("");
    match action {
        "start" => {
            if target.is_empty() {
                return Some(format!(
                    "start needs a section: lab record start <section>\n{USAGE}"
                ));
            }
            if !is_section(target) {
                return Some(format!(
                    "section: give a section id, {SECTION_PATTERN} (first-session); got '{target}'"
                ));
            }
        }
        "transform" => {
            if target.is_empty() {
                return Some(format!(
                    "transform needs a recording: lab record transform <recording id or folder>\n{USAGE}"
                ));
            }
        }
        _ => {
            if !target.is_empty() {
                return Some(format!(
                    "{action} takes no positional argument; got '{target}'"
                ));
            }
        }
    }
    let instance = given(&args.instance).unwrap_or("");
    if !instance.is_empty() && !is_instance(instance) {
        return Some(format!(
            "-Instance: give an instance name like p2 ([A-Za-z0-9_-]); got '{instance}'"
        ));
    }
    // A string, so `-MaxMinutes abc` exits 2 with this message.
    let minutes_ok = match given(&args.max_minutes) {
        None => true,
        Some(m) => {
            m.len() <= 9
                && m.bytes().all(|b| b.is_ascii_digit())
                && m.parse::<u32>()
                    .is_ok_and(|n| (1..=MAX_MINUTES).contains(&n))
        }
    };
    if !minutes_ok {
        return Some(format!("-MaxMinutes must be 1 to {MAX_MINUTES}"));
    }
    let expr = given(&args.character_expr).unwrap_or("");
    if !expr.is_empty() && !is_character_expr(expr) {
        return Some("-CharacterExpr: give ^[A-Za-z0-9 _${}]{1,64}$ (Px${run_id})".to_string());
    }
    let section = given(&args.section).unwrap_or("");
    if !section.is_empty() && !is_section(section) {
        return Some(format!(
            "-Section: give a section id, {SECTION_PATTERN}; got '{section}'"
        ));
    }
    // (action the option belongs to, option name, whether it was given)
    let only_for = [
        ("start", "Character", given(&args.character).is_some()),
        ("start", "MaxMinutes", given(&args.max_minutes).is_some()),
        ("stop", "NoDraft", args.no_draft),
        ("stop", "RecordingId", given(&args.recording_id).is_some()),
        ("transform", "Section", given(&args.section).is_some()),
    ];
    for (owner, option, set) in only_for {
        if action != owner && set {
            return Some(format!("-{option} is for lab record {owner}"));
        }
    }
    let expr_or_profile = given(&args.character_expr).is_some() || given(&args.profile).is_some();
    if matches!(action, "start" | "status") && expr_or_profile {
        return Some("-CharacterExpr and -Profile are for stop and transform".to_string());
    }
    if matches!(action, "status" | "stop") && given(&args.specs_dir).is_some() {
        return Some("-SpecsDir is for start and transform".to_string());
    }
    if action == "transform" && !instance.is_empty() {
        return Some("transform runs on no instance; drop -Instance".to_string());
    }
    None
}
