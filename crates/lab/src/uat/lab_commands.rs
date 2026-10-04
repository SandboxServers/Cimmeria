//! The ability lab commands as capabilities (ability-mechanics AB-L3):
//! `@cooldowns_reset`, `@dummy` and `@clear_effects` are the AB-L2 dot
//! commands (`.cooldowns reset`, `.dummy`, `.cleareffects`), typed into
//! chat at tier G. They are not router tools: the runner builds the line
//! from the action's `args`, types it, and waits for the command's own
//! feedback line, so a refused or unanswered command fails the action
//! instead of passing silently.
//!
//! This module is pure (spec-time checks and the line builder); the
//! runner side is `runner::lab_commands`.

use serde_json::Value;

/// `@cooldowns_reset`: `.cooldowns reset [abilityId]`.
pub const COOLDOWNS_RESET_TOOL: &str = "uat_cooldowns_reset";
/// `@dummy`: `.dummy [hostile|friendly|clear] [templateId]`.
pub const DUMMY_TOOL: &str = "uat_dummy";
/// `@clear_effects`: `.cleareffects` on the selection (else the caller).
pub const CLEAR_EFFECTS_TOOL: &str = "uat_clear_effects";

/// A refusal from any of the three: the server's error lines start with
/// the dotted command (`.dummy: ...`, `.cooldowns reset: abilityId must
/// ...`, `.cleareffects: entity 7 is gone`); the success lines do not.
pub const ERROR_REPLY: &str = r"\.(?:dummy|cooldowns|cleareffects)\b[^:]*: ";

/// The var a placed dummy's entity id is stored in.
pub const DUMMY_ID_VAR: &str = "dummy_id";

/// One command ready to type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LabCommand {
    /// The chat line.
    pub line: String,
    /// Target this character or entity name first, with real input
    /// (`client_target`). The action fails, and nothing is typed, when the
    /// target does not take: `.cleareffects` acts on the selection, so a
    /// stale one would strip the wrong entity.
    pub target_name: Option<String>,
    /// The feedback line that confirms it; group 1 (when present) is
    /// stored in [`Self::capture`].
    pub reply: &'static str,
    pub capture: Option<&'static str>,
}

/// Whether `tool` is one of the three.
pub fn is_lab_command(tool: &str) -> bool {
    matches!(tool, COOLDOWNS_RESET_TOOL | DUMMY_TOOL | CLEAR_EFFECTS_TOOL)
}

fn keys_only(args: Option<&Value>, allowed: &[&str], tool: &str) -> Result<(), String> {
    match args {
        None | Some(Value::Null) => Ok(()),
        Some(Value::Object(m)) => match m.keys().find(|k| !allowed.contains(&k.as_str())) {
            Some(k) => Err(format!(
                "{tool}: unknown arg {k:?} (takes {})",
                if allowed.is_empty() {
                    "none".to_string()
                } else {
                    allowed.join(", ")
                }
            )),
            None => Ok(()),
        },
        Some(other) => Err(format!("{tool}: args must be a table, not {other}")),
    }
}

/// A positive id, or (at spec time) a `${var}` that will hold one.
fn id_arg(v: Option<&Value>, what: &str, allow_var: bool) -> Result<Option<u64>, String> {
    match v {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Number(n)) => n
            .as_u64()
            .filter(|n| *n > 0 && *n <= i32::MAX as u64)
            .map(Some)
            .ok_or_else(|| format!("{what} must be a positive integer, not {n}")),
        Some(Value::String(s)) if allow_var && s.contains("${") => Ok(None),
        Some(Value::String(s)) => s
            .trim()
            .parse::<u64>()
            .ok()
            .filter(|n| *n > 0 && *n <= i32::MAX as u64)
            .map(Some)
            .ok_or_else(|| format!("{what} must be a positive integer, not {s:?}")),
        Some(other) => Err(format!("{what} must be a positive integer, not {other}")),
    }
}

fn disposition(args: Option<&Value>) -> Result<&str, String> {
    match args.and_then(|a| a.get("disposition")) {
        None | Some(Value::Null) => Ok("hostile"),
        Some(Value::String(s)) if matches!(s.as_str(), "hostile" | "friendly" | "clear") => {
            Ok(s.as_str())
        }
        Some(other) => Err(format!(
            "{DUMMY_TOOL}: disposition is hostile, friendly or clear, not {other}"
        )),
    }
}

/// Spec-time check of a lab command's args (`${var}` placeholders
/// allowed). Any other tool passes.
pub fn check(tool: &str, args: Option<&Value>) -> Result<(), String> {
    match tool {
        COOLDOWNS_RESET_TOOL => {
            keys_only(args, &["ability_id"], tool)?;
            id_arg(args.and_then(|a| a.get("ability_id")), "ability_id", true).map(drop)
        }
        DUMMY_TOOL => {
            keys_only(args, &["disposition", "template_id"], tool)?;
            let d = disposition(args)?;
            let t = args.and_then(|a| a.get("template_id"));
            id_arg(t, "template_id", true)?;
            if d == "clear" && t.is_some_and(|v| !v.is_null()) {
                return Err(format!("{DUMMY_TOOL}: clear takes no template_id"));
            }
            Ok(())
        }
        CLEAR_EFFECTS_TOOL => {
            keys_only(args, &["name"], tool)?;
            match args.and_then(|a| a.get("name")) {
                None | Some(Value::Null | Value::String(_)) => Ok(()),
                Some(other) => Err(format!("{tool}: name must be a string, not {other}")),
            }
        }
        _ => Ok(()),
    }
}

