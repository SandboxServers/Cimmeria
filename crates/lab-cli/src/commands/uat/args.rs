//! The argument checks of `lab uat`. They run before the daemon is asked, so
//! they exit 2 whatever its state.

use super::Args;

/// Most lanes one batch may run (and most hosted instances).
pub const MAX_LEASES: u32 = 5;
/// Most runs one lane may do.
pub const MAX_RUNS_PER_LEASE: u32 = 20;

/// The usage error for a missing section (exit 2).
pub const USAGE: &str = "usage: lab uat <section> [--rows FS-01,...] [--leases 1-5] [--runs-per-lease 1-20] [--instance p2] [--fixtures auto|always|never] [--diff-golden] [--chaos spec,...] [--heal] [--plan-only] [--json]";

/// Where the progress lines go (`Get-UatProgressMode`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Progress {
    /// Stdout (the PowerShell's `Write-Host`).
    Host,
    /// Stderr: `--progress-to-stderr`, even under `--json` or `--quiet`.
    Stderr,
    /// Nowhere: `--json` or `--quiet`.
    None,
}

/// `--progress-to-stderr` wins, then `--json` or `--quiet` silence, else stdout.
pub fn progress_mode(json: bool, quiet: bool, to_stderr: bool) -> Progress {
    if to_stderr {
        Progress::Stderr
    } else if json || quiet {
        Progress::None
    } else {
        Progress::Host
    }
}

