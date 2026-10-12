//! Argument rules, the golden file's path and the `--rows` cleanup.
//! Ports `Test-GoldenArguments` and `Get-GoldenPath` (the removed PowerShell golden library) and
//! `ConvertTo-UatRowIds` (the removed PowerShell uat library).

use std::path::{Path, PathBuf};

/// The owner's rule is "golden only when five runs agree" (README D-GD6).
pub const MIN_RUNS: u32 = 5;
/// The most fresh runs one golden may be recorded from.
pub const MAX_RUNS: u32 = 20;
/// The most lanes the runs may be split over.
pub const MAX_LEASES: u32 = 5;

/// The argument checks that need no daemon: an error message, or `None`.
pub fn validate_arguments(
    action: &str,
    section: &str,
    runs: u32,
    leases: u32,
    server_version: &str,
) -> Option<String> {
    if action != "record" && action != "show" {
        return Some("usage: lab golden record|show <section> ...".to_string());
    }
    if section.is_empty() {
        return Some(format!("usage: lab golden {action} <section>"));
    }
    // The section names a file under golden/, so it may not reach outside it.
    if !is_section_id(section) || section.contains("..") {
        return Some(format!(
            "section '{section}' is not a section id (letters, digits, '.', '_' and '-')"
        ));
    }
    if !(MIN_RUNS..=MAX_RUNS).contains(&runs) {
        return Some(format!(
            "--runs must be {MIN_RUNS} to {MAX_RUNS} (a golden needs five agreeing runs)"
        ));
    }
    if !(1..=MAX_LEASES).contains(&leases) {
        return Some(format!("--leases must be 1 to {MAX_LEASES}"));
    }
    if !runs.is_multiple_of(leases) {
        return Some(format!(
            "--runs {runs} does not split evenly over --leases {leases}"
        ));
    }
    if action == "record" && !is_build_sha(server_version) {
        return Some(
            "--server-version must be the server build's commit (7 to 40 lowercase hex characters)"
                .to_string(),
        );
    }
    None
}

/// `^[A-Za-z0-9][A-Za-z0-9._-]*$`, ASCII only and case-sensitive.
fn is_section_id(section: &str) -> bool {
    let mut chars = section.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    first.is_ascii_alphanumeric()
        && chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
}