/// The command for a lab-command action, its `${var}`s already filled in.
/// `None` for any other tool.
pub fn build(tool: &str, args: &Value) -> Result<Option<LabCommand>, String> {
    let args = Some(args);
    Ok(Some(match tool {
        COOLDOWNS_RESET_TOOL => {
            let id = id_arg(args.and_then(|a| a.get("ability_id")), "ability_id", false)?;
            LabCommand {
                line: id.map_or_else(
                    || ".cooldowns reset".to_string(),
                    |id| format!(".cooldowns reset {id}"),
                ),
                target_name: None,
                reply: r"cooldowns reset(?: \d+)?: ",
                capture: None,
            }
        }
        DUMMY_TOOL => {
            let d = disposition(args)?;
            let t = id_arg(
                args.and_then(|a| a.get("template_id")),
                "template_id",
                false,
            )?;
            if d == "clear" {
                LabCommand {
                    line: ".dummy clear".into(),
                    target_name: None,
                    reply: r"dummy clear: \d+ of your dummies removed",
                    capture: None,
                }
            } else {
                LabCommand {
                    line: t.map_or_else(|| format!(".dummy {d}"), |t| format!(".dummy {d} {t}")),
                    target_name: None,
                    reply: r"dummy \[(\d+)\] placed",
                    capture: Some(DUMMY_ID_VAR),
                }
            }
        }
        CLEAR_EFFECTS_TOOL => LabCommand {
            line: ".cleareffects".into(),
            target_name: args
                .and_then(|a| a.get("name"))
                .and_then(Value::as_str)
                .map(str::to_string),
            reply: r"cleareffects \[(\d+)\]",
            capture: None,
        },
        _ => return Ok(None),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn each_command_builds_its_line_and_reply() {
        let c = build(COOLDOWNS_RESET_TOOL, &json!({})).unwrap().unwrap();
        assert_eq!(c.line, ".cooldowns reset");
        let c = build(COOLDOWNS_RESET_TOOL, &json!({ "ability_id": 597 }))
            .unwrap()
            .unwrap();
        assert_eq!(c.line, ".cooldowns reset 597");
        assert!(regex::Regex::new(c.reply).unwrap().is_match(
            "cooldowns reset 597: was running, 0 moniker group(s) cleared; your client was sent the clear"
        ));
        let c = build(
            DUMMY_TOOL,
            &json!({ "disposition": "friendly", "template_id": "34" }),
        )
        .unwrap()
        .unwrap();
        assert_eq!(c.line, ".dummy friendly 34");
        let caps = regex::Regex::new(c.reply)
            .unwrap()
            .captures("dummy [4242] placed: friendly SGC Jaffa (template 34), Health 1000000")
            .unwrap();
        assert_eq!(&caps[1], "4242");
        assert_eq!(c.capture, Some(DUMMY_ID_VAR));
        assert_eq!(
            build(DUMMY_TOOL, &json!({})).unwrap().unwrap().line,
            ".dummy hostile"
        );
        let c = build(CLEAR_EFFECTS_TOOL, &json!({ "name": "Labone" }))
            .unwrap()
            .unwrap();
        assert_eq!(
            (c.line.as_str(), c.target_name.as_deref()),
            (".cleareffects", Some("Labone"))
        );
        assert!(build("client_hotbar", &json!({})).unwrap().is_none());
    }

    #[test]
    fn bad_args_are_spec_errors_and_vars_wait_for_the_run() {
        assert!(check(DUMMY_TOOL, Some(&json!({ "disposition": "angry" }))).is_err());
        assert!(check(
            DUMMY_TOOL,
            Some(&json!({ "disposition": "clear", "template_id": 3 }))
        )
        .is_err());
        assert!(check(COOLDOWNS_RESET_TOOL, Some(&json!({ "ability_id": -1 }))).is_err());
        assert!(check(COOLDOWNS_RESET_TOOL, Some(&json!({ "id": 5 }))).is_err());
        assert!(check(CLEAR_EFFECTS_TOOL, Some(&json!({ "name": 5 }))).is_err());
        check(
            COOLDOWNS_RESET_TOOL,
            Some(&json!({ "ability_id": "${ability}" })),
        )
        .unwrap();
        // At the run, an unfilled or non-numeric id is an error.
        assert!(build(COOLDOWNS_RESET_TOOL, &json!({ "ability_id": "x" })).is_err());
    }

    #[test]
    fn refusals_match_the_error_reply_and_successes_do_not() {
        let err = regex::Regex::new(ERROR_REPLY).unwrap();
        for refusal in [
            ".dummy: you are not in a space",
            ".cooldowns reset: abilityId must be a positive integer (got x)",
            ".cooldowns: usage .cooldowns | .cooldowns reset [abilityId]",
            ".cleareffects: entity 7 is gone",
        ] {
            assert!(err.is_match(refusal), "{refusal}");
        }
        for ok in [
            "dummy [7] placed: hostile SGC Jaffa (template 34)",
            "cooldowns reset: none were running",
            "cleareffects [7] Labone: nothing to clear",
        ] {
            assert!(!err.is_match(ok), "{ok}");
        }
    }
}