/// The checks that need no daemon: the message to print (exit 2), or `None`.
/// The order is the PowerShell's.
pub fn check_arguments(args: &Args) -> Option<String> {
    let instance = args.instance.as_deref().is_some_and(|i| !i.is_empty());
    if !(1..=MAX_LEASES).contains(&args.leases) {
        return Some(format!("--leases must be 1 to {MAX_LEASES}"));
    }
    if !(1..=MAX_RUNS_PER_LEASE).contains(&args.runs_per_lease) {
        return Some(format!(
            "--runs-per-lease must be 1 to {MAX_RUNS_PER_LEASE}"
        ));
    }
    if instance && args.leases != 1 {
        return Some("--instance runs one lane; drop --leases or --instance".to_string());
    }
    if args.stagger_seconds > 300 {
        return Some("--stagger-seconds must be 0 to 300".to_string());
    }
    if !(1..=120).contains(&args.run_timeout_minutes) {
        return Some("--run-timeout-minutes must be 1 to 120".to_string());
    }
    if args.max_consecutive_failures > 20 {
        return Some("--max-consecutive-failures must be 0 to 20".to_string());
    }
    // lab-fixtures D-FX10: when a row's fixture is applied. Case sensitive: the
    // daemon takes snake_case.
    if let Some(fixtures) = args.fixtures.as_deref().filter(|f| !f.is_empty()) {
        if !["auto", "always", "never"].contains(&fixtures) {
            return Some("--fixtures must be auto, always or never".to_string());
        }
    }
    // D-SV8: a heal recalibrates on the batch's one instance, where the row failed.
    if args.heal && args.leases != 1 {
        return Some(
            "--heal needs --leases 1 (it recalibrates on the instance the batch ran on)"
                .to_string(),
        );
    }
    None
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

/// The non-empty `--chaos` values. They are not split on commas: a multi-knob
/// spec is one value. Both the request body and the header use this one filter,
/// so they always agree.
pub fn chaos_specs(chaos: &[String]) -> Vec<String> {
    chaos.iter().filter(|c| !c.is_empty()).cloned().collect()
}

/// The typed header: one `chaos=<spec>` per value, joined with `"; "`; `None`
/// when there is no chaos.
pub fn chaos_header(specs: &[String]) -> Option<String> {
    if specs.is_empty() {
        return None;
    }
    let parts: Vec<String> = specs.iter().map(|s| format!("chaos={s}")).collect();
    Some(parts.join("; "))
}

/// cpu chaos loads the whole machine and each lane's `lab_uat_run` starts its
/// own load, so `--leases 2` would double it. The profile is the text before
/// the first `:` of the trimmed spec, compared ignoring case (PowerShell `-eq`).
pub fn check_chaos_leases(chaos: &[String], leases: u32) -> Option<String> {
    let cpu = chaos.iter().any(|c| {
        c.trim()
            .split(':')
            .next()
            .unwrap_or("")
            .eq_ignore_ascii_case("cpu")
    });
    if cpu && leases > 1 {
        return Some("--chaos cpu loads the whole machine and each lease's run would start its own load; run it with --leases 1".to_string());
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::{parse_ok, try_parse, Command};

    fn uat(tokens: &[&str]) -> Args {
        let Command::Uat(args) = parse_ok("uat", tokens) else {
            panic!("not uat")
        };
        args
    }

    fn strings(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    // the removed PowerShell test file: 'valid arguments pass without a daemon'
    #[test]
    fn valid_arguments_pass_without_a_daemon() {
        assert_eq!(check_arguments(&uat(&["first-session"])), None);
    }

    // the removed PowerShell test file: 'argument checks need no daemon' (-Leases 9; 0 and 6 too)
    #[test]
    fn leases_outside_1_to_5_are_a_usage_error() {
        for n in ["0", "6", "9"] {
            let a = uat(&["first-session", "--leases", n]);
            assert_eq!(
                check_arguments(&a),
                Some("--leases must be 1 to 5".to_string()),
                "{n}"
            );
        }
        assert_eq!(
            check_arguments(&uat(&["first-session", "--leases", "5"])),
            None
        );
    }

    // the removed PowerShell test file (planning): '21 runs per lease is refused', at the argument stage
    #[test]
    fn runs_per_lease_outside_1_to_20_are_a_usage_error() {
        for n in ["0", "21"] {
            let a = uat(&["first-session", "--runs-per-lease", n]);
            assert_eq!(
                check_arguments(&a),
                Some("--runs-per-lease must be 1 to 20".to_string()),
                "{n}"
            );
        }
        assert_eq!(
            check_arguments(&uat(&["first-session", "--runs-per-lease", "20"])),
            None
        );
    }

    // the removed PowerShell test file: '-Instance with leases is a usage error'
    #[test]
    fn instance_with_two_leases_is_a_usage_error() {
        let a = uat(&["first-session", "--leases", "2", "--instance", "p2"]);
        assert_eq!(
            check_arguments(&a),
            Some("--instance runs one lane; drop --leases or --instance".to_string())
        );
        assert_eq!(
            check_arguments(&uat(&["first-session", "--instance", "p2"])),
            None
        );
    }

    // the removed PowerShell test file: 'a negative stagger is a usage error' (clap rejects the
    // negative; our check keeps the upper bound)
    #[test]
    fn stagger_seconds_must_be_0_to_300() {
        assert!(try_parse("uat", &["first-session", "--stagger-seconds=-5"]).is_err());
        let a = uat(&["first-session", "--stagger-seconds", "301"]);
        assert_eq!(
            check_arguments(&a),
            Some("--stagger-seconds must be 0 to 300".to_string())
        );
        assert_eq!(
            check_arguments(&uat(&["first-session", "--stagger-seconds", "0"])),
            None
        );
        assert_eq!(
            check_arguments(&uat(&["first-session", "--stagger-seconds", "300"])),
            None
        );
    }

    // the removed PowerShell test file: 'a zero timeout (no timeout at all) is a usage error'
    #[test]
    fn run_timeout_minutes_must_be_1_to_120() {
        for n in ["0", "121"] {
            let a = uat(&["first-session", "--run-timeout-minutes", n]);
            assert_eq!(
                check_arguments(&a),
                Some("--run-timeout-minutes must be 1 to 120".to_string()),
                "{n}"
            );
        }
        assert_eq!(
            check_arguments(&uat(&["first-session", "--run-timeout-minutes", "120"])),
            None
        );
    }

    // the removed PowerShell test file: 'a negative brake is a usage error'
    #[test]
    fn max_consecutive_failures_must_be_0_to_20() {
        assert!(try_parse("uat", &["first-session", "--max-consecutive-failures=-1"]).is_err());
        let a = uat(&["first-session", "--max-consecutive-failures", "21"]);
        assert_eq!(
            check_arguments(&a),
            Some("--max-consecutive-failures must be 0 to 20".to_string())
        );
        assert_eq!(
            check_arguments(&uat(&["first-session", "--max-consecutive-failures", "0"])),
            None
        );
    }

    // the removed PowerShell test file: '-Fixtures always is valid'
    #[test]
    fn fixtures_always_is_valid() {
        assert_eq!(
            check_arguments(&uat(&["first-session", "--fixtures", "always"])),
            None
        );
    }

    // the removed PowerShell test file: 'no -Fixtures leaves the daemon default'
    #[test]
    fn no_fixtures_leaves_the_daemon_default() {
        let a = uat(&["first-session"]);
        assert_eq!(a.fixtures, None);
        assert_eq!(check_arguments(&a), None);
    }

    // the removed PowerShell test file: '-Fixtures is case-sensitive' and 'an unknown -Fixtures
    // mode is a usage error': clap refuses both, and our own check refuses a
    // value that gets past it.
    #[test]
    fn fixtures_is_case_sensitive_and_closed() {
        assert!(try_parse("uat", &["first-session", "--fixtures", "Always"]).is_err());
        assert!(try_parse("uat", &["first-session", "--fixtures", "sometimes"]).is_err());
        for bad in ["Always", "sometimes"] {
            let mut a = uat(&["first-session"]);
            a.fixtures = Some(bad.to_string());
            assert_eq!(
                check_arguments(&a),
                Some("--fixtures must be auto, always or never".to_string()),
                "{bad}"
            );
        }
    }

    // the removed PowerShell test file: '-Heal -Leases 2 is refused', '-Heal with one lease is
    // allowed', 'two leases without -Heal are allowed'
    #[test]
    fn heal_needs_one_lease() {
        let a = uat(&["first-session", "--leases", "2", "--heal"]);
        assert_eq!(
            check_arguments(&a),
            Some(
                "--heal needs --leases 1 (it recalibrates on the instance the batch ran on)"
                    .to_string()
            )
        );
        assert_eq!(
            check_arguments(&uat(&[
                "first-session",
                "--leases",
                "1",
                "--runs-per-lease",
                "5",
                "--heal"
            ])),
            None
        );
        assert_eq!(
            check_arguments(&uat(&["first-session", "--leases", "2"])),
            None
        );
    }

    // the removed PowerShell test file: 'row ids are trimmed and de-duplicated'
    #[test]
    fn row_ids_are_trimmed_and_deduplicated() {
        let got = row_ids(&strings(&["FS-01, FS-02", "FS-02", " FS-P1 "]));
        assert_eq!(got, ["FS-01", "FS-02", "FS-P1"]);
        assert_eq!(row_ids(&strings(&["fs-01", "FS-01", ",,"])), ["fs-01"]);
        assert!(row_ids(&[]).is_empty());
    }

    // the removed PowerShell test file: 'progress goes to the console by default', '-Json alone
    // silences progress', '-Quiet silences progress', '-ProgressToStderr keeps
    // progress under -Json, on stderr', '-ProgressToStderr wins over -Quiet'
    #[test]
    fn progress_mode_follows_the_flags() {
        assert_eq!(progress_mode(false, false, false), Progress::Host);
        assert_eq!(progress_mode(true, false, false), Progress::None);
        assert_eq!(progress_mode(false, true, false), Progress::None);
        assert_eq!(progress_mode(true, false, true), Progress::Stderr);
        assert_eq!(progress_mode(true, true, true), Progress::Stderr);
    }

    // the removed PowerShell test file: 'the chaos header has one chaos=<spec> per value,
    // joined with "; "'
    #[test]
    fn the_chaos_header_has_one_entry_per_value() {
        let specs = strings(&["cpu", "flush:pack"]);
        assert_eq!(
            chaos_header(&specs),
            Some("chaos=cpu; chaos=flush:pack".to_string())
        );
    }

    // the removed PowerShell test file: 'no chaos gives no header'
    #[test]
    fn no_chaos_gives_no_header() {
        assert_eq!(chaos_header(&[]), None);
    }

    // the removed PowerShell test file: 'a quoted multi-knob spec stays one value'
    #[test]
    fn a_multi_knob_spec_stays_one_value() {
        let a = uat(&["first-session", "--chaos", "net:latency=120,loss=1"]);
        let specs = chaos_specs(&a.chaos);
        assert_eq!(specs, ["net:latency=120,loss=1"]);
        assert_eq!(
            chaos_header(&specs),
            Some("chaos=net:latency=120,loss=1".to_string())
        );
    }

    // the removed PowerShell test file: 'an empty -Chaos value reaches neither the body nor
    // the header', 'no -Chaos leaves the body without a chaos key'
    #[test]
    fn an_empty_chaos_value_is_dropped() {
        let specs = chaos_specs(&strings(&["", "cpu"]));
        assert_eq!(specs, ["cpu"]);
        assert_eq!(chaos_header(&specs), Some("chaos=cpu".to_string()));
        assert!(chaos_specs(&[]).is_empty());
        assert!(chaos_specs(&strings(&[""])).is_empty());
    }

    // the removed PowerShell test file: 'cpu with -Leases 2 is refused'
    #[test]
    fn cpu_chaos_with_two_leases_is_refused() {
        let msg = "--chaos cpu loads the whole machine and each lease's run would start its own load; run it with --leases 1";
        assert_eq!(
            check_chaos_leases(&strings(&["cpu:cores=2", "flush:pack"]), 2),
            Some(msg.to_string())
        );
        assert_eq!(
            check_chaos_leases(&strings(&["CPU"]), 2),
            Some(msg.to_string())
        );
    }

    // the removed PowerShell test file: 'cpu with -Leases 1 is fine', 'net and flush with
    // -Leases 3 are fine', 'no chaos with -Leases 3 is fine'
    #[test]
    fn other_chaos_combinations_are_fine() {
        assert_eq!(check_chaos_leases(&strings(&["cpu"]), 1), None);
        assert_eq!(
            check_chaos_leases(&strings(&["net", "flush:pack"]), 3),
            None
        );
        assert_eq!(check_chaos_leases(&[], 3), None);
        // a profile that merely starts with cpu is not cpu
        assert_eq!(check_chaos_leases(&strings(&["cpufoo"]), 3), None);
    }
}