/// `^[0-9a-f]{7,40}$`.
fn is_build_sha(value: &str) -> bool {
    (7..=40).contains(&value.len())
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

/// The committed golden of a section under a spec folder, named as the
/// daemon names it (`evidence.rs path_safe`): every character outside
/// `[A-Za-z0-9_-]` becomes `_`, so `v1.2` is `golden/v1_2.json`.
pub fn golden_path(specs_dir: &Path, section: &str) -> PathBuf {
    let safe: String = section
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect();
    specs_dir.join("golden").join(format!("{safe}.json"))
}

/// `--rows` as clean ids: split on commas, trimmed, blanks and repeats
/// (compared ignoring case, as PowerShell's `-notcontains` does) dropped.
pub fn row_ids(rows: &[String]) -> Vec<String> {
    let mut ids: Vec<String> = Vec::new();
    for raw in rows {
        for part in raw.split(',') {
            let id = part.trim();
            if !id.is_empty() && !ids.iter().any(|seen| seen.eq_ignore_ascii_case(id)) {
                ids.push(id.to_string());
            }
        }
    }
    ids
}

#[cfg(test)]
mod tests {
    use super::*;

    const SHA: &str = "1a2b3c4";

    fn valid(action: &str, section: &str, runs: u32, leases: u32, sha: &str) -> bool {
        validate_arguments(action, section, runs, leases, sha).is_none()
    }

    /// The refusal text; empty when the arguments were accepted (so a
    /// `starts_with` assertion fails then).
    fn msg(action: &str, section: &str, runs: u32, leases: u32, sha: &str) -> String {
        validate_arguments(action, section, runs, leases, sha).unwrap_or_default()
    }

    // PS L21
    #[test]
    fn five_serial_runs_with_a_build_are_valid() {
        assert!(valid("record", "first-session", 5, 1, SHA));
    }

    // PS L22
    #[test]
    fn ten_runs_on_two_lanes_with_a_40_hex_build_are_valid() {
        let sha40 = "0123456789abcdef".repeat(2) + "01234567";
        assert_eq!(sha40.len(), 40);
        assert!(valid("record", "first-session", 10, 2, &sha40));
    }

    // PS L23
    #[test]
    fn show_needs_no_build() {
        assert!(valid("show", "first-session", 5, 1, ""));
    }

    // PS L24
    #[test]
    fn an_unknown_action_is_refused() {
        assert!(
            msg("bless", "first-session", 5, 1, SHA).starts_with("usage: lab golden record|show")
        );
    }

    // PS L25
    #[test]
    fn no_action_is_refused() {
        assert!(msg("", "first-session", 5, 1, SHA).starts_with("usage:"));
    }

    // PS L26
    #[test]
    fn a_section_is_required() {
        assert_eq!(
            msg("record", "", 5, 1, SHA),
            "usage: lab golden record <section>"
        );
    }

    // PS L27
    #[test]
    fn a_section_cannot_reach_outside_golden() {
        assert!(msg("show", "..\\..\\secrets", 5, 1, "").contains("is not a section id"));
    }

    // PS L28
    #[test]
    fn a_section_with_dot_dot_is_refused() {
        assert!(msg("record", "a..b", 5, 1, SHA).contains("is not a section id"));
    }

    // PS L29
    #[test]
    fn fewer_than_five_runs_is_impossible() {
        assert!(msg("record", "first-session", 4, 1, SHA).starts_with("--runs must be 5 to 20"));
    }

    // PS L30
    #[test]
    fn one_run_is_impossible() {
        assert!(msg("record", "first-session", 1, 1, SHA).starts_with("--runs must be 5 to 20"));
    }

    // PS L31
    #[test]
    fn more_than_twenty_runs_is_refused() {
        assert!(msg("record", "first-session", 21, 1, SHA).starts_with("--runs must be 5 to 20"));
    }

    // PS L32
    #[test]
    fn the_run_rule_holds_for_show_too() {
        assert!(msg("show", "first-session", 4, 1, "").starts_with("--runs must be"));
    }

    // PS L33
    #[test]
    fn zero_leases_is_refused() {
        assert_eq!(
            msg("record", "first-session", 5, 0, SHA),
            "--leases must be 1 to 5"
        );
    }

    // PS L34
    #[test]
    fn six_leases_is_refused() {
        assert_eq!(
            msg("record", "first-session", 12, 6, SHA),
            "--leases must be 1 to 5"
        );
    }

    // PS L35
    #[test]
    fn runs_that_do_not_split_over_the_lanes_are_refused() {
        assert!(msg("record", "first-session", 5, 2, SHA).contains("does not split evenly"));
    }

    // PS L36
    #[test]
    fn record_without_a_build_is_refused() {
        assert!(msg("record", "first-session", 5, 1, "").starts_with("--server-version must be"));
    }

    // PS L37
    #[test]
    fn a_6_character_build_is_refused() {
        assert!(
            msg("record", "first-session", 5, 1, "1a2b3c").starts_with("--server-version must be")
        );
    }

    // PS L38
    #[test]
    fn an_uppercase_build_is_refused() {
        assert!(
            msg("record", "first-session", 5, 1, "1A2B3C4").starts_with("--server-version must be")
        );
    }

    // PS L39
    #[test]
    fn a_41_character_build_is_refused() {
        let sha41 = "a".repeat(41);
        assert!(
            msg("record", "first-session", 5, 1, &sha41).starts_with("--server-version must be")
        );
    }

    // PS L40
    #[test]
    fn a_branch_name_is_not_a_build() {
        assert!(
            msg("record", "first-session", 5, 1, "main").starts_with("--server-version must be")
        );
    }

    // PS L41
    #[test]
    fn the_golden_sits_under_specs_golden_section_json() {
        let specs = Path::new("specs");
        assert_eq!(
            golden_path(specs, "first-session"),
            specs.join("golden").join("first-session.json")
        );
    }

    // PS L43
    #[test]
    fn a_section_with_a_dot_is_valid() {
        assert!(valid("record", "v1.2", 5, 1, SHA));
    }

    // PS L44
    #[test]
    fn v1_2_is_stored_as_v1_2_json() {
        let specs = Path::new("specs");
        assert_eq!(
            golden_path(specs, "v1.2"),
            specs.join("golden").join("v1_2.json")
        );
    }

    // PS L45
    #[test]
    fn letters_digits_underscore_and_dash_keep_their_case() {
        let specs = Path::new("specs");
        assert_eq!(
            golden_path(specs, "gm_Parity-2"),
            specs.join("golden").join("gm_Parity-2.json")
        );
    }

    // PS L46
    #[test]
    fn a_section_with_a_slash_is_refused() {
        assert!(msg("show", "a/b", 5, 1, "").contains("is not a section id"));
    }

    // PS L47
    #[test]
    fn a_section_with_a_backslash_is_refused() {
        assert!(msg("show", "a\\b", 5, 1, "").contains("is not a section id"));
    }

    // PS L48
    #[test]
    fn a_non_ascii_letter_kelvin_sign_is_refused() {
        assert!(msg("show", "\u{212A}", 5, 1, "").contains("is not a section id"));
    }

    // New (ConvertTo-UatRowIds had its own PowerShell test elsewhere).
    #[test]
    fn row_ids_split_trim_and_drop_repeats() {
        let rows = vec![
            "FS-01, FS-02".to_string(),
            "fs-01".to_string(),
            " ,FS-P1,".to_string(),
        ];
        assert_eq!(row_ids(&rows), vec!["FS-01", "FS-02", "FS-P1"]);
        assert!(row_ids(&[]).is_empty());
    }
}
